// SPDX-License-Identifier: MPL-2.0
//! Engine fields that scripts set or query without a device effect, and the native
//! seen-name registry that `system.dat` persists. Offsets refer to the 2021 x64 engine
//! object; the 2016 build stores the same fields 16 bytes lower.

use super::{abi::SystemResult, evaluate::error, machine::Machine, DecodeError, VmProfile};

/// Name kinds of the registry: image names (+12032) and main-script names (+12040).
pub(crate) const SEEN_IMAGE: usize = 0;
pub(crate) const SEEN_SCRIPT: usize = 1;

#[derive(Clone)]
pub(super) struct NativeState {
    /// `0x212a`/`0x212b` (+12048/+12052): record loaded image and main-script names.
    recording: [bool; 2],
    /// `0x212c` (+11840): written to `system.dat` header +72.
    system_flag: bool,
    /// `0x200a` (+11160). Slot saves store it; loads and resets set it again.
    saved_flag: bool,
    /// `0x20e8` (+1660).
    word_1660: u32,
    /// `0x22cf` (+12112).
    flag_12112: bool,
    /// `0x229c`/`0x229d` (+12088/+12092), initialized to 1 by 0x14002c7e0.
    flags_12088: [bool; 2],
    /// `0x2006`: 0x140020010 selects the performance counter clock.
    precise_timer: bool,
}

impl Default for NativeState {
    fn default() -> Self {
        Self {
            recording: [false; 2],
            system_flag: false,
            saved_flag: true,
            word_1660: 0,
            flag_12112: false,
            flags_12088: [true; 2],
            precise_timer: true,
        }
    }
}

/// 0x1400201f0: CP932 comparison folding ASCII lower case outside double-byte pairs.
pub(crate) fn same_name(left: &[u8], right: &[u8]) -> bool {
    let (mut a, mut b) = (0, 0);
    loop {
        let Some(&first) = left.get(a) else {
            return right.get(b).is_none_or(|&byte| byte == 0);
        };
        if first == 0 {
            return right.get(b).is_none_or(|&byte| byte == 0);
        }
        let other = right.get(b).copied().unwrap_or(0);
        if matches!(first, 0x81..=0x9f | 0xe0..=0xfc) {
            if first != other || left.get(a + 1) != right.get(b + 1) {
                return false;
            }
            a += 2;
            b += 2;
        } else {
            if first.to_ascii_uppercase() != other.to_ascii_uppercase() {
                return false;
            }
            a += 1;
            b += 1;
        }
    }
}

impl NativeState {
    pub(super) fn recording(&self, kind: usize) -> bool {
        self.recording[kind]
    }

    pub(super) fn system_flag(&self) -> bool {
        self.system_flag
    }

    pub(super) fn set_saved_flag(&mut self, value: bool) {
        self.saved_flag = value;
    }

    /// 0x14002da80: append a name unless recording is off or an equal name exists.
    pub(super) fn record(&self, lists: &mut [Vec<Vec<u8>>; 2], kind: usize, name: &[u8]) {
        if !self.recording[kind] || lists[kind].iter().any(|entry| same_name(name, entry)) {
            return;
        }
        let end = name.iter().position(|&byte| byte == 0).unwrap_or(name.len());
        lists[kind].push(name[..end].to_vec());
    }

