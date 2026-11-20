// SPDX-License-Identifier: MPL-2.0
//! CVNS selectors whose whole native effect is VM or resource-binding state.

use super::{error, CmvsVm, DecodeError, SystemResult};

impl CmvsVm {
    /// Runs a CVNS native selector on the VM owner. `None` leaves it pending for the host.
    pub(super) fn cvns_command(&mut self, opcode: u16) -> Result<Option<SystemResult>, DecodeError> {
        let pc = self.pc();
        let word = |vm: &Self, distance| vm.required_parameter(opcode, distance);
        let cleanup: u16 = match opcode {
            0x21ca => {
                let name = self.machine.resolve_string(word(self, 4)?)?;
                if name.len() < 64 {
                    self.native.record(&mut self.persistent_lists, super::super::native_state::SEEN_IMAGE, name);
                }
                4
            }
            0x2028 | 0x2029 | 0x202a | 0x202c => {
                self.control.cvns_command(&self.machine, opcode)?
            }
            0x20a5 => {
                let number = word(self, 4)?;
                let name = self.machine.resolve_string(word(self, 8)?)?;
                if number < 64 && name.len() < 64 {
                    self.cvns.cd_tracks.insert(number as u8, name.to_vec());
                }
                self.cvns.last_cd_track = self.cvns.last_cd_track.max(number);
                8
            }
            0x20a6 => {
                self.cvns.last_cd_track = word(self, 4)?;
                4
            }
            0x21c4 => {
                self.cvns.external_slot = true;
                0
            }
            0x206b => {
                self.cvns.popup_suppressed = true;
                0
            }
            // 0x439a90 and 0x439ae0 use degrees and hundredths of a degree.
            0x20e7 | 0x20e8 | 0x20e9 => {
                let (results, bytes) = if opcode != 0x20e8 {
                    let raw = f64::from(word(self, 4)? as i32);
                    let (scale, radius, bytes) = if opcode == 0x20e7 {
                        (0.005555555555555556, f64::from(word(self, 8)? as i32), 8)
                    } else {
                        (0.00005555555555555556, 4096.0, 4)
                    };
                    let angle = raw * scale * std::f64::consts::PI;
                    ([(1, (angle.cos() * radius) as i64 as u32), (2, (angle.sin() * radius) as i64 as u32)], bytes)
                } else {
                    let [x1, y1, x2, y2] = [word(self, 4)?, word(self, 8)?, word(self, 12)?, word(self, 16)?].map(|value| value as i32);
                    let dx = f64::from(x2.wrapping_sub(x1));
                    let dy = y2.wrapping_sub(y1);
                    let theta = f64::from(dy).atan2(dx);
                    let mut degrees = (-57.29577951308232 * theta) as i64 as i32;
                    if degrees < 0 {
                        degrees += 360;
                    }
                    degrees = 360 - degrees;
                    if degrees == 360 {
                        degrees = 0;
                    }
                    let distance = if x2 == x1 { dy.unsigned_abs() } else { (dx / theta.cos()) as i64 as u32 };
                    ([(4, degrees as u32), (5, distance)], 16)
                };
                for (slot, value) in results {
                    self.machine.state_mut().set_system_integer(slot, value).map_err(|why| error(pc, opcode, why))?;
                }
                bytes
            }
            // 0x42aaf0..0x42b130: the x86 string family has its own numbering.
            0x20f0..=0x20fd => crate::vm::strings::command(&mut self.machine, opcode)?,
            0x20ea => {
                let value = word(self, 4)? as i32;
                let scale = word(self, 8)? as i32;
                let mode = word(self, 12)?;
                let result = match mode {
                    0 => Some((value.wrapping_mul(scale) / 1000) as u32),
                    1 | 2 => {
                        // 0x42b460 squares the scaled root, then takes EAX from
                        // __ftol2's signed 64-bit result. Invalid conversion
                        // produces the integer indefinite, whose low word is 0.
                        let scaled = f64::from(value).sqrt() * f64::from(scale) * 0.001;
                        let squared = scaled * scaled;
                        let low = if squared < 9_223_372_036_854_775_808.0 {
                            squared as i64 as u32
                        } else {
                            0
                        };
                        Some(if mode == 1 { low } else { (value as u32).wrapping_sub(low) })
                    }
                    _ => None,
                };
                if let Some(value) = result {
                    self.machine.state_mut().set_system_integer(0, value)
                        .map_err(|why| error(pc, opcode, why))?;
                }
                12
            }
            0x2069 => {
                let enabled = u32::from(self.popup_enabled);
                self.machine
                    .state_mut()
                    .set_system_integer(0, enabled)
                    .map_err(|why| error(pc, opcode, why))?;
                0
            }
            0x2128..=0x212f | 0x2136 => self.history.cvns_command(&mut self.machine, opcode)?,
            0x210e => {
                self.cvns.audit = true;
                0
            }
            // 0x42c460/0x42c4d0/0x42c540 run `index < first + count` in u32 arithmetic.
            0x2108..=0x210a => {
                let first = word(self, 4)?;
                let count = word(self, 8)?;
                let value = word(self, 12)?;
                let end = first.wrapping_add(count);
                if first < end {
                    let text = match opcode {
                        0x210a => self.machine.resolve_string(value)?.to_vec(),
                        _ => Vec::new(),
                    };
                    let state = self.machine.state_mut();
                    let filled = match opcode {
                        0x2108 => state.fill_bits(first, end - first, value != 0),
                        0x2109 => state.fill_integers(first, end - first, value as i32),
                        _ => state.fill_strings(first, end - first, &text),
                    };
                    filled.map_err(|why| error(pc, opcode, why))?;
                }
                12
            }
            // 0x429490.. bind slot SP-4 to the install-relative path at SP-8;
            // 0x40a190 and 0x423200 ignore slots above 4.
            0x2020..=0x2024 => {
                let slot = word(self, 4)?;
                let path = self.machine.resolve_string(word(self, 8)?)?;
                if slot <= 4 {
                    self.bindings
                        .set_source(opcode - 0x10, slot, path)
                        .map_err(|_| error(pc, opcode, "invalid CVNS resource registration path"))?;
                }
                8
            }
            // 0x429710 keeps the movie directory and 0x429760 the save directory;
            // both consume the unused slot word.
            0x2025 | 0x2026 => {
                let path = self.machine.resolve_string(word(self, 8)?)?;
                self.bindings
                    .set_root(opcode - 0x10, path)
                    .map_err(|_| error(pc, opcode, "invalid CVNS resource root path"))?;
                8
            }
            0x210b..=0x210d => {
                self.cvns.persistent_start[usize::from(opcode - 0x210b)] = word(self, 4)?;
                4
            }
            0x2014 => {
                self.cvns.version = word(self, 4)?.wrapping_shl(16) | word(self, 8)?;
                8
            }
            // The owner implements or rejects every other native selector.
            _ => return Ok(None),
        };
        Ok(Some(SystemResult(0x4000 | cleanup)))
    }
}

