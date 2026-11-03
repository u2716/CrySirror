// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//!77A860/629850 events0/1 and5F3AE0 message296. Native bitmap0/visibility
//!policy only; engine Canvas owns immutable copies, composition and live views.
use crate::{
    fes_program::PendingOperation,
    fes_runtime::Services,
    native_class_state::State,
    native_image_publication::{ImageList, Plane},
};
use anyhow::{ensure, Context, Result};
use crysirror_core::service::HostServices;
use crysirror_scene::{
    canvas::{CanvasCommand as C, CanvasNode, CanvasSurface},
    layer::{LayerCommand as L, LayerSpec},
};
impl Services<'_> {
    ///780710 -> event28 -> ImageList297 sets original base position(0,0,z).
    ///Its signed integer default1000 is distinct from draw-key0's priority.
    pub(crate) fn base_image_priority(
        &mut self,
        host: &mut dyn HostServices,
        owner: u32,
        operation: &PendingOperation,
    ) -> Result<()> {
        ensure!(
            operation.arguments.len() <= 1,
            "CatSystem2 baseimg_pri arguments"
        );
        let mut vm = self.interpreter.clone();
        let z = if let Some(raw) = operation.arguments.first() {
            vm.evaluate(raw)? as i32 as f32
        } else {
            1000.
        };
        ensure!(
            z.abs() <= 1_000_000.,
            "CatSystem2 baseimg_pri outside reviewed native depth budget"
        );
        let list = self.tasks.image_list(
            self.tasks
                .fes(owner)?
                .image_list
                .context("CatSystem2 baseimg_pri ImageList missing")?,
        )?;
        let base = self.tasks.plane(
            list.base_plane
                .context("CatSystem2 baseimg_pri base Plane missing")?,
        )?;
        let payload = base
            .payload
            .context("CatSystem2 baseimg_pri base geometry missing")?;
        let mut layer: LayerSpec =
            serde_json::from_slice::<Option<LayerSpec>>(&host.query_engine(
                self.client,
                crysirror_core::service::EngineQuery::Layer(payload),
            )?)?
            .context("CatSystem2 baseimg_pri engine geometry missing")?;
        layer.rect[0] = 0.;
        layer.rect[1] = 0.;
        layer.transform.z = z;
        let batch = crysirror_scene::ServiceBatch {
            layers: vec![
                L::SetRect {
                    id: payload,
                    rect: layer.rect,
                },
                L::SetTransform {
                    id: payload,
                    transform: layer.transform,
                },
                L::SetDepth {
                    id: payload,
                    depth: Some(z),
                },
                L::SetVisible {
                    id: payload,
                    visible: (0.0..1_000_000.0).contains(&z),
                },
            ],
            ..Default::default()
        };
        host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)?;
        *self.interpreter = vm;
        Ok(())
    }
    pub(crate) fn base_image(
        &mut self,
        host: &mut dyn HostServices,
        owner: u32,
        operation: &PendingOperation,
    ) -> Result<()> {
        ensure!(
            operation.arguments.len() <= 1,
            "CatSystem2 baseimg arguments"
        );
        let task = self.tasks.fes(owner)?;
        let list_id = task
            .image_list
            .context("CatSystem2 baseimg ImageList binding missing")?;
        let list = self.tasks.image_list(list_id)?;
        let manager = list
            .manager
            .as_ref()
            .context("CatSystem2 baseimg real atlas missing")?;
        ensure!(
            manager.resource.client() == self.client,
            "CatSystem2 baseimg atlas belongs to another client"
        );
        let base = self.tasks.plane(
            list.base_plane
                .context("CatSystem2 original base Plane missing")?,
        )?;
        let payload = base
            .payload
            .context("CatSystem2 original base Plane has no engine geometry")?;
        let base_id = base.id;
        let viewport = base
            .image
            .as_ref()
            .context("CatSystem2 original base Plane has no viewport resource")?
            .resource
            .size();
        let mut publication = list.publication.clone().unwrap_or(ImageList::new(
            self.client,
            0x58000000u32
                .checked_add(list_id)
                .context("CatSystem2 atlas Canvas range")?,
            manager.size,
        )?);
        let mut commands = Vec::new();
        let bitmap = 0x5e000000u32
            .checked_add(list_id)
            .context("CatSystem2 copied bitmap Canvas range")?;
        if let Some(raw) = operation.arguments.first() {
            let bank = task
                .object_bank
                .as_ref()
                .context("CatSystem2 baseimg named object bank missing")?;
            let mut vm = self.interpreter.clone();
            let reference =
                crate::fes_references::evaluate(raw, &mut vm, self.string_variables, &|name| {
                    bank.count(name)
                })?;
            ensure!(
                reference.index >= 0,
                "CatSystem2 baseimg requires one original bitmap/Plane"
            );
            let id = bank
                .lookup(&reference)
                .context("CatSystem2 baseimg native object missing")?;
            let object = &bank.objects[&id];
            let size = match &object.state {
                State::CImage(_) | State::Image(_) => {
                    let resident = self.fes_children.frames.ready.get(&owner).context(
                        "CatSystem2 baseimg bitmap has not completed its original resource phase",
                    )?;
                    let image = resident
                        .resources
                        .get(&id)
                        .context("CatSystem2 baseimg original bitmap absent")?;
                    let layer: LayerSpec = serde_json::from_value(
                        serde_json::json!({"id":0,"rect":[0,0,image.size[0],image.size[1]],"paint":{"kind":"image","asset":image.path.relative()}}),
                    )?;
                    commands.push(C::Restore {
                        id: bitmap,
                        surface: CanvasSurface {
                            width: image.size[0],
                            height: image.size[1],
                            nodes: vec![CanvasNode::Draw { layer }],
                        },
                    });
                    image.size
                }
                State::Plane(plane) => {
                    let mut size = [u32::try_from(plane.size[0])?, u32::try_from(plane.size[1])?];
                    ensure!(
                        size.iter().all(|v| (1..=8192).contains(v)),
                        "CatSystem2 baseimg Plane copy extent"
                    );
                    ensure!(
                        plane.vram[..2]
                            .iter()
                            .all(|v| v.is_finite() && *v >= 0. && v.fract() == 0.),
                        "CatSystem2 baseimg Plane negative/fractional source profile"
                    );
                    let origin = [plane.vram[0] as u32, plane.vram[1] as u32];
                    for i in 0..2 {
                        ensure!(
                            origin[i] < manager.size[i],
                            "CatSystem2 baseimg Plane source lies outside original bitmap"
                        );
                        size[i] = size[i].min(manager.size[i] - origin[i]);
                    }
                    //Original event1 explicitly flushes290 before285/296.
                    //Copy current VRAM, independently of Plane position/alpha/
                    //visibility; the engine freezes nodes, no window is captured.
                    commands.extend(publication.commands()?);
                    commands.push(C::Restore {
                        id: bitmap,
                        surface: CanvasSurface {
                            width: size[0],
                            height: size[1],
                            nodes: Vec::new(),
                        },
                    });
                    commands.push(C::Copy {
                        source: publication.scope().1,
                        destination: bitmap,
                        rect: [
                            origin[0] as f32,
                            origin[1] as f32,
                            size[0] as f32,
                            size[1] as f32,
                        ],
                        position: [0.; 2],
                        opacity: 255,
                        replace: true,
                        clip: None,
                        atop: None,
                    });
                    size
                }
                _ => anyhow::bail!(
                    "CatSystem2 baseimg unsupported original class {}",
                    object.declared_type
                ),
            };
            publication.put_canvas(0, bitmap, size)?;
            publication.plane(Plane {
                canvas: 0x5f000000u32
                    .checked_add(list_id)
                    .context("CatSystem2 base view Canvas range")?,
                presentation: 0x12000000u32
                    .checked_add(list_id)
                    .context("CatSystem2 base view presentation range")?,
                parent: payload,
                source: [0, 0, viewport[0] as i32, viewport[1] as i32],
            })?;
            self.commit_base_image(
                host,
                list_id,
                base_id,
                publication,
                commands,
                true,
                Some(vm),
            )
        } else {
            //742FE0(manager,0) removes only reserved bitmap0. Other sender
            //records and the previous copied bitmap are deliberately retained.
            publication.remove(0);
            self.commit_base_image(host, list_id, base_id, publication, commands, false, None)
        }
    }
    fn commit_base_image(
        &mut self,
        host: &mut dyn HostServices,
        list: u32,
        plane: u32,
        publication: ImageList,
        mut commands: Vec<C>,
        visible: bool,
        vm: Option<crate::native_interpreter::Interpreter>,
    ) -> Result<()> {
        let mut proposed = self.tasks.clone();
        let visibility = proposed.visibility_messages(plane, visible)?;
        proposed.set_base_image_publication(list, publication.clone(), visible)?;
        commands.extend(publication.commands()?);
        let batch = crysirror_scene::ServiceBatch {
            canvases: commands,
            layers: visibility
                .into_iter()
                .map(|(id, visible)| L::SetVisible { id, visible })
                .collect(),
            ..Default::default()
        };
        host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)?;
        *self.tasks = proposed;
        if let Some(vm) = vm {
            *self.interpreter = vm;
        }
        Ok(())
    }
}
