// SPDX-License-Identifier: MPL-2.0

//! Verified VM value/stack conventions. The bounded executor lives in `machine`.

#[derive(Clone, Copy, Debug)]
pub(super) struct StackError(pub &'static str);

/// Strings are 32-bit tagged offsets, not native host pointers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringRef {
    Text(u32),
    ScriptData(u32),
    StringBank(u32),
    Frame(u32),
}

impl StringRef {
    pub fn decode(raw: u32) -> Self {
        let offset = raw & 0x3fff_ffff;
        match raw >> 30 {
            0 => Self::Text(offset),
            1 => Self::ScriptData(offset),
            2 => Self::StringBank(offset),
            _ => Self::Frame(offset),
        }
    }
}

/// Return word from the system dispatcher, as consumed by the main VM loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemResult(pub u16);

impl SystemResult {
    pub fn yields(self) -> bool {
        self.0 & 0x8000 != 0
    }
    pub fn advances_pc(self) -> bool {
        self.0 & 0x4000 != 0
    }
    pub fn increments_repeat_counter(self) -> bool {
        self.0 & 0x2000 != 0
    }
    /// The original VM consumes this byte count only when bit 14 is set.
    pub fn cleanup_bytes(self) -> usize {
        if self.advances_pc() {
            usize::from(self.0 as u8)
        } else {
            0
        }
    }
}

/// Frame operand address: the original adds or subtracts `(int)raw` to `(int)frame[depth]`
/// (0x1400652e0/0x1400657b0), so raw values with bit 31 set reach the other side of the base.
pub(crate) fn frame_offset(base: usize, raw: u32, below_base: bool) -> Option<usize> {
    let (base, raw) = (i64::try_from(base).ok()?, i64::from(raw as i32));
    let offset = if below_base { base - raw } else { base + raw };
    usize::try_from(offset).ok()
}

/// Previous contents of a stack word; None marks a byte that was never written.
pub type StackWord = [Option<u8>; 4];

/// Explicit byte stack: return flags encode byte counts, not counts of values.
/// 0x0440/0x442 reserve bytes without writing them; the original leaves stale buffer
/// contents there, which this model keeps unknown instead of guessing.
#[derive(Debug, Clone)]
pub struct ParameterStack {
    bytes: Vec<u8>,
    written: Vec<bool>,
    limit: usize,
}

#[cfg(feature = "runtime")]
impl ParameterStack {
    pub(super) fn snapshot(&self) -> serde_json::Value {
        let written: Vec<u8> = self.written.iter().map(|&set| u8::from(set)).collect();
        serde_json::json!({
            "bytes": super::snapshot::hex(&self.bytes),
            "written": super::snapshot::hex(&written),
            "limit": self.limit,
        })
    }

    pub(super) fn from_snapshot(value: &serde_json::Value) -> Option<Self> {
        use super::snapshot::{bytes, word};
        let data = bytes(value, "bytes")?;
        let written: Vec<bool> = bytes(value, "written")?.into_iter().map(|set| set != 0).collect();
        (data.len() == written.len()).then_some(Self {
            bytes: data,
            written,
            limit: word(value, "limit")? as usize,
        })
    }
}

