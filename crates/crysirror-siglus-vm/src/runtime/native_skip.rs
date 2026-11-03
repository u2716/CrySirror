// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
// Native policy adopted from xmoezzz/siglus_rs commit
// 5f358ef1360b0439e92aed2b749e8f25d8721954,
// crates/siglus_scene_vm/src/runtime/mod.rs. See UPSTREAM.md and SOURCE.json.
//! Complete native read/automatic/trigger-skip policy group. Key stocks and
//! reveal/timeline mechanisms belong to Scene; only Siglus precedence lives here.
//! A consumed reveal request is published through engine CompleteReveal by message.
use super::{CommandContext, forms::codes::syscom_op};
impl CommandContext {
    pub(crate) fn runtime_ctrl_skip_flags(&self) -> (bool, bool) {
        // eng_frame.cpp suppresses Ctrl/Ctrl+Shift skip while an editbox owns
        // keyboard focus. Ctrl+Shift is message-only skip (`cs_skip_flag`).
        if self.globals.focused_editbox.is_some() {
            return (false, false);
        }
        let ctrl = self.input.vk_is_down(0x11);
        let shift = self.input.vk_is_down(0x10);
        (ctrl && !shift, ctrl && shift)
    }

    pub(crate) fn runtime_is_skip_unread_msg(&self) -> bool {
        let configured = self
            .globals
            .syscom
            .config_int
            .get(&syscom_op::GET_SKIP_UNREAD_MESSAGE_ONOFF)
            .copied()
            .unwrap_or({
                if self.globals.syscom.original_config.skip_unread_message_flag {
                    1
                } else {
                    0
                }
            })
            != 0;
        self.globals.script.skip_unread_message || configured
    }

    pub(crate) fn runtime_read_skip_is_enable(&self) -> bool {
        let f = self.globals.syscom.read_skip;
        f.enable && f.exist && (self.runtime_is_skip_unread_msg() || self.read_skip_enable_flag)
    }

    pub(crate) fn set_current_read_flag_for_skip(&mut self, scene_no: i64, flag_no: i64) {
        self.globals.script.cur_read_flag_scn_no = scene_no;
        self.globals.script.cur_read_flag_flag_no = flag_no;
        // eng_message.cpp updates against the previously committed value. The
        // new flag is only committed later by tnm_msg_proc_clear_ready().
        self.runtime_update_read_skip();
        self.runtime_update_start_skip();
    }

    pub(crate) fn runtime_update_read_skip(&mut self) {
        let scene_no = self.globals.script.cur_read_flag_scn_no;
        let flag_no = self.globals.script.cur_read_flag_flag_no;
        self.read_skip_enable_flag = self.globals.read_flag(scene_no, flag_no);
        if !self.read_skip_enable_flag && !self.runtime_is_skip_unread_msg() {
            self.globals.syscom.read_skip.onoff = false;
            self.cant_auto_skip_before_click = true;
        }
        if !self.runtime_read_skip_is_enable() {
            self.globals.syscom.read_skip.onoff = false;
        }
    }

    pub(crate) fn runtime_update_start_skip(&mut self) {
        if self.cant_auto_skip_before_click {
            return;
        }
        if self.globals.syscom.auto_skip.onoff
            && (self.read_skip_enable_flag || self.runtime_is_skip_unread_msg())
            && self.runtime_read_skip_is_enable()
        {
            self.globals.syscom.read_skip.onoff = true;
        }
    }

    pub(crate) fn runtime_update_read_skip_menu(&mut self) {
        if !self.runtime_read_skip_is_enable() {
            self.globals.syscom.read_skip.onoff = false;
        }
    }

    pub(crate) fn runtime_is_skipping(&self) -> bool {
        // Exact ordering from eng_etc.cpp::tnm_is_skipping().
        if self.globals.syscom.msg_back_open {
            return false;
        }
        let script = &self.globals.script;
        if script.ctrl_disable {
            return false;
        }
        let (ctrl_skip, _) = self.runtime_ctrl_skip_flags();
        if ctrl_skip {
            return true;
        }
        if script.skip_disable {
            return false;
        }
        self.skip_because_skip_trigger || self.globals.syscom.read_skip.onoff
    }

    pub(crate) fn runtime_is_skipping_msg(&self) -> bool {
        // Exact ordering from eng_etc.cpp::tnm_is_skipping_msg().
        if self.globals.syscom.msg_back_open {
            return false;
        }
        let script = &self.globals.script;
        if script.ctrl_disable {
            return false;
        }
        let (ctrl_skip, ctrl_shift_skip) = self.runtime_ctrl_skip_flags();
        if ctrl_skip || ctrl_shift_skip {
            return true;
        }
        if script.skip_disable {
            return false;
        }
        self.skip_because_skip_trigger || self.globals.syscom.read_skip.onoff
    }

    pub(crate) fn clear_script_trigger_skip(&mut self) {
        self.skip_because_skip_trigger = false;
    }

    pub(crate) fn register_skip_user_action(&mut self) {
        // eng_frame.cpp drops the auto-skip guard on DECIDE/CANCEL/Ctrl down.
        self.cant_auto_skip_before_click = false;
    }

    pub(crate) fn stop_read_skip_by_decide_down(&mut self, vk: u8) -> bool {
        self.register_skip_user_action();
        // eng_frame.cpp consumes DECIDE-DOWN when it stops read-skip. tona3's
        // BUTTON::use_down_stock() also clears down_up_stock, which is crucial:
        // the matching release must not advance MESSAGE_KEY_WAIT as a second
        // logical click.
        if self.globals.syscom.read_skip.onoff
            && !self.globals.script.not_stop_skip_by_click
            && self.input.use_vk_down_stock(vk)
        {
            self.globals.syscom.read_skip.onoff = false;
            self.cant_auto_skip_before_click = true;
            return true;
        }
        false
    }

