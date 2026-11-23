// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded JSON syntax with byte-preserving strings. Raw legacy-encoded string
//! bytes remain raw; Unicode escapes become UTF-8, without guessing an encoding.
use anyhow::{ensure, Context, Result};
/// Original object order and duplicate fields are retained. This is distinct
/// from decoding all strings as UTF-8 or silently replacing invalid bytes.
#[derive(Clone, Debug, PartialEq)]
pub enum ByteJson {
    Null,
    Bool(bool),
    Number(f64),
    String(Vec<u8>),
    Array(Vec<ByteJson>),
    Object(Vec<(Vec<u8>, ByteJson)>),
}
impl ByteJson {
    /// First matching member, with ASCII-only case folding for legacy consumers.
    pub fn get_ascii_case_insensitive(&self, name: &[u8]) -> Option<&Self> {
        let Self::Object(fields) = self else {
            return None;
        };
        fields
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value)
    }
    /// Saturating truncation of a numeric value to a signed32-bit integer.
    pub fn integer32(&self) -> Result<i32> {
        let Self::Number(value) = self else {
            anyhow::bail!("byte JSON expected number")
        };
        ensure!(!value.is_nan(), "byte JSON invalid numeric value");
        Ok(*value as i32)
    }
    /// Borrow a string without converting or normalizing its original encoding.
    pub fn string_bytes(&self) -> Result<&[u8]> {
        let Self::String(bytes) = self else {
            anyhow::bail!("byte JSON expected string")
        };
        Ok(bytes)
    }
    /// Borrow a sequence without accepting an object as an array.
    pub fn array(&self) -> Result<&[Self]> {
        let Self::Array(values) = self else {
            anyhow::bail!("byte JSON expected array")
        };
        Ok(values)
    }
}
/// Admission limits for bytes, syntax nodes, nesting and cumulative string data.
#[derive(Clone, Copy, Debug)]
pub struct ByteJsonLimits {
    pub bytes: usize,
    pub nodes: usize,
    pub depth: usize,
    pub string_bytes: usize,
    pub number_bytes: usize,
}
impl Default for ByteJsonLimits {
    fn default() -> Self {
        Self {
            bytes: 64 * 1024 * 1024,
            nodes: 1_000_000,
            depth: 64,
            string_bytes: 64 * 1024 * 1024,
            number_bytes: 128,
        }
    }
}
/// Syntax policy selected explicitly by a legacy consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteJsonProfile {
    /// Complete JSON document with standard whitespace and string syntax.
    Strict,
    /// cJSON C-string/prefix conventions, raw controls, BOM and decimal prefix.
    /// Only this profile accepts trailing bytes and nonstandard numeric prefixes.
    LegacyCJson,
}
/// Parse one complete JSON document. The callback is polled cooperatively;
/// cancellation returns an error and never publishes a partially parsed tree.
pub fn parse_byte_json(
    bytes: &[u8],
    limits: ByteJsonLimits,
    check: &dyn Fn() -> Result<()>,
) -> Result<ByteJson> {
    parse_byte_json_profile(bytes, limits, ByteJsonProfile::Strict, check)
}
/// Explicit legacy syntax admission over the same bounded parser. The legacy
/// conventions follow cJSON (MIT, Copyright2009–2017 Dave Gamble and contributors),
/// inspected at xsystem4 a85467d/src/cJSON.c; no locale-global C parser is called.
pub fn parse_byte_json_profile(
    bytes: &[u8],
    limits: ByteJsonLimits,
    profile: ByteJsonProfile,
    check: &dyn Fn() -> Result<()>,
) -> Result<ByteJson> {
    ensure!(
        limits.bytes <= 64 * 1024 * 1024
            && limits.nodes <= 1_000_000
            && limits.depth <= 128
            && limits.string_bytes <= 64 * 1024 * 1024
            && limits.number_bytes <= 256,
        "byte JSON maximum limits"
    );
    ensure!(bytes.len() <= limits.bytes, "byte JSON input byte limit");
    let bytes = if profile == ByteJsonProfile::LegacyCJson {
        bytes.split(|b| *b == 0).next().unwrap()
    } else {
        bytes
    };
    let mut parser = Parser {
        bytes,
        at: if profile == ByteJsonProfile::LegacyCJson && bytes.starts_with(b"\xef\xbb\xbf") {
            3
        } else {
            0
        },
        profile,
        nodes: 0,
        strings: 0,
        limits,
        check,
    };
    let value = parser.value(0)?;
    parser.space()?;
    if profile == ByteJsonProfile::Strict {
        ensure!(parser.at == bytes.len(), "byte JSON trailing data");
    }
    Ok(value)
}
struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    profile: ByteJsonProfile,
    nodes: usize,
    strings: usize,
    limits: ByteJsonLimits,
    check: &'a dyn Fn() -> Result<()>,
}
impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }
    fn space(&mut self) -> Result<()> {
        while self.peek().is_some_and(|b| {
            if self.profile == ByteJsonProfile::LegacyCJson {
                b <= 32
            } else {
                matches!(b, b' ' | b'\t' | b'\r' | b'\n')
            }
        }) {
            if self.at % 4096 == 0 {
                (self.check)()?;
            }
            self.at += 1;
        }
        Ok(())
    }
    fn claim(&mut self) -> Result<()> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|n| *n <= self.limits.nodes)
            .context("byte JSON node limit")?;
        (self.check)()
    }
    fn value(&mut self, depth: usize) -> Result<ByteJson> {
        ensure!(depth <= self.limits.depth, "byte JSON depth limit");
        self.space()?;
        self.claim()?;
        Ok(match self.peek().context("byte JSON missing value")? {
            b'n' => {
                self.word(b"null")?;
                ByteJson::Null
            }
            b't' => {
                self.word(b"true")?;
                ByteJson::Bool(true)
            }
            b'f' => {
                self.word(b"false")?;
                ByteJson::Bool(false)
            }
            b'"' => ByteJson::String(self.string()?),
            b'[' => {
                self.at += 1;
                self.space()?;
                let mut values = Vec::new();
                if self.peek() == Some(b']') {
                    self.at += 1;
                    return Ok(ByteJson::Array(values));
                }
                loop {
                    values.push(self.value(depth + 1)?);
                    self.space()?;
                    match self.peek() {
                        Some(b',') => self.at += 1,
                        Some(b']') => {
                            self.at += 1;
                            break;
                        }
                        _ => anyhow::bail!("byte JSON array delimiter"),
                    }
                }
                ByteJson::Array(values)
            }
            b'{' => {
                self.at += 1;
                self.space()?;
                let mut fields = Vec::new();
                if self.peek() == Some(b'}') {
                    self.at += 1;
                    return Ok(ByteJson::Object(fields));
                }
                loop {
                    self.space()?;
                    self.claim()?;
                    let key = self.string()?;
                    self.space()?;
                    ensure!(self.peek() == Some(b':'), "byte JSON object colon");
                    self.at += 1;
                    let value = self.value(depth + 1)?;
                    fields.push((key, value));
                    self.space()?;
                    match self.peek() {
                        Some(b',') => self.at += 1,
                        Some(b'}') => {
                            self.at += 1;
                            break;
                        }
                        _ => anyhow::bail!("byte JSON object delimiter"),
                    }
                }
                ByteJson::Object(fields)
            }
            b'-' | b'0'..=b'9' => ByteJson::Number(self.number()?),
            _ => anyhow::bail!("byte JSON invalid value"),
        })
    }
    fn word(&mut self, word: &[u8]) -> Result<()> {
        let end = self
            .at
            .checked_add(word.len())
            .context("byte JSON word offset")?;
        ensure!(
            self.bytes.get(self.at..end) == Some(word),
            "byte JSON literal"
        );
        self.at = end;
        Ok(())
    }
    fn push(&mut self, out: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
        self.strings = self
            .strings
            .checked_add(bytes.len())
            .filter(|n| *n <= self.limits.string_bytes)
            .context("byte JSON string byte limit")?;
        out.extend_from_slice(bytes);
        Ok(())
    }
    fn string(&mut self) -> Result<Vec<u8>> {
        ensure!(self.peek() == Some(b'"'), "byte JSON string opening quote");
        self.at += 1;
        let mut out = Vec::new();
        loop {
            if self.at % 4096 == 0 {
                (self.check)()?;
            }
            let byte = self.peek().context("byte JSON string terminator")?;
            self.at += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escaped = self.peek().context("byte JSON escape terminator")?;
                    self.at += 1;
                    match escaped {
                        b'"' | b'\\' | b'/' => self.push(&mut out, &[escaped])?,
                        b'b' => self.push(&mut out, &[8])?,
                        b'f' => self.push(&mut out, &[12])?,
                        b'n' => self.push(&mut out, b"\n")?,
                        b'r' => self.push(&mut out, b"\r")?,
                        b't' => self.push(&mut out, b"\t")?,
                        b'u' => {
                            let first = self.hex4()?;
                            let scalar = if (0xd800..=0xdbff).contains(&first) {
                                ensure!(
                                    self.bytes.get(self.at..self.at + 2) == Some(b"\\u"),
                                    "byte JSON missing low surrogate"
                                );
                                self.at += 2;
                                let low = self.hex4()?;
                                ensure!(
                                    (0xdc00..=0xdfff).contains(&low),
                                    "byte JSON low surrogate"
                                );
                                0x10000
                                    + ((u32::from(first) - 0xd800) << 10)
                                    + (u32::from(low) - 0xdc00)
                            } else {
                                ensure!(
                                    !(0xdc00..=0xdfff).contains(&first),
                                    "byte JSON isolated low surrogate"
                                );
                                u32::from(first)
                            };
                            let character =
                                char::from_u32(scalar).context("byte JSON Unicode scalar")?;
                            let mut utf8 = [0; 4];
                            self.push(&mut out, character.encode_utf8(&mut utf8).as_bytes())?;
                        }
                        _ => anyhow::bail!("byte JSON invalid escape"),
                    }
                }
                0..=31 if self.profile == ByteJsonProfile::Strict => {
                    anyhow::bail!("byte JSON unescaped string control")
                }
                byte => self.push(&mut out, &[byte])?,
            }
        }
    }
    fn hex4(&mut self) -> Result<u16> {
        let end = self.at.checked_add(4).context("byte JSON Unicode offset")?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .context("byte JSON truncated Unicode escape")?;
        // Original parse_hex4 returns zero for an invalid four-byte sequence.
        // Keep this quirk local to the explicit legacy profile.
        self.at = end;
        let mut value = 0u16;
        for byte in bytes {
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ if self.profile == ByteJsonProfile::LegacyCJson => return Ok(0),
                _ => anyhow::bail!("byte JSON Unicode hex digit"),
            };
            value = (value << 4) | u16::from(digit);
        }
        Ok(value)
    }
    fn legacy_number(&mut self) -> Result<f64> {
        let start = self.at;
        let max = 63.min(self.limits.number_bytes);
        let input = &self.bytes[start..];
        let n = input
            .iter()
            .take(max)
            .take_while(|b| matches!(b, b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' | b'.'))
            .count();
        let token = &input[..n];
        let mut at = usize::from(token.first() == Some(&b'-'));
        let integer = at;
        while token.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        let integer_digits = at - integer;
        let mut fraction_digits = 0;
        if token.get(at) == Some(&b'.') {
            at += 1;
            let fraction = at;
            while token.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            fraction_digits = at - fraction;
        }
        ensure!(
            integer_digits + fraction_digits > 0,
            "byte JSON legacy number"
        );
        let mantissa_end = at;
        if token.get(at).is_some_and(|b| matches!(b, b'e' | b'E')) {
            at += 1;
            if token.get(at).is_some_and(|b| matches!(b, b'+' | b'-')) {
                at += 1;
            }
            let exponent = at;
            while token.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            if at == exponent {
                at = mantissa_end;
            }
        }
        self.at += at;
        let mut text = std::str::from_utf8(&token[..at])?.to_owned();
        if text.starts_with("-.") {
            text.insert(1, '0');
        }
        if let Some(dot) = text.find('.') {
            if !text.as_bytes().get(dot + 1).is_some_and(u8::is_ascii_digit) {
                text.insert(dot + 1, '0');
            }
        }
        Ok(text.parse()?)
    }
    fn number(&mut self) -> Result<f64> {
        if self.profile == ByteJsonProfile::LegacyCJson {
            return self.legacy_number();
        }
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                self.at += 1;
                while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.at += 1;
                    ensure!(
                        self.at - start <= self.limits.number_bytes,
                        "byte JSON numeric token limit"
                    );
                }
            }
            _ => anyhow::bail!("byte JSON integer part"),
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            let first = self.at;
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.at += 1;
                ensure!(
                    self.at - start <= self.limits.number_bytes,
                    "byte JSON numeric token limit"
                );
            }
            ensure!(self.at > first, "byte JSON fraction digits");
        }
        if self.peek().is_some_and(|b| matches!(b, b'e' | b'E')) {
            self.at += 1;
            if self.peek().is_some_and(|b| matches!(b, b'+' | b'-')) {
                self.at += 1;
            }
            let first = self.at;
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.at += 1;
                ensure!(
                    self.at - start <= self.limits.number_bytes,
                    "byte JSON numeric token limit"
                );
            }
            ensure!(self.at > first, "byte JSON exponent digits");
        }
        ensure!(
            self.at - start <= self.limits.number_bytes,
            "byte JSON numeric token limit"
        );
        Ok(std::str::from_utf8(&self.bytes[start..self.at])?.parse()?)
    }
}
