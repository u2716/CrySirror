// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS image objects (68E6F0 family) over shared immutable image resources. Blits and
//! screen copies are shared Image jobs; this module only holds their native geometry.
use crate::native_image_resource::{Geometry, Prepared};
use anyhow::{ensure, Result};
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageResource,
};
use std::sync::Arc;

/// Image handed out for an image list's work bitmap (message 285).
pub(crate) const LIST_BITMAP: u32 = 0x4800_0000;

/// A native surface in progress; the native call stays pending until it is done.
pub(crate) enum Pending {
    /// 112, 113, 118: the job's image becomes a new handle.
    Create(ImageJob),
    /// 116, 115, 367: waiting for a capture of the source list's region.
    Capture { job: ImageJob, blit: Blit, region: u32 },
    /// The composite that replaces the destination image.
    Compose { job: ImageJob, destination: u32 },
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Blit {
    pub destination: u32,
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub source_at: [i32; 2],
    pub math: Math,
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Math {
    Copy,
    AlphaOver,
}
/// The list's rectangle as a small temporary Canvas: the whole work bitmap is far
/// larger than one capture may be, and a Canvas copy owns its draw nodes.
pub(crate) fn region_commands(source: u32, region: u32, rect: [f32; 4]) -> Vec<crysirror_scene::canvas::CanvasCommand> {
    use crysirror_scene::canvas::CanvasCommand as C;
    vec![
        C::Create { id: region, width: rect[2] as u32, height: rect[3] as u32 },
        C::Copy { source, destination: region, rect, position: [0.; 2], clip: None, opacity: 255, replace: true, atop: None },
    ]
}
pub(crate) fn empty_input(size: [u32; 2]) -> Result<ImageInput> {
    ensure!(
        size.iter().all(|v| (1..=8192).contains(v)) && u64::from(size[0]) * u64::from(size[1]) <= 16 * 1024 * 1024,
        "CatSystem2 native image extent"
    );
    Ok(ImageInput::Solid { size, color: [0; 4] })
}
pub(crate) fn prepared(image: Arc<dyn ImageResource>) -> Prepared {
    let size = image.size();
    Prepared {
        source: String::new(),
        geometry: Geometry {
            frame_id: 0,
            size,
            canvas: size,
            origin: [0; 2],
            base: None,
            color_bits: 32,
            flags: None,
            image_mode: None,
            compression_type: None,
        },
        image,
    }
}
/// 68B620 clips both rectangles to their images, in the source's origin frame.
pub(crate) fn clip(blit: Blit, source: &Prepared, destination: &Prepared) -> Option<([i32; 2], [i32; 2], [i32; 2])> {
    let (so, dst_origin) = (source.geometry.origin, destination.geometry.origin);
    let ss = source.image.size();
    let ds = destination.image.size();
    let mut src = [blit.source_at[0] - so[0], blit.source_at[1] - so[1]];
    let mut dst = [blit.at[0] + so[0] - dst_origin[0], blit.at[1] + so[1] - dst_origin[1]];
    let mut size = blit.size;
    for axis in 0..2 {
        let before = (-src[axis]).max(-dst[axis]).max(0);
        src[axis] += before;
        dst[axis] += before;
        size[axis] -= before;
        size[axis] = size[axis].min(ss[axis] as i32 - src[axis]).min(ds[axis] as i32 - dst[axis]);
        if size[axis] <= 0 {
            return None;
        }
    }
    Some((src, dst, size))
}
pub(crate) fn composite(
    base: Arc<dyn ImageResource>,
    source: Arc<dyn ImageResource>,
    src: [i32; 2],
    dst: [i32; 2],
    size: [i32; 2],
    math: Math,
) -> ImageInput {
    ImageInput::Composite {
        base: Box::new(ImageInput::Resource(base)),
        layers: vec![ImageCompositeLayer {
            input: Box::new(ImageInput::Resource(source)),
            source: [src[0], src[1], size[0], size[1]],
            destination: dst,
            clip: None,
            opacity: 255,
            math: match math {
                Math::Copy => ImageCompositeMath::Copy,
                Math::AlphaOver => ImageCompositeMath::AlphaOnAlphaScalar256,
            },
        }],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_images_have_a_budget() {
        assert!(empty_input([1536, 814]).is_ok());
        assert!(empty_input([0, 1]).is_err());
    }
}
