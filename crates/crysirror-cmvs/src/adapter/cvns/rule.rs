// SPDX-License-Identifier: MPL-2.0
//! Rule masks at 0x422b10, sampled by the 0x416b80 and 0x416cd0 blitters.
use super::{DecodedImage, SpriteId};
use anyhow::Result;
use crysirror_core::state::ClientId;
use crysirror_image::label_image::LabelImage;

pub(crate) struct Request {
    pub name: String,
    pub native_name: Vec<u8>,
    pub target: u16,
    pub duration: u32,
    pub edge: u32,
    pub reveal: bool,
    pub screen: Option<DecodedImage>,
}

pub(crate) struct Active {
    name: Vec<u8>,
    pub(super) target: SpriteId,
    pub duration: u32,
    pub elapsed: u32,
    pub reveal: bool,
    edge: u32,
    labels: LabelImage,
}

impl Active {
    pub fn new(request: &Request, mask: DecodedImage) -> Self {
        let labels = mask.rgba().chunks_exact(4).map(|p| p[1]).collect();
        Self {
            name: request.native_name.clone(),
            target: SpriteId::Global(request.target), duration: request.duration,
            elapsed: 0, reveal: request.reveal, edge: request.edge,
            labels: LabelImage { size: mask.size(), labels },
        }
    }

    /// sub_422cd0 writes the load arguments and a copy of the live palette
    /// record. sub_422d30 keeps only the elapsed word after reopening the mask.
    pub(super) fn record(&self,out:&mut Vec<u8>) -> Result<()> {
        let SpriteId::Global(target)=self.target else { anyhow::bail!("invalid CVNS rule target"); };
        let target=u32::from(target);
        let mode=u32::from(self.reveal);
        let span=self.duration.wrapping_sub(self.edge);
        let mut bytes=Vec::with_capacity(1073+self.name.len());
        for word in [target,self.duration,self.edge,mode] {bytes.extend_from_slice(&word.to_le_bytes());}
        bytes.extend_from_slice(&self.name); bytes.push(0);
        // The original pointer (field 4) is process-local and ignored on import.
        for word in [mode,target,span,self.edge,0,self.labels.size[0],self.labels.size[1]] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for i in 0..256u32 {bytes.extend_from_slice(&(i.wrapping_mul(span)/255).to_le_bytes());}
        bytes.extend_from_slice(&self.elapsed.to_le_bytes());
        crate::vm::save_slot::csp1::append(out,0x620,&[],&bytes)
    }

    pub fn remaining(&self) -> u32 {
        self.duration.saturating_sub(self.elapsed)
    }

    pub fn sample(&self, client: ClientId) -> Result<DecodedImage> {
        let palette = std::array::from_fn(|i| {
            let delay = (i as u32).wrapping_mul(self.duration.wrapping_sub(self.edge)) / 255;
            let weight = if self.elapsed < delay {
                if self.reveal { 0 } else { 256 }
            } else if self.elapsed >= delay.wrapping_add(self.edge) || self.edge == 0 {
                if self.reveal { 256 } else { 0 }
            } else {
                let age = self.elapsed - delay;
                if self.reveal { (age.wrapping_mul(256) / self.edge).min(256) }
                else { 255 - age.wrapping_mul(255) / self.edge }
            };
            let byte = ((weight * 255 + 128) >> 8) as u8;
            [byte, byte, byte, 255]
        });
        DecodedImage::from_surface(client, self.labels.expand_rgba(&palette)?, Some(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reveal_and_hide_use_different_integer_numerators() -> Result<()> {
        let mut rule = Active {
            name: b"mask".to_vec(),
            target: SpriteId::Global(1), duration: 455, edge: 200,
            elapsed: 100, reveal: true,
            labels: LabelImage { size: [3, 1], labels: [0, 100, 255].into() },
        };
        let red = |rule: &Active| -> Result<Vec<u8>> {
            Ok(rule.sample(ClientId(1))?.rgba().iter().step_by(4).copied().collect())
        };
        assert_eq!(red(&rule)?, [128, 0, 0]);
        rule.reveal = false;
        assert_eq!(red(&rule)?, [128, 254, 255]);
        rule.elapsed = 455;
        assert_eq!(red(&rule)?, [0, 0, 0]);
        Ok(())
    }
}
