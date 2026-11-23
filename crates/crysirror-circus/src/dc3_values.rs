//! DC3WY40BF90/40C4B0 value banks and original preference-write completion.
//! Do not translate these selectors through the DC4PH body/prefix offsets.
use super::*;
impl Machine {
    pub(super) fn value(&self, bank: i16, index: i16) -> Result<i32> {
        if let Some(value) = self.state.system_data.byte(bank, index)? {
            return Ok(i32::from(value));
        }
        match bank {
            0..=10 => {
                ensure!((0..100).contains(&index), "DC3WY script byte index");
                Ok(i32::from(
                    *self
                        .state
                        .script_bytes
                        .get(bank as usize * 100 + index as usize)
                        .context("DC3WY script byte state extent")?,
                ))
            }
            93 => {
                if (0..20).contains(&index) {
                    self.state.system_data.checkbox(index)
                } else {
                    Ok(0)
                }
            }
            94 | 95 => self
                .state
                .system_data
                .source_value(bank, index, self.state.source_count),
            99 => {
                if index == 3 {
                    return Ok(self.state.window_transition_flag);
                }
                if matches!(index, 4 | 21) {
                    return self
                        .state
                        .observations
                        .as_ref()
                        .filter(|observation| observation.pc == self.state.pc)
                        .and_then(|observation| observation.values.get(&index))
                        .copied()
                        .context(
                            "DC3WY live value lacks its owning instruction's host observation",
                        );
                }
                if let Some(value) = self.state.system_data.prefix_value(index)? {
                    return Ok(value);
                }
                Ok(self.state.runtime_values.get(&index).copied().unwrap_or(0))
            }
            value if value < 0 => bail!("DC3WY negative bank {bank}"),
            _ => Ok(0), // native getter's explicit unknown-bank return0
        }
    }
    pub(super) fn assign(
        &mut self,
        host: &mut dyn NativeHost,
        bank: i16,
        index: i16,
        value: i32,
    ) -> Result<bool> {
        let mut data = self.state.system_data.clone();
        if data.set_byte(bank, index, value)? {
            //100/49 invokes99/11 before setting byte241. All100 writes
            //call40B500 even if the previous value was identical.
            if bank == 100 {
                if index == 49 {
                    ensure!(data.set_prefix_value(11, value)?, "DC3WY text-speed prefix");
                }
                if !host.write_profile_system(Profile::Dc3WithYou, data.bytes())? {
                    return Ok(false);
                }
            }
            self.state.system_data = data;
            return Ok(true);
        }
        match bank {
            0..=10 => {
                ensure!((0..100).contains(&index), "DC3WY script byte index");
                *self
                    .state
                    .script_bytes
                    .get_mut(bank as usize * 100 + index as usize)
                    .context("DC3WY script byte state extent")? = value as u8;
            }
            93 => {
                if (0..20).contains(&index) {
                    data.set_checkbox(index, value)?;
                }
            }
            94 | 95 => data.set_source_value(bank, index, value, self.state.source_count)?,
            99 => {
                //These setters invoke native audio/window services. Keep the
                //PC until their precise shared-service policy is connected;
                //merely changing the persisted dword would lose a side effect.
                ensure!(
                    !matches!(index, 5 | 7 | 8 | 14 | 15 | 203 | 225 | 237),
                    "DC3WY value setter99/{index} requires its audio/window completion policy"
                );
                if index == 10 {
                    data.set_prefix_value(10, value)?;
                    data.set_prefix_value(214, i32::from(value == 2))?;
                } else if index == 227 {
                }
                // getter exists; original setter ignores it
                else if data.set_prefix_value(index, value)? {
                } else if matches!(
                    index,
                    0 | 2 | 254 | 253 | 246 | 245 | 243 | 241 | 233 | 224 | 204 | 195 | 193 | 192
                ) {
                    let value = if matches!(index, 254 | 253 | 246 | 245 | 243) && value == 255 {
                        -1
                    } else if index == 224 {
                        i32::from(value > 0)
                    } else {
                        value
                    };
                    self.state.runtime_values.insert(index, value);
                }
                //Other selectors fall out of40C4B0 without a state write.
            }
            value if value < 0 => bail!("DC3WY negative bank {bank}"),
            _ => {}
        }
        self.state.system_data = data;
        Ok(true)
    }
}
