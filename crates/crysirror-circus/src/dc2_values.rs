//! DC2 41A300/41A5C0. Unlike later generations ordinary script bytes clamp,
//! while banks200..209 overlap four consecutive bytes in big-endian order.
use super::*;
use crate::machine::system_ops::{SystemReply, SystemRequest};
impl Machine {
    pub(super) fn value(
        &mut self,
        bank: i16,
        index: i16,
        host: &mut dyn NativeHost,
    ) -> Result<Option<i32>> {
        ensure!(
            bank >= 0 && index >= 0,
            "DC2 negative native variable address"
        );
        if let Some(value) = self.state.settings.byte(bank, index)? {
            return Ok(Some(value.into()));
        }
        if (200..210).contains(&bank) {
            if index >= 97 {
                return Ok(Some(0));
            }
            let at = (bank as usize - 200) * 100 + index as usize;
            return Ok(Some(i32::from_be_bytes(
                self.state.script_bytes[at..at + 4].try_into()?,
            )));
        }
        if bank < 10 {
            return Ok(Some(if index < 100 {
                self.state.script_bytes[bank as usize * 100 + index as usize].into()
            } else {
                0
            }));
        }
        if bank == 90 {
            ensure!(
                (index as usize) < 1500,
                "DC2 native1500-scene completion bank"
            );
            return Ok(Some(self.state.scene_completed[index as usize].into()));
        }
        if bank != 99 {
            return Ok(Some(0));
        }
        if let Some(value) = self.state.settings.prefix(index)? {
            return Ok(Some(value));
        }
        if let Some(value) = self.state.observations.get(&(bank, index)) {
            return Ok(Some(*value));
        }
        let value = match index {
            4 => match host.system(SystemRequest::NativeWindowMode)? {
                None => return Ok(None),
                Some(SystemReply::NativeWindowMode(mode)) => i32::from(mode),
                Some(_) => bail!("DC2 window-mode foreign receipt"),
            },
            21 => match host.system(SystemRequest::LocalHour)? {
                None => return Ok(None),
                Some(SystemReply::Hour(hour)) => i32::from(hour),
                Some(_) => bail!("DC2 civil-hour foreign receipt"),
            },
            //These getters require their original player/movie/input state. A zero
            //answer would bypass genuine native branch semantics.
            249 => {
                let enabled =
                    i32::from_le_bytes(self.state.settings.bytes()[56..60].try_into()?) != 0;
                i32::from(!enabled || !host.audio_playing(8)?)
            }
            //41A300: A240E8 ("NoMovie", absent -> 0) == 0.
            247 => 1,
            248 | 250 => bail!("DC2 runtime observation99/{index} is not connected"),
            252 | 253 => {
                let value = self.state.runtime.get(&index).copied().unwrap_or(-1);
                if value == -1 { 255 } else { value }
            }
            244 => self.state.runtime.get(&index).copied().unwrap_or(-1),
            0 | 1 | 2 | 3 | 25 | 254 | 251 | 246 | 243 | 242 => {
                self.state.runtime.get(&index).copied().unwrap_or(0)
            }
            _ => 0, //41A300's actual unknown-selector return0.
        };
        self.state.observations.insert((bank, index), value);
        Ok(Some(value))
    }
    pub(super) fn assign(
        &mut self,
        bank: i16,
        index: i16,
        value: i32,
        host: &mut dyn NativeHost,
    ) -> Result<bool> {
        ensure!(
            bank >= 0 && index >= 0,
            "DC2 negative native variable address"
        );
        let mut data = self.state.settings.clone();
        if data.set_byte(bank, index, value)? {
            if bank == 100
                && !host.write_profile_system(Profile::Dc2PlusCommunication, data.bytes())?
            {
                return Ok(false);
            }
            self.state.settings = data;
            return Ok(true);
        }
        if (200..210).contains(&bank) {
            if index < 97 {
                let at = (bank as usize - 200) * 100 + index as usize;
                self.state.script_bytes[at..at + 4].copy_from_slice(&value.to_be_bytes());
            }
            return Ok(true);
        }
        if bank < 10 {
            if index < 100 {
                self.state.script_bytes[bank as usize * 100 + index as usize] =
                    value.clamp(0, 255) as u8
            }
            return Ok(true);
        }
        if bank == 90 {
            ensure!(
                (index as usize) < 1500,
                "DC2 native1500-scene completion bank"
            );
            self.state.scene_completed[index as usize] = value as u8;
            return Ok(true);
        }
        if bank != 99 {
            return Ok(true);
        }
        ensure!(
            !matches!(index, 7 | 14),
            "DC2 value setter99/{index} requires its native music receipt"
        );
        if data.set_prefix(index, value)? {
            self.state.settings = data;
            return Ok(true);
        }
        if matches!(index, 0 | 1 | 19 | 25 | 254 | 253 | 252 | 246 | 243 | 242) {
            let value = if matches!(index, 254 | 253 | 252 | 243) && value == 255 {
                -1
            } else {
                value
            };
            self.state.runtime.insert(index, value);
        }
        //All other41A5C0 selector arms return without writing state.
        Ok(true)
    }
}
