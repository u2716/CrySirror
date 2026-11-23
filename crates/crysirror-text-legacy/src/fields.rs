// Copyright (c) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Owned byte-field cursors with explicit chunk, comment and quoting policy.
//! This is not RFC CSV: separators/comments may deliberately apply inside quotes.
use super::{ByteLeadProfile, Result};
use anyhow::{Context, ensure};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct ByteFieldOptions {
    pub lead: ByteLeadProfile,
    pub separator: u8,
    pub comments: Vec<Vec<u8>>,
    pub chunk_bytes: usize,
    pub trim_max: u8,
    pub strip_quote: Option<u8>,
    pub skip_blank_lines: bool,
    pub windows_text: bool,
}
#[derive(Clone)]
pub struct ByteFieldCursor {
    source: Arc<[u8]>,
    options: ByteFieldOptions,
    offset: usize,
    pending: Option<Vec<u8>>,
}
impl ByteFieldCursor {
    pub fn new(source: Arc<[u8]>, options: ByteFieldOptions, limit: usize) -> Result<Self> {
        ensure!(
            source.len() <= limit && limit <= super::MAX_BYTE_STRING,
            "byte-field source budget"
        );
        options.lead.validate()?;
        ensure!(
            (1..=4096).contains(&options.chunk_bytes)
                && options.trim_max <= 32
                && options.separator != 0
                && !options.lead.is_lead(options.separator)
                && options.comments.len() <= 8
                && options
                    .comments
                    .iter()
                    .all(|v| !v.is_empty() && v.len() <= 8),
            "byte-field grammar limits"
        );
        Ok(Self {
            source,
            options,
            offset: 0,
            pending: None,
        })
    }
    /// Only commit the cursor after a complete successful read. Cancellation,
    /// malformed byte units and work errors retain the old source position.
    pub fn next_field(&mut self, check: &dyn Fn() -> Result<()>) -> Result<Option<Vec<u8>>> {
        let mut next = self.clone();
        let value = next.read_field(check, true)?;
        *self = next;
        Ok(value)
    }
    /// The same cursor grammar, retaining surrounding quote bytes for numeric
    /// or other callers whose native conversion does not strip them.
    pub fn next_raw_field(&mut self, check: &dyn Fn() -> Result<()>) -> Result<Option<Vec<u8>>> {
        let mut next = self.clone();
        let value = next.read_field(check, false)?;
        *self = next;
        Ok(value)
    }
    fn find(&self, bytes: &[u8], needle: &[u8]) -> Result<Option<usize>> {
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at..].starts_with(needle) {
                return Ok(Some(at));
            }
            let width = if self.options.lead.is_lead(bytes[at]) {
                2
            } else {
                1
            };
            ensure!(
                at + width <= bytes.len(),
                "truncated byte-field character at chunk boundary"
            );
            at += width;
        }
        Ok(None)
    }
    fn trim<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        let first = bytes
            .iter()
            .position(|b| *b > self.options.trim_max)
            .unwrap_or(bytes.len());
        let last = bytes
            .iter()
            .rposition(|b| *b > self.options.trim_max)
            .map_or(first, |n| n + 1);
        &bytes[first..last]
    }
    fn strip_comments(&self, bytes: &mut Vec<u8>) -> Result<()> {
        let mut at = 0;
        while at < bytes.len() {
            if self
                .options
                .comments
                .iter()
                .any(|comment| bytes[at..].starts_with(comment))
            {
                bytes.truncate(at);
                break;
            }
            let width = if self.options.lead.is_lead(bytes[at]) {
                2
            } else {
                1
            };
            ensure!(
                at + width <= bytes.len(),
                "truncated byte-field comment character"
            );
            at += width;
        }
        Ok(())
    }
    fn read_field(
        &mut self,
        check: &dyn Fn() -> Result<()>,
        strip_quotes: bool,
    ) -> Result<Option<Vec<u8>>> {
        loop {
            check()?;
            if self.pending.is_none() {
                if self.offset == self.source.len() {
                    return Ok(None);
                }
                let mut chunk = Vec::with_capacity(self.options.chunk_bytes);
                while self.offset < self.source.len() && chunk.len() < self.options.chunk_bytes {
                    let mut byte = self.source[self.offset];
                    self.offset += 1;
                    if self.options.windows_text && byte == 26 {
                        self.offset = self.source.len();
                        break;
                    }
                    if self.options.windows_text
                        && byte == 13
                        && self.source.get(self.offset) == Some(&10)
                    {
                        byte = 10;
                        self.offset += 1;
                    }
                    chunk.push(byte);
                    if byte == 10 {
                        break;
                    }
                }
                chunk.truncate(chunk.iter().position(|b| *b == 0).unwrap_or(chunk.len()));
                self.strip_comments(&mut chunk)?;
                let chunk = self.trim(&chunk).to_vec();
                if chunk.is_empty() && self.options.skip_blank_lines {
                    continue;
                }
                self.pending = Some(chunk);
            }
            let bytes = self.pending.take().context("byte-field pending chunk")?;
            let mut field = if let Some(at) = self.find(&bytes, &[self.options.separator])? {
                self.pending = Some(bytes[at + 1..].to_vec());
                bytes[..at].to_vec()
            } else {
                bytes
            };
            self.strip_comments(&mut field)?;
            let mut field = self.trim(&field);
            if let Some(quote) = self.options.strip_quote.filter(|_| strip_quotes) {
                if field.len() >= 2 && field.first() == Some(&quote) && field.last() == Some(&quote)
                {
                    field = &field[1..field.len() - 1];
                }
            }
            return Ok(Some(field.to_vec()));
        }
    }
}
