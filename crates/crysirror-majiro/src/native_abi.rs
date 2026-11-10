// Copyright (c) 2026 CrySirror contributors. MIT.
//! Reviewed PE32 Majiro value/call layouts, identified from executable data.
//! No original code is loaded or executed and no whole-file digest is an ABI.
//!
//! Classic: Akatsuki dispatcher410060/top41DB20/push41DB40/page419500/key41B0B0.
//! Extended: YUKIIRO dispatcher416430/top426BD0/push426BF0/page4219B0/key4239A0.
//! Both read u32 hash+i16 argc; values have16-byte stride, type+8, payload+12;
//! strings begin at object+12. Page arguments and nonempty RCT-password policy
//! are unchanged. Extended uppercase403270 has2048 scratch bytes (classic512).
//! These shapes identify this interface, not complete native-call coverage.
use anyhow::{Context, Result, ensure};
use crysirror_resource::pe::PeData;
mod legacy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeAbi {
    Classic,
    Extended,
}
impl NativeAbi {
    pub fn name(self) -> &'static str {
        match self {
            Self::Classic => "majiro-pe32-classic",
            Self::Extended => "majiro-pe32-extended",
        }
    }
    pub fn string_capacity(self) -> usize {
        match self {
            Self::Classic => 512,
            Self::Extended => 2048,
        }
    }
    pub fn inspect(pe: &PeData<'_>) -> Result<Self> {
        let dos = pe.rva_slice(0, 64)?;
        let header = u32::from_le_bytes(dos[60..64].try_into().unwrap());
        ensure!(
            pe.rva_slice(
                header.checked_add(4).context("Majiro PE header overflow")?,
                2
            )? == 0x14cu16.to_le_bytes()
                && pe.rva_slice(
                    header
                        .checked_add(24)
                        .context("Majiro PE optional header overflow")?,
                    2
                )? == 0x10bu16.to_le_bytes(),
            "Majiro requires a reviewed x86 PE32 address model"
        );
        ensure!(
            pe.image_base() <= u64::from(u32::MAX),
            "Majiro requires a reviewed PE32 address model"
        );
        ensure!(pe.sections().len() <= 32, "Majiro ABI PE section budget");
        let scan_bytes = pe
            .sections()
            .iter()
            .filter(|s| s.characteristics & 0x2000_0000 != 0)
            .try_fold(0u64, |sum, s| sum.checked_add(u64::from(s.file_size)))
            .context("Majiro ABI scan size overflow")?;
        ensure!(
            scan_bytes <= 16 * 1024 * 1024,
            "Majiro ABI executable-section scan budget"
        );
        let mut candidates = 0;
        let mut accepted = None;
        for (abi, dispatcher, top, push, page, uppercase) in SHAPES {
            let dispatcher = Template::new(dispatcher)?;
            let top = Template::new(top)?;
            let push = Template::new(push)?;
            let page = Template::new(page)?;
            let uppercase = Template::new(uppercase)?;
            let key = Template::new(KEY)?;
            let testplay = Template::new(TESTPLAY)?;
            for section in pe
                .sections()
                .iter()
                .filter(|s| s.characteristics & 0x2000_0000 != 0)
            {
                let code = pe.rva_slice(section.rva, section.file_size as usize)?;
                let address = pe.image_base() + u64::from(section.rva);
                for offset in dispatcher.candidates(code) {
                    let Some(captures) =
                        dispatcher.matches(&code[offset..], address + offset as u64, &[])
                    else {
                        continue;
                    };
                    candidates += 1;
                    ensure!(candidates <= 2, "Majiro ABI dispatcher candidate budget");
                    let thread = find(&captures, "thread")?;
                    let state = thread
                        .checked_add(4)
                        .context("Majiro native state address overflow")?;
                    if !writable(pe, thread) || !writable(pe, state) {
                        continue;
                    }
                    let expected = [("thread", thread), ("state", state)];
                    let Some(top_address) = unique(pe, &top, &expected)? else {
                        continue;
                    };
                    let Some(push_address) = unique(pe, &push, &expected)? else {
                        continue;
                    };
                    let helper = [("top", top_address), ("push", push_address)];
                    let Some(key_address) = unique(pe, &key, &helper)? else {
                        continue;
                    };
                    let Some(page_address) = unique(pe, &page, &helper)? else {
                        continue;
                    };
                    let Some(test_address) = unique(pe, &testplay, &helper)? else {
                        continue;
                    };
                    let dispatch = &code[offset..code.len().min(offset + 16384)];
                    let base = address + offset as u64;
                    let Some(upper_address) = branch_call(dispatch, base, 0x266a3c79) else {
                        continue;
                    };
                    if uppercase
                        .matches(
                            pe.va_slice(upper_address, uppercase.size)?,
                            upper_address,
                            &helper,
                        )
                        .is_none()
                    {
                        continue;
                    }

                    let key_calls = key
                        .matches(pe.va_slice(key_address, key.size)?, key_address, &helper)
                        .context("Majiro password ABI shape changed")?;
                    let setter_address = find(&key_calls, "picture_key")?;
                    let setter = Template::new(
                        "8B 44 24 04 85 C0 74 0C 80 38 00 74 07 50 E8 ~password_crc 59 C3",
                    )?;
                    let Some(setter_calls) = setter.matches(
                        pe.va_slice(setter_address, setter.size)?,
                        setter_address,
                        &[],
                    ) else {
                        continue;
                    };
                    let crc_address = find(&setter_calls, "password_crc")?;
                    let crc_body = pe.va_slice(crc_address, 256)?;
                    if !crc_body
                        .windows(4)
                        .any(|v| v == 0xedb88320u32.to_le_bytes())
                    {
                        continue;
                    }
                    let test_calls = testplay
                        .matches(
                            pe.va_slice(test_address, testplay.size)?,
                            test_address,
                            &helper,
                        )
                        .context("Majiro startup mode ABI shape changed")?;
                    let mode_address = find(&test_calls, "mode")?;
                    let mode = Template::new("A1 &startup_mode C3")?;
                    if mode
                        .matches(pe.va_slice(mode_address, mode.size)?, mode_address, &[])
                        .is_none()
                    {
                        continue;
                    }

                    // These audited dispatchers are below16KiB. Stop at the
                    // file-backed code boundary, never cross into PE data.
                    if [
                        (0x7a7b6ed4, key_address),
                        (0x83d81f59, page_address),
                        (0x6e6c641a, test_address),
                        (0x266a3c79, upper_address),
                    ]
                    .into_iter()
                    .all(|(hash, target)| branch_call(dispatch, base, hash) == Some(target))
                    {
                        ensure!(
                            accepted.is_none(),
                            "Majiro executable has ambiguous native ABI structures"
                        );
                        accepted = Some(*abi);
                    }
                }
            }
        }
        if let Some(abi)=legacy::inspect(pe)?{
            ensure!(accepted.is_none(),"Majiro executable has ambiguous native ABI structures");
            accepted=Some(abi);
        }
        accepted.context("Majiro executable native ABI is not a reviewed PE32 structure")
    }
}
const KEY: &str = "E8 ~top 8B 40 0C 83 C0 0C 50 E8 ~picture_key 6A 00 E8 ~push 83 C4 08 C3";
const TESTPLAY: &str = "E8 ~mode 48 F7 D8 1B C0 F7 D8 50 E8 ~push 59 C3";
const SHAPES: &[(NativeAbi, &str, &str, &str, &str, &str)] = &[
    (
        NativeAbi::Classic,
        "81 EC 08 04 00 00 8B 15 &thread 53 55 56 8B 0A 57 8B 01 83 C1 04 89 0A 8B 15 &thread 8B 0A 0F BF 31 83 C1 02 89 0A 8B 3D &thread 8B 4F 08 2B CE 3D 43 0A DB 10",
        "A1 &thread 8B 15 &state 8B 48 08 8B 42 24 C1 E1 04 8D 44 01 F0 C3",
        "A1 &thread 8B 0D &state 56 8B 40 08 8B 51 24 C1 E0 04 03 C2 8B 54 24 08 89 50 0C C7 40 08 03 00 00 00 C7 40 04 00 00 00 00 A1 &thread",
        "53 56 57 E8 ~top 8B 4C 24 10 8B 78 0C 8B 70 FC 83 F9 03 7C 05 8B 58 EC EB 05 BB 01 00 00 00",
        "81 EC 00 02 00 00 E8 ~top 8B 40 0C 8D 4C 24 00 83 C0 0C 50 51",
    ),
    (
        NativeAbi::Extended,
        "B8 10 11 00 00 E8 ~stack_probe 8B 15 &thread 8B 0A 8B 01 83 C1 04 89 0A 8B 15 &thread 8B 0A 53 55 56 0F BF 31 83 C1 02 89 0A 8B 0D &thread 57 8B 79 08 2B FE 3D 43 0A DB 10",
        "A1 &thread 8B 48 08 8B 15 &state 8B 42 24 C1 E1 04 8D 44 01 F0 C3",
        "A1 &thread 8B 40 08 8B 0D &state 8B 51 24 C1 E0 04 03 C2 8B 54 24 04 89 50 0C C7 40 08 03 00 00 00 C7 40 04 00 00 00 00 A1 &thread",
        "53 56 57 E8 ~top 83 7C 24 10 03 8B 78 0C 8B 70 FC 7C 05 8B 58 EC EB 05 BB 01 00 00 00",
        "81 EC 00 08 00 00 E8 ~top 8B 40 0C 83 C0 0C 50 8D 4C 24 04 51",
    ),
];
#[derive(Clone, Copy)]
enum Part {
    Byte(u8),
    Address(&'static str, bool),
}
struct Template {
    parts: Vec<Part>,
    size: usize,
    first: u8,
}
type Captures = Vec<(&'static str, u64)>;
impl Template {
    fn new(pattern: &'static str) -> Result<Self> {
        let mut parts = Vec::new();
        let mut size = 0;
        for word in pattern.split_ascii_whitespace() {
            if let Some(name) = word.strip_prefix('&') {
                parts.push(Part::Address(name, false));
                size += 4;
            } else if let Some(name) = word.strip_prefix('~') {
                parts.push(Part::Address(name, true));
                size += 4;
            } else {
                parts.push(Part::Byte(u8::from_str_radix(word, 16)?));
                size += 1;
            }
        }
        let Some(Part::Byte(first)) = parts.first().copied() else {
            anyhow::bail!("Majiro internal ABI template prefix");
        };
        Ok(Self { parts, size, first })
    }
    fn candidates<'a>(&'a self, bytes: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
        bytes
            .windows(self.size)
            .enumerate()
            .filter_map(|(i, b)| (b[0] == self.first).then_some(i))
    }
    fn matches(
        &self,
        bytes: &[u8],
        address: u64,
        expected: &[(&'static str, u64)],
    ) -> Option<Captures> {
        let mut offset = 0;
        let mut captures = expected.to_vec();
        for part in &self.parts {
            match *part {
                Part::Byte(value) => {
                    if bytes.get(offset) != Some(&value) {
                        return None;
                    }
                    offset += 1;
                }
                Part::Address(name, relative) => {
                    let word = u32::from_le_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?);
                    let value = if relative {
                        relative_target(address, offset + 4, word as i32)?
                    } else {
                        u64::from(word)
                    };
                    if let Some((_, previous)) = captures.iter().find(|(key, _)| *key == name) {
                        if *previous != value {
                            return None;
                        }
                    } else {
                        captures.push((name, value));
                    }
                    offset += 4;
                }
            }
        }
        Some(captures)
    }
}
fn find(captures: &Captures, name: &str) -> Result<u64> {
    captures
        .iter()
        .find_map(|(key, v)| (*key == name).then_some(*v))
        .context("Majiro ABI template capture")
}
fn writable(pe: &PeData<'_>, address: u64) -> bool {
    pe.sections().iter().any(|s| {
        let start = pe.image_base() + u64::from(s.rva);
        s.characteristics & 0x8000_0000 != 0
            && s.characteristics & 0x2000_0000 == 0
            && address >= start
            && address
                .checked_add(4)
                .is_some_and(|end| end <= start + u64::from(s.virtual_size))
    })
}
fn unique(
    pe: &PeData<'_>,
    template: &Template,
    expected: &[(&'static str, u64)],
) -> Result<Option<u64>> {
    let mut found = None;
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let bytes = pe.rva_slice(section.rva, section.file_size as usize)?;
        let base = pe.image_base() + u64::from(section.rva);
        for offset in template.candidates(bytes) {
            if template
                .matches(&bytes[offset..], base + offset as u64, expected)
                .is_some()
            {
                if found.is_some() {
                    return Ok(None);
                }
                found = Some(base + offset as u64);
            }
        }
    }
    Ok(found)
}
fn relative_target(base: u64, next: usize, offset: i32) -> Option<u64> {
    let target = i128::from(base) + next as i128 + i128::from(offset);
    (0..=i128::from(u32::MAX))
        .contains(&target)
        .then_some(target as u64)
}
/// Follow only the bounded equality path from an actual CMP EAX,hash. This
/// recognizes registered native calls, never interprets general guest code.
fn branch_call(code: &[u8], base: u64, hash: u32) -> Option<u64> {
    branch_call_compare(code,base,hash,&[0x3d])
}
/// Reviewed register-specific comparisons only: EAX or the legacy ESI
/// dispatcher. The same equality-path/control-flow budget applies to both.
fn branch_call_compare(code: &[u8],base:u64,hash:u32,prefix:&[u8])->Option<u64>{
    if prefix!=[0x3d] && prefix!=[0x81,0xfe]{return None;}
    let mut cmp=prefix.to_vec();cmp.extend_from_slice(&hash.to_le_bytes());
    let mut found = None;
    for (index, _) in code.windows(cmp.len()).enumerate().filter(|(_, w)| *w == cmp) {
        let mut pc = index + cmp.len();
        let mut target = None;
        for _ in 0..16 {
            let op = *code.get(pc)?;
            match op {
                0xe8 => {
                    target = relative_target(
                        base,
                        pc + 5,
                        i32::from_le_bytes(code.get(pc + 1..pc + 5)?.try_into().ok()?),
                    );
                    break;
                }
                0x50..=0x57 | 0x90 => pc += 1,
                0xe9 => {
                    let next = relative_target(
                        base,
                        pc + 5,
                        i32::from_le_bytes(code.get(pc + 1..pc + 5)?.try_into().ok()?),
                    )?;
                    pc = usize::try_from(next.checked_sub(base)?).ok()?;
                }
                0xeb => {
                    let next = relative_target(base, pc + 2, i32::from(*code.get(pc + 1)? as i8))?;
                    pc = usize::try_from(next.checked_sub(base)?).ok()?;
                }
                0x70..=0x7f | 0x0f => {
                    let (condition, length, offset) = if op == 0x0f {
                        let sub = *code.get(pc + 1)?;
                        if !(0x80..=0x8f).contains(&sub) {
                            break;
                        }
                        (
                            sub & 15,
                            6,
                            i32::from_le_bytes(code.get(pc + 2..pc + 6)?.try_into().ok()?),
                        )
                    } else {
                        (op & 15, 2, i32::from(*code.get(pc + 1)? as i8))
                    };
                    // Flags from CMP(equal): ZF=1, CF=SF=OF=0, PF=1.
                    let taken = match condition {
                        0 | 2 | 5 | 7 | 8 | 0xb | 0xc | 0xf => false,
                        1 | 3 | 4 | 6 | 9 | 0xa | 0xd | 0xe => true,
                        _ => return None,
                    };
                    if taken {
                        let next = relative_target(base, pc + length, offset)?;
                        pc = usize::try_from(next.checked_sub(base)?).ok()?;
                    } else {
                        pc += length;
                    }
                }
                _ => break,
            }
        }
        if let Some(target) = target {
            if found.is_some() {
                return None;
            }
            found = Some(target);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equality_dispatch_follows_only_the_registered_call_and_checks_relative_bounds() {
        // CMP EAX,hash; JG(other); JE(equal); INT3; equal: PUSH ESI; CALL.
        let mut bytes = vec![
            0x3d, 0xd4, 0x6e, 0x7b, 0x7a, 0x7f, 0x40, 0x74, 0x01, 0xcc, 0x56, 0xe8,
        ];
        bytes.extend_from_slice(&0x20i32.to_le_bytes());
        assert_eq!(branch_call(&bytes, 0x401000, 0x7a7b6ed4), Some(0x401030));
        assert_eq!(branch_call(&bytes, 0x401000, 0x7a7b6ed5), None);
        bytes.truncate(14);
        assert_eq!(branch_call(&bytes, 0x401000, 0x7a7b6ed4), None);
        assert_eq!(relative_target(0, 1, -2), None);
        assert_eq!(relative_target(u64::from(u32::MAX), 1, 1), None);
    }
    #[test]
    #[ignore = "requires MAJIRO_ABI_CLASSIC and MAJIRO_ABI_EXTENDED owner PE files; static data only"]
    fn owner_classic_and_extended_pe_layouts_and_unknown_stack_types() {
        for (key, expected) in [
            ("MAJIRO_ABI_CLASSIC", NativeAbi::Classic),
            ("MAJIRO_ABI_EXTENDED", NativeAbi::Extended),
        ] {
            let path = std::env::var_os(key).expect("explicit owner PE path");
            let bytes = std::fs::read(path).unwrap();
            let pe = PeData::parse(&bytes).unwrap();
            assert_eq!(
                NativeAbi::inspect(&pe).unwrap_or_else(|error| panic!("{key}: {error:#}")),
                expected
            );
            // Change the actual typed-value constructor to an unreviewed type.
            // This is an ABI mutation check, not a file-identity comparison.
            let constructor = [0xc7, 0x40, 0x08, 0x03, 0, 0, 0, 0xc7, 0x40, 0x04];
            let shape = SHAPES.iter().find(|row| row.0 == expected).unwrap();
            let push = Template::new(shape.3).unwrap();
            let mut position = None;
            for section in pe
                .sections()
                .iter()
                .filter(|section| section.characteristics & 0x2000_0000 != 0)
            {
                let code = pe
                    .rva_slice(section.rva, section.file_size as usize)
                    .unwrap();
                for offset in push.candidates(code) {
                    if push
                        .matches(
                            &code[offset..],
                            pe.image_base() + u64::from(section.rva) + offset as u64,
                            &[],
                        )
                        .is_some()
                    {
                        let delta = code[offset..offset + push.size]
                            .windows(constructor.len())
                            .position(|v| v == constructor)
                            .unwrap();
                        position = Some(section.file_offset as usize + offset + delta);
                    }
                }
            }
            let position = position.expect("reviewed integer-value constructor");
            let mut unknown = bytes;
            unknown[position + 3] = 7;
            assert!(NativeAbi::inspect(&PeData::parse(&unknown).unwrap()).is_err());
        }
    }
}
