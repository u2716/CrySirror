// SPDX-License-Identifier: MPL-2.0
//! CVNS mask sheets, character registrations and their render resources.

use super::DecodedImage;
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    service::{HostServices, UploadedImage},
    state::ClientId,
};
use crysirror_image::alpha_cells;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Sheet {
    size: u32,
    width: u32,
    mask: Vec<u8>,
}

impl Sheet {
    pub fn new(image: DecodedImage, size: u32) -> Self {
        Self {
            size,
            width: image.size()[0],
            mask: image.rgba().chunks_exact(4).map(|p| p[0]).collect(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Ink {
    pub code: u16,
    pub size: u32,
    pub colors: [u32; 2],
    pub gradient: u32,
    pub shadow: Option<u32>,
}

#[derive(Default)]
pub(in crate::adapter) struct Fonts {
    sheets: Vec<Sheet>,
    // First registration for a character wins, including an empty one.
    glyphs: BTreeMap<u16, (usize, BTreeMap<u32, Vec<u8>>)>,
    published: BTreeMap<Ink, UploadedImage>,
}

impl Fonts {
    pub fn sheet(&mut self, sheet: Sheet) -> Result<()> {
        ensure!(
            self.sheets.len() < 16,
            "CVNS mask-font record holds at most sixteen sizes"
        );
        self.sheets.push(sheet);
        Ok(())
    }

    pub fn generation(&self) -> usize {
        self.glyphs.len()
    }

    pub fn register(&mut self, cell: u32, code: u16) -> Result<()> {
        if self.glyphs.contains_key(&code) {
            return Ok(());
        }
        let mut sizes = BTreeMap::new();
        for sheet in &self.sheets {
            let size = sheet.size;
            crysirror_image::rgba_len(size, size)?;
            let data = alpha_cells::cut(&sheet.mask, sheet.width, size, cell)
                .with_context(|| format!("CVNS mask-font cell {cell} outside sheet"))?;
            sizes.entry(size).or_insert(data);
        }
        self.glyphs.insert(code, (self.generation() + 1, sizes));
        Ok(())
    }

    pub(super) fn image(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        ink: Ink,
        generation: usize,
        used: &mut BTreeSet<Ink>,
    ) -> Result<Option<String>> {
        let Some((issued, sizes)) = self.glyphs.get(&ink.code) else {
            return Ok(None);
        };
        if *issued > generation {
            return Ok(None);
        }
        let Some(mask) = sizes.get(&ink.size) else {
            return Ok(None);
        };
        if !self.published.contains_key(&ink) {
            let surface = alpha_cells::paint_cell(
                mask,
                ink.size,
                ink.colors.map(super::rgb),
                ink.gradient,
                ink.shadow.map(super::rgb),
            )?;
            let image = DecodedImage::from_surface(client, surface, Some(true))?.publish(
                host,
                client,
                0x4356_9202,
            )?;
            self.published.insert(ink, image);
        }
        used.insert(ink);
        Ok(Some(self.published[&ink].path.relative().to_owned()))
    }

    pub(super) fn retire(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        used: &BTreeSet<Ink>,
    ) -> Result<()> {
        let unused: Vec<_> = self
            .published
            .keys()
            .filter(|ink| !used.contains(ink))
            .copied()
            .collect();
        for ink in unused {
            host.release_image_resource(client, self.published[&ink].path.relative())?;
            self.published.remove(&ink);
        }
        Ok(())
    }
}

pub(in crate::adapter) fn load(
    sources: &crate::vm::ScriptSources,
    name: &str,
    size: u32,
    context: &crysirror_resource::DecodeContext,
) -> Result<Option<Sheet>> {
    let Some(reader) = sources.open_entry(name)? else {
        return Ok(None);
    };
    let mut header = [0u8; 4];
    if reader.len()? < 4 {
        return Ok(None);
    }
    crysirror_resource::read_exact_at(reader.as_ref(), 0, &mut header)?;
    if &header != b"MSK0" {
        return Ok(None);
    }
    let bytes =
        crysirror_resource::read_all(reader.as_ref(), crysirror_image::MAX_DECODED_BYTES as u64)?;
    let image =
        super::super::visual::prepare_image_with(sources, name, Some(bytes.into()), context)?;
    Ok(Some(Sheet::new(image, size)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_character_registration_owns_exact_sizes_available_at_issue() -> Result<()> {
        let mut fonts = Fonts::default();
        fonts.sheet(Sheet {
            size: 2,
            width: 4,
            mask: (0..8).collect(),
        })?;
        fonts.register(1, 0x8197)?;
        fonts.sheet(Sheet {
            size: 1,
            width: 2,
            mask: vec![99, 88],
        })?;
        fonts.register(0, 0x8197)?;
        fonts.register(1, 0x8141)?;
        let (issued, sizes) = &fonts.glyphs[&0x8197];
        assert_eq!(*issued, 1);
        assert_eq!(sizes[&2], [2, 3, 6, 7]);
        assert!(!sizes.contains_key(&1));
        assert_eq!(fonts.glyphs[&0x8141].1[&1], [88]);
        assert_eq!(fonts.generation(), 2);
        Ok(())
    }
}
