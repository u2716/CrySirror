//! misc methods of 6EF018 beyond host information and package queries.
//!
//! Paths are the native's Windows strings: `efn`/`exp`/`cfe` use the Delphi
//! ExtractFileName/ExtractFilePath/ChangeFileExt rules ('\\' and ':' are
//! separators). File queries resolve `./`-rooted and relative names in the
//! save capability, then the asset capability (gsf maps both native roots to
//! `./`); anything else is outside this client's authority and fails.
use super::*;
use crysirror_core::service::{CivilTimeQuery, CivilTimeState};

pub(super) enum MiscIo {
    Done,
    Exists(bool),
    Digest(Option<[u8; 32]>),
    Build { keys: [String; 4], name: String, digest: [u8; 32] },
}
/// Delphi UnicodeString literal header: code page 1200, element size 2,
/// reference count -1.
const LITERAL: [u8; 8] = [0xB0, 0x04, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0xFF];
/// bsno (6F0764) answers four A-Z literals, each shifted by 13 (503F0C:
/// (c - 65 + 13) mod 26 + 65 for every unit). They are the four literals
/// placed immediately before the method's "564" revision literal; they are
/// read from the game's own executable rather than copied into this source.
pub(super) fn build_keys(exe: &[u8]) -> Result<[String; 4]> {
    let mut literals: Vec<(usize, usize, String)> = Vec::new();
    let mut at = 0;
    while let Some(found) = exe[at..].windows(8).position(|w| w == LITERAL) {
        let start = at + found;
        at = start + 1;
        let Some(len) = exe.get(start + 8..start + 12) else { break };
        let len = u32::from_le_bytes(len.try_into()?) as usize;
        let body = start + 12;
        let Some(units) = exe.get(body..body.saturating_add(len.saturating_mul(2))) else { continue };
        if len == 0 || len > 4096 || exe.get(body + len * 2..body + len * 2 + 2) != Some(&[0, 0][..]) {
            continue;
        }
        let units: Vec<u16> = units.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        if let Ok(text) = String::from_utf16(&units) {
            literals.push((start, body + len * 2 + 2, text));
        }
    }
    for i in 4..literals.len() {
        if literals[i].2 != "564" {
            continue;
        }
        let run = &literals[i - 4..i];
        let adjacent = run.iter().zip(&literals[i - 3..=i]).all(|(a, b)| b.0 >= a.1 && b.0 - a.1 < 16);
        if adjacent && run.iter().all(|(_, _, t)| t.len() >= 16 && t.bytes().all(|c| c.is_ascii_uppercase())) {
            let shift = |t: &str| t.bytes().map(|c| char::from((c - b'A' + 13) % 26 + b'A')).collect::<String>();
            return Ok([shift(&run[0].2), shift(&run[1].2), shift(&run[2].2), shift(&run[3].2)]);
        }
    }
    bail!("QLIE bsno literals not found in the original executable")
}
pub(super) struct MiscPending {
    owner: (String, usize, String),
    task: Option<Task<MiscIo>>,
    clock: Option<Box<dyn CivilTimeQuery>>,
}

