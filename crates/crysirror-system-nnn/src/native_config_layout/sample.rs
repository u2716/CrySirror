// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original Config PrintSampleText's per-Print counter. The renderer receives
//! native message slices; glyph shaping/rasterization is never implemented here.
use super::n;
use crate::config::Names;
use anyhow::{Result, ensure};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub(crate) struct TextDraw {
    pub bytes: Arc<[u8]>,
    pub origin: [i32; 2],
    pub font_size: i32,
    pub rgb: [i32; 3],
    /// None calls PrintMessage; Some calls PrintMessageParts(0,length), whose
    /// native parser counts message cells, not Rust chars or UTF8 byte offsets.
    pub length: Option<i32>,
}
#[derive(Clone, Debug)]
pub(crate) struct Sample {
    pub page: i32,
    origin: [i32; 2],
    font_size: i32,
    rgb: [i32; 3],
    next_y: i32,
    wait: i32,
    repeat: bool,
    text: Vec<Arc<[u8]>>,
    moji_time: i32,
    line: usize,
    last: Option<(u64, Vec<TextDraw>)>,
}
impl Sample {
    pub(crate) fn read(table: &Names) -> Result<Option<Self>> {
        if n(table, "useSampleText", 0)? == 0 {
            return Ok(None);
        }
        let lines = n(table, "sampleTextGyo", 1)?;
        // A zero allocation was dereferenced by the original Print. Reject that
        // undefined profile instead of manufacturing a line of empty text.
        ensure!(
            (1..=128).contains(&lines),
            "NNN Config sample-text line budget"
        );
        let mut text = Vec::new();
        let mut bytes = 0usize;
        for line in 1..=lines {
            let value = table
                .parameter(format!("sampleText{line}").as_bytes())
                .unwrap_or(b"\x83T\x83\x93\x83v\x83\x8b\x83e\x83L\x83X\x83g");
            bytes += value.len();
            ensure!(
                bytes <= 1 << 20 && !value.contains(&0),
                "NNN Config sample-text byte budget"
            );
            text.push(Arc::from(value));
        }
        let font_size = n(table, "sampleTextFontSize", 24)?;
        ensure!(
            (1..=4096).contains(&font_size),
            "NNN Config sample font geometry"
        );
        Ok(Some(Self {
            page: n(table, "sampleTextPrintPage", 1)?,
            origin: [
                n(table, "sampleTextPrintX", 0)?,
                n(table, "sampleTextPrintY", 0)?,
            ],
            font_size,
            rgb: [
                n(table, "sampleTextColorR", 255)?,
                n(table, "sampleTextColorG", 255)?,
                n(table, "sampleTextColorB", 255)?,
            ],
            next_y: n(table, "sampleTextNextY", 28)?,
            wait: n(table, "sampleTextWait", 0)?,
            repeat: n(table, "sampleTextLoop", 1)? != 0,
            text,
            moji_time: 0,
            line: 0,
            last: None,
        }))
    }
    pub(crate) fn reset(&mut self) {
        self.moji_time = 0;
        self.line = 0;
        self.last = None;
    }
    pub(crate) fn print(
        &mut self,
        ticket: u64,
        frame_count: i32,
        speed: i32,
    ) -> Result<Vec<TextDraw>> {
        if let Some((previous, draws)) = &self.last {
            if *previous == ticket {
                return Ok(draws.clone());
            }
        }
        // GetNowFrameCount is the admitted callback FrameControl input, not
        // another wall-clock sample or the resource decoder's polling delay.
        self.moji_time = self.moji_time.wrapping_add(speed.wrapping_mul(frame_count));
        let mut visible = self.moji_time / 1000;
        // This raw strlen/2 test is peculiar to Config. Do not replace it with
        // a Unicode length or the richer message parser's glyph/cell count.
        let length = i32::try_from(self.text[self.line].len() / 2)?;
        if visible > length {
            visible = length;
            if self.line == self.text.len() - 1 {
                if self.moji_time
                    >= length
                        .wrapping_mul(1000)
                        .wrapping_add(self.wait.wrapping_mul(speed))
                    && self.repeat
                {
                    visible = 0;
                    self.line = 0;
                    self.moji_time = 0;
                }
            } else {
                visible = 0;
                self.line += 1;
                self.moji_time = 0;
            }
        }
        let draws = (0..=self.line)
            .map(|index| TextDraw {
                bytes: self.text[index].clone(),
                origin: [
                    self.origin[0],
                    self.origin[1].wrapping_add(self.next_y.wrapping_mul(index as i32)),
                ],
                font_size: self.font_size,
                rgb: self.rgb,
                length: (index == self.line).then_some(visible),
            })
            .collect::<Vec<_>>();
        self.last = Some((ticket, draws.clone()));
        Ok(draws)
    }
}
