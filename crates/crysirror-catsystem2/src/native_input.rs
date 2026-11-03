// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original KEYCUSTOMIZE/KEY and FES input-query policy. Device observation,
//! physical-button identity and delivery ownership belong to CrySirror.
use anyhow::{Context, Result, ensure};
use crysirror_core::service::{InputObservation, buttons};
use crysirror_resource::xml::XmlDocument;

#[derive(Clone, Debug, Default)]
pub(crate) struct Slot {
    enabled: bool,
    keyboard: [u8; 4],
    mouse: [u8; 4],
    joypad: [u8; 4],
    touch: [u8; 4],
    repeat_keyboard: [u8; 4],
    repeat_mouse: [u8; 4],
    repeat_joypad: [u8; 4],
    repeat_touch: [u8; 4],
}
#[derive(Clone, Debug)]
pub(crate) struct Bindings {
    slots: Vec<Slot>,
    joypad_enabled: u32,
    /// INPUT this[1151..1231] (byte 4604): 5E66C0 polls a slot only while its
    /// flag is nonzero. 5E6270 starts every slot at 1; INPUT messages 150/151 write it.
    pub polled: Vec<bool>,
}
impl Default for Bindings {
    fn default() -> Self {
        Self {
            slots: vec![Slot::default(); 80],
            joypad_enabled: 0,
            polled: vec![true; 80],
        }
    }
}
fn number(d: &XmlDocument, path: &str, fallback: u32) -> Result<u32> {
    Ok(d.unique_child_path(path)?
        .map_or(fallback, |n| crate::native_values::integer(&n.text)))
}
impl Bindings {
    pub fn startup(d: &XmlDocument) -> Result<Self> {
        let mut out = Self::default();
        for (i, s) in out.slots.iter_mut().enumerate() {
            let base = format!("KEYCUSTOMIZE/key{i:02}");
            s.enabled = d
                .unique_child_path(&format!("{base}/name"))?
                .is_some_and(|n| !n.text.is_empty());
            for p in 0..4 {
                for (field, dst) in [
                    ("keyboard", &mut s.keyboard),
                    ("mouse", &mut s.mouse),
                    ("joypad", &mut s.joypad),
                    ("touch", &mut s.touch),
                    ("rep_keyboard", &mut s.repeat_keyboard),
                    ("rep_mouse", &mut s.repeat_mouse),
                    ("rep_joypad", &mut s.repeat_joypad),
                    ("rep_touch", &mut s.repeat_touch),
                ] {
                    //528710 copies only the native low byte into each player
                    //source, after71EC50's wrapping XML integer conversion.
                    dst[p] = number(d, &format!("{base}/{field}{}", p + 1), 0)? as u8;
                }
            }
        }
        Ok(out)
    }
    ///720300 changes only enabled rows and preserves absent fields. Touch
    ///customization is not present in the reviewed persisted-source routine.
    pub fn persisted(&mut self, d: &XmlDocument) -> Result<()> {
        self.joypad_enabled = number(d, "KEY/enable_joypad", self.joypad_enabled)?;
        for (i, s) in self.slots.iter_mut().enumerate().filter(|(_, s)| s.enabled) {
            for p in 0..4 {
                for (field, dst) in [
                    ("kb", &mut s.keyboard),
                    ("ms", &mut s.mouse),
                    ("jp", &mut s.joypad),
                    ("kbr", &mut s.repeat_keyboard),
                    ("msr", &mut s.repeat_mouse),
                    ("jpr", &mut s.repeat_joypad),
                ] {
                    let key = if field.ends_with('r') {
                        format!("KEY/{}{}r_{i:02}", &field[..2], p + 1)
                    } else {
                        format!("KEY/{field}{}_{i:02}", p + 1)
                    };
                    dst[p] = number(d, &key, u32::from(dst[p]))? as u8;
                }
            }
        }
        Ok(())
    }
    ///70F740 + INPUT144: source masks are ORed across the four configured
    ///players. This is a held-state query, not repeat/callback dispatch.
    /// Logical keys a physical button drives, in slot order: scripts receive these
    /// KEYCUSTOMIZE indices (0 decide, 1 cancel, 2 advance text), not virtual keys.
    pub fn slots_for(&self, mask: u64) -> Vec<u32> {
        self.slots.iter().enumerate().filter(|(index, slot)| {
            self.polled.get(*index).copied().unwrap_or(false) && slot.enabled && (0..4).any(|p| {
                let keyboard = slot.keyboard[p] != 0 && buttons::windows_virtual_key(u32::from(slot.keyboard[p])) == Some(mask);
                let mouse = match slot.mouse[p] {
                    1 => mask == buttons::MOUSE_LEFT,
                    2 => mask == buttons::MOUSE_MIDDLE,
                    3 => mask == buttons::MOUSE_RIGHT,
                    6 => mask == buttons::WHEEL_UP,
                    7 => mask == buttons::WHEEL_DOWN,
                    _ => false,
                };
                keyboard || mouse
            })
        }).map(|(index, _)| index as u32).collect()
    }
    pub fn held_sources(&self, index: usize, observation: &InputObservation) -> Result<u32> {
        let s = self
            .slots
            .get(index)
            .context("CatSystem2 native input slot extent")?;
        if !s.enabled || !self.polled.get(index).copied().unwrap_or(false) {
            return Ok(0);
        }
        let mut value = 0;
        for p in 0..4 {
            if s.keyboard[p] != 0 {
                let mask =
                    buttons::windows_virtual_key(u32::from(s.keyboard[p])).with_context(|| {
                        format!(
                            "CatSystem2 native slot{index}/player{p} requires unrepresented VK{}",
                            s.keyboard[p]
                        )
                    })?;
                if observation.buttons_down & mask != 0 {
                    value |= 1;
                }
            }
            if s.mouse[p] != 0 {
                let mask = match s.mouse[p] {
                    1 => buttons::MOUSE_LEFT,
                    2 => buttons::MOUSE_MIDDLE,
                    3 => buttons::MOUSE_RIGHT,
                    6 => buttons::WHEEL_UP,
                    7 => buttons::WHEEL_DOWN,
                    code => anyhow::bail!(
                        "CatSystem2 native mouse source{code} requires a reviewed mapping"
                    ),
                };
                if (observation.buttons_down | observation.buttons_pressed) & mask != 0 {
                    value |= 2;
                }
            }
            ensure!(
                s.joypad[p] == 0,
                "CatSystem2 native logical-key gamepad observation unsupported"
            );
            ensure!(
                s.touch[p] == 0,
                "CatSystem2 native logical-key touch observation unsupported"
            );
        }
        Ok(value)
    }
}
#[derive(Clone, Debug, Default)]
pub(crate) struct InputState {
    pub available: bool,
    pub revision: u64,
    pub position: Option<[i32; 2]>,
    /// INPUT this[1238]: source bits of the last key/button event 5E7A30
    /// offered (keyboard 1, mouse 2, joypad 4; touch 16 reads back as 2).
    pub last_source: u32,
}
impl InputState {
    ///7106F0's inside-or-retained-foreground rule differs from a focus flag.
    ///Unknown backend metadata never creates coordinates or an active pointer.
    pub fn observe(&mut self, o: &InputObservation) -> Result<()> {
        // A KCS task's retained tick may finish after another task saw a newer sample.
        if o.revision <= self.revision {
            return Ok(());
        }
        self.revision = o.revision;
        if let Some(point) = o.pointer {
            self.position = Some(point);
        }
        if o.pointer_inside == Some(true) && o.pointer.is_some() {
            self.available = true;
        } else if o.focused == Some(false) && o.pointer_inside == Some(false) {
            self.available = false;
        }
        Ok(())
    }
    pub fn fes_position(&self) -> [i32; 2] {
        if self.available {
            self.position.unwrap_or([i32::MIN; 2])
        } else {
            [i32::MIN; 2]
        }
    }
}

