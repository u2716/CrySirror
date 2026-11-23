// SPDX-License-Identifier: MPL-2.0
use super::{ClientId, HostServices, Result};
use crysirror_core::path::{Mount, VfsPath};
use crysirror_image::{
    ImageSurface,
    luma_lookup::{LookupIndex, LumaLookup},
};
use std::sync::mpsc::Receiver;

#[derive(Clone, Copy)]
pub(crate) enum Tone {
    Mono,
    Sepia,
}
pub(super) fn split(name: &str) -> (Option<Tone>, &str) {
    if let Some(raw) = name.strip_prefix("[mono]") {
        (Some(Tone::Mono), raw)
    } else if let Some(raw) = name.strip_prefix("[sepia]") {
        (Some(Tone::Sepia), raw)
    } else {
        (None, name)
    }
}
pub(crate) fn path(tagged: &str) -> Option<(Tone, String)> {
    let (bank, name) = tagged.split_once('/')?;
    let (tone, raw) = split(name);
    Some((tone?, format!("{bank}/{raw}")))
}
impl Tone {
    pub fn map(self, source: &ImageSurface) -> Result<ImageSurface> {
        let table = (0..=255u32)
            .map(|v| match self {
                Self::Mono => [v as u8, v as u8, v as u8, 255],
                Self::Sepia => {
                    let component = |k| ((u64::from(v) * k) >> 32) as u8;
                    [
                        component(0xf0f0f0f0),
                        component(0xc8c8c8c8),
                        component(0x91919191),
                        255,
                    ]
                }
            })
            .collect();
        let lookup = LumaLookup {
            table,
            index: LookupIndex::WeightedRgb {
                coefficients: [5, 2, 1],
                fractional_bits: 3,
            },
            preserve_alpha: true,
        };
        let mut image = lookup.apply(source, &|| Ok(()))?;
        for (out, original) in image
            .rgba
            .chunks_exact_mut(4)
            .zip(source.rgba.chunks_exact(4))
        {
            if original[3] == 0 {
                out.copy_from_slice(original);
            }
        }
        Ok(image)
    }
}
pub(super) fn prepare(
    h: &mut dyn HostServices,
    c: ClientId,
    tone: Tone,
    path: String,
) -> Result<Receiver<Result<ImageSurface>>> {
    let storage = h.storage(c)?;
    crate::worker(move || {
        let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &path)?)?;
        let source = crysirror_image::decode_standard(&crysirror_resource::read_all(
            reader.as_ref(),
            64 << 20,
        )?)?;
        tone.map(&source)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_tables_weight_red_and_leave_invisible_pixels_untouched() {
        let source = ImageSurface {
            width: 2,
            height: 1,
            rgba: vec![200, 64, 8, 128, 200, 64, 8, 0],
        };
        assert_eq!(
            Tone::Mono.map(&source).unwrap().rgba,
            [142, 142, 142, 128, 200, 64, 8, 0]
        );
        assert_eq!(
            Tone::Sepia.map(&source).unwrap().rgba,
            [133, 111, 80, 128, 200, 64, 8, 0]
        );
    }
}