fn last_separator(path: &str) -> Option<usize> {
    path.rfind(['\\', ':'])
}
/// Delphi ExtractFileName.
pub(super) fn extract_file_name(path: &str) -> &str {
    last_separator(path).map_or(path, |i| &path[i + 1..])
}
/// Delphi ExtractFilePath (keeps the trailing separator).
fn extract_file_path(path: &str) -> &str {
    last_separator(path).map_or("", |i| &path[..=i])
}
/// Delphi ChangeFileExt: replaces the extension found after the last
/// separator, or appends.
fn change_file_ext(path: &str, ext: &str) -> String {
    let dot = path.rfind(['.', '\\', ':']).filter(|i| path[*i..].starts_with('.'));
    format!("{}{ext}", &path[..dot.unwrap_or(path.len())])
}
/// Uppercase hex of the SHA-256 state words (54BF80: IntToHex(word, 8)).
fn sha_hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|b| format!("{b:02X}")).collect()
}
/// UTF-16LE code units of a Delphi UnicodeString.
fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}
/// Windows LANGID (primary, sublanguage) of a locale tag; 0/0 is
/// LANG_NEUTRAL, used when the host reports no locale.
fn lang_id(locale: Option<&str>) -> (i32, i32) {
    let Some(tag) = locale else { return (0, 0) };
    let lower = tag.to_ascii_lowercase();
    let (lang, region) = lower.split_once('-').unwrap_or((lower.as_str(), ""));
    match (lang, region) {
        ("ja", _) => (0x11, 1),
        ("en", "gb") => (0x09, 2),
        ("en", "au") => (0x09, 3),
        ("en", "ca") => (0x09, 4),
        ("en", _) => (0x09, 1),
        ("zh", "tw") | ("zh", "hant") => (0x04, 1),
        ("zh", "hk") => (0x04, 3),
        ("zh", "sg") => (0x04, 4),
        ("zh", _) => (0x04, 2),
        ("ko", _) => (0x12, 1),
        ("fr", "ca") => (0x0C, 3),
        ("fr", _) => (0x0C, 1),
        ("de", _) => (0x07, 1),
        ("es", "mx") => (0x0A, 2),
        ("es", _) => (0x0A, 3),
        ("it", _) => (0x10, 1),
        ("ru", _) => (0x19, 1),
        ("pt", "br") => (0x16, 1),
        ("pt", _) => (0x16, 2),
        _ => (0, 0),
    }
}
/// 6C8074: 31 rounds of xor 0xE379DF84 then a 33-bit rotate-through-carry
/// right by 5 with the carry cleared by the xor.
fn guid_mix(mut v: u32) -> u32 {
    for _ in 0..31 {
        let wide = u64::from(v ^ 0xE379DF84); // carry bit 32 = 0
        let rotated = ((wide >> 5) | (wide << 28)) & 0x1_FFFF_FFFF;
        v = rotated as u32;
    }
    v
}

/// 4E5284: XOR the UTF-16 units with the key units, cycling the key.
fn key_xor(units: &mut [u16], key: &[u16]) {
    if key.is_empty() {
        return;
    }
    for (i, u) in units.iter_mut().enumerate() {
        *u ^= key[i % key.len()];
    }
}
/// 4E4E20 keystream: seed = (n*n ^ n ^ 0x3E13 ^ hi16(s) ^ lo16(s)) as u16,
/// then v = v18 + i + 8*v (16-bit) per unit.
fn stream_xor(units: &mut [u16], seed: u32) {
    let n = units.len() as u32;
    let mix = ((seed >> 16) ^ seed) & 0xFFFF;
    let start = ((n.wrapping_mul(n)) ^ n ^ 0x3E13 ^ mix) as u16;
    let mut v = start;
    for (i, u) in units.iter_mut().enumerate() {
        v = start.wrapping_add(i as u16).wrapping_add(v.wrapping_mul(8));
        *u ^= v;
    }
}
/// code encode (4E5384): key XOR, the length-prefixed keystream record
/// (seed | 0xF58D3F1), uppercase hex of every byte.
fn code_encode(text: &str, key: &str, seed: u32) -> String {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    key_xor(&mut units, &key.encode_utf16().collect::<Vec<_>>());
    stream_xor(&mut units, seed | 0x0F58_D3F1);
    let mut bytes = (units.len() as u16).to_le_bytes().to_vec();
    bytes.extend(units.iter().flat_map(|u| u.to_le_bytes()));
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}
/// code decode (4E5448): the inverse of `code_encode`; malformed input
/// decodes to an empty string.
fn code_decode(hex: &str, key: &str, seed: u32) -> String {
    let chars: Vec<char> = hex.chars().collect();
    let bytes: Vec<u8> = chars
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(&p.iter().collect::<String>(), 16).unwrap_or(0))
        .collect();
    if bytes.len() < 2 {
        return String::new();
    }
    let n = usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));
    if bytes.len() < 2 + 2 * n {
        return String::new();
    }
    let mut units: Vec<u16> =
        bytes[2..2 + 2 * n].chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
    stream_xor(&mut units, seed | 0x0F58_D3F1);
    key_xor(&mut units, &key.encode_utf16().collect::<Vec<_>>());
    String::from_utf16_lossy(&units)
}

