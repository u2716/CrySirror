// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Chart's grow-group/new-mark owner. Candidate pictures and Print clocks are
//! frozen over shared prefix/image jobs; UI delivers scoped hit-test results.
use super::*;
use crate::{
    native_chart::ChartNewMark,
    native_chart_decorations::{FrameWork, Pictures, Preparation},
    native_radio_group::{ButtonInput, RadioGroup, RadioSpec},
};
use std::collections::BTreeSet;
pub(super) struct Skin {
    pub radio: RadioGroup,
    pub mark: Option<ChartNewMark>,
    pictures: Option<Pictures>,
    preparation: Option<Preparation>,
    paint: Option<Paint>,
    path: Option<String>,
    pub retired: Vec<String>,
    ready: bool,
    widgets_match: bool,
}
enum Paint {
    Lower(ImageJob, Vec<Value>, i32),
    Prefix(ImageJob),
    Decorations(FrameWork),
}
pub(super) struct Signal {
    pub group: Option<usize>,
    pub sound: i32,
    pub voice: i32,
    pub volume_type: i32,
    pub blocking: bool,
}
impl Skin {
    pub fn new(
        config: &crate::native_bootstrap::NativeConfiguration,
        runtime: &ChartRuntime,
        state: &ChartState,
        current: i32,
        group: usize,
        variables: &mut Variables,
    ) -> Result<Self> {
        let mut radio = RadioGroup::new(RadioSpec::read(
            &runtime.definition.setup,
            "grow",
            runtime.definition.maps.len(),
            config,
        )?);
        for (i, map) in runtime.definition.maps.iter().enumerate() {
            let open = runtime.open(i, variables)?;
            let mut enabled = false;
            if open {
                for slot in &map.nodes {
                    if runtime.selectable(state, *slot, variables)?
                        && (runtime.definition.node(current)?.next.contains(slot)
                            || runtime.seen(*slot)?)
                    {
                        enabled = true;
                        break;
                    }
                }
            }
            radio.set_flags(i, i < runtime.definition.basic || open, enabled)?;
        }
        radio.init(group)?;
        let mut mark = ChartNewMark::read(config, &runtime.definition.setup)?;
        if let Some(mark) = mark.as_mut() {
            mark.init(runtime, state, current, variables)?;
        }
        Ok(Self {
            radio,
            mark,
            pictures: None,
            preparation: None,
            paint: None,
            path: None,
            retired: vec![],
            ready: false,
            widgets_match: false,
        })
    }
    pub fn reload(
        &mut self,
        config: &crate::native_bootstrap::NativeConfiguration,
        resources: NativeResources,
        pool: TaskPool,
        client: ClientId,
        group: usize,
    ) -> Result<()> {
        ensure!(
            self.paint.is_none(),
            "NNN chart skin reload during admitted Print"
        );
        self.radio.init(group)?;
        let lookup = self
            .mark
            .as_ref()
            .map(|mark| {
                let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
                config.resources.picture(
                    &[b"sys/".as_slice(), &mark.clock.spec.filename].concat(),
                    levels,
                )
            })
            .transpose()?;
        self.preparation = Some(Preparation::begin(
            client,
            resources,
            pool,
            &self.radio,
            self.mark.as_ref(),
            self.pictures.as_ref(),
            lookup,
        )?);
        self.ready = false;
        self.widgets_match = false;
        Ok(())
    }
    pub fn prepare(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if let Some(preparation) = self.preparation.as_mut() {
            let Some(pictures) = preparation.poll(host)? else {
                return Ok(false);
            };
            self.pictures = Some(pictures);
            self.preparation = None;
        }
        Ok(true)
    }
    pub fn pending(&self) -> bool {
        self.preparation.is_some() || self.paint.is_some()
    }
    pub fn print_prefix(
        &mut self,
        client: ClientId,
        host: &mut dyn HostServices,
        commands: Vec<Value>,
        count: i32,
    ) -> Result<Option<String>> {
        if self.ready {
            self.ready = false;
            return Ok(self.path.clone());
        }
        ensure!(
            self.paint.is_none(),
            "NNN chart duplicate skin Print admission"
        );
        self.paint = Some(Paint::Lower(
            host.begin_layer_prefix_image(client, SIZE, [ORDER, 0])?,
            commands,
            count,
        ));
        Ok(None)
    }
    pub fn poll_paint(&mut self, client: ClientId, host: &mut dyn HostServices) -> Result<bool> {
        let Some(work) = self.paint.take() else {
            return Ok(false);
        };
        self.paint = Some(match work {
            Paint::Lower(mut job, mut commands, count) => match job.poll()? {
                ImageProcessState::Pending => Paint::Lower(job, commands, count),
                ImageProcessState::Failed(e) => anyhow::bail!("NNN chart lower prefix: {e}"),
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == client && image.size() == SIZE,
                        "NNN chart lower prefix scope"
                    );
                    let base = host
                        .publish_image_resource(client, ROOT + 9500, image)?
                        .path
                        .relative()
                        .to_owned();
                    commands.retain(|c| c["op"] == "put");
                    commands.insert(0,json!({"op":"put","layer":{"id":ROOT+9500,"rect":[0,0,800,600],"order":[ORDER-1,0],"blend":{"stored_byte":{"mode":"opaque_scalar256","force_output_opaque":true}},"paint":{"kind":"image","asset":base}}}));
                    let captured = host.begin_detached_layers_image(
                        client,
                        SIZE,
                        [ORDER, count],
                        &serde_json::to_vec(&commands)?,
                    );
                    // The shared capture freezes generations before returning;
                    // no temporary published path survives a failed admission.
                    host.release_image_resource(client, &base)?;
                    Paint::Prefix(captured?)
                }
            },
            Paint::Prefix(mut job) => match job.poll()? {
                ImageProcessState::Pending => Paint::Prefix(job),
                ImageProcessState::Failed(e) => anyhow::bail!("NNN chart detached prefix: {e}"),
                ImageProcessState::Ready(image) => Paint::Decorations(FrameWork::begin(
                    client,
                    image,
                    self.radio.clone(),
                    self.mark.clone(),
                    self.pictures
                        .as_ref()
                        .context("NNN chart skin pictures absent")?,
                    host,
                )?),
            },
            Paint::Decorations(mut work) => {
                if let Some(frame) = work.poll(host)? {
                    let published =
                        host.publish_image_resource(client, ROOT + 9501, frame.image)?;
                    if let Some(old) = self.path.replace(published.path.relative().to_owned()) {
                        self.retired.push(old);
                    }
                    self.radio = frame.radio;
                    self.mark = frame.mark;
                    self.ready = true;
                    self.paint = None;
                    return Ok(true);
                }
                Paint::Decorations(work)
            }
        });
        Ok(false)
    }
    pub fn calcu(
        &mut self,
        client: ClientId,
        host: &mut dyn HostServices,
        input: InputFrame,
        activated: &BTreeSet<usize>,
        enabled: bool,
    ) -> Result<Signal> {
        if !self.widgets_match {
            self.radio.calcu(None)?;
            return Ok(Signal {
                group: None,
                sound: 0,
                voice: 0,
                volume_type: 0,
                blocking: false,
            });
        }
        let mut inputs = Vec::with_capacity(self.radio.spec.buttons.len());
        for i in 0..self.radio.spec.buttons.len() {
            let (spec, _) = self.radio.active(i)?;
            let interaction: Value = serde_json::from_slice(
                &host.query_engine(client, EngineQuery::UiInteraction(ROOT + 100 + i as u32))?,
            )?;
            let wheel_up = input.buttons_pressed & buttons::WHEEL_UP != 0;
            let wheel_down = input.buttons_pressed & buttons::WHEEL_DOWN != 0;
            let wheel = if spec.wheel_reverse {
                [wheel_down, wheel_up]
            } else {
                [wheel_up, wheel_down]
            };
            let pressed = activated.contains(&i)
                || spec.cancel && input.buttons_pressed & buttons::MOUSE_RIGHT != 0
                || spec
                    .wheel
                    .iter()
                    .zip(wheel)
                    .any(|(flag, trig)| *flag && trig)
                || spec
                    .button
                    .accelerators
                    .iter()
                    .filter(|key| **key != 0)
                    .filter_map(|key| buttons::windows_virtual_key(u32::from(*key)))
                    .any(|bit| input.buttons_down & bit != 0);
            inputs.push(ButtonInput {
                hovered: interaction["hovered"] == true,
                pressed,
            });
        }
        let signal = self.radio.calcu(enabled.then_some(inputs.as_slice()))?;
        let status = signal & 0xff000000u32 as i32;
        let sound = if signal == -1 || signal & 0x8000 == 0 {
            0
        } else {
            (signal >> 8) & 0x7f
        };
        Ok(Signal {
            group: (signal != -1 && status == 0xf0000000u32 as i32)
                .then_some((signal & 0x7f) as usize),
            sound,
            voice: if signal == -1 {
                0
            } else {
                (signal >> 16) & 0x3f
            },
            volume_type: if signal == -1 { 0 } else { (signal >> 22) & 3 },
            blocking: signal != -1
                && matches!(status, x if x==0xf8000000u32 as i32||x==0xf9000000u32 as i32),
        })
    }
    pub fn forget_widgets(&mut self) {
        self.widgets_match = false;
    }
    pub fn published(&mut self) {
        self.widgets_match = true;
    }
    pub fn widgets(&self) -> Result<Vec<Value>> {
        let pictures = self
            .pictures
            .as_ref()
            .context("NNN chart skin widgets before pictures")?;
        let mut out = vec![];
        for i in 0..self.radio.spec.buttons.len() {
            let (spec, _) = self.radio.active(i)?;
            let b = &spec.button;
            let mut widget = json!({"id":ROOT+100+i as u32,"parent":ROOT,"kind":"button","rect":[b.origin[0]+b.check_delta[0],b.origin[1]+b.check_delta[1],b.check_size[0],b.check_size[1]],"interaction_only":true,"keyboard_focusable":false,"activate_on_press":true,"visible":self.radio.exists(i)?,"enabled":true,"text":"","color":[0,0,0,0],"z":i});
            if b.print_mode != 0 {
                let coverage = if let Some(picture) = &pictures.grow[i] {
                    let view = if picture.has_mask {
                        &picture.raw
                    } else {
                        &picture.keyed
                    };
                    view.alpha_region([
                        u32::try_from(b.check_origin[0])?,
                        u32::try_from(b.check_origin[1])?,
                        b.check_size[0] as u32,
                        b.check_size[1] as u32,
                    ])?
                } else {
                    vec![0u8; b.check_size[0] as usize * b.check_size[1] as usize].into()
                };
                widget["hit_mask"] = json!({"size":b.check_size,"coverage":coverage,"threshold":1,"strict_bounds":false});
            }
            out.push(json!({"op":"put","widget":widget}));
        }
        Ok(out)
    }
    pub fn cleanup(&self) -> Vec<String> {
        self.path
            .iter()
            .cloned()
            .chain(self.retired.iter().cloned())
            .collect()
    }
}
