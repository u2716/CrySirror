//! Native blocking Window calls over shared UI chrome and retained Layer trees.
//! No nested OS event loop and no adapter framebuffer: the desktop keeps its
//! window/device, while a modal viewport routes events to the child KAGWindow.
use super::*;
use crysirror_tjs2::{Result as TjsResult, TjsError};
const TITLE: f32 = 28.;
#[derive(Clone)]
pub(crate) struct Modal {
    pub window: usize,
    pub widget: u32,
    pub offset: [f32; 2],
    pub scale: f32,
    pub parking: bool,
    pub closed: bool,
    pub querying_close: bool,
    keys: u64,
}
pub(super) struct ParkedPass {
    pass: Option<FramePass>,
    input: Option<input_delivery::Delivery>,
    inbox: crysirror_core::input_buffer::InputBuffer,
}
impl ParkedPass {
    pub(super) fn roots(&self, out: &mut Vec<Variant>) {
        if let Some(pass) = &self.pass {
            out.extend(pass.handlers.iter().cloned());
            if let Some(paints) = &pass.paints {
                out.extend(
                    paints
                        .iter()
                        .skip(pass.paint)
                        .map(|id| Variant::Object(ObjectHandle(*id))),
                );
            }
        }
        if let Some(input) = &self.input {
            input.roots(out);
        }
    }
}
pub(crate) fn begin(
    rt: &mut Rt,
    this: Option<ObjectHandle>,
    _: Vec<Variant>,
) -> TjsResult<Variant> {
    let h = natives::this_of(rt, this, "Window.showModal")?;
    let host = rt.host();
    let window = host
        .windows
        .get(&h.0)
        .ok_or_else(|| TjsError::runtime("Window not constructed"))?;
    if !host.can_suspend
        || window.output
        || window.primary.is_none()
        || host.modal_windows.len() >= 8
        || host.modal_windows.iter().any(|m| m.window == h.0)
    {
        return Err(TjsError::runtime(
            "Window.showModal requires a bound secondary window and a resumable event scope",
        ));
    }
    let [cw, ch] = host.canvas.map(|v| v as f32);
    let [w, height] = window.inner.map(|v| v as f32);
    let scale = ((cw - 16.) / w).min((ch - TITLE - 16.) / height).min(1.);
    if !scale.is_finite() || scale <= 0. {
        return Err(TjsError::runtime(
            "modal viewport does not fit the engine canvas",
        ));
    }
    let offset = [(cw - w * scale) * 0.5, (ch - height * scale + TITLE) * 0.5];
    let caption = rt.object_member(h, "__caption").to_tjs_string()?;
    let host = rt.host_mut();
    let font = host.font.clone();
    host.modal_serial = host
        .modal_serial
        .checked_add(1)
        .filter(|v| *v < 1_000_000)
        .ok_or_else(|| TjsError::runtime("modal viewport identifier limit"))?;
    let widget = 0x6c00_0000 + host.modal_serial * 4;
    if let Some(previous) = host.modal_windows.last() {
        let id = previous.widget;
        host.ui_op(json!({"op":"set_visible","id":id,"visible":false}));
    }
    let mut keys = Vec::new();
    for bit in (0..64).map(|n| 1u64 << n).filter(|bit| {
        *bit & (buttons::MOUSE_LEFT | buttons::MOUSE_RIGHT | buttons::MOUSE_MIDDLE) == 0
    }) {
        for (edge, name) in [("press", "modal_press_"), ("release", "modal_release_")] {
            keys.push(json!({"buttons":bit,"edge":edge,"event":format!("{name}{bit}")}));
        }
    }
    host.ui_op(json!({"op":"put","widget":{"id":widget,"rect":[0.,0.,cw,ch],"modal":true,
        "interaction_only":true,"keyboard_navigation":false,"keyboard_activation":false,"key_event_scope":true,"key_events":keys,"z":100}}));
    host.ui_op(json!({"op":"put","widget":{"id":widget+1,"parent":widget,"rect":[offset[0],offset[1],w*scale,height*scale],
        "interaction_only":true,"keyboard_navigation":false,"keyboard_activation":false,"z":1}}));
    host.ui_op(json!({"op":"put","widget":{"id":widget+2,"parent":widget,"kind":"label",
        "rect":[offset[0],offset[1]-TITLE,w*scale,TITLE],"color":[238,238,238,255],"text_color":[20,20,20,255],
        "text":caption,"font":font,"text_size":16,"text_inset":[6,4],"z":2}}));
    host.ui_op(
        json!({"op":"put","widget":{"id":widget+3,"parent":widget,"kind":"button",
        "rect":[offset[0]+w*scale-TITLE,offset[1]-TITLE,TITLE,TITLE],"color":[220,220,220,255],
        "text":"×","font":font,"text_color":[20,20,20,255],"text_size":18,"z":3}}),
    );
    host.modal_windows.push(Modal {
        window: h.0,
        widget,
        offset,
        scale,
        parking: true,
        closed: false,
        querying_close: false,
        keys: 0,
    });
    host.windows.get_mut(&h.0).unwrap().visible = true;
    host.menus_revision += 1;
    for layer in host.layers.values_mut() {
        layer.dirty = true;
        layer.presented = None;
    }
    crate::native_mouse::release(rt);
    rt.host_mut().mouse.hover = None;
    rt.request_suspend();
    Ok(Variant::Void)
}
pub(crate) fn contains(host: &KrkrHost, h: usize) -> bool {
    host.modal_windows.iter().any(|m| m.window == h)
}
pub(crate) fn close(rt: &mut Rt, h: ObjectHandle) -> TjsResult<()> {
    if !rt
        .host()
        .modal_windows
        .last()
        .is_some_and(|m| m.window == h.0)
    {
        return Err(TjsError::runtime("only the topmost modal Window can close"));
    }
    rt.host_mut()
        .modal_windows
        .last_mut()
        .unwrap()
        .querying_close = true;
    rt.call_object_method(h, "onCloseQuery", vec![Variant::Integer(1)])?;
    Ok(())
}
pub(crate) fn answer_close(rt: &mut Rt, h: ObjectHandle, allow: bool) {
    if let Some(modal) = rt
        .host_mut()
        .modal_windows
        .iter_mut()
        .find(|m| m.window == h.0)
    {
        if modal.querying_close {
            modal.querying_close = false;
            if allow {
                modal.closed = true;
            }
        }
    }
}
impl KrkrVm {
    /// A context switch ends this VM step; the native TJS caller and its outer
    /// input cursor are parked independently of the modal window's callbacks.
    pub(super) fn modal_boundary(&mut self) -> Result<bool> {
        let Some(modal) = self
            .rt
            .as_ref()
            .unwrap()
            .host()
            .modal_windows
            .last()
            .cloned()
        else {
            return Ok(false);
        };
        if modal.parking {
            let rt = self.rt.as_mut().unwrap();
            ensure!(rt.is_suspended(), "modal native caller did not suspend");
            rt.park_suspended_call()?;
            rt.host_mut().modal_windows.last_mut().unwrap().parking = false;
            let input = self.input_delivery.take();
            let inbox = std::mem::replace(
                &mut self.inbox,
                crysirror_core::input_buffer::InputBuffer::new(self.client.0, 1, 256)?,
            );
            self.modal_passes.push(ParkedPass {
                pass: self.pass.take(),
                input,
                inbox,
            });
            self.last_down = 0;
            self.last_pointer = [i32::MIN, i32::MIN];
            return Ok(true);
        }
        if modal.closed && !self.rt.as_ref().unwrap().is_suspended() {
            let rt = self.rt.as_mut().unwrap();
            rt.host_mut().modal_windows.pop();
            rt.host_mut()
                .ui_op(json!({"op":"remove","id":modal.widget}));
            if let Some(previous) = rt.host().modal_windows.last() {
                let id = previous.widget;
                rt.host_mut()
                    .ui_op(json!({"op":"set_visible","id":id,"visible":true}));
            }
            if let Some(w) = rt.host_mut().windows.get_mut(&modal.window) {
                w.visible = false;
            }
            rt.host_mut().menus_revision += 1;
            for layer in rt.host_mut().layers.values_mut() {
                layer.dirty = true;
                layer.presented = None;
            }
            crate::native_mouse::release(rt);
            rt.host_mut().mouse.hover = None;
            let parked = self
                .modal_passes
                .pop()
                .context("modal delivery scope missing")?;
            self.pass = parked.pass;
            self.input_delivery = parked.input;
            self.inbox = parked.inbox;
            if let Some(pass) = &mut self.pass {
                pass.now = rt.host().now;
            }
            self.last_down = 0;
            self.last_pointer = [i32::MIN, i32::MIN];
            rt.restore_parked_call(Variant::Void)?;
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn modal_input(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
    ) -> Result<InputFrame> {
        let Some(modal) = self
            .rt
            .as_ref()
            .and_then(|rt| rt.host().modal_windows.last())
            .cloned()
        else {
            return Ok(input);
        };
        if modal.parking {
            return Ok(input);
        }
        let events: Vec<crysirror_scene::ui::UiEvent> =
            serde_json::from_slice(&host.query_engine(
                self.client,
                crysirror_core::service::EngineQuery::UiSubtreeEvents(modal.widget),
            )?)?;
        let interaction: crysirror_scene::ui::WidgetInteraction =
            serde_json::from_slice(&host.query_engine(
                self.client,
                crysirror_core::service::EngineQuery::UiInteraction(modal.widget + 1),
            )?)?;
        let mut pressed = 0;
        let mut close = false;
        {
            let rt = self.rt.as_mut().unwrap();
            let state = rt.host_mut().modal_windows.last_mut().unwrap();
            state.keys &= !(buttons::WHEEL_UP | buttons::WHEEL_DOWN);
            for event in events {
                if event.widget.id == modal.widget + 3 && event.kind == "activate" {
                    close = true;
                }
                if let Some(bit) = event
                    .kind
                    .strip_prefix("modal_press_")
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    if bit & (buttons::WHEEL_UP | buttons::WHEEL_DOWN) == 0 {
                        state.keys |= bit;
                    }
                    pressed |= bit;
                }
                if let Some(bit) = event
                    .kind
                    .strip_prefix("modal_release_")
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    state.keys &= !bit;
                }
            }
        }
        // Run the close query at the normal event boundary, after any child
        // image/clock/storage continuation completes. Do not overwrite it.
        if close && !modal.querying_close {
            self.rt.as_mut().unwrap().host_mut().user_close_pending = true;
        }
        let mut frame = input;
        if let Some([x, y]) = interaction.pointer {
            frame.pointer_x = ((x as f32 - modal.offset[0]) / modal.scale).floor() as i32;
            frame.pointer_y = ((y as f32 - modal.offset[1]) / modal.scale).floor() as i32;
        }
        frame.buttons_down = self
            .rt
            .as_ref()
            .unwrap()
            .host()
            .modal_windows
            .last()
            .map_or(0, |m| m.keys);
        frame.buttons_pressed = pressed;
        // The viewport is an interaction-only Panel, not an engine Button:
        // Ui.hover/pressed intentionally only identify interactive widgets.
        // Forward the owner-scoped pointer inside the content rectangle so
        // the native Layer buttons can perform their own hit/click delivery.
        // The title/close chrome stays outside that rectangle.
        let inside_content = interaction.pointer.is_some_and(|[x, y]| {
            let window = &self.rt.as_ref().unwrap().host().windows[&modal.window];
            let [width, height] = window.inner.map(|v| v as f32 * modal.scale);
            x as f32 >= modal.offset[0]
                && (x as f32) < modal.offset[0] + width
                && y as f32 >= modal.offset[1]
                && (y as f32) < modal.offset[1] + height
        });
        if inside_content || interaction.pressed {
            frame.buttons_down |= interaction.pointer_buttons_down;
            frame.buttons_pressed |= interaction.pointer_buttons_pressed;
        }
        Ok(frame)
    }
}
