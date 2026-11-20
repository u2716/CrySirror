// SPDX-License-Identifier: MPL-2.0
use super::super::checkpoint::{put_short, put_word, short, word};
use super::*;
use anyhow::{Context, Result, ensure};

impl Window {
    pub(in crate::adapter::cvns) fn record(&self, index: usize, out: &mut Vec<u8>) -> Result<()> {
        ensure!(
            self.native_face.len() < 64,
            "CSP1 message font exceeds the native face field"
        );
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.font_height.to_le_bytes());
        bytes.extend_from_slice(&self.font_weight.to_le_bytes());
        bytes.extend_from_slice(&self.native_face);
        bytes.push(0);
        let mut raw = [0; 160];
        for (i, v) in self
            .buffer
            .into_iter()
            .chain(self.position)
            .chain(self.area)
            .chain([self.size, self.spacing, self.line_spacing])
            .enumerate()
        {
            put_word(&mut raw, i * 4, v as u32);
        }
        raw[44..44 + self.native_face.len()].copy_from_slice(&self.native_face);
        for (i, v) in [
            self.kinsoku as u32,
            self.colors[0],
            self.colors[1],
            self.shadow.0 as u32,
            self.shadow.1,
            self.gradient,
            self.reveal[0],
            self.reveal[1],
            self.delay,
            self.shown as u32,
            self.layer,
            self.cursor[0] as u32,
            self.cursor[1] as u32,
        ]
        .into_iter()
        .enumerate()
        {
            put_word(&mut raw, 108 + 4 * i, v);
        }
        bytes.extend_from_slice(&raw);
        bytes.extend_from_slice(&u16::try_from(self.glyphs.len())?.to_le_bytes());
        for glyph in &self.glyphs {
            let mut node = [0; 124];
            put_word(&mut node, 4, 80);
            put_word(&mut node, 8, 10);
            put_short(&mut node, 22, 256);
            put_short(&mut node, 28, glyph.alpha as i16);
            let w = if glyph.full_width {
                glyph.size
            } else {
                glyph.size / 2
            };
            put_short(&mut node, 32, (w / 2 + 1) as i16);
            put_short(&mut node, 34, (glyph.size / 2 + 1) as i16);
            put_short(&mut node, 36, glyph.scale as i16);
            put_short(&mut node, 38, glyph.scale as i16);
            put_short(&mut node, 42, index as i16);
            let [x, y] = glyph.cell;
            for (i, value) in [
                x,
                y,
                x.wrapping_add(w as i32 + 1),
                y.wrapping_add(glyph.size as i32 + 1),
                x.wrapping_add(self.position[0]),
                y.wrapping_add(self.position[1]),
                -1,
            ]
            .into_iter()
            .enumerate()
            {
                put_short(&mut node, 44 + i * 2, value as i16);
            }
            put_short(&mut node, 80, 44);
            put_short(&mut node, 82, glyph.code as i16);
            for (i, v) in [
                glyph.gradient,
                x as u32,
                y as u32,
                glyph.size,
                glyph.colors[0],
                glyph.colors[1],
                1,
                u32::from(glyph.shadow.is_some()),
                glyph.shadow_color,
                256,
            ]
            .into_iter()
            .enumerate()
            {
                put_word(&mut node, 84 + 4 * i, v);
            }
            bytes.extend_from_slice(&node);
        }
        crate::vm::save_slot::csp1::append(out, 0x500, &[index as u16], &bytes)
    }

    pub(in crate::adapter::cvns) fn resolve_restored_font(&mut self, font: std::sync::Arc<str>) {
        self.font = font.clone();
        for glyph in &mut self.glyphs {
            glyph.font = font.clone();
        }
    }

    pub(in crate::adapter::cvns) fn from_record(
        bytes: &[u8],
        font: std::sync::Arc<str>,
        generation: usize,
    ) -> Result<Self> {
        ensure!(bytes.len() >= 5, "truncated CSP1 message record");
        let n = bytes[4..]
            .iter()
            .position(|&b| b == 0)
            .context("unterminated CSP1 font")?;
        ensure!(n < 64, "CSP1 message font exceeds its native field");
        let (raw, tail) = bytes[5 + n..]
            .split_at_checked(160)
            .context("truncated CSP1 message fields")?;
        let (count, nodes) = tail
            .split_at_checked(2)
            .context("missing CSP1 glyph count")?;
        ensure!(
            nodes.len() == usize::from(u16::from_le_bytes(count.try_into()?)) * 124,
            "invalid CSP1 glyph count"
        );
        let mut window = Self::new(font);
        window.native_face = bytes[4..4 + n].to_vec();
        window.default_face = window.native_face.is_empty();
        window.font_height = short(bytes, 0) as u16;
        window.font_weight = short(bytes, 2) as u16;
        window.buffer = [word(raw, 0) as i32, word(raw, 4) as i32];
        window.position = [word(raw, 8) as i32, word(raw, 12) as i32];
        window.area = std::array::from_fn(|i| word(raw, 16 + 4 * i) as i32);
        window.size = word(raw, 32) as i32;
        window.spacing = word(raw, 36) as i32;
        window.line_spacing = word(raw, 40) as i32;
        window.kinsoku = word(raw, 108) != 0;
        window.colors = [word(raw, 112), word(raw, 116)];
        window.shadow = (word(raw, 120) != 0, word(raw, 124));
        window.gradient = word(raw, 128);
        window.reveal = [word(raw, 132), word(raw, 136)];
        window.delay = word(raw, 140);
        window.shown = word(raw, 144) != 0;
        window.layer = word(raw, 148);
        // sub_422940 copies the scalar fields, then sub_421dd0 resets the
        // cursor before sub_421920 restores the saved glyphs without advancing it.
        window.clear();
        for node in nodes.chunks_exact(124) {
            let code = short(node, 82) as u16;
            let pair = code.to_be_bytes();
            let full_width = pair[0] != 0;
            let raw_code = if full_width { &pair[..] } else { &pair[1..] };
            let character = encoding_rs::SHIFT_JIS
                .decode_without_bom_handling(raw_code)
                .0
                .chars()
                .next()
                .unwrap_or(' ');
            let shadow_color = word(node, 116);
            let size = word(node, 96);
            ensure!(size <= u16::MAX.into(), "CSP1 glyph exceeds native font size");
            window.font_height = size as u16;
            window.glyphs.push(Glyph {
                code,
                character,
                full_width,
                size,
                generation,
                font: window.font.clone(),
                cell: [word(node, 88) as i32, word(node, 92) as i32],
                colors: [word(node, 100), word(node, 104)],
                gradient: word(node, 84),
                shadow: (word(node, 112) != 0).then_some(shadow_color),
                shadow_color,
                alpha: 256,
                scale: 1000,
                reveal: None,
            });
        }
        Ok(window)
    }
}
