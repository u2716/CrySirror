// SPDX-License-Identifier: MPL-2.0
//! The v1 poweredByRetouch layer uses a once-per-session TmAsyncDelayFade.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::HostServices,
    state::ClientId,
};
use serde_json::json;

#[derive(Debug, PartialEq)]
pub(super) struct Billboard {
    pub path: String,
    pub x: i32,
    pub y: i32,
    pub slot: i32,
}
pub(super) struct Fade {
    layer: u32,
    start: u64,
    level: u8,
}

impl Billboard {
    pub fn parse(fields: &[u8]) -> Result<Self> {
        let mut f = fields.split(|b| *b == b',');
        let raw =
            std::str::from_utf8(f.next().context("missing billboard path")?)?.replace('\\', "/");
        let path = VfsPath::new(Mount::Asset, &raw)?.relative().to_owned();
        let mut arg = || crate::branch::number(f.next().context("missing billboard argument")?);
        let (x, y, slot) = (arg()?, arg()?, arg()?);
        ensure!(
            f.next().is_none() && (-32768..=32767).contains(&x) && (-32768..=32767).contains(&y),
            "unsupported ExHibit billboard coordinate encoding"
        );
        Ok(Self { path, x, y, slot })
    }

    pub fn begin(self, h: &mut dyn HostServices, c: ClientId) -> Result<Option<Fade>> {
        if self.slot < 0 {
            return Ok(None);
        }
        ensure!(self.slot < 127, "unsupported ExHibit billboard slot");
        let layer = 256 + self.slot as u32;
        let [w, height] = h.image_size(c, &self.path)?;
        let payload = json!({"layers":[{"op":"put","layer":{"id":layer,"order":[2,self.slot],"rect":[self.x,self.y,w,height],"opacity":0,"paint":{"kind":"image","asset":self.path}}}]});
        h.submit_engine(c, 1, &serde_json::to_vec(&payload)?)?;
        Ok(Some(Fade {
            layer,
            start: h.now_ms(),
            level: 0,
        }))
    }
}
impl Fade {
    fn level(elapsed: u64) -> u8 {
        match elapsed {
            0..300 => (12 * elapsed / 300) as u8,
            300..4300 => 12,
            4300..5100 => 12 - (12 * (elapsed - 4300) / 800) as u8,
            _ => 0,
        }
    }
    pub fn present(&mut self, h: &mut dyn HostServices, c: ClientId) -> Result<bool> {
        let age = h.now_ms().saturating_sub(self.start);
        let level = Self::level(age);
        if self.level != level {
            let payload =
                json!({"layers":[{"op":"set_opacity","id":self.layer,"opacity":level*16}]});
            h.submit_engine(c, 1, &serde_json::to_vec(&payload)?)?;
            self.level = level;
        }
        Ok(age >= 5100)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn title_billboard_keeps_its_slot_and_native_fade_plateau() -> Result<()> {
        let b = Billboard::parse(br"res\g\gn\retouch.gyu,680,16,85")?;
        assert_eq!(
            (b.path.as_str(), b.x, b.y, b.slot),
            ("res/g/gn/retouch.gyu", 680, 16, 85)
        );
        assert_eq!(
            [0, 25, 299, 300, 4299, 4300, 4700, 5099, 5100].map(Fade::level),
            [0, 1, 11, 12, 12, 12, 6, 1, 0]
        );
        Ok(())
    }
}
