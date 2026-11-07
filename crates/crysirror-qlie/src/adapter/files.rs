//! Native path manager (6E140C/6CC2FC) and TStrings file object (6E1FB0).
//! Directories, filename and lines are native VM policy; byte access, decoding
//! and line framing use scoped engine storage and shared resource codecs.
use super::*;

const LIMIT: usize = 16 * 1024 * 1024;

// Delphi StrToIntDef ->405044: leading U+0020 only, no trailing whitespace,
// signed decimal or $, x/X, 0x/0X hexadecimal; hex retains all 32 bits.
fn integer_or_zero(text: &str) -> i32 {
    let text = text
        .split('\0')
        .next()
        .unwrap_or("")
        .trim_start_matches(' ');
    let (negative, text) = if let Some(text) = text.strip_prefix('-') {
        (true, text)
    } else {
        (false, text.strip_prefix('+').unwrap_or(text))
    };
    let hex = text
        .strip_prefix('$')
        .or_else(|| text.strip_prefix('x'))
        .or_else(|| text.strip_prefix('X'))
        .or_else(|| text.strip_prefix("0x"))
        .or_else(|| text.strip_prefix("0X"));
    if let Some(digits) = hex {
        if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_hexdigit()) {
            return 0;
        }
        return u32::from_str_radix(digits, 16)
            .map(|v| {
                if negative {
                    (v as i32).wrapping_neg()
                } else {
                    v as i32
                }
            })
            .unwrap_or(0);
    }
    if text.is_empty() || !text.bytes().all(|c| c.is_ascii_digit()) {
        return 0;
    }
    let Ok(value) = text.parse::<u32>() else {
        return 0;
    };
    if negative && value <= 0x8000_0000 {
        (value as i32).wrapping_neg()
    } else if !negative && value <= i32::MAX as u32 {
        value as i32
    } else {
        0
    }
}
pub(super) struct Pending {
    owner: (String, String, usize, ThreadKind, String),
    task: Task<(String, Vec<String>)>,
}

impl QlieVm {
    /// Source reader's current directory, then root, then insertion-ordered
    /// search directories. 6CC2FC uses first success, not reverse insertion.
    pub(super) fn resource_candidates(&self, m: &Machine, name: &str) -> Result<Vec<String>> {
        ensure!(
            !name.is_empty() && name.len() <= 65536,
            "QLIE resource name limit"
        );
        // Validate the request separately: joining it to an existing directory
        // must never turn an absolute/drive path into a capability-relative one.
        ensure!(
            !name.starts_with(['/', '\\']) && !name.contains(':') && !name.contains('\0'),
            "QLIE resource path outside asset capability"
        );
        // 6CC2FC order: self path + name, the bare name, then \path prefixes.
        let source = m.source_code_path();
        let directory = source.rsplit_once('/').map(|v| v.0).unwrap_or("");
        let mut candidates = vec![format!("{directory}/{name}"), name.to_owned()];
        if directory.is_empty() {
            candidates.remove(0);
        }
        if let Some(path) = self
            .objects
            .get(&Self::builtin("path", m.source_thread_kind()))
        {
            candidates.extend(path.lines.iter().map(|prefix| format!("{prefix}{name}")));
        }
        let mut seen = BTreeSet::new();
        Ok(candidates
            .into_iter()
            .filter_map(|candidate| crate::program::canonical(&candidate).ok())
            .filter(|candidate| seen.insert(candidate.clone()))
            .collect())
    }

