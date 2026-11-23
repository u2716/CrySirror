// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::source_path::SourceFile;
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::pe::{MAX_PE_BYTES, PeData};
use crysirror_resource::{DecodeContext, DependencyResolver, FormatRegistry};
use std::{collections::BTreeMap, sync::Arc};

struct GameFiles(SourcePath);
impl DependencyResolver for GameFiles {
    fn open(&self, name: &str) -> Result<Arc<dyn AssetReader>> {
        let path = self.0.join(name).canonicalize()?;
        ensure!(
            path.starts_with(&self.0),
            "minori input escaped selected directory"
        );
        Ok(SourceFile::open(path)?.into_reader())
    }
}

fn relative_call(code: &[u8], at: usize) -> Option<usize> {
    let call = code.get(at..at.checked_add(5)?)?;
    if call[0] != 0xe8 { return None; }
    (at + 5).checked_add_signed(i32::from_le_bytes(call[1..].try_into().ok()?) as isize)
}

fn canvas_transition_stage(code: &[u8], selector: usize) -> Option<&'static str> {
    // The stage calls the same draw dispatcher for normal and forced completion.
    // One ABI also forces completion when the signed script speed exceeds 255.
    for (at, block) in code.windows(21).enumerate() {
        if block[0] != 0xe8 || block[5..10] != [0x84,0xc0,0x74,0x0c,0xe9]
            || block[14..17] != [0x6a,0x01,0xe8] { continue; }
        let Some(draw) = relative_call(code, at) else { continue };
        if relative_call(code, at + 16) != Some(draw)
            || !selector.checked_sub(draw).is_some_and(|n| n < 512)
            || !code.get(draw..)?.starts_with(&[0x55,0x8b,0xec,0x83,0xe4,0xf8]) { continue; }
        let Some(before) = at.checked_sub(23).map(|start| &code[start..at]) else { continue };
        if before.starts_with(&[0x8b,0x53,0x0c,0x81,0xba])
            && before[9..18] == [0xff,0,0,0,0x0f,0x9f,0xc0,0x88,0x45]
            && before[19..21] == [0x8b,0x4d] && before[18] == before[21] && before[22] == 0x51 {
            return Some("canvas-fast-finish");
        }
        if before[14..18] == [0x57,0x8b,0xc3,0xe8] && before[22] == 0x57 {
            return Some("canvas");
        }
    }
    None
}

