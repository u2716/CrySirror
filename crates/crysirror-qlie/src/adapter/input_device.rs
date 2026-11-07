//! Input-device state behind inp methods (TImoInput 5C60B4..5C88D8).
//!
//! - `i[,ms]` (5C765C) clears every key/button state and suppresses device
//!   sampling for `ms` (default 250); mouse buttons held across the block
//!   must be released before they press again (5C6AFC +75/+77/+79).
//! - `sk,code` (5C86FC) posts synthetic keyboard/wheel events: modifier bits
//!   press Ctrl/Alt/Shift (never released, as natively), mode 0 presses,
//!   1 releases, 2/3/4 tap; the events arrive with the next frame.
//! - `ms,v` (5C7DB8) and MouseHideTime (5C87EC): the cursor is shown while
//!   the flag is set and the pointer has moved within MouseHideTime ms.
use super::*;
use crysirror_core::service::buttons as B;

#[derive(Default)]
pub(super) struct InputDevice {
    block_until: u64,
    release_wait: u64,
    synthetic_down: u64,
    pending_pulses: u64,
    frame_pulses: u64,
    cursor_flag: Option<bool>,
    idle_ms: u64,
    last_pointer: Option<[i32; 2]>,
    cursor_visible: Option<bool>,
}
const MOUSE: u64 = B::MOUSE_LEFT | B::MOUSE_RIGHT | B::MOUSE_MIDDLE;

impl QlieVm {
    /// 5C765C: clear key state and suppress input for `ms`.
    pub(super) fn block_input(&mut self, now: u64, ms: i32) {
        let d = &mut self.input_device;
        d.block_until = now.saturating_add(ms.max(0) as u64);
        d.release_wait = MOUSE;
        d.idle_ms = 0;
        self.input.buttons_down = 0;
        self.input.buttons_pressed = 0;
        self.keys = Default::default();
        self.swipe.reset_latch();
    }
    /// Applies the device filters to the frame the host delivered.
    pub(super) fn filter_input(&mut self, now: u64, new_tick: bool) {
        let d = &mut self.input_device;
        if new_tick {
            d.frame_pulses = std::mem::take(&mut d.pending_pulses);
        }
        self.input.buttons_down |= d.synthetic_down;
        self.input.buttons_pressed |= d.frame_pulses;
        if now < d.block_until {
            self.input.buttons_down = 0;
            self.input.buttons_pressed = 0;
            return;
        }
        d.release_wait &= self.input.buttons_down;
        self.input.buttons_down &= !d.release_wait;
        self.input.buttons_pressed &= !d.release_wait;
    }
    /// 5C87EC once per frame; submits only changes.
    pub(super) fn advance_cursor(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let hide = self.engine_setting("mht");
        let d = &mut self.input_device;
        let pointer = [self.input.pointer_x, self.input.pointer_y];
        if d.last_pointer != Some(pointer) {
            d.idle_ms = 0;
            d.last_pointer = Some(pointer);
        } else {
            d.idle_ms = d.idle_ms.saturating_add(self.tick_ms);
        }
        let shown = d.cursor_flag.unwrap_or(true);
        let visible = shown && !(hide > 0 && d.idle_ms >= hide as u64);
        if d.cursor_visible != Some(visible) {
            d.cursor_visible = Some(visible);
            self.submit(h, json!({"ui":[{"op":"set_cursor_visible","visible":visible}]}))?;
        }
        Ok(())
    }
    /// Returns false when `op` is not one of these inp methods.
    pub(super) fn input_device_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let n = |i: usize| -> Result<i32> {
            a.get(i).context("QLIE input argument")?.integer()
        };
        match op {
            "i" => {
                ensure!(a.len() <= 1, "QLIE input reset ABI");
                let ms = if a.is_empty() { 250 } else { n(0)? };
                self.block_input(h.now_ms(), ms);
            }
            "sk" => {
                if a.is_empty() {
                    return Ok(true);
                }
                let code = n(0)?;
                self.synthetic_key(code)?;
            }
            "ms" => {
                ensure!(a.len() == 1, "QLIE mouse cursor flag ABI");
                let shown = n(0)? == 1;
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert("ms".into(), vec![Value::Int(i32::from(shown))]);
                let d = &mut self.input_device;
                d.cursor_flag = Some(shown);
                if shown {
                    d.idle_ms = 0;
                }
                self.advance_cursor_now(h)?;
            }
            "?hit" => self.input_hit_query(h, m, a)?,
            "bs" => {
                ensure!(a.len() == 1, "QLIE button select ABI");
                self.button_select(h, &a[0].text())?;
            }
            // +174/+178 store two script values; ?bsv returns them.
            "bsv" => {
                ensure!(a.len() == 2, "QLIE button-select values ABI");
                let values = vec![Value::Int(n(0)?), Value::Int(n(1)?)];
                self.objects
                    .get_mut(id)
                    .unwrap()
                    .properties
                    .insert("bsv".into(), values);
            }
            "?bsv" | "?swe" => {
                ensure!(a.is_empty(), "QLIE input pair query ABI");
                let values = self.objects[id]
                    .properties
                    .get(&op[1..])
                    .cloned()
                    .unwrap_or_default();
                // swe without an inertia argument stores 1 (6F98D0).
                let defaults = if op == "?swe" { [0, 1] } else { [0, 0] };
                for (i, default) in defaults.into_iter().enumerate() {
                    m.result(i, values.get(i).cloned().unwrap_or(Value::Int(default)));
                }
            }
            // mp/smp,x,y move the host pointer (5C7CE8 SetCursorPos); without
            // coordinates they answer the pointer position.
            "mp" | "smp" => {
                if a.is_empty() {
                    m.result(0, Value::Int(self.input.pointer_x));
                    m.result(1, Value::Int(self.input.pointer_y));
                } else {
                    ensure!(a.len() == 2, "QLIE pointer move ABI");
                    let position = [n(0)?, n(1)?];
                    self.submit(h, json!({"ui":[{"op":"warp_pointer","position":position}]}))?;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    fn advance_cursor_now(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let saved = self.tick_ms;
        self.tick_ms = 0;
        let result = self.advance_cursor(h);
        self.tick_ms = saved;
        result
    }
    fn synthetic_key(&mut self, code: i32) -> Result<()> {
        for (bit, key) in [(0x1000, 0x11), (0x2000, 0x12), (0x4000, 0x10)] {
            if code & bit != 0 {
                self.synthetic_key(key)?;
            }
        }
        let key = code & 0xFFF;
        let mask = objects::native_key(key)?;
        let d = &mut self.input_device;
        if matches!(key, 0xF0 | 0xF1) {
            d.pending_pulses |= mask;
            return Ok(());
        }
        match code & 0xF0000 {
            0x00000 => {
                d.synthetic_down |= mask;
                d.pending_pulses |= mask;
            }
            0x10000 => d.synthetic_down &= !mask,
            0x20000 | 0x30000 | 0x40000 => {
                d.synthetic_down &= !mask;
                d.pending_pulses |= mask;
            }
            _ => {}
        }
        Ok(())
    }
}
