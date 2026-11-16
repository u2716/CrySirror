// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! psd.dll: the `PSD` class and the `psd` storage media.
//!
//! Contract: wtnbgo `psdfile` (7edea2b, `psdclass.cpp`, `main.cpp`,
//! `manual.tjs`) for the members the shipped plug-in registers: `load`,
//! `clearStorageCache`, read-only `width/height/channels/depth/color_mode/
//! layer_count` (-1 before a load), `getLayerType`, `getLayerName`,
//! `getLayerInfo`, `getLayerData`, `getLayerDataRaw`, `getLayerDataMask`,
//! `getBlend`, `getSlices`, `getGuides`, `getLayerComp`, plus the
//! `color_mode_*`, `layer_type_*` and `blend_mode_*` constants. Errors use the
//! reference texts (`no data`, `not such layer`, `not layer`, `invalid layer
//! type`, `%1:cannot open psdfile`, `invalid path:%1`).
//!
//! Parsing and pixel composition are the shared `crysirror_image::psd`
//! decoder. `getLayerData*` / `getBlend` assign the target Layer's properties
//! through their real setters, in the reference order. They then hand the
//! pixels to the shared image job, which publishes them before the script
//! continues.
//!
//! Media: `psd://<file>/root/<folder>/…/<layer>.bmp` and
//! `psd://<file>/id/<layerId>.bmp` (lower-cased; `/` in names becomes `_`;
//! for duplicate names the bottom-most layer wins) answer 32-bit bottom-up
//! BMPs of the masked layer image. `<file>` is the lower-cased base name of a
//! loaded `PSD` object, held weakly. Otherwise the media opens that storage
//! itself and keeps the most recent document as its cache, which
//! `clearStorageCache` drops.
use crate::natives::{Rt, this_of};
use crate::storage::Storage;
use crysirror_image::psd::{BlendMode, Document, ImageMode, Layer, LayerType};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{NativeArgCount, ObjectHandle, Variant},
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

fn err(e: impl std::fmt::Display) -> TjsError {
    TjsError::runtime(e.to_string())
}

/// A loaded document with its media name maps.
pub(crate) struct PsdIndex {
    doc: Document,
    /// `root/<folders>/` → layer path name → index.
    paths: BTreeMap<String, BTreeMap<String, usize>>,
    ids: BTreeMap<i32, usize>,
}

fn path_name(layer: &Layer) -> String {
    layer.display_name().replace('/', "_").to_lowercase()
}

impl PsdIndex {
    fn new(doc: Document) -> Self {
        let mut paths: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        let mut ids = BTreeMap::new();
        for i in (0..doc.layers.len()).rev() {
            let layer = &doc.layers[i];
            if layer.layer_type != LayerType::Normal {
                continue;
            }
            let mut dir = String::new();
            let mut parent = layer.parent;
            while let Some(p) = parent {
                dir = format!("{}/{dir}", path_name(&doc.layers[p]));
                parent = doc.layers[p].parent;
            }
            paths.entry(format!("root/{dir}")).or_default().insert(path_name(layer), i);
            ids.insert(layer.layer_id, i);
        }
        Self { doc, paths, ids }
    }
    /// `CheckExistentStorage`: the layer index a media-relative name names.
    fn lookup(&self, name: &str) -> Option<usize> {
        if let Some(rest) = name.strip_prefix("id/") {
            let digits = rest.strip_suffix(".bmp")?;
            if rest.contains('/') || digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            return self.ids.get(&digits.parse().ok()?).copied();
        }
        let (dir, file) = name.rsplit_once('/')?;
        let base = file.strip_suffix(".bmp")?;
        self.paths.get(&format!("{dir}/"))?.get(base).copied()
    }
    fn bmp(&self, index: usize) -> anyhow::Result<Vec<u8>> {
        let layer = &self.doc.layers[index];
        anyhow::ensure!(layer.layer_type == LayerType::Normal, "not a normal layer");
        let (rect, rgba) = self.doc.layer_rgba(index, ImageMode::MaskedImage)?;
        anyhow::ensure!(!rgba.is_empty(), "empty layer");
        Ok(crysirror_image::psd::bmp32(rect.width() as u32, rect.height() as u32, &rgba))
    }
}

/// The `psd` storage media.
#[derive(Default)]
pub(crate) struct PsdMedia {
    registered: Mutex<BTreeMap<String, Weak<PsdIndex>>>,
    cache: Mutex<Option<(String, Arc<PsdIndex>)>>,
}