    /// Handles the selectors whose whole native effect is VM state. Returns the cleanup bytes.
    pub(super) fn command(
        &mut self,
        machine: &mut Machine,
        lists: &mut [Vec<Vec<u8>>; 2],
        opcode: u16,
    ) -> Result<Option<SystemResult>, DecodeError> {
        let pc = machine.pc();
        let word = |machine: &Machine, distance: usize| {
            machine
                .stack()
                .peek_u32(distance)
                .map_err(|_| error(pc, opcode, "parameter stack underflow or unknown word"))
        };
        let result = |machine: &mut Machine, value: u32| {
            machine
                .state_mut()
                .set_system_integer(0, value)
                .map_err(|why| error(pc, opcode, why))
        };
        let cleanup = match opcode {
            0x2006 => {
                self.precise_timer = word(machine, 4)? != 0;
                4
            }
            0x200a => {
                self.saved_flag = word(machine, 4)? != 0;
                4
            }
            0x20e8 => {
                self.word_1660 = word(machine, 4)?;
                4
            }
            0x2082 => {
                let reference = word(machine, 4)?;
                let name = machine.main_name().to_vec();
                machine.write_string_reference(opcode, reference, &name)?;
                4
            }
            0x2120 | 0x2121 => {
                let kind = usize::from(opcode == 0x2121);
                let name = machine.resolve_string(word(machine, 4)?)?;
                let found = lists[kind].iter().any(|entry| same_name(entry, name));
                result(machine, u32::from(found))?;
                4
            }
            0x2122 | 0x2123 => {
                lists[usize::from(opcode == 0x2123)].clear();
                0
            }
            0x212a | 0x212b => {
                self.recording[usize::from(opcode == 0x212b)] = true;
                0
            }
            0x212c => {
                self.system_flag = true;
                0
            }
            0x229c | 0x229d => {
                self.flags_12088[usize::from(opcode == 0x229d)] = word(machine, 4)? != 0;
                4
            }
            0x22cf => {
                self.flag_12112 = true;
                0
            }
            0x23b2 => {
                result(machine, 1)?;
                0
            }
            // The 2021 build answers capability queries without consulting the device.
            0x216b if machine.profile() == VmProfile::Pc20211007X64 => {
                result(machine, 7)?;
                0
            }
            0x22c0 | 0x22c1 if machine.profile() == VmProfile::Pc20211007X64 => {
                result(machine, 1)?;
                4
            }
            0x22ff if machine.profile() == VmProfile::Pc20211007X64 => {
                result(machine, 0)?;
                0
            }
            // 0x140075570 (2016): 3, 7 from pixel shader 2.0 and |8 from 3.0, read from
            // D3DCAPS9 byte 205. The shared renderer is at least a shader model 3 device.
            0x216b => {
                result(machine, 7 | 8)?;
                0
            }
            // 0x14007ecf0 / 0x14007ed90 (2016): with argument 1 they validate the machine
            // bound registration file `comap.dat` (0x22c0 first runs `reg.exe`). No such
            // file or machine key exists here, so both report unregistered without launching it.
            0x22c0 | 0x22c1 => {
                result(machine, 0)?;
                4
            }
            _ => return Ok(None),
        };
        Ok(Some(SystemResult(0x4000 | cleanup)))
    }

    pub(super) fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({
            "recording": self.recording,
            "system_flag": self.system_flag,
            "saved_flag": self.saved_flag,
            "word_1660": self.word_1660,
            "flag_12112": self.flag_12112,
            "flags_12088": self.flags_12088,
            "precise_timer": self.precise_timer,
        })
    }

    pub(super) fn from_snapshot(value: &serde_json::Value) -> Option<Self> {
        let pair = |key: &str| -> Option<[bool; 2]> {
            let array = value[key].as_array()?;
            Some([array.first()?.as_bool()?, array.get(1)?.as_bool()?])
        };
        Some(Self {
            recording: pair("recording")?,
            system_flag: value["system_flag"].as_bool()?,
            saved_flag: value["saved_flag"].as_bool()?,
            word_1660: value["word_1660"].as_u64()? as u32,
            flag_12112: value["flag_12112"].as_bool()?,
            flags_12088: pair("flags_12088")?,
            precise_timer: value["precise_timer"].as_bool()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_fold_ascii_case_outside_double_byte_pairs() {
        assert!(same_name(b"bg01.pb3", b"BG01.PB3"));
        assert!(!same_name(b"bg01", b"bg012"));
        // The trail byte 0x61 of a CP932 pair keeps its case.
        assert!(!same_name(b"\x82\x61", b"\x82\x41"));
        assert!(same_name(b"\x82\x61x", b"\x82\x61X"));
    }

    #[test]
    fn recording_needs_its_flag_and_skips_duplicates() {
        let mut state = NativeState::default();
        let mut lists = [Vec::new(), Vec::new()];
        state.record(&mut lists, SEEN_IMAGE, b"ev01");
        assert!(lists[0].is_empty());
        state.recording[SEEN_IMAGE] = true;
        state.record(&mut lists, SEEN_IMAGE, b"ev01");
        state.record(&mut lists, SEEN_IMAGE, b"EV01");
        state.record(&mut lists, SEEN_SCRIPT, b"sn1010.ps3");
        assert_eq!(lists, [vec![b"ev01".to_vec()], Vec::new()]);
    }
}
