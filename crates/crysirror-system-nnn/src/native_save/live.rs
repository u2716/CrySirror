// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native script/save producer fields, separated from file I/O and UI policy.
//! 53ad20 uses controller header word0 to choose adjusted call IDs for every
//! context. The pending PRINT/SELECT callback has already fetched its nextPC
//! in the original interpreter, while CrySirror keeps its receipt unconsumed.
use super::*;
use crate::{
    machine::{NativeCall, Payload},
    native_control::ScriptSession,
};
use anyhow::Context as _;

#[derive(Clone)]
struct Context {
    pc: u32,
    script_index: i32,
    depth: usize,
    calls: [u32; 255],
}
pub(crate) struct LiveExecution {
    contexts: Vec<Context>,
    adjusted: bool,
    variables: Vec<i32>,
}
impl LiveExecution {
    pub(crate) fn capture(
        session: &ScriptSession,
        configuration: &crate::native_bootstrap::NativeConfiguration,
        pending: Option<&NativeCall>,
    ) -> Result<Self> {
        let frames: Vec<_> = session.contexts_for_save().collect();
        ensure!(
            (1..=4).contains(&frames.len()) && frames[0].0 == b"sys",
            "NNN live save four-context ABI"
        );
        if let Some(call) = pending {
            session.machine().validate_receipt(call)?;
            ensure!(
                matches!(
                    call.payload,
                    Payload::Message { .. } | Payload::Selection { .. }
                ),
                "NNN save requested outside an original PRINT/SELECT boundary"
            );
        }
        let adjusted = frames[0].1.header_words >= 32;
        let mut contexts = Vec::with_capacity(frames.len());
        for (index, (name, program, machine)) in frames.iter().enumerate() {
            let pc = if index + 1 == frames.len() {
                pending.map_or(machine.pc(), |call| call.next)
            } else {
                machine.pc()
            };
            crate::control::target(program, pc)?;
            let returns = machine.return_frames();
            ensure!(returns.len() <= 249, "NNN native save return-stack budget");
            let mut calls = [0; 255];
            for (slot, frame) in returns.iter().enumerate() {
                crate::control::target(program, frame.pc)?;
                if adjusted {
                    ensure!(
                        continuation_next(program, 0, frame.native_id)? == frame.pc,
                        "NNN save CALL id does not match native return address"
                    );
                    calls[slot] = frame.native_id;
                } else {
                    calls[slot] = frame.pc;
                }
            }
            let continuation = machine.native_continuations();
            if adjusted && index + 1 < frames.len() {
                let native_id = continuation[0]
                    .context("NNN parent save lacks its original SCRIPT continuation")?;
                ensure!(
                    continuation_next(program, 3, native_id)? == pc,
                    "NNN parent save SCRIPT continuation mismatch"
                );
            }
            calls[253] = continuation[0].unwrap_or(0);
            calls[254] = continuation[1].unwrap_or(0);
            contexts.push(Context {
                pc,
                depth: returns.len(),
                calls,
                script_index: if index == 0 {
                    0
                } else {
                    i32::from(configuration.project.index(name).unwrap_or(0))
                },
            });
        }
        let variables = match pending {
            Some(call) => session.machine().native_variables_candidate(call)?,
            None => session
                .machine()
                .variables()
                .context("NNN save native variable bank absent")?
                .clone(),
        };
        ensure!(
            variables.stored_word_count() == 1000,
            "NNN original save variable-bank ABI"
        );
        Ok(Self {
            contexts,
            adjusted,
            variables: (0..1000).map(|i| variables.read_stored(i)).collect(),
        })
    }
}
impl NativeSaveFile {
    /// Replace only fields owned by the original script producer. Existing
    /// validated section bytes remain intact; this does not invent status,
    /// MINI-CG, message, effects, sound or extension producers.
    pub(crate) fn replace_live_execution(
        &mut self,
        state: &LiveExecution,
        game_mode: i32,
    ) -> Result<()> {
        ensure!(
            matches!(game_mode, 9 | 10),
            "NNN live native save mode requires its own actor producer"
        );
        let variables = self.blocks.get(4).context("NNN save VARIABLE absent")?;
        ensure!(
            variables.code == 4 && variables.payload.len() == 4096,
            "NNN live save VARIABLE ABI"
        );
        let status = self.blocks.get_mut(2).context("NNN save STATUS absent")?;
        ensure!(
            status.code == 2 && status.payload.len() == 7056,
            "NNN live save STATUS ABI"
        );
        let put = |status: &mut SaveBlock, native: usize, value: u32| {
            let at = (native - 8) * 4;
            status.payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        put(status, 20, game_mode as u32);
        put(status, 22, u32::from(state.adjusted));
        put(status, 24, (state.contexts.len() - 1) as u32);
        for (index, frame) in state.contexts.iter().enumerate() {
            put(status, 472 + index, frame.pc);
            if index > 0 {
                put(status, [0, 26, 27, 31][index], frame.script_index as u32);
            }
            let stack = 488 + 256 * index;
            put(status, stack, frame.depth as u32);
            for (at, value) in frame.calls.iter().enumerate() {
                put(status, stack + 1 + at, *value);
            }
        }
        self.replace_variables(&state.variables)
    }
}

impl NativeSaveFile {
    pub(crate) fn replace_live_wait(
        &mut self,
        wait: &crate::native_save_wait::SavedWait,
    ) -> Result<()> {
        use crate::native_save_wait::SavedWait;
        let (rows, fields) = match wait {
            SavedWait::Message {
                rows,
                mode,
                serial: _,
                window_number,
                window_visible,
                cannot_click,
            } => (
                rows,
                vec![
                    (29, rows.len() as i32),
                    (30, *mode as i32),
                    (180, *window_number),
                    (44, i32::from(*window_visible)),
                    (485, i32::from(*cannot_click)),
                ],
            ),
            SavedWait::Selection {
                rows,
                total,
                supplemental,
                time_limit,
                auto_select,
                special,
            } => {
                ensure!(
                    rows.len() == *total && *supplemental < *total,
                    "NNN original selection save rows"
                );
                (
                    rows,
                    vec![
                        (33, *total as i32),
                        (34, *supplemental as i32),
                        (480, *time_limit),
                        (481, *auto_select),
                        (482, *special),
                    ],
                )
            }
        };
        ensure!(
            rows.len() <= 64 && rows.iter().all(|row| row.len() <= 254 && !row.contains(&0)),
            "NNN original MESSAGE row ABI"
        );
        ensure!(
            self.blocks
                .get(2)
                .is_some_and(|block| block.code == 2 && block.payload.len() == 7056)
                && self
                    .blocks
                    .get(7)
                    .is_some_and(|block| block.code == 7 && block.payload.len() == 16384),
            "NNN original STATUS/MESSAGE ABI"
        );
        let mut message = vec![0; 16384];
        for (index, row) in rows.iter().enumerate() {
            message[index * 256..index * 256 + row.len()].copy_from_slice(row);
        }
        self.blocks[7].payload = message;
        for (native, value) in fields {
            self.replace_status_word(native, value)?;
        }
        Ok(())
    }
    pub(crate) fn replace_status_name(&mut self, native: usize, name: &[u8]) -> Result<()> {
        ensure!(
            matches!(native, 448 | 452) && name.len() <= 15 && !name.contains(&0),
            "NNN native player-name16byte ABI"
        );
        let status = self.blocks.get_mut(2).context("NNN native STATUS absent")?;
        ensure!(
            status.code == 2 && status.payload.len() == 7056,
            "NNN original STATUS ABI"
        );
        let at = (native - 8) * 4;
        status.payload[at..at + 16].fill(0);
        status.payload[at..at + name.len()].copy_from_slice(name);
        Ok(())
    }
    pub(crate) fn replace_status_word(&mut self, native: usize, value: i32) -> Result<()> {
        let status = self.blocks.get_mut(2).context("NNN native STATUS absent")?;
        ensure!(
            status.code == 2 && status.payload.len() == 7056 && (8..1772).contains(&native),
            "NNN original STATUS word bounds"
        );
        let at = (native - 8) * 4;
        status.payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
}
