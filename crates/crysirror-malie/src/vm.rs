// SPDX-License-Identifier: MPL-2.0
//! Ragnarok native interpreter 67A0F0. All guest addresses are checked numeric
//! offsets into owned buffers; C imports become retained semantic calls, never
//! executable pointers. Opcode32 reads cdecl cleanup at the caller's return PC.
use crate::program::Program;
use anyhow::{bail, ensure, Context, Result};
use std::sync::Arc;
pub const STRING_BASE: u32 = 0x1000_0000;
pub const GLOBAL_BASE: u32 = 0x2000_0000;
pub const STACK_BASE: u32 = 0x3000_0000;
/// Engine-owned strings returned to scripts (CSV_RefString, name slots).
pub const HOST_BASE: u32 = 0x4000_0000;
const HOST_BYTES: usize = 256 * 1024;
const STACK_BYTES: u32 = 1024 * 1024;
const RETURN_HOST: u32 = u32::MAX;
#[derive(Clone, Debug)]
pub struct Call {
    pub pc: u32,
    pub function: u32,
    pub args: Vec<u32>,
}
/// Serializable interpreter state for native-equivalent save records. The
/// live stack above `sp` is kept; a pending native call is not saveable.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MachineState {
    pub pc: u32,
    pub globals: Vec<u8>,
    pub stack: Vec<u8>,
    pub sp: u32,
    pub fp: u32,
    pub stopped: bool,
}
pub enum Step {
    Continue,
    Call(Call),
    Stop,
}
pub struct Machine {
    pub program: Arc<Program>,
    pub pc: u32,
    pub globals: Vec<u8>,
    stack: Vec<u8>,
    sp: u32,
    fp: u32,
    pub pending: Option<Call>,
    pub stopped: bool,
    host: Vec<u8>,
    host_at: usize,
}
fn word(data: &[u8], at: u32) -> Result<u32> {
    let at = at as usize;
    Ok(u32::from_le_bytes(
        data.get(at..at.checked_add(4).context("Malie memory arithmetic")?)
            .context("Malie memory read range")?
            .try_into()?,
    ))
}
fn store(data: &mut [u8], at: u32, value: u32) -> Result<()> {
    let at = at as usize;
    data.get_mut(at..at.checked_add(4).context("Malie memory arithmetic")?)
        .context("Malie memory write range")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}
