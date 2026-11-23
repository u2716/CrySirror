// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original message/history/window triggers, separate from engine text layout.
use crate::native_skip::NativeSkipPolicy;
use anyhow::{Context, Result, ensure};
/// Original SystemMenu actions affecting the suspended foreground actor.
/// They are applied after its real context is restored, never to a modal
/// background reader or by acknowledging its retained PRINT/SELECT receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SystemMessageAction { HideWindow, Skip, Automatic }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeMessageControl {
    /// No original saved/read-message ID has been observed yet.
    pub last_message_id: Option<i32>,
    pub next_type: i32,
    pub next_window: i32,
    pub window_visible: bool,
    pub has_voice: bool,
}
impl Default for NativeMessageControl {
    fn default() -> Self {
        Self {
            last_message_id: None,
            next_type: 3,
            next_window: -1,
            window_visible: false,
            has_voice: false,
        }
    }
}
impl NativeMessageControl {
    /// Legacy 0x530f80: negative IDs preserve the previous ID. Unrecognised
    /// mode values preserve the previous mode, rather than forcing automatic.
    pub fn start_message(&mut self, values: &[i32], skip: &NativeSkipPolicy) -> Result<()> {
        self.start_message_with_history(values, &mut skip.clone(), None)
    }
    /// Actual530f80 reads the current source-reader's SystemFile bank. Stage
    /// both records before acknowledging the callback; a missing history
    /// capability never silently converts read-only skip into skip-all.
    pub(crate) fn start_message_with_history(
        &mut self,
        values: &[i32],
        skip: &mut NativeSkipPolicy,
        read: Option<bool>,
    ) -> Result<()> {
        ensure!(values.len() <= 256, "NNN STARTMESSAGE parameter budget");
        let unread = if values.first().is_some_and(|id| *id != -1) && skip.next_command && skip.f4 {
            !read.context("NNN read-only F4 skip requires original message-read history")?
        } else {
            false
        };
        if unread {
            skip.next_command = false;
            skip.effect_command = false;
        }
        if let Some(&id) = values.first() {
            if id >= 0 {
                self.last_message_id = Some(id);
            }
            match values.get(1).copied().unwrap_or(0) {
                0 => self.next_window = -1,
                1 => self.next_window = 0,
                2 => self.next_window = 1,
                _ => {}
            }
        }
        self.has_voice = false;
        Ok(())
    }
    /// Legacy 0x531080. No shared glyph/window raster changes occur here;
    /// the following original Print/Draw consumes next-window policy.
    pub fn start_koma(&mut self, values: &[i32], skip: &NativeSkipPolicy) -> Result<()> {
        self.start_koma_with_history(values, &mut skip.clone(), None)
    }
    /// Actual531080 stops only effect skipping at an unread frame. PRINT is
    /// still skipped until STARTMESSAGE observes its own unread boundary.
    pub(crate) fn start_koma_with_history(
        &mut self,
        values: &[i32],
        skip: &mut NativeSkipPolicy,
        read: Option<bool>,
    ) -> Result<()> {
        ensure!(values.len() <= 256, "NNN STARTKOMA parameter budget");
        let unread = if values.first().is_some_and(|id| *id != -1)
            && skip.next_command
            && skip.f4
            && skip.effect_command
        {
            !read.context("NNN F4 frame skip requires original message-read history")?
        } else {
            false
        };
        if unread {
            skip.effect_command = false;
        }
        if let Some(&id) = values.first() {
            self.next_type = values.get(1).copied().unwrap_or(3);
            self.next_window = values.get(2).copied().unwrap_or(-1);
            self.window_visible = self.next_type == 0;
            match self.next_window {
                1 => self.window_visible = false,
                2 => self.window_visible = true,
                _ => {}
            }
        }
        Ok(())
    }
    /// Legacy 0x52fc80; any mode other than -1 is consumed, but only 0/1
    /// override the flag that the original Print/LPrint/Append set first.
    pub fn before_print(&mut self, default_visible: bool) -> bool {
        self.window_visible = match self.next_window {
            0 => false,
            1 => true,
            _ => default_visible,
        };
        self.next_window = -1;
        self.window_visible
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_message_ids_modes_and_voice_reset_preserve_native_edges() {
        let skip = NativeSkipPolicy::default();
        let mut p = NativeMessageControl::default();
        p.has_voice = true;
        p.start_message(&[12, 2], &skip).unwrap();
        assert_eq!(p.last_message_id, Some(12));
        assert!(!p.has_voice);
        assert_eq!(p.next_window, 1);
        p.start_message(&[-1, 99], &skip).unwrap();
        assert_eq!(p.last_message_id, Some(12));
        assert_eq!(p.next_window, 1);
        assert!(p.before_print(false));
        assert_eq!(p.next_window, -1);
        assert!(!p.before_print(false));
        p.start_koma(&[4, 3, 0], &skip).unwrap();
        assert!(!p.before_print(true));
        p.start_koma(&[-1, 0, 2], &skip).unwrap();
        assert!(p.window_visible);
        assert!(p.before_print(true));
        let mut skip = skip;
        skip.f4 = true;
        skip.next_command = true;
        let before = p.clone();
        assert!(p.start_message(&[17], &skip).is_err());
        assert_eq!(p, before);
        p.start_message(&[-1], &skip).unwrap();
    }
}
