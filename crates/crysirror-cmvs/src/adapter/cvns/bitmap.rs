// SPDX-License-Identifier: MPL-2.0
//! The eight component images and separate baked result of the 2004 bitmap object.
use super::{BitmapSlot, DecodedImage};
use anyhow::{Context, Result, ensure};
use crysirror_core::state::ClientId;
use crysirror_image::{ImageSurface, layer_stack::LayerStack};
use std::sync::Arc;

/// Component data copied by sub_41ba50. Device pointers and baked pixels
/// are excluded. Keep the script's CP932 bytes, including spelling and case.
#[derive(Clone)]
struct Component {
    name: Vec<u8>,
    size: [u32; 2],
    flag: u32,
    frame: u32,
    position: [i32; 2],
    tint: u32,
    filter: [u32; 2],
}

impl Component {
    fn blank(size: [u32; 2]) -> Self {
        Self { name: Vec::new(), size, flag: 0, frame: 0, position: [0; 2],
            tint: 0xffffff, filter: [0; 2] }
    }

    fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        ensure!(self.name.len() < 64, "CVNS bitmap name exceeds CSP1 component");
        out.extend_from_slice(&self.name);
        out.resize(out.len() + 64 - self.name.len(), 0);
        for value in [self.size[0], self.size[1], self.flag, self.frame,
                      self.position[0] as u32, self.position[1] as u32,
                      self.tint, self.filter[0], self.filter[1]] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
pub(super) struct Components([Option<Component>; 8]);

impl Components {
    pub(super) fn record(&self, tag: u16, indices: &[u16], out: &mut Vec<u8>) -> Result<()> {
        let count = self.0.iter().flatten().count() as u16;
        let mut bytes = count.to_le_bytes().to_vec();
        for (index, component) in self.0.iter().enumerate() {
            if let Some(component) = component {
                bytes.extend_from_slice(&(index as u16).to_le_bytes());
                component.write(&mut bytes)?;
            }
        }
        crate::vm::save_slot::csp1::append(out, tag, indices, &bytes)
    }
}

#[derive(Clone, Default)]
pub(crate) struct Picture {
    pub(super) components: Components,
    stack: LayerStack,
    alpha: [bool; 8],
    composed: Option<Arc<ImageSurface>>,
}

pub(crate) enum Edit {
    Create([u32; 2]),
    Capture(DecodedImage),
    Load {
        layer: usize,
        position: [i32; 2],
        flag: u32,
        name: Vec<u8>,
        frame: u32,
    },
    Release(usize),
    Bake,
    Copy,
    Tint(usize, u32),
    Filter(usize, u32, u32),
    Position(usize, [i32; 2]),
    Fill(u32),
    CopyLayer {
        source: Picture,
        from: usize,
        to: usize,
        position: [i32; 2],
        same: bool,
    },
}

pub(crate) struct Job {
    pub slot: BitmapSlot,
    pub words: u16,
    pub picture: Picture,
    pub edit: Edit,
}

pub(crate) struct Prepared {
    pub slot: BitmapSlot,
    pub words: u16,
    pub picture: Picture,
    pub result: Option<u32>,
    pub refresh: bool,
    pub image: Option<DecodedImage>,
}

impl Picture {
    /// sub_41e000 reconstructs the pixels from assets on the worker. Unnamed
    /// components create a blank base. The composed image is regenerated.
    pub(super) fn from_record(
        slot: BitmapSlot, bytes: &[u8], client: ClientId,
        mut load: impl FnMut(&str, u32) -> Result<DecodedImage>,
    ) -> Result<Prepared> {
        let (count, components) = bytes.split_at_checked(2).context("short CVNS bitmap record")?;
        let count = usize::from(u16::from_le_bytes(count.try_into()?));
        ensure!(count <= 8 && components.len() == count * 102, "invalid CVNS bitmap components");
        let mut picture = Self::default();
        for bytes in components.chunks_exact(102) {
            let index = usize::from(u16::from_le_bytes(bytes[..2].try_into()?));
            ensure!(index < 8, "CVNS bitmap record index");
            let name = &bytes[2..66];
            let end = name.iter().position(|&b| b == 0).context("unterminated CSP1 bitmap name")?;
            let fields = bytes[66..].as_chunks::<4>().0;
            let value = |i:usize| u32::from_le_bytes(fields[i]);
            let edit = if end != 0 {
                Some(Edit::Load { layer: index, name: name[..end].to_vec(), flag: value(2),
                    frame: value(3), position: [value(4) as i32, value(5) as i32] })
            } else if picture.size(0).is_none() {
                Some(Edit::Create([value(0),value(1)]))
            } else { None };
            for edit in [edit, (value(6) != 0xffffff).then_some(Edit::Tint(index,value(6))),
                (value(7) != 0).then_some(Edit::Filter(index,value(7),value(8)))].into_iter().flatten() {
                let job = Job { words: 0, slot, picture, edit };
                picture = job.apply(client, &mut load, false)?.picture;
            }
        }
        let edit = if count >= 2 { Edit::Bake } else { Edit::Copy };
        Job { words: 0, slot, picture, edit }.run(client, load)
    }

