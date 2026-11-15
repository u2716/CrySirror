// SPDX-License-Identifier: MPL-2.0
//! Cover art for game libraries (console launcher, desktop and web pickers).
//!
//! Lookup order, following xmoezzz/siglus_rs `crates/game_launcher/src/cover.rs`
//! at bef39ea (MPL-2.0, same owner):
//! 1. conventional cover files in the game folder (`cover.png`, ...);
//! 2. an `.ico` file in the game folder (names with "icon"/"game" first);
//! 3. the largest icon group of the game's Windows executable (PE
//!    `RT_GROUP_ICON`/`RT_ICON`, read as data through crysirror-resource;
//!    nothing is loaded or executed). Installer/config/update tools are
//!    skipped, and renamed game executables rank before generic engine names.
//!
//! Icons are small, so callers present them on a generated card
//! ([`CoverKind::Icon`]); pictures are shown full-bleed ([`CoverKind::Image`]).
use image::RgbaImage;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverKind {
    /// A picture meant to be shown full-bleed.
    Image,
    /// A game icon, to be shown on a generated card.
    Icon,
}

#[derive(Clone, Debug)]
pub struct Cover {
    pub image: RgbaImage,
    pub kind: CoverKind,
    /// The file it came from (diagnostics).
    pub source: PathBuf,
}

const COVER_FILES: &[&str] = &["cover.png", "cover.jpg", "cover.jpeg", "thumbnail.png", "icon.png"];
/// Read limits: covers and icons are small; executables are read as data.
const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EXE_BYTES: u64 = crysirror_resource::pe::MAX_PE_BYTES as u64;

/// The cover of the game in `dir`, or `None` (callers then draw a text card).
pub fn resolve(dir: &Path) -> Option<Cover> {
    let files = list_files(dir);
    for wanted in COVER_FILES {
        let Some(path) = files.iter().find(|p| file_name_lower(p) == *wanted) else { continue };
        if let Some(image) = read(path, MAX_IMAGE_BYTES).and_then(|b| image::load_from_memory(&b).ok()) {
            return Some(Cover { image: image.to_rgba8(), kind: CoverKind::Image, source: path.clone() });
        }
    }
    let mut icons: Vec<&PathBuf> = files.iter().filter(|p| has_ext(p, "ico")).collect();
    icons.sort_by_key(|p| icon_rank(p));
    for path in icons {
        if let Some(icon) = read(path, MAX_IMAGE_BYTES).and_then(|b| decode_ico(&b)) {
            return Some(Cover { image: icon, kind: CoverKind::Icon, source: path.clone() });
        }
    }
    let mut exes: Vec<&PathBuf> = files.iter().filter(|p| has_ext(p, "exe") && !is_helper_exe(p)).collect();
    exes.sort_by_key(|p| exe_rank(p));
    for path in exes {
        if let Some(icon) = read(path, MAX_EXE_BYTES).and_then(|b| exe_icon_ico(&b)).and_then(|ico| decode_ico(&ico)) {
            return Some(Cover { image: icon, kind: CoverKind::Icon, source: path.clone() });
        }
    }
    None
}

fn read(path: &Path, limit: u64) -> Option<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

fn list_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect())
        .unwrap_or_default();
    files.sort();
    files
}

