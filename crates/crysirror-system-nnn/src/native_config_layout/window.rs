// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonConfig's PrintMessageWindowDirect preview: the configured page,
//! window origin and sample message/name text. Composition and glyphs are
//! shared services; only original setup data is retained here.
use super::n;
use crate::config::Names;
use anyhow::{Result, ensure};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub(crate) struct Line {
    pub bytes: Arc<[u8]>,
    pub origin: [i32; 2],
    pub size: i32,
    pub rgb: [u8; 4],
}
#[derive(Clone, Debug)]
pub(crate) struct WindowPreview {
    pub page: i32,
    /// Absolute top-left of the body window (CCommonMessageWindow::PrintDirect).
    pub at: [i32; 2],
    pub messages: Vec<Line>,
    pub name: Option<Line>,
}
fn color(table: &Names, prefix: &str, default: [i32; 4]) -> Result<[i32; 4]> {
    Ok([
        n(table, &format!("{prefix}ColorR"), default[0])?,
        n(table, &format!("{prefix}ColorG"), default[1])?,
        n(table, &format!("{prefix}ColorB"), default[2])?,
        n(table, &format!("{prefix}ColorA"), default[3])?,
    ])
}
fn rgba(v: [i32; 4]) -> [u8; 4] {
    v.map(|c| c.clamp(0, 255) as u8)
}
impl WindowPreview {
    /// `variants` is GetWindowShurui; windowPrintFlag defaults to variants-1.
    pub(crate) fn read(table: &Names, variants: i32) -> Result<Option<Self>> {
        if n(table, "windowPrintFlag", variants - 1)? == 0 {
            return Ok(None);
        }
        // Clicking the preview cycles SetWindowNumberByConfig when more than
        // one configured variant exists; that owner is not connected.
        ensure!(
            variants <= 1,
            "NNN Config message-window variant selection requires its native owner"
        );
        let count = n(table, "windowMessageNumber", 0)?;
        ensure!((0..=64).contains(&count), "NNN Config window message budget");
        let origin = [n(table, "messagePrintX", 0)?, n(table, "messagePrintY", 0)?];
        let next = [n(table, "messageNextX", 0)?, n(table, "messageNextY", 20)?];
        let size = n(table, "messageFontSize", 24)?;
        ensure!((1..=4096).contains(&size), "NNN Config window message font size");
        let message_rgb = color(table, "message", [255; 4])?;
        let at = [n(table, "windowPrintX", 0)?, n(table, "windowPrintY", 0)?];
        let mut messages = Vec::new();
        for index in 0..count {
            let Some(bytes) = table.parameter(format!("windowMessage{}", index + 1).as_bytes()) else {
                continue;
            };
            ensure!(bytes.len() <= 1024 && !bytes.contains(&0), "NNN Config window message bounds");
            // '@' marks an intentionally blank row.
            if bytes.first() == Some(&b'@') {
                continue;
            }
            messages.push(Line {
                bytes: Arc::from(bytes),
                origin: [
                    at[0] + origin[0] + next[0] * index,
                    at[1] + origin[1] + next[1] * index,
                ],
                size,
                rgb: rgba(message_rgb),
            });
        }
        let name = match table.parameter(b"nameWindowMessage") {
            Some(bytes) => {
                ensure!(bytes.len() <= 1024 && !bytes.contains(&0), "NNN Config name message bounds");
                let size = n(table, "nameFontSize", size)?;
                ensure!((1..=4096).contains(&size), "NNN Config name font size");
                Some(Line {
                    bytes: Arc::from(bytes),
                    origin: [
                        at[0] + n(table, "namePrintX", 0)?,
                        at[1] + n(table, "namePrintY", 0)?,
                    ],
                    size,
                    rgb: rgba(color(table, "name", message_rgb)?),
                })
            }
            None => None,
        };
        Ok(Some(Self {
            page: n(table, "windowPrintPage", 1)?,
            at,
            messages,
            name,
        }))
    }
}