// Original registration and NEXT records are described in the adapter ABI notes.
// Follow the mode-minus-three call in the native transition dispatcher. The
// fixed-grid worker allocates 300 counters; the later worker divides each
// rounded-up canvas dimension by 40. Unrecognized workers stay unsupported.
fn tile_transition_grid(pe: &PeData<'_>) -> Result<Option<&'static str>> {
    let mut grid = None;
    for section in pe.sections().iter().filter(|s| s.characteristics & 0x2000_0000 != 0) {
        let code = pe.rva_slice(section.rva, section.file_size as usize)?;
        for (at, prefix) in code.windows(3).enumerate() {
            if prefix != [0x83, 0xc0, 0xfd] { continue; }
            let tail = &code[at + 3..];
            let call = if tail.starts_with(&[0x50, 0xe8]) { 2 }
                else if tail.starts_with(&[0x8b, 0xce, 0x50, 0xe8]) { 4 }
                else { continue };
            let Some(target) = relative_call(code, at + 2 + call) else { continue };
            let Some(worker) = code.get(target..).and_then(|body| body.get(..96)) else { continue };
            let kind = if call == 2
                && worker.windows(12).any(|s| s == [0x83,0xc1,0x27,0xb8,0x67,0x66,0x66,0x66,0xf7,0xe9,0xc1,0xfa])
                && worker.windows(12).any(|s| s == [0x83,0xc2,0x27,0xb8,0x67,0x66,0x66,0x66,0xf7,0xea,0xc1,0xfa]) {
                let Some(stage) = canvas_transition_stage(code, at) else { continue };
                stage
            } else if call == 4 && worker.windows(9).any(|s| s == [0xbe,0xb0,0x04,0,0,0x85,0xc0,0x74,0x0c])
                && worker.windows(6).any(|s| s == [0x83,0x79,0x20,0,0x53,0x56]) {
                "fixed-800x600"
            } else { continue };
            if grid.is_some_and(|previous| previous != kind) { return Ok(None); }
            grid = Some(kind);
        }
    }
    Ok(grid)
}

// Mode 1 binds two adjacent text surfaces. Match both rectangle assignments
// and the intervening 26-pixel font / 12-pixel leading call in executable code.
fn message_panel_layout(pe: &PeData<'_>) -> Result<Option<&'static str>> {
    let name_rect = [0xc7,0x44,0x24,0x2c,65,0,0,0,0xc7,0x44,0x24,0x34,221,0,0,0,0xff,0xd0];
    let body_rect = [0xc7,0x44,0x24,0x2c,169,0,0,0,0xc7,0x44,0x24,0x34,0xd3,4,0,0,0xff,0xd2];
    for section in pe.sections().iter().filter(|s| s.characteristics & 0x2000_0000 != 0) {
        let code = pe.rva_slice(section.rva, section.file_size as usize)?;
        for (at, op) in code.windows(name_rect.len()).enumerate() {
            if op == name_rect && code.get(at + 27..at + 31) == Some(&[0x6a,12,0x6a,26])
                && code.get(at + 167..at + 185) == Some(&body_rect)
            {
                return Ok(Some("columns-65-169"));
            }
        }
        let shared_rect = [0xc7,0x44,0x24,0x18,120,0,0,0,0xc7,0x44,0x24,0x20,0x88,4,0,0];
        for (at, op) in code.windows(shared_rect.len()).enumerate() {
            if op == shared_rect && code.get(at + 39..at + 44) == Some(&[0x05,0x28,0xff,0xff,0xff])
                && code.get(at + 130..at + 146) == Some(&shared_rect)
                && code.get(at + 194..at + 199) == Some(&[0x05,0x18,0xff,0xff,0xff])
            {
                return Ok(Some("stacked-120"));
            }
        }
    }
    Ok(None)
}

fn scenario_candidate(data: &[u8]) -> bool {
    let Ok(pe) = PeData::parse(data) else {
        return false;
    };
    let nt = u32::from_le_bytes(data[0x3c..0x40].try_into().unwrap()) as usize;
    if data.get(nt + 4..nt + 6) != Some(&0x14cu16.to_le_bytes()[..]) {
        return false;
    }
    let sections = pe.sections();
    let contents = |section: &crysirror_resource::pe::PeSection| {
        let start = section.file_offset as usize;
        &data[start..start + section.file_size as usize]
    };
    // Recover relocated literal addresses from section metadata. Strings in
    // overlays, debug text or a copied installer payload cannot suffice.
    let mut addresses: [Vec<[u8; 4]>; 4] = Default::default();
    for section in sections
        .iter()
        .filter(|s| s.characteristics & 0x40 != 0 && s.characteristics & 0x2000_0000 == 0)
    {
        let mut va = pe.image_base() + u64::from(section.rva);
        for field in contents(section).split_inclusive(|&c| c == 0) {
            let slot = match field {
                b".if\0" => Some(0),
                b".chain\0" => Some(1),
                b".HSVfilter\0" => Some(2),
                b"NEXT\0" => Some(3),
                _ => None,
            };
            if let Some(slot) = slot {
                let Ok(address) = u32::try_from(va) else {
                    return false;
                };
                if addresses[slot].len() >= 16 {
                    return false;
                }
                addresses[slot].push(address.to_le_bytes());
            }
            va += field.len() as u64;
        }
    }
    if addresses.iter().any(Vec::is_empty) {
        return false;
    }
    addresses.iter().all(|candidates| {
        sections
            .iter()
            .filter(|s| s.characteristics & 0x2000_0000 != 0)
            .any(|section| {
                candidates
                    .iter()
                    .any(|addr| contents(section).windows(4).any(|v| v == addr))
            })
    })
}

// The retail constructors register label/select/panel first and end/debug
// last. Each name is a reference into initialized PE data. Two of the native
// string constructors take both the pointer and its length, repeating a name.
fn registration(data: &[u8]) -> Result<Option<Vec<String>>> {
    let pe = PeData::parse(data)?;
    let mut names = std::collections::HashMap::new();
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x40 != 0 && s.characteristics & 0x2000_0000 == 0)
    {
        let mut addr = pe.image_base() + u64::from(section.rva);
        let bytes = pe.rva_slice(section.rva, section.file_size as usize)?;
        for field in bytes.split_inclusive(|b| *b == 0) {
            if let Some(tag) = field.strip_prefix(b".").and_then(|v| v.strip_suffix(&[0])) {
                if (1..=32).contains(&tag.len())
                    && tag[0].is_ascii_alphabetic()
                    && tag.iter().all(|v| v.is_ascii_alphanumeric() || *v == b'_')
                {
                    names.insert(
                        u32::try_from(addr)?.to_le_bytes(),
                        std::str::from_utf8(tag)?,
                    );
                }
            }
            addr += field.len() as u64;
        }
    }
    let mut found = None;
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let bytes = pe.rva_slice(section.rva, section.file_size as usize)?;
        let mut sequence = Vec::new();
        let mut start = 0;
        for (pc, operand) in bytes.windows(4).enumerate() {
            let Some(&name) = names.get(<&[u8; 4]>::try_from(operand)?) else {
                continue;
            };
            if name == "label" {
                sequence.clear();
                start = pc;
            } else if sequence.is_empty() || pc - start > 16384 {
                sequence.clear();
                continue;
            }
            if sequence.last() == Some(&name) {
                continue;
            }
            sequence.push(name);
            if name == "debug" {
                if sequence.starts_with(&["label", "select", "panel", "movie"])
                    && matches!(sequence.get(4), Some(&"movie2" | &"transition"))
                    && sequence.ends_with(&["chain", "end", "debug"])
                {
                    let commands: Vec<String> = sequence.iter().map(|s| (*s).to_owned()).collect();
                    ensure!(
                        found.as_ref().is_none_or(|prior| prior == &commands),
                        "multiple minori command registries"
                    );
                    found = Some(commands);
                }
                sequence.clear();
            }
        }
    }
    Ok(found)
}