/// 5E8B36's message for one InputTask callback on a Main-bound button: `[from,
/// kind, button, payload]`. Press/Release carry the Windows virtual key that
/// 7106F0's callback reports; hover carries entered/left. Move, Drag and Repeat
/// reach no Main message in the original.
pub(crate) fn main_event_message(
    button: u32,
    event: &crysirror_scene::input_routes::InputRouteEvent,
) -> Option<[u32; 4]> {
    use crysirror_scene::input_routes::InputRoutePhase as Phase;
    let (kind, payload) = match event.phase {
        Phase::Press => (-65533i32, virtual_key(event.code)?),
        Phase::Release => (-65532, virtual_key(event.code)?),
        Phase::HoverEnter => (-65531, 1),
        Phase::HoverLeave => (-65531, 0),
        Phase::Repeat | Phase::Drag | Phase::Move => return None,
    };
    Some([button, kind as u32, button, payload])
}
/// Every message one physical event sends a KCS task. Hover has one fixed payload;
/// presses and releases go out once per logical key the button is bound to.
pub(crate) fn script_messages(button: u32, event: &crysirror_scene::input_routes::InputRouteEvent, bindings: &Bindings) -> Vec<[u32; 4]> {
    use crysirror_scene::input_routes::InputRoutePhase as Phase;
    let kind = match event.phase {
        Phase::Press => -65533i32,
        Phase::Release => -65532,
        Phase::HoverEnter => return vec![[button, -65531i32 as u32, button, 1]],
        Phase::HoverLeave => return vec![[button, -65531i32 as u32, button, 0]],
        Phase::Repeat | Phase::Drag | Phase::Move => return Vec::new(),
    };
    let Some(mask) = 1u64.checked_shl(event.code) else { return Vec::new() };
    bindings.slots_for(mask).into_iter().map(|slot| [button, kind as u32, button, slot]).collect()
}
fn virtual_key(code: u32) -> Option<u32> {
    let bit = 1u64.checked_shl(code)?;
    (0u32..256).find(|vk| crysirror_core::service::buttons::windows_virtual_key(*vk) == Some(bit))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(revision: u64) -> InputObservation {
        InputObservation {
            client: crysirror_core::state::ClientId(1),
            revision,
            time_ms: 16 * revision,
            buttons_down: 0,
            buttons_pressed: 0,
            pointer: None,
            pointer_inside: None,
            focused: None,
        }
    }
    #[test]
    fn original_configured_slot_is_not_a_core_button_number_and_setup_preserves_missing_fields() {
        let xml=XmlDocument::parse(b"<document><KEYCUSTOMIZE><key12><name>skip</name><keyboard1>273</keyboard1><keyboard2>0</keyboard2></key12></KEYCUSTOMIZE></document>").unwrap();
        let mut bindings = Bindings::startup(&xml).unwrap();
        let mut sample = observation(1);
        sample.buttons_down = buttons::CTRL;
        assert_eq!(bindings.held_sources(12, &sample).unwrap(), 1);
        assert_eq!(bindings.held_sources(13, &sample).unwrap(), 0);
        let setup =
            XmlDocument::parse(b"<document><KEY><kb1_12>32</kb1_12></KEY></document>").unwrap();
        bindings.persisted(&setup).unwrap();
        assert_eq!(bindings.held_sources(12, &sample).unwrap(), 0);
        sample.buttons_down = buttons::SPACE;
        assert_eq!(bindings.held_sources(12, &sample).unwrap(), 1);
        bindings
            .persisted(&XmlDocument::parse(b"<document/>").unwrap())
            .unwrap();
        assert_eq!(bindings.held_sources(12, &sample).unwrap(), 1);
        bindings.slots[12].keyboard[0] = 104;
        assert!(bindings.held_sources(12, &sample).is_err());
    }
    #[test]
    fn native_availability_distinguishes_unknown_focus_outside_and_reported_origin() {
        let mut state = InputState::default();
        let mut o = observation(1);
        state.observe(&o).unwrap();
        assert_eq!(state.fes_position(), [i32::MIN; 2]);
        o.revision = 2;
        o.pointer = Some([0, 0]);
        o.pointer_inside = Some(true);
        o.focused = Some(true);
        state.observe(&o).unwrap();
        assert_eq!(state.fes_position(), [0, 0]);
        o.revision = 3;
        o.pointer = Some([-5, 7]);
        o.pointer_inside = Some(false);
        state.observe(&o).unwrap();
        assert_eq!(state.fes_position(), [-5, 7]);
        o.revision = 4;
        o.focused = Some(false);
        state.observe(&o).unwrap();
        assert_eq!(state.fes_position(), [i32::MIN; 2]);
        assert!(state.observe(&observation(0)).is_ok()); // an older retained sample is ignored
        o.revision = 5;
        o.pointer = Some([7, 8]);
        o.pointer_inside = Some(true);
        state.observe(&o).unwrap();
        assert_eq!(state.fes_position(), [7, 8]);
    }
    #[test]
    fn main_event_messages_carry_the_original_virtual_key_and_ignore_moves() {
        use crysirror_scene::input_routes::{InputRouteEvent, InputRoutePhase, InputRouteSource};
        let event = |phase, code| InputRouteEvent { source: InputRouteSource::Pointer, code, phase, point: [0, 0], origin: None };
        assert_eq!(main_event_message(5, &event(InputRoutePhase::Press, 16)), Some([5, -65533i32 as u32, 5, 1]));
        assert_eq!(main_event_message(5, &event(InputRoutePhase::Release, 16)), Some([5, -65532i32 as u32, 5, 1]));
        assert_eq!(main_event_message(5, &event(InputRoutePhase::HoverLeave, 0)), Some([5, -65531i32 as u32, 5, 0]));
        assert_eq!(main_event_message(5, &event(InputRoutePhase::Move, 0)), None);
    }
}
