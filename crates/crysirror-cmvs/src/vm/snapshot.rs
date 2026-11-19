// SPDX-License-Identifier: MPL-2.0
//! CrySirror's own payload for CSV2 save slots. The framing and header follow the native
//! format, but the checkpoint record is not interchangeable with the original program.

use super::{
    machine::PreparedScript, resources::path_string, ScriptSources, VmProfile,
};
use crate::Script;
use anyhow::{Context, Result};
use serde_json::Value;

pub(super) fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 15), 16).unwrap_or('0'));
    }
    out
}

pub(super) fn unhex(text: &str) -> Option<Vec<u8>> {
    let digits = text.as_bytes();
    if digits.len() % 2 != 0 {
        return None;
    }
    digits
        .chunks_exact(2)
        .map(|pair| {
            let high = char::from(pair[0]).to_digit(16)?;
            let low = char::from(pair[1]).to_digit(16)?;
            Some((high << 4 | low) as u8)
        })
        .collect()
}

pub(super) fn word(value: &Value, key: &str) -> Option<u32> {
    value.get(key)?.as_u64().map(|word| word as u32)
}

pub(super) fn words(value: &Value, key: &str) -> Option<Vec<u32>> {
    value
        .get(key)?
        .as_array()?
        .iter()
        .map(|word| word.as_u64().map(|word| word as u32))
        .collect()
}

pub(super) fn bytes(value: &Value, key: &str) -> Option<Vec<u8>> {
    unhex(value.get(key)?.as_str()?)
}

/// Scripts named by a snapshot, parsed away from the VM owner.
pub struct SavedScripts(pub(super) Vec<(u8, PreparedScript)>);

impl SavedScripts {
    pub fn prepare(profile: VmProfile, sources: &ScriptSources, snapshot: &Value) -> Result<Self> {
        let images = snapshot["vm"]["machine"]["images"]
            .as_array()
            .context("CMVS save has no script images")?;
        let mut scripts = Vec::with_capacity(images.len());
        for image in images {
            let slot = word(image, "slot").context("CMVS saved script slot")? as u8;
            let name = bytes(image, "name").context("CMVS saved script name")?;
            let native = path_string(&name)?;
            let input = sources
                .open_entry(&native)?
                .with_context(|| format!("CMVS saved script missing: {native}"))?;
            scripts.push((slot, PreparedScript::new(profile, Script::read(input.as_ref())?, name)?));
        }
        Ok(Self(scripts))
    }
}