#[cfg(feature = "runtime")]
impl CmvsVm {
    pub(crate) fn complete_cvns_menu(&mut self, menu: super::SaveMenu) -> Result<(), DecodeError> {
        self.complete_owner_call(1, None)?;
        self.save_menu = Some(menu);
        Ok(())
    }

    pub(crate) fn write_cvns_text(&mut self, opcode: u16, destination: u32, bytes: &[u8]) -> Result<(), DecodeError> {
        self.machine.write_string_reference(opcode, destination, bytes)
    }

    pub(crate) fn complete_cvns_undefined_word(&mut self) -> Result<(), DecodeError> {
        self.complete_owner_call(2, None)?;
        self.machine.state_mut().forget_system_integer(0);
        Ok(())
    }

    pub(crate) fn complete_registry_text(&mut self, reference: u32, bytes: &[u8]) -> Result<(), DecodeError> {
        self.machine.write_registry_bytes(reference, bytes)?;
        self.complete_owner_call(3, Some(0))
    }

    pub(crate) fn cvns_header(&self, time: crysirror_core::service::CivilTime) -> Box<[u8; 600]> {
        let mut header = Box::new([0; 600]);
        header[..4].copy_from_slice(b"CSP1");
        header[4..8].copy_from_slice(&600u32.to_le_bytes());
        header[8..12].copy_from_slice(&self.cvns.version.to_le_bytes());
        let stamp = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} ",
            time.year, time.month, time.day, time.hour, time.minute, time.second
        );
        for (at, b) in stamp
            .bytes()
            .chain(self.save_title.iter().copied())
            .take(254)
            .enumerate()
        {
            header[16 + at] = b;
        }
        for (i, v) in self.save_information.iter().enumerate() {
            header[528 + 4 * i..532 + 4 * i].copy_from_slice(&v.unwrap_or(0).to_le_bytes());
        }
        header[544..546].copy_from_slice(&3u16.to_le_bytes());
        header[548] = u8::from(self.settings.thumbnail_size.iter().all(|&n| n != 0));
        header
    }

    #[cfg(feature = "runtime")]
    pub(crate) fn cvns_main_image(&self) -> anyhow::Result<Vec<u8>> {
        self.machine
            .main_expanded()
            .ok_or_else(|| anyhow::anyhow!("CVNS main script image is unavailable"))
    }

    #[cfg(feature="runtime")]
    pub(crate) fn cvns_save_suffix(&self) -> anyhow::Result<crate::vm::css0::Suffix> {
        crate::vm::css0::Suffix::new(self.cvns.persistent_start)
    }

    pub(crate) fn cvns_system_data(&self) -> anyhow::Result<crate::vm::css0::SystemData> {
        let lists = std::array::from_fn(|i| {
            if self.native.recording(i) { self.persistent_lists[i].clone() } else { Vec::new() }
        });
        crate::vm::css0::SystemData::capture(self.machine.state(), self.cvns_save_suffix()?, lists)
    }

    pub(crate) fn complete_cvns_system_load(&mut self, data: Option<crate::vm::css0::SystemData>) -> anyhow::Result<()> {
        self.owner_arguments::<0>(0x29c0)?;
        if let Some(data) = data {
            let mut next = self.machine.state().clone();
            let lists = data.apply(&mut next)?;
            self.complete_owner_call(0, None)?;
            *self.machine.state_mut() = next;
            for (i,list) in lists.iter().enumerate() {
                if !list.is_empty() {
                    self.persistent_lists[i].clear();
                    for name in list { self.native.record(&mut self.persistent_lists, i, name); }
                }
            }
        } else {
            self.complete_owner_call(0, None)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CmvsVm, VmProfile, VmYield};
    use crysirror_core::service::MemoryAssetReader;

    #[test]
    fn cvns_math_preserves_result_for_unknown_modes_and_uses_x86_widths() -> anyhow::Result<()> {
        let cases: &[(u16, &[u32], u32)] = &[
            (0x20e0, &[i32::MIN as u32], i32::MIN as u32),
            (0x20e1, &[10, 9, 2], 2),
            (0x20e6, &[3, 0], 3),
            (0x20ea, &[400, 500, 1], 100),
            (0x20ea, &[400, 500, 2], 300),
            (0x20ea, &[400, 500, 3], 300),
            (0x20ea, &[u32::MAX, 1000, 0], u32::MAX),
            (0x20ea, &[u32::MAX, 1000, 1], 0),
            (0x20ea, &[65536, 256000, 1], 0),
        ];
        let mut code = Vec::new();
        for &(selector, args, _) in cases {
            for arg in args.iter().rev() {
                code.extend_from_slice(&[0x40, 0x04, 4, 0]);
                code.extend_from_slice(&arg.to_le_bytes());
            }
            code.extend_from_slice(&selector.to_le_bytes());
            code.extend_from_slice(&0x2000u16.to_le_bytes());
        }
        let mut bytes = b"PSS0".to_vec();
        for word in [48, 0, 0, 0, code.len() as u32, 0, 0, 0, 0, 0, 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend(code);
        let reader = MemoryAssetReader(bytes.into());
        let mut vm = CmvsVm::prepare(VmProfile::Cvns20040515X86, &reader)?;
        for &(selector, _, expected) in cases {
            assert!(matches!(vm.run(32)?, VmYield::Frame));
            assert_eq!(vm.machine.state().system_integer(0), Some(expected), "selector {selector:#x}");
        }
        Ok(())
    }

    #[test]
    fn cvns_history_and_formatter_address_the_upper_string_bank() -> anyhow::Result<()> {
        let mut code = Vec::new();
        let mut call = |opcode: u16, args: &[u32]| {
            for arg in args.iter().rev() {
                code.extend_from_slice(&0x0440u16.to_le_bytes());
                code.extend_from_slice(&4u16.to_le_bytes());
                code.extend_from_slice(&arg.to_le_bytes());
            }
            code.extend_from_slice(&opcode.to_le_bytes());
        };
        let text = b"actor\0line\0voice\0%z3\0";
        call(0x20a5, &[63, 6]);
        call(0x20a5, &[u32::MAX, 0]);
        call(0x212d, &[2]);
        for i in 0..3 {
            call(0x212c, &[0, 6, 11, i, i + 10]);
        }
        call(0x2129, &[0, 0x8000_00fc]);
        call(0x212a, &[u32::MAX, 0x8000_00fd]);
        call(0x212b, &[1, 0x8000_00fe]);
        call(0x20fb, &[0x8000_00ff, 17, 7]);
        call(0x2136, &[1, 3]);
        call(0x2000, &[]);
        call(0x20a6, &[19]);
        call(0x2128, &[]);
        call(0x2000, &[]);
        call(0x212f, &[]);
        call(0x2136, &[0, 0]);
        call(0x2000, &[]);
        let mut bytes = b"PSS0".to_vec();
        for word in [48u32, 0, 0, 0, code.len() as u32, 0, text.len() as u32, 0, 0, 0, 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend(code);
        bytes.extend(text);
        let reader = MemoryAssetReader(bytes.into());
        let mut vm = CmvsVm::prepare(VmProfile::Cvns20040515X86, &reader)?;
        assert!(matches!(vm.run(256)?, VmYield::Frame));
        assert_eq!(vm.cvns.cd_tracks.get(&63).map(Vec::as_slice), Some(&b"line"[..]));
        assert_eq!(vm.cvns.cd_tracks.len(), 1);
        assert_eq!(vm.cvns.last_cd_track, u32::MAX);
        for (slot, expected) in [b"voice".as_slice(), b"line", b"actor", b"007"].into_iter().enumerate() {
            assert_eq!(vm.text_reference(0x8000_00fc + slot as u32), Some(expected));
        }
        assert_eq!(vm.machine.state().system_integer(0), Some(11));
        assert!(matches!(vm.run(32)?, VmYield::Frame));
        assert_eq!(vm.machine.state().system_integer(0), Some(2));
        assert_eq!(vm.cvns.last_cd_track, 19);
        assert!(matches!(vm.run(32)?, VmYield::Frame));
        assert_eq!(vm.machine.state().system_integer(0), Some(u32::MAX));
        Ok(())
    }
}
