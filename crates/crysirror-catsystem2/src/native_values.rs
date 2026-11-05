// SPDX-License-Identifier: MPL-2.0
//! Original numeric/string substitution policy, never a host pointer or layout.
use crate::kcs::Machine;
use anyhow::{ensure, Context, Result};
/// Native sub6A48F0 integer syntax: no whitespace, optional sign and hex prefix,
/// wrapping 32-bit accumulation, and decimal fractions explicitly become zero.
pub fn integer(text: &str) -> u32 {
    integer_prefix(text.as_bytes()).map_or(0,|(_,value)|value)
}
///6A49D0 accepts a signed wrapping DWORD prefix; decimal dots invalidate the
///whole match, while `0x` without digits still succeeds as zero.
pub(crate) fn integer_prefix(source: &[u8]) -> Option<(usize, u32)> {
    let (mut at, negative) = match source.first()? {
        b'-' => (1, true),
        b'+' => (1, false),
        _ => (0, false),
    };
    if !source.get(at)?.is_ascii_digit() {
        return None;
    }
    let mut value = 0u32;
    if source.get(at) == Some(&b'0') && matches!(source.get(at + 1), Some(b'x' | b'X')) {
        at += 2;
        while let Some(b) = source.get(at) {
            let digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => break,
            };
            value = value.wrapping_mul(16).wrapping_add(digit as u32);
            at += 1;
        }
    } else {
        while let Some(b @ b'0'..=b'9') = source.get(at) {
            value = value.wrapping_mul(10).wrapping_add((b - b'0') as u32);
            at += 1;
        }
        if source.get(at) == Some(&b'.') {
            return None;
        }
    }
    Some((
        at,
        if negative {
            value.wrapping_neg()
        } else {
            value
        },
    ))
}
pub fn format_bytes(machine: &Machine, address: u32) -> Result<Vec<u8>> {
    format_inner(machine, address, 0)
}
pub fn format(machine: &Machine, address: u32) -> Result<String> {
    let bytes = format_bytes(machine, address)?;
    String::from_utf8(bytes).context("unsupported non-UTF8 CatSystem2 native path")
}
/// Original native19/20 signed-char difference, or the reviewed module's
/// native21/22 CRT C-locale unsigned ASCII-folded difference. The raw forms
/// reverse their two arguments; callers retain that ABI order.
pub fn compare(left: &[u8], right: &[u8], insensitive: bool) -> u32 {
    let mut index = 0;
    loop {
        let mut a = left.get(index).copied().unwrap_or(0);
        let mut b = right.get(index).copied().unwrap_or(0);
        if insensitive {
            a = a.to_ascii_lowercase();
            b = b.to_ascii_lowercase();
        }
        if a != b || a == 0 {
            return if insensitive {
                i32::from(a) - i32::from(b)
            } else {
                i32::from(a as i8) - i32::from(b as i8)
            } as u32;
        }
        index += 1;
    }
}
/// Read native42's NUL-list at numeric VM addresses. A negative signed index
/// selects its first element, and crossing the first empty entry stops there.
/// This is VM memory layout policy; delimiter splitting belongs to shared Text.
pub fn nul_list_item(machine: &Machine, source: u32, index: u32) -> Result<Vec<u8>> {
    let mut at = source;
    let mut consumed = 0usize;
    let count = (index as i32).max(0) as usize;
    ensure!(count <= 65536, "CatSystem2 NUL-list index budget");
    for _ in 0..count {
        let bytes = machine.read_cstring(at, 65534usize.saturating_sub(consumed))?;
        if bytes.is_empty() {
            return Ok(bytes);
        }
        consumed = consumed
            .checked_add(bytes.len() + 1)
            .context("CatSystem2 NUL-list byte overflow")?;
        ensure!(consumed <= 65534, "CatSystem2 NUL-list byte budget");
        at = source
            .checked_add(consumed as u32)
            .context("CatSystem2 NUL-list address overflow")?;
    }
    machine.read_cstring(at, 65534usize.saturating_sub(consumed))
}
fn format_inner(machine: &Machine, address: u32, depth: u8) -> Result<Vec<u8>> {
    ensure!(depth < 8, "CatSystem2 native format recursion budget");
    let input = machine.read_cstring(address, 4096)?;
    let mut output = Vec::new();
    let mut i = 0;
    while i < input.len() {
        if input[i] != b'%' {
            ensure!(output.len() < 4096, "CatSystem2 native format output budget");
            output.push(input[i]); i += 1; continue;
        }
        if input.get(i + 1) == Some(&b'%') {
            ensure!(output.len() < 4096, "CatSystem2 native format output budget");
            output.push(b'%'); i += 2; continue;
        }
        let begin = i;
        // 8763B0 scans to '[' before interpreting a printf template. Missing
        // brackets/unknown regions/malformed addresses copy only '%' literally,
        // then resume at the following byte; ordinary command text is valid.
        let placeholder = input[begin + 1..].iter().position(|byte| *byte == b'[').and_then(|relative| {
            let open = begin + 1 + relative;
            let region = *input.get(open + 1)?;
            if !matches!(region, b'G' | b'L' | b'H') { return None; }
            let mut end = open + 2;
            let mut offset = 0u32;
            while let Some(byte @ b'0'..=b'9') = input.get(end) {
                offset = offset.wrapping_mul(10).wrapping_add(u32::from(byte - b'0'));
                end += 1;
            }
            // Native accepts an omitted numeric offset as zero.
            (input.get(end) == Some(&b']')).then_some((open, end + 1, region, offset))
        });
        let Some((open, end, region, offset)) = placeholder else {
            ensure!(output.len() < 4096, "CatSystem2 native format output budget");
            output.push(b'%'); i += 1; continue;
        };
        ensure!(open - begin < 64, "CatSystem2 native printf template budget");
        let at = match region {
            b'G' => offset,
            b'L' => machine.frame_base.checked_add(offset).context("native local format overflow")?,
            b'H' => offset | 0x80000000,
            _ => unreachable!(),
        };
        let template = &input[begin..open];
        let string;
        let argument = if template.iter().any(|byte| matches!(byte, b's' | b'S')) {
            string = format_inner(machine, at, depth + 1)?;
            crysirror_text::printf32::Printf32Argument::Bytes(&string)
        } else {
            // The native reads a float and promotes it for f/F. Shared C32
            // formatting deliberately rejects unsupported floating varargs;
            // it must not reinterpret the word as an integer conversion.
            ensure!(!template.iter().any(|byte| matches!(byte, b'f' | b'F')), "CatSystem2 floating native printf is unsupported");
            crysirror_text::printf32::Printf32Argument::Word(machine.read_word(at)?)
        };
        let formatted = crysirror_text::printf32::format_printf32_bytes(template, &[argument], 4096 - output.len())
            .with_context(|| format!("CatSystem2 native printf at byte {begin}, template {:?}", String::from_utf8_lossy(template)))?;
        output.extend(formatted.bytes);
        i = end;
    }
    // All users consume the formatted result as a C string, including %c=0.
    if let Some(end) = output.iter().position(|byte| *byte == 0) { output.truncate(end); }
    Ok(output)
}
#[cfg(test)]
mod tests {
    #[test]
    fn string_variables_keep_empty_presence_raw_bytes_and_replacement_dirty_state() {
        let mut vars=super::StringVariables::default();
        assert_eq!(vars.get(800),None);assert!(!vars.dirty);
        vars.assign(800,Vec::new()).unwrap();assert_eq!(vars.get(800),Some(b"".as_slice()));assert!(vars.dirty);
        vars.dirty=false;vars.assign(800,vec![0x81,0x5c]).unwrap();assert_eq!(vars.get(800),Some([0x81,0x5c].as_slice()));assert!(vars.dirty);
        assert!(vars.assign(800,vec![1,0,2]).is_err());assert_eq!(vars.get(800),Some([0x81,0x5c].as_slice()));
    }