    /// eng_frame.cpp script-trigger promotion for a DECIDE source. Returns true
    /// iff this input edge was consumed by the trigger logic.
    pub(crate) fn process_script_skip_trigger_decide(&mut self, vk: u8) -> bool {
        if self.excall_state.ex_call_flag || !self.globals.script.skip_trigger {
            return false;
        }

        if self.wait.message_reveal_waiting() && self.ui.message_waiting() {
            if self.globals.script.not_skip_msg_by_click || !self.input.use_vk_down_up_stock(vk) {
                return false;
            }
            self.msg_wait_skip_by_click = true;
            return true;
        }

        // Fully revealed text uses UP stock in the original, not DOWN_UP.
        // Consuming UP also invalidates the pair and prevents the same physical
        // release from immediately advancing MESSAGE_KEY_WAIT.
        if self.input.use_vk_up_stock(vk) {
            self.skip_because_skip_trigger = true;
            return true;
        }
        false
    }

    /// Wheel-down follows the same script-trigger state transition, but the
    /// platform event itself is already the consumable stock.
    #[allow(dead_code)]
    pub(crate) fn process_script_skip_trigger_wheel_down(&mut self) -> bool {
        if self.excall_state.ex_call_flag || !self.globals.script.skip_trigger {
            return false;
        }
        if self.wait.message_reveal_waiting() && self.ui.message_waiting() {
            if self.globals.script.not_skip_msg_by_click {
                return false;
            }
            self.msg_wait_skip_by_click = true;
            return true;
        }
        self.skip_because_skip_trigger = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::input::{VmKey, VmMouseButton};

    #[test]
    fn read_eligibility_observes_previous_commit_and_click_guard_before_auto_skip() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.globals.syscom.auto_skip.onoff = true;
        ctx.set_current_read_flag_for_skip(3, 7);
        assert!(!ctx.runtime_read_skip_is_enable());
        assert!(!ctx.globals.syscom.read_skip.onoff);
        ctx.globals.set_read_flag(3, 7);
        // A newly committed flag alone must not clear the unread-click guard.
        ctx.set_current_read_flag_for_skip(3, 7);
        assert!(ctx.runtime_read_skip_is_enable());
        assert!(!ctx.globals.syscom.read_skip.onoff);
        ctx.register_skip_user_action();
        ctx.set_current_read_flag_for_skip(3, 7);
        assert!(ctx.globals.syscom.read_skip.onoff);
        ctx.globals.syscom.read_skip.exist = false;
        ctx.runtime_update_read_skip_menu();
        assert!(!ctx.globals.syscom.read_skip.onoff);
    }

    #[test]
    fn decide_stopping_read_skip_consumes_pair_and_native_flag_can_preserve_it() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.globals.syscom.read_skip.onoff = true;
        ctx.input.on_mouse_down(VmMouseButton::Left);
        assert!(ctx.stop_read_skip_by_decide_down(1));
        ctx.input.next_frame();
        ctx.input.on_mouse_up(VmMouseButton::Left);
        assert!(!ctx.input.vk_down_up_stock(1));
        assert!(!ctx.globals.syscom.read_skip.onoff);
        ctx.globals.syscom.read_skip.onoff = true;
        ctx.globals.script.not_stop_skip_by_click = true;
        ctx.input.on_key_down(VmKey::Enter);
        assert!(!ctx.stop_read_skip_by_decide_down(0x0d));
        ctx.input.on_key_up(VmKey::Enter);
        assert!(ctx.input.vk_down_up_stock(0x0d));
        assert!(ctx.globals.syscom.read_skip.onoff);
    }

    #[test]
    fn local_armed_trigger_is_distinct_from_promoted_global_and_excall_cannot_promote() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.globals.script.skip_trigger = true;
        assert!(!ctx.runtime_is_skipping());
        ctx.input.on_key_down(VmKey::Enter);
        ctx.input.on_key_up(VmKey::Enter);
        ctx.excall_state.ex_call_flag = true;
        assert!(!ctx.process_script_skip_trigger_decide(0x0d));
        ctx.excall_state.ex_call_flag = false;
        assert!(ctx.process_script_skip_trigger_decide(0x0d));
        assert!(ctx.runtime_is_skipping());
        assert!(!ctx.input.vk_down_up_stock(0x0d));
        ctx.globals.script.skip_trigger = false;
        assert!(ctx.runtime_is_skipping());
        ctx.clear_script_trigger_skip();
        assert!(!ctx.runtime_is_skipping());
    }

    #[test]
    fn ctrl_shift_is_message_only_while_backlog_focus_and_disable_keep_original_precedence() {
        let mut ctx = CommandContext::new("asset".into());
        ctx.input.on_key_down(VmKey::Control);
        ctx.input.on_key_down(VmKey::Shift);
        ctx.globals.script.skip_disable = true;
        assert!(ctx.runtime_is_skipping_msg());
        assert!(!ctx.runtime_is_skipping());
        ctx.globals.focused_editbox = Some((1, 0));
        assert!(!ctx.runtime_is_skipping_msg());
        ctx.globals.focused_editbox = None;
        ctx.globals.script.ctrl_disable = true;
        assert!(!ctx.runtime_is_skipping_msg());
        ctx.globals.script.ctrl_disable = false;
        ctx.globals.syscom.msg_back_open = true;
        assert!(!ctx.runtime_is_skipping_msg());
    }
}
