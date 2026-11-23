// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native resource resolution over mounted archives. The original resolver
//! prefers a loose file, else the
//! longest directory prefix naming an `.xfl` archive, one nested `.xfl` per
//! remaining directory. Detection mounts every archive in place, so a native
//! path maps to one case-folded logical key; this module only looks keys up.
use crate::sprite::{Content, Point};
use crysirror_core::{service::HostServices, state::ClientId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One LWG layer (`LG` composite image, reader ).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LwgLayer {
    pub name: String,
    pub x: i32,
    pub y: i32,
    /// Mounted asset of the layer's WCG data; None for empty layer-set markers.
    pub asset: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lwg {
    pub canvas: [u32; 2],
    pub layers: Vec<LwgLayer>,
}

/// Native image search order: WCG, LIM, BMP, TGA, PSD.
pub const IMAGE_EXTENSIONS: [&str; 5] = ["wcg", "lim", "bmp", "tga", "psd"];

/// `.\grpo\0001` → `grpo/0001` (ASCII case folded like lstrcmpiA).
pub fn key(native: &str) -> Option<String> {
    let parts: Vec<&str> = native.split(['\\', '/']).filter(|s| !s.is_empty() && *s != ".").collect();
    if parts.is_empty() || parts.contains(&"..") {
        return None;
    }
    Some(parts.join("/").to_ascii_lowercase())
}
pub fn join(dir: &str, name: &str) -> String {
    format!("{dir}\\{name}")
}

#[derive(Default)]
pub struct Resources {
    /// Case-folded logical key with extension → mounted asset path.
    pub assets: BTreeMap<String, String>,
    pub lwg: BTreeMap<String, Lwg>,
    sizes: BTreeMap<String, [u32; 2]>,
    reported: BTreeSet<String>,
    pub diagnostics: Vec<String>,
}

impl Resources {
    pub fn new(assets: BTreeMap<String, String>, lwg: BTreeMap<String, Lwg>) -> Self {
        Self { assets, lwg, ..Default::default() }
    }
    pub fn diag(&mut self, message: impl Into<String>) {
        let m = message.into();
        if self.reported.insert(m.clone()) {
            log::warn!("RScript: {m}");
            self.diagnostics.push(m);
            if self.diagnostics.len() > 64 {
                self.diagnostics.remove(0);
            }
        }
    }
    /// Mounted asset of a native path that includes its extension.
    pub fn file(&self, native: &str) -> Option<&String> {
        self.assets.get(&key(native)?)
    }
    pub fn size(&mut self, h: &mut dyn HostServices, client: ClientId, asset: &str) -> Option<[u32; 2]> {
        if let Some(s) = self.sizes.get(asset) {
            return Some(*s);
        }
        match h.image_size(client, asset) {
            Ok(s) => {
                self.sizes.insert(asset.to_owned(), s);
                Some(s)
            }
            Err(e) => {
                self.diag(format!("image {asset}: {e}"));
                None
            }
        }
    }
    /// A numbered or named image without extension, in native search order.
    pub fn image(&mut self, h: &mut dyn HostServices, client: ClientId, native: &str) -> Option<Content> {
        let base = key(native)?;
        let asset = IMAGE_EXTENSIONS.iter().find_map(|ext| self.assets.get(&format!("{base}.{ext}")).cloned());
        let Some(asset) = asset else {
            self.diag(format!("missing image {native}"));
            return None;
        };
        if !asset.ends_with(".wcg") && !asset.ends_with(".bmp") {
            self.diag(format!("{native}: native image format of {asset} is not mapped"));
            return None;
        }
        let size = self.size(h, client, &asset)?;
        Some(Content::Image { asset, size })
    }
    pub fn lwg_of(&self, native: &str) -> Option<&Lwg> {
        self.lwg.get(&key(native)?)
    }
    /// A named layer of `<native>.lwg` with its placement.
    pub fn lwg_layer(&mut self, h: &mut dyn HostServices, client: ClientId, native: &str, name: &str) -> Option<(Content, Point)> {
        let layer = self.lwg_of(native)?.layers.iter().find(|l| l.name == name)?.clone();
        let asset = layer.asset?;
        let size = self.size(h, client, &asset)?;
        Some((Content::Image { asset, size }, Point { x: layer.x, y: layer.y }))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_paths_fold_to_mount_keys() {
        assert_eq!(super::key(".\\GRPO\\0001.wcg").as_deref(), Some("grpo/0001.wcg"));
        assert_eq!(super::key(".\\voice\\1\\0002.wav").as_deref(), Some("voice/1/0002.wav"));
        assert!(super::key(".\\..\\x").is_none());
    }
}