fn file_name_lower(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

fn has_ext(path: &Path, ext: &str) -> bool {
    path.extension().is_some_and(|v| v.to_string_lossy().eq_ignore_ascii_case(ext))
}

fn icon_rank(path: &Path) -> u8 {
    let name = file_name_lower(path);
    u8::from(!(name.contains("icon") || name.contains("game")))
}

fn is_helper_exe(path: &Path) -> bool {
    let name = file_name_lower(path);
    ["unins", "setup", "install", "config", "update", "patch", "dxsetup", "vcredist", "設定"]
        .iter()
        .any(|word| name.contains(word))
}

/// Titles often rename the engine executable after the game; prefer those
/// over generic engine names.
fn exe_rank(path: &Path) -> u8 {
    let name = file_name_lower(path);
    let generic = ["siglusengine", "reallive", "kinetic", "avg32", "avg2000", "krkr", "tvpwin32", "bgi", "cs2"];
    u8::from(generic.iter().any(|g| name.starts_with(g)))
}

/// The largest image of an `.ico` file.
pub fn decode_ico(bytes: &[u8]) -> Option<RgbaImage> {
    if bytes.len() < 6 || bytes[..4] != [0, 0, 1, 0] {
        return None;
    }
    image::load_from_memory_with_format(bytes, image::ImageFormat::Ico)
        .ok()
        .map(|i| i.to_rgba8())
        .filter(|i| i.width() > 0 && i.height() > 0)
}

/// Rebuild an `.ico` file from the largest icon group of a PE executable.
pub fn exe_icon_ico(pe: &[u8]) -> Option<Vec<u8>> {
    use crysirror_resource::pe::{PeData, PeResourceName};
    const RT_ICON: u32 = 3;
    const RT_GROUP_ICON: u32 = 14;
    let data = PeData::parse(pe).ok()?;
    let ok = || Ok(());
    let groups = data.raw_resources(&PeResourceName::Id(RT_GROUP_ICON), &ok).ok()?;
    let icons = data.raw_resources(&PeResourceName::Id(RT_ICON), &ok).ok()?;
    let icon = |id: u32| icons.iter().find(|r| r.name == PeResourceName::Id(id)).map(|r| r.bytes.as_slice());
    let mut best: Option<(u64, Vec<u8>)> = None;
    for group in groups.iter().map(|g| g.bytes.as_slice()) {
        if group.len() < 6 {
            continue;
        }
        let count = usize::from(u16::from_le_bytes([group[4], group[5]]));
        let mut images = Vec::new();
        let mut area = 0u64;
        for index in 0..count {
            let at = 6 + index * 14;
            let Some(entry) = group.get(at..at + 14) else { break };
            let Some(bytes) = icon(u32::from(u16::from_le_bytes([entry[12], entry[13]]))) else { continue };
            let side = |v: u8| if v == 0 { 256 } else { u64::from(v) };
            area = area.max(side(entry[0]) * side(entry[1]));
            images.push((&entry[..8], bytes));
        }
        if images.is_empty() || best.as_ref().is_some_and(|(a, _)| *a >= area) {
            continue;
        }
        let mut ico = vec![0, 0, 1, 0];
        ico.extend((images.len() as u16).to_le_bytes());
        let mut offset = 6 + images.len() * 16;
        for (header, bytes) in &images {
            ico.extend(*header);
            ico.extend((bytes.len() as u32).to_le_bytes());
            ico.extend((offset as u32).to_le_bytes());
            offset += bytes.len();
        }
        for (_, bytes) in &images {
            ico.extend(*bytes);
        }
        best = Some((area, ico));
    }
    best.map(|(_, ico)| ico)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ico(size: u8) -> Vec<u8> {
        let image = RgbaImage::from_pixel(u32::from(size), u32::from(size), image::Rgba([200, 40, 40, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image).write_to(&mut out, image::ImageFormat::Ico).unwrap();
        out.into_inner()
    }
    #[test]
    fn conventional_file_then_ico_then_none() {
        let dir = std::env::temp_dir().join(format!("crysirror-cover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(resolve(&dir).is_none());
        std::fs::write(dir.join("Game.ico"), ico(32)).unwrap();
        let cover = resolve(&dir).unwrap();
        assert_eq!((cover.kind, cover.image.width()), (CoverKind::Icon, 32));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(RgbaImage::new(64, 36)).write_to(&mut png, image::ImageFormat::Png).unwrap();
        std::fs::write(dir.join("Cover.PNG"), png.into_inner()).unwrap();
        assert_eq!(resolve(&dir).unwrap().kind, CoverKind::Image);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