// The route selector loads two parallel arrays: six CRects (the pointer
// addresses each right edge) and six button commands of the form !script.sc.
fn route_selector(data: &[u8]) -> Result<Vec<(String, [i32; 4])>> {
    let pe = PeData::parse(data)?;
    let mut result = Vec::new();
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let code = pe.rva_slice(section.rva, section.file_size as usize)?;
        for init in code.windows(21).filter(|w| {
            w[0] == 0xb8
                && w[5..7] == [0x8d, 0x8f]
                && w[11..13] == [0xc7, 0x45]
                && w[18..20] == [0x89, 0x45]
        }) {
            let word = |p: &[u8]| u64::from(u32::from_le_bytes(p.try_into().unwrap()));
            let Some(rect_va) = word(&init[1..5]).checked_sub(8) else {
                continue;
            };
            let Ok(rects) = pe.va_slice(rect_va, 96) else {
                continue;
            };
            let Ok(commands) = pe.va_slice(word(&init[14..18]), 24) else {
                continue;
            };
            let mut routes = Vec::new();
            for (r, ptr) in rects.chunks_exact(16).zip(commands.chunks_exact(4)) {
                let Ok(bytes) = pe.va_slice(word(ptr), 64) else {
                    break;
                };
                let Some(nul) = bytes.iter().position(|&b| b == 0) else {
                    break;
                };
                let Some(file) = bytes[..nul].strip_prefix(b"!") else {
                    break;
                };
                if !file.ends_with(b".sc")
                    || !file
                        .iter()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(b))
                {
                    break;
                }
                let mut bounds = [0i32; 4];
                for (v, b) in bounds.iter_mut().zip(r.chunks_exact(4)) {
                    *v = i32::from_le_bytes(b.try_into()?);
                }
                if bounds[0] < 0
                    || bounds[1] < 0
                    || bounds[2] <= bounds[0]
                    || bounds[3] <= bounds[1]
                    || bounds[2] > 8192
                    || bounds[3] > 8192
                {
                    break;
                }
                routes.push((std::str::from_utf8(file)?.to_owned(), bounds));
            }
            if routes.len() == 6 {
                ensure!(
                    result.is_empty() || result == routes,
                    "ambiguous minori route selector"
                );
                result = routes;
            }
        }
    }
    Ok(result)
}

