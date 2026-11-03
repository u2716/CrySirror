// SPDX-License-Identifier: MPL-2.0
// Adopted from siglus_rs 256c398 crates/reallive/src/modules/sel.rs (MPL-2.0).
//! Selections (module 0:2): the button-object forms. The `select` /
//! `select_s` elements stay pending for the adapter's shared UI choices.

use anyhow::Result;

use crate::vm::bytecode::Command;
use crate::vm::machine::{Machine, Next, SavepointKind};
use crate::vm::select::{ObjectButtonSelect, PolledButtons};

fn savepoint(machine: &mut Machine) {
    if machine.should_set_savepoint(SavepointKind::Selcom) {
        machine.mark_savepoint();
    }
}

pub fn dispatch(machine: &mut Machine, command: &Command) -> Result<Next> {
    match command.op.opcode {
        4 | 14 => {
            savepoint(machine);
            let group = machine.int_param_or(command, 0, 0)?;
            let op = ObjectButtonSelect::new(group, command.op.opcode == 14);
            machine.push_long_op(Box::new(op));
        }
        // objbtn_init: nothing to prepare.
        20 => {}
        // select_btnobjinitall / select_btnobjend
        21 | 23 => {
            if let Some(mut polled) = machine.sys.polled_buttons.take() {
                polled.finish(&mut machine.sys);
            }
        }
        // select_btnobjstart(group) / _with_right_click(group)
        22 | 122 => {
            let group = machine.int_param_or(command, 0, 0)?;
            if let Some(mut polled) = machine.sys.polled_buttons.take() {
                polled.finish(&mut machine.sys);
            }
            let mut polled = PolledButtons::new(group);
            polled.right_click = command.op.opcode == 122;
            polled.update(&mut machine.sys);
            machine.sys.polled_buttons = Some(polled);
        }
        // select_btnobjnow_hit / select_btnobjnow_decide
        30 => {
            machine.store = machine
                .sys
                .polled_buttons
                .as_ref()
                .map_or(-1, |p| p.hovered());
        }
        // select_btnobjnow_push (_left, _right) / _decide (_left, _right)
        31 | 33 | 35 => {
            let right = [None, Some(false), Some(true)][usize::from((command.op.opcode - 31) / 2)];
            machine.store = machine
                .sys
                .polled_buttons
                .as_ref()
                .map_or(-1, |p| p.pushed(right));
        }
        32 | 34 | 36 => {
            let right = [None, Some(false), Some(true)][usize::from((command.op.opcode - 32) / 2)];
            machine.store = machine
                .sys
                .polled_buttons
                .as_mut()
                .map_or(-1, |p| p.take_decided(right));
        }
        _ => return machine.request_command(command),
    }
    Ok(Next::Advance)
}