impl QlieVm {
    fn capability_path(&self, path: &str) -> Result<String> {
        let path = path.replace('\\', "/");
        let path = path.trim_start_matches("./");
        ensure!(
            !path.is_empty() && !path.starts_with('/') && !path.contains(':'),
            "QLIE file query outside the client capability: {path}"
        );
        Ok(crate::program::canonical(path)?)
    }
    /// Returns false when `op` is not handled here; Ok(true) with a pending
    /// task re-executes the instruction.
    pub(super) fn misc_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<Option<bool>> {
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        match op {
            "efn" => {
                ensure!(a.len() == 1, "QLIE file name ABI");
                m.result_text(0, extract_file_name(&s(0)));
            }
            "exp" => {
                ensure!(a.len() == 1, "QLIE file path ABI");
                m.result_text(0, extract_file_path(&s(0)));
            }
            "cfe" => {
                ensure!(a.len() == 2, "QLIE change extension ABI");
                m.result_text(0, change_file_ext(&s(0), &s(1)));
            }
            // crc,text: 4E6384 over the UTF-16 bytes, IntToHex(v, 4).
            "crc" => {
                ensure!(a.len() == 1, "QLIE text CRC ABI");
                let crc = crysirror_resource::checksum::crc16_ccitt_serial(&utf16(&s(0)));
                m.result_text(0, format!("{crc:04X}"));
            }
            // tsha,text (6EEF70) hashes as many bytes of the UTF-16 buffer as
            // the string has characters, i.e. the first half of its code units.
            "tsha" => {
                ensure!(a.len() == 1, "QLIE text SHA ABI");
                let text = s(0);
                let bytes = utf16(&text);
                let chars = text.encode_utf16().count();
                let digest = crysirror_resource::checksum::sha256(&bytes[..chars]);
                m.result_text(0, sha_hex(&digest));
            }
            // id (6C809C): a fresh GUID mixed by 6C8074, four 8-digit words.
            "id" => {
                ensure!(a.is_empty(), "QLIE unique id ABI");
                let mut seed = h.now_ms() ^ (self.id_counter.wrapping_mul(0x9E3779B97F4A7C15));
                self.id_counter = self.id_counter.wrapping_add(1);
                let mut words = [0u32; 4];
                for w in &mut words {
                    // SplitMix64 over host time and a per-client counter.
                    seed = seed.wrapping_add(0x9E3779B97F4A7C15);
                    let mut z = seed;
                    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
                    *w = (z ^ (z >> 31)) as u32;
                }
                let text: String = words.iter().map(|w| format!("{:08X}", guid_mix(*w))).collect();
                m.result_text(0, text);
            }
            "lng" | "?lng" => {
                let locale = h.host_information()?.locale;
                let (primary, sub) = lang_id(locale.as_deref());
                for (i, v) in [primary, sub, primary, sub].into_iter().enumerate() {
                    m.result(i, Value::Int(v));
                }
            }
            // fkey answers only when an external key object was attached
            // (engine+324, set by the online activation path); never here.
            "fkey" => {}
            // exec,file,params: ShellExecute "open". Only the client's own
            // assets may be offered to the platform viewer; programs, URLs and
            // parameters are not executed.
            "exec" => {
                ensure!((1..=2).contains(&a.len()), "QLIE shell open ABI");
                ensure!(s(1).is_empty(), "QLIE exec parameters are not supported: {}", s(1));
                let path = self.capability_path(&s(0))?;
                h.open_document(self.client, &VfsPath::new(Mount::Asset, &path)?)?;
            }
            // code,text,mode,seed,key: mode 0 encodes, otherwise decodes; a
            // zero seed is 0xC4 (6EF018).
            "code" => {
                ensure!(a.len() == 4, "QLIE string code ABI");
                let mode = a[1].integer()?;
                let seed = match a[2].integer()? {
                    0 => 0xC4,
                    v => v as u32,
                };
                let key = s(3);
                let out = if mode == 0 {
                    code_encode(&s(0), &key, seed)
                } else {
                    code_decode(&s(0), &key, seed)
                };
                m.result_text(0, out);
            }
            "ptof" => return self.resource_to_file(h, m, a).map(Some),
            "now" | "?now" => return self.now_method(h, m, op).map(Some),
            "gfe" | "fid" | "fsha" | "bsno" => return self.file_query(h, m, op, a).map(Some),
            _ => return Ok(None),
        }
        Ok(Some(true))
    }
    fn now_method(&mut self, h: &mut dyn HostServices, m: &mut Machine, op: &str) -> Result<bool> {
        let owner = (m.frame.path.clone(), m.frame.pc, op.to_owned());
        if self.misc_io.is_none() {
            self.misc_io = Some(MiscPending {
                owner: owner.clone(),
                task: None,
                clock: Some(h.begin_civil_time(self.client)?),
            });
        }
        let pending = self.misc_io.as_mut().unwrap();
        ensure!(pending.owner == owner, "QLIE clock query owner");
        let clock = pending.clock.as_mut().context("QLIE clock query state")?;
        let time = match clock.poll()? {
            CivilTimeState::Pending => return Ok(false),
            CivilTimeState::Failed(e) => bail!("QLIE local time: {e}"),
            CivilTimeState::Ready(t) => t,
        };
        self.misc_io = None;
        let values = [
            time.year,
            i32::from(time.month),
            i32::from(time.day),
            i32::from(time.hour),
            i32::from(time.minute),
            i32::from(time.second),
            i32::from(time.millisecond),
        ];
        for (i, v) in values.into_iter().enumerate() {
            m.result(i, Value::Int(v));
        }
        Ok(true)
    }
    fn file_query(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let owner = (m.frame.path.clone(), m.frame.pc, op.to_owned());
        if self.misc_io.is_none() {
            let name = a.first().map(Value::text).unwrap_or_default();
            // ParamStr(0): the original executable in the game folder.
            let executable = || -> Result<String> {
                self.config
                    .executable
                    .clone()
                    .context("QLIE original executable is not present in the game folder")
            };
            if op == "bsno" || (op == "fsha" && name.eq_ignore_ascii_case("self")) {
                let exe = executable()?;
                let path = VfsPath::new(Mount::Asset, &crate::program::canonical(&exe)?)?;
                let storage = h.storage(self.client)?;
                let build = op == "bsno";
                let task = self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let bytes = storage.read(&path)?;
                    let digest = crysirror_resource::checksum::sha256(&bytes);
                    if !build {
                        return Ok(MiscIo::Digest(Some(digest)));
                    }
                    let keys = build_keys(&bytes)?;
                    Ok(MiscIo::Build { keys, name: extract_file_name(&exe).to_owned(), digest })
                })?;
                self.misc_io = Some(MiscPending { owner: owner.clone(), task: Some(task), clock: None });
            }
        }
        if self.misc_io.is_none() {
            let name = a.first().map(Value::text).unwrap_or_default();
            // fid,path,version: a version stamp needs the Windows version
            // resource of the file (4E68A4), which this host does not read.
            ensure!(
                op != "fid" || a.get(1).map(Value::text).unwrap_or_default().is_empty(),
                "QLIE file version comparison is not supported"
            );
            let path = self.capability_path(&name)?;
            let storage = h.storage(self.client)?;
            let digest = op == "fsha";
            let task = self.pool.spawn(move |cancel| {
                cancel.check()?;
                for mount in [Mount::Save, Mount::Asset] {
                    let Some(info) = storage.metadata(mount, &path)? else {
                        continue;
                    };
                    if info.is_directory {
                        continue;
                    }
                    if !digest {
                        return Ok(MiscIo::Exists(true));
                    }
                    let bytes = storage.read(&VfsPath::new(mount, &path)?)?;
                    return Ok(MiscIo::Digest(Some(crysirror_resource::checksum::sha256(&bytes))));
                }
                Ok(if digest { MiscIo::Digest(None) } else { MiscIo::Exists(false) })
            })?;
            self.misc_io = Some(MiscPending { owner: owner.clone(), task: Some(task), clock: None });
        }
        let pending = self.misc_io.as_mut().unwrap();
        ensure!(pending.owner == owner, "QLIE file query owner");
        let task = pending.task.as_mut().context("QLIE file query state")?;
        let Some(result) = task.try_take()? else {
            return Ok(false);
        };
        self.misc_io = None;
        match result {
            MiscIo::Done => {}
            MiscIo::Exists(found) => m.result(0, Value::Int(i32::from(found))),
            // A missing file yields an empty string (6EEEB8).
            MiscIo::Digest(digest) => m.result_text(0, digest.map(|d| sha_hex(&d)).unwrap_or_default()),
            // Texts: the four keys, the executable name and its SHA-256;
            // integer: the engine revision.
            MiscIo::Build { keys, name, digest } => {
                for (i, key) in keys.into_iter().enumerate() {
                    m.result_text(i, key);
                }
                m.result(0, Value::Int(564));
                m.result_text(4, name);
                m.result_text(5, sha_hex(&digest));
            }
        }
        Ok(true)
    }
    /// ptof,resource,file (6CC2FC + SaveToFile): copies a script-visible
    /// resource into the save capability; a missing resource is reported
    /// like the native script_filenotfound error.
    fn resource_to_file(&mut self, h: &mut dyn HostServices, m: &mut Machine, a: &[Value]) -> Result<bool> {
        ensure!(a.len() == 2, "QLIE resource export ABI");
        let owner = (m.frame.path.clone(), m.frame.pc, "ptof".to_owned());
        if self.misc_io.is_none() {
            let candidates: Vec<String> = self
                .resource_candidates(m, &a[0].text())?
                .into_iter()
                .map(|logical| self.config.assets.get(&logical).cloned().unwrap_or(logical))
                .collect();
            let target = self.capability_path(&a[1].text())?;
            let storage = h.storage(self.client)?;
            let task = self.pool.spawn(move |cancel| {
                for candidate in &candidates {
                    cancel.check()?;
                    if storage.metadata(Mount::Asset, candidate)?.is_some_and(|i| !i.is_directory) {
                        let bytes = storage.read(&VfsPath::new(Mount::Asset, candidate)?)?;
                        storage.write_atomic(&VfsPath::new(Mount::Save, &target)?, &bytes)?;
                        return Ok(MiscIo::Done);
                    }
                }
                anyhow::bail!("QLIE script_filenotfound: {}", candidates.first().cloned().unwrap_or_default())
            })?;
            self.misc_io = Some(MiscPending { owner: owner.clone(), task: Some(task), clock: None });
        }
        let pending = self.misc_io.as_mut().unwrap();
        ensure!(pending.owner == owner, "QLIE resource export owner");
        let Some(MiscIo::Done) = pending.task.as_mut().context("QLIE export state")?.try_take()? else {
            return Ok(false);
        };
        self.misc_io = None;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_keys_follow_the_revision_literal() {
        let literal = |t: &str| {
            let mut v = super::LITERAL.to_vec();
            v.extend((t.len() as u32).to_le_bytes());
            v.extend(t.encode_utf16().flat_map(u16::to_le_bytes));
            v.extend([0, 0]);
            while v.len() % 4 != 0 {
                v.push(0);
            }
            v
        };
        let mut exe = vec![1, 2, 3];
        for t in ["ABCDEFGHIJKLMNOPQ", "NOPQRSTUVWXYZABCD", "ZZZZZZZZZZZZZZZZZ", "AAAAAAAAAAAAAAAAA", "564"] {
            exe.extend(literal(t));
        }
        let keys = super::build_keys(&exe).unwrap();
        assert_eq!(keys[0], "NOPQRSTUVWXYZABCD");
        assert_eq!(keys[1], "ABCDEFGHIJKLMNOPQ");
        assert_eq!(keys[2], "MMMMMMMMMMMMMMMMM");
    }
    use super::*;
    #[test]
    fn code_round_trip_and_paths() {
        let e = code_encode("abcあ", "key", 0xC4);
        assert_eq!(e.len(), 4 + 4 * 4);
        assert_eq!(code_decode(&e, "key", 0xC4), "abcあ");
        assert_eq!(extract_file_name("a\\b\\c.txt"), "c.txt");
        assert_eq!(extract_file_path("a\\b\\c.txt"), "a\\b\\");
        assert_eq!(change_file_ext("a.b\\c.txt", ".dat"), "a.b\\c.dat");
        assert_eq!(change_file_ext("a.b\\c", ".dat"), "a.b\\c.dat");
    }
}