impl ParameterStack {
    /// Caller chooses a bounded research policy; this does not infer original capacity.
    pub fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            written: Vec::new(),
            limit,
        }
    }
    pub fn len_bytes(&self) -> usize {
        self.bytes.len()
    }
    pub fn push_u32(&mut self, value: u32) -> Result<(), StackError> {
        self.reserve(value, 4)
    }
    /// 0x0440/0x442 (0x140067930): write `value` at SP, then advance SP by `count` bytes.
    /// Only the bytes below the new SP are kept; bytes past the first word stay unknown.
    pub fn reserve(&mut self, value: u32, count: usize) -> Result<(), StackError> {
        if self.limit.saturating_sub(self.bytes.len()) < count {
            return Err(StackError("parameter stack limit"));
        }
        self.bytes
            .try_reserve(count)
            .map_err(|_| StackError("parameter stack allocation failed"))?;
        self.written
            .try_reserve(count)
            .map_err(|_| StackError("parameter stack allocation failed"))?;
        let value = value.to_le_bytes();
        let known = count.min(4);
        self.bytes.extend_from_slice(&value[..known]);
        self.written.resize(self.bytes.len(), true);
        self.bytes.resize(self.bytes.len() + count - known, 0);
        self.written.resize(self.bytes.len(), false);
        Ok(())
    }
    fn word(&self, start: usize) -> Result<u32, StackError> {
        if self.written[start..start + 4].contains(&false) {
            return Err(StackError("read of stack bytes reserved but never written"));
        }
        Ok(u32::from_le_bytes(
            self.bytes[start..start + 4]
                .try_into()
                .expect("checked word"),
        ))
    }
    /// distance=4 reads the last pushed word; distance=8 reads the preceding word.
    pub fn peek_u32(&self, distance: usize) -> Result<u32, StackError> {
        if distance < 4 {
            return Err(StackError("invalid stack distance"));
        }
        let start = self
            .bytes
            .len()
            .checked_sub(distance)
            .ok_or(StackError("parameter stack underflow"))?;
        self.word(start)
    }
    fn live(&self, offset: usize) -> Result<(), StackError> {
        match offset.checked_add(4) {
            Some(end) if end <= self.bytes.len() => Ok(()),
            _ => Err(StackError("stack access outside live bytes")),
        }
    }
    /// Absolute byte offset from the stack base, as used by frame-relative operands.
    pub fn read_u32_at(&self, offset: usize) -> Result<u32, StackError> {
        self.live(offset)?;
        self.word(offset)
    }
    /// Overwrites a live word in place, including never-written bytes, and returns the
    /// previous contents so a failed transaction can restore them exactly.
    pub fn write_u32_at(&mut self, offset: usize, value: u32) -> Result<StackWord, StackError> {
        self.live(offset)?;
        let old = [0, 1, 2, 3].map(|i| self.written[offset + i].then_some(self.bytes[offset + i]));
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        self.written[offset..offset + 4].fill(true);
        Ok(old)
    }
    /// Undo of `write_u32_at`, restoring unknown bytes as unknown.
    pub(crate) fn restore_at(&mut self, offset: usize, old: StackWord) {
        self.restore_bytes(offset, &old);
    }
    pub(super) fn c_string(&self, offset: usize) -> Result<&[u8], StackError> {
        let bytes = self
            .bytes
            .get(offset..)
            .ok_or(StackError("frame string outside stack"))?;
        for (len, (&byte, &known)) in bytes.iter().zip(&self.written[offset..]).enumerate() {
            if !known {
                return Err(StackError("read of stack bytes reserved but never written"));
            }
            if byte == 0 {
                return Ok(&bytes[..len]);
            }
        }
        Err(StackError("unterminated frame string"))
    }
    pub(super) fn initialized_byte(&self, offset: usize) -> Result<u8, StackError> {
        match self.written.get(offset) {
            Some(true) => Ok(self.bytes[offset]),
            Some(false) => Err(StackError("read of stack bytes reserved but never written")),
            None => Err(StackError("fixed-width frame read outside stack")),
        }
    }
    pub(super) fn write_c_string(
        &mut self,
        offset: usize,
        bytes: &[u8],
    ) -> Result<Vec<Option<u8>>, StackError> {
        let end = offset
            .checked_add(bytes.len() + 1)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(StackError("frame string write outside live bytes"))?;
        let old = (offset..end)
            .map(|i| self.written[i].then_some(self.bytes[i]))
            .collect();
        self.bytes[offset..end - 1].copy_from_slice(bytes);
        self.bytes[end - 1] = 0;
        self.written[offset..end].fill(true);
        Ok(old)
    }
    pub(super) fn restore_bytes(&mut self, offset: usize, old: &[Option<u8>]) {
        for (i, byte) in old.iter().enumerate() {
            self.bytes[offset + i] = byte.unwrap_or_default();
            self.written[offset + i] = byte.is_some();
        }
    }
    pub fn discard(&mut self, count: usize) -> Result<(), StackError> {
        let length = self
            .bytes
            .len()
            .checked_sub(count)
            .ok_or(StackError("parameter stack underflow"))?;
        self.bytes.truncate(length);
        self.written.truncate(length);
        Ok(())
    }
    pub fn apply_cleanup(&mut self, result: SystemResult) -> Result<(), StackError> {
        self.discard(result.cleanup_bytes())
    }
}