/// The media-relative part of a `psd://` name, lower-cased.
pub(crate) fn psd_uri(name: &str) -> Option<String> {
    let (scheme, rest) = name.split_once("://")?;
    scheme.eq_ignore_ascii_case("psd").then(|| rest.replace('\\', "/").to_lowercase())
}

impl PsdMedia {
    fn register(&self, dname: String, index: &Arc<PsdIndex>) {
        if let Ok(mut map) = self.registered.lock() {
            map.retain(|_, w| w.strong_count() > 0);
            map.insert(dname, Arc::downgrade(index));
        }
    }
    fn unregister(&self, index: &Arc<PsdIndex>) {
        if let Ok(mut map) = self.registered.lock() {
            map.retain(|_, w| w.upgrade().is_some_and(|i| !Arc::ptr_eq(&i, index)));
        }
    }
    fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = None;
        }
    }
    fn document(&self, storage: &Storage, dname: &str) -> Option<Arc<PsdIndex>> {
        if let Some((name, index)) = self.cache.lock().ok()?.as_ref() {
            if name == dname {
                return Some(index.clone());
            }
        }
        let registered = self.registered.lock().ok()?.get(dname).and_then(Weak::upgrade);
        let index = match registered {
            Some(index) => index,
            None => {
                let path = storage.resolve(dname)?;
                let bytes = storage.read(&path).ok()?;
                Arc::new(PsdIndex::new(Document::parse(bytes).ok()?))
            }
        };
        *self.cache.lock().ok()? = Some((dname.to_owned(), index.clone()));
        Some(index)
    }
    fn split(rest: &str) -> anyhow::Result<(&str, &str)> {
        rest.split_once('/').ok_or_else(|| anyhow::anyhow!("invalid path:{rest}"))
    }
    pub(crate) fn exists(&self, storage: &Storage, rest: &str) -> bool {
        let Ok((dname, name)) = Self::split(rest) else { return false };
        self.document(storage, dname).is_some_and(|index| index.lookup(name).is_some())
    }
    pub(crate) fn open(&self, storage: &Storage, rest: &str) -> anyhow::Result<Vec<u8>> {
        let (dname, name) = Self::split(rest)?;
        self.document(storage, dname)
            .and_then(|index| index.lookup(name).and_then(|i| index.bmp(i).ok()))
            .ok_or_else(|| anyhow::anyhow!("psd://{rest}:cannot open psdfile"))
    }
}

/// Per-object documents (`None` until a successful `load`).
#[derive(Default)]
pub(crate) struct Psd {
    objects: BTreeMap<usize, Option<Arc<PsdIndex>>>,
}
impl Psd {
    /// Drop an object's document and its weak media registration.
    pub(crate) fn invalidate(&mut self, object: usize, storage: Option<&Storage>) {
        if let Some(index) = self.objects.remove(&object).flatten() {
            if let Some(media) = storage.and_then(|s| s.wrappers.psd.as_ref()) {
                media.unregister(&index);
            }
        }
    }
}

fn instance(rt: &Rt, this: Option<ObjectHandle>, what: &str) -> Result<ObjectHandle> {
    let h = this_of(rt, this, what)?;
    if !rt.host().psd.objects.contains_key(&h.0) {
        return Err(err(format!("{what}: native class crash (no PSD instance)")));
    }
    Ok(h)
}

fn loaded(rt: &Rt, this: Option<ObjectHandle>, what: &str) -> Result<Arc<PsdIndex>> {
    let h = instance(rt, this, what)?;
    rt.host().psd.objects.get(&h.0).cloned().flatten().ok_or_else(|| err("no data"))
}

fn layer_no(index: &PsdIndex, v: Option<&Variant>) -> Result<usize> {
    let no = v.ok_or_else(TjsError::bad_param_count)?.to_integer()?;
    usize::try_from(no).ok().filter(|n| *n < index.doc.layers.len()).ok_or_else(|| err("not such layer"))
}

/// krkr `Layer.type` for a Photoshop blend mode (unsupported modes are normal).
fn krkr_type(mode: BlendMode) -> i64 {
    use BlendMode::*;
    match mode {
        Darken => 25,
        Multiply => 16,
        ColorBurn => 23,
        LinearBurn => 15,
        Lighten => 24,
        Screen => 17,
        ColorDodge => 21,
        LinearDodge => 14,
        Overlay => 18,
        SoftLight => 20,
        HardLight => 19,
        Difference => 26,
        Exclusion => 28,
        _ => 13,
    }
}

