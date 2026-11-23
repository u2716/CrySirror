// SPDX-License-Identifier: MPL-2.0
//! Original 454ED0/4561A0 slot records. These contain portable logical data,
//! never native linked-list pointers, engine handles or reconstructed pixels.
//! The separate fileNNc.dat thumbnail is not part of this body.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Slot {
    pub title: String,
    pub memo: String,
    ///454ED0 history payload+20/44C510 String1: cvon or final c voice source,
    ///not the displayed speaker. Kept named for codec API compatibility.
    pub previous_caption: String,
    pub script: String,
    pub section: String,
    pub display_caption: String,
    pub line: i32,
    pub text_offset: i32,
    pub current_text: String,
    pub resume_text: String,
    pub audio: SlotAudio,
    /// 4B62C4,4B62C8,4B4158,4B60B8,4B60B4,4B60B0, in file order.
    pub execution: [i32; 6],
    pub calls: Vec<Call>,
    pub variables: Vec<Variable>,
    pub definitions: Vec<Variable>,
    pub selections: Vec<Selection>,
    pub windows: Vec<SlotWindow>,
    pub sprites: Vec<SlotSprite>,
    pub buffers: Vec<SlotBuffer>,
    pub buttons: Vec<SlotButton>,
    pub gauges: Vec<SlotGauge>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SlotAudio {
    pub bgm_mode: i32,
    pub bgm_gain: i32,
    pub bgm: String,
    pub se_mode: i32,
    pub se: String,
    pub env_mode: i32,
    pub env: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Call {
    Jsb {
        section: String,
        pc: i32,
    },
    Gsb {
        script: String,
        section: String,
        pc: i32,
        continuation: String,
    },
    While(i32),
    Top,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Variable {
    pub name: String,
    pub value: StoredValue,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum StoredValue {
    Scalar(NativeValue),
    /// Original list children share a name; the reader uses that name as its
    /// group boundary. No count/JSON/container header is invented here.
    List {
        element_name: String,
        elements: Vec<NativeValue>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Selection {
    pub rect: [i32; 4],
    pub text: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct SlotWindow {
    /// Native offsets: colour 4..7, rect 8..14, byte20 (not visibility24), text bounds52..58,
    /// colour48..51, blend64, cursor38/40, attributes33/32/35/34.
    pub fields: [i32; 18],
    pub frame: Option<WindowFrame>,
    pub foreground: Option<WindowForeground>,
    pub page: Option<WindowPage>,
    pub buttons: Vec<WindowButton>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WindowFrame {
    pub image: String,
    pub position: [i32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WindowForeground {
    pub image: String,
    pub fields: [i32; 4],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WindowPage {
    pub image: String,
    pub fields: [i32; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct WindowButton {
    pub image: String,
    pub label: String,
    pub position: [i32; 2],
    pub overlay: String,
    pub overlay_position: [i32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SlotSprite {
    pub image: String,
    /// Position X/Y, native system19, cell24, flags56/23/57, word68.
    pub fields: [i32; 8],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SlotBuffer {
    /// Native controller fields5919/5920, 4B5ED8, fields5921/5922,
    /// animation5579/5581 and image5597. Numbers are native word offsets.
    pub fields: [i32; 8],
    pub image: String,
    pub position: [i32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SlotButton {
    pub image: String,
    pub position: [i32; 2],
    pub text: String,
    pub text_offset: [i32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SlotGauge {
    /// Packed RGBA, XYWH, enabled20, mode24, subtype26, maximum28, value30.
    pub fields: [i32; 10],
}

impl Slot {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= 0x10000 && bytes.len() % 4 == 0,
            "SugarPot native slot size"
        );
        let mut c = Cursor::new(bytes);
        ensure!(c.word()? == 1, "SugarPot native slot version");
        c.take(12)?;
        let title = fixed(&mut c, 64)?;
        let memo = fixed(&mut c, 32)?;
        let previous_caption = fixed(&mut c, 32)?;
        let script = fixed(&mut c, 16)?;
        let section = fixed(&mut c, 32)?;
        let display_caption = fixed(&mut c, 32)?;
        let line = c.word()?;
        let text_offset = c.word()?;
        ensure!(
            line >= 0 && text_offset >= 0,
            "SugarPot native slot position"
        );
        let current_text = c.string(0x10000)?;
        let resume_text = c.string(0x10000)?;
        let audio = SlotAudio {
            bgm_mode: c.word()?,
            bgm_gain: c.word()?,
            bgm: fixed(&mut c, 16)?,
            se_mode: c.word()?,
            se: fixed(&mut c, 16)?,
            env_mode: c.word()?,
            env: fixed(&mut c, 16)?,
        };
        let execution = words(&mut c)?;
        tag(&mut c, b"STK\0")?;
        let mut calls = Vec::new();
        while !peek(&c, b"VAR\0") {
            ensure!(calls.len() < 4096, "SugarPot saved call limit");
            calls.push(match c.string(4)?.as_str() {
                "jsb" => Call::Jsb {
                    section: c.string(32)?,
                    pc: c.word()?,
                },
                "gsb" => Call::Gsb {
                    script: c.string(16)?,
                    section: c.string(32)?,
                    pc: c.word()?,
                    continuation: c.string(0x10000)?,
                },
                "wh" => Call::While(c.word()?),
                "top" => Call::Top,
                kind => anyhow::bail!("unsupported SugarPot saved call {kind}"),
            });
        }
        tag(&mut c, b"VAR\0")?;
        let variables = variable_block(&mut c, b"DEF\0")?;
        tag(&mut c, b"DEF\0")?;
        let definitions = variable_block(&mut c, b"SEL\0")?;
        tag(&mut c, b"SEL\0")?;
        let count =
            usize::try_from(c.word()?).context("negative SugarPot saved selection count")?;
        ensure!(count <= 1024, "SugarPot saved selection limit");
        let mut selections = Vec::with_capacity(count);
        for _ in 0..count {
            selections.push(Selection {
                rect: words(&mut c)?,
                text: c.string(0x10000)?,
            });
        }
        tag(&mut c, b"WND\0")?;
        let mut windows = Vec::new();
        while !peek(&c, b"SPR\0") {
            ensure!(windows.len() < 1024, "SugarPot saved window limit");
            let fields = words(&mut c)?;
            let frame = if peek(&c, b"FRM\0") {
                c.take(4)?;
                Some(WindowFrame {
                    image: fixed(&mut c, 16)?,
                    position: words(&mut c)?,
                })
            } else {
                None
            };
            let foreground = if peek(&c, b"FP\0\0") {
                c.take(4)?;
                Some(WindowForeground {
                    image: fixed(&mut c, 16)?,
                    fields: words(&mut c)?,
                })
            } else {
                None
            };
            let page = if peek(&c, b"PP\0\0") {
                c.take(4)?;
                Some(WindowPage {
                    image: fixed(&mut c, 16)?,
                    fields: words(&mut c)?,
                })
            } else {
                None
            };
            let mut buttons = Vec::new();
            if peek(&c, b"BTN\0") {
                c.take(4)?;
                while !peek(&c, b"SPR\0") {
                    ensure!(buttons.len() < 1024, "SugarPot saved window button limit");
                    buttons.push(WindowButton {
                        image: fixed(&mut c, 32)?,
                        label: fixed(&mut c, 32)?,
                        position: words(&mut c)?,
                        overlay: fixed(&mut c, 32)?,
                        overlay_position: words(&mut c)?,
                    });
                }
            }
            windows.push(SlotWindow {
                fields,
                frame,
                foreground,
                page,
                buttons,
            });
        }
        tag(&mut c, b"SPR\0")?;
        let mut sprites = Vec::new();
        while !peek(&c, b"BUF\0") {
            ensure!(sprites.len() < 1024, "SugarPot saved sprite limit");
            sprites.push(SlotSprite {
                image: fixed(&mut c, 16)?,
                fields: words(&mut c)?,
            });
        }
        tag(&mut c, b"BUF\0")?;
        let mut buffers = Vec::new();
        while !peek(&c, b"BTN\0") {
            ensure!(buffers.len() < 1024, "SugarPot saved buffer limit");
            buffers.push(SlotBuffer {
                fields: words(&mut c)?,
                image: fixed(&mut c, 16)?,
                position: words(&mut c)?,
            });
        }
        tag(&mut c, b"BTN\0")?;
        let mut buttons = Vec::new();
        while !peek(&c, b"ANM\0") {
            ensure!(buttons.len() < 1024, "SugarPot saved button limit");
            buttons.push(SlotButton {
                image: fixed(&mut c, 16)?,
                position: words(&mut c)?,
                text: c.string(0x10000)?,
                text_offset: words(&mut c)?,
            });
        }
        tag(&mut c, b"ANM\0")?;
        tag(&mut c, b"GG\0\0")?;
        let mut gauges = Vec::new();
        while c.position < bytes.len() {
            ensure!(gauges.len() < 1024, "SugarPot saved gauge limit");
            gauges.push(SlotGauge {
                fields: words(&mut c)?,
            });
        }
        Ok(Self {
            title,
            memo,
            previous_caption,
            script,
            section,
            display_caption,
            line,
            text_offset,
            current_text,
            resume_text,
            audio,
            execution,
            calls,
            variables,
            definitions,
            selections,
            windows,
            sprites,
            buffers,
            buttons,
            gauges,
        })
    }
    pub fn encode(&self) -> Result<Vec<u8>> {
        ensure!(
            self.calls.len() <= 4096
                && self.variables.len() <= 8192
                && self.definitions.len() <= 8192
                && self.windows.len() <= 1024
                && self.sprites.len() <= 1024
                && self.buffers.len() <= 1024
                && self.buttons.len() <= 1024
                && self.gauges.len() <= 1024,
            "SugarPot native slot record budget"
        );
        let mut out = vec![0; 16];
        out[..4].copy_from_slice(&1i32.to_le_bytes());
        for (text, size) in [
            (&self.title, 64),
            (&self.memo, 32),
            (&self.previous_caption, 32),
            (&self.script, 16),
            (&self.section, 32),
            (&self.display_caption, 32),
        ] {
            put_fixed(&mut out, text, size)?;
        }
        ensure!(
            self.line >= 0 && self.text_offset >= 0,
            "SugarPot saved position"
        );
        put_word(&mut out, self.line);
        put_word(&mut out, self.text_offset);
        put_string(&mut out, &self.current_text, 0x10000)?;
        put_string(&mut out, &self.resume_text, 0x10000)?;
        put_word(&mut out, self.audio.bgm_mode);
        put_word(&mut out, self.audio.bgm_gain);
        put_fixed(&mut out, &self.audio.bgm, 16)?;
        put_word(&mut out, self.audio.se_mode);
        put_fixed(&mut out, &self.audio.se, 16)?;
        put_word(&mut out, self.audio.env_mode);
        put_fixed(&mut out, &self.audio.env, 16)?;
        put_words(&mut out, &self.execution);
        out.extend_from_slice(b"STK\0");
        for call in &self.calls {
            match call {
                Call::Jsb { section, pc } => {
                    out.extend_from_slice(b"jsb\0");
                    put_string(&mut out, section, 32)?;
                    put_word(&mut out, *pc);
                }
                Call::Gsb {
                    script,
                    section,
                    pc,
                    continuation,
                } => {
                    out.extend_from_slice(b"gsb\0");
                    put_string(&mut out, script, 16)?;
                    put_string(&mut out, section, 32)?;
                    put_word(&mut out, *pc);
                    put_string(&mut out, continuation, 0x10000)?;
                }
                Call::While(pc) => {
                    out.extend_from_slice(b"wh\0\0");
                    put_word(&mut out, *pc);
                }
                Call::Top => out.extend_from_slice(b"top\0"),
            }
        }
        out.extend_from_slice(b"VAR\0");
        put_variables(&mut out, &self.variables)?;
        out.extend_from_slice(b"DEF\0");
        put_variables(&mut out, &self.definitions)?;
        out.extend_from_slice(b"SEL\0");
        ensure!(
            self.selections.len() <= 1024,
            "SugarPot saved selection limit"
        );
        put_word(&mut out, self.selections.len() as i32);
        for selection in &self.selections {
            put_words(&mut out, &selection.rect);
            put_string(&mut out, &selection.text, 0x10000)?;
        }
        out.extend_from_slice(b"WND\0");
        for (i, window) in self.windows.iter().enumerate() {
            put_words(&mut out, &window.fields);
            if let Some(frame) = &window.frame {
                out.extend_from_slice(b"FRM\0");
                put_fixed(&mut out, &frame.image, 16)?;
                put_words(&mut out, &frame.position);
            }
            if let Some(foreground) = &window.foreground {
                out.extend_from_slice(b"FP\0\0");
                put_fixed(&mut out, &foreground.image, 16)?;
                put_words(&mut out, &foreground.fields);
            }
            if let Some(page) = &window.page {
                out.extend_from_slice(b"PP\0\0");
                put_fixed(&mut out, &page.image, 16)?;
                put_words(&mut out, &page.fields);
            }
            if !window.buttons.is_empty() {
                // 4561A0 consumes inline buttons until SPR, without a window
                // count. Refuse unrepresentable intermediate button groups.
                ensure!(
                    i + 1 == self.windows.len(),
                    "SugarPot inline window buttons must precede SPR"
                );
                out.extend_from_slice(b"BTN\0");
                for button in &window.buttons {
                    put_fixed(&mut out, &button.image, 32)?;
                    put_fixed(&mut out, &button.label, 32)?;
                    put_words(&mut out, &button.position);
                    put_fixed(&mut out, &button.overlay, 32)?;
                    put_words(&mut out, &button.overlay_position);
                }
            }
        }
        out.extend_from_slice(b"SPR\0");
        for sprite in &self.sprites {
            put_fixed(&mut out, &sprite.image, 16)?;
            put_words(&mut out, &sprite.fields);
        }
        out.extend_from_slice(b"BUF\0");
        for buffer in &self.buffers {
            put_words(&mut out, &buffer.fields);
            put_fixed(&mut out, &buffer.image, 16)?;
            put_words(&mut out, &buffer.position);
        }
        out.extend_from_slice(b"BTN\0");
        for button in &self.buttons {
            put_fixed(&mut out, &button.image, 16)?;
            put_words(&mut out, &button.position);
            put_string(&mut out, &button.text, 0x10000)?;
            put_words(&mut out, &button.text_offset);
        }
        out.extend_from_slice(b"ANM\0GG\0\0");
        for gauge in &self.gauges {
            put_words(&mut out, &gauge.fields);
        }
        ensure!(out.len() <= 0x10000, "SugarPot native slot writer budget");
        Ok(out)
    }
}
fn peek(c: &Cursor<'_>, expected: &[u8; 4]) -> bool {
    c.bytes.get(c.position..c.position + 4) == Some(expected.as_slice())
}
fn tag(c: &mut Cursor<'_>, expected: &[u8; 4]) -> Result<()> {
    ensure!(
        c.take(4)? == expected,
        "SugarPot native slot section tag {:?}",
        expected
    );
    Ok(())
}
fn fixed(c: &mut Cursor<'_>, size: usize) -> Result<String> {
    let bytes = c.take(size)?;
    let end = bytes
        .iter()
        .position(|b| *b == 0)
        .context("SugarPot fixed save string terminator")?;
    SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(&bytes[..end])
        .map(|s| s.into_owned())
        .context("SugarPot fixed save string CP932")
}
fn put_fixed(out: &mut Vec<u8>, text: &str, size: usize) -> Result<()> {
    let (bytes, _, invalid) = SHIFT_JIS.encode(text);
    ensure!(
        !invalid && bytes.len() < size && !bytes.contains(&0),
        "SugarPot fixed save string CP932/capacity"
    );
    ensure!(
        out.len().checked_add(size).is_some_and(|n| n <= 0x10000),
        "SugarPot fixed save string budget"
    );
    out.extend_from_slice(&bytes);
    out.resize(out.len() + size - bytes.len(), 0);
    Ok(())
}
fn words<const N: usize>(c: &mut Cursor<'_>) -> Result<[i32; N]> {
    let mut values = [0; N];
    for value in &mut values {
        *value = c.word()?;
    }
    Ok(values)
}
fn put_words(out: &mut Vec<u8>, values: &[i32]) {
    for value in values {
        put_word(out, *value);
    }
}
fn variable_block(c: &mut Cursor<'_>, end: &[u8; 4]) -> Result<Vec<Variable>> {
    let mut variables = Vec::new();
    while !peek(c, end) {
        ensure!(variables.len() < 8192, "SugarPot saved variable limit");
        let name = c.string(243)?;
        let kind = c.word()?;
        let value = if kind == 42 {
            ensure!(
                !peek(c, end),
                "SugarPot empty list has no native save boundary"
            );
            let first = c.position;
            let element_name = c.string(16)?;
            c.position = first;
            let mut elements = Vec::new();
            loop {
                if peek(c, end) {
                    break;
                }
                let position = c.position;
                let next = c.string(243)?;
                if next != element_name {
                    c.position = position;
                    break;
                }
                ensure!(elements.len() < 8192, "SugarPot saved list limit");
                let kind = c.word()?;
                elements.push(read_value(c, kind)?);
            }
            StoredValue::List {
                element_name,
                elements,
            }
        } else {
            StoredValue::Scalar(read_value(c, kind)?)
        };
        variables.push(Variable { name, value });
    }
    Ok(variables)
}
fn put_variables(out: &mut Vec<u8>, variables: &[Variable]) -> Result<()> {
    for variable in variables {
        ensure!(
            !["DEF", "SEL"].contains(&variable.name.as_str()),
            "SugarPot variable conflicts with save tag"
        );
        put_string(out, &variable.name, 243)?;
        match &variable.value {
            StoredValue::Scalar(value) => write_value(out, value)?,
            StoredValue::List {
                element_name,
                elements,
            } => {
                ensure!(
                    !elements.is_empty(),
                    "SugarPot empty list has no native save boundary"
                );
                ensure!(elements.len() <= 8192, "SugarPot saved list limit");
                ensure!(
                    !["DEF", "SEL"].contains(&element_name.as_str()),
                    "SugarPot list conflicts with save tag"
                );
                put_word(out, 42);
                for value in elements {
                    put_string(out, element_name, 16)?;
                    write_value(out, value)?;
                }
            }
        }
    }
    Ok(())
}