    pub(super) fn file_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        kind: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let argument = || a.first().context("QLIE file/path method argument");
        if kind == "path" {
            let o = self
                .objects
                .get_mut(id)
                .context("QLIE path manager absent")?;
            match op {
                "a" => {
                    let mut directory = argument()?.text();
                    if !directory.is_empty() {
                        ensure!(
                            o.lines.len() < 4096 && directory.len() <= 65536,
                            "QLIE search directory limit"
                        );
                        crate::program::canonical(&directory)?;
                        if !directory.ends_with('\\') {
                            directory.push('\\');
                        }
                        o.lines.push(directory);
                    }
                }
                "d" => {
                    // Native delete does not append a separator, unlike add.
                    let directory = argument()?.text();
                    if let Some(index) = o
                        .lines
                        .iter()
                        .position(|v| v.eq_ignore_ascii_case(&directory))
                    {
                        o.lines.remove(index);
                    }
                }
                "i" => {
                    o.lines.clear();
                    o.locked = false;
                }
                "lk" => o.locked = argument()?.integer()? == 1,
                "?lk" => m.result(0, Value::Int(i32::from(o.locked))),
                _ => bail!("unsupported QLIE path method {op}"),
            }
            return Ok(true);
        }
        ensure!(kind == "file", "QLIE file method receiver");
        match op {
            "l" => {
                let requested = argument()?.text();
                let owner = (
                    id.to_owned(),
                    m.frame.path.clone(),
                    m.frame.pc,
                    m.thread_kind(),
                    requested.clone(),
                );
                if self.file_operation.is_none() {
                    let candidates = self
                        .resource_candidates(m, &requested)?
                        .into_iter()
                        .map(|logical| {
                            let actual =
                                self.config.assets.get(&logical).unwrap_or(&logical).clone();
                            (logical, actual)
                        })
                        .collect::<Vec<_>>();
                    let storage = h.storage(self.client)?;
                    self.file_operation = Some(Pending {
                        owner: owner.clone(),
                        task: self.pool.spawn(move |cancel| {
                            for (logical, actual) in candidates {
                                cancel.check()?;
                                let stream = match storage
                                    .open_asset(&VfsPath::new(Mount::Asset, &actual)?)
                                {
                                    Ok(stream) => stream,
                                    Err(Error::NotFound) => continue,
                                    Err(error) => return Err(error.into()),
                                };
                                let bytes =
                                    crysirror_resource::read_all(stream.as_ref(), LIMIT as u64)?;
                                cancel.check()?;
                                let text =
                                    crysirror_resource::text_stream::decode_bom_or_shift_jis(
                                        &bytes, LIMIT,
                                    )?;
                                let mut cursor =
                                    crysirror_resource::text_stream::LineCursor::default();
                                cursor.reset(text, LIMIT * 3)?;
                                let mut lines = Vec::new();
                                while let Some(line) = cursor.next_line() {
                                    ensure!(lines.len() < 262144, "QLIE text file line budget");
                                    lines.push(line.to_owned());
                                }
                                return Ok((logical, lines));
                            }
                            bail!("QLIE file resource absent: {requested}")
                        })?,
                    });
                }
                let pending = self.file_operation.as_mut().unwrap();
                ensure!(pending.owner == owner, "QLIE text file completion owner");
                let Some((filename, lines)) = pending.task.try_take()? else {
                    return Ok(false);
                };
                self.file_operation = None;
                let object = self
                    .objects
                    .get_mut(id)
                    .context("QLIE file object absent")?;
                object.lines = lines;
                object.asset = Some(filename);
            }
            "i" => {
                let object = self
                    .objects
                    .get_mut(id)
                    .context("QLIE file object absent")?;
                object.lines.clear();
                object.asset = None;
            }
            "?l" => m.result_text(
                0,
                self.objects
                    .get(id)
                    .and_then(|o| o.asset.clone())
                    .unwrap_or_default(),
            ),
            "c" => m.result(
                0,
                Value::Int(i32::try_from(
                    self.objects
                        .get(id)
                        .context("QLIE file object absent")?
                        .lines
                        .len(),
                )?),
            ),
            "g" | "gl" => {
                let object = self.objects.get(id).context("QLIE file object absent")?;
                let text = if op == "g" {
                    let name = argument()?.text();
                    // TStrings.IndexOfName returns the first '=' name match;
                    // names are ASCII case-insensitive and are not trimmed.
                    object
                        .lines
                        .iter()
                        .filter_map(|line| line.split_once('='))
                        .find(|(key, _)| key.eq_ignore_ascii_case(&name))
                        .map(|(_, value)| value.to_owned())
                        .unwrap_or_default()
                } else {
                    let index = usize::try_from(argument()?.integer()?)
                        .context("QLIE negative file line index")?;
                    object.lines.get(index).cloned().unwrap_or_default()
                };
                m.result(0, Value::Int(integer_or_zero(&text)));
                m.result_text(0, text);
            }
            _ => bail!("unsupported QLIE file method {op}"),
        }
        Ok(true)
    }
}

/// 6FE1D0 lff/stf continuation (one outstanding per adapter).
pub(super) struct ListIo {
    owner: (String, String, usize, String),
    task: Task<Option<Vec<String>>>,
}
/// 64-byte plaintext prefix of a mode-1 list stream (72B394).
const LIST_HEADER: &[u8; 64] =
    b"parity:a8Kf26QikNBaYUpka89#$&pL><?89aBD!#AkNALPz129QLM\\\\IahQ6@+=";