fn dictionary(rt: &mut Rt, entries: Vec<(String, Variant)>) -> Variant {
    let d = rt.alloc_dictionary_object();
    for (k, v) in entries {
        rt.set_object_member(d, k, v);
    }
    Variant::Object(d)
}
fn s(v: &str) -> Variant {
    Variant::String(v.into())
}
fn i(v: impl Into<i64>) -> Variant {
    Variant::Integer(v.into())
}
fn b(v: bool) -> Variant {
    Variant::Integer(i64::from(v))
}

fn layer_target(rt: &Rt, v: Option<&Variant>) -> Result<ObjectHandle> {
    v.and_then(crate::natives::object_arg)
        .filter(|h| rt.object_class_infos(*h).iter().any(|c| c == "Layer"))
        .ok_or_else(|| err("not layer"))
}

fn assign(rt: &mut Rt, layer: ObjectHandle, props: Vec<(&str, Variant)>) -> Result<()> {
    for (name, value) in props {
        rt.put_object_member(layer, name, value)?;
    }
    Ok(())
}

fn layer_data(rt: &mut Rt, this: Option<ObjectHandle>, a: &[Variant], mode: ImageMode) -> Result<Variant> {
    let target = layer_target(rt, a.first())?;
    let index = loaded(rt, this, "PSD.getLayerData")?;
    let no = layer_no(&index, a.get(1))?;
    let layer = &index.doc.layers[no];
    let folder_mask = layer.layer_type == LayerType::Folder && mode == ImageMode::Mask;
    if layer.layer_type != LayerType::Normal && !folder_mask {
        return Err(err("invalid layer type"));
    }
    let (rect, rgba) = index.doc.layer_rgba(no, mode).map_err(err)?;
    let (mut left, mut top, mut width, mut height) = (rect.left, rect.top, rect.width(), rect.height());
    let dummy = mode == ImageMode::Mask && (width == 0 || height == 0);
    if dummy {
        (left, top, width, height) = (0, 0, 1, 1);
    }
    if width <= 0 || height <= 0 {
        return Ok(Variant::Void);
    }
    let (opacity, fill, kind) = if mode == ImageMode::Mask {
        (255, 255, 13)
    } else {
        (i64::from(layer.opacity), i64::from(layer.fill_opacity), krkr_type(layer.blend_mode))
    };
    let mut props = vec![
        ("left", i(left)),
        ("top", i(top)),
        ("opacity", i(opacity)),
        ("fill_opacity", i(fill)),
        ("width", i(width)),
        ("height", i(height)),
        ("type", i(kind)),
        ("visible", b(layer.visible())),
        ("imageLeft", i(0)),
        ("imageTop", i(0)),
        ("imageWidth", i(width)),
        ("imageHeight", i(height)),
        ("width", i(width)),
        ("height", i(height)),
        ("name", s(layer.display_name())),
    ];
    if mode == ImageMode::Mask {
        props.push(("defaultMaskColor", i(layer.mask.default_color)));
    }
    assign(rt, target, props)?;
    let pixels = if dummy {
        let c = layer.mask.default_color;
        vec![c, c, c, 255]
    } else {
        rgba
    };
    crate::psbfile_plugin::defer_rgba(rt, target, [width as u32, height as u32], pixels)?;
    Ok(Variant::Void)
}

