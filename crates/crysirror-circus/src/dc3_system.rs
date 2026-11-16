//! DC3WY4233A0 case16: native platform/preference operations. File reads are
//! owned worker transactions; retries retain their original native PC/result.
use super::*;
use crate::machine::{
    resource_ops::{ResourceReply, ResourceRequest},
    system_ops::{SystemReply, SystemRequest},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreferenceReload {
    pub owner: u32,
    pub bytes: Option<Vec<u8>>,
    pub phase: u8,
}
impl Machine {
    pub(super) fn execute_system(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<bool>> {
        if instruction.opcode != 0x16 {
            return Ok(None);
        }
        match a[0] {
            2 | 3 => {
                let Some(SystemReply::Done) = host.system(SystemRequest::Visible(a[0] == 3))?
                else {
                    return Ok(Some(false));
                };
            }
            7 => {
                let Some(SystemReply::WindowTransitionFlag(value)) =
                    host.system(SystemRequest::WindowTransitionFlag(Some(i32::from(a[1]))))?
                else {
                    return Ok(Some(false));
                };
                ensure!(
                    value == i32::from(a[1]),
                    "DC3WY window transition flag receipt"
                );
                self.state.window_transition_flag = value;
            }
            //40B6A0: operand nonzero writes slot 0 (only outside replay),
            //zero writes slot var(99,254).
            4 => {
                let slot = if a[1] != 0 { 0 } else { self.value(99, 254)? };
                if !self.write_snapshot(host, slot)? {
                    return Ok(Some(false));
                }
            }
            //408F00 case 18: 0 backs the 0x4528 preference bytes up, 2
            //restores them, 1 sets var(3,0) = 1 when they differ from the
            //backup, var(100,54) excluded.
            18 => {
                let prefix = crate::dc3_settings::PREFERENCES_BYTES;
                match a[1] {
                    0 => self.state.preference_backup = Some(self.state.system_data.bytes()[..prefix].to_vec()),
                    1 => {
                        let backup = self.state.preference_backup.clone().context("DC3WY preference compare without a backup")?;
                        let mut current = self.state.system_data.clone();
                        let mut saved = crate::dc3_settings::SystemData::try_from([backup.as_slice(), &current.bytes()[prefix..]].concat())?;
                        let ignored = current.byte(100, 54)?.unwrap_or(0);
                        ensure!(saved.set_byte(100, 54, i32::from(ignored))?, "DC3WY preference compare field");
                        let differs = current.bytes()[..prefix] != saved.bytes()[..prefix];
                        let _ = &mut current;
                        if !self.assign(host, 3, 0, i32::from(differs))? {
                            return Ok(Some(false));
                        }
                    }
                    _ => {
                        let backup = self.state.preference_backup.clone().context("DC3WY preference restore without a backup")?;
                        let current = self.state.system_data.bytes();
                        self.state.system_data = crate::dc3_settings::SystemData::try_from([backup.as_slice(), &current[prefix..]].concat())?;
                    }
                }
            }
            15 => {
                //41D800 only clearsBuf62076 andthis2840/2841. This is
                //native input policy, not a renderer/menu emulation call.
                self.state.skip_mode = i32::from(a[1]);
                self.state.skip_latches.fill(0);
                self.state.runtime_values.insert(195, 0);
            }
            //402B20(entries[Buf5983]) marks the current history run read
            //(402870) when its scene is not -1 and its first label nonzero.
            //Selector 5 then writes system.dat (40B500). Selector 6 with
            //operand 1 additionally calls 402130, which is not connected.
            5 | 6 => {
                ensure!(
                    a[0] == 5 || a[1] != 1,
                    "DC3WY case16 selector 6/1 (402130) is not connected"
                );
                let mut data = self.state.system_data.clone();
                let history = &self.state.history;
                let entry = history
                    .entries
                    .get(history.current)
                    .context("DC3WY history run index")?;
                if entry.scene != -1 && entry.first != 0 {
                    ensure!(entry.scene >= 0, "DC3WY history run scene");
                    data.mark_read(entry.scene as usize, entry.first, entry.last)?;
                }
                if a[0] == 5 && !host.write_profile_system(Profile::Dc3WithYou, data.bytes())? {
                    return Ok(Some(false));
                }
                self.state.system_data = data;
            }
            10 => match host.system(SystemRequest::License)? {
                None => return Ok(Some(false)),
                Some(SystemReply::Done) => {}
                Some(SystemReply::Cancelled) => self.state.halted = true,
                Some(_) => bail!("DC3WY installation-key service returned a foreign receipt"),
            },
            26 => {
                if self.state.preference_reload.is_none() {
                    let reply = host.resource(&ResourceRequest::ReadPreferences {
                        profile: Profile::Dc3WithYou,
                    })?;
                    let bytes = match reply {
                        None => return Ok(Some(false)),
                        Some(ResourceReply::Preferences(bytes)) => bytes,
                        Some(_) => bail!("DC3WY preference worker returned a foreign receipt"),
                    };
                    self.state.preference_reload = Some(PreferenceReload {
                        owner: instruction.offset,
                        bytes,
                        phase: 0,
                    });
                }
                let pending = self.state.preference_reload.as_ref().unwrap().clone();
                ensure!(
                    pending.owner == instruction.offset,
                    "DC3WY preference reload continuation owner"
                );
                let reset = pending.bytes.is_none();
                let data = if let Some(bytes) = pending.bytes {
                    crate::dc3_settings::SystemData::try_from(bytes)?
                } else {
                    let mut data = self.state.system_data.clone();
                    data.reset_preferences();
                    data
                };
                data.names()?;
                //40D320's successful read only copies the two native blocks.
                //Its missing-file path40D1B0 additionally invokes40CC50:
                //enable flags and presentation aspect, not live gain setters.
                if reset {
                    if pending.phase == 0 {
                        let controls = audio::controls(&data)?;
                        if !host.emit(&Event::NativeMixerControls {
                            enabled: controls.enabled,
                            gain: controls.gain,
                            playback_gain: controls.playback_gain,
                            force_gain: [false; 3],
                            force_enable: [true; 3],
                        })? {
                            return Ok(Some(false));
                        }
                        self.state.preference_reload.as_mut().unwrap().phase = 1;
                    }
                    if pending.phase <= 1 {
                        let Some(SystemReply::Done) =
                            host.system(SystemRequest::PresentationAspect(
                                data.prefix_value(237)?.unwrap_or(0) != 0,
                            ))?
                        else {
                            return Ok(Some(false));
                        };
                        self.state.preference_reload.as_mut().unwrap().phase = 2;
                    }
                }
                self.state.system_data = data;
                self.state.preference_reload = None;
            }
            //8: 4312D0(a1, 0) with renderer+24 forced to 1 for the call
            //(D.C.III 418470(a1)): the native window/fullscreen switch.
            8 => {
                let Some(SystemReply::Done) = host.system(SystemRequest::NativeFullscreenMode(a[1]))? else {
                    return Ok(Some(false));
                };
            }
            //Actual case16 nested switch fall-throughs, not missing hooks.
            selector
                if !matches!(
                    selector,
                    0 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 10 | 15 | 18 | 20 | 22 | 23 | 24 | 25 | 26 | 27
                ) => {}
            selector => bail!(
                "unsupported original DC3WY system selector {selector}; native call remains unacknowledged"
            ),
        }
        Ok(Some(true))
    }
}
