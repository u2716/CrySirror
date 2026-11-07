// Copyright (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. Native ABI: xsystem4 a85467d src/vm.c::system_call.
//! Original modal/wait/stack policy. Shared UI owns layout, text and input.
use crate::{
    adapter::{Bridge, Dispatch},
    vm::{ExternalCall, ExternalTarget, Machine},
};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    service::{EngineQuery, HostServices},
    state::ClientId,
};
use serde::Deserialize;
const DIALOG_ID: u32 = 62464;
#[derive(Deserialize)]
struct Widget {
    owner: u32,
    id: u32,
}
#[derive(Deserialize)]
struct Event {
    widget: Widget,
    kind: String,
    value: f32,
}
struct Pending {
    call: ExternalCall,
    code: i32,
    choice: Option<i32>,
}
pub(crate) struct NativeDialog {
    owner: ClientId,
    pending: Option<Pending>,
}
impl NativeDialog {
    pub(crate) fn new(owner: ClientId) -> Self {
        Self {
            owner,
            pending: None,
        }
    }
    pub(crate) fn active(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn close(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if self.pending.as_ref().is_some_and(|p| p.choice.is_none()) {
            host.submit_engine(
                self.owner,
                1,
                &serde_json::to_vec(
                    &serde_json::json!({"ui":[{"op":"close_dialog","id":DIALOG_ID}]}),
                )?,
            )?;
            self.pending = None;
        }
        self.pending = None;
        Ok(())
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::System(code) = call.target else {
            ensure!(
                self.pending.is_none(),
                "System4 changed modal syscall target"
            );
            return Ok(None);
        };
        if !matches!(code, 7 | 18 | 15) {
            ensure!(self.pending.is_none(), "System4 changed modal syscall");
            return Ok(None);
        }
        ensure!(owner == self.owner, "System4 foreign modal owner");
        machine.validate_system(call)?;
        if self.pending.is_none() {
            if code == 18 {
                machine.validate_system_owned_strings(call, 1)?;
            }
            let raw = machine.heap.string(machine.stack_value(0)?)?;
            let raw = raw.split(|b| *b == 0).next().unwrap();
            let message = crysirror_resource::text_stream::decode_shift_jis(raw, 16384)?;
            ensure!(message.len() <= 16384, "System4 modal text budget");
            let sact = bridge
                .sact
                .as_ref()
                .context("System4 modal font configuration unavailable")?;
            let fonts = host.font_library(owner)?;
            let font = fonts
                .resolve_alias(&sact.font_aliases[0])?
                .context("System4 modal font not published")?;
            ensure!(font.len() <= 128, "System4 modal font alias limit");
            let buttons = match code {
                7 => serde_json::json!([{"id":1,"label":"OK"}]),
                18 => serde_json::json!([{"id":1,"label":"OK"},{"id":0,"label":"Cancel"}]),
                _ => serde_json::json!([{"id":1,"label":"Stop"},{"id":0,"label":"Continue"}]),
            };
            let cancel = match code {
                7 => Some(1),
                18 => Some(0),
                _ => None,
            };
            if code == 15 {
                host.append_log(owner, &format!("*GAME ERROR*: {message}"))?;
            }
            let title = if code == 15 {
                "Game Error"
            } else {
                "CrySirror"
            };
            // No source string is consumed until the actual UI result. A
            // missing UI/font capability retains the original native receipt.
            host.submit_engine(owner,1,&serde_json::to_vec(&serde_json::json!({"ui":[{"op":"open_dialog","dialog":{"id":DIALOG_ID,"title":title,"message":message,"buttons":buttons,"default":1,"cancel":cancel,"font":font,"text_size":22}}]}))?)?;
            self.pending = Some(Pending {
                call: call.clone(),
                code,
                choice: None,
            });
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(
            pending.call == *call && pending.code == code,
            "System4 stale modal completion"
        );
        if pending.choice.is_none() {
            let bytes = host.query_engine(owner, EngineQuery::DialogEvents)?;
            ensure!(bytes.len() <= 64 * 1024, "System4 modal event byte budget");
            let events: Vec<Event> = serde_json::from_slice(&bytes)?;
            ensure!(events.len() <= 32, "System4 modal event count budget");
            for event in events {
                ensure!(
                    event.widget.owner == owner.0
                        && event.widget.id == DIALOG_ID
                        && event.kind == "dialog_result",
                    "System4 foreign modal event"
                );
                ensure!(
                    event.value == 0. || event.value == 1.,
                    "System4 modal result ABI"
                );
                ensure!(pending.choice.is_none(), "System4 duplicate modal result");
                if code == 7 {
                    ensure!(event.value == 1., "System4 informational modal result");
                }
                pending.choice = Some(event.value as i32);
            }
        }
        let Some(choice) = pending.choice else {
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        };
        match (code, choice) {
            (7, _) | (15, 0) => machine.complete_system(call, 0, None)?, // original caller S_POPs
            (18, _) => machine.complete_system_owned_string(call, Some(choice))?,
            (15, 1) => machine.complete_system_exit(call, 0, 1)?,
            _ => bail!("System4 unsupported modal completion"),
        }
        self.pending = None;
        Ok(Some(Dispatch::Completed))
    }
}
