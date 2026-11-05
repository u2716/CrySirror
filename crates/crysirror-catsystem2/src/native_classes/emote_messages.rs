// SPDX-License-Identifier: MPL-2.0
//! KCS natives answered by the E-mote task (class 54, handler 637F30).
use super::Ctx;
use anyhow::{ensure, Context, Result};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[907, 908, 909, 910, 924, 929, 930, 931, 965, 1025, 1027, 1028, 1029, 1030, 1047, 1048];

#[derive(Default)]
pub(crate) struct State {}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    ensure!(!a.is_empty(), "CatSystem2 E-mote native {} arguments", cx.call.id);
    #[cfg(not(feature = "runtime"))]
    anyhow::bail!("CatSystem2 E-mote native {} needs the runtime feature", cx.call.id);
    #[cfg(feature = "runtime")]
    {
        use crate::native_m2::Command;
        match cx.call.id {
            // 64C9E0 → 528: 0 without an M2 entry, else 57BFD0 (busy).
            909 => {
                cx.machine.preflight_native(true, &[])?;
                let busy = cx.m2.busy(a[0]);
                cx.machine.acknowledge(Some(u32::from(busy)))?;
            }
            // 64C710 → 526 → 5744D0: inertia on or off.
            907 => {
                ensure!(a.len() == 2, "CatSystem2 E-mote inertia arguments");
                cx.machine.preflight_native(false, &[])?;
                cx.m2.command(a[0], Command::Inertia(a[1] != 0));
                cx.machine.acknowledge(None)?;
            }
            // 545 → 5743F0: the subsequent motion setup resets the player's state.
            // 547 → 598760 selects a labelled waiting-wave set; #pattern_shift is
            // a separate per-digit offset resource parsed by 58B190.
            931 => {
                ensure!(a.len() == 2, "CatSystem2 E-mote motion reset arguments");
                cx.machine.preflight_native(false, &[])?;
                cx.m2.command(a[0], Command::ResetOnMotion(a[1] != 0));
                cx.machine.acknowledge(None)?;
            }
            965 => {
                ensure!(a.len() == 4, "CatSystem2 E-mote waiting arguments");
                cx.machine.preflight_native(false, &[])?;
                let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[1])?).context("CatSystem2 E-mote waiting name")?;
                cx.m2.command(a[0], Command::Waiting(name));
                cx.machine.acknowledge(None)?;
            }
            // 548/549 → 575650: variables are resolved by name, so registration
            // remains valid across a render-resource rebind.
            1025 => {
                cx.machine.preflight_native(false, &[])?;
                cx.machine.acknowledge(None)?;
            }
            // 57E500 → 57C1E0: every player's macro queue resets.
            908 | 1048 => {
                cx.machine.preflight_native(false, &[])?;
                cx.m2.command(a[0], Command::ActionClear);
                cx.machine.acknowledge(None)?;
            }
            // 64C7A0 → 529 → 587790: `#attribute` {x, y, extra} into three VM words.
            910 => {
                ensure!(a.len() == 5, "CatSystem2 E-mote attribute arguments");
                let [x, y, extra] = cx.m2.attribute(a[0], a[1]).unwrap_or([0; 3]);
                cx.machine.commit_native(None, &[(a[2], x.to_le_bytes().to_vec()), (a[3], y.to_le_bytes().to_vec()), (a[4], extra.to_le_bytes().to_vec())])?;
            }
            // 64CBC0 → 540: a2 is the voice channel, a1 on/off.
            924 => {
                ensure!(a.len() == 3, "CatSystem2 E-mote lip arguments");
                cx.machine.preflight_native(false, &[])?;
                cx.m2.command(a[0], Command::Lip((a[1] != 0 && a[2] != u32::MAX).then_some(a[2])));
                cx.machine.acknowledge(None)?;
            }
            // 64CBF0 → 543 → 574D70 / 64D230 → 544 → 574DC0.
            929 => {
                ensure!(a.len() == 2, "CatSystem2 E-mote timeline arguments");
                cx.machine.preflight_native(false, &[])?;
                let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[1])?).context("CatSystem2 E-mote timeline name")?;
                cx.m2.command(a[0], Command::Timeline(Some(name)));
                cx.machine.acknowledge(None)?;
            }
            930 => {
                cx.machine.preflight_native(false, &[])?;
                cx.m2.command(a[0], Command::Timeline(None));
                cx.machine.acknowledge(None)?;
            }
            // 64CB50 → 551 → 591620: start a macro; the native always answers its out word 0.
            1027 => {
                ensure!(a.len() == 2, "CatSystem2 E-mote macro arguments");
                cx.machine.preflight_native(true, &[])?;
                let name = String::from_utf8(crate::native_values::format_bytes(cx.machine, a[1])?).context("CatSystem2 E-mote macro name")?;
                cx.m2.command(a[0], Command::Action(name));
                cx.machine.acknowledge(Some(0))?;
            }
            // 64CA40 → 553 → 5758F0.
            1028 => {
                cx.machine.preflight_native(true, &[])?;
                let active = cx.m2.action_active(a[0]);
                cx.machine.acknowledge(Some(u32::from(active)))?;
            }
            // 64C980 → 554 → 580020: rows of `#touch` colour a1, table a2.
            1029 => {
                ensure!(a.len() == 3, "CatSystem2 E-mote touch count arguments");
                cx.machine.preflight_native(true, &[])?;
                let rows = cx.m2.touch_rows(a[0], a[1], a[2]);
                cx.machine.acknowledge(Some(rows))?;
            }
            // 64C890 → 554 → 57FFC0: one row's action and voice names.
            1030 => {
                ensure!(a.len() == 6, "CatSystem2 E-mote touch row arguments");
                match cx.m2.touch_row(a[0], a[1], a[2], a[3]) {
                    Some((action, voice)) => {
                        let terminated = |text: String| {
                            let mut bytes = encoding_rs::SHIFT_JIS.encode(&text).0.into_owned();
                            bytes.push(0);
                            bytes
                        };
                        cx.machine.commit_native(Some(1), &[(a[4], terminated(action)), (a[5], terminated(voice))])?;
                    }
                    None => {
                        cx.machine.preflight_native(true, &[])?;
                        cx.machine.acknowledge(Some(0))?;
                    }
                }
            }
            // 64CAC0 → 552 → 591820.
            1047 => {
                cx.machine.preflight_native(false, &[])?;
                cx.m2.command(a[0], Command::ActionAdvance);
                cx.machine.acknowledge(None)?;
            }
            id => anyhow::bail!("CatSystem2 E-mote native {id} is not implemented"),
        }
        Ok(true)
    }
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    #[test]
    fn nothing_here_touches_a_model_without_one() {
        let planes = crate::native_m2::Planes::default();
        assert!(!planes.action_active(1));
        assert_eq!(planes.touch_rows(1, 0, 0), 0);
        assert_eq!(planes.attribute(1, 0), None);
    }
}