impl Machine {
    pub fn new(program: Arc<Program>) -> Self {
        Self {
            globals: vec![0; program.global_size as usize],
            program,
            pc: 0,
            stack: vec![0; STACK_BYTES as usize],
            sp: STACK_BYTES,
            fp: STACK_BYTES,
            pending: None,
            stopped: true,
            host: vec![0; HOST_BYTES],
            host_at: 0,
        }
    }
    pub fn state(&self) -> Result<MachineState> {
        ensure!(self.pending.is_none(), "Malie VM has a pending native call");
        Ok(MachineState {
            pc: self.pc,
            globals: self.globals.clone(),
            stack: self.stack[self.sp as usize..].to_vec(),
            sp: self.sp,
            fp: self.fp,
            stopped: self.stopped,
        })
    }
    pub fn restore(&mut self, s: &MachineState) -> Result<()> {
        ensure!(s.globals.len() == self.globals.len(), "Malie save global size");
        ensure!(s.sp <= STACK_BYTES && s.stack.len() == (STACK_BYTES - s.sp) as usize && s.fp <= STACK_BYTES, "Malie save stack extent");
        ensure!((s.pc as usize) < self.program.code.len(), "Malie save PC");
        self.pc = s.pc;
        self.globals.copy_from_slice(&s.globals);
        self.stack.fill(0);
        self.stack[s.sp as usize..].copy_from_slice(&s.stack);
        self.sp = s.sp;
        self.fp = s.fp;
        self.stopped = s.stopped;
        self.pending = None;
        Ok(())
    }
    pub fn label(&mut self, name: &str) -> Result<()> {
        self.pc = *self
            .program
            .labels
            .get(name)
            .with_context(|| format!("Malie label {name:?} is absent"))?;
        self.sp = STACK_BYTES;
        self.fp = STACK_BYTES;
        self.pending = None;
        self.stopped = false;
        Ok(())
    }
    pub fn function(&mut self, name: &str) -> Result<()> {
        self.pc = self
            .program
            .functions
            .iter()
            .find(|f| f.name == name && f.kind == 3)
            .with_context(|| format!("Malie script function {name:?} is absent"))?
            .pc;
        self.sp = STACK_BYTES;
        self.fp = STACK_BYTES;
        self.push(RETURN_HOST)?;
        self.pending = None;
        self.stopped = false;
        Ok(())
    }
    fn byte(&mut self) -> Result<u32> {
        let v = *self
            .program
            .code
            .get(self.pc as usize)
            .context("Malie code PC range")?;
        self.pc += 1;
        Ok(u32::from(v))
    }
    fn immediate(&mut self, size: u32) -> Result<u32> {
        let mut v = 0;
        for shift in 0..size {
            v |= self.byte()? << (shift * 8);
        }
        Ok(v)
    }
    fn push(&mut self, v: u32) -> Result<()> {
        self.sp = self.sp.checked_sub(4).context("Malie stack capacity")?;
        store(&mut self.stack, self.sp, v)
    }
    fn pop(&mut self) -> Result<u32> {
        let v = word(&self.stack, self.sp)?;
        self.sp = self
            .sp
            .checked_add(4)
            .filter(|v| *v <= STACK_BYTES)
            .context("Malie stack underflow")?;
        Ok(v)
    }
    fn top(&self) -> Result<u32> {
        word(&self.stack, self.sp)
    }
    fn replace(&mut self, v: u32) -> Result<()> {
        store(&mut self.stack, self.sp, v)
    }
    fn jump(&mut self, pc: u32) -> Result<()> {
        ensure!((pc as usize) < self.program.code.len(), "Malie jump range");
        self.pc = pc;
        Ok(())
    }
    pub fn complete(&mut self, result: u32) -> Result<()> {
        let call = self
            .pending
            .take()
            .context("Malie has no pending native call")?;
        for _ in &call.args {
            self.pop()?;
        }
        self.push(result)
    }
    /// Copy a string into the engine string ring and return its address.
    pub fn host_string(&mut self, text: &str) -> Result<u32> {
        let units: Vec<u16> = text.encode_utf16().chain([0]).collect();
        let n = units.len() * 2;
        ensure!(n <= 32 * 1024, "Malie host string budget");
        if self.host_at + n > HOST_BYTES {
            self.host_at = 0;
        }
        let at = self.host_at;
        for (i, u) in units.iter().enumerate() {
            self.host[at + 2 * i..at + 2 * i + 2].copy_from_slice(&u.to_le_bytes());
        }
        self.host_at += n;
        Ok(HOST_BASE + at as u32)
    }
    fn region_mut(&mut self, address: u32, len: usize) -> Result<&mut [u8]> {
        let (data, at) = if (GLOBAL_BASE..STACK_BASE).contains(&address) {
            (&mut self.globals[..], address - GLOBAL_BASE)
        } else if (STACK_BASE..HOST_BASE).contains(&address) {
            (&mut self.stack[..], address - STACK_BASE)
        } else {
            bail!("Malie write to non-writable guest address {address:#x}")
        };
        let at = at as usize;
        data.get_mut(at..at.checked_add(len).context("Malie write extent")?).context("Malie guest write range")
    }
    pub fn read_bytes(&self, address: u32, len: usize) -> Result<Vec<u8>> {
        ensure!(len <= 1024 * 1024, "Malie guest read budget");
        let (data, at): (&[u8], u32) = if (STRING_BASE..GLOBAL_BASE).contains(&address) {
            (&self.program.strings, address - STRING_BASE)
        } else if (GLOBAL_BASE..STACK_BASE).contains(&address) {
            (&self.globals, address - GLOBAL_BASE)
        } else if (STACK_BASE..HOST_BASE).contains(&address) {
            (&self.stack, address - STACK_BASE)
        } else if address >= HOST_BASE {
            (&self.host, address - HOST_BASE)
        } else {
            bail!("Malie read from non-guest address {address:#x}")
        };
        let at = at as usize;
        Ok(data.get(at..at.checked_add(len).context("Malie read extent")?).context("Malie guest read range")?.to_vec())
    }
    pub fn write_bytes(&mut self, address: u32, bytes: &[u8]) -> Result<()> {
        self.region_mut(address, bytes.len())?.copy_from_slice(bytes);
        Ok(())
    }
    pub fn write_string(&mut self, address: u32, text: &str) -> Result<()> {
        let bytes: Vec<u8> = text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
        self.write_bytes(address, &bytes)
    }
    pub fn write_word(&mut self, address: u32, value: u32) -> Result<()> {
        self.write_bytes(address, &value.to_le_bytes())
    }
    pub fn string(&self, address: u32) -> Result<String> {
        let (data, at) = if address >= HOST_BASE {
            (&self.host[..], address - HOST_BASE)
        } else if address >= STRING_BASE && address < GLOBAL_BASE {
            (&self.program.strings[..], address - STRING_BASE)
        } else if address >= GLOBAL_BASE && address < STACK_BASE {
            (&self.globals[..], address - GLOBAL_BASE)
        } else if (STACK_BASE..HOST_BASE).contains(&address) {
            (&self.stack[..], address - STACK_BASE)
        } else {
            bail!("Malie string pointer is not a guest string address")
        };
        let mut units = Vec::new();
        let mut at = at as usize;
        for _ in 0..32768 {
            let v = data
                .get(at..at + 2)
                .context("Malie unterminated guest string")?;
            let c = u16::from_le_bytes([v[0], v[1]]);
            if c == 0 {
                return Ok(String::from_utf16(&units)?);
            }
            units.push(c);
            at += 2;
        }
        bail!("Malie guest string budget")
    }
    pub fn step(&mut self) -> Result<Step> {
        if self.stopped {
            return Ok(Step::Stop);
        }
        if let Some(call) = &self.pending {
            return Ok(Step::Call(call.clone()));
        }
        let pc = self.pc;
        let op = self.byte()?;
        let result = (|| -> Result<Step> {
            match op {
                0..=2 => {
                    let dest = self.immediate(4)?;
                    let yes = op == 0 || {
                        let v = self.pop()?;
                        (v != 0) == (op == 1)
                    };
                    if yes {
                        self.jump(dest)?;
                    }
                }
                3 | 4 => {
                    let function = self.immediate(if op == 3 { 4 } else { 1 })?;
                    let argc = self.byte()?;
                    ensure!(
                        (function as usize) < self.program.functions.len(),
                        "Malie import index"
                    );
                    let mut args = Vec::with_capacity(argc as usize);
                    for i in 0..argc {
                        args.push(word(
                            &self.stack,
                            self.sp
                                .checked_add(i * 4)
                                .context("Malie argument offset")?,
                        )?);
                    }
                    let call = Call { pc, function, args };
                    self.pending = Some(call.clone());
                    return Ok(Step::Call(call));
                }
                5 => {
                    self.stopped = true;
                    return Ok(Step::Stop);
                }
                6 => {
                    let at = self.top()?;
                    self.replace(word(&self.globals, at)?)?;
                }
                7 => {
                    let at = self.pop()?;
                    let v = self.top()?;
                    store(&mut self.globals, at, v)?;
                }
                8 | 13 => {
                    let v = self.immediate(4)?;
                    self.push(v)?;
                }
                9 | 10 | 12 => {
                    let at = self.immediate(match op {
                        9 => 1,
                        10 => 2,
                        _ => 4,
                    })?;
                    ensure!(
                        (at as usize) < self.program.strings.len(),
                        "Malie constant string range"
                    );
                    self.push(STRING_BASE + at)?;
                }
                11 | 16 => {} // Native default branch (67A0F0) leaves all state intact.
                14 => {
                    self.pop()?;
                }
                15 => self.push(0)?,
                17 => {
                    let v = self.byte()?;
                    self.push(v)?;
                }
                18 => self.push(self.top()?)?,
                19 => {
                    let v = self.top()?;
                    self.replace(v.wrapping_neg())?;
                }
                20..=27 | 30 | 31 | 33..=40 => {
                    let rhs = self.pop()?;
                    let lhs = self.top()?;
                    let v = match op {
                        20 => lhs.wrapping_add(rhs),
                        21 => lhs.wrapping_sub(rhs),
                        22 => lhs.wrapping_mul(rhs),
                        23 | 24 => {
                            let a = lhs as i32;
                            let b = rhs as i32;
                            ensure!(
                                b != 0 && !(a == i32::MIN && b == -1),
                                "Malie signed division fault"
                            );
                            if op == 23 {
                                (a / b) as u32
                            } else {
                                (a % b) as u32
                            }
                        }
                        25 => lhs & rhs,
                        26 => lhs | rhs,
                        27 => lhs ^ rhs,
                        30 => u32::from(lhs != 0 && rhs != 0),
                        31 => u32::from(lhs != 0 || rhs != 0),
                        33 => u32::from((lhs as i32) < rhs as i32),
                        34 => u32::from((lhs as i32) <= rhs as i32),
                        35 => u32::from((lhs as i32) > rhs as i32),
                        36 => u32::from((lhs as i32) >= rhs as i32),
                        37 => u32::from(lhs == rhs),
                        38 => u32::from(lhs != rhs),
                        39 => lhs.wrapping_shl(rhs & 31),
                        40 => ((lhs as i32) >> (rhs & 31)) as u32,
                        _ => unreachable!(),
                    };
                    self.replace(v)?;
                }
                28 | 29 | 32 | 41 | 42 | 43 | 48 => {
                    let v = self.top()?;
                    let v = match op {
                        28 => !v,
                        29 => u32::from(v != 0),
                        32 => u32::from(v == 0),
                        41 => v.wrapping_add(1),
                        42 => v.wrapping_sub(1),
                        43 => v.checked_add(GLOBAL_BASE).context("Malie global address")?,
                        48 => v.wrapping_add(STACK_BASE + self.fp),
                        _ => unreachable!(),
                    };
                    self.replace(v)?;
                }
                // 67A0F0 case 44: 679C00 dumps IP/stack/data to the debug
                // log, then fgetc(stdin) returns at once in a GUI process.
                44 => log::debug!("Malie debug dump pc={pc:#x} sp={:#x} fp={:#x}", self.sp, self.fp),
                45 => {
                    let index = self.immediate(4)?;
                    let f = self
                        .program
                        .functions
                        .get(index as usize)
                        .context("Malie call index")?;
                    ensure!(
                        f.kind == 3,
                        "Malie indirect script call refers to an unbound native function"
                    );
                    let next = f.pc;
                    self.push(self.pc)?;
                    self.jump(next)?;
                }
                46 => {
                    let at = self.fp.wrapping_add(self.top()?);
                    self.replace(word(&self.stack, at)?)?;
                }
                47 => {
                    let at = self.fp.wrapping_add(self.pop()?);
                    let value = self.top()?;
                    store(&mut self.stack, at, value)?;
                }
                49 => {
                    let bytes = self.immediate(4)?;
                    let old = self.fp;
                    self.push(old)?;
                    self.fp = self.sp;
                    self.sp = self.sp.checked_sub(bytes).context("Malie frame capacity")?;
                }
                50 | 51 => {
                    let count = if op == 51 { Some(self.byte()?) } else { None };
                    let value = self.top()?;
                    let previous = word(&self.stack, self.fp)?;
                    let ret = word(
                        &self.stack,
                        self.fp.checked_add(4).context("Malie return frame")?,
                    )?;
                    let old = self.fp;
                    self.fp = previous;
                    if ret == RETURN_HOST {
                        self.sp = STACK_BYTES;
                        self.stopped = true;
                        return Ok(Step::Stop);
                    }
                    self.jump(ret)?;
                    let count = match count {
                        Some(n) => n,
                        None => self.byte()?,
                    };
                    self.sp = old
                        .checked_add(4)
                        .and_then(|v| v.checked_add(count))
                        .filter(|v| *v <= STACK_BYTES - 4)
                        .context("Malie return stack range")?;
                    self.replace(value)?;
                }
                _ => bail!("Malie unknown native opcode {op:#x}"),
            }
            Ok(Step::Continue)
        })();
        result.with_context(|| format!("Malie pc={pc:#x} opcode={op:#x}"))
    }
}