/// Native list paths are composed from misc\gsf,$1000 ("./" here) and name
/// user files; they are confined to the save capability.
fn list_path(path: &str) -> Result<String> {
    let path = path.replace('\\', "/");
    let path = path.trim_start_matches("./");
    crate::program::canonical(path)
}
/// 4E5830: key 0 yields the plain stream; mode 1 replaces it with the
/// decoded stream when its header matches. Otherwise nothing is loaded.
fn decode_list(bytes: Vec<u8>, mode: i32, key: i32) -> Option<Vec<u8>> {
    let mut stream = (key == 0).then(|| bytes.clone());
    if mode == 1 {
        let mut decoded = bytes;
        crysirror_resource::lane_feedback::decrypt(&mut decoded, key as u32);
        if decoded.len() >= 64 && decoded[..64] == LIST_HEADER[..] {
            stream = Some(decoded[64..].to_vec());
        }
    }
    stream
}
/// 4E5764: mode 0 copies, mode 1 prefixes the header and encodes the whole
/// stream; any other mode produces no stream and nothing is written.
fn encode_list(text: &[u8], mode: i32, key: i32) -> Option<Vec<u8>> {
    match mode {
        0 => Some(text.to_vec()),
        1 => {
            let mut out = LIST_HEADER.to_vec();
            out.extend_from_slice(text);
            crysirror_resource::lane_feedback::encrypt(&mut out, key as u32);
            Some(out)
        }
        _ => None,
    }
}
impl QlieVm {
    /// lff,path[,mode[,key]] (LoadFromFile -> ResultInt[0]=1) and
    /// stf,path[,mode[,key]] (ForceDirectories + SaveToFile). TWideStrings
    /// streams are UTF-16LE with a byte-order mark; unmarked text loads as
    /// Shift_JIS.
    pub(super) fn list_file_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        ensure!((1..=3).contains(&a.len()), "QLIE list file ABI");
        let requested = a[0].text();
        let mode = a.get(1).map(Value::integer).transpose()?.unwrap_or(0);
        let key = a.get(2).map(Value::integer).transpose()?.unwrap_or(0);
        let owner = (id.to_owned(), m.frame.path.clone(), m.frame.pc, op.to_owned());
        if self.list_io.is_none() {
            let path = list_path(&requested)?;
            let storage = h.storage(self.client)?;
            let task = if op == "lff" {
                self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let vfs = VfsPath::new(Mount::Save, &path)?;
                    let stream = match storage.open_read(&vfs) {
                        Ok(stream) => stream,
                        Err(Error::NotFound) => return Ok(None),
                        Err(error) => return Err(error.into()),
                    };
                    let bytes = crysirror_resource::read_all(stream.as_ref(), LIMIT as u64)?;
                    cancel.check()?;
                    let Some(bytes) = decode_list(bytes, mode, key) else {
                        return Ok(None);
                    };
                    let text = crysirror_resource::text_stream::decode_bom_or_shift_jis(&bytes, LIMIT)?;
                    Ok(Some(vec![text]))
                })?
            } else {
                let object = self.objects.get(id).context("QLIE list object absent")?;
                let mut text = vec![0xff, 0xfe];
                for unit in super::objects::strings_text(object).encode_utf16() {
                    text.extend_from_slice(&unit.to_le_bytes());
                }
                self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    if let Some(bytes) = encode_list(&text, mode, key) {
                        storage.write_atomic(&VfsPath::new(Mount::Save, &path)?, &bytes)?;
                    }
                    Ok(None)
                })?
            };
            self.list_io = Some(ListIo { owner: owner.clone(), task });
        }
        let pending = self.list_io.as_mut().unwrap();
        ensure!(pending.owner == owner, "QLIE list file completion owner");
        let Some(loaded) = pending.task.try_take()? else {
            return Ok(false);
        };
        self.list_io = None;
        if op == "lff" {
            m.result(0, Value::Int(0));
            if let Some(text) = loaded.and_then(|mut v| v.pop()) {
                let object = self.objects.get_mut(id).context("QLIE list object absent")?;
                super::objects::strings_install(object, &text)?;
                m.result(0, Value::Int(1));
            }
        }
        Ok(true)
    }
}
