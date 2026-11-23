// SPDX-License-Identifier: MPL-2.0
//! Full-canvas native fades over shared external-clock Layer motion.

use crate::vm::{CmvsVm, FadeCommand};
use anyhow::Result;
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::json;

pub(super) const LAYER: u32 = 2;

#[derive(Clone, Copy)]
pub(super) struct Fade {
    reveal: bool,
    duration: u32,
    elapsed: u32,
    complete: bool,
}

impl Fade {
    pub fn advance(
        active: &mut Option<Self>,
        host: &mut dyn HostServices,
        client: ClientId,
        delta: u32,
    ) -> Result<()> {
        let Some(fade) = *active else {
            return Ok(());
        };
        if fade.complete {
            return Ok(());
        }
        let elapsed = fade.elapsed.wrapping_add(delta);
        if elapsed >= fade.duration {
            Self::finish(active, host, client)?;
        } else {
            submit(
                host,
                client,
                json!([{"op":"set_motion_elapsed","id":LAYER,
                "property":"opacity","elapsed":elapsed}]),
            )?;
            *active = Some(Self { elapsed, ..fade });
        }
        Ok(())
    }

    pub fn apply(
        active: &mut Option<Self>,
        host: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
        vm: &mut CmvsVm,
        allow_immediate_skip: bool,
    ) -> Result<bool> {
        let result = match vm.pending_fade()? {
            FadeCommand::Begin {
                reveal,
                color,
                duration,
                priority,
            } => {
                let [from, to] = if reveal { [255, 0] } else { [0, 255] };
                submit(
                    host,
                    client,
                    json!([
                        {"op":"put","layer":{"id":LAYER,"rect":[0,0,canvas[0],canvas[1]],
                            "order":[!priority,256],"opacity":from,"paint":{"kind":"solid","color":color}}},
                        {"op":"begin_motion","id":LAYER,"track":{
                            "property":"opacity","from":from,"to":to,"duration":duration.max(1),"external":true
                        }}
                    ]),
                )?;
                *active = Some(Self {
                    reveal,
                    duration,
                    elapsed: 0,
                    complete: false,
                });
                Some(1)
            }
            FadeCommand::Remove => {
                Self::remove(active, host, client)?;
                None
            }
            FadeCommand::Finish => {
                Self::finish(active, host, client)?;
                None
            }
            FadeCommand::Remaining => Some(Self::remaining(*active)),
            FadeCommand::Wait { allow_skip } => {
                let ongoing = (Self::remaining(*active) as i32) >= 0;
                let skip =
                    ongoing && allow_skip && (allow_immediate_skip || vm.take_fade_confirm());
                if skip {
                    Self::finish(active, host, client)?;
                }
                vm.complete_fade(Some(u32::from(skip)), ongoing && !skip, skip)?;
                return Ok(ongoing);
            }
        };
        vm.complete_fade(result, false, false)?;
        Ok(false)
    }

    fn remaining(active: Option<Self>) -> u32 {
        active
            .filter(|fade| !fade.complete)
            .map_or(u32::MAX, |fade| fade.duration.wrapping_sub(fade.elapsed))
    }

    fn finish(
        active: &mut Option<Self>,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        let Some(fade) = *active else {
            return Ok(());
        };
        if fade.reveal {
            return Self::remove(active, host, client);
        }
        submit(
            host,
            client,
            json!([{"op":"end_motion","id":LAYER,"property":"opacity","finish":true}]),
        )?;
        *active = Some(Self {
            complete: true,
            ..fade
        });
        Ok(())
    }

    pub fn remove(
        active: &mut Option<Self>,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        if active.is_some() {
            submit(host, client, json!([{"op":"remove","id":LAYER}]))?;
            *active = None;
        }
        Ok(())
    }
}

fn submit(host: &mut dyn HostServices, client: ClientId, layers: serde_json::Value) -> Result<()> {
    host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":layers}))?)?;
    Ok(())
}
