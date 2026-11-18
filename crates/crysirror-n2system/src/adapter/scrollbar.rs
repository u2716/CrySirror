// SPDX-License-Identifier: MPL-2.0
//! Native knob/endpoint policy from NitroSharp Scrollbar.cs, 77a93db4 (MIT).
//! Shared widgets retain touch/pointer capture and focus; the engine-owned
//! LinearSliderTrack retains the directional path and grab-offset calculation.
use super::*;
use crysirror_scene::ui::{WidgetInteraction, linear_slider::LinearSliderTrack};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Scrollbar {
    pub track: LinearSliderTrack,
    pub target: Option<u32>,
}
impl N2SystemVm {
    pub(super) fn native_scrollbar(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        let mut result = Value::Null;
        match call.function {
            Builtin::CreateScrollbar => {
                let mut surface_lease = None;
                let source = args.string(8)?;
                let surface = self.surface_source(current,&source)?;
                if state.surface_source.as_ref().is_some_and(|(id,_)|Some(*id)!=surface) {state.surface_source=None;}
                let image = if let Some(id) = surface {
                    if state.surface_source.is_none() && self.shades.get(&id).is_some_and(shade::Shade::pending) {return Ok(false);}
                    state.image=None;
                    if state.surface_source.is_none() {state.surface_source=Some((id,self.begin_surface_image(host,id)?));}
                    let image = match state.surface_source.as_mut().unwrap().1.poll()? {
                        crysirror_core::image_processing::ImageProcessState::Pending=>return Ok(false),
                        crysirror_core::image_processing::ImageProcessState::Failed(error)=>bail!("N2System scrollbar surface: {error}"),
                        crysirror_core::image_processing::ImageProcessState::Ready(image)=>image,
                    };
                    let size=image.size();surface_lease=Some(image);state.surface_source=None;
                    Image {paint:json!({"kind":"image","asset":""}),size}
                } else if let Some(task) = &mut state.image {
                    let Some(image) = task.try_take()? else {
                        return Ok(false);
                    };
                    state.image = None;
                    image
                } else {
                    let image = self
                        .source_image(current, &args.string(8)?)?
                        .context("N2System scrollbar knob missing")?;
                    if image.size == [0; 2] {
                        let metadata = self.images.as_ref().unwrap().clone();
                        match self.pool.as_ref().unwrap().spawn(move |cancel| {
                            cancel.check()?;
                            let mut image = image;
                            image.size = metadata.size(
                                image.paint["asset"]
                                    .as_str()
                                    .context("scrollbar image asset")?,
                            )?;
                            cancel.check()?;
                            Ok(image)
                        }) {
                            Ok(task) => state.image = Some(task),
                            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => {}
                            Err(e) => return Err(e),
                        }
                        return Ok(false);
                    }
                    image
                };
                let vertical = match args.constant(7)? {
                    "Vertical" => true,
                    "Horizon" => false,
                    _ => bail!("N2System scrollbar direction"),
                };
                let start = [args.integer(2)? as f32, args.integer(3)? as f32];
                let end = [args.integer(4)? as f32, args.integer(5)? as f32];
                let track = LinearSliderTrack::new(
                    start,
                    end,
                    image.size.map(|v| v as f32),
                    vertical,
                    args.value(6).number()? / 1000.,
                )?;
                let id = self.visual_owned(host, current, args, image.paint, image.size, None,surface_lease)?;
                let position = track.position();
                let mut layer = self.layer(host, id)?;
                layer["rect"][0] = json!(position[0]);
                layer["rect"][1] = json!(position[1]);
                self.submit(host,vec![json!({"op":"set_rect","id":id,"rect":layer["rect"]})],vec![json!({"op":"put","widget":{
                    "id":id,"kind":"button","rect":track.bounds(),"z":args.priority(1)?,"interaction_only":true,
                    "keyboard_navigation":false,"keyboard_activation":false,"focus_follows_pointer":true,
                }})])?;
                self.scrollbars.insert(
                    id,
                    Scrollbar {
                        track,
                        target: None,
                    },
                );
            }
            Builtin::SetScrollbar => {
                let target = self.query(current, &args.string(1)?)?.first().copied();
                for id in self.query(current, &args.string(0)?)? {
                    if let Some(bar) = self.scrollbars.get_mut(&id) {
                        bar.target = target;
                    }
                }
            }
            Builtin::SetScrollSpeed => {
                let speed = args.value(1).number()?;
                let speed = if speed.is_finite() {
                    speed.clamp(0., 1_000_000.)
                } else {
                    0.
                };
                for id in self.query(current, &args.string(0)?)? {
                    if let Some(bar) = self.scrollbars.get_mut(&id) {
                        bar.track.speed = speed;
                    }
                }
            }
            Builtin::ScrollbarValue => {
                result = Value::Number(0.);
                for id in self.query(current, &args.string(0)?)? {
                    if self.scrollbars.contains_key(&id) {
                        let layer = self.layer(host, id)?;
                        let bar = self.scrollbars.get_mut(&id).unwrap();
                        bar.track.observe_position([
                            layer["rect"][0].as_f64().context("scrollbar x")? as f32,
                            layer["rect"][1].as_f64().context("scrollbar y")? as f32,
                        ]);
                        result = Value::Number(bar.track.value);
                        break;
                    }
                }
            }
            _ => bail!("N2System scrollbar dispatch mismatch"),
        }
        self.machine.complete(&mut state.thread, result)?;
        Ok(true)
    }
    pub(super) fn update_scrollbars(
        &mut self,
        host: &mut dyn HostServices,
        input: &InputFrame,
    ) -> Result<()> {
        let ids = self.scrollbars.keys().copied().collect::<Vec<_>>();
        let mut changes = Vec::new();
        for id in ids {
            let layer = self.layer(host, id)?;
            let interaction: WidgetInteraction = serde_json::from_slice(
                &host.query_engine(self.client, EngineQuery::UiInteraction(id))?,
            )?;
            let bar = self.scrollbars.get_mut(&id).unwrap();
            bar.track.observe_position([
                layer["rect"][0].as_f64().context("scrollbar x")? as f32,
                layer["rect"][1].as_f64().context("scrollbar y")? as f32,
            ]);
            let old = bar.track.value;
            if interaction.pressed && interaction.pointer_buttons_down & buttons::MOUSE_LEFT != 0 {
                if let Some(pointer) = interaction.pointer {
                    if !bar.track.dragging() {
                        bar.track.begin_drag(pointer.map(|v| v as f32));
                    } else {
                        bar.track.drag(pointer.map(|v| v as f32));
                    }
                }
            } else {
                bar.track.end_drag();
            }
            if interaction.focused {
                let (negative, positive) = if bar.track.vertical {
                    (buttons::UP, buttons::DOWN)
                } else {
                    (buttons::LEFT, buttons::RIGHT)
                };
                let direction = i32::from(input.buttons_pressed & positive != 0)
                    - i32::from(input.buttons_pressed & negative != 0);
                if direction != 0 {
                    bar.track.step(direction);
                }
            }
            if old != bar.track.value {
                let p = bar.track.position();
                changes.push(json!({"op":"set_rect","id":id,"rect":[p[0],p[1],layer["rect"][2],layer["rect"][3]]}));
            }
        }
        self.submit(host, changes, vec![])
    }
}
