// SPDX-License-Identifier: MPL-2.0
//! Adobe Photoshop documents (PSD, version 1, and large-document PSB,
//! version 2): header, colour table, the image resources a layered-image
//! consumer needs (resolution, grid and guides, slices, layer comps), layer
//! records with their masks, additional layer information and channel image
//! data, and the merged composite.
//!
//! Written from the published Photoshop file-format specification. Field
//! naming and the resource/descriptor key choices follow the MIT-licensed
//! wamsoft `psdparse` (a85a9ee) so script-facing consumers such as the
//! KiriKiri `psd.dll` class read the same facts.
//!
//! Channel data is decoded on demand: [`Document::layer_rgba`] composes one
//! layer (image, image with its user mask folded into alpha, or the mask
//! alone) and [`Document::merged_rgba`] the stored composite. Pixels are
//! straight-alpha RGBA8. 16-bit samples keep the high byte and 32-bit samples
//! are clamped floats scaled to 0..255. Bitmap, grayscale, indexed, RGB and
//! CMYK documents produce pixels; multichannel, duotone and Lab documents
//! produce none, so their layer images stay transparent.
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;

pub const MODE_BITMAP: u16 = 0;
pub const MODE_GRAYSCALE: u16 = 1;
pub const MODE_INDEXED: u16 = 2;
pub const MODE_RGB: u16 = 3;
pub const MODE_CMYK: u16 = 4;

/// Bound on one decoded plane and on the whole document's pixel count.
const MAX_SIDE: u32 = 300_000;
const MAX_PIXELS: u64 = 1 << 28;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerType {
    Normal = 0,
    /// Section divider closing a folder (internal structure).
    Hidden = 1,
    Folder = 2,
    Adjust = 3,
    Fill = 4,
}

/// Blend modes in the order of the KiriKiri `blend_mode_*` constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Invalid = -1,
    Normal = 0,
    Dissolve,
    Darken,
    Multiply,
    ColorBurn,
    LinearBurn,
    Lighten,
    Screen,
    ColorDodge,
    LinearDodge,
    Overlay,
    SoftLight,
    HardLight,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    PassThrough,
    DarkerColor,
    LighterColor,
    Subtract,
    Divide,
}
impl BlendMode {
    pub fn from_key(key: [u8; 4]) -> Self {
        use BlendMode::*;
        match &key {
            b"norm" => Normal,
            b"diss" => Dissolve,
            b"dark" => Darken,
            b"mul " => Multiply,
            b"idiv" => ColorBurn,
            b"lbrn" => LinearBurn,
            b"lite" => Lighten,
            b"scrn" => Screen,
            b"div " => ColorDodge,
            b"lddg" => LinearDodge,
            b"over" => Overlay,
            b"sLit" => SoftLight,
            b"hLit" => HardLight,
            b"vLit" => VividLight,
            b"lLit" => LinearLight,
            b"pLit" => PinLight,
            b"hMix" => HardMix,
            b"diff" => Difference,
            b"smud" => Exclusion,
            b"hue " => Hue,
            b"sat " => Saturation,
            b"colr" => Color,
            b"lum " => Luminosity,
            b"pass" => PassThrough,
            b"dkCl" => DarkerColor,
            b"lgCl" | b"ltCl" => LighterColor,
            b"fsub" => Subtract,
            b"fdiv" => Divide,
            _ => Invalid,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageMode {
    /// Colour channels with the transparency channel.
    Image,
    /// Image alpha multiplied by the user mask (unless the mask is disabled).
    MaskedImage,
    /// The user mask alone as opaque grayscale over the mask rectangle.
    Mask,
}

#[derive(Clone, Debug, Default)]
pub struct Rect {
    pub top: i32,
    pub left: i32,
    pub bottom: i32,
    pub right: i32,
}
impl Rect {
    pub fn width(&self) -> i32 {
        (self.right - self.left).max(0)
    }
    pub fn height(&self) -> i32 {
        (self.bottom - self.top).max(0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct LayerMask {
    pub present: bool,
    pub rect: Rect,
    pub default_color: u8,
    pub flags: u8,
    pub user_density: Option<u8>,
    pub user_feather: Option<f64>,
    pub vector_density: Option<u8>,
    pub vector_feather: Option<f64>,
    /// "Real" user mask rectangle (channel -3), when recorded.
    pub real_rect: Option<Rect>,
}
impl LayerMask {
    pub fn disabled(&self) -> bool {
        self.flags & 2 != 0
    }
    pub fn has_parameters(&self) -> bool {
        self.user_density.is_some()
            || self.user_feather.is_some()
            || self.vector_density.is_some()
            || self.vector_feather.is_some()
    }
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub id: i16,
    offset: usize,
    length: usize,
}

#[derive(Clone, Debug, Default)]
pub struct LayerCompState {
    pub id: i32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub rect: Rect,
    pub channels: Vec<Channel>,
    pub blend_key: [u8; 4],
    pub blend_mode: BlendMode,
    pub opacity: u8,
    pub fill_opacity: u8,
    pub clipping: u8,
    pub flags: u8,
    pub mask: LayerMask,
    pub name: String,
    pub unicode_name: Option<String>,
    pub layer_type: LayerType,
    pub layer_id: i32,
    /// Index of the enclosing folder layer.
    pub parent: Option<usize>,
    pub comps: BTreeMap<i32, LayerCompState>,
}
impl Layer {
    pub fn visible(&self) -> bool {
        self.flags & 2 == 0
    }
    pub fn transparency_protected(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn obsolete(&self) -> bool {
        self.flags & 4 != 0
    }
    pub fn pixel_data_irrelevant(&self) -> bool {
        self.flags & 8 != 0 && self.flags & 16 != 0
    }
    /// The Unicode name when recorded, else the Pascal name.
    pub fn display_name(&self) -> &str {
        self.unicode_name.as_deref().unwrap_or(&self.name)
    }
    pub fn has_mask_channel(&self) -> bool {
        self.channels.iter().any(|c| c.id <= -2)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Guide {
    pub location: i32,
    /// 0 vertical, 1 horizontal.
    pub direction: u8,
}
#[derive(Clone, Debug, Default)]
pub struct GridGuides {
    pub horizontal_grid: i32,
    pub vertical_grid: i32,
    pub guides: Vec<Guide>,
}

#[derive(Clone, Debug, Default)]
pub struct Slice {
    pub id: i32,
    pub group_id: i32,
    pub origin: i32,
    pub associated_layer_id: i32,
    pub name: String,
    pub kind: i32,
    pub rect: Rect,
    pub url: String,
    pub target: String,
    pub message: String,
    pub alt_tag: String,
    pub cell_text_is_html: bool,
    pub cell_text: String,
    pub horizontal_alignment: i32,
    pub vertical_alignment: i32,
    /// ARGB.
    pub color: u32,
}
#[derive(Clone, Debug, Default)]
pub struct Slices {
    pub bounds: Rect,
    pub group_name: String,
    pub slices: Vec<Slice>,
}

#[derive(Clone, Debug, Default)]
pub struct LayerComp {
    pub id: i32,
    pub record_visibility: bool,
    pub record_position: bool,
    pub record_appearance: bool,
    pub name: String,
    pub comment: String,
}

pub struct Document {
    bytes: std::sync::Arc<[u8]>,
    pub large: bool,
    pub channels: u16,
    pub width: u32,
    pub height: u32,
    pub depth: u16,
    pub color_mode: u16,
    pub palette: Vec<[u8; 3]>,
    pub resolution: [f64; 2],
    pub grid_guides: Option<GridGuides>,
    pub slices: Option<Slices>,
    pub layer_comps: Vec<LayerComp>,
    pub last_applied_comp: i32,
    pub layers: Vec<Layer>,
    /// Absolute offset of the merged image section, when present.
    merged: Option<usize>,
    pub merged_alpha: bool,
}

struct Cursor<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Cursor<'a> {
    fn new(b: &'a [u8], p: usize) -> Self {
        Self { b, p }
    }
    fn rest(&self) -> usize {
        self.b.len().saturating_sub(self.p)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        ensure!(n <= self.rest(), "PSD truncated at byte {}", self.p);
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into()?))
    }
    fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into()?))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into()?))
    }
    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.u64()?))
    }
    fn key(&mut self) -> Result<[u8; 4]> {
        Ok(self.take(4)?.try_into()?)
    }
    /// u32 length, or u64 for the PSB long forms.
    fn len(&mut self, long: bool) -> Result<usize> {
        let n = if long { self.u64()? } else { u64::from(self.u32()?) };
        usize::try_from(n).context("PSD length")
    }
    fn unicode(&mut self) -> Result<String> {
        let n = self.u32()? as usize;
        ensure!(n <= self.rest() / 2, "PSD Unicode string length");
        let units: Vec<u16> = (0..n).map(|_| self.u16()).collect::<Result<_>>()?;
        let mut s = String::from_utf16_lossy(&units);
        while s.ends_with('\0') {
            s.pop();
        }
        Ok(s)
    }
}