    #[test]
    fn native_compare_returns_the_complete_original_difference() {
        assert_eq!(super::compare(b"A", b"z", false), (-57i32) as u32);
        assert_eq!(super::compare(b"KCS", b"kcs", true), 0);
        assert_eq!(super::compare(&[0x80], b"", false), (-128i32) as u32);
        assert_eq!(super::compare(&[0x80], b"", true), 128);
        assert_eq!(super::compare(b"ab", b"abc", true), (-99i32) as u32);
    }
    #[test]
    fn nul_list_addresses_preserve_negative_and_empty_terminator_policy() {
        let p = crate::kcs::Program {
            code: vec![0x3b],
            globals_bytes: 256,
            stack_bytes: 256,
            strings: vec![],
        };
        let mut m = crate::kcs::Machine::new(&p, 256).unwrap();
        m.write_bytes(0x80000000, b"first\0second\0\0").unwrap();
        assert_eq!(
            super::nul_list_item(&m, 0x80000000, u32::MAX).unwrap(),
            b"first"
        );
        assert_eq!(super::nul_list_item(&m, 0x80000000, 1).unwrap(), b"second");
        assert_eq!(super::nul_list_item(&m, 0x80000000, 2).unwrap(), b"");
        assert_eq!(super::nul_list_item(&m, 0x80000000, 25).unwrap(), b"");
        assert!(super::nul_list_item(&m, 0x80000000, 65537).is_err());
    }
    #[test]
    fn format_uses_owned_global_local_heap_addresses_and_checks_recursion() {
        let p = crate::kcs::Program {
            code: vec![0x3b],
            globals_bytes: 256,
            stack_bytes: 256,
            strings: vec![],
        };
        let mut m = crate::kcs::Machine::new(&p, 256).unwrap();
        m.write_bytes(0, b"plane/%02d[L12]/%X[H4]/%s[G128]/%%\0")
            .unwrap();
        m.write_bytes(m.frame_base + 12, &3u32.to_le_bytes())
            .unwrap();
        m.write_bytes(0x80000004, &0xabu32.to_le_bytes()).unwrap();
        m.write_bytes(128, b"name\0").unwrap();
        assert_eq!(super::format(&m, 0).unwrap(), "plane/03/AB/name/%");
        m.write_bytes(0, b"%s[G0]\0").unwrap();
        assert!(super::format(&m, 0)
            .unwrap_err()
            .to_string()
            .contains("recursion"));
    }
    #[test]
    fn native_integer_syntax_is_not_rust_parse_or_atoi() {
        for (source, want) in [
            ("12tail", 12),
            (" 12", 0),
            ("12.0", 0),
            ("-0x10", 0xfffffff0),
            ("0XfG", 15),
            ("4294967296", 0),
            ("+2", 2),
        ] {
            assert_eq!(super::integer(source), want, "{source}");
        }
    }
}

