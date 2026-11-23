// SPDX-License-Identifier: MPL-2.0
//! RLD v1 image-backed cmdBillboard surfaces. The host owns the image and composition.
use crate::branch::number;
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::HostServices,
    state::ClientId,
};
use serde_json::{Value, json};

#[derive(Debug, PartialEq)]
pub(super) struct Surface {
    pub slot: u8,
    pub asset: String,
    pub origin: [i32; 2],
    pub level: u8,
}

impl Surface {
    pub fn image_csv(raw: &[u8], extended: bool) -> Result<Self> {
        // splitParam preserves the final font descriptor (it has its own commas).
        let count = if extended { 27 } else { 26 };
        let fields: Vec<_> = raw.splitn(count, |b| *b == b',').collect();
        ensure!(fields.len() == count, "short ExHibit billboard definition");
        let n = |i: usize| number(fields[i]);
        let path_at = if extended { 20 } else { 19 };
        ensure!(n(0)? == 2, "unsupported ExHibit billboard source");
        let slot = n(1)?;
        ensure!(
            (0..127).contains(&slot),
            "unsupported ExHibit billboard layer {slot}"
        );
        ensure!(
            n(17)? == -1 && n(18)? == 0 && (!extended || n(19)? == 0) && n(11)? == 0 && n(12)? == 0,
            "unsupported ExHibit billboard surface properties or relative positioning"
        );
        let text = fields[path_at + 1];
        ensure!(
            text.is_empty() || text == b"-1",
            "unsupported ExHibit billboard text"
        );
        let origin = [n(4)?, n(5)?];
        ensure!(
            origin.iter().all(|v| (-32768..=32767).contains(v)),
            "unsupported ExHibit billboard encoded coordinates"
        );
        let effect = n(9)?;
        let level = match effect {
            519 => 0,
            255 => u8::try_from(n(10)?)
                .ok()
                .filter(|n| *n <= 16)
                .context("unsupported ExHibit billboard blend level")?,
            _ => anyhow::bail!("unsupported ExHibit billboard effect {effect}"),
        };
        let asset = std::str::from_utf8(fields[path_at])?.replace('\\', "/");
        Ok(Self {
            slot: slot as u8,
            asset: VfsPath::new(Mount::Asset, &asset)?.relative().to_owned(),
            origin,
            level,
        })
    }
    fn layer(&self, [width, height]: [u32; 2], asset: &str) -> Value {
        json!({"id":256 + u32::from(self.slot),"order":[2,self.slot],
            "rect":[self.origin[0],self.origin[1],width,height],
            "opacity":self.level.saturating_mul(16),
            "paint":{"kind":"image","asset":asset}})
    }
    pub fn put(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        size: [u32; 2],
        asset: &str,
    ) -> Result<()> {
        let layer = self.layer(size, asset);
        host.submit_engine(
            client,
            1,
            &serde_json::to_vec(&json!({"layers":[{"op":"put","layer":layer}]}))?,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fields(slot: i32, x: i32, effect: i32, level: i32) -> String {
        format!(
            "2,{slot},0,0,{x},40,123,456,400,{effect},{level},0,0,0,0,0,0,-1,0,0,res\\sprite.gyu,-1,0,0,0,0,font,0,400"
        )
    }
    #[test]
    fn hidden_and_exposed_billboards_use_distinct_layers_and_actual_bitmap_size() -> Result<()> {
        let make = |s, x, e, l| Surface::image_csv(fields(s, x, e, l).as_bytes(), true);
        let hidden = make(40, 600, 519, 16)?.layer([184, 15], "image");
        let exposed = make(41, -800, 255, 16)?.layer([800, 100], "image");
        assert_eq!(hidden["opacity"], 0);
        assert_eq!(hidden["rect"], json!([600, 40, 184, 15]));
        assert_eq!(exposed["opacity"], 255);
        assert_eq!(exposed["rect"], json!([-800, 40, 800, 100]));
        assert_eq!(hidden["id"], 296);
        assert_eq!(exposed["order"], json!([2, 41]));
        assert_eq!(make(41, 8, 255, 5)?.layer([2, 2], "image")["opacity"], 80);
        assert!(make(127, 0, 255, 16).is_err());
        assert!(make(1, 0, 9, 16).is_err());
        assert!(make(1, 100000001, 519, 16).is_err());
        Ok(())
    }
}