fn shift_jis(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes).0.into_owned()
}

impl Document {
    pub fn parse(bytes: impl Into<std::sync::Arc<[u8]>>) -> Result<Self> {
        let bytes: std::sync::Arc<[u8]> = bytes.into();
        let b = &bytes[..];
        let mut c = Cursor::new(b, 0);
        ensure!(c.take(4)? == b"8BPS", "not a Photoshop document");
        let version = c.u16()?;
        ensure!(version == 1 || version == 2, "Photoshop version {version}");
        let large = version == 2;
        c.skip(6)?;
        let channels = c.u16()?;
        let height = c.u32()?;
        let width = c.u32()?;
        let depth = c.u16()?;
        let color_mode = c.u16()?;
        ensure!((1..=56).contains(&channels), "PSD channel count {channels}");
        ensure!(matches!(depth, 1 | 8 | 16 | 32), "PSD depth {depth}");
        ensure!(width <= MAX_SIDE && height <= MAX_SIDE, "PSD dimensions");
        let palette_len = c.u32()? as usize;
        let palette_bytes = c.take(palette_len)?;
        let palette = if color_mode == MODE_INDEXED && palette_bytes.len() >= 768 {
            (0..256)
                .map(|i| [palette_bytes[i], palette_bytes[256 + i], palette_bytes[512 + i]])
                .collect()
        } else {
            Vec::new()
        };
        let mut doc = Self {
            bytes: bytes.clone(),
            large,
            channels,
            width,
            height,
            depth,
            color_mode,
            palette,
            resolution: [72.; 2],
            grid_guides: None,
            slices: None,
            layer_comps: Vec::new(),
            last_applied_comp: -1,
            layers: Vec::new(),
            merged: None,
            merged_alpha: false,
        };
        let resources_len = c.u32()? as usize;
        let resources = c.take(resources_len)?;
        doc.parse_resources(resources);
        let layer_len = c.len(large)?;
        let layer_start = c.p;
        ensure!(layer_len <= c.rest(), "PSD layer section length");
        if layer_len > 0 {
            doc.parse_layer_section(&bytes, layer_start, layer_start + layer_len)?;
        }
        c.p = layer_start + layer_len;
        if c.rest() >= 2 {
            doc.merged = Some(c.p);
            doc.merged_alpha = doc.layers.is_empty() && channels > 3 || channels > 4;
        }
        Ok(doc)
    }

    fn parse_resources(&mut self, data: &[u8]) {
        let mut c = Cursor::new(data, 0);
        while c.rest() >= 12 {
            let Ok(sig) = c.key() else { break };
            if &sig != b"8BIM" && &sig != b"MeSa" && &sig != b"AgHg" && &sig != b"PHUT" && &sig != b"DCSR" {
                break;
            }
            let Ok(id) = c.u16() else { break };
            let Ok(name_len) = c.u8() else { break };
            let pad = (usize::from(name_len) + 1) % 2;
            if c.skip(usize::from(name_len) + pad).is_err() {
                break;
            }
            let Ok(len) = c.u32() else { break };
            let Ok(body) = c.take(len as usize) else { break };
            if len % 2 == 1 {
                let _ = c.skip(1);
            }
            // Resources are independent: a malformed one is skipped.
            let _ = self.resource(id, body);
        }
    }

