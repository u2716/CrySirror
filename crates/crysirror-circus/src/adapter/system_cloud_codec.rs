//! DC4PH account wire/save policy (427200/426300/455770/455AA0).
//! HTTP, MD5, storage and text encodings are shared capabilities. This module
//! only retains the original field order, source quirks and native byte ABI.
use super::*;
use crate::settings::SystemData;
#[derive(Clone)]
pub(super) struct CloudMetadata {
    pub endpoint: String,
    pub game: String,
    pub storage_game: String,
    pub action: String,
    pub verify: String,
    pub identity: String,
    pub mail: String,
    pub document: String,
    pub data: String,
    pub report_template: String,
    pub token_keys: [String; 4],
    pub report_keys: [String; 4],
    pub default_names: [Vec<u8>; 2],
}
impl CloudMetadata {
    pub(super) fn from_pe(pe: &crysirror_resource::pe::PeData<'_>) -> Result<Self> {
        let read = |va: u64| -> Result<String> {
            let mut bytes = Vec::new();
            for n in 0..512 {
                let byte = pe.va_slice(va + n, 1)?[0];
                if byte == 0 {
                    return Ok(String::from_utf8(bytes)?);
                }
                bytes.push(byte);
            }
            bail!("Circus cloud metadata terminator")
        };
        //Addresses are data VAs in the already fingerprinted PE, not a new
        //endpoint/identity policy. No protocol salt or user data is embedded.
        let format = read(0x4929c8)?;
        let tokens = format
            .strip_prefix("%s")
            .context("Circus cloud URL template prefix")?
            .split('&')
            .map(|field| {
                field
                    .split_once('=')
                    .map(|(key, _)| key.to_owned())
                    .context("Circus cloud URL field template")
            })
            .collect::<Result<Vec<_>>>()?;
        let token_keys: [String; 4] = tokens
            .try_into()
            .map_err(|_| anyhow::anyhow!("Circus cloud URL field count"))?;
        Ok(Self {
            endpoint: read(0x4927e4)?,
            game: read(0x48b94c)?,
            storage_game: read(0x48c9ec)?,
            action: read(0x48c4b4)?,
            verify: read(0x48c4b8)?,
            identity: read(0x48c4b0)?,
            mail: read(0x48c4ac)?,
            document: read(0x48c9dc)?,
            data: read(0x48ca18)?,
            report_template: read(0x48b84c)?,
            token_keys,
            report_keys: [
                read(0x48b8ac)?,
                read(0x48b8a4)?,
                read(0x48b878)?,
                read(0x48b844)?,
            ],
            default_names: [
                pe.va_slice(0x48b934, 4)?.to_vec(),
                pe.va_slice(0x48b92c, 4)?.to_vec(),
            ],
        })
    }
}
pub(super) fn digest(bytes: &[u8]) -> String {
    crysirror_resource::checksum::md5_legacy(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn radix(mut value: i32, base: i32) -> String {
    //453AD0 negative values fall straight through453950's decimal itoa case.
    if value < base {
        return value.to_string();
    }
    let mut bytes = Vec::new();
    while value != 0 {
        let digit = (value % base) as u8;
        bytes.push(if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        });
        value /= base;
    }
    bytes.reverse();
    String::from_utf8(bytes).expect("ASCII native radix")
}
pub(super) fn token(time: &crysirror_core::service::CivilTime, game: &str) -> (String, String) {
    let seconds = (time.unix_ms / 1000) as i32;
    (
        format!(
            "{}{}{}{}",
            radix(seconds, 29),
            radix(time.year, 27),
            digest(b"10"),
            digest(game.as_bytes())
        ),
        radix(seconds, 24),
    )
}
pub(super) fn cp932(text: &str) -> Result<Vec<u8>> {
    let (bytes, _, error) = encoding_rs::SHIFT_JIS.encode(text);
    ensure!(!error && !text.contains('\0'), "Circus account CP932 text");
    Ok(bytes.into_owned())
}
pub(super) fn ansi_url(bytes: &[u8]) -> String {
    let mut text = String::new();
    for byte in bytes {
        if byte.is_ascii() {
            text.push(char::from(*byte));
        } else {
            text.push_str(&format!("%{byte:02X}"));
        }
    }
    text
}
pub(super) fn response_ansi(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    for ch in text.chars() {
        let s = ch.to_string();
        let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(&s);
        if error {
            out.push(b'?')
        } else {
            out.extend_from_slice(&encoded)
        }
    }
    out
}
fn cstring(bytes: &[u8], at: usize, max: usize) -> Result<&[u8]> {
    let tail = bytes
        .get(at..at + max)
        .context("Circus cloud native string extent")?;
    Ok(&tail[..tail
        .iter()
        .position(|byte| *byte == 0)
        .context("Circus cloud native string terminator")?])
}
fn upload_text(bytes: &[u8]) -> Result<String> {
    let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
    ensure!(!error, "Circus cloud saved CP932 text");
    //407580 first bounds the UTF16 count to260, then requires UTF8+NUL<260.
    Ok(
        if text.encode_utf16().count() + 1 <= 260 && text.len() + 1 < 260 {
            text.into_owned()
        } else {
            String::new()
        },
    )
}
fn word(bytes: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(at..at + 2)
            .context("Circus cloud word extent")?
            .try_into()?,
    ))
}
fn dword(bytes: &[u8], at: usize) -> Result<i32> {
    Ok(i32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .context("Circus cloud dword extent")?
            .try_into()?,
    ))
}
fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn put32(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn csv_bytes(out: &mut String, bytes: &[u8]) {
    for byte in bytes {
        out.push_str(&format!("{byte},"));
    }
}
fn rle(out: &mut String, bytes: &[u8]) {
    let mut at = 0;
    while at < bytes.len() {
        let value = bytes[at];
        let end = at + bytes[at..].iter().take_while(|b| **b == value).count();
        out.push_str(&format!("{value}|{},", end - at));
        at = end;
    }
}
pub(super) fn upload(system: &[u8], slots: &[(usize, Vec<u8>)]) -> Result<Vec<u8>> {
    ensure!(
        system.len() == crate::settings::SYSTEM_BYTES,
        "Circus cloud system extent"
    );
    let mut text = String::new();
    let mut output = Vec::new();
    for (slot, body) in slots {
        ensure!(
            body.len() == crate::native_save::BODY_BYTES && *slot < 5,
            "Circus cloud slot extent"
        );
        output.extend(format!("SAVEDATA{slot:02}::").bytes());
        //427200 copies the scene bytes directly; only the following three
        //display strings pass through407580's CP932→UTF8 conversion.
        output.extend_from_slice(cstring(body, 8, 260)?);
        text.push_str(&format!(",{},", dword(body, 2812)?));
        for (at, max) in [(246224, 110100), (245960, 264), (245940, 20)] {
            text.push_str(&upload_text(cstring(body, at, max)?)?);
            text.push(',');
        }
        csv_bytes(&mut text, &body[100064..100164]);
        text.push_str("SCEREAD;");
        let last = dword(body, 86908)?;
        ensure!(last < 650, "Circus cloud run count");
        for n in 0..=last {
            let at = 86912 + n as usize * 14;
            let scene = word(body, at)? as i16;
            if scene != -1 {
                text.push_str(&format!(
                    "{scene}!{}|{},;",
                    word(body, at + 4)?,
                    i32::from(word(body, at + 6)?) - 1
                ));
            }
        }
        text.push_str("END;");
        output.extend_from_slice(text.as_bytes());
        text.clear();
    }
    text.push_str("SYSDATA::");
    csv_bytes(&mut text, &system[316..366]);
    text.push_str("END;");
    for (label, at, len) in [
        ("SYSCG::", 784, 1200),
        ("SYSBGM::", 384, 400),
        ("SYSSCE::", 1984, 650),
    ] {
        text.push_str(label);
        rle(&mut text, &system[at..at + len]);
        text.push_str("END;");
    }
    text.push_str("SYSREAD::");
    //427200 intentionally sends the four inline WORD pairs only, even when
    //the read-history directory entry points at an overflow list.
    for scene in 0..650 {
        for n in 0..4 {
            let at = 2640 + scene * 20 + n * 4;
            let lo = word(system, at)?;
            let hi = word(system, at + 2)?;
            if lo != 0 || hi != 0 {
                if n != 0 {
                    text.push('!');
                }
                text.push_str(&format!("{lo}|{hi},"));
            }
        }
        text.push(';');
    }
    text.push_str("END;");
    output.extend(text.bytes());
    Ok(output)
}
fn section<'a>(bytes: &'a [u8], label: &[u8]) -> Option<&'a [u8]> {
    let start = bytes.windows(label.len()).position(|v| v == label)? + label.len();
    let tail = &bytes[start..];
    let end = tail.windows(4).position(|v| v == b"END;")?;
    Some(&tail[..end])
}
fn integer(bytes: &[u8]) -> i32 {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start();
    let sign = if text.starts_with('-') { -1i32 } else { 1 };
    text.trim_start_matches(['+', '-'])
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i32, |v, b| {
            v.wrapping_mul(10).wrapping_add(i32::from(b - b'0'))
        })
        .wrapping_mul(sign)
}
fn unpack(bytes: &[u8], count: usize) -> Vec<u8> {
    let mut values = Vec::new();
    for part in bytes.split(|b| *b == b',').filter(|v| !v.is_empty()) {
        let mut pair = part.splitn(2, |b| *b == b'|');
        let value = integer(pair.next().unwrap()) as u8;
        let repeat = pair.next().map_or(1, integer).max(0) as usize;
        values.extend(std::iter::repeat_n(value, repeat.min(count - values.len())));
        if values.len() == count {
            break;
        }
    }
    values.resize(count, 0);
    values
}
fn copy_cstring(body: &mut [u8], at: usize, capacity: usize, value: &[u8]) -> Result<()> {
    ensure!(
        value.len() < capacity && !value.contains(&0),
        "Circus cloud string exceeds native field"
    );
    body[at..at + value.len()].copy_from_slice(value);
    body[at + value.len()] = 0;
    Ok(())
}
pub(super) fn download(
    response: &[u8],
    system: &[u8],
    faces: &[u8],
    meta: &CloudMetadata,
) -> Result<(Vec<u8>, Vec<(usize, Vec<u8>)>)> {
    ensure!(
        system.len() == crate::settings::SYSTEM_BYTES && faces.len() == 336,
        "Circus cloud merge extent"
    );
    let response = response_ansi(response);
    let bytes = &response;
    let mut system = SystemData::try_from(system.to_vec())?;
    if let Some(data) = section(bytes, b"SYSDATA::") {
        let values = unpack(data, 50);
        for (n, value) in values.into_iter().enumerate() {
            let at = 316 + n;
            system.set_byte(at, system.byte(at).max(value));
        }
    }
    for (label, at, len) in [
        (b"SYSCG::".as_slice(), 784, 1200),
        (b"SYSBGM::".as_slice(), 384, 400),
        (b"SYSSCE::".as_slice(), 1984, 650),
    ] {
        if let Some(data) = section(bytes, label) {
            for (n, value) in unpack(data, len).into_iter().enumerate() {
                let at = at + n;
                if system.byte(at) != 0 || value != 0 {
                    system.set_byte(at, 1);
                }
            }
        }
    }
    if let Some(data) = section(bytes, b"SYSREAD::") {
        for (scene, row) in data.split(|b| *b == b';').take(650).enumerate() {
            for pair in row.split(|b| *b == b'!') {
                let mut pieces = pair.splitn(2, |b| *b == b'|');
                let lo = integer(pieces.next().unwrap());
                if let Some(hi) = pieces.next() {
                    let hi = integer(hi.split(|b| *b == b',').next().unwrap());
                    if lo != 0 {
                        system.mark_read(scene, lo as u16, hi as u16)?;
                    }
                }
            }
        }
    }
    let mut slots = Vec::new();
    for slot in 0..5 {
        let label = format!("SAVEDATA{slot:02}::");
        let Some(data) = section(bytes, label.as_bytes()) else {
            continue;
        };
        let mut fields = data.splitn(6, |b| *b == b',');
        let scene = fields.next().context("Circus cloud slot scene")?;
        let label = integer(fields.next().context("Circus cloud slot label")?);
        let text = fields.next().context("Circus cloud slot text")?;
        let name = fields.next().context("Circus cloud slot name")?;
        let time = fields.next().context("Circus cloud slot time")?;
        let tail = fields.next().context("Circus cloud slot values")?;
        let split = tail
            .windows(8)
            .position(|v| v == b"SCEREAD;")
            .context("Circus cloud slot read marker")?;
        let values = unpack(&tail[..split], 100);
        let read = &tail[split + 8..];
        let mut body = vec![0; crate::native_save::BODY_BYTES];
        body[1944..2280].copy_from_slice(faces);
        copy_cstring(&mut body, 8, 260, scene)?;
        put32(&mut body, 2812, label);
        copy_cstring(&mut body, 246224, 110100, text)?;
        copy_cstring(&mut body, 245960, 264, name)?;
        copy_cstring(&mut body, 245940, 20, time)?;
        put32(&mut body, 246716, 1);
        let mut runs = 0usize;
        for row in read
            .split(|b| *b == b';')
            .filter(|v| !v.is_empty())
            .take(650)
        {
            let Some(bang) = row.iter().position(|b| *b == b'!') else {
                continue;
            };
            let Some(pipe) = row.iter().position(|b| *b == b'|') else {
                continue;
            };
            let scene = integer(&row[..bang]) as u16;
            let start = integer(&row[bang + 1..pipe]).max(2) as u16;
            let end = integer(row[pipe + 1..].split(|b| *b == b',').next().unwrap()).wrapping_add(1)
                as u16;
            let at = 86912 + runs * 14;
            put16(&mut body, at, scene);
            put16(&mut body, at + 4, start);
            put16(&mut body, at + 6, end);
            runs += 1;
        }
        put32(&mut body, 86908, runs as i32 - 1);
        let mut changed = 0;
        for (n, value) in values.into_iter().enumerate() {
            if value != 0 {
                put16(&mut body, 96560 + changed * 2, n as u16 + 500);
                body[98560 + changed] = value;
                changed += 1;
            }
            body[100064 + n] = value;
        }
        put32(&mut body, 96556, changed as i32);
        put32(
            &mut body,
            246736,
            i32::from(
                !(name == meta.default_names[0].as_slice()
                    && time == meta.default_names[1].as_slice()),
            ),
        );
        slots.push((slot, body));
    }
    Ok((system.bytes().to_vec(), slots))
}