// The episode picker range-checks a one-based ordinal before looking up the
// scenario in an array of nine pointers. Read the installed table in UI order.
// The system-parameter reader iterates an array of pointers to short voice
// identifiers. "etc" is its fallback entry, and need not be the last one.
fn voice_actors(pe: &PeData<'_>) -> Result<(Vec<String>, Vec<[i32; 2]>)> {
    let mut found = Vec::new();
    let mut table = 0u64;
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x8000_0000 != 0)
    {
        let data = pe.rva_slice(section.rva, section.file_size as usize)?;
        let mut run = Vec::new();
        for (i, word) in data
            .chunks_exact(4)
            .map(Some)
            .chain(std::iter::once(None))
            .enumerate()
        {
            let name = word.and_then(|w| {
                let addr = u64::from(u32::from_le_bytes(w.try_into().ok()?));
                if addr < pe.image_base() {
                    return None;
                }
                let bytes = pe.va_slice(addr, 5).ok()?;
                let len = bytes.iter().position(|b| *b == 0)?;
                let key = &bytes[..len];
                ((2..=3).contains(&len) && key.iter().all(u8::is_ascii_lowercase))
                    .then(|| std::str::from_utf8(key).ok())
                    .flatten()
            });
            if let Some(name) = name {
                run.push(name);
            } else {
                if run.len() >= 3 && run.contains(&"etc") && run.len() > found.len() {
                    table = pe.image_base() + u64::from(section.rva) + ((i - run.len()) * 4) as u64;
                    found = run.iter().map(|key| (*key).to_owned()).collect();
                }
                run.clear();
            }
        }
    }
    let mut marks = Vec::new();
    if !found.is_empty() {
        for section in pe
            .sections()
            .iter()
            .filter(|s| s.characteristics & 0x2000_0000 != 0)
        {
            let code = pe.rva_slice(section.rva, section.file_size as usize)?;
            let addr = u32::try_from(table)?;
            for (i, _) in code
                .windows(4)
                .enumerate()
                .filter(|(_, w)| *w == addr.to_le_bytes())
            {
                if let Some(points) = actor_marks(&code[i..], addr, found.len()) {
                    marks = points;
                    break;
                }
            }
            if !marks.is_empty() {
                break;
            }
        }
    }
    Ok((found, marks))
}

// Each checkbox draw reads the corresponding actor pointer, calls the flag
// getter, then conditionally passes a stack POINT to the bitmap blitter.
fn actor_marks(mut code: &[u8], base: u32, count: usize) -> Option<Vec<[i32; 2]>> {
    fn immediate(bytes: &[u8], before: &[u8]) -> Option<i32> {
        let at = bytes.windows(before.len()).position(|w| w == before)? + before.len();
        i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?).into()
    }
    let mut x = None;
    let mut points = Vec::new();
    for i in 0..count {
        let addr = base.checked_add(u32::try_from(i * 4).ok()?)?.to_le_bytes();
        let at = code.get(..64)?.windows(4).position(|w| w == addr)?;
        code = &code[at + 4..];
        let branch = code
            .get(..28)?
            .windows(3)
            .position(|w| w == [0x84, 0xc0, 0x74])?;
        let end = branch + 4 + usize::from(*code.get(branch + 3)?);
        let body = code.get(branch + 4..end)?;
        if body.len() < 5 || body[body.len() - 5] != 0xe8 {
            return None;
        }
        let y = if let Some(y) = immediate(body, &[0xc7, 0x45, 0xfc]) {
            x = immediate(body, &[0xc7, 0x45, 0xf8]);
            y
        } else {
            if let Some(n) = immediate(&code[..branch], &[0xbe]) {
                x = Some(n);
            }
            if let Some(n) = immediate(body, &[0xc7, 0x44, 0x24, 0x0c]) {
                x = Some(n);
            }
            immediate(body, &[0xc7, 0x44, 0x24, 0x10])?
        };
        let point = [x?, y];
        if !point.iter().all(|v| (0..8192).contains(v)) {
            return None;
        }
        points.push(point);
        code = &code[end..];
    }
    Some(points)
}