    fn resource(&mut self, id: u16, body: &[u8]) -> Result<()> {
        let mut c = Cursor::new(body, 0);
        match id {
            1005 => {
                let h = c.u32()?;
                c.skip(4)?;
                let v = c.u32()?;
                self.resolution = [f64::from(h) / 65536., f64::from(v) / 65536.];
            }
            1032 => {
                let _version = c.u32()?;
                let horizontal_grid = c.i32()?;
                let vertical_grid = c.i32()?;
                let count = c.u32()?;
                let mut guides = Vec::new();
                for _ in 0..count {
                    if c.rest() < 5 {
                        break;
                    }
                    guides.push(Guide { location: c.i32()?, direction: c.u8()? });
                }
                self.grid_guides = Some(GridGuides { horizontal_grid, vertical_grid, guides });
            }
            1050 => self.slices = Some(parse_slices(&mut c)?),
            1065 => {
                ensure!(c.u32()? == 16, "layer comp descriptor version");
                let d = descriptor(&mut c)?;
                self.last_applied_comp = d.get("lastAppliedComp").and_then(Item::int).unwrap_or(-1);
                if let Some(Item::List(list)) = d.get("list") {
                    for comp in list {
                        let Item::Object(comp) = comp else { continue };
                        let Some(id) = comp.get("compID").and_then(Item::int) else { continue };
                        let captured = comp.get("capturedInfo").and_then(Item::int).unwrap_or(0);
                        self.layer_comps.push(LayerComp {
                            id,
                            record_visibility: captured & 1 != 0,
                            record_position: captured & 2 != 0,
                            record_appearance: captured & 4 != 0,
                            name: comp.get("Nm  ").and_then(Item::text).unwrap_or_default(),
                            comment: comp.get("comment").and_then(Item::text).unwrap_or_default(),
                        });
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn parse_layer_section(&mut self, bytes: &std::sync::Arc<[u8]>, start: usize, end: usize) -> Result<()> {
        let b = &bytes[..end];
        let mut c = Cursor::new(b, start);
        let info_len = c.len(self.large)?;
        let info_start = c.p;
        ensure!(info_len <= c.rest(), "PSD layer info length");
        if info_len > 0 {
            self.parse_layer_info(b, info_start, info_start + info_len)?;
        }
        c.p = info_start + info_len;
        // Global layer mask info, then additional blocks (16/32-bit layers
        // live in Lr16/Lr32 here).
        if c.rest() >= 4 {
            let global = c.u32()? as usize;
            if c.skip(global).is_err() {
                return Ok(());
            }
        }
        while c.rest() >= 12 {
            let sig = c.key()?;
            if &sig != b"8BIM" && &sig != b"8B64" {
                break;
            }
            let key = c.key()?;
            let long = self.large && long_key(&key);
            let len = c.len(long)?;
            let body = c.p;
            if len > c.rest() {
                break;
            }
            if (&key == b"Lr16" || &key == b"Lr32" || &key == b"Layr") && self.layers.is_empty() {
                self.parse_layer_info(b, body, body + len)?;
            }
            c.p = body + len;
            while c.p % 4 != 0 && c.rest() > 0 && c.p - body < len + 3 {
                // Blocks are padded to four bytes.
                if c.b[c.p] != 0 {
                    break;
                }
                c.p += 1;
            }
        }
        Ok(())
    }

    fn parse_layer_info(&mut self, b: &[u8], start: usize, end: usize) -> Result<()> {
        let b = &b[..end];
        let mut c = Cursor::new(b, start);
        let count = c.i16()?.unsigned_abs() as usize;
        let mut layers = Vec::with_capacity(count);
        for _ in 0..count {
            layers.push(self.layer_record(&mut c)?);
        }
        // Channel image data follows the records in record/channel order.
        for layer in &mut layers {
            for ch in &mut layer.channels {
                ch.offset = c.p;
                ensure!(ch.length <= c.rest(), "PSD channel data length");
                c.p += ch.length;
            }
        }
        // Folder linkage: from the top, a folder opens and its divider closes.
        let mut stack: Vec<usize> = Vec::new();
        for i in (0..layers.len()).rev() {
            layers[i].parent = stack.last().copied();
            match layers[i].layer_type {
                LayerType::Folder => stack.push(i),
                LayerType::Hidden => {
                    stack.pop();
                    layers[i].parent = stack.last().copied();
                }
                _ => {}
            }
        }
        self.layers = layers;
        Ok(())
    }

    fn layer_record(&self, c: &mut Cursor) -> Result<Layer> {
        let rect = Rect { top: c.i32()?, left: c.i32()?, bottom: c.i32()?, right: c.i32()? };
        let n = c.u16()?;
        ensure!(n <= 56, "PSD layer channel count");
        let mut channels = Vec::new();
        for _ in 0..n {
            let id = c.i16()?;
            let length = c.len(self.large)?;
            channels.push(Channel { id, offset: 0, length });
        }
        ensure!(c.take(4)? == b"8BIM", "PSD layer blend signature");
        let blend_key = c.key()?;
        let opacity = c.u8()?;
        let clipping = c.u8()?;
        let flags = c.u8()?;
        c.skip(1)?;
        let extra_len = c.u32()? as usize;
        let extra_end = c.p + extra_len;
        ensure!(extra_len <= c.rest(), "PSD layer extra length");
        let mut layer = Layer {
            rect,
            channels,
            blend_key,
            blend_mode: BlendMode::from_key(blend_key),
            opacity,
            fill_opacity: 255,
            clipping,
            flags,
            mask: LayerMask::default(),
            name: String::new(),
            unicode_name: None,
            layer_type: LayerType::Normal,
            layer_id: -1,
            parent: None,
            comps: BTreeMap::new(),
        };
        // Layer mask / adjustment layer data.
        let mask_len = c.u32()? as usize;
        let mask_end = c.p + mask_len;
        ensure!(mask_end <= extra_end, "PSD layer mask length");
        if mask_len >= 20 {
            let m = &mut layer.mask;
            m.present = true;
            m.rect = Rect { top: c.i32()?, left: c.i32()?, bottom: c.i32()?, right: c.i32()? };
            m.default_color = c.u8()?;
            m.flags = c.u8()?;
            if m.flags & 16 != 0 && c.p + 1 <= mask_end {
                let params = c.u8()?;
                if params & 1 != 0 {
                    m.user_density = Some(c.u8()?);
                }
                if params & 2 != 0 {
                    m.user_feather = Some(c.f64()?);
                }
                if params & 4 != 0 {
                    m.vector_density = Some(c.u8()?);
                }
                if params & 8 != 0 {
                    m.vector_feather = Some(c.f64()?);
                }
            }
            if mask_end - c.p >= 18 {
                let _real_flags = c.u8()?;
                let _real_background = c.u8()?;
                m.real_rect = Some(Rect { top: c.i32()?, left: c.i32()?, bottom: c.i32()?, right: c.i32()? });
            }
        }
        c.p = mask_end;
        let ranges = c.u32()? as usize;
        c.skip(ranges)?;
        let name_len = c.u8()? as usize;
        let name = c.take(name_len)?;
        layer.name = shift_jis(name);
        let padded = (name_len + 1).div_ceil(4) * 4;
        c.skip(padded - 1 - name_len)?;
        // Additional layer information.
        while c.p + 12 <= extra_end {
            let sig = c.key()?;
            if &sig != b"8BIM" && &sig != b"8B64" {
                break;
            }
            let key = c.key()?;
            let len = c.len(self.large && long_key(&key))?;
            let body_start = c.p;
            ensure!(body_start + len <= extra_end, "PSD additional layer info length");
            let body = &c.b[body_start..body_start + len];
            let _ = layer_info(&mut layer, &key, body);
            c.p = body_start + len;
        }
        c.p = extra_end;
        Ok(layer)
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Compose one layer. Returns `(rect, rgba)`; an empty rectangle yields
    /// no pixels.
    pub fn layer_rgba(&self, index: usize, mode: ImageMode) -> Result<(Rect, Vec<u8>)> {
        let layer = self.layers.get(index).context("no such layer")?;
        if mode == ImageMode::Mask {
            let rect = layer.mask.rect.clone();
            let (w, h) = (rect.width() as u32, rect.height() as u32);
            if w == 0 || h == 0 {
                return Ok((rect, Vec::new()));
            }
            check_size(w, h)?;
            let plane = match layer.channels.iter().find(|ch| ch.id == -2) {
                Some(ch) => self.channel_plane(ch, w, h)?,
                None => vec![layer.mask.default_color; (w * h) as usize],
            };
            let mut out = vec![0; (w * h * 4) as usize];
            for (px, v) in out.chunks_exact_mut(4).zip(plane) {
                px.copy_from_slice(&[v, v, v, 255]);
            }
            return Ok((rect, out));
        }
        let rect = layer.rect.clone();
        let (w, h) = (rect.width() as u32, rect.height() as u32);
        if w == 0 || h == 0 {
            return Ok((rect, Vec::new()));
        }
        check_size(w, h)?;
        let mut planes: BTreeMap<i16, Vec<u8>> = BTreeMap::new();
        for ch in &layer.channels {
            if ch.id >= -1 {
                planes.insert(ch.id, self.channel_plane(ch, w, h)?);
            }
        }
        let mut out = self.compose(&planes, w, h, planes.get(&-1));
        if mode == ImageMode::MaskedImage && layer.mask.present && !layer.mask.disabled() {
            if let Some(ch) = layer.channels.iter().find(|ch| ch.id == -2) {
                let m = &layer.mask.rect;
                let (mw, mh) = (m.width() as u32, m.height() as u32);
                let plane = if mw > 0 && mh > 0 { self.channel_plane(ch, mw, mh)? } else { Vec::new() };
                for y in 0..h as i32 {
                    for x in 0..w as i32 {
                        let (dx, dy) = (rect.left + x - m.left, rect.top + y - m.top);
                        let v = if dx >= 0 && dy >= 0 && dx < mw as i32 && dy < mh as i32 {
                            plane[(dy as u32 * mw + dx as u32) as usize]
                        } else {
                            layer.mask.default_color
                        };
                        let a = &mut out[((y as u32 * w + x as u32) * 4 + 3) as usize];
                        *a = ((u16::from(*a) * u16::from(v) + 127) / 255) as u8;
                    }
                }
            }
        }
        Ok((rect, out))
    }

    /// The stored merged composite, if the file has one.
    pub fn merged_rgba(&self) -> Result<Option<Vec<u8>>> {
        let Some(start) = self.merged else { return Ok(None) };
        let (w, h) = (self.width, self.height);
        if w == 0 || h == 0 {
            return Ok(None);
        }
        check_size(w, h)?;
        let b = &self.bytes[..];
        let mut c = Cursor::new(b, start);
        let compression = c.u16()?;
        let channels = usize::from(self.channels);
        let row = row_bytes(w, self.depth);
        let plane_len = row * h as usize;
        let mut planes: BTreeMap<i16, Vec<u8>> = BTreeMap::new();
        match compression {
            0 => {
                for i in 0..channels {
                    let raw = c.take(plane_len)?;
                    planes.insert(i as i16, self.to_bytes(raw, w, h)?);
                }
            }
            1 => {
                let counts: Vec<usize> = (0..channels * h as usize)
                    .map(|_| if self.large { c.u32().map(|n| n as usize) } else { c.u16().map(usize::from) })
                    .collect::<Result<_>>()?;
                for i in 0..channels {
                    let mut raw = Vec::with_capacity(plane_len);
                    for y in 0..h as usize {
                        let packed = c.take(counts[i * h as usize + y])?;
                        packbits(packed, row, &mut raw)?;
                    }
                    planes.insert(i as i16, self.to_bytes(&raw, w, h)?);
                }
            }
            2 | 3 => {
                let total = plane_len * channels;
                let mut raw = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&b[c.p..], total)
                    .map_err(|e| anyhow::anyhow!("PSD merged zlib: {e:?}"))?;
                raw.resize(total, 0);
                for i in 0..channels {
                    let plane = &mut raw[i * plane_len..(i + 1) * plane_len];
                    if compression == 3 {
                        unpredict(plane, w, h, self.depth);
                    }
                    planes.insert(i as i16, self.to_bytes(plane, w, h)?);
                }
            }
            other => bail!("PSD merged compression {other}"),
        }
        let colour = match self.color_mode {
            MODE_RGB => 3,
            MODE_CMYK => 4,
            _ => 1,
        };
        let alpha = (channels > colour).then(|| planes.get(&(colour as i16)).cloned()).flatten();
        Ok(Some(self.compose(&planes, w, h, alpha.as_ref())))
    }

    fn compose(&self, planes: &BTreeMap<i16, Vec<u8>>, w: u32, h: u32, alpha: Option<&Vec<u8>>) -> Vec<u8> {
        let n = (w * h) as usize;
        let mut out = vec![0u8; n * 4];
        let plane = |id: i16| planes.get(&id);
        let full = vec![255u8; n];
        let alpha = alpha.unwrap_or(&full);
        match self.color_mode {
            MODE_RGB => {
                let (Some(r), Some(g), Some(b)) = (plane(0), plane(1), plane(2)) else { return out };
                for i in 0..n {
                    out[i * 4..i * 4 + 4].copy_from_slice(&[r[i], g[i], b[i], alpha[i]]);
                }
            }
            MODE_GRAYSCALE | MODE_BITMAP => {
                let Some(k) = plane(0) else { return out };
                for i in 0..n {
                    out[i * 4..i * 4 + 4].copy_from_slice(&[k[i], k[i], k[i], alpha[i]]);
                }
            }
            MODE_INDEXED => {
                let Some(k) = plane(0) else { return out };
                for i in 0..n {
                    let [r, g, b] = self.palette.get(usize::from(k[i])).copied().unwrap_or([0; 3]);
                    out[i * 4..i * 4 + 4].copy_from_slice(&[r, g, b, alpha[i]]);
                }
            }
            MODE_CMYK => {
                let (Some(cc), Some(mm), Some(yy), Some(kk)) = (plane(0), plane(1), plane(2), plane(3)) else { return out };
                // Photoshop stores CMYK inverted (255 = no ink).
                let mul = |a: u8, b: u8| ((u16::from(a) * u16::from(b) + 127) / 255) as u8;
                for i in 0..n {
                    out[i * 4..i * 4 + 4].copy_from_slice(&[mul(cc[i], kk[i]), mul(mm[i], kk[i]), mul(yy[i], kk[i]), alpha[i]]);
                }
            }
            _ => {}
        }
        out
    }

    /// Decode one channel of a layer into an 8-bit plane of `w * h`.
    fn channel_plane(&self, ch: &Channel, w: u32, h: u32) -> Result<Vec<u8>> {
        let n = (w * h) as usize;
        if ch.length < 2 {
            return Ok(vec![0; n]);
        }
        let data = &self.bytes[ch.offset..ch.offset + ch.length];
        let mut c = Cursor::new(data, 0);
        let compression = c.u16()?;
        let row = row_bytes(w, self.depth);
        let raw = match compression {
            0 => {
                let mut raw = c.take(c.rest().min(row * h as usize))?.to_vec();
                raw.resize(row * h as usize, 0);
                raw
            }
            1 => {
                let counts: Vec<usize> = (0..h)
                    .map(|_| if self.large { c.u32().map(|n| n as usize) } else { c.u16().map(usize::from) })
                    .collect::<Result<_>>()?;
                let mut raw = Vec::with_capacity(row * h as usize);
                for count in counts {
                    let packed = c.take(count.min(c.rest()))?;
                    packbits(packed, row, &mut raw)?;
                }
                raw
            }
            2 | 3 => {
                let total = row * h as usize;
                let mut raw = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&data[2..], total)
                    .map_err(|e| anyhow::anyhow!("PSD channel zlib: {e:?}"))?;
                raw.resize(total, 0);
                if compression == 3 {
                    unpredict(&mut raw, w, h, self.depth);
                }
                raw
            }
            // Unknown codecs read as opaque.
            _ => return Ok(vec![255; n]),
        };
        self.to_bytes(&raw, w, h)
    }

    /// Reduce one decoded plane of `depth`-bit samples to bytes.
    fn to_bytes(&self, raw: &[u8], w: u32, h: u32) -> Result<Vec<u8>> {
        let n = (w * h) as usize;
        Ok(match self.depth {
            8 => raw[..n.min(raw.len())].iter().copied().chain(std::iter::repeat(0)).take(n).collect(),
            16 => (0..n).map(|i| raw.get(i * 2).copied().unwrap_or(0)).collect(),
            32 => (0..n)
                .map(|i| {
                    let b = raw.get(i * 4..i * 4 + 4).map_or([0; 4], |s| s.try_into().unwrap());
                    (f32::from_be_bytes(b).clamp(0., 1.) * 255. + 0.5) as u8
                })
                .collect(),
            1 => {
                let row = row_bytes(w, 1);
                let mut out = Vec::with_capacity(n);
                for y in 0..h as usize {
                    for x in 0..w as usize {
                        let byte = raw.get(y * row + x / 8).copied().unwrap_or(0);
                        // Set bits are black.
                        out.push(if byte & (0x80 >> (x % 8)) != 0 { 0 } else { 255 });
                    }
                }
                out
            }
            other => bail!("PSD depth {other}"),
        })
    }
}

fn check_size(w: u32, h: u32) -> Result<()> {
    ensure!(u64::from(w) * u64::from(h) <= MAX_PIXELS, "PSD plane exceeds {MAX_PIXELS} pixels");
    Ok(())
}

fn row_bytes(w: u32, depth: u16) -> usize {
    match depth {
        1 => (w as usize).div_ceil(8),
        d => w as usize * usize::from(d / 8),
    }
}

/// Keys whose block length is 8 bytes in PSB documents.
fn long_key(key: &[u8; 4]) -> bool {
    matches!(
        key,
        b"LMsk" | b"Lr16" | b"Lr32" | b"Layr" | b"Mt16" | b"Mt32" | b"Mtrn" | b"Alph" | b"FMsk" | b"lnk2" | b"FEid" | b"FXid" | b"PxSD"
    )
}

/// PackBits (Apple RLE) for one row of `row` bytes.
fn packbits(src: &[u8], row: usize, out: &mut Vec<u8>) -> Result<()> {
    let target = out.len() + row;
    let mut i = 0;
    while i < src.len() && out.len() < target {
        let n = src[i] as i8;
        i += 1;
        if n >= 0 {
            let len = n as usize + 1;
            let end = (i + len).min(src.len());
            out.extend_from_slice(&src[i..end]);
            i = end;
        } else if n != -128 {
            let len = (1 - n as isize) as usize;
            let v = src.get(i).copied().unwrap_or(0);
            i += 1;
            out.extend(std::iter::repeat_n(v, len));
        }
    }
    out.resize(target, 0);
    Ok(())
}

/// Undo the zip-with-prediction row deltas (8/16-bit samples, and the
/// byte-planar 32-bit layout).
fn unpredict(raw: &mut [u8], w: u32, h: u32, depth: u16) {
    let w = w as usize;
    match depth {
        8 => {
            for row in raw.chunks_exact_mut(w) {
                for x in 1..w {
                    row[x] = row[x].wrapping_add(row[x - 1]);
                }
            }
        }
        16 => {
            for row in raw.chunks_exact_mut(w * 2) {
                let mut prev = u16::from_be_bytes([row[0], row[1]]);
                for x in 1..w {
                    let v = u16::from_be_bytes([row[x * 2], row[x * 2 + 1]]).wrapping_add(prev);
                    row[x * 2..x * 2 + 2].copy_from_slice(&v.to_be_bytes());
                    prev = v;
                }
            }
        }
        32 => {
            let stride = w * 4;
            for row in raw.chunks_exact_mut(stride).take(h as usize) {
                for x in 1..stride {
                    row[x] = row[x].wrapping_add(row[x - 1]);
                }
                let planar = row.to_vec();
                for x in 0..w {
                    for b in 0..4 {
                        row[x * 4 + b] = planar[b * w + x];
                    }
                }
            }
        }
        _ => {}
    }
}

fn layer_info(layer: &mut Layer, key: &[u8; 4], body: &[u8]) -> Result<()> {
    let mut c = Cursor::new(body, 0);
    match key {
        b"luni" => layer.unicode_name = Some(c.unicode()?),
        b"lyid" => layer.layer_id = c.i32()?,
        b"iOpa" => layer.fill_opacity = c.u8()?,
        b"lsct" | b"lsdk" => {
            match c.u32()? {
                1 | 2 => layer.layer_type = LayerType::Folder,
                3 => layer.layer_type = LayerType::Hidden,
                _ => {}
            }
            if body.len() >= 12 {
                c.skip(4)?;
                let k = c.key()?;
                layer.blend_mode = BlendMode::from_key(k);
            }
        }
        b"SoCo" | b"GdFl" | b"PtFl" => {
            if layer.layer_type == LayerType::Normal {
                layer.layer_type = LayerType::Fill;
            }
        }
        b"brit" | b"levl" | b"curv" | b"expA" | b"vibA" | b"hue " | b"hue2" | b"blnc" | b"blwh" | b"phfl" | b"mixr"
        | b"clrL" | b"nvrt" | b"post" | b"thrs" | b"grdm" | b"selc" => {
            if layer.layer_type == LayerType::Normal {
                layer.layer_type = LayerType::Adjust;
            }
        }
        b"shmd" => {
            let count = c.u32()?;
            for _ in 0..count {
                if c.rest() < 16 {
                    break;
                }
                c.skip(4)?;
                let item = c.key()?;
                c.skip(4)?;
                let len = c.u32()? as usize;
                let start = c.p;
                if &item == b"cmls" && c.u32()? == 16 {
                    if let Ok(d) = descriptor(&mut c) {
                        if let Some(Item::List(settings)) = d.get("layerSettings") {
                            for s in settings {
                                let Item::Object(s) = s else { continue };
                                let Some(Item::List(ids)) = s.get("compList") else { continue };
                                let Some(id) = ids.first().and_then(Item::int) else { continue };
                                let (offset_x, offset_y) = match s.get("Ofst") {
                                    Some(Item::Object(o)) => (
                                        o.get("Hrzn").and_then(Item::int).unwrap_or(0),
                                        o.get("Vrtc").and_then(Item::int).unwrap_or(0),
                                    ),
                                    _ => (0, 0),
                                };
                                let enabled = s.get("enab").and_then(Item::boolean).unwrap_or(layer.visible());
                                layer.comps.insert(id, LayerCompState { id, offset_x, offset_y, enabled });
                            }
                        }
                    }
                }
                c.p = (start + len).min(body.len());
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_slices(c: &mut Cursor) -> Result<Slices> {
    let version = c.u32()?;
    let mut s = Slices::default();
    if version == 6 {
        s.bounds = Rect { top: c.i32()?, left: c.i32()?, bottom: c.i32()?, right: c.i32()? };
        s.group_name = c.unicode()?;
        let count = c.u32()?;
        for _ in 0..count {
            if c.rest() == 0 {
                break;
            }
            let mut item = Slice { id: c.i32()?, group_id: c.i32()?, origin: c.i32()?, ..Default::default() };
            item.associated_layer_id = if item.origin == 1 { c.i32()? } else { -1 };
            item.name = c.unicode()?;
            item.kind = c.i32()?;
            item.rect = Rect { left: c.i32()?, top: c.i32()?, right: c.i32()?, bottom: c.i32()? };
            item.url = c.unicode()?;
            item.target = c.unicode()?;
            item.message = c.unicode()?;
            item.alt_tag = c.unicode()?;
            item.cell_text_is_html = c.u8()? != 0;
            item.cell_text = c.unicode()?;
            item.horizontal_alignment = c.i32()?;
            item.vertical_alignment = c.i32()?;
            let [a, r, g, b] = [c.u8()?, c.u8()?, c.u8()?, c.u8()?];
            item.color = u32::from_be_bytes([a, r, g, b]);
            s.slices.push(item);
        }
    } else if version == 7 || version == 8 {
        ensure!(c.u32()? == 16, "slice descriptor version");
        let d = descriptor(c)?;
        let rect = |o: Option<&Item>| match o {
            Some(Item::Object(b)) => Rect {
                left: b.get("Left").and_then(Item::int).unwrap_or(0),
                top: b.get("Top ").and_then(Item::int).unwrap_or(0),
                right: b.get("Rght").and_then(Item::int).unwrap_or(0),
                bottom: b.get("Btom").and_then(Item::int).unwrap_or(0),
            },
            _ => Rect::default(),
        };
        s.group_name = d.get("baseName").and_then(Item::text).unwrap_or_default();
        s.bounds = rect(d.get("bounds"));
        if let Some(Item::List(list)) = d.get("slices") {
            for it in list {
                let Item::Object(o) = it else { continue };
                let en = |k: &str| match o.get(k) {
                    Some(Item::Enum(e)) => e.clone(),
                    _ => String::new(),
                };
                let text = |k: &str| o.get(k).and_then(Item::text).unwrap_or_default();
                let mut item = Slice {
                    id: o.get("sliceID").and_then(Item::int).unwrap_or(0),
                    group_id: o.get("groupID").and_then(Item::int).unwrap_or(0),
                    associated_layer_id: o.get("layerID").and_then(Item::int).unwrap_or(-1),
                    name: text("Nm  "),
                    rect: rect(o.get("bounds")),
                    url: text("url"),
                    target: text("null"),
                    message: text("Msge"),
                    alt_tag: text("altTag"),
                    cell_text_is_html: o.get("cellTextIsHTML").and_then(Item::boolean).unwrap_or(true),
                    cell_text: text("cellText"),
                    ..Default::default()
                };
                item.origin = match en("origin").as_str() {
                    "layerGenerated" => 1,
                    "userGenerated" => 2,
                    _ => 0,
                };
                item.kind = match en("Type").as_str() {
                    "noImage" => 0,
                    "Tbl " => 2,
                    _ => 1,
                };
                item.horizontal_alignment = match en("horzAlign").as_str() {
                    "Left" => 1,
                    "Cntr" => 2,
                    "Rght" => 3,
                    _ => 0,
                };
                item.vertical_alignment = match en("vertAlign").as_str() {
                    "Top " => 1,
                    "Cntr" => 2,
                    "Bsln" => 3,
                    "Btom" => 4,
                    _ => 0,
                };
                if let Some(Item::Object(col)) = o.get("bgColor") {
                    let ch = |k: &str| col.get(k).and_then(Item::int).unwrap_or(0) as u8;
                    item.color = u32::from_be_bytes([ch("alpha"), ch("Rd  "), ch("Grn "), ch("Bl  ")]);
                }
                s.slices.push(item);
            }
        }
    } else {
        bail!("slice resource version {version}");
    }
    Ok(s)
}

/// Action descriptor values (the subset the document facts use; other
/// types are skipped by length where the format allows it).
#[derive(Clone, Debug)]
pub enum Item {
    Object(Descriptor),
    List(Vec<Item>),
    Int(i32),
    Large(i64),
    Double(f64),
    Unit(f64),
    Bool(bool),
    Text(String),
    Enum(String),
    Class,
    Raw(Vec<u8>),
}
impl Item {
    fn int(&self) -> Option<i32> {
        match self {
            Item::Int(v) => Some(*v),
            Item::Large(v) => i32::try_from(*v).ok(),
            Item::Double(v) | Item::Unit(v) => Some(*v as i32),
            _ => None,
        }
    }
    fn text(&self) -> Option<String> {
        match self {
            Item::Text(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn boolean(&self) -> Option<bool> {
        match self {
            Item::Bool(v) => Some(*v),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Descriptor {
    pub items: Vec<(String, Item)>,
}
impl Descriptor {
    pub fn get(&self, key: &str) -> Option<&Item> {
        self.items.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

fn id_string(c: &mut Cursor) -> Result<String> {
    let n = c.u32()? as usize;
    let n = if n == 0 { 4 } else { n };
    ensure!(n <= 4096, "descriptor key length");
    Ok(String::from_utf8_lossy(c.take(n)?).into_owned())
}

fn descriptor(c: &mut Cursor) -> Result<Descriptor> {
    descriptor_depth(c, 0)
}

fn descriptor_depth(c: &mut Cursor, depth: u32) -> Result<Descriptor> {
    ensure!(depth < 32, "descriptor nesting");
    let _name = c.unicode()?;
    let _class = id_string(c)?;
    let count = c.u32()?;
    ensure!(count as usize <= c.rest(), "descriptor item count");
    let mut items = Vec::new();
    for _ in 0..count {
        let key = id_string(c)?;
        let value = item(c, depth)?;
        items.push((key, value));
    }
    Ok(Descriptor { items })
}

fn item(c: &mut Cursor, depth: u32) -> Result<Item> {
    let kind = c.key()?;
    Ok(match &kind {
        b"Objc" | b"GlbO" => Item::Object(descriptor_depth(c, depth + 1)?),
        b"VlLs" => {
            let n = c.u32()?;
            ensure!(n as usize <= c.rest(), "descriptor list length");
            Item::List((0..n).map(|_| item(c, depth + 1)).collect::<Result<_>>()?)
        }
        b"long" => Item::Int(c.i32()?),
        b"comp" => Item::Large(c.u64()? as i64),
        b"doub" => Item::Double(c.f64()?),
        b"UntF" => {
            c.skip(4)?;
            Item::Unit(c.f64()?)
        }
        b"UnFl" => {
            c.skip(4)?;
            let n = c.u32()? as usize;
            ensure!(n <= c.rest() / 8, "unit float list");
            c.skip(n * 8)?;
            Item::Class
        }
        b"bool" => Item::Bool(c.u8()? != 0),
        b"TEXT" => Item::Text(c.unicode()?),
        b"enum" => {
            let _type = id_string(c)?;
            Item::Enum(id_string(c)?)
        }
        b"type" | b"GlbC" => {
            let _name = c.unicode()?;
            let _class = id_string(c)?;
            Item::Class
        }
        b"alis" | b"tdta" | b"Pth " => {
            let n = c.u32()? as usize;
            Item::Raw(c.take(n)?.to_vec())
        }
        b"obj " => {
            let n = c.u32()?;
            for _ in 0..n {
                let kind = c.key()?;
                match &kind {
                    b"prop" => {
                        let _ = c.unicode()?;
                        id_string(c)?;
                        id_string(c)?;
                    }
                    b"Clss" => {
                        let _ = c.unicode()?;
                        id_string(c)?;
                    }
                    b"Enmr" => {
                        let _ = c.unicode()?;
                        id_string(c)?;
                        id_string(c)?;
                        id_string(c)?;
                    }
                    b"rele" => {
                        let _ = c.unicode()?;
                        id_string(c)?;
                        c.skip(4)?;
                    }
                    b"Idnt" | b"indx" => c.skip(4)?,
                    b"name" => {
                        let _ = c.unicode()?;
                        id_string(c)?;
                        let _ = c.unicode()?;
                    }
                    other => bail!("descriptor reference {:?}", String::from_utf8_lossy(other)),
                }
            }
            Item::Class
        }
        other => bail!("descriptor type {:?}", String::from_utf8_lossy(other)),
    })
}

/// A 32-bit bottom-up BI_RGB bitmap of straight RGBA pixels (stored BGRA),
/// as KiriKiri's `psd://` media serves layer images.
pub fn bmp32(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let pitch = width as usize * 4;
    let size = 54 + pitch * height as usize;
    let mut out = Vec::with_capacity(size);
    out.extend(b"BM");
    out.extend((size as u32).to_le_bytes());
    out.extend([0; 4]);
    out.extend(54u32.to_le_bytes());
    out.extend(40u32.to_le_bytes());
    out.extend((width as i32).to_le_bytes());
    out.extend((height as i32).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend([0; 24]);
    for y in (0..height as usize).rev() {
        for px in rgba[y * pitch..(y + 1) * pitch].chunks_exact(4) {
            out.extend([px[2], px[1], px[0], px[3]]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn be16(v: &mut Vec<u8>, x: u16) {
        v.extend(x.to_be_bytes());
    }
    fn be32(v: &mut Vec<u8>, x: u32) {
        v.extend(x.to_be_bytes());
    }

    /// A 4x2 RGB document with one 2x2 layer (RLE alpha, raw colour, a user
    /// mask), a guide resource and a merged image.
    pub(crate) fn fixture() -> Vec<u8> {
        let mut f = b"8BPS".to_vec();
        be16(&mut f, 1);
        f.extend([0; 6]);
        be16(&mut f, 3);
        be32(&mut f, 2);
        be32(&mut f, 4);
        be16(&mut f, 8);
        be16(&mut f, MODE_RGB);
        be32(&mut f, 0);
        // Guides: one vertical at x=2 (fixed 27.5 → 2*32).
        let mut res = b"8BIM".to_vec();
        be16(&mut res, 1032);
        res.extend([0, 0]);
        let mut body = Vec::new();
        be32(&mut body, 1);
        be32(&mut body, 576);
        be32(&mut body, 576);
        be32(&mut body, 1);
        be32(&mut body, 64);
        body.push(0);
        be32(&mut res, body.len() as u32);
        res.extend(&body);
        if body.len() % 2 == 1 {
            res.push(0);
        }
        be32(&mut f, res.len() as u32);
        f.extend(res);
        // Layer record.
        let mut info = Vec::new();
        be16(&mut info, 1);
        for v in [0u32, 1, 2, 3] {
            be32(&mut info, v); // top 0 left 1 bottom 2 right 3
        }
        let colour: Vec<Vec<u8>> = vec![vec![10, 20, 30, 40], vec![50, 60, 70, 80], vec![90, 100, 110, 120]];
        let alpha_rle: Vec<u8> = {
            let mut v = Vec::new();
            be16(&mut v, 1);
            be16(&mut v, 2);
            be16(&mut v, 3);
            v.extend([0xFF, 255]); // run of 2 x 255
            v.extend([0x01, 128, 64]); // literal 2
            v
        };
        let mask_plane: Vec<u8> = {
            let mut v = Vec::new();
            be16(&mut v, 0);
            v.extend([255, 0, 255, 255]);
            v
        };
        let channel_ids: [i16; 5] = [-1, 0, 1, 2, -2];
        let mut data: Vec<Vec<u8>> = vec![alpha_rle];
        for plane in &colour {
            let mut v = Vec::new();
            be16(&mut v, 0);
            v.extend(plane);
            data.push(v);
        }
        data.push(mask_plane);
        be16(&mut info, channel_ids.len() as u16);
        for (id, d) in channel_ids.iter().zip(&data) {
            be16(&mut info, *id as u16);
            be32(&mut info, d.len() as u32);
        }
        info.extend(b"8BIMmul ");
        info.extend([200, 0, 0, 0]);
        let mut extra = Vec::new();
        be32(&mut extra, 20);
        for v in [0u32, 1, 2, 3] {
            be32(&mut extra, v);
        }
        extra.extend([0, 0, 0, 0]);
        be32(&mut extra, 0);
        extra.extend([3, b'a', b'b', b'c']);
        extra.extend(b"8BIMlyid");
        be32(&mut extra, 4);
        be32(&mut extra, 7);
        be32(&mut info, extra.len() as u32);
        info.extend(extra);
        for d in &data {
            info.extend(d);
        }
        let mut section = Vec::new();
        be32(&mut section, info.len() as u32);
        section.extend(info);
        be32(&mut section, 0);
        be32(&mut f, section.len() as u32);
        f.extend(section);
        // Merged raw image.
        be16(&mut f, 0);
        for c in 0..3u8 {
            f.extend([c * 10 + 1; 8]);
        }
        f
    }

    #[test]
    fn layers_masks_resources_and_merged_image() {
        let doc = Document::parse(fixture()).unwrap();
        assert_eq!((doc.width, doc.height, doc.color_mode, doc.depth), (4, 2, MODE_RGB, 8));
        assert_eq!(doc.grid_guides.as_ref().unwrap().guides[0].location, 64);
        let layer = &doc.layers[0];
        assert_eq!((layer.display_name(), layer.layer_id, layer.opacity), ("abc", 7, 200));
        assert_eq!(layer.blend_mode, BlendMode::Multiply);
        assert!(layer.has_mask_channel());
        let (rect, raw) = doc.layer_rgba(0, ImageMode::Image).unwrap();
        assert_eq!((rect.left, rect.top, rect.width(), rect.height()), (1, 0, 2, 2));
        assert_eq!(&raw[..4], &[10, 50, 90, 255]);
        assert_eq!(&raw[12..16], &[40, 80, 120, 64]);
        let (_, masked) = doc.layer_rgba(0, ImageMode::MaskedImage).unwrap();
        assert_eq!(masked[7], 0, "mask zero at (1,0)");
        assert_eq!(masked[15], 64);
        let (_, mask) = doc.layer_rgba(0, ImageMode::Mask).unwrap();
        assert_eq!(&mask[4..8], &[0, 0, 0, 255]);
        let merged = doc.merged_rgba().unwrap().unwrap();
        assert_eq!(&merged[..4], &[1, 11, 21, 255]);
        let bmp = bmp32(2, 2, &raw);
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(bmp.len(), 54 + 16);
    }
}
