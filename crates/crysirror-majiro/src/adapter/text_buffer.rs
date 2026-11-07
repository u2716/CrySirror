// Copyright (c) 2026 CrySirror contributors. MIT.
//! Native416F00 encoded message buffer. Raster/layout remain shared engine
//! work; accepting an append never claims that PROC has displayed its text.
use super::*;
impl MajiroVm {
    pub(super) fn append_text(&mut self, bytes: &[u8]) -> Result<Option<Vec<Value>>> {
        let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
        let count = self
            .text_buffer
            .len()
            .checked_add(bytes.len())
            .context("Majiro message buffer overflow")?;
        //The original buffer has1024 bytes plus a separate allocation flag.
        //Reject its overflow domain instead of overwriting adjacent native RAM.
        ensure!(
            count <= 1023,
            "Majiro text exceeds native1024-byte message allocation"
        );
        self.text_buffer.extend_from_slice(bytes);
        Ok(Some(vec![]))
    }
}
