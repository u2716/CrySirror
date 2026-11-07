// Copyright (c) 2026 CrySirror contributors. MIT.
//! Native console policy, not a second text renderer.
//!
//! 416120 consumes its arguments/result through40AE80, then4156E0("w",1)
//! records a control, clears20 voice-line records, resets cursor/speed flags
//! and dispatches CONSOLE_CLS (4094E0/0x93b38a0b). It DOES NOT clear the page
//! or discard Ptr's queued text. The original MJO hooks own those operations.
//! 416160/4161F0 similarly dispatch CONSOLE_ON/OFF only on a state transition.
//! The globals here lie in PE .data's virtual zero tail, not loaded FF bytes
//! reported by an IDA query for uninitialised addresses.
//!
//! Backlog records retain the native40000-byte shift window and trailing u16
//! size. Native4156E0 uses overlapping memcpy; copy_within defines that case
//! without reproducing C undefined behaviour. This is retained logical state,
//! not an original-save encoder; replay, voice/control rendering and complete
//! PROC shaping/raster scheduling require their own admitted native paths.
use super::*;

const HISTORY: usize = 40000;
#[derive(Clone)]
pub(super) struct ConsoleFont {
    pub face: Vec<u8>,
    pub alias: String,
    pub height: i32,
    pub half_width: i32,
    pub line_pitch: i32,
    pub weight: i32,
}
#[derive(Clone)]
pub(super) struct Console {
    pub page: Option<u32>,
    pub origin: [i32; 2],
    pub extent: [i32; 2],
    pub cursor: [i32; 2],
    saved_cursor: [i32; 2],
    pub colors: [i32; 2],
    pub font: Option<ConsoleFont>,
    pub visible: bool,
    pub no_wait: bool,
    pub speed: i32,
    pub suppress_history: i32,
    pub replay: bool,
    indent: i32,
    clear_pending: bool,
    break_pending: bool,
    just_wrapped: bool,
    history: Arc<Vec<u8>>,
    pub history_controls: u64,
    voice_lines: Arc<Vec<u8>>,
    voice_line: usize,
    pub selected_face: Vec<u8>,
    pub selected_proportional: i32,
}
impl Default for Console {
    fn default() -> Self {
        Self {
            page: None,
            origin: [0; 2],
            extent: [0; 2],
            cursor: [0; 2],
            saved_cursor: [0; 2],
            colors: [0; 2],
            font: None,
            visible: false,
            no_wait: false,
            speed: 0,
            suppress_history: 0,
            replay: false,
            indent: 0,
            clear_pending: false,
            break_pending: false,
            just_wrapped: false,
            history: Arc::new(vec![0; HISTORY]),
            history_controls: 0,
            voice_lines: Arc::new(vec![0; 20 * 532]),
            voice_line: 0,
            selected_face: vec![],
            selected_proportional: 0,
        }
    }
}
impl Console {
    pub(super) fn diagnostics(&self) -> serde_json::Value {
        json!({"page":self.page,"origin":self.origin,"extent":self.extent,"cursor":self.cursor,
            "visible":self.visible,"no_wait":self.no_wait,"speed":self.speed,
            "history_suppression":self.suppress_history,"history_controls":self.history_controls,
            "last_history_record_bytes":u16::from_le_bytes(self.history[HISTORY-2..].try_into().unwrap()),
            "font":self.font.as_ref().map(|font|json!({"alias":font.alias,"height":font.height,
                "half_width":font.half_width,"line_pitch":font.line_pitch,"weight":font.weight}))})
    }
    fn record(&mut self, bytes: &[u8]) -> Result<()> {
        if self.replay || self.suppress_history != 0 {
            return Ok(());
        }
        let size = bytes
            .len()
            .checked_add(4)
            .context("Majiro control history size")?;
        ensure!(
            size <= HISTORY && size <= u16::MAX as usize && !bytes.contains(&0),
            "Majiro control history record"
        );
        let total = self
            .history_controls
            .checked_add(1)
            .context("Majiro history counter overflow")?;
        let history = Arc::make_mut(&mut self.history);
        history.copy_within(size.., 0);
        let start = HISTORY - size;
        history[start] = b'\\';
        history[start + 1..start + 1 + bytes.len()].copy_from_slice(bytes);
        history[start + 1 + bytes.len()] = 0;
        history[HISTORY - 2..].copy_from_slice(&(size as u16).to_le_bytes());
        self.history_controls = total;
        Ok(())
    }
    /// 417070 history text record: bytes, NUL, u16 record size (len+3).
    pub(super) fn record_text(&mut self, bytes: &[u8]) -> Result<()> {
        if self.replay || bytes.is_empty() {
            return Ok(());
        }
        let size = bytes.len() + 3;
        ensure!(size <= HISTORY && size <= u16::MAX as usize, "Majiro history text record");
        let history = Arc::make_mut(&mut self.history);
        history.copy_within(size.., 0);
        let start = HISTORY - size;
        history[start..start + bytes.len()].copy_from_slice(bytes);
        history[start + bytes.len()] = 0;
        history[HISTORY - 2..].copy_from_slice(&(size as u16).to_le_bytes());
        Ok(())
    }
    pub(super) fn history(&self) -> &[u8] {
        &self.history
    }
    fn clear(&mut self, token: &[u8]) -> Result<bool> {
        self.just_wrapped = false;
        // 4156E0 executes 'w' during replay too; only recording is skipped.
        self.record(token)?;
        Arc::make_mut(&mut self.voice_lines).fill(0);
        self.voice_line = 0;
        self.no_wait = false;
        self.speed = -1;
        self.indent = 0;
        self.cursor = self.origin;
        self.clear_pending = true;
        self.break_pending = false;
        Ok(true)
    }
}
fn hex_fields(bytes: &[u8], count: usize) -> Result<Vec<i32>> {
    ensure!(
        bytes.len() >= 1 + count * 10,
        "Majiro short formatted control"
    );
    bytes[1..1 + count * 10]
        .chunks_exact(10)
        .map(|word| {
            ensure!(
                &word[..2] == b"0x" && word[2..].iter().all(u8::is_ascii_hexdigit),
                "Majiro malformed control integer"
            );
            Ok(u32::from_str_radix(std::str::from_utf8(&word[2..])?, 16)? as i32)
        })
        .collect()
}
fn font_name(value: &Value) -> Result<Vec<u8>> {
    let bytes = value.bytes()?;
    let bytes = &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())];
    //Native face buffers and LOGFONTA use LF_FACESIZE32. Oversized strcpy
    //input corrupts adjacent globals in the original; reject it explicitly.
    ensure!(
        bytes.len() <= 31,
        "Majiro console font exceeds native face buffer"
    );
    ensure!(
        encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(bytes)
            .is_some(),
        "Majiro console font CP932"
    );
    Ok(bytes.to_vec())
}
impl MajiroVm {
    /// Returns true only after the native call and its callback plan commit.
    pub(super) fn try_console(
        &mut self,
        machine: &mut Machine,
        event: &Event,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        let (hash, args, returns, control) = match event {
            Event::Native {
                hash,
                arguments,
                returns,
                ..
            } => (*hash, arguments.as_slice(), *returns, None),
            Event::Control { bytes, .. } => (0, &[][..], false, Some(bytes.as_slice())),
            _ => return Ok(false),
        };
        if control.is_none()
            && !matches!(
                hash,
                0xb5a1e3c9
                    | 0x403960f8
                    | 0xf1f8a206
                    | 0x5131e234
                    | 0xcc3e0342
                    | 0x0e42730d
                    | 0x934d927b
                    | 0x30fa2a29
                    | 0x5c9ed743
                    | 0xd4066e31
                    | 0x4133d7a9
                    | 0xa2743878
                    | 0x979d1f16
                    | 0x06b3c8ac
                    | 0x5fdcccee
                    | 0x924ee3eb
                    | 0xc5270429
                    | 0x1ec3c6bb
                    | 0x5b87a41d
                    | 0xba81ef8d
            )
        {
            return Ok(false);
        }
        ensure!(
            self.native_abi,
            "Majiro console native ABI has not been reviewed"
        );
        let output = |value| if returns { vec![value] } else { vec![] };
        let integer = |n| output(Value::Integer(n));
        let mut candidate = self.console.clone();
        let mut callback = None;
        let mut token = None;
        let mut result = Value::Integer(0);
        if let Some(bytes) = control {
            token = Some(bytes.to_vec());
        } else {
            match hash {
                0x5b87a41d => {
                    ensure!(args.is_empty(), "Majiro console_is_on ABI");
                    machine
                        .complete_with_callback(integer(i32::from(self.console.visible)), None)?;
                    return Ok(true);
                }
                0xb5a1e3c9 => {
                    ensure!(args.is_empty(), "Majiro console_cls ABI");
                    token = Some(b"w".to_vec());
                }
                0x403960f8 => {
                    ensure!(args.len() == 5, "Majiro console_set ABI");
                    let page = u32::try_from(args[0].integer()?)
                        .context("Majiro console destination page")?;
                    ensure!(
                        self.gfx.pages.contains_key(&page),
                        "Majiro console page missing or foreign"
                    );
                    candidate.page = Some(page);
                    candidate.origin = [args[1].integer()?, args[2].integer()?];
                    candidate.cursor = candidate.origin;
                    candidate.extent = [args[3].integer()?, args[4].integer()?];
                    ensure!(
                        candidate
                            .origin
                            .iter()
                            .chain(candidate.extent.iter())
                            .all(|v| v.unsigned_abs() <= 1_000_000),
                        "Majiro console geometry profile"
                    );
                }
                0xf1f8a206 => {
                    ensure!(args.len() == 2, "Majiro console cursor ABI");
                    candidate.cursor = [args[0].integer()?, args[1].integer()?];
                }
                0x5131e234 | 0xcc3e0342 | 0x0e42730d | 0x934d927b => {
                    ensure!(args.is_empty(), "Majiro console cursor query ABI");
                    let axis = usize::from(matches!(hash, 0xcc3e0342 | 0x934d927b));
                    let relative = matches!(hash, 0x5131e234 | 0xcc3e0342);
                    let value = self.console.cursor[axis].wrapping_sub(if relative {
                        self.console.origin[axis]
                    } else {
                        0
                    });
                    machine.complete_with_callback(integer(value), None)?;
                    return Ok(true);
                }
                0x30fa2a29 | 0x5c9ed743 => {
                    ensure!(args.is_empty(), "Majiro console speed mode ABI");
                    candidate.no_wait = hash == 0x30fa2a29;
                }
                0xd4066e31 | 0x4133d7a9 => {
                    ensure!(args.is_empty(), "Majiro history suppression ABI");
                    candidate.suppress_history = if hash == 0xd4066e31 {
                        candidate.suppress_history.wrapping_add(1)
                    } else {
                        candidate.suppress_history.wrapping_sub(1)
                    };
                }
                0xa2743878 | 0x979d1f16 => {
                    ensure!(args.len() <= 2, "Majiro console visibility ABI");
                    let values = (0..2)
                        .map(|i| {
                            args.get(i)
                                .map(Value::integer)
                                .unwrap_or(Ok(0))
                                .map(Value::Integer)
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let show = hash == 0xa2743878;
                    if candidate.visible != show && !candidate.replay {
                        callback = Some((if show { 0xf7a4c8d8 } else { 0x0d9b7f0e }, values));
                    }
                    candidate.visible = show;
                }
                0x06b3c8ac => {
                    ensure!((1..=5).contains(&args.len()), "Majiro console font ABI");
                    let name = font_name(&args[0])?;
                    let height = args.get(1).map(Value::integer).unwrap_or(Ok(20))?;
                    let width = args.get(2).map(Value::integer).unwrap_or(Ok(height / 2))?;
                    let pitch = args
                        .get(3)
                        .map(Value::integer)
                        .unwrap_or(Ok(height.wrapping_mul(5) / 4))?;
                    let weight = args.get(4).map(Value::integer).unwrap_or(Ok(0))?;
                    let mut bytes = format!(
                        "f0x{:08x}0x{:08x}0x{:08x}0x{:08x}",
                        height as u32, width as u32, pitch as u32, weight as u32
                    )
                    .into_bytes();
                    bytes.extend(name);
                    token = Some(bytes);
                }
                0x5fdcccee => {
                    ensure!(matches!(args.len(), 1 | 2), "Majiro console colors ABI");
                    let first = args[0].integer()?;
                    let second = args.get(1).map(Value::integer).unwrap_or(Ok(-1))?;
                    token = Some(
                        format!("c0x{:08x}0x{:08x}", first as u32, second as u32).into_bytes(),
                    );
                }
                0xba81ef8d => {
                    //4160C0 formats an 'l' locate control through 4156E0.
                    ensure!(args.len() == 2, "Majiro console locate ABI");
                    token = Some(
                        format!("l0x{:08x}0x{:08x}", args[0].integer()? as u32, args[1].integer()? as u32).into_bytes(),
                    );
                }
                0x924ee3eb => {
                    ensure!(args.len() == 3, "Majiro RGB packing ABI");
                    let values = args
                        .iter()
                        .map(Value::integer)
                        .collect::<Result<Vec<_>>>()?;
                    let value = (values[0] as u8 as u32)
                        | ((values[1] as u8 as u32) << 8)
                        | ((values[2] as u8 as u32) << 16);
                    machine.complete_with_callback(integer(value as i32), None)?;
                    return Ok(true);
                }
                0x1ec3c6bb => {
                    ensure!(args.len() == 2, "Majiro selected font name ABI");
                    candidate.selected_face = font_name(&args[0])?;
                    candidate.selected_proportional = args[1].integer()?;
                }
                0xc5270429 => {
                    ensure!(args.is_empty(), "Majiro get_font_name ABI");
                    result = Value::native_string(&candidate.selected_face)?;
                }
                _ => return Ok(false),
            }
        }
        if let Some(bytes) = token {
            callback = self.console_token(&mut candidate, &bytes, host)?;
        }
        machine.complete_with_callback(output(result), callback)?;
        self.console = candidate;
        Ok(true)
    }
}

impl MajiroVm {
    /// 4156E0: apply one console control. Replay (49B074) executes controls
    /// and their hooks but records nothing. Returns the hook to dispatch.
    pub(super) fn console_token(
        &mut self,
        candidate: &mut Console,
        bytes: &[u8],
        host: &mut dyn HostServices,
    ) -> Result<Option<(u32, Vec<Value>)>> {
        let bytes = bytes.to_vec();
        let mut callback = None;
            ensure!(
                !bytes.is_empty() && bytes.len() <= 127 && !bytes.contains(&0),
                "Majiro console control buffer"
            );
            let tag = bytes[0];
            if tag == b'w' {
                self.page_text.clear();
                self.page_text_open = true;
                if candidate.clear(&bytes)? {
                    callback = Some((0x93b38a0b, vec![]));
                }
            } else {
                let just_wrapped = candidate.just_wrapped;
                candidate.just_wrapped = false;
                if tag == b'n' && just_wrapped && candidate.cursor == candidate.origin {
                    return Ok(None);
                }
                //Validate the entire native operation before committing history.
                match tag {
                    b'c' => {
                        let v = hex_fields(&bytes, 2)?;
                        ensure!(bytes.len() == 21, "Majiro color control extent");
                        candidate.colors = [v[0], v[1]];
                    }
                    b'f' => {
                        let v = hex_fields(&bytes, 4)?;
                        let face = font_name(&Value::String(bytes[41..].to_vec()))?;
                        let face = if face.is_empty() {
                            candidate
                                .font
                                .as_ref()
                                .context("Majiro console font has no previous face")?
                                .face
                                .clone()
                        } else {
                            face
                        };
                        let name = encoding_rs::SHIFT_JIS
                            .decode_without_bom_handling_and_without_replacement(&face)
                            .context("Majiro font CP932")?;
                        let alias = self.resolve_font_alias(&name, host)?;
                        let half_width = if v[1] == -1 { v[0] / 2 } else { v[1] };
                        let line_pitch = if v[2] == -1 {
                            v[0].wrapping_mul(5) / 4
                        } else {
                            v[2]
                        };
                        ensure!(
                            (1..=512).contains(&v[0])
                                && (1..=512).contains(&half_width)
                                && line_pitch.unsigned_abs() <= 1_000_000,
                            "Majiro console font geometry profile"
                        );
                        candidate.font = Some(ConsoleFont {
                            face,
                            alias,
                            height: v[0],
                            half_width,
                            line_pitch,
                            weight: if v[3] == -1 { 0 } else { v[3] },
                        });
                    }
                    b'l' | b'o' => {
                        let v = hex_fields(&bytes, 2)?;
                        ensure!(bytes.len() == 21, "Majiro cursor control extent");
                        if tag == b'l' {
                            candidate.cursor = [v[0], v[1]];
                        } else {
                            candidate.saved_cursor = candidate.cursor;
                            candidate.cursor = [
                                candidate.cursor[0].wrapping_add(v[0]),
                                candidate.cursor[1].wrapping_add(v[1]),
                            ];
                        }
                    }
                    b'r' => candidate.cursor = candidate.saved_cursor,
                    b's' => {
                        let v = hex_fields(&bytes, 1)?;
                        ensure!(bytes.len() == 11, "Majiro speed control extent");
                        candidate.speed = v[0];
                    }
                    b'z' => candidate.no_wait = true,
                    b'N' | b'n' => {
                        let pitch = candidate
                            .font
                            .as_ref()
                            .context("Majiro newline requires native font metrics")?
                            .line_pitch;
                        candidate.cursor = [
                            candidate.origin[0].wrapping_add(candidate.indent),
                            candidate.cursor[1].wrapping_add(pitch),
                        ];
                        candidate.clear_pending = false;
                        callback = Some((0x04ae36bd, vec![]));
                    }
                    b'g' => {
                        let values = hex_fields(&bytes, 6)?;
                        ensure!(bytes.len() == 61, "Majiro graphic control extent");
                        callback = Some((
                            0xd3ca70e5,
                            values
                                .into_iter()
                                .filter(|n| *n != -99)
                                .map(Value::Integer)
                                .collect(),
                        ));
                    }
                    b'x' => callback = Some((0xec771b85, vec![Value::native_string(&bytes[1..])?])),
                    b'p' => {
                        //423790 changes the diagnostic window caption only
                        //outside run mode1. Release mode has no caption action;
                        //the PAUSE script hook owns the actual input/wait.
                        ensure!(
                            self.config.run_mode == 1,
                            "Majiro debug pause caption presenter is not connected"
                        );
                        candidate.no_wait = false;
                        candidate.speed = -1;
                        callback = Some((crate::program::name_hash(b"PAUSE"), vec![]));
                    }
                    //t sets the same per-frame deadline subsequently read by
                    //native wait, rather than an unconditional owner sleep.
                    //v prepares audio/voice-history state. Neither is a
                    //callback-only completion; retain them for those services.
                    _ => bail!(
                        "Majiro console control {:?} needs its native external/layout service; original PC retained",
                        char::from(tag)
                    ),
                }
                candidate.record(&bytes)?;
            }
        
        Ok(callback)
    }
}
