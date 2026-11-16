// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Palette-indexed linear image plans over ordinary engine packed-pixel and
//! image-processing jobs. No native framebuffer or private colour renderer.
use crate::{
    effects::{ImageEffect, MAX_EFFECT_PIXELS},
    integer_color::ColorMatrixFixed,
    rgb_lut::{RgbLookup, RgbLookupProfile},
};
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{PackedImageSpec, PackedPixelLayout};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct IndexedPlane {
    pub size: [u32; 2],
    pub palette: [[u8; 3]; 256],
    pub indices: Arc<[u8]>,
}
impl IndexedPlane {
    pub fn validate(&self) -> Result<()> {
        let bytes = crate::rgba_len(self.size[0], self.size[1])?;
        ensure!(
            bytes / 4 <= MAX_EFFECT_PIXELS as usize && self.indices.len() == bytes / 4,
            "indexed plane dimensions/index bytes"
        );
        Ok(())
    }
    /// First worker stage retains the original index in red and opaque alpha.
    /// Palette colour is deliberately applied only by the second worker stage.
    pub fn packed(&self) -> Result<PackedImageSpec> {
        self.validate()?;
        Ok(PackedImageSpec {
            size: self.size,
            layout: PackedPixelLayout::Red8,
            bytes: self.indices.clone(),
        })
    }
    /// Replicate the index, then apply independent palette channel tables.
    /// These are existing reusable image operations; no adapter scans pixels.
    pub fn effects(&self) -> Result<Vec<ImageEffect>> {
        self.validate()?;
        Ok(vec![
            ImageEffect::IntegerColorMatrix {
                spec: ColorMatrixFixed {
                    matrix: [[1, 0, 0]; 3],
                    offset: [0; 3],
                    divisor: 1,
                },
            },
            ImageEffect::RgbLookup {
                spec: RgbLookup {
                    red: self.palette.iter().map(|p| p[0]).collect(),
                    green: self.palette.iter().map(|p| p[1]).collect(),
                    blue: self.palette.iter().map(|p| p[2]).collect(),
                    profile: RgbLookupProfile::StraightRgba,
                },
            },
        ])
    }
}
