// SPDX-License-Identifier: MPL-2.0
use anyhow::{Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_image::{
    ImageSurface,
    row_strip::{self, StripBlend},
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Default)]
pub(super) struct Strips {
    pub hsv: crate::vm::HsvFilter,
    pub overlay: [Option<VfsPath>; 3],
    pub screen: [Option<VfsPath>; 3],
}
impl Strips {
    pub fn slots(storage: &dyn ClientStorage, names: [String; 3]) -> Result<[Option<VfsPath>; 3]> {
        let mut slots = [None, None, None];
        for (slot, name) in slots.iter_mut().zip(names) {
            let name = name
                .strip_prefix('[')
                .and_then(|n| n.split_once(']'))
                .map_or(name.as_str(), |(_, n)| n);
            if name.is_empty() || name == "*" {
                continue;
            }
            let path = super::presentation::asset("sys", name)?;
            if storage.exists(&path) {
                *slot = Some(path);
            }
        }
        Ok(slots)
    }
    /// A standing-image suffix replaces this plane's strip. Bracketed H,S,V
    /// operands also replace the current standing-image colour parameters.
    pub fn standing(&self, storage: &dyn ClientStorage, spec: &str) -> Result<Self> {
        let mut result = self.clone();
        let strip = if let Some(spec) = spec.strip_prefix('[') {
            let (values, name) = spec
                .split_once(']')
                .ok_or_else(|| anyhow::anyhow!("unterminated standing colour operands"))?;
            let values = values
                .split(',')
                .map(str::parse::<i32>)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let [h, s, v] = values[..] else {
                anyhow::bail!("minori standing image requires three colour operands");
            };
            (result.hsv.st_h, result.hsv.st_s, result.hsv.st_v) = (h, s, v);
            name
        } else {
            spec
        };
        result.overlay[2] = if strip.is_empty() || strip == "*" {
            None
        } else {
            let path = super::presentation::asset("sys", strip)?;
            storage.exists(&path).then_some(path)
        };
        Ok(result)
    }
    fn matrix(&self, slot: usize) -> Option<crysirror_image::integer_color::ColorMatrixFixed> {
        let (h, s, v) = match slot {
            1 => (self.hsv.bg_h, self.hsv.bg_s, self.hsv.bg_v),
            2 => (self.hsv.st_h, self.hsv.st_s, self.hsv.st_v),
            _ => return None,
        };
        if (h, s, v) == (0, 0, 0) {
            return None;
        }
        // The H operand gates this operation in the stage loader. All three
        // verified kernels build their channel tables from S and V alone.
        let q = ((100 + s.clamp(-100, 100)) << 16) / 100;
        let bias = v.clamp(-100, 100) << 8;
        let common = [19595_i32, 38470, 7471].map(|k| (k.wrapping_mul(65536 - q) >> 16) + bias);
        let mut matrix = [common; 3];
        for (i, row) in matrix.iter_mut().enumerate() {
            row[i] += q;
        }
        Some(crysirror_image::integer_color::ColorMatrixFixed {
            matrix,
            offset: [0; 3],
            divisor: 65536,
        })
    }
    pub fn active(&self) -> bool {
        self.hsv != crate::vm::HsvFilter::default()
            || self.overlay.iter().chain(&self.screen).any(Option::is_some)
    }
    pub fn render(
        &self,
        storage: Arc<dyn ClientStorage>,
        layers: Vec<(usize, usize, String)>,
        overrides: BTreeMap<usize, Strips>,
        fixed_background: &[crate::vm::FixedFacePose],
    ) -> Result<Vec<(usize, ImageSurface)>> {
        let decode = |path: &VfsPath| -> Result<ImageSurface> {
            let reader = storage.open_asset(path)?;
            crysirror_image::decode_standard(&crysirror_resource::read_all(
                reader.as_ref(),
                64 << 20,
            )?)
        };
        let mut strips = BTreeMap::new();
        let mut bytes = 0usize;
        for path in std::iter::once(self)
            .chain(overrides.values())
            .flat_map(|s| s.overlay.iter().chain(&s.screen).flatten())
        {
            if !strips.contains_key(path.relative()) {
                let image = decode(path)?;
                bytes += image.rgba.len();
                ensure!(bytes <= 128 << 20, "minori stage strip memory budget");
                strips.insert(path.relative().to_owned(), image);
            }
        }
        let mut rendered = Vec::new();
        for (id, slot, name) in layers {
            let material = overrides.get(&id).unwrap_or(self);
            let matrix = material.matrix(slot);
            let poses = if id == 0 { fixed_background } else { &[] };
            if material.overlay[slot].is_none()
                && material.screen[slot].is_none()
                && matrix.is_none()
                && poses.is_empty()
            {
                continue;
            }
            let (tone, path) = crate::presentation::tone::path(&name)
                .map_or((None, name), |(tone, path)| (Some(tone), path));
            let mut image = decode(&VfsPath::new(Mount::Asset, &path)?)?;
            if let Some(tone) = tone { image = tone.map(&image)?; }
            if !poses.is_empty() {
                crate::presentation::apply_fixed_poses(storage.as_ref(), &path, poses, tone, &mut image)?;
            }
            bytes += image.rgba.len();
            ensure!(bytes <= 128 << 20, "minori filtered stage memory budget");
            if let Some(matrix) = matrix {
                let mut colored = matrix.apply(&image, &|| Ok(()))?;
                for (dst, src) in colored
                    .rgba
                    .chunks_exact_mut(4)
                    .zip(image.rgba.chunks_exact(4))
                {
                    if src[3] == 0 {
                        dst.copy_from_slice(src);
                    }
                }
                image = colored;
            }
            for (path, blend) in [
                (&material.overlay[slot], StripBlend::Overlay),
                (&material.screen[slot], StripBlend::Screen),
            ] {
                if let Some(path) = path {
                    row_strip::apply(&mut image, &strips[path.relative()], blend)?;
                }
            }
            rendered.push((id, image));
        }
        Ok(rendered)
    }
}