// The executable stores six terminated (pose, 10-ms duration) programs and
// an array of pointers to their starts. The last program differs by ABI.
fn eye_sequences(pe: &PeData<'_>) -> Result<Vec<Vec<(u8, u64)>>> {
    let signature = [0i32, 40, 1, 40, 2, 80].map(i32::to_le_bytes).concat();
    let data_sections = pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x40 != 0);
    for section in data_sections.clone() {
        let data = pe.rva_slice(section.rva, section.file_size as usize)?;
        for i in (0..data.len().saturating_sub(signature.len())).step_by(4) {
            if !data[i..].starts_with(&signature) {
                continue;
            }
            let mut at = i;
            let mut starts = Vec::new();
            let mut programs = Vec::new();
            for _ in 0..6 {
                let start = pe.image_base() + u64::from(section.rva) + at as u64;
                starts.extend_from_slice(&u32::try_from(start)?.to_le_bytes());
                let mut program = Vec::new();
                let mut terminated = false;
                while let Some(pair) = data.get(at..at + 8) {
                    let pose = i32::from_le_bytes(pair[..4].try_into()?);
                    let delay = i32::from_le_bytes(pair[4..].try_into()?);
                    at += 8;
                    if pose == -1 {
                        terminated = true;
                        break;
                    }
                    if !(0..=2).contains(&pose)
                        || !(1..=36000).contains(&delay)
                        || program.len() >= 128
                    {
                        break;
                    }
                    program.push((pose as u8, delay as u64));
                }
                if !terminated || program.is_empty() {
                    break;
                }
                programs.push(program);
            }
            if programs.len() == 6 {
                for refs in data_sections.clone() {
                    if pe
                        .rva_slice(refs.rva, refs.file_size as usize)?
                        .windows(starts.len())
                        .step_by(4)
                        .any(|w| w == starts)
                    {
                        return Ok(programs);
                    }
                }
            }
        }
    }
    Ok(Vec::new())
}

fn episode_scripts(pe: &PeData<'_>) -> Result<Vec<String>> {
    let mut found = Vec::new();
    for section in pe
        .sections()
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let code = pe.rva_slice(section.rva, section.file_size as usize)?;
        for op in code
            .windows(37)
            .filter(|w| w.starts_with(&[0x83, 0xf8, 1, 0x0f, 0x8c]))
        {
            if op[9..14] != [0x83, 0xf8, 9, 0x0f, 0x8f]
                || op[27..31] != [0x56, 0x8b, 0x34, 0x85]
                || op[35..37] != [0x56, 0xe8]
            {
                continue;
            }
            let base = u64::from(u32::from_le_bytes(op[31..35].try_into()?)) + 4;
            let mut scripts = Vec::new();
            for ptr in pe.va_slice(base, 36)?.chunks_exact(4) {
                let va = u64::from(u32::from_le_bytes(ptr.try_into()?));
                let bytes = pe.va_slice(va, 64)?;
                let end = bytes
                    .iter()
                    .position(|b| *b == 0)
                    .context("unterminated minori episode name")?;
                let name = &bytes[..end];
                ensure!(
                    name.ends_with(b".sc")
                        && name
                            .iter()
                            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(b)),
                    "invalid minori episode name"
                );
                scripts.push(std::str::from_utf8(name)?.to_owned());
            }
            ensure!(
                found.is_empty() || found == scripts,
                "ambiguous minori episode selector"
            );
            found = scripts;
        }
    }
    Ok(found)
}

fn same_parameter_sections(a: &[u8], b: &[u8]) -> Result<bool> {
    let a = PeData::parse(a)?;
    let b = PeData::parse(b)?;
    let initialized = |s: &&crysirror_resource::pe::PeSection| {
        s.characteristics & 0x40 != 0 && s.characteristics & 0x2000_0000 == 0
    };
    let mut left = a.sections().iter().filter(initialized);
    let mut right = b.sections().iter().filter(initialized);
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ok(a.image_base() == b.image_base()),
            (Some(x), Some(y))
                if x.rva == y.rva
                    && x.file_size == y.file_size
                    && a.rva_slice(x.rva, x.file_size as usize)?
                        == b.rva_slice(y.rva, y.file_size as usize)? =>
            {
                ()
            }
            _ => return Ok(false),
        }
    }
}

pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let Some(script) = find(names, "scr.paz") else {
        return Ok(None);
    };
    let mut candidates = Vec::new();
    let mut inspected = 0u64;
    for (i, exe) in with_ext(names, "exe").enumerate() {
        ensure!(i < 128, "minori executable candidate limit");
        let size = dir.join(exe).metadata()?.len();
        if size < 64 || size > MAX_PE_BYTES as u64 {
            continue;
        }
        inspected += size;
        ensure!(inspected <= 256 << 20, "minori executable scan byte limit");
        let data = read_bounded(dir, exe, MAX_PE_BYTES)?;
        if scenario_candidate(&data) {
            candidates.push((exe, registration(&data)?));
        }
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    let files = Arc::new(GameFiles(dir.canonicalize()?));
    let mut parts = BTreeMap::new();
    let mut mounts = Vec::new();
    for bank in ["scr", "bg", "st", "sys", "voice", "bgm", "se", "mov"] {
        let Some(file) = find(names, &format!("{bank}.paz")) else {
            continue;
        };
        let mut volumes = Vec::new();
        for suffix in b'A'..=b'Z' {
            if let Some(part) = find(names, &format!("{file}{}", char::from(suffix))) {
                ensure!(
                    volumes.len() == usize::from(suffix - b'A'),
                    "gap in split minori bank {file}"
                );
                volumes.push(part);
            }
        }
        parts.insert(bank, volumes);
        mounts.push(ArchiveMount {
            fallback: false,
            source: file.clone(),
            target: bank.into(),
            options: json!({"format":"minori/paz","profile":"minori"}),
        });
    }
    let mut formats = FormatRegistry::default();
    crysirror_formats_arc::register(&mut formats)?;
    let mut selected: Option<(Value, Option<Vec<String>>)> = None;
    let mut failure = None;
    for (exe, commands) in candidates {
        let parameters = json!({"executable":exe,"parts":parts});
        let mut ctx = DecodeContext::new(ClientId(1));
        ctx.limits.max_entry_size = 256 << 20;
        ctx.source_name = Some(script.clone());
        ctx.resolver = Some(files.clone());
        ctx.parameters
            .insert("minori/paz".into(), parameters.clone());
        let verify = || -> Result<()> {
            let (_, archive) = formats.open(files.open(script)?, Some("minori/paz"), &ctx)?;
            let entry = archive
                .entries()
                .iter()
                .find(|e| e.path == "test.sc")
                .context("minori PAZ has no test.sc")?;
            ensure!(entry.size <= 4 << 20, "oversized minori entry scenario");
            let reader = archive.open(entry.id, &ctx)?;
            let bytes = crysirror_resource::read_all(reader.as_ref(), 4 << 20)?;
            crysirror_minori::validate_entry(&bytes)?;
            Ok(())
        };
        match verify() {
            Ok(()) => {
                if let Some((prior, registry)) = &selected {
                    let previous = prior["executable"]
                        .as_str()
                        .context("missing minori executable")?;
                    let original =
                        |name: &str| name.to_ascii_lowercase().replace(".original.exe", ".exe");
                    ensure!(
                        original(previous) == original(exe)
                            && registry == &commands
                            && same_parameter_sections(
                                &read_bounded(dir, previous, MAX_PE_BYTES)?,
                                &read_bounded(dir, exe, MAX_PE_BYTES)?
                            )?,
                        "multiple minori executables match the scenario and script bank"
                    );
                    if !exe.to_ascii_lowercase().ends_with(".original.exe") {
                        continue;
                    }
                }
                selected = Some((parameters, commands));
            }
            Err(e) => failure = Some(e.context(format!("minori script bank with {exe}"))),
        }
    }
    let (parameters, commands) = match selected {
        Some(parameters) => parameters,
        None => {
            return Err(
                failure.unwrap_or_else(|| anyhow::anyhow!("unrecognized minori scenario ABI"))
            );
        }
    };
    let exe = parameters["executable"]
        .as_str()
        .context("missing minori executable")?;
    let executable = read_bounded(dir, exe, MAX_PE_BYTES)?;
    let routes = route_selector(&executable)?;
    let pe = PeData::parse(&executable)?;
    let episodes = episode_scripts(&pe)?;
    let (voice_actors,voice_marks) = voice_actors(&pe)?;
    let eye_sequences = eye_sequences(&pe)?;
    // Full-screen fades share the logical stage dimensions. These assets also
    // carry the authored coordinate space when the host output is scaled.
    let mut canvas = [800u32, 600u32];
    if let Some(bg) = find(names, "bg.paz") {
        let mut ctx = DecodeContext::new(ClientId(1));
        ctx.source_name = Some(bg.clone());
        ctx.resolver = Some(files.clone());
        ctx.parameters
            .insert("minori/paz".into(), parameters.clone());
        let (_, bank) = formats.open(files.open(bg)?, Some("minori/paz"), &ctx)?;
        if let Some(white) = bank
            .entries()
            .iter()
            .find(|entry| entry.path == "white.png")
        {
            let reader = bank.open(white.id, &ctx)?;
            let mut header = [0u8; 24];
            crysirror_resource::read_exact_at(reader.as_ref(), 0, &mut header)?;
            ensure!(
                header.starts_with(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"),
                "invalid minori backdrop header"
            );
            canvas = [
                u32::from_be_bytes(header[16..20].try_into()?),
                u32::from_be_bytes(header[20..24].try_into()?),
            ];
            ensure!(
                canvas.iter().all(|x| (1..=8192).contains(x)),
                "invalid minori stage dimensions"
            );
        }
    }
    Ok(Some(Plan {
        engine: "minori",
        project: project(
            name,
            canvas,
            "minori",
            json!({"abi":crysirror_minori::SCENARIO_ABI,"script":"test.sc","language":"j","canvas":canvas,"commands":commands,"routes":routes,"episodes":episodes,"voice_actors":voice_actors,"voice_marks":voice_marks,"eye_sequences":eye_sequences,"tile_transition_grid":tile_transition_grid(&pe)?,"message_panel_layout":message_panel_layout(&pe)?}),
        ),
        profiles: BTreeMap::from([(
            "minori".into(),
            json!({"limits":{"max_entry_size":256u64<<20},"parameters":{"minori/paz":parameters}}),
        )]),
        mounts,
        evidence: Vec::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Authored PE metadata, command names and address operands. No native code
    // or data from a game is needed to exercise this detector.
    fn fixture(base: u32) -> Vec<u8> {
        let mut data = vec![0; 1536];
        data[..2].copy_from_slice(b"MZ");
        data[60..64].copy_from_slice(&128u32.to_le_bytes());
        data[128..132].copy_from_slice(b"PE\0\0");
        data[132..134].copy_from_slice(&0x14cu16.to_le_bytes());
        data[134..136].copy_from_slice(&2u16.to_le_bytes());
        data[148..150].copy_from_slice(&224u16.to_le_bytes());
        let opt = 152;
        data[opt..opt + 2].copy_from_slice(&0x10bu16.to_le_bytes());
        data[opt + 28..opt + 32].copy_from_slice(&base.to_le_bytes());
        data[opt + 56..opt + 60].copy_from_slice(&0x3000u32.to_le_bytes());
        data[opt + 60..opt + 64].copy_from_slice(&512u32.to_le_bytes());
        for (i, flags) in [0x6000_0020u32, 0x4000_0040].into_iter().enumerate() {
            let sec = opt + 224 + 40 * i;
            data[sec + 8..sec + 12].copy_from_slice(&512u32.to_le_bytes());
            data[sec + 12..sec + 16].copy_from_slice(&(0x1000 * (1 + i as u32)).to_le_bytes());
            data[sec + 16..sec + 20].copy_from_slice(&512u32.to_le_bytes());
            data[sec + 20..sec + 24].copy_from_slice(&(512 * (1 + i as u32)).to_le_bytes());
            data[sec + 36..sec + 40].copy_from_slice(&flags.to_le_bytes());
        }
        let mut offset = 0;
        for (i, literal) in [b".if\0".as_slice(), b".chain\0", b".HSVfilter\0", b"NEXT\0"]
            .into_iter()
            .enumerate()
        {
            let at = 512 + 4 * i;
            data[at..at + 4].copy_from_slice(&(base + 0x2000 + offset as u32).to_le_bytes());
            data[1024 + offset..1024 + offset + literal.len()].copy_from_slice(literal);
            offset += literal.len();
        }
        data
    }

    #[test]
    fn retail_tile_grid_follows_the_call_and_requires_both_canvas_divisions() {
        let mut data = fixture(0x10000000);
        let call = 840;
        let worker = 592;
        data[call..call + 5].copy_from_slice(&[0x83,0xc0,0xfd,0x50,0xe8]);
        data[call + 5..call + 9].copy_from_slice(&((worker as i32) - (call + 9) as i32).to_le_bytes());
        let width = [0x83,0xc1,0x27,0xb8,0x67,0x66,0x66,0x66,0xf7,0xe9,0xc1,0xfa];
        let height = [0x83,0xc2,0x27,0xb8,0x67,0x66,0x66,0x66,0xf7,0xea,0xc1,0xfa];
        data[worker+20..worker+32].copy_from_slice(&width);
        data[worker+48..worker+60].copy_from_slice(&height);
        let draw = 704;
        data[draw..draw+6].copy_from_slice(&[0x55,0x8b,0xec,0x83,0xe4,0xf8]);
        let stage = 955;
        data[stage..stage+21].copy_from_slice(&[
            0xe8,0,0,0,0,0x84,0xc0,0x74,0x0c,0xe9,0,0,0,0,0x6a,1,0xe8,0,0,0,0]);
        for at in [stage, stage+16] {
            data[at+1..at+5].copy_from_slice(&((draw as i32) - (at+5) as i32).to_le_bytes());
        }
        assert_eq!(tile_transition_grid(&PeData::parse(&data).unwrap()).unwrap(), None);
        data[stage-9..stage-5].copy_from_slice(&[0x57,0x8b,0xc3,0xe8]);
        data[stage-1] = 0x57;
        assert_eq!(tile_transition_grid(&PeData::parse(&data).unwrap()).unwrap(), Some("canvas"));
        data[stage-23..stage].copy_from_slice(&[
            0x8b,0x53,0x0c,0x81,0xba,0xe8,2,0,0,0xff,0,0,0,0x0f,0x9f,0xc0,0x88,0x45,0xe0,0x8b,0x4d,0xe0,0x51]);
        assert_eq!(tile_transition_grid(&PeData::parse(&data).unwrap()).unwrap(), Some("canvas-fast-finish"));
        data[stage+17] ^= 1;
        assert_eq!(tile_transition_grid(&PeData::parse(&data).unwrap()).unwrap(), None);
        data[stage+17] ^= 1;
        data[worker+50] = 0;
        assert_eq!(tile_transition_grid(&PeData::parse(&data).unwrap()).unwrap(), None);
        data[call + 5..call + 9].copy_from_slice(&i32::MAX.to_le_bytes());
        assert_eq!(tile_transition_grid(&PeData::parse(&data).unwrap()).unwrap(), None);
    }

    #[test]
    #[ignore = "requires an original PE path and expected transition grid"]
    fn retail_tile_grid_from_installed_program() {
        let path = std::env::var_os("MINORI_TILE_PE").expect("set MINORI_TILE_PE");
        let expected = std::env::var("MINORI_TILE_GRID").expect("set MINORI_TILE_GRID (or none)");
        let bytes = std::fs::read(path).unwrap();
        let grid = tile_transition_grid(&PeData::parse(&bytes).unwrap()).unwrap();
        assert_eq!(grid, (expected != "none").then_some(expected.as_str()));
    }

    #[test]
    fn relocated_constructor_reports_only_registered_names() {
        let base = 0x10000000;
        let mut data = fixture(base);
        let names = [
            "label", "select", "panel", "movie", "movie2", "if", "chain", "end", "debug",
        ];
        let mut at = 1120;
        let mut pc = 544;
        for name in names {
            let text = format!(".{name}\0");
            data[at..at + text.len()].copy_from_slice(text.as_bytes());
            let va = (base + 0x2000 + at as u32 - 1024).to_le_bytes();
            for _ in 0..2 {
                data[pc..pc + 4].copy_from_slice(&va);
                pc += 4;
            }
            at += text.len();
        }
        data[at..at + 7].copy_from_slice(b".never\0");
        assert_eq!(registration(&data).unwrap().unwrap(), names);
        data[pc - 8..pc].fill(0);
        assert!(registration(&data).unwrap().is_none());
    }

    #[test]
    fn scenario_probe_follows_section_addresses_and_requires_next_reference() {
        for base in [0x400000, 0x10000000] {
            let mut data = fixture(base);
            assert!(scenario_candidate(&data));
            data[524..528].fill(0);
            assert!(!scenario_candidate(&data));
        }
    }
}