fn layer_info(rt: &mut Rt, index: &PsdIndex, no: usize) -> Variant {
    let layer = &index.doc.layers[no];
    let r = &layer.rect;
    let mut e = vec![
        ("top".into(), i(r.top)),
        ("left".into(), i(r.left)),
        ("bottom".into(), i(r.bottom)),
        ("right".into(), i(r.right)),
        ("width".into(), i(r.width())),
        ("height".into(), i(r.height())),
        ("opacity".into(), i(layer.opacity)),
        ("fill_opacity".into(), i(layer.fill_opacity)),
        ("mask".into(), b(layer.has_mask_channel())),
    ];
    let m = &layer.mask;
    if m.present && m.has_parameters() {
        let mut p = Vec::new();
        if let Some(v) = m.user_density {
            p.push(("user_density".into(), i(v)));
        }
        if let Some(v) = m.user_feather {
            p.push(("user_feather".into(), Variant::Real(v)));
        }
        if let Some(v) = m.vector_density {
            p.push(("vector_density".into(), i(v)));
        }
        if let Some(v) = m.vector_feather {
            p.push(("vector_feather".into(), Variant::Real(v)));
        }
        let d = dictionary(rt, p);
        e.push(("mask_params".into(), d));
    }
    e.extend([
        ("type".into(), i(krkr_type(layer.blend_mode))),
        ("layer_type".into(), i(layer.layer_type as i64)),
        ("blend_mode".into(), i(layer.blend_mode as i64)),
        ("blend_mode_key".into(), s(&String::from_utf8_lossy(&layer.blend_key))),
        ("visible".into(), b(layer.visible())),
        ("name".into(), s(layer.display_name())),
        ("clipping".into(), i(layer.clipping)),
        ("layer_id".into(), i(layer.layer_id)),
        ("obsolete".into(), b(layer.obsolete())),
        ("transparency_protected".into(), b(layer.transparency_protected())),
        ("pixel_data_irrelevant".into(), b(layer.pixel_data_irrelevant())),
    ]);
    if !layer.comps.is_empty() {
        let mut comps = Vec::new();
        for (id, c) in &layer.comps {
            let entry = dictionary(rt, vec![
                ("id".into(), i(c.id)),
                ("offset_x".into(), i(c.offset_x)),
                ("offset_y".into(), i(c.offset_y)),
                ("enable".into(), b(c.enabled)),
            ]);
            comps.push((id.to_string(), entry));
        }
        let d = dictionary(rt, comps);
        e.push(("layer_comp".into(), d));
    }
    if let Some(p) = layer.parent {
        e.push(("group_layer_id".into(), i(index.doc.layers[p].layer_id)));
    }
    e.push(("parent_index".into(), i(layer.parent.map_or(-1, |p| p as i64))));
    dictionary(rt, e)
}

/// `Layer.loadImages("psd://…")`: the media's BMP, decoded by the shared
/// image decoder and staged like any deferred layer image.
pub(crate) fn load_image(rt: &mut Rt, layer: ObjectHandle, name: &str) -> Result<Variant> {
    let bytes = rt.host().storage()?.read_named(name).map_err(err)?;
    let image = crysirror_image::decode_standard(&bytes).map_err(|e| err(format!("decode `{name}`: {e}")))?;
    let (w, h) = (image.width, image.height);
    crate::psbfile_plugin::defer_rgba(rt, layer, [w, h], image.rgba)?;
    let l = rt.host_mut().layer_mut(layer)?;
    (l.width, l.height) = (i64::from(w), i64::from(h));
    Ok(Variant::Void)
}

pub(crate) fn install(rt: &mut Rt) -> Result<()> {
    if rt.host().storage.is_some() && rt.host().storage()?.wrappers.psd.is_none() {
        rt.host_mut().storage_mut()?.wrappers.psd = Some(PsdMedia::default());
    }
    if rt.global_member("PSD").object_handle().is_some() {
        return Ok(());
    }
    let class = rt.alloc_native_constructor(|rt: &mut Rt, this: Option<ObjectHandle>, _args: Vec<Variant>| {
        let h = this
            .map(|h| rt.bound_this(h).unwrap_or(h))
            .filter(|h| *h != rt.global_handle())
            .unwrap_or_else(|| rt.alloc_ordinary_object());
        rt.add_object_class_info(h, "PSD");
        members(rt, h);
        rt.host_mut().psd.objects.insert(h.0, None);
        Ok(Variant::Object(h))
    });
    rt.add_object_class_info(class, "PSD");
    members(rt, class);
    for (name, value) in [
        ("color_mode_bitmap", 0),
        ("color_mode_grayscale", 1),
        ("color_mode_indexed", 2),
        ("color_mode_rgb", 3),
        ("color_mode_cmyk", 4),
        ("color_mode_multichannel", 7),
        ("color_mode_duotone", 8),
        ("color_mode_lab", 9),
        ("layer_type_normal", 0),
        ("layer_type_hidden", 1),
        ("layer_type_folder", 2),
        ("layer_type_adjust", 3),
        ("layer_type_fill", 4),
    ] {
        rt.set_object_member(class, name, i(value));
    }
    for (value, name) in [
        "normal", "dissolve", "darken", "multiply", "color_burn", "linear_burn", "lighten", "screen", "color_dodge",
        "linear_dodge", "overlay", "soft_light", "hard_light", "vivid_light", "linear_light", "pin_light", "hard_mix",
        "difference", "exclusion", "hue", "saturation", "color", "luminosity", "pass_through", "darker_color",
        "lighter_color", "subtract", "divide",
    ]
    .iter()
    .enumerate()
    {
        rt.set_object_member(class, format!("blend_mode_{name}"), i(value as i64));
    }
    rt.set_global_member("PSD", Variant::Object(class));
    Ok(())
}