///73FE70/740450: original per-KCS numeric string variable bank, not a text
/// layout/formatting engine. Empty assigned strings remain present values.
#[derive(Clone,Debug,Default)]
pub(crate) struct StringVariables {
    values: std::collections::BTreeMap<u32,Vec<u8>>,
    /// VM+180 exists (73FE70 ran: first assignment or a 253 load).
    created:bool,
    pub dirty:bool,
    /// 7136F0's `%LANG%` and `%LANG_M%` substitution.
    pub language_prefix:Vec<u8>,
}
impl StringVariables {
    /// 73FE70: the bank exists even while empty.
    pub fn mark_created(&mut self) {self.created=true;}
    /// 740070: [count][id, NUL-terminated text]... in id order, plus the 8 bytes
    /// 6A4510 allocates beyond the written size. None before 73FE70 built the bank.
    pub fn serialize(&self)->Option<Vec<u8>> {
        if !self.created{return None;}
        let mut out=(self.values.len() as u32).to_le_bytes().to_vec();
        for (id,text) in &self.values{out.extend(id.to_le_bytes());out.extend(text);out.push(0);}
        out.extend([0;8]);
        Some(out)
    }
    /// 740290: the bank is emptied, then every record is assigned again.
    pub fn deserialize(&mut self,bytes:&[u8])->Result<()> {
        let word=|at:usize|->Result<u32>{Ok(u32::from_le_bytes(bytes.get(at..at+4).context("CatSystem2 string bank record")?.try_into().unwrap()))};
        let count=word(0)?;let mut at=4;
        let mut values=std::collections::BTreeMap::new();
        for _ in 0..count{
            let id=word(at)?;at+=4;
            let end=bytes[at..].iter().position(|b|*b==0).context("CatSystem2 string bank text")?;
            values.insert(id,bytes[at..at+end].to_vec());at+=end+1;
        }
        self.values=values;self.created=true;self.dirty=true;Ok(())
    }
    pub fn assign(&mut self,id:u32,bytes:Vec<u8>)->Result<()> {
        self.created=true;
        ensure!(bytes.len()<=65534&&!bytes.contains(&0),"CatSystem2 string variable extent");
        ensure!(self.values.contains_key(&id)||self.values.len()<65536,"CatSystem2 string variable count");
        let resident=self.values.values().map(Vec::len).sum::<usize>()-self.values.get(&id).map_or(0,Vec::len)+bytes.len();
        ensure!(resident<=16*1024*1024,"CatSystem2 string variable byte budget");
        self.values.insert(id,bytes);self.dirty=true;Ok(())
    }
    pub fn get(&self,id:u32)->Option<&[u8]> {self.values.get(&id).map(Vec::as_slice)}
    pub(crate) fn remove(&mut self,id:u32) {if self.values.remove(&id).is_some(){self.dirty=true;}}
    pub fn diagnostics(&self)->serde_json::Value {serde_json::json!({"count":self.values.len(),"bytes":self.values.values().map(Vec::len).sum::<usize>(),"dirty":self.dirty})}
}

