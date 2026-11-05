use std::borrow::Cow;
use std::str::FromStr;

use encoding_rs::{Encoding, GBK, SHIFT_JIS, UTF_8};

use crate::native_asset::error::{AssetError, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Nls {
    ShiftJis,
    Gbk,
    Utf8,
}

impl Nls {
    pub fn name(self) -> &'static str {
        match self {
            Self::ShiftJis => "sjis",
            Self::Gbk => "gbk",
            Self::Utf8 => "utf-8",
        }
    }

    pub fn encoding(self) -> &'static Encoding {
        match self {
            Self::ShiftJis => SHIFT_JIS,
            Self::Gbk => GBK,
            Self::Utf8 => UTF_8,
        }
    }

    pub fn decode(self, bytes: &[u8]) -> Result<String> {
        let (decoded, _encoding_used, had_errors) = self.encoding().decode(bytes);
        if had_errors {
            return Err(AssetError::EncodingDecode { nls: self.name() });
        }
        Ok(match decoded {
            Cow::Borrowed(s) => s.to_owned(),
            Cow::Owned(s) => s,
        })
    }

    pub fn encode(self, text: &str) -> Result<Vec<u8>> {
        let (encoded, _encoding_used, had_errors) = self.encoding().encode(text);
        if had_errors {
            return Err(AssetError::EncodingEncode {
                nls: self.name(),
                text: text.to_owned(),
            });
        }
        Ok(match encoded {
            Cow::Borrowed(bytes) => bytes.to_vec(),
            Cow::Owned(bytes) => bytes,
        })
    }
}

impl Default for Nls {
    fn default() -> Self {
        Self::ShiftJis
    }
}

impl FromStr for Nls {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "sjis" | "shift-jis" | "shift_jis" | "cp932" | "932" => Ok(Self::ShiftJis),
            "gbk" | "cp936" | "936" => Ok(Self::Gbk),
            "utf8" | "utf-8" => Ok(Self::Utf8),
            other => Err(format!(
                "unsupported nls {:?}; expected sjis, gbk, or utf-8",
                other
            )),
        }
    }
}