    pub fn size(&self, layer: usize) -> Option<[u32; 2]> {
        self.stack.surface(layer).map(|s| [s.width, s.height])
    }
}

impl Job {
    pub fn run(
        self,
        client: ClientId,
        load: impl FnOnce(&str, u32) -> Result<DecodedImage>,
    ) -> Result<Prepared> {
        self.apply(client, load, true)
    }

    fn apply(
        mut self, client: ClientId, load: impl FnOnce(&str, u32) -> Result<DecodedImage>,
        publish: bool,
    ) -> Result<Prepared> {
        let picture = &mut self.picture;
        let mut refresh = false;
        let mut result = None;
        match self.edit {
            Edit::Capture(image) => {
                let [width, height] = image.size();
                let mut rgba = image.rgba().to_vec();
                for alpha in rgba.iter_mut().skip(3).step_by(4) { *alpha = 255; }
                picture.components.0[0] = Some(Component::blank([width, height]));
                picture.stack.place(0, ImageSurface { width, height, rgba }, true, [0; 2]);
                result = Some(1);
                refresh = true;
            }
            Edit::Create(size) => {
                crysirror_image::rgba_len(size[0], size[1])?;
                picture.components.0[0] = Some(Component::blank(size));
                picture.stack.solid(0, size, [0; 2]);
                refresh = true;
            }
            Edit::Load {
                layer,
                position,
                flag,
                name,
                frame,
            } => {
                let (path, invalid) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&name);
                ensure!(!invalid, "invalid CVNS bitmap name");
                let image = load(&path, frame)?;
                let [width, height] = image.size();
                picture.stack.place(
                    layer,
                    ImageSurface {
                        width,
                        height,
                        rgba: image.rgba().to_vec(),
                    },
                    true,
                    position,
                );
                picture.components.0[layer] = Some(Component { name, flag, position, frame, ..Component::blank([width, height]) });
                picture.alpha[layer] = flag != 0;
                refresh = layer == 0 && picture.composed.is_none();
            }
            Edit::Copy => {
                // 0x41c4b0 copies the eight records but excludes the baked pointer at +896.
                picture.composed = None;
                refresh = true;
            }
            Edit::Release(layer) => {
                picture.components.0[layer] = None;
                picture.stack.release(layer);
                picture.alpha[layer] = false;
                if !picture.stack.has_overlays() {
                    refresh = picture.composed.take().is_some();
                }
            }
            Edit::Bake => {
                picture.composed = if picture.stack.has_overlays() {
                    picture.stack.bake().map(Arc::new)
                } else {
                    None
                };
                refresh = true;
            }
            Edit::CopyLayer {
                source,
                from,
                to,
                position,
                same,
            } => {
                picture.components.0[to] = None;
                picture.stack.release(to);
                if !picture.stack.has_overlays() {
                    refresh = picture.composed.take().is_some();
                }
                if !same {
                    picture.stack.copy_layer(to, &source.stack, from, position);
                    picture.alpha[to] = source.alpha[from];
                    picture.components.0[to] = source.components.0[from].clone().map(|mut c| {
                        c.position = position;
                        c
                    });
                }
                refresh |= to == 0 && picture.composed.is_none();
            }
            Edit::Position(layer, position) => {
                if let Some(component) = &mut picture.components.0[layer] { component.position = position; }
                picture.stack.reposition(layer, position);
            }
            Edit::Tint(layer, color) => {
                let range = if layer >= 8 { 0..8 } else { layer..layer+1 };
                for component in &mut picture.components.0[range] {
                    let Some(component) = component else { break };
                    component.tint = color;
                }
                picture
                    .stack
                    .tint(if layer >= 8 { usize::MAX } else { layer }, color);
                refresh = (layer == 0 || layer >= 8) && picture.composed.is_none();
            }
            Edit::Filter(layer, mode, parameter) => {
                picture.stack.filter(layer, mode, parameter);
                if let Some(component) = &mut picture.components.0[layer] {
                    match mode {
                        0 => component.filter = [0, 0],
                        1 => component.filter = [1, 256],
                        3 | 4 | 9 => component.filter = [mode, parameter],
                        10..=16 => component.filter = [mode, parameter & !1],
                        _ => {}
                    }
                }
                refresh = layer == 0 && picture.composed.is_none();
            }
            Edit::Fill(color) => {
                let [r, g, b] = super::rgb(color);
                picture.stack.fill_rgba(0, [r, g, b, 255]);
                refresh = picture.composed.is_none();
            }
        }
        let surface = picture.composed.as_deref().or_else(|| picture.stack.surface(0));
        let image = if let Some(surface) = surface.filter(|_| refresh && publish) {
            let mut surface = surface.clone();
            if !picture.alpha[0] {
                for alpha in surface.rgba.iter_mut().skip(3).step_by(4) {
                    *alpha = 255;
                }
            }
            Some(DecodedImage::from_surface(client, surface, Some(picture.alpha[0]))?)
        } else {
            None
        };
        Ok(Prepared {
            slot: self.slot,
            words: self.words,
            picture: self.picture,
            result,
            refresh,
            image,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csp1_restores_components_and_rebakes_from_the_named_frame() -> Result<()> {
        let client = ClientId(1);
        let slot = BitmapSlot::Global(19);
        let load = |name: &str, frame| {
            assert_eq!((name, frame), ("あ.pb2", 7));
            DecodedImage::from_surface(client, ImageSurface::filled(1, 1, [64, 224, 128, 255])?, Some(true))
        };
        let mut picture = Picture::default();
        for edit in [
            Edit::Create([2,1]),
            Edit::Fill(0xffffff),
            Edit::Load { layer: 3, name: vec![0x82,0xa0,b'.',b'p',b'b',b'2'], flag: 7, frame: 7, position: [1,0] },
            Edit::Filter(3,1,0),
        ] {
            picture = Job { slot, words:0, picture, edit }.run(client,load)?.picture;
        }
        let mut record=Vec::new();
        picture.components.record(0x480,&[19],&mut record)?;
        crate::vm::save_slot::csp1::visit(&record,|tag,indices,data| {
            assert_eq!((tag,indices),(0x480,&[19][..]));
            let pixels=Picture::from_record(slot,data,client,load)?.image.unwrap();
            assert_eq!(&pixels.rgba()[..4], &[0,0,0,255]);
            assert_eq!(&pixels.rgba()[4..], &[191,31,127,255]);
            Ok(())
        })
    }

    #[test]
    fn baked_image_is_independent_of_components_and_copies() -> Result<()> {
        let client = ClientId(1);
        let run = |picture, edit| Job { slot: BitmapSlot::Global(0), words: 0, picture, edit }
            .run(client, |_, _| {
                let surface = ImageSurface::filled(1, 1, [255, 0, 0, 128])?;
                DecodedImage::from_surface(client, surface, Some(true))
            });
        let base = run(Picture::default(), Edit::Create([2, 1]))?.picture;
        let overlay = run(base, Edit::Load {
            layer: 1, position: [1, 0], flag: 0, name: b"overlay".to_vec(), frame: 0,
        })?;
        assert!(!overlay.refresh);
        let baked = run(overlay.picture, Edit::Bake)?;
        assert_eq!(&baked.image.unwrap().rgba()[4..8], &[127, 0, 0, 255]);
        let modified = run(baked.picture, Edit::Fill(0xffffff))?;
        assert!(!modified.refresh);
        let copied = run(modified.picture.clone(), Edit::Copy)?;
        assert_eq!(&copied.image.unwrap().rgba()[4..8], &[255; 4]);
        let released = run(modified.picture, Edit::Release(1))?;
        assert_eq!(&released.image.unwrap().rgba()[4..8], &[255; 4]);
        let empty = run(Picture::default(), Edit::Copy)?;
        assert!(empty.refresh && empty.image.is_none());
        Ok(())
    }
}