fn members(rt: &mut Rt, h: ObjectHandle) {
    rt.register_object_native_with_arg_count(h, "load", NativeArgCount::AtLeast(1), |rt: &mut Rt, this, a: Vec<Variant>| {
        let obj = instance(rt, this, "PSD.load")?;
        let name = a[0].to_tjs_string()?;
        let previous = rt.host_mut().psd.objects.insert(obj.0, None).flatten();
        let storage = rt.host().storage()?;
        if let (Some(previous), Some(media)) = (&previous, &storage.wrappers.psd) {
            media.unregister(previous);
        }
        let Some(path) = storage.resolve(&name) else { return Ok(b(false)) };
        let bytes = storage.read(&path).map_err(err)?;
        let Ok(doc) = Document::parse(bytes) else { return Ok(b(false)) };
        let index = Arc::new(PsdIndex::new(doc));
        let dname = name.rsplit('/').next().unwrap_or(&name).to_lowercase();
        if let Some(media) = &rt.host().storage()?.wrappers.psd {
            media.register(dname, &index);
        }
        rt.host_mut().psd.objects.insert(obj.0, Some(index));
        Ok(b(true))
    });
    rt.register_object_native(h, "clearStorageCache", |rt: &mut Rt, _, _| {
        if let Some(media) = &rt.host().storage()?.wrappers.psd {
            media.clear_cache();
        }
        Ok(Variant::Void)
    });
    type Fact = fn(&Document) -> i64;
    let facts: [(&str, Fact); 6] = [
        ("width", |d| i64::from(d.width)),
        ("height", |d| i64::from(d.height)),
        ("channels", |d| i64::from(d.channels)),
        ("depth", |d| i64::from(d.depth)),
        ("color_mode", |d| i64::from(d.color_mode)),
        ("layer_count", |d| d.layers.len() as i64),
    ];
    for (name, fact) in facts {
        rt.register_object_native_property(
            h,
            name,
            move |rt: &mut Rt, this: Option<ObjectHandle>| {
                let obj = instance(rt, this, "PSD")?;
                Ok(i(rt.host().psd.objects.get(&obj.0).cloned().flatten().map_or(-1, |x| fact(&x.doc))))
            },
            move |_, _, _| Err(err(format!("PSD.{name} is read-only"))),
        );
    }
    rt.register_object_native_with_arg_count(h, "getLayerType", NativeArgCount::AtLeast(1), |rt: &mut Rt, this, a: Vec<Variant>| {
        let index = loaded(rt, this, "PSD.getLayerType")?;
        let no = layer_no(&index, a.first())?;
        Ok(i(index.doc.layers[no].layer_type as i64))
    });
    rt.register_object_native_with_arg_count(h, "getLayerName", NativeArgCount::AtLeast(1), |rt: &mut Rt, this, a: Vec<Variant>| {
        let index = loaded(rt, this, "PSD.getLayerName")?;
        let no = layer_no(&index, a.first())?;
        Ok(s(index.doc.layers[no].display_name()))
    });
    rt.register_object_native_with_arg_count(h, "getLayerInfo", NativeArgCount::AtLeast(1), |rt: &mut Rt, this, a: Vec<Variant>| {
        let index = loaded(rt, this, "PSD.getLayerInfo")?;
        let no = layer_no(&index, a.first())?;
        Ok(layer_info(rt, &index, no))
    });
    rt.register_object_native_with_arg_count(h, "getLayerData", NativeArgCount::AtLeast(2), |rt: &mut Rt, this, a: Vec<Variant>| {
        layer_data(rt, this, &a, ImageMode::MaskedImage)
    });
    rt.register_object_native_with_arg_count(h, "getLayerDataRaw", NativeArgCount::AtLeast(2), |rt: &mut Rt, this, a: Vec<Variant>| {
        layer_data(rt, this, &a, ImageMode::Image)
    });
    rt.register_object_native_with_arg_count(h, "getLayerDataMask", NativeArgCount::AtLeast(2), |rt: &mut Rt, this, a: Vec<Variant>| {
        layer_data(rt, this, &a, ImageMode::Mask)
    });
    rt.register_object_native_with_arg_count(h, "getBlend", NativeArgCount::AtLeast(1), |rt: &mut Rt, this, a: Vec<Variant>| {
        let target = layer_target(rt, a.first())?;
        let index = loaded(rt, this, "PSD.getBlend")?;
        let Some(rgba) = index.doc.merged_rgba().map_err(err)? else { return Ok(b(false)) };
        let (w, h) = (i64::from(index.doc.width), i64::from(index.doc.height));
        assign(rt, target, vec![
            ("width", i(w)),
            ("height", i(h)),
            ("imageLeft", i(0)),
            ("imageTop", i(0)),
            ("imageWidth", i(w)),
            ("imageHeight", i(h)),
            ("width", i(w)),
            ("height", i(h)),
        ])?;
        crate::psbfile_plugin::defer_rgba(rt, target, [w as u32, h as u32], rgba)?;
        Ok(b(true))
    });
    rt.register_object_native(h, "getGuides", |rt: &mut Rt, this, _| {
        let index = loaded(rt, this, "PSD.getGuides")?;
        let Some(g) = &index.doc.grid_guides else { return Ok(Variant::Void) };
        let vertical: Vec<Variant> = g.guides.iter().filter(|x| x.direction == 0).map(|x| i(x.location)).collect();
        let horizontal: Vec<Variant> = g.guides.iter().filter(|x| x.direction != 0).map(|x| i(x.location)).collect();
        let (v, hz) = (rt.alloc_array_object(vertical), rt.alloc_array_object(horizontal));
        Ok(dictionary(rt, vec![
            ("horz_grid".into(), i(g.horizontal_grid)),
            ("vert_grid".into(), i(g.vertical_grid)),
            ("vertical".into(), Variant::Object(v)),
            ("horizontal".into(), Variant::Object(hz)),
        ]))
    });
    rt.register_object_native(h, "getSlices", |rt: &mut Rt, this, _| {
        let index = loaded(rt, this, "PSD.getSlices")?;
        let Some(sr) = &index.doc.slices else { return Ok(Variant::Void) };
        let mut items = Vec::new();
        for it in &sr.slices {
            items.push(dictionary(rt, vec![
                ("id".into(), i(it.id)),
                ("group_id".into(), i(it.group_id)),
                ("origin".into(), i(it.origin)),
                ("type".into(), i(it.kind)),
                ("left".into(), i(it.rect.left)),
                ("top".into(), i(it.rect.top)),
                ("right".into(), i(it.rect.right)),
                ("bottom".into(), i(it.rect.bottom)),
                ("color".into(), i(i64::from(it.color as i32))),
                ("cell_text_is_html".into(), b(it.cell_text_is_html)),
                ("horizontal_alignment".into(), i(it.horizontal_alignment)),
                ("vertical_alignment".into(), i(it.vertical_alignment)),
                ("associated_layer_id".into(), i(it.associated_layer_id)),
                ("name".into(), s(&it.name)),
                ("url".into(), s(&it.url)),
                ("target".into(), s(&it.target)),
                ("message".into(), s(&it.message)),
                ("alt_tag".into(), s(&it.alt_tag)),
                ("cell_text".into(), s(&it.cell_text)),
            ]));
        }
        let arr = rt.alloc_array_object(items);
        Ok(dictionary(rt, vec![
            ("top".into(), i(sr.bounds.top)),
            ("left".into(), i(sr.bounds.left)),
            ("bottom".into(), i(sr.bounds.bottom)),
            ("right".into(), i(sr.bounds.right)),
            ("name".into(), s(&sr.group_name)),
            ("slices".into(), Variant::Object(arr)),
        ]))
    });
    rt.register_object_native(h, "getLayerComp", |rt: &mut Rt, this, _| {
        let index = loaded(rt, this, "PSD.getLayerComp")?;
        if index.doc.layer_comps.is_empty() {
            return Ok(Variant::Void);
        }
        let mut comps = Vec::new();
        for c in &index.doc.layer_comps {
            comps.push(dictionary(rt, vec![
                ("id".into(), i(c.id)),
                ("record_visibility".into(), b(c.record_visibility)),
                ("record_position".into(), b(c.record_position)),
                ("record_appearance".into(), b(c.record_appearance)),
                ("name".into(), s(&c.name)),
                ("comment".into(), s(&c.comment)),
            ]));
        }
        let arr = rt.alloc_array_object(comps);
        Ok(dictionary(rt, vec![
            ("last_applied_id".into(), i(index.doc.last_applied_comp)),
            ("comps".into(), Variant::Object(arr)),
        ]))
    });
}
