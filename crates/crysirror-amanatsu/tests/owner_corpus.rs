// SPDX-License-Identifier: MPL-2.0
//! Owner-corpus checks over an installed game. Ignored by default:
//! `AMANATSU_ROOT=<game> cargo test --release -p crysirror-amanatsu -- --ignored --nocapture`.
use crysirror_core::service::AssetReader;
use crysirror_unity::{bundle::Bundle, SerializedFile};
use std::{collections::BTreeMap, path::PathBuf, sync::{Arc, Mutex}};

struct File(Mutex<std::fs::File>);
impl AssetReader for File {
    fn len(&self) -> crysirror_core::Result<u64> {
        let f = self.0.lock().unwrap();
        f.metadata().map(|m| m.len()).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = self.0.lock().unwrap();
        f.seek(SeekFrom::Start(at)).and_then(|_| f.read(out)).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}

/// The game directory as client asset storage (`Mount::Asset`).
struct Dir(PathBuf);
impl crysirror_core::service::ClientStorage for Dir {
    fn read(&self, p: &crysirror_core::path::VfsPath) -> crysirror_core::Result<Vec<u8>> {
        std::fs::read(self.0.join(p.relative())).map_err(|_| crysirror_core::Error::NotFound)
    }
    fn write_atomic(&self, _: &crysirror_core::path::VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::ReadOnly)
    }
    fn remove(&self, _: &crysirror_core::path::VfsPath) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::ReadOnly)
    }
    fn list(&self, _: crysirror_core::path::Mount, dir: &str) -> crysirror_core::Result<Vec<String>> {
        let rd = std::fs::read_dir(self.0.join(dir)).map_err(|_| crysirror_core::Error::NotFound)?;
        Ok(rd.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect())
    }
    fn open_asset(&self, p: &crysirror_core::path::VfsPath) -> crysirror_core::Result<Arc<dyn AssetReader>> {
        let f = std::fs::File::open(self.0.join(p.relative())).map_err(|_| crysirror_core::Error::NotFound)?;
        Ok(Arc::new(File(Mutex::new(f))))
    }
}

/// The adapter's Unity view of `AMANATSU_ROOT`.
fn unity() -> crysirror_amanatsu::unity::Unity {
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    crysirror_amanatsu::unity::Unity::new(Arc::new(Dir(root)), "AmanatsuLocation_Data").unwrap()
}

fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|e| e == "unity3d") {
            out.push(p);
        }
    }
}

/// Every scenario of `lib/adv` decodes; prints the command census.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn every_scenario_decodes() {
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let mut files = Vec::new();
    walk(&root.join("lib/adv"), &mut files);
    let (mut scenarios, mut commands) = (0, 0);
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    for path in files {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let bundle = Bundle::open(source).unwrap();
        for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
            let file = SerializedFile::open(bundle.reader(node)).unwrap();
            for info in file.objects.iter().filter(|o| o.class_id == 114) {
                let f = file.decode(info).unwrap();
                let Some(crysirror_unity::managed::Value::Bytes(b)) = f.get("_bytes") else { continue };
                let list = crysirror_amanatsu::adv::parse_scenario(b).unwrap_or_else(|e| panic!("{} {}: {e:#}", path.display(), f.string("m_Name").unwrap_or("")));
                scenarios += 1;
                commands += list.len();
                for c in &list {
                    *census.entry(c.name()).or_default() += 1;
                }
            }
        }
    }
    let mut by_count: Vec<_> = census.into_iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("{scenarios} scenarios, {commands} commands, {} kinds", by_count.len());
    eprintln!("{by_count:?}");
    assert!(scenarios > 0);
}

/// Census of `Texture2D` formats (count, total level-0 pixels) under `lib`.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn texture_format_census() {
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let mut files = Vec::new();
    walk(&root.join("lib"), &mut files);
    let mut census: BTreeMap<i64, (usize, u64)> = BTreeMap::new();
    let mut streamed = 0;
    for path in files {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let bundle = Bundle::open(source).unwrap();
        for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
            let file = SerializedFile::open(bundle.reader(node)).unwrap();
            for info in file.objects.iter().filter(|o| o.class_id == 28) {
                let f = file.decode(info).unwrap();
                let fmt = f.int("m_TextureFormat").unwrap_or(-1);
                let px = (f.int("m_Width").unwrap_or(0) * f.int("m_Height").unwrap_or(0)) as u64;
                let e = census.entry(fmt).or_default();
                e.0 += 1;
                e.1 += px;
                if let Some(crysirror_unity::managed::Value::Struct(s)) = f.get("m_StreamData") {
                    if s.int("size").unwrap_or(0) > 0 {
                        streamed += 1;
                    }
                }
            }
        }
    }
    eprintln!("formats {census:?}; streamed {streamed}");
}

/// Every `Texture2D` of `AMANATSU_DIR` (default `lib/chara`) decodes.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn textures_decode() {
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let sub = std::env::var("AMANATSU_DIR").unwrap_or_else(|_| "lib/chara".into());
    let mut files = Vec::new();
    walk(&root.join(sub), &mut files);
    let (mut ok, mut failures) = (0, Vec::new());
    for path in files {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let bundle = Bundle::open(source).unwrap();
        let resolve = |p: &str| bundle.node(crysirror_unity::unity6::stream_node(p)).map(|n| bundle.reader(n));
        for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
            let file = SerializedFile::open(bundle.reader(node)).unwrap();
            for info in file.objects.iter().filter(|o| o.class_id == 28) {
                let f = file.decode(info).unwrap();
                match crysirror_unity::unity6::texture_2d(&f, &resolve) {
                    Ok(img) => {
                        ok += 1;
                        let name = f.string("m_Name").unwrap_or("");
                        if let Ok(want) = std::env::var("AMANATSU_PNG") {
                            if name.contains(&want) && img.width <= 4096 {
                                let out = std::env::temp_dir().join(format!("amanatsu_{name}.png"));
                                image::save_buffer(&out, &img.rgba, img.width, img.height, image::ColorType::Rgba8).unwrap();
                                eprintln!("saved {}", out.display());
                            }
                        }
                    }
                    Err(e) => failures.push(format!("{} {}: {e:#}", path.display(), f.string("m_Name").unwrap_or(""))),
                }
            }
        }
    }
    eprintln!("{ok} textures decoded");
    for f in failures.iter().take(10) {
        eprintln!("{f}");
    }
    assert!(failures.is_empty(), "{} failures", failures.len());
}

/// Every `Mesh` of `AMANATSU_DIR` (default `lib/chara`) decodes with
/// indices inside the vertex range.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn meshes_decode() {
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let sub = std::env::var("AMANATSU_DIR").unwrap_or_else(|_| "lib/chara".into());
    let mut files = Vec::new();
    walk(&root.join(sub), &mut files);
    let (mut ok, mut shapes, mut skinned, mut failures) = (0, 0, 0, Vec::new());
    for path in files {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let bundle = Bundle::open(source).unwrap();
        let resolve = |p: &str| bundle.node(crysirror_unity::unity6::stream_node(p)).map(|n| bundle.reader(n));
        for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
            let file = SerializedFile::open(bundle.reader(node)).unwrap();
            for info in file.objects.iter().filter(|o| o.class_id == 43) {
                let f = file.decode(info).unwrap();
                match crysirror_unity::unity6::mesh(&f, &resolve) {
                    Ok(m) => {
                        let n = m.positions.len() as u32;
                        let bad = m.submeshes.iter().any(|s| m.indices[s.first_index as usize..(s.first_index + s.index_count) as usize].iter().any(|i| i + s.base_vertex >= n));
                        if bad || m.positions.is_empty() {
                            failures.push(format!("{} {}: indices outside {n} vertices", path.display(), m.name));
                        } else {
                            ok += 1;
                            shapes += m.blend_shapes.len();
                            skinned += usize::from(!m.weights.is_empty());
                        }
                    }
                    Err(e) => failures.push(format!("{} {}: {e:#}", path.display(), f.string("m_Name").unwrap_or(""))),
                }
            }
        }
    }
    eprintln!("{ok} meshes ({skinned} skinned, {shapes} blend shapes)");
    for f in failures.iter().take(10) {
        eprintln!("{f}");
    }
    assert!(failures.is_empty(), "{} failures", failures.len());
}

/// Lays out `AMANATSU_LEVEL` (default `level2`, the title) at 1920×1080 and
/// rasterizes its images (nearest sampling, text as outlines) to
/// `AMANATSU_OUT` for inspection.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn layout_level_png() {
    use crysirror_unity::{managed::Value, scene6::Scene6, ugui, unity6, PPtr, TreeRegistry};
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let data = root.join("AmanatsuLocation_Data");
    let level = std::env::var("AMANATSU_LEVEL").unwrap_or_else(|_| "level2".into());
    let mut registry = TreeRegistry::default();
    let mut files = Vec::new();
    walk(&root.join("lib"), &mut files);
    walk(&data.join("StreamingAssets"), &mut files);
    for path in files {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let Ok(bundle) = Bundle::open(source) else { continue };
        for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
            registry.add(&SerializedFile::open(bundle.reader(node)).unwrap());
        }
    }
    let open = |n: &str| -> Option<Arc<SerializedFile>> {
        let f = std::fs::File::open(data.join(n)).ok()?;
        Some(Arc::new(SerializedFile::open(Arc::new(File(Mutex::new(f))) as Arc<dyn AssetReader>).ok()?))
    };
    let file = open(&level).unwrap();
    let script = |p: PPtr| crysirror_unity::scene6::resolve_script(&file, p, &registry, &open).unwrap_or_default();
    let scene = Scene6::load(&file, &registry, &script).unwrap();
    // Sprite → (texture file, texture path id, rect in texture px, border, ppu).
    let sprite = |p: PPtr| -> Option<(Arc<SerializedFile>, PPtr, [f32; 4], [f32; 4], f32)> {
        if p.is_null() {
            return None;
        }
        let src = if p.file_id == 0 { file.clone() } else { open(file.external(p.file_id).ok()?)? };
        let f = src.decode_using(src.object(p.path_id)?, &registry).ok()?;
        let atlas_src = std::cell::RefCell::new(src.clone());
        let load_atlas = |a: PPtr| -> Option<crysirror_unity::managed::Fields> {
            let asrc = if a.file_id == 0 { src.clone() } else { open(src.external(a.file_id).ok()?)? };
            let f = asrc.decode_using(asrc.object(a.path_id)?, &registry).ok();
            *atlas_src.borrow_mut() = asrc;
            f
        };
        let (tex, r, in_atlas) = unity6::sprite_render(&f, &load_atlas)?;
        let src = if in_atlas { atlas_src.borrow().clone() } else { src };
        let b = match f.get("m_Border") { Some(Value::Struct(s)) => ["x", "y", "z", "w"].map(|k| s.float(k).unwrap_or(0.) as f32), _ => [0.; 4] };
        Some((src, tex, r, b, f.float("m_PixelsToUnits").unwrap_or(100.) as f32))
    };
    let info = |p: PPtr| sprite(p).map(|s| (s.3, s.4));
    let size = |p: PPtr| sprite(p).map(|s| ([s.2[2], s.2[3]], s.4));
    let measure = |t: &str, _f: PPtr, px: f32| [t.chars().map(|c| if c.is_ascii() { px * 0.55 } else { px }).sum::<f32>(), px * 1.25];
    let svc = ugui::Services { sprite_info: &info, sprite_size: &size, measure: &measure };
    let mut states: std::collections::HashMap<usize, ugui::NodeState> = Default::default();
    for hide in std::env::var("AMANATSU_HIDE").unwrap_or_default().split(',').filter(|s| !s.is_empty()) {
        if let Some(n) = scene.find(hide) {
            states.entry(n).or_default().active = Some(false);
        }
    }
    let layout = ugui::layout(&scene, &states, 1920., 1080., &svc);
    if std::env::var_os("AMANATSU_ITEMS").is_some() {
        for item in &layout.items {
            let r = item.rect;
            let what = match &item.paint { ugui::Paint::Image { sprite, color, .. } => format!("image {sprite:?} {color:?}"), ugui::Paint::Text { text, .. } => format!("text {text:?}") };
            println!("{} [{:.0},{:.0},{:.0},{:.0}] {what}", scene.path(item.node), r.x0, r.y0, r.x1, r.y1);
        }
    }
    if std::env::var_os("AMANATSU_FONTS").is_some() {
        let mut seen = std::collections::BTreeSet::new();
        for item in &layout.items {
            if let ugui::Paint::Text { font, .. } = &item.paint {
                if !seen.insert(*font) {
                    continue;
                }
                // Also the first fallback of the asset.
                if let Some(fb) = file.external(font.file_id).ok().and_then(|n| open(n)) {
                    for pid in [480i64, 481] {
                        if let Some(Ok(f)) = fb.object(pid).map(|i| fb.decode_using(i, &registry)) {
                            eprintln!("fallback #{pid}: {} pop {:?} source {:?} atlas {:?}", f.string("m_Name").unwrap_or(""), f.int("m_AtlasPopulationMode"), f.get("m_SourceFontFile"), f.get("m_AtlasTextures"));
                            if let Some(crysirror_unity::managed::Value::PPtr(p)) = f.get("m_SourceFontFile") {
                                if let Some(Ok(ff)) = fb.object(p.path_id).map(|i| fb.decode_using(i, &registry)) {
                                    eprintln!("  font {} keys {:?} data {}", ff.string("m_Name").unwrap_or(""), ff.0.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), match ff.get("m_FontData") { Some(crysirror_unity::managed::Value::Bytes(b)) => b.len(), Some(crysirror_unity::managed::Value::Array(a)) => a.len(), _ => 0 });
                                }
                            }
                        }
                    }
                }
                let src = if font.file_id == 0 { Some(file.clone()) } else { file.external(font.file_id).ok().and_then(|n| open(n)) };
                let Some(src) = src else { eprintln!("font {font:?} file missing"); continue };
                let f = src.object(font.path_id).map(|i| src.decode_using(i, &registry));
                match f {
                    Some(Ok(f)) => eprintln!("font {font:?} in {}: {} keys {:?} pop {:?} source {:?} atlas {:?} fallback {:?}", file.external(font.file_id).unwrap_or("self"), f.string("m_Name").unwrap_or(""), f.0.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(), f.int("m_AtlasPopulationMode"), f.get("m_SourceFontFile"), f.get("m_AtlasTextures"), f.get("m_FallbackFontAssetTable")),
                    other => eprintln!("font {font:?}: {:?}", other.map(|r| r.err().map(|e| e.to_string()))),
                }
            }
        }
    }
    let (w, h) = (1920usize, 1080usize);
    let mut canvas = vec![0u8; w * h * 4];
    let mut cache: std::collections::HashMap<(String, i64), Arc<crysirror_image::ImageSurface>> = Default::default();
    for item in &layout.items {
        eprintln!("{} {:?} {:?}", scene.path(item.node), item.rect, match &item.paint { ugui::Paint::Text { text, .. } => text.chars().take(20).collect::<String>(), ugui::Paint::Image { sprite: sp, color, .. } => format!("sprite {sp:?} resolved {} color {color:?}", sprite(*sp).is_some()) });
        let (color, img) = match &item.paint {
            ugui::Paint::Image { sprite: sp, color, .. } => {
                let img = sprite(*sp).and_then(|(src, tex, r, _, _)| {
                    let tsrc = if tex.file_id == 0 { src.clone() } else { open(src.external(tex.file_id).ok()?)? };
                    let key = (format!("{:p}", Arc::as_ptr(&tsrc)), tex.path_id);
                    let surf = match cache.get(&key) {
                        Some(s) => s.clone(),
                        None => {
                            let tf = match tsrc.object(tex.path_id).map(|i| tsrc.decode_using(i, &registry)) {
                                Some(Ok(f)) => f,
                                other => {
                                    eprintln!("texture object {tex:?}: {:?}", other.map(|r| r.err().map(|e| e.to_string())));
                                    return None;
                                }
                            };
                            let resolve = |p: &str| -> Option<Arc<dyn AssetReader>> { Some(Arc::new(File(Mutex::new(std::fs::File::open(data.join(p)).ok()?)))) };
                            let s = match unity6::texture_2d(&tf, &resolve) {
                                Ok(s) => Arc::new(s),
                                Err(e) => {
                                    eprintln!("texture {}: {e:#}", tf.string("m_Name").unwrap_or(""));
                                    return None;
                                }
                            };
                            cache.insert(key, s.clone());
                            s
                        }
                    };
                    Some((surf, r))
                });
                (*color, img)
            }
            ugui::Paint::Text { color, .. } => (*color, None),
        };
        let rr = item.rect;
        for y in rr.y0.max(0.) as usize..(rr.y1.min(h as f32)) as usize {
            for x in rr.x0.max(0.) as usize..(rr.x1.min(w as f32)) as usize {
                let (mut r, mut g, mut b, mut a) = (color[0], color[1], color[2], color[3]);
                match &img {
                    Some((surf, tr)) => {
                        let u = (x as f32 + 0.5 - rr.x0) / rr.width();
                        let v = (y as f32 + 0.5 - rr.y0) / rr.height();
                        let tx = (tr[0] + u * tr[2]) as usize;
                        let ty = surf.height as usize - 1 - ((tr[1] + (1. - v) * tr[3]) as usize).min(surf.height as usize - 1);
                        let i = (ty * surf.width as usize + tx.min(surf.width as usize - 1)) * 4;
                        let p = &surf.rgba[i..i + 4];
                        r *= p[0] as f32 / 255.;
                        g *= p[1] as f32 / 255.;
                        b *= p[2] as f32 / 255.;
                        a *= p[3] as f32 / 255.;
                    }
                    None if matches!(item.paint, ugui::Paint::Text { .. }) => {
                        let edge = x as f32 - rr.x0 < 2. || rr.x1 - x as f32 <= 2. || y as f32 - rr.y0 < 2. || rr.y1 - y as f32 <= 2.;
                        if !edge {
                            continue;
                        }
                    }
                    None => {}
                }
                let i = (y * w + x) * 4;
                for (k, c) in [r, g, b].iter().enumerate() {
                    canvas[i + k] = (canvas[i + k] as f32 * (1. - a) + c * 255. * a) as u8;
                }
                canvas[i + 3] = 255;
            }
        }
    }
    let out = std::env::var("AMANATSU_OUT").unwrap_or_else(|_| "/tmp/amanatsu_layout.png".into());
    image::save_buffer(&out, &canvas, w as u32, h as u32, image::ColorType::Rgba8).unwrap();
    eprintln!("saved {out}");
}

/// Regenerates `data/managed_trees.json`: type trees of the player-file
/// MonoBehaviours whose scripts no bundle carries, derived from metadata-only
/// assemblies (`AMANATSU_ASSEMBLIES`, Cpp2IL `--use-processor
/// attributeanalyzer,attributeinjector` output). Structure only (field
/// names and types), keyed by the 16-byte script id.
#[test]
#[ignore = "needs AMANATSU_ROOT and AMANATSU_ASSEMBLIES"]
fn generate_managed_trees() {
    use crysirror_unity::{scene6, TreeRegistry};
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let data = root.join("AmanatsuLocation_Data");
    let mut registry = TreeRegistry::default();
    let mut files = Vec::new();
    walk(&root.join("lib"), &mut files);
    walk(&data.join("StreamingAssets"), &mut files);
    for path in files {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let Ok(bundle) = Bundle::open(source) else { continue };
        for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
            registry.add(&SerializedFile::open(bundle.reader(node)).unwrap());
        }
    }
    let open = |n: &str| -> Option<Arc<SerializedFile>> {
        let f = std::fs::File::open(data.join(n)).ok()?;
        Some(Arc::new(SerializedFile::open(Arc::new(File(Mutex::new(f))) as Arc<dyn AssetReader>).ok()?))
    };
    let mut names: Vec<String> = std::fs::read_dir(&data).unwrap().filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| n.starts_with("level") || n.ends_with(".assets")).collect();
    names.sort();
    let mut wanted: BTreeMap<String, ([u8; 16], String)> = BTreeMap::new();
    for name in &names {
        let file = open(name).unwrap();
        for info in file.objects.iter().filter(|o| o.class_id == 114) {
            let ty = &file.types[info.type_index.unwrap()];
            let Some(id) = ty.script_id else { continue };
            if registry.lookup(114, Some(id)).is_some() {
                continue;
            }
            let b = file.bytes(info).unwrap();
            let p = crysirror_unity::PPtr { file_id: i32::from_le_bytes(b[16..20].try_into().unwrap()), path_id: i64::from_le_bytes(b[20..28].try_into().unwrap()) };
            let Ok(class) = scene6::resolve_script(&file, p, &registry, &open) else { continue };
            if !class.is_empty() {
                wanted.insert(id.iter().map(|b| format!("{b:02x}")).collect(), (id, class));
            }
        }
    }
    let dir = PathBuf::from(std::env::var_os("AMANATSU_ASSEMBLIES").unwrap());
    let mut asms = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()) {
        if e.path().extension().is_some_and(|x| x == "dll") {
            asms.push(crysirror_dotnet::Assembly::parse(std::fs::read(e.path()).unwrap()).unwrap());
        }
    }
    let g = crysirror_unity::managed_gen::Generator::new(asms, &registry).unwrap();
    let mut out = serde_json::Map::new();
    for (key, (_, class)) in &wanted {
        match g.mono_behaviour(class) {
            Ok(tree) => {
                out.insert(key.clone(), serde_json::json!({"class": class, "tree": tree}));
            }
            Err(e) => eprintln!("skip {class}: {e:#}"),
        }
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/managed_trees.json");
    std::fs::write(&path, serde_json::to_string(&serde_json::json!({"schema": 1, "unity": "6000.0.65f1", "scripts": out})).unwrap()).unwrap();
    eprintln!("{} trees → {}", out.len(), path.display());
}

/// MonoBehaviours of a level with their fields (`AMANATSU_LEVEL`,
/// `AMANATSU_SCRIPT` substring filter); object pointers in the level are
/// printed as node paths.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn print_scripts() {
    use crysirror_unity::{managed::Value, scene6::Scene6, PPtr, TreeRegistry};
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let data = root.join("AmanatsuLocation_Data");
    let level = std::env::var("AMANATSU_LEVEL").unwrap_or_else(|_| "level2".into());
    let filter = std::env::var("AMANATSU_SCRIPT").unwrap_or_default();
    let mut registry = TreeRegistry::default();
    let mut files = Vec::new();
    walk(&root.join("lib"), &mut files);
    walk(&data.join("StreamingAssets"), &mut files);
    for path in files {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let Ok(bundle) = Bundle::open(source) else { continue };
        for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
            registry.add(&SerializedFile::open(bundle.reader(node)).unwrap());
        }
    }
    let doc: serde_json::Value = serde_json::from_str(include_str!("../data/managed_trees.json")).unwrap();
    for (k, v) in doc["scripts"].as_object().unwrap() {
        let id: [u8; 16] = std::array::from_fn(|i| u8::from_str_radix(&k[i * 2..i * 2 + 2], 16).unwrap());
        if registry.lookup(crysirror_unity::class_id::MONO_BEHAVIOUR, Some(id)).is_none() {
            registry.insert_script(id, serde_json::from_value(v["tree"].clone()).unwrap());
        }
    }
    let open = |n: &str| -> Option<Arc<SerializedFile>> {
        let f = std::fs::File::open(data.join(n)).ok()?;
        Some(Arc::new(SerializedFile::open(Arc::new(File(Mutex::new(f))) as Arc<dyn AssetReader>).ok()?))
    };
    let file = open(&level).unwrap();
    let script = |p: PPtr| crysirror_unity::scene6::resolve_script(&file, p, &registry, &open).unwrap_or_default();
    let scene = Scene6::load(&file, &registry, &script).unwrap();
    if let Ok(spec) = std::env::var("AMANATSU_OBJ") {
        // file_id:path_id of the level's pointer space.
        let (fid, pid) = spec.split_once(':').unwrap();
        let (fid, pid): (i32, i64) = (fid.parse().unwrap(), pid.parse().unwrap());
        let src = if fid == 0 { file.clone() } else { open(file.external(fid).unwrap()).unwrap() };
        let info = src.object(pid).unwrap();
        println!("class {} in {}", info.class_id, if fid == 0 { level.clone() } else { file.external(fid).unwrap().to_owned() });
        let f = src.decode_using(info, &registry).unwrap();
        for (k, v) in f.0.iter() {
            let s = format!("{v:?}");
            println!("  {k} = {}", s.chars().take(std::env::var("AMANATSU_WIDTH").ok().and_then(|w| w.parse().ok()).unwrap_or(600)).collect::<String>());
        }
        return;
    }
    if std::env::var_os("AMANATSU_TREE").is_some() {
        for (i, n) in scene.nodes.iter().enumerate() {
            let group = n.components.iter().find(|c| c.class_id == 225).and_then(|c| c.fields.float("m_Alpha"));
            println!("{}{} {}", if n.active { "" } else { "(inactive) " }, scene.path(i), group.map(|a| format!("group alpha {a}")).unwrap_or_default());
        }
    }
    for info in file.objects.iter().filter(|o| o.class_id == 114) {
        if let Err(e) = file.decode_using(info, &registry) {
            println!("UNDECODED {} {e:#}", info.path_id);
        }
    }
    let mut owner = std::collections::HashMap::new();
    for (i, n) in scene.nodes.iter().enumerate() {
        owner.insert(n.game_object, i);
        owner.insert(n.transform, i);
        for c in &n.components {
            owner.insert(c.path_id, i);
        }
    }
    fn show(v: &Value, scene: &Scene6, owner: &std::collections::HashMap<i64, usize>, depth: usize) -> String {
        match v {
            Value::PPtr(p) if p.file_id == 0 && p.path_id != 0 => match owner.get(&p.path_id) {
                Some(&n) => format!("@{}", scene.path(n)),
                None => format!("@{}", p.path_id),
            },
            Value::Array(a) if depth < 4 => format!("[{}]", a.iter().take(24).map(|x| show(x, scene, owner, depth + 1)).collect::<Vec<_>>().join(", ")),
            Value::Struct(s) if depth < 4 => format!("{{{}}}", s.0.iter().map(|(k, x)| format!("{k}: {}", show(x, scene, owner, depth + 1))).collect::<Vec<_>>().join(", ")),
            other => {
                let s = format!("{other:?}");
                s.chars().take(160).collect()
            }
        }
    }
    for (i, n) in scene.nodes.iter().enumerate() {
        for c in &n.components {
            let class: Option<i32> = std::env::var("AMANATSU_CLASS").ok().and_then(|v| v.parse().ok());
            let wanted = match class {
                Some(id) => c.class_id == id,
                None => !c.script.is_empty() && c.script.contains(filter.as_str()),
            };
            if !wanted {
                continue;
            }
            println!("{} [{}]", scene.path(i), c.script);
            for (k, v) in c.fields.0.iter() {
                if matches!(k.as_str(), "m_GameObject" | "m_Enabled" | "m_Script" | "m_Name" | "m_EditorHideFlags" | "m_EditorClassIdentifier") {
                    continue;
                }
                println!("    {k} = {}", show(v, &scene, &owner, 0));
            }
        }
    }
}

/// Every Animator of the build scenes: controller and clips decode, and the
/// bindings census (uGUI-mapped vs unsupported) per level.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn animators_decode() {
    use crysirror_unity::{animator, scene6::Scene6, PPtr, TreeRegistry};
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let data = root.join("AmanatsuLocation_Data");
    let mut registry = TreeRegistry::default();
    let mut files = Vec::new();
    walk(&root.join("lib"), &mut files);
    walk(&data.join("StreamingAssets"), &mut files);
    for path in files {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let Ok(bundle) = Bundle::open(source) else { continue };
        for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
            registry.add(&SerializedFile::open(bundle.reader(node)).unwrap());
        }
    }
    let doc: serde_json::Value = serde_json::from_str(include_str!("../data/managed_trees.json")).unwrap();
    for (k, v) in doc["scripts"].as_object().unwrap() {
        let id: [u8; 16] = std::array::from_fn(|i| u8::from_str_radix(&k[i * 2..i * 2 + 2], 16).unwrap());
        if registry.lookup(crysirror_unity::class_id::MONO_BEHAVIOUR, Some(id)).is_none() {
            registry.insert_script(id, serde_json::from_value(v["tree"].clone()).unwrap());
        }
    }
    let open = |n: &str| -> Option<Arc<SerializedFile>> {
        let f = std::fs::File::open(data.join(n)).ok()?;
        Some(Arc::new(SerializedFile::open(Arc::new(File(Mutex::new(f))) as Arc<dyn AssetReader>).ok()?))
    };
    let mut failures = 0;
    for level in 0..6 {
        let name = format!("level{level}");
        let file = open(&name).unwrap();
        let script = |p: PPtr| crysirror_unity::scene6::resolve_script(&file, p, &registry, &open).unwrap_or_default();
        let scene = Scene6::load(&file, &registry, &script).unwrap();
        let (mut animators, mut mapped, mut unsupported) = (0, 0, std::collections::BTreeMap::<(i32, u32), usize>::new());
        for (i, n) in scene.nodes.iter().enumerate() {
            for c in n.components.iter().filter(|c| c.class_id == 95) {
                animators += 1;
                let p = c.fields.pptr("m_Controller").unwrap_or_default();
                if p.is_null() {
                    continue;
                }
                let src = if p.file_id == 0 { file.clone() } else { open(file.external(p.file_id).unwrap()).unwrap() };
                let ctl = match src.decode_using(src.object(p.path_id).unwrap(), &registry).map_err(anyhow::Error::from).and_then(|f| animator::Controller::decode(&f)) {
                    Ok(c) => c,
                    Err(e) => {
                        failures += 1;
                        println!("{name} {}: controller {e:#}", scene.path(i));
                        continue;
                    }
                };
                for cp in &ctl.clips {
                    let csrc = if cp.file_id == 0 { src.clone() } else { open(src.external(cp.file_id).unwrap()).unwrap() };
                    let clip = match csrc.decode_using(csrc.object(cp.path_id).unwrap(), &registry).map_err(anyhow::Error::from).and_then(|f| animator::Clip::decode(&f)) {
                        Ok(c) => c,
                        Err(e) => {
                            failures += 1;
                            println!("{name} {} {}: clip {e:#}", scene.path(i), ctl.name);
                            continue;
                        }
                    };
                    for b in &clip.bindings {
                        let resolved = animator::resolve_path(&scene, i, b.path).is_some();
                        if resolved && (0..b.curves).all(|k| b.ugui(k).is_some()) {
                            mapped += 1;
                        } else {
                            *unsupported.entry((b.class_id, if resolved { b.attribute } else { b.path })).or_default() += 1;
                            if std::env::var_os("AMANATSU_VERBOSE").is_some() {
                                println!("  {} {} {}: class {} attr {} path {} resolved {resolved}", scene.path(i), ctl.name, clip.name, b.class_id, b.attribute, b.path);
                            }
                        }
                    }
                }
            }
        }
        println!("{name}: {animators} animators, {mapped} mapped bindings, unsupported {unsupported:?}");
    }
    assert_eq!(failures, 0);
}

/// MonoBehaviours of script class `AMANATSU_FIND` (substring) in the player
/// files and every bundle, with their fields.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn find_scripts() {
    let want = std::env::var("AMANATSU_FIND").unwrap();
    let mut u = unity();
    let mut keys: Vec<String> = ["resources.assets", "globalgamemanagers.assets"].iter().map(|s| s.to_string()).collect();
    keys.extend((0..6).map(|i| format!("level{i}")));
    keys.extend((0..6).map(|i| format!("sharedassets{i}.assets")));
    keys.extend(u.bundle_files());
    let key_filter = std::env::var("AMANATSU_KEY").unwrap_or_default();
    for key in keys.into_iter().filter(|k| k.contains(key_filter.as_str())) {
        let Ok(file) = u.file(&key) else { continue };
        for info in file.objects.iter().filter(|o| o.class_id == 114) {
            let Ok(f) = u.decode(&key, info.path_id) else { continue };
            let class = u.script_name(&key, f.pptr("m_Script").unwrap_or_default());
            if !class.contains(want.as_str()) {
                continue;
            }
            println!("{key} #{} [{class}] {}", info.path_id, f.string("m_Name").unwrap_or(""));
            for (k, v) in f.0.iter().filter(|(k, _)| !matches!(k.as_str(), "m_GameObject" | "m_Enabled" | "m_Script" | "m_Name" | "m_EditorHideFlags" | "m_EditorClassIdentifier")) {
                println!("    {k} = {}", format!("{v:?}").chars().take(std::env::var("AMANATSU_WIDTH").ok().and_then(|w| w.parse().ok()).unwrap_or(300)).collect::<String>());
            }
        }
    }
}

/// Persistent click/value-changed calls of every Selectable in a level
/// (`AMANATSU_LEVEL`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn print_events() {
    let level = std::env::var("AMANATSU_LEVEL").unwrap_or_else(|_| "level2".into());
    let mut u = unity();
    let scene = u.scene(&level).unwrap();
    let mut owner = std::collections::HashMap::new();
    for (i, n) in scene.nodes.iter().enumerate() {
        owner.insert(n.game_object, i);
        for c in &n.components {
            owner.insert(c.path_id, i);
        }
    }
    for (i, n) in scene.nodes.iter().enumerate() {
        for c in &n.components {
            for field in ["m_OnClick", "m_OnValueChanged", "onValueChanged"] {
                let Some(ev) = c.fields.fields(field) else { continue };
                for call in crysirror_unity::events::persistent_calls(ev) {
                    let target = owner.get(&call.target.path_id).map(|&t| scene.path(t)).unwrap_or_else(|| format!("{:?}", call.target));
                    println!("{} {field}: {}.{}({:?}) on {target}", scene.path(i), call.type_name, call.method, call.argument);
                }
            }
        }
    }
}

/// AudioClip census of `lib/<AMANATSU_DIR>` (default `sd`): load type,
/// compression format, channels/frequency, resource sizes; FSB5 header mode.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn audio_census() {
    let mut u = unity();
    let sub = std::env::var("AMANATSU_DIR").unwrap_or_else(|_| "lib/sd".into());
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    let mut shown = 0;
    for key in u.bundle_files().into_iter().filter(|k| k.contains(&format!("{sub}/"))) {
        let Ok(file) = u.file(&key) else { continue };
        for info in file.objects.iter().filter(|o| o.class_id == 83) {
            let f = match u.decode(&key, info.path_id) {
                Ok(f) => f,
                Err(e) => {
                    *census.entry(format!("decode error {e}")).or_default() += 1;
                    continue;
                }
            };
            let res = f.fields("m_Resource").cloned().unwrap_or_default();
            let k = format!("load {} fmt {} ch {} hz {}", f.int("m_LoadType").unwrap_or(-1), f.int("m_CompressionFormat").unwrap_or(-1), f.int("m_Channels").unwrap_or(-1), f.int("m_Frequency").unwrap_or(-1));
            *census.entry(k).or_default() += 1;
            {
                let src = res.string("m_Source").unwrap_or("");
                let off = res.int("m_Offset").unwrap_or(0) as u64;
                let (bundle, _) = key.trim_start_matches("bundle:").split_once('#').unwrap();
                let b = u.bundle(bundle).unwrap();
                if let Some(n) = b.node(src.rsplit('/').next().unwrap()) {
                    let mut h = vec![0u8; 0x100];
                    let got = b.reader(n).read_at(off, &mut h).unwrap_or(0);
                    h.truncate(got);
                    // FSB5 v1: 0x3C header, then the sample's u64 mode bits and chunks.
                    let le = |o: usize| u32::from_le_bytes(h[o..o + 4].try_into().unwrap());
                    if h.len() >= 0x48 && &h[..4] == b"FSB5" && le(24) == 15 {
                        let bits = u64::from_le_bytes(h[0x3C..0x44].try_into().unwrap());
                        let mut o = 0x44;
                        let mut more = bits & 1 != 0;
                        while more && o + 4 <= h.len() {
                            let c = le(o);
                            more = c & 1 != 0;
                            let size = ((c >> 1) & 0xFF_FFFF) as usize;
                            let ty = c >> 25;
                            if ty == 11 {
                                *census.entry(format!("vorbis setup crc {:08x}", le(o + 4))).or_default() += 1;
                            }
                            o += 4 + size;
                        }
                    }
                }
            }
            if shown < 3 {
                shown += 1;
                println!("{key} {} {:?}", f.string("m_Name").unwrap_or(""), res);
                let src = res.string("m_Source").unwrap_or("");
                let off = res.int("m_Offset").unwrap_or(0) as u64;
                let size = res.int("m_Size").unwrap_or(0) as usize;
                let (bundle, _) = key.trim_start_matches("bundle:").split_once('#').unwrap();
                let b = u.bundle(bundle).unwrap();
                let node = src.rsplit('/').next().unwrap();
                if let Some(n) = b.node(node) {
                    let r = b.reader(n);
                    let mut head = vec![0u8; size.min(96)];
                    r.read_at(off, &mut head).unwrap();
                    println!("  head {:02x?}", &head[..head.len().min(96)]);
                }
            }
        }
    }
    for (k, n) in census {
        println!("{n:6} {k}");
    }
}

/// Every AudioClip of `lib/<AMANATSU_DIR>` (default `sd`) converts from
/// FSB5 and decodes through the shared decoder; the Vorbis setup headers
/// come from the installed `UnityPlayer.dll`. `AMANATSU_LIMIT` caps clips.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn audio_decodes() {
    use crysirror_audio::fsb5;
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let player = std::fs::read(root.join("UnityPlayer.dll")).unwrap();
    let setups: std::cell::RefCell<std::collections::HashMap<u32, Option<Vec<u8>>>> = Default::default();
    let mut u = unity();
    let sub = std::env::var("AMANATSU_DIR").unwrap_or_else(|_| "lib/sd".into());
    let limit: usize = std::env::var("AMANATSU_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let (mut ok, mut failed, mut frames_total) = (0usize, 0usize, 0u64);
    'outer: for key in u.bundle_files().into_iter().filter(|k| k.contains(&format!("{sub}/"))) {
        let Ok(file) = u.file(&key) else { continue };
        for info in file.objects.iter().filter(|o| o.class_id == 83) {
            if ok + failed >= limit {
                break 'outer;
            }
            let f = u.decode(&key, info.path_id).unwrap();
            let name = f.string("m_Name").unwrap_or("").to_owned();
            let bank = u.audio_clip_bytes(&key, &f).unwrap();
            let stream = fsb5::to_stream(&bank, &|crc| setups.borrow_mut().entry(crc).or_insert_with(|| fsb5::find_vorbis_setup(&player, crc)).clone());
            // Decode to the end (streamed, as long music plays).
            let result = stream.and_then(|s| {
                let ext = if s.starts_with(b"OggS") { "ogg" } else { "wav" };
                let mut src = crysirror_audio::decoder::open_memory(s.into(), Some(ext))?;
                let mut block = [[0f32; 2]; 4096];
                let mut n = 0u64;
                loop {
                    let got = src.read(&mut block)?;
                    if got == 0 {
                        break;
                    }
                    n += got as u64;
                }
                Ok(n)
            });
            match result {
                Ok(n) => {
                    ok += 1;
                    frames_total += n;
                }
                Err(e) => {
                    failed += 1;
                    if failed < 10 {
                        println!("{key} {name}: {e:#}");
                    }
                }
            }
        }
    }
    println!("{ok} clips decoded ({frames_total} frames), {failed} failed; setups {:?}", setups.borrow().iter().map(|(k, v)| (format!("{k:08x}"), v.as_ref().map(|v| v.len()))).collect::<Vec<_>>());
    assert_eq!(failed, 0);
}

/// Raw object bytes (hex) of TextAssets in bundles matching `AMANATSU_KEY`.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn hex_text_assets() {
    let mut u = unity();
    let want = std::env::var("AMANATSU_KEY").unwrap();
    let n: usize = std::env::var("AMANATSU_WIDTH").ok().and_then(|w| w.parse().ok()).unwrap_or(256);
    for key in u.bundle_files().into_iter().filter(|k| k.contains(want.as_str())) {
        let file = u.file(&key).unwrap();
        for info in file.objects.iter().filter(|o| o.class_id == 49) {
            let b = file.bytes(info).unwrap();
            println!("{key} #{} {} bytes", info.path_id, b.len());
            for (i, c) in b[..b.len().min(n)].chunks(32).enumerate() {
                println!("  {:04x}: {}  {}", i * 32, c.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" "), c.iter().map(|&x| if (32..127).contains(&x) { x as char } else { '.' }).collect::<String>());
            }
        }
    }
}

/// Material census of `lib/<AMANATSU_DIR>` bundles: shader name → material
/// count, with the texture/float/colour property names of each shader.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn material_census() {
    let mut u = unity();
    let sub = std::env::var("AMANATSU_DIR").unwrap_or_else(|_| "lib/ma".into());
    let mut shaders: BTreeMap<String, (usize, std::collections::BTreeSet<String>)> = BTreeMap::new();
    let mut shader_names: std::collections::HashMap<(String, i64), String> = Default::default();
    for key in u.bundle_files().into_iter().filter(|k| k.contains(&format!("{sub}/"))) {
        let Ok(file) = u.file(&key) else { continue };
        for info in file.objects.iter().filter(|o| o.class_id == 21) {
            let Ok(f) = u.decode(&key, info.path_id) else { continue };
            let sp = f.pptr("m_Shader").unwrap_or_default();
            let name = match u.target(&key, sp) {
                Ok(t) => shader_names.entry(t.clone()).or_insert_with(|| u.decode(&t.0, t.1).ok().and_then(|s| s.fields("m_ParsedForm").and_then(|p| p.string("m_Name")).map(str::to_owned).or_else(|| s.string("m_Name").map(str::to_owned))).unwrap_or_else(|| format!("?{}", t.0.rsplit('/').next().unwrap_or("")))).clone(),
                Err(_) => "<unresolved>".into(),
            };
            let e = shaders.entry(name).or_default();
            e.0 += 1;
            if let Some(props) = f.fields("m_SavedProperties") {
                for list in ["m_TexEnvs", "m_Floats", "m_Colors"] {
                    for p in props.array(list).unwrap_or(&[]) {
                        if let crysirror_unity::managed::Value::Struct(p) = p {
                            if let Some(n) = p.string("first") {
                                e.1.insert(format!("{}:{n}", &list[2..5]));
                            }
                        }
                    }
                }
            }
        }
    }
    for (name, (n, props)) in &shaders {
        println!("{n:6} {name}  [{} props] {}", props.len(), props.iter().take(std::env::var("AMANATSU_PROPS").ok().and_then(|w| w.parse().ok()).unwrap_or(40)).cloned().collect::<Vec<_>>().join(" "));
    }
}

/// Nodes, `m_Container` entries and object classes of the bundles named in
/// `AMANATSU_BUNDLES` (comma-separated `lib/…` paths).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn bundle_census() {
    let mut u = unity();
    for path in std::env::var("AMANATSU_BUNDLES").unwrap().split(',') {
        let b = u.bundle(path).unwrap();
        println!("== {path}");
        let nodes: Vec<(String, u32)> = b.nodes.iter().map(|n| (n.path.clone(), n.flags)).collect();
        for (node, flags) in nodes {
            println!("  node {node} flags {flags}");
            if flags & 4 == 0 {
                continue;
            }
            let key = format!("bundle:{path}#{node}");
            let f = u.file(&key).unwrap();
            let mut classes = std::collections::BTreeMap::<i32, usize>::new();
            for o in &f.objects {
                *classes.entry(o.class_id).or_default() += 1;
            }
            println!("    classes {classes:?}");
            if let Some(ab) = f.objects.iter().find(|o| o.class_id == crysirror_unity::class_id::ASSET_BUNDLE) {
                let fields = f.decode_using(ab, &u.registry).unwrap();
                for e in fields.array("m_Container").unwrap_or(&[]).iter().take(60) {
                    let crysirror_unity::managed::Value::Struct(e) = e else { continue };
                    let p = e.fields("second").and_then(|s| s.pptr("asset")).unwrap_or_default();
                    let class = f.object(p.path_id).map(|o| o.class_id).unwrap_or(-1);
                    println!("    {} -> {}:{} class {class}", e.string("first").unwrap_or(""), p.file_id, p.path_id);
                }
            }
        }
    }
}

/// The first `AMANATSU_COUNT` (default 3) objects of class `AMANATSU_CLASS`
/// in file `AMANATSU_KEY` (a `bundle:<path>#<node>` key).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn dump_class() {
    let mut u = unity();
    let key = std::env::var("AMANATSU_KEY").unwrap();
    let class: i32 = std::env::var("AMANATSU_CLASS").unwrap().parse().unwrap();
    let count: usize = std::env::var("AMANATSU_COUNT").ok().and_then(|c| c.parse().ok()).unwrap_or(3);
    let width: usize = std::env::var("AMANATSU_WIDTH").ok().and_then(|c| c.parse().ok()).unwrap_or(300);
    let file = u.file(&key).unwrap();
    for info in file.objects.iter().filter(|o| o.class_id == class).take(count) {
        println!("-- {} ", info.path_id);
        match u.decode(&key, info.path_id) {
            Ok(f) => {
                for (k, v) in f.0.iter() {
                    println!("  {k} = {}", format!("{v:?}").chars().take(width).collect::<String>());
                }
            }
            Err(e) => println!("  undecoded: {e:#}"),
        }
    }
}

/// Builds the GLB of a scene file (`AMANATSU_KEY`, default map000) through
/// the shared importer; reports shaders, counts and the build time, and
/// writes `AMANATSU_OUT` when set.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn scene_glb() {
    let mut u = unity();
    let key = std::env::var("AMANATSU_KEY").unwrap_or_else(|_| "bundle:lib/ma/pre/sc/000_08/00.unity3d#BuildPlayer-map000".into());
    let t = std::time::Instant::now();
    let scene = u.scene(&key).unwrap();
    println!("scene: {} nodes, {} roots in {:?}", scene.nodes.len(), scene.roots.len(), t.elapsed());
    let t = std::time::Instant::now();
    let roots = scene.roots.clone();
    let (glb, report) = crysirror_unity::world6::scene_glb(&scene, &key, &roots, &mut u, &crysirror_unity::world6::GlbOptions { max_texture: std::env::var("AMANATSU_MAXTEX").ok().and_then(|v| v.parse().ok()).unwrap_or(1024), texture_budget: crysirror_amanatsu::map::MAP_TEXTURE_BUDGET, spring_unit: 10., morph_targets: false }, "test").unwrap();
    println!("glb chunks {:?} bytes in {:?}", glb.iter().map(|g| g.len()).collect::<Vec<_>>(), t.elapsed());
    println!("texture bytes {} renderers {} skipped {} lod-culled {} materials {} textures {}", report.texture_bytes, report.renderers, report.skipped_renderers, report.lod_culled, report.materials, report.textures);
    for (s, (support, n)) in &report.shaders {
        println!("  {n:4} {support:12} {s}");
    }
    for e in report.errors.iter().take(20) {
        println!("  error {e}");
    }
    println!("{} errors", report.errors.len());
    for l in crysirror_unity::world6::scene_lights(&scene).iter().filter(|l| l.kind == 1) {
        println!("  directional {} {:?} x{} dir {:?}", scene.path(l.node), l.color, l.intensity, l.direction);
    }
    if let Ok(out) = std::env::var("AMANATSU_OUT") {
        for (i, g) in glb.iter().enumerate() {
            std::fs::write(format!("{out}{i}.glb"), g).unwrap();
        }
    }
}

/// Rows of a `TableData` TextAsset by bundle and asset name
/// (`AMANATSU_BUNDLES` = `lib/…`, `AMANATSU_ASSET` = name stem).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn print_table() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLES").unwrap();
    let asset = std::env::var("AMANATSU_ASSET").unwrap();
    let (k, id) = u.bundle_asset(&bundle, &asset, 49).unwrap().expect("asset");
    let t = crysirror_amanatsu::tables::Table::parse(&u.text_asset(&k, id).unwrap()).unwrap();
    println!("keys {:?}", t.keys);
    for r in &t.rows {
        println!("  {r:?}");
    }
}

/// Node paths (and MonoBehaviour classes) of a bundle file's hierarchy
/// (`AMANATSU_KEY`), limited to roots named `AMANATSU_ROOTNAME` if set.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn prefab_tree() {
    let mut u = unity();
    let key = std::env::var("AMANATSU_KEY").unwrap();
    let want = std::env::var("AMANATSU_ROOTNAME").ok();
    let scene = u.scene(&key).unwrap();
    for (i, n) in scene.nodes.iter().enumerate() {
        let mut r = i;
        while let Some(p) = scene.nodes[r].parent {
            r = p;
        }
        if want.as_deref().is_some_and(|w| !scene.nodes[r].name.eq_ignore_ascii_case(w)) {
            continue;
        }
        let scripts: Vec<&str> = n.components.iter().filter(|c| !c.script.is_empty()).map(|c| c.script.as_str()).collect();
        println!("{}{} {}", if n.active { "" } else { "(inactive) " }, scene.path(i), scripts.join(","));
        // `AMANATSU_FIELDS`: the fields of components whose script contains it.
        if let Ok(want) = std::env::var("AMANATSU_FIELDS") {
            if let Some(r) = n.xform.rect {
                println!("    rect {r:?}");
            }
            for c in n.components.iter().filter(|c| want.split(',').any(|w| c.script.contains(w))) {
                println!("    [{}] {:?}", c.script, c.fields.0.iter().filter(|(k, _)| !matches!(k.as_str(), "m_GameObject" | "m_Script" | "m_Name" | "m_EditorHideFlags")).map(|(k, v)| format!("{k}={v:?}")).collect::<Vec<_>>());
            }
        }
    }
}

/// A character card's blocks as MessagePack trees (`AMANATSU_CARD`, a path
/// under the game root; `AMANATSU_BLOCK` limits to one block).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn print_card() {
    let root = PathBuf::from(std::env::var_os("AMANATSU_ROOT").unwrap());
    let card = crysirror_amanatsu::card::Card::parse(&std::fs::read(root.join(std::env::var("AMANATSU_CARD").unwrap())).unwrap()).unwrap();
    println!("product {} version {}", card.product, card.version);
    let want = std::env::var("AMANATSU_BLOCK").ok();
    fn show(v: &crysirror_unity::msgpack::Value, depth: usize, out: &mut String) {
        use crysirror_unity::msgpack::Value as V;
        let pad = "  ".repeat(depth);
        match v {
            V::Map(m) => {
                for (k, x) in m {
                    let key = match k { V::Str(s) => s.clone(), other => format!("{other:?}") };
                    match x {
                        V::Map(_) => { out.push_str(&format!("{pad}{key}:\n")); show(x, depth + 1, out); }
                        V::Array(a) if a.iter().any(|e| matches!(e, V::Map(_))) => { out.push_str(&format!("{pad}{key}: [{}]\n", a.len())); for e in a.iter().take(3) { show(e, depth + 1, out); out.push_str(&format!("{pad}  --\n")); } }
                        V::Bin(b) => { out.push_str(&format!("{pad}{key}: bin {} -> \n", b.len())); if let Ok(inner) = crysirror_unity::msgpack::decode(b) { show(&inner, depth + 1, out); } }
                        V::Array(a) if a.iter().any(|e| matches!(e, V::Bin(_))) => { out.push_str(&format!("{pad}{key}: [{} bins]\n", a.len())); }
                        other => out.push_str(&format!("{pad}{key} = {}\n", format!("{other:?}").chars().take(200).collect::<String>())),
                    }
                }
            }
            V::Array(a) if a.iter().all(|e| matches!(e, V::Bin(_))) => {
                for (i, e) in a.iter().enumerate() {
                    let V::Bin(b) = e else { continue };
                    match crysirror_amanatsu::card::Card::container(b) {
                        Ok((marker, c)) => {
                            out.push_str(&format!("{pad}[{i}] {marker}\n"));
                            for (n, _, bytes) in &c.blocks {
                                out.push_str(&format!("{pad}  -- {n} {} bytes\n", bytes.len()));
                                if let Ok(v) = crysirror_unity::msgpack::decode(bytes) { show(&v, depth + 2, out); }
                            }
                        }
                        Err(e) => out.push_str(&format!("{pad}[{i}] bin {} ({e:#})\n", b.len())),
                    }
                    if i >= 0 { break; }
                }
            }
            other => out.push_str(&format!("{pad}{}\n", format!("{other:?}").chars().take(400).collect::<String>())),
        }
    }
    for (name, ver, bytes) in &card.blocks {
        if want.as_deref().is_some_and(|w| w != name) {
            continue;
        }
        println!("== {name} {ver} {} bytes", bytes.len());
        let mut out = String::new();
        if name == "Custom" {
            for (i, v) in card.parts(name).unwrap().iter().enumerate() {
                out.push_str(&format!(" part {i}\n"));
                show(v, 1, &mut out);
            }
            println!("{out}");
            continue;
        }
        match crysirror_unity::msgpack::decode(bytes) {
            Ok(v) => show(&v, 1, &mut out),
            Err(e) => out = format!("  undecoded: {e:#}"),
        }
        println!("{out}");
    }
}

/// GLB of a bundle prefab (`AMANATSU_BUNDLES` = one `lib/…` path,
/// `AMANATSU_ASSET` = prefab name) through the shared importer; writes
/// `AMANATSU_OUT<i>.glb` when set.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn prefab_glb() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLES").unwrap();
    let asset = std::env::var("AMANATSU_ASSET").unwrap();
    let level = crysirror_amanatsu::level::Level::prefab(&mut u, &bundle, &asset).unwrap();
    let (glb, report) = crysirror_unity::world6::scene_glb(&level.scene, &level.key, &[0], &mut u, &Default::default(), "test").unwrap();
    println!("glb chunks {:?} renderers {} skipped {} materials {} textures {} ({} bytes)", glb.iter().map(|g| g.len()).collect::<Vec<_>>(), report.renderers, report.skipped_renderers, report.materials, report.textures, report.texture_bytes);
    for (s, (support, n)) in &report.shaders {
        println!("  {n:4} {support:12} {s}");
    }
    for e in &report.errors {
        println!("  error {e}");
    }
    if let Ok(out) = std::env::var("AMANATSU_OUT") {
        for (i, g) in glb.iter().enumerate() {
            std::fs::write(format!("{out}{i}.glb"), g).unwrap();
        }
    }
}

/// Renderers of a bundle prefab with their materials' shaders and texture
/// slots (`AMANATSU_BUNDLES`, `AMANATSU_ASSET`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn prefab_renderers() {
    use crysirror_unity::world6::{shader_name, SceneAssets, UnityMaterial};
    let mut u = unity();
    let level = crysirror_amanatsu::level::Level::prefab(&mut u, &std::env::var("AMANATSU_BUNDLES").unwrap(), &std::env::var("AMANATSU_ASSET").unwrap()).unwrap();
    for (i, n) in level.scene.nodes.iter().enumerate() {
        for c in n.components.iter().filter(|c| c.class_id == 23 || c.class_id == 137) {
            let mut mats = Vec::new();
            for p in c.fields.array("m_Materials").unwrap_or(&[]) {
                let crysirror_unity::managed::Value::PPtr(p) = p else { continue };
                let Some((mf, _, _, f)) = SceneAssets::object(&mut u, &level.key, *p).unwrap() else { continue };
                let shader = SceneAssets::object(&mut u, &mf, f.pptr("m_Shader").unwrap_or_default()).unwrap().map(|s| shader_name(&s.3)).unwrap_or_default();
                let m = UnityMaterial::decode(&f, shader);
                let tex: Vec<&String> = m.textures.iter().filter(|(_, t)| !t.texture.is_null()).map(|(k, _)| k).collect();
                mats.push(format!("{} <{}> {:?} queue {}", m.name, m.shader, tex, m.render_queue));
            }
            println!("{}{} enabled {:?}: {}", if n.active { "" } else { "(inactive) " }, level.scene.path(i), c.fields.get("m_Enabled"), mats.join(" | "));
        }
    }
}

/// The default heroine assembled from `AMANATSU_CARD` (default
/// `AL_F_00`): parts log, and the GLB's materials (name, alpha mode, base
/// texture) with nodes in `AMANATSU_HIDE` (comma-separated names) hidden.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn chara_glb() {
    let mut u = unity();
    let card_path = std::env::var("AMANATSU_CARD").unwrap_or_else(|_| "DefaultData/0/chara/female/AL_F_00.png".into());
    let card = crysirror_amanatsu::card::Card::parse(&u.read_all(&card_path).unwrap()).unwrap();
    let lists = crysirror_amanatsu::chara::Lists::load(&mut u).unwrap();
    let female = std::env::var("AMANATSU_MALE").is_err();
    let mut human = crysirror_amanatsu::chara::Human::from_card(&mut u, &lists, &card, female).unwrap();
    for l in &human.log {
        println!("{l}");
    }
    for name in std::env::var("AMANATSU_HIDE").unwrap_or_default().split(',').filter(|s| !s.is_empty()) {
        for n in human.scene.nodes.iter_mut().filter(|n| n.name == name) {
            n.active = false;
        }
    }
    let mut assets = crysirror_amanatsu::chara::CharaAssets { unity: &mut u, human: &human, light: 1., ambient: 1. };
    let options = crysirror_unity::world6::GlbOptions { morph_targets: true, ..Default::default() };
    let (glb, report) = crysirror_unity::world6::scene_glb(&human.scene, &human.key, &[0], &mut assets, &options, "test").unwrap();
    println!("chunks {:?} renderers {} errors {:?}", glb.iter().map(|g| g.len()).collect::<Vec<_>>(), report.renderers, report.errors);
    for g in &glb {
        let len = u32::from_le_bytes(g[12..16].try_into().unwrap()) as usize;
        let json: serde_json::Value = serde_json::from_slice(&g[20..20 + len]).unwrap();
        for m in json["meshes"].as_array().into_iter().flatten() {
            let names = m["extras"]["targetNames"].as_array().map_or(0, Vec::len);
            let targets = m["primitives"][0]["targets"].as_array().map_or(0, Vec::len);
            if names + targets > 0 {
                println!("  mesh {} targets {targets} names {names}", m["name"]);
            }
        }
    }
    for g in &glb {
        let len = u32::from_le_bytes(g[12..16].try_into().unwrap()) as usize;
        let json: serde_json::Value = serde_json::from_slice(&g[20..20 + len]).unwrap();
        let springs = json["extras"]["crysirror_springs"].as_array().cloned().unwrap_or_default();
        println!("  {} spring chains: {}", springs.len(), springs.iter().take(16).map(|c| format!("{}(g {} d {} s {} lim {} max {} col {})", json["nodes"][c["root"].as_u64().unwrap() as usize]["name"], c["gravity"][1], c["damping"], c["stiffness"], c["angle_limit"], c["max_distance"], c["colliders"].as_array().map_or(0, Vec::len))).collect::<Vec<_>>().join(" "));
        for m in json["materials"].as_array().unwrap() {
            let e = &m["extras"]["crysirror_material"]["material"];
            println!("  {} alpha {} tex {} color {}", m["name"], e["alpha_mode"], e["base_color_texture"], e["base_color_factor"]);
        }
    }
    if let Ok(out) = std::env::var("AMANATSU_OUT") {
        for (i, g) in glb.iter().enumerate() {
            std::fs::write(format!("{out}{i}.glb"), g).unwrap();
        }
    }
}

/// The female animator controller (`ani/mot/1`): layers/states and the
/// binding classes of the first clips, paths resolved on the body prefab.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn chara_controller() {
    use crysirror_unity::animator::{self, Clip, Controller};
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap_or_else(|_| "lib/ani/mot/1/000_00.unity3d".into());
    let asset = std::env::var("AMANATSU_ASSET").unwrap_or_else(|_| "f_00".into());
    let (k, id) = u.bundle_asset(&bundle, &asset, 91).unwrap().unwrap();
    let ctl = Controller::decode(&u.decode(&k, id).unwrap()).unwrap();
    println!("layers {} clips {}", ctl.layers.len(), ctl.clips.len());
    for (li, l) in ctl.layers.iter().enumerate() {
        println!("layer {li}: {} states, default {:?}", l.states.len(), l.default_state);
        for s in l.states.iter().take(std::env::var("AMANATSU_COUNT").ok().and_then(|c| c.parse().ok()).unwrap_or(10)) {
            println!("  {} clip {:?}", ctl.names.get(&s.name_id).cloned().unwrap_or_default(), s.clip);
        }
    }
    let body = crysirror_amanatsu::level::Level::prefab(&mut u, "lib/chara/body/body_00.unity3d", "p_cf_al_body_00").unwrap();
    for cp in ctl.clips.iter().take(2) {
        let Ok((f, cid)) = u.target(&k, *cp) else { continue };
        let clip = Clip::decode(&u.decode(&f, cid).unwrap()).unwrap();
        let mut classes = std::collections::BTreeMap::<(i32, u32), usize>::new();
        let mut resolved = 0;
        for b in &clip.bindings {
            *classes.entry((b.class_id, b.attribute)).or_default() += 1;
            if animator::resolve_path(&body.scene, 0, b.path).is_some() {
                resolved += 1;
            }
        }
        println!("clip {} {:.2}s loop {} bindings {} resolved {} classes {:?}", clip.name, clip.duration(), clip.looping, clip.bindings.len(), resolved, classes);
    }
}

/// Commands of the scenarios in `AMANATSU_BUNDLES` (one `lib/adv/scene/…`
/// bundle) whose name contains `AMANATSU_ASSET` (default all, first 3).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn print_scenarios() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLES").unwrap();
    let want = std::env::var("AMANATSU_ASSET").unwrap_or_default();
    let max: usize = std::env::var("AMANATSU_COUNT").ok().and_then(|c| c.parse().ok()).unwrap_or(3);
    let mut shown = 0;
    for key in u.bundle_files().into_iter().filter(|k| k.contains(&format!("bundle:{bundle}#"))) {
        let file = u.file(&key).unwrap();
        let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == 114).map(|o| o.path_id).collect();
        for id in ids {
            let f = u.decode(&key, id).unwrap();
            let name = f.string("m_Name").unwrap_or("").to_owned();
            if !name.contains(&want) || shown >= max {
                continue;
            }
            let Some(crysirror_unity::managed::Value::Bytes(b)) = f.get("_bytes") else { continue };
            shown += 1;
            println!("== {name}");
            for (i, c) in crysirror_amanatsu::adv::parse_scenario(b).unwrap().iter().enumerate() {
                println!("  {i:3} {}{} {:?}", c.name(), if c.multi { "*" } else { "" }, c.args);
            }
        }
    }
}

/// MagicaCloth components and colliders under `lib/chara`: cloth types,
/// collider lists and collider component classes.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn magica_census() {
    let mut u = unity();
    let mut types = std::collections::BTreeMap::<String, usize>::new();
    for key in u.bundle_files().into_iter().filter(|k| k.contains("lib/chara/")) {
        let Ok(file) = u.file(&key) else { continue };
        let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == 114).map(|o| o.path_id).collect();
        for id in ids {
            let Ok(f) = u.decode(&key, id) else { continue };
            let class = u.script_name(&key, f.pptr("m_Script").unwrap_or_default());
            if !class.starts_with("MagicaCloth2") {
                continue;
            }
            let mut k = class.clone();
            if let Some(d) = f.fields("serializeData") {
                let colliders = d.fields("colliderCollisionConstraint").and_then(|c| c.array("colliderList")).map_or(0, |l| l.len());
                k = format!("{class} type {} colliders {}", d.int("clothType").unwrap_or(-1), colliders.min(1));
                if d.int("clothType").unwrap_or(-1) != 1 || colliders > 0 {
                    println!("{} {k} {}", key.rsplit('#').next().unwrap_or(""), key.split('#').next().unwrap_or(""));
                }
            } else {
                println!("{} {class} {}", key.split('#').next().unwrap_or(""), f.fields("center").map(|_| "").unwrap_or(""));
            }
            *types.entry(k).or_default() += 1;
        }
    }
    for (k, n) in types {
        println!("{n:6} {k}");
    }
}

/// Writes the skin material textures of the card's body (`mt_*` list rows:
/// sunburn, two overlays, detail) to `AMANATSU_OUT` for inspection.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn skin_textures() {
    let mut u = unity();
    let lists = crysirror_amanatsu::chara::Lists::load(&mut u).unwrap();
    let out = PathBuf::from(std::env::var("AMANATSU_OUT").unwrap());
    for id in 0..2 {
        let r = lists.row("bo_body", id).unwrap();
        println!("bo_body {id}: {:?}", (0..130).filter_map(|k| r.get(k).map(|v| (k, v.to_owned()))).collect::<Vec<_>>());
    }
    for (list, id, tex) in [("mt_eye", 1, 23), ("mt_eyepipil", 4, 18), ("mt_eyepipil", 1, 18), ("mt_eye_gradation", 1, 23), ("mt_eye_gradation", 1, 10), ("mt_eye_hi_up", 4, 14), ("mt_eye_hi_up", 3, 14), ("mt_eye_hi_up", 2, 14), ("mt_eye_white", 0, 24), ("mt_sunburnUp", 1, 60), ("mt_sunburnDown", 2, 60), ("mt_nip", 0, 40), ("mt_underhair", 4, 61), ("mt_body_detail", 0, 42), ("mt_body_detail", 0, 28), ("mt_body_shine", 3, 124), ("bo_body", 1, 34)] {
        let Some(row) = lists.row(list, id) else {
            println!("{list} {id}: no row");
            continue;
        };
        let Some((b, n)) = row.asset(if list == "bo_body" { crysirror_amanatsu::chara::key::MAIN_TEX_AB } else { crysirror_amanatsu::chara::key::MAIN_AB }, tex) else {
            println!("{list} {id}: no key {tex}");
            continue;
        };
        let Some((k, pid)) = u.bundle_asset(&b, &n, crysirror_unity::class_id::TEXTURE_2D).unwrap() else {
            println!("{list} {id}: {b} {n} missing");
            continue;
        };
        let img = crysirror_unity::world6::SceneAssets::texture(&mut u, &k, pid).unwrap();
        let ext = |c: usize| img.rgba.chunks_exact(4).map(|p| p[c]).fold((255u8, 0u8), |(a, b), v| (a.min(v), b.max(v)));
        println!("{list} {id} key {tex}: {n} {}x{} rgba {:?}", img.width, img.height, [ext(0), ext(1), ext(2), ext(3)]);
        image::RgbaImage::from_raw(img.width, img.height, img.rgba).unwrap().save(out.join(format!("{list}_{id}_{tex}.png"))).unwrap();
    }
}

/// Prints the fields of the components whose script ends with
/// `AMANATSU_SCRIPT` in a prefab (`AMANATSU_BUNDLE`, `AMANATSU_ASSET`),
/// with node paths for object pointers resolved within the prefab.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn prefab_component() {
    let mut u = unity();
    let p = crysirror_amanatsu::level::Level::prefab(&mut u, &std::env::var("AMANATSU_BUNDLE").unwrap(), &std::env::var("AMANATSU_ASSET").unwrap()).unwrap();
    let script = std::env::var("AMANATSU_SCRIPT").unwrap();
    let by_go: std::collections::HashMap<i64, usize> = p.scene.nodes.iter().enumerate().map(|(i, n)| (n.game_object, i)).collect();
    let by_tr: std::collections::HashMap<i64, usize> = p.scene.nodes.iter().enumerate().map(|(i, n)| (n.transform, i)).collect();
    let name = |id: i64| by_go.get(&id).or(by_tr.get(&id)).map(|&i| p.scene.path(i)).unwrap_or_else(|| id.to_string());
    for (i, n) in p.scene.nodes.iter().enumerate() {
        for c in n.components.iter().filter(|c| c.script.ends_with(&script)) {
            println!("== {} {}", p.scene.path(i), c.script);
            for (k, v) in &c.fields.0 {
                let s = match v {
                    crysirror_unity::managed::Value::PPtr(q) => name(q.path_id),
                    crysirror_unity::managed::Value::Array(a) if a.iter().all(|e| matches!(e, crysirror_unity::managed::Value::PPtr(_))) => {
                        format!("[{}]", a.iter().map(|e| if let crysirror_unity::managed::Value::PPtr(q) = e { name(q.path_id) } else { String::new() }).collect::<Vec<_>>().join(", "))
                    }
                    v => format!("{v:?}").chars().take(std::env::var("AMANATSU_WIDTH").ok().and_then(|w| w.parse().ok()).unwrap_or(200)).collect(),
                };
                println!("  {k} = {s}");
            }
        }
    }
}

/// Prints a named MonoBehaviour asset of a bundle (`AMANATSU_BUNDLE`,
/// `AMANATSU_ASSET`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn print_asset() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap_or_else(|_| {
        // Default: map 0's light setting bundle.
        let infos = crysirror_amanatsu::map::map_infos(&mut u).unwrap();
        println!("map 0 light {:?}", infos[0].light);
        format!("lib/{}", infos[0].light.0)
    });
    let (k, id) = u.bundle_asset(&bundle, &std::env::var("AMANATSU_ASSET").unwrap(), 114).unwrap().expect("asset");
    for (name, v) in u.decode(&k, id).unwrap().0 {
        println!("  {name} = {}", format!("{v:?}").chars().take(std::env::var("AMANATSU_WIDTH").ok().and_then(|w| w.parse().ok()).unwrap_or(400)).collect::<String>());
    }
}

/// Decodes a `.sav` file (`AMANATSU_SAVE`) and prints its contents.
#[test]
#[ignore = "needs AMANATSU_SAVE"]
fn print_save() {
    let b = std::fs::read(std::env::var("AMANATSU_SAVE").unwrap()).unwrap();
    let f = crysirror_amanatsu::save::SaveFile::decode(&b).unwrap();
    println!("{:?}", f.save);
    println!("player {:?} card {} bytes", f.player.actor.chara_file_name, f.player.actor.human.as_ref().map_or(0, Vec::len));
    for n in &f.npcs {
        println!("npc {:?} id {} map {} {:?} {:?} {:?} card {} bytes", n.actor.chara_file_name, n.actor.unique_id, n.action.map_id, n.parameter, n.count, n.count_extra, n.actor.human.as_ref().map_or(0, Vec::len));
    }
    assert_eq!(f.encode().unwrap(), b);
}

/// Prints the massage tables and hit volumes.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn oil_data() {
    let mut u = unity();
    let d = crysirror_amanatsu::oil::OilData::load(&mut u).unwrap();
    for s in &d.satisfaction {
        println!("sat {:?}", s);
    }
    println!("sum max {}", d.satisfaction.iter().map(|s| s.max).sum::<f32>());
    for p in &d.poses {
        println!("pose {:?}", p);
    }
    for v in &d.volumes {
        println!("hit {} bone {} {:?}..{:?}", v.tag, v.bone, v.min, v.max);
    }
}

/// Lists `map_info` and the action points of a map (`AMANATSU_MAP`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn map_points() {
    let mut u = unity();
    let infos = crysirror_amanatsu::map::map_infos(&mut u).unwrap();
    for i in &infos {
        println!("map {} {} scene {:?} point {:?}", i.id, i.name, i.scene, i.point);
    }
    let id: i32 = std::env::var("AMANATSU_MAP").ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    if let Some(info) = infos.iter().find(|i| i.id == id) {
        let p = crysirror_amanatsu::level::Level::prefab(&mut u, &format!("lib/{}", info.point.0), &info.point.1).unwrap();
        for (i, n) in p.scene.nodes.iter().enumerate() {
            let scripts: Vec<&str> = n.components.iter().filter(|c| !c.script.is_empty()).map(|c| c.script.as_str()).collect();
            if !scripts.is_empty() {
                println!("  {} {:?} {}", p.scene.path(i), crysirror_unity::world6::world_matrix(&p.scene, i).w_axis.truncate(), scripts.join(","));
            }
        }
    }
}

/// Writes `AMANATSU_SAVE` with every heroine's favourability gauge full,
/// one massage and map 0 (for testing the favourability short movie).
#[test]
#[ignore = "needs AMANATSU_SAVE"]
fn patch_save_favorability() {
    let path = std::env::var("AMANATSU_SAVE").unwrap();
    let mut f = crysirror_amanatsu::save::SaveFile::decode(&std::fs::read(&path).unwrap()).unwrap();
    for n in &mut f.npcs {
        n.parameter.favorability.point = std::env::var("AMANATSU_FAV_POINT").ok().and_then(|v| v.parse().ok()).unwrap_or(100);
        n.parameter.favorability.lv = std::env::var("AMANATSU_FAV_LV").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        n.parameter.late_point = 20;
        n.parameter.cost = 5;
        n.count.massage = 1;
        n.action.map_id = 0;
        if std::env::var("AMANATSU_IS_H").is_ok() {
            n.count.h = 1;
            n.count_extra.is_h = true;
            n.count_extra.is_night_h = false;
        }
        if let Some(c) = std::env::var("AMANATSU_NIGHT_EVENTS").ok().and_then(|v| v.parse().ok()) {
            n.count.night_events = c;
        }
    }
    f.save.time_zone = std::env::var("AMANATSU_ZONE").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    std::fs::write(&path, f.encode().unwrap()).unwrap();
}

/// Lists the `characustom` table names.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn list_tables() {
    let mut u = unity();
    let lists = crysirror_amanatsu::chara::Lists::load(&mut u).unwrap();
    let mut names = lists.names();
    names.sort();
    println!("{}", names.join(" "));
}

/// Prints the H postures.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn h_postures() {
    let mut u = unity();
    for p in crysirror_amanatsu::h::load_postures(&mut u).unwrap() {
        println!("{} {} {} cam {} mode {} kind {} pos {:?} cat {} release {} state {} weak {} {:?}", p.file, p.id, p.name, p.camera, p.mode, p.kind, p.positions, p.category, p.release_phase, p.state, p.weakness, p.charas.iter().map(|c| format!("{}:{}", c.info.bundle, c.info.asset)).collect::<Vec<_>>());
    }
}

/// Prints `PropertyData` members (`AMANATSU_FIELD`: one top-level member).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn property_data() {
    let mut u = unity();
    let pd = u.find_script_object("StreamingAssets/aa", "AL.PropertyData").unwrap().expect("AL.PropertyData");
    let field = std::env::var("AMANATSU_FIELD").unwrap_or_else(|_| "_explore".into());
    println!("{field} = {:?}", pd.get(&field));
}

/// Lists a bundle's objects (`AMANATSU_BUNDLE`): class, path id, name;
/// TextAssets print their first bytes.
#[test]
#[ignore = "needs AMANATSU_ROOT and AMANATSU_BUNDLE"]
fn bundle_objects() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let objects: Vec<(i32, i64)> = u.file(&key).unwrap().objects.iter().map(|o| (o.class_id, o.path_id)).collect();
        for (class, id) in objects {
            let name = u.decode(&key, id).ok().and_then(|f| f.string("m_Name").map(str::to_owned)).unwrap_or_default();
            print!("{key} {class} {id} {name}");
            if class == 1 {
                print!(" layer {}", u.decode(&key, id).ok().and_then(|f| f.int("m_Layer")).unwrap_or(-1));
            }
            if class == 49 {
                let b = u.text_asset(&key, id).unwrap_or_default();
                match crysirror_amanatsu::tables::Table::parse(&b) {
                    Ok(t) if std::env::var("AMANATSU_FILTER").is_ok_and(|f| name.contains(&f)) => print!(" keys {:?} rows {:?}", t.keys, t.rows),
                    Ok(t) => print!(" table {} rows", t.rows.len()),
                    Err(_) => {
                        print!(" {} bytes {:02x?}", b.len(), &b[..b.len().min(48)]);
                        if let Ok(dir) = std::env::var("AMANATSU_DUMP") {
                            std::fs::write(format!("{dir}/{name}.bin"), &b).unwrap();
                        }
                    }
                }
            }
            println!();
        }
    }
}

/// Finds `TableData` lists with a column key (`AMANATSU_KEY_ID`) among the
/// list bundles and prints them.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn tables_with_key() {
    let mut u = unity();
    let want: i32 = std::env::var("AMANATSU_KEY_ID").unwrap().parse().unwrap();
    let keys: Vec<String> = u.bundle_files().into_iter().filter(|k| (k.contains("/lis/") || k.contains("/list/") || k.contains("al_lib")) && !k.ends_with(".resS")).collect();
    for key in keys {
        let Ok(file) = u.file(&key) else { continue };
        let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == 49).map(|o| o.path_id).collect();
        for id in ids {
            let Ok(b) = u.text_asset(&key, id) else { continue };
            if let Ok(t) = crysirror_amanatsu::tables::Table::parse(&b) {
                if t.keys.contains(&want) {
                    let name = u.decode(&key, id).ok().and_then(|f| f.string("m_Name").map(str::to_owned)).unwrap_or_default();
                    println!("{key} {name} keys {:?} rows {:?}", t.keys, t.rows);
                }
            }
        }
    }
}

/// MonoBehaviours of a bundle (`AMANATSU_BUNDLE`) by script class, with
/// the members of those whose name contains `AMANATSU_ASSET`.
#[test]
#[ignore = "needs AMANATSU_ROOT and AMANATSU_BUNDLE"]
fn bundle_behaviours() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    let want = std::env::var("AMANATSU_ASSET").ok();
    let width: usize = std::env::var("AMANATSU_WIDTH").ok().and_then(|w| w.parse().ok()).unwrap_or(300);
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let ids: Vec<i64> = u.file(&key).unwrap().objects.iter().filter(|o| o.class_id == 114).map(|o| o.path_id).collect();
        for id in ids {
            let Ok(f) = u.decode(&key, id) else { continue };
            let script = f.pptr("m_Script").map(|p| u.script_name(&key, p)).unwrap_or_default();
            *counts.entry(script.clone()).or_default() += 1;
            if std::env::var("AMANATSU_SCRIPT").is_ok_and(|w| script.contains(&w)) {
                println!("-- {id} {script} {}", f.string("m_Name").unwrap_or(""));
            }
            let name = f.string("m_Name").unwrap_or("").to_owned();
            let by_id = std::env::var("AMANATSU_IDS").is_ok_and(|v| v.split(',').any(|x| x.trim() == id.to_string()));
            if by_id || want.as_deref().is_some_and(|w| name.contains(w)) {
                println!("== {id} {script} {name}");
                for (k, v) in &f.0 {
                    println!("  {k} = {}", format!("{v:?}").chars().take(width).collect::<String>());
                }
            }
        }
    }
    for (k, n) in counts {
        println!("{n:5} {k}");
    }
}

/// A controller state's clip bindings (`AMANATSU_BUNDLE`, `AMANATSU_ASSET`
/// controller, `AMANATSU_STATE`) with samples at 0 s and the end.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn state_clip_bindings() {
    use crysirror_unity::animator::{path_hash, Clip, Controller};
    let mut u = unity();
    let (k, id) = u.bundle_asset(&std::env::var("AMANATSU_BUNDLE").unwrap(), &std::env::var("AMANATSU_ASSET").unwrap(), 91).unwrap().unwrap();
    let ctl = Controller::decode(&u.decode(&k, id).unwrap()).unwrap();
    let si = ctl.find_state(0, &std::env::var("AMANATSU_STATE").unwrap()).expect("state");
    let first = ctl.layers[0].states[si].clip.unwrap();
    for i in 0..3 {
        if let Some(&p) = ctl.clips.get(first + i) {
            let (f, cid) = u.target(&k, p).unwrap();
            println!("clip[{}] {:?}", first + i, u.decode(&f, cid).unwrap().string("m_Name"));
        }
    }
    let pick: usize = std::env::var("AMANATSU_PICK").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let cp = ctl.clips[first + pick];
    let (f, cid) = u.target(&k, cp).unwrap();
    let clip = Clip::decode(&u.decode(&f, cid).unwrap()).unwrap();
    println!("clip {} {}..{} loop {}", clip.name, clip.start, clip.stop, clip.looping);
    let names = ["", "Camera", "Main Camera", "camera", "cam", "CameraBase", "Cam", "MainCamera"];
    for b in &clip.bindings {
        let n = names.iter().find(|n| path_hash(n) == b.path);
        println!("  path {:#x} {:?} attr {:#x} class {} curves {}", b.path, n, b.attribute, b.class_id, b.curves);
    }
    println!("t0 {:?}", clip.sample(clip.start));
    println!("t1 {:?}", clip.sample(clip.stop));
}

/// Prints a short movie's tracks (`AMANATSU_SM`, default 20).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn short_movie() {
    let mut u = unity();
    let id: i32 = std::env::var("AMANATSU_SM").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let m = crysirror_amanatsu::sm::Movie::load(&mut u, id).unwrap();
    println!("stm_{id:02}: {:.2} s", m.duration);
    println!("heroine {:?}", m.heroine);
    println!("camera {:?}", m.camera);
    println!("fades {:?}", m.fades);
    println!("ses {:?}", m.ses);
    let clips = m.camera_clips(&mut u, 1).unwrap();
    for (k, c) in &clips {
        println!("{k}: {:?} .. {:?}", crysirror_amanatsu::sm::camera_pose(c, c.start), crysirror_amanatsu::sm::camera_pose(c, c.stop));
    }
    println!("personal 0 LV1 → {:?}", crysirror_amanatsu::sm::personal_movie(&mut u, 0, 1).unwrap());
}

/// The H breath voices of personality 0.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn h_breaths() {
    let mut u = unity();
    let list = crysirror_amanatsu::hvoice::load(&mut u, 0).unwrap();
    assert_eq!(list.len(), 375);
    let groups: std::collections::BTreeSet<i32> = list.iter().map(|b| b.group).collect();
    println!("{} breaths, groups {groups:?}, first {:?}", list.len(), list[0]);
    for b in list.iter().filter(|b| b.stage == 0).step_by(25) {
        println!("group {} {}: {:?}", b.group, b.asset, b.faces);
    }
    // Face parameters are sane: openness in [0, 1], tears 0..=3, cheek in
    // [0, 1].
    assert!(list.iter().flat_map(|b| &b.faces).all(|f| (0. ..=1.).contains(&f.open_eye) && f.open_mouth.0 <= f.open_mouth.1 + 1e-4));
    assert!(list.iter().flat_map(|b| &b.faces).all(|f| (0..=3).contains(&f.tear) && (0. ..=1.).contains(&f.cheek)));
    let tears: std::collections::BTreeMap<i32, usize> = list.iter().flat_map(|b| &b.faces).fold(Default::default(), |mut m, f| {
        *m.entry(f.tear).or_default() += 1;
        m
    });
    let cheeks: Vec<i32> = list.iter().flat_map(|b| &b.faces).map(|f| (f.cheek * 10.).round() as i32).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    println!("tears {tears:?}, cheek×10 {cheeks:?}, groups with tears {:?}", list.iter().filter(|b| b.faces.iter().any(|f| f.tear > 0)).map(|b| b.group).collect::<std::collections::BTreeSet<_>>());
}

/// Blend shapes of the meshes of a prefab (`AMANATSU_BUNDLE`,
/// `AMANATSU_ASSET`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn prefab_blend_shapes() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap_or_else(|_| "lib/chara/fo_head_000_00.unity3d".into());
    let keys: Vec<String> = u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")).collect();
    for key in keys {
        let ids: Vec<i64> = u.file(&key).unwrap().objects.iter().filter(|o| o.class_id == 43).map(|o| o.path_id).collect();
        for id in ids {
            let Ok(m) = u.mesh(&key, id) else { continue };
            if m.blend_shapes.is_empty() {
                continue;
            }
            println!("{} ({} vertices): {} shapes", m.name, m.positions.len(), m.blend_shapes.len());
            for b in m.blend_shapes.iter().take(std::env::var("AMANATSU_COUNT").ok().and_then(|c| c.parse().ok()).unwrap_or(200)) {
                println!("  {} frames {:?}", b.name, b.frames.iter().map(|f| (f.0, f.1.len())).collect::<Vec<_>>());
            }
        }
    }
}

/// Members of the components of a serialized file's nodes (`AMANATSU_KEY`)
/// whose script contains `AMANATSU_SCRIPT`.
#[test]
#[ignore = "needs AMANATSU_ROOT and AMANATSU_KEY"]
fn scene_component() {
    let mut u = unity();
    let scene = u.scene(&std::env::var("AMANATSU_KEY").unwrap()).unwrap();
    let want = std::env::var("AMANATSU_SCRIPT").unwrap();
    let width: usize = std::env::var("AMANATSU_WIDTH").ok().and_then(|w| w.parse().ok()).unwrap_or(400);
    for (i, n) in scene.nodes.iter().enumerate() {
        for c in n.components.iter().filter(|c| c.script.contains(&want)) {
            println!("== {} {} {:?}", scene.path(i), c.script, n.xform);
            for (k, v) in &c.fields.0 {
                println!("  {k} = {}", format!("{v:?}").chars().take(width).collect::<String>());
            }
        }
    }
}

/// The command camera table (`AngleData` prefab): key → entry node.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn command_angles() {
    let mut u = unity();
    let level = crysirror_amanatsu::level::Level::prefab(&mut u, "lib/am/pre/s/000_00.unity3d", "AngleData").unwrap();
    let s = &level.scene;
    let c = s.nodes.iter().flat_map(|n| &n.components).find(|c| c.script == "AL.CinemachineCameraAngleData").unwrap();
    let t = c.fields.fields("_table").unwrap();
    for (k, v) in t.array("_keyData").unwrap().iter().zip(t.array("_valueData").unwrap()) {
        let crysirror_unity::managed::Value::PPtr(p) = v else { continue };
        let n = s.nodes.iter().position(|n| n.components.iter().any(|c| c.path_id == p.path_id)).map(|i| s.path(i)).unwrap_or_default();
        print!("{k:?}={n}  ");
    }
    println!();
    for h in [0.2f32, 0.5, 0.8] {
        println!("id 0 height {h}: {:?}", crysirror_amanatsu::face::command_camera(s, 0, h));
    }
}

/// Directional lights of a map scene (`AMANATSU_MAP`, default 0) in all
/// light groups: path, active, mask, colour, intensity, direction.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn map_directional_lights() {
    let mut u = unity();
    let infos = crysirror_amanatsu::map::map_infos(&mut u).unwrap();
    let id: i32 = std::env::var("AMANATSU_MAP").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let info = infos.iter().find(|i| i.id == id).unwrap();
    let key = u.bundle_files().into_iter().find(|k| k.starts_with(&format!("bundle:lib/{}#", info.scene.0)) && !k.ends_with(".resS")).unwrap();
    let scene = u.scene(&key).unwrap();
    for l in crysirror_unity::world6::scene_lights(&scene).into_iter().filter(|l| l.kind == 1) {
        println!("{} active {} mask {:#x} color {:?} intensity {} dir {:?}", scene.path(l.node), scene.nodes[l.node].active, l.culling_mask, l.color, l.intensity, l.direction);
    }
}

/// Writes the Texture2D assets of a bundle (`AMANATSU_BUNDLE`) whose names
/// contain `AMANATSU_FILTER` as PNG files into `AMANATSU_DUMP`.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn bundle_textures() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    let filter = std::env::var("AMANATSU_FILTER").unwrap_or_default();
    let dir = std::env::var("AMANATSU_DUMP").unwrap();
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let ids: Vec<i64> = u.file(&key).unwrap().objects.iter().filter(|o| o.class_id == crysirror_unity::class_id::TEXTURE_2D).map(|o| o.path_id).collect();
        for id in ids {
            let name = u.decode(&key, id).ok().and_then(|f| f.string("m_Name").map(str::to_owned)).unwrap_or_default();
            if !name.contains(&filter) {
                continue;
            }
            match crysirror_unity::world6::SceneAssets::texture(&mut u, &key, id) {
                Ok(img) => {
                    println!("{name} {}x{}", img.width, img.height);
                    std::fs::write(format!("{dir}/{name}.png"), crysirror_image::encode_png(&img).unwrap()).unwrap();
                }
                Err(e) => println!("{name}: {e}"),
            }
        }
    }
}

/// Materials of a bundle (`AMANATSU_BUNDLE`) whose names contain
/// `AMANATSU_FILTER`: shader, render queue, keywords, floats and colours.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn bundle_materials() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    let filter = std::env::var("AMANATSU_FILTER").unwrap_or_default();
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let ids: Vec<i64> = u.file(&key).unwrap().objects.iter().filter(|o| o.class_id == 21).map(|o| o.path_id).collect();
        for id in ids {
            let f = u.decode(&key, id).unwrap();
            if !f.string("m_Name").unwrap_or("").contains(&filter) {
                continue;
            }
            let shader = u.target(&key, f.pptr("m_Shader").unwrap_or_default()).ok().and_then(|t| u.decode(&t.0, t.1).ok()).and_then(|s| s.fields("m_ParsedForm").and_then(|p| p.string("m_Name")).map(str::to_owned)).unwrap_or_default();
            let m = crysirror_unity::world6::UnityMaterial::decode(&f, shader);
            println!("{} {} queue {} keywords {:?}", m.name, m.shader, m.render_queue, m.keywords);
            let mut floats: Vec<_> = m.floats.iter().collect();
            floats.sort_by(|a, b| a.0.cmp(b.0));
            println!("  floats {floats:?}");
            let mut colors: Vec<_> = m.colors.iter().collect();
            colors.sort_by(|a, b| a.0.cmp(b.0));
            println!("  colors {colors:?}");
            let mut tex: Vec<_> = m.textures.iter().filter(|(_, t)| t.texture.path_id != 0).map(|(k, t)| (k.clone(), u.target(&key, t.texture).ok().and_then(|(f, o)| u.decode(&f, o).ok()).and_then(|f| f.string("m_Name").map(str::to_owned)))).collect();
            tex.sort();
            println!("  textures {tex:?}");
            if let Ok(dir) = std::env::var("AMANATSU_DUMP") {
                for t in m.textures.values().filter(|t| t.texture.path_id != 0) {
                    let Ok((f, o)) = u.target(&key, t.texture) else { continue };
                    let name = u.decode(&f, o).ok().and_then(|f| f.string("m_Name").map(str::to_owned)).unwrap_or_default();
                    if let Ok(img) = crysirror_unity::world6::SceneAssets::texture(&mut u, &f, o) {
                        std::fs::write(format!("{dir}/{name}.png"), crysirror_image::encode_png(&img).unwrap()).unwrap();
                    }
                }
            }
        }
    }
}

/// UV channels of the card character's renderers using a material whose
/// name contains `AMANATSU_FILTER` (default `head`): per channel the
/// bounds and, with `AMANATSU_DUMP`, a plot of channel k over UV0.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn chara_uv_channels() {
    let mut u = unity();
    let card = crysirror_amanatsu::card::Card::parse(&u.read_all("DefaultData/0/chara/female/AL_F_00.png").unwrap()).unwrap();
    let lists = crysirror_amanatsu::chara::Lists::load(&mut u).unwrap();
    let human = crysirror_amanatsu::chara::Human::from_card(&mut u, &lists, &card, true).unwrap();
    let filter = std::env::var("AMANATSU_FILTER").unwrap_or_else(|_| "head".into());
    for n in &human.scene.nodes {
        for c in n.components.iter().filter(|c| c.class_id == 137 || c.class_id == 23) {
            let file = if n.origin == 0 { human.key.clone() } else { human.scene.files[n.origin as usize - 1].clone() };
            let mats: Vec<String> = c.fields.array("m_Materials").unwrap_or(&[]).iter().filter_map(|v| if let crysirror_unity::managed::Value::PPtr(p) = v { Some(*p) } else { None }).filter_map(|p| {
                let (f, o) = u.target(&file, p).ok()?;
                u.decode(&f, o).ok()?.string("m_Name").map(str::to_owned)
            }).collect();
            if !mats.iter().any(|m| m.contains(&filter)) {
                continue;
            }
            let Some(mp) = c.fields.pptr("m_Mesh") else { continue };
            let (mf, mo) = u.target(&file, mp).unwrap();
            let mesh = crysirror_unity::world6::SceneAssets::mesh(&mut u, &mf, mo).unwrap();
            println!("{} mats {mats:?} vertices {} submeshes {}", n.name, mesh.positions.len(), mesh.submeshes.len());
            for (k, uv) in mesh.uv.iter().enumerate() {
                if uv.is_empty() {
                    continue;
                }
                let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                for p in uv {
                    for i in 0..2 {
                        lo[i] = lo[i].min(p[i]);
                        hi[i] = hi[i].max(p[i]);
                    }
                }
                println!("  uv{k} {} {lo:?}..{hi:?}", uv.len());
                if let (Ok(dir), true) = (std::env::var("AMANATSU_DUMP"), k > 0) {
                    // Channel k as red/green at each UV0 texel of the triangles.
                    let s = 512usize;
                    let mut img = crysirror_image::ImageSurface { width: s as u32, height: s as u32, rgba: vec![0; s * s * 4] };
                    let uv0 = &mesh.uv[0];
                    for t in mesh.indices.chunks_exact(3) {
                        let p: Vec<[f32; 2]> = t.iter().map(|&i| uv0[i as usize]).collect();
                        let q: Vec<[f32; 2]> = t.iter().map(|&i| uv[i as usize]).collect();
                        let (x0, x1) = (p.iter().map(|v| v[0]).fold(f32::MAX, f32::min), p.iter().map(|v| v[0]).fold(f32::MIN, f32::max));
                        let (y0, y1) = (p.iter().map(|v| v[1]).fold(f32::MAX, f32::min), p.iter().map(|v| v[1]).fold(f32::MIN, f32::max));
                        let d = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
                        if d.abs() < 1e-12 {
                            continue;
                        }
                        for py in ((y0 * s as f32) as i64).max(0)..=((y1 * s as f32) as i64).min(s as i64 - 1) {
                            for px in ((x0 * s as f32) as i64).max(0)..=((x1 * s as f32) as i64).min(s as i64 - 1) {
                                let (x, y) = ((px as f32 + 0.5) / s as f32, (py as f32 + 0.5) / s as f32);
                                let b1 = ((x - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (y - p[0][1])) / d;
                                let b2 = ((p[1][0] - p[0][0]) * (y - p[0][1]) - (x - p[0][0]) * (p[1][1] - p[0][1])) / d;
                                let b0 = 1. - b1 - b2;
                                if b0 < 0. || b1 < 0. || b2 < 0. {
                                    continue;
                                }
                                let v = [0, 1].map(|i| q[0][i] * b0 + q[1][i] * b1 + q[2][i] * b2);
                                let o = ((s - 1 - py as usize) * s + px as usize) * 4;
                                img.rgba[o..o + 4].copy_from_slice(&[(v[0].fract().abs() * 255.) as u8, (v[1].fract().abs() * 255.) as u8, if (0. ..=1.).contains(&v[0]) && (0. ..=1.).contains(&v[1]) { 0 } else { 255 }, 255]);
                            }
                        }
                    }
                    std::fs::write(format!("{dir}/{}_uv{k}.png", n.name), crysirror_image::encode_png(&img).unwrap()).unwrap();
                }
            }
        }
    }
}

/// Rows of characustom lists (`AMANATSU_LISTS`, comma-separated) for an id
/// (`AMANATSU_ID`): every key 0..64 with a value.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn list_rows() {
    let mut u = unity();
    let lists = crysirror_amanatsu::chara::Lists::load(&mut u).unwrap();
    let id: i64 = std::env::var("AMANATSU_ID").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    for list in std::env::var("AMANATSU_LISTS").unwrap().split(',') {
        match lists.row(list, id) {
            Some(r) => println!("{list} {id}: {:?}", (0..64).filter_map(|k| r.get(k).map(|v| (k, v.to_owned()))).collect::<Vec<_>>()),
            None => println!("{list} {id}: none"),
        }
    }
}

/// The used accessory slots of a card's first coordinate (`AMANATSU_CARD`):
/// type, id, parent key and the move/colour values.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn card_accessories() {
    let mut u = unity();
    let path = std::env::var("AMANATSU_CARD").unwrap_or_else(|_| "DefaultData/0/chara/female/AL_F_00.png".into());
    let card = crysirror_amanatsu::card::Card::parse(&u.read_all(&path).unwrap()).unwrap();
    let coords = crysirror_unity::msgpack::decode(card.block("Coordinate").unwrap()).unwrap();
    let first = coords.as_array().first().and_then(crysirror_unity::msgpack::Value::as_bytes).unwrap().to_vec();
    let coord = crysirror_amanatsu::card::Card::container(&first).unwrap().1;
    let acc = crysirror_unity::msgpack::decode(coord.block("Accessory").unwrap()).unwrap();
    for (i, p) in acc.get("parts").map(crysirror_unity::msgpack::Value::as_array).unwrap_or(&[]).iter().enumerate() {
        let ty = p.get("type").and_then(crysirror_unity::msgpack::Value::as_i64).unwrap_or(0);
        if ty == 120 {
            continue;
        }
        println!("{i}: type {ty} id {:?} parent {:?} move {:?} hideCategory {:?}", p.get("id"), p.get("parentKeyType"), p.get("addMove"), p.get("hideCategory"));
    }
}

/// The tracks of a short movie timeline (`AMANATSU_SM`) with the first
/// clip's asset fields of the tracks whose script contains
/// `AMANATSU_TRACK` (default `Face`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn short_movie_tracks() {
    let mut u = unity();
    let id: i32 = std::env::var("AMANATSU_SM").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let want = std::env::var("AMANATSU_TRACK").unwrap_or_else(|_| "Face".into());
    let (key, oid) = u.bundle_asset(crysirror_amanatsu::sm::TIMELINES, &format!("stm_{id:02}"), 114).unwrap().unwrap();
    let asset = u.decode(&key, oid).unwrap();
    let mut object = |p: crysirror_unity::PPtr| -> Option<(String, crysirror_unity::managed::Fields)> {
        let (f, o) = u.target(&key, p).ok()?;
        let fields = u.decode(&f, o).ok()?;
        let script = fields.pptr("m_Script").map(|s| u.script_name(&f, s)).unwrap_or_default();
        Some((script, fields))
    };
    let t = crysirror_unity::timeline::Timeline::decode(&asset, &mut object).unwrap();
    for track in t.all_tracks() {
        println!("{} [{}] muted {} clips {}", track.name, track.script, track.muted, track.clips.len());
        if track.script.contains(&want) {
            for c in track.clips.iter().take(3) {
                if let Some((s, f)) = object(c.asset) {
                    println!("  {:.2}+{:.2} {s}: {:?}", c.start, c.duration, f);
                }
            }
        }
    }
}

/// Bundles holding GameObjects whose name contains `AMANATSU_FIND`.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn find_game_object() {
    let mut u = unity();
    let want = std::env::var("AMANATSU_FIND").unwrap();
    for key in u.bundle_files().into_iter().filter(|k| !k.ends_with(".resS")) {
        let Ok(file) = u.file(&key) else { continue };
        let class: i32 = std::env::var("AMANATSU_CLASS").ok().and_then(|c| c.parse().ok()).unwrap_or(1);
        let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == class).map(|o| o.path_id).collect();
        for id in ids {
            let Ok(f) = u.decode(&key, id) else { continue };
            let name = f.string("m_Name").unwrap_or("");
            if name.contains(&want) {
                println!("{key} {name}");
            }
        }
    }
}

/// Components of a scene (`AMANATSU_KEY`) whose script contains
/// `AMANATSU_SCRIPT`, with their pointer fields resolved to node names.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn component_refs() {
    use crysirror_unity::managed::Value;
    let mut u = unity();
    let key = std::env::var("AMANATSU_KEY").unwrap();
    let want = std::env::var("AMANATSU_SCRIPT").unwrap();
    let scene = u.scene(&key).unwrap();
    let mut owner = std::collections::HashMap::new();
    for (i, n) in scene.nodes.iter().enumerate() {
        for c in &n.components {
            owner.insert(c.path_id, i);
        }
        owner.insert(n.game_object, i);
        owner.insert(n.transform, i);
    }
    let name = |p: &crysirror_unity::PPtr| owner.get(&p.path_id).map_or_else(|| format!("#{}", p.path_id), |&i| {
        let n = &scene.nodes[i];
        let parent = n.parent.map_or("", |q| scene.nodes[q].name.as_str());
        format!("{parent}/{}", n.name)
    });
    for n in &scene.nodes {
        for c in n.components.iter().filter(|c| c.script.contains(&want)) {
            println!("== {} {}", n.name, c.script);
            for (k, v) in &c.fields.0 {
                match v {
                    Value::PPtr(p) if p.path_id != 0 => println!("  {k} = {}", name(p)),
                    Value::Array(a) if a.iter().any(|x| matches!(x, Value::PPtr(_))) => println!("  {k} = [{}]", a.iter().filter_map(|x| if let Value::PPtr(p) = x { Some(name(p)) } else { None }).collect::<Vec<_>>().join(", ")),
                    _ => {}
                }
            }
        }
    }
}

/// The state names of a controller (`AMANATSU_BUNDLE`, `AMANATSU_ASSET`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn controller_states() {
    use crysirror_unity::animator::Controller;
    let mut u = unity();
    let (k, id) = u.bundle_asset(&std::env::var("AMANATSU_BUNDLE").unwrap(), &std::env::var("AMANATSU_ASSET").unwrap(), 91).unwrap().unwrap();
    let ctl = Controller::decode(&u.decode(&k, id).unwrap()).unwrap();
    for (li, l) in ctl.layers.iter().enumerate() {
        let names: Vec<String> = l.states.iter().map(|s| ctl.names.get(&s.name_id).cloned().unwrap_or_else(|| format!("#{:x}", s.name_id))).collect();
        println!("layer {li}: {names:?}");
    }
}

/// ADV command counts over every scenario of personality 0 (with up to
/// three argument examples of `AMANATSU_CMD`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn adv_census() {
    let mut u = unity();
    let index = crysirror_amanatsu::talk::scenario_index(&mut u, 0).unwrap();
    let want = std::env::var("AMANATSU_CMD").ok();
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    let mut shown = 0;
    let mut names: Vec<&String> = index.keys().collect();
    names.sort();
    for name in names {
        let Ok(cmds) = crysirror_amanatsu::talk::load_scenario(&mut u, &index, name) else { continue };
        for c in cmds {
            let cname = c.name();
            if std::env::var("AMANATSU_SCENARIO").ok().as_deref() == Some(name.as_str()) {
                println!("{name}: {cname} {:?}", c.args);
            }
            if want.as_deref() == Some(cname.as_str()) && shown < 12 {
                shown += 1;
                println!("{name}: {cname} {:?}", c.args);
            }
            *counts.entry(cname).or_default() += 1;
            if false {
            }
        }
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, n) in v {
        println!("{n:6} {k}");
    }
}

/// List tables (any bundle) with a row holding the value `AMANATSU_VALUE`.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn tables_with_value() {
    let mut u = unity();
    let want = std::env::var("AMANATSU_VALUE").unwrap();
    let keys: Vec<String> = u.bundle_files().into_iter().filter(|k| !k.ends_with(".resS") && !k.contains("/chara/") && !k.contains("/ma/")).collect();
    for key in keys {
        let Ok(file) = u.file(&key) else { continue };
        let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == 49).map(|o| o.path_id).collect();
        for id in ids {
            let Ok(b) = u.text_asset(&key, id) else { continue };
            if let Ok(t) = crysirror_amanatsu::tables::Table::parse(&b) {
                if let Some(r) = t.rows.iter().find(|r| r.iter().any(|c| *c == want)) {
                    let name = u.decode(&key, id).ok().and_then(|f| f.string("m_Name").map(str::to_owned)).unwrap_or_default();
                    println!("{key} {name} keys {:?} row {:?}", t.keys, r);
                    if std::env::var("AMANATSU_ALLROWS").is_ok() {
                        for r in &t.rows {
                            println!("  {r:?}");
                        }
                    }
                }
            }
        }
    }
}

/// Particle systems of a map scene (`AMANATSU_MAP`): node, active, emission
/// rate, start lifetime/size/speed and renderer material shader.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn map_particles() {
    let mut u = unity();
    let infos = crysirror_amanatsu::map::map_infos(&mut u).unwrap();
    let id: i32 = std::env::var("AMANATSU_MAP").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let info = infos.iter().find(|i| i.id == id).unwrap();
    let key = u.bundle_files().into_iter().find(|k| k.starts_with(&format!("bundle:lib/{}#", info.scene.0)) && !k.ends_with(".resS")).unwrap();
    let scene = u.scene(&key).unwrap();
    let mut n = 0;
    for (i, node) in scene.nodes.iter().enumerate() {
        for c in node.components.iter().filter(|c| c.class_id == 198) {
            n += 1;
            if n > 25 {
                continue;
            }
            let mat = node.components.iter().find(|c| c.class_id == 199).and_then(|r| r.fields.array("m_Materials").and_then(|a| a.first().cloned()));
            let shader = mat.and_then(|m| if let crysirror_unity::managed::Value::PPtr(p) = m { u.target(&key, p).ok() } else { None }).and_then(|(f, o)| {
                let mf = u.decode(&f, o).ok()?;
                let sp = mf.pptr("m_Shader")?;
                let (sf, so) = u.target(&f, sp).ok()?;
                Some(format!("{} / {}", mf.string("m_Name").unwrap_or(""), u.decode(&sf, so).ok()?.fields("m_ParsedForm").and_then(|p| p.string("m_Name")).unwrap_or("")))
            });
            let module = |k: &str| c.fields.fields(k).cloned();
            let rate = module("EmissionModule").and_then(|e| e.fields("rateOverTime").and_then(|r| r.float("scalar")));
            let init = module("InitialModule");
            let f = |k: &str| init.as_ref().and_then(|m| m.fields(k)).and_then(|v| v.float("scalar"));
            println!("{} active {} rate {:?} life {:?} size {:?} speed {:?} {:?}", scene.path(i), node.active, rate, f("startLifetime"), f("startSize"), f("startSpeed"), shader);
        }
    }
    println!("{n} particle systems");
}

/// One ParticleSystem's enabled modules and its renderer material
/// (`AMANATSU_NODE` path suffix).
#[test]
#[ignore]
fn particle_dump() {
    use crysirror_unity::managed::{Fields, Value};
    fn dump(f: &Fields, depth: usize, out: &mut String) {
        for (k, v) in &f.0 {
            match v {
                Value::Struct(s) => {
                    if s.int("enabled") == Some(0) && depth == 0 {
                        out.push_str(&format!("{k}: off\n"));
                        continue;
                    }
                    out.push_str(&format!("{}{k}:\n", "  ".repeat(depth)));
                    dump(s, depth + 1, out);
                }
                Value::Array(a) if a.len() > 8 => out.push_str(&format!("{}{k}: [{} items]\n", "  ".repeat(depth), a.len())),
                Value::Bytes(b) => out.push_str(&format!("{}{k}: {} bytes\n", "  ".repeat(depth), b.len())),
                v => out.push_str(&format!("{}{k}: {v:?}\n", "  ".repeat(depth))),
            }
        }
    }
    let mut u = unity();
    let infos = crysirror_amanatsu::map::map_infos(&mut u).unwrap();
    let info = infos.iter().find(|i| i.id == 0).unwrap();
    let key = u.bundle_files().into_iter().find(|k| k.starts_with(&format!("bundle:lib/{}#", info.scene.0)) && !k.ends_with(".resS")).unwrap();
    let scene = u.scene(&key).unwrap();
    let want = std::env::var("AMANATSU_NODE").unwrap_or("effect00_morning/Waterfall_Clouds".into());
    let i = (0..scene.nodes.len()).find(|&i| scene.path(i).ends_with(&want)).unwrap();
    let node = &scene.nodes[i];
    let mut out = String::new();
    for c in node.components.iter().filter(|c| matches!(c.class_id, 198 | 199)) {
        out.push_str(&format!("== class {}\n", c.class_id));
        dump(&c.fields, 0, &mut out);
        if c.class_id == 199 {
            for m in c.fields.array("m_Materials").unwrap_or(&[]) {
                if let Value::PPtr(p) = m {
                    let (f, o) = u.target(&key, *p).unwrap();
                    let mf = u.decode(&f, o).unwrap();
                    out.push_str("== material\n");
                    dump(&mf, 0, &mut out);
                }
            }
        }
    }
    let (_, _, pos) = crysirror_unity::world6::world_matrix(&scene, i).to_scale_rotation_translation();
    out.push_str(&format!("world position {pos:?}\n"));
    std::fs::write(std::env::var("AMANATSU_DUMP").unwrap_or("/dev/stdout".into()), out).unwrap();
}

/// The nodes of a root (`AMANATSU_KEY`, `AMANATSU_ROOTNAME`) with local
/// transforms and their script components' fields.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn prefab_dump() {
    let mut u = unity();
    let key = std::env::var("AMANATSU_KEY").unwrap();
    let want = std::env::var("AMANATSU_ROOTNAME").unwrap();
    let scene = u.scene(&key).unwrap();
    for (i, n) in scene.nodes.iter().enumerate() {
        let mut r = i;
        while let Some(p) = scene.nodes[r].parent {
            r = p;
        }
        if scene.nodes[r].name != want {
            continue;
        }
        println!("{} {:?}", scene.path(i), n.xform);
        for c in n.components.iter().filter(|c| !c.script.is_empty()) {
            println!("  {}", c.script);
            for (k, v) in &c.fields.0 {
                println!("    {k}: {}", format!("{v:?}").chars().take(300).collect::<String>());
            }
        }
    }
}

/// MonoBehaviours under `lib/{AMANATSU_PREFIX}` with a field named
/// `AMANATSU_FIELD`, with their GameObject names.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn behaviours_with_field() {
    let mut u = unity();
    let prefix = format!("bundle:lib/{}", std::env::var("AMANATSU_PREFIX").unwrap_or_default());
    let field = std::env::var("AMANATSU_FIELD").unwrap();
    let keys: Vec<String> = u.bundle_files().into_iter().filter(|k| k.starts_with(&prefix) && !k.ends_with(".resS")).collect();
    let mut shown = 0;
    for key in keys {
        let Ok(file) = u.file(&key) else { continue };
        let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == 114).map(|o| o.path_id).collect();
        for id in ids {
            let Ok(f) = u.decode(&key, id) else { continue };
            if f.get(&field).is_none() || shown > 3 {
                continue;
            }
            shown += 1;
            let go = f.pptr("m_GameObject").and_then(|p| u.decode(&key, p.path_id).ok()).and_then(|g| g.string("m_Name").map(str::to_owned));
            println!("{key} {go:?}");
            for (k, v) in &f.0 {
                println!("  {k}: {}", format!("{v:?}").chars().take(1500).collect::<String>());
            }
        }
    }
}

/// A block of a card's first coordinate (`AMANATSU_CARD`,
/// `AMANATSU_BLOCK`, default `Hair`) as a MessagePack tree.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn coordinate_block() {
    let mut u = unity();
    let path = std::env::var("AMANATSU_CARD").unwrap_or_else(|_| "DefaultData/0/chara/female/AL_F_00.png".into());
    let card = crysirror_amanatsu::card::Card::parse(&u.read_all(&path).unwrap()).unwrap();
    let coords = crysirror_unity::msgpack::decode(card.block("Coordinate").unwrap()).unwrap();
    let first = coords.as_array().first().and_then(crysirror_unity::msgpack::Value::as_bytes).unwrap().to_vec();
    let coord = crysirror_amanatsu::card::Card::container(&first).unwrap().1;
    let block = std::env::var("AMANATSU_BLOCK").unwrap_or_else(|_| "Hair".into());
    println!("{:?}", crysirror_unity::msgpack::decode(coord.block(&block).unwrap()).unwrap());
}

/// The hierarchy of a build scene (`AMANATSU_SCENE`) with scripts.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn build_scene_tree() {
    let mut u = unity();
    let name = std::env::var("AMANATSU_SCENE").unwrap_or_else(|_| "Memory".into());
    let index = u.build_scenes().unwrap().iter().position(|s| *s == name).unwrap();
    let level = crysirror_amanatsu::level::Level::load(&mut u, &format!("level{index}"), &name).unwrap();
    let want = std::env::var("AMANATSU_SCRIPT").ok();
    for (i, n) in level.scene.nodes.iter().enumerate() {
        let scripts: Vec<&str> = n.components.iter().filter(|c| !c.script.is_empty()).map(|c| c.script.as_str()).collect();
        println!("{}{} {}", if n.active { "" } else { "(inactive) " }, level.scene.path(i), scripts.join(","));
        for c in n.components.iter().filter(|c| want.as_deref().is_some_and(|w| c.script.ends_with(w))) {
            for (k, v) in &c.fields.0 {
                println!("    {k}: {}", format!("{v:?}").chars().take(300).collect::<String>());
            }
        }
    }
}

/// A TextAsset's bytes (`AMANATSU_BUNDLE`, `AMANATSU_ASSET`) as hex.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn text_asset_hex() {
    let mut u = unity();
    let (k, id) = u.bundle_asset(&std::env::var("AMANATSU_BUNDLE").unwrap(), &std::env::var("AMANATSU_ASSET").unwrap(), 49).unwrap().unwrap();
    let b = u.text_asset(&k, id).unwrap();
    println!("{} bytes", b.len());
    for (i, chunk) in b.chunks(32).take(24).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|x| format!("{x:02x}")).collect();
        let text: String = chunk.iter().map(|&x| if (32..127).contains(&x) { x as char } else { '.' }).collect();
        println!("{:04x} {} {}", i * 32, hex.join(" "), text);
    }
}

/// A Shader's tree (`AMANATSU_SHADER` by parsed name, bundles under
/// `AMANATSU_DIR`, default `lib/chara`); with `AMANATSU_DUMP` the
/// decompressed program blobs per platform.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn shader_dump() {
    use crysirror_unity::managed::Value;
    fn show(v: &Value, depth: usize, name: &str) {
        let pad = "  ".repeat(depth);
        match v {
            Value::Struct(f) => {
                println!("{pad}{name}:");
                for (k, v) in &f.0 {
                    show(v, depth + 1, k);
                }
            }
            Value::Array(a) if a.iter().any(|x| matches!(x, Value::Struct(_) | Value::Array(_))) => {
                println!("{pad}{name}: [{}]", a.len());
                for (i, x) in a.iter().enumerate().take(2000) {
                    show(x, depth + 1, &format!("[{i}]"));
                }
            }
            Value::Bytes(b) => println!("{pad}{name}: <{} bytes>", b.len()),
            v => println!("{pad}{name}: {}", format!("{v:?}").chars().take(200).collect::<String>()),
        }
    }
    let mut u = unity();
    let want = std::env::var("AMANATSU_SHADER").unwrap();
    let sub = std::env::var("AMANATSU_DIR").unwrap_or_else(|_| "lib/chara".into());
    for key in u.bundle_files().into_iter().filter(|k| k.contains(&format!("{sub}/")) && !k.ends_with(".resS")) {
        let Ok(file) = u.file(&key) else { continue };
        let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == 48).map(|o| o.path_id).collect();
        for id in ids {
            let Ok(f) = u.decode(&key, id) else { continue };
            if f.fields("m_ParsedForm").and_then(|p| p.string("m_Name")) != Some(want.as_str()) {
                continue;
            }
            println!("== {key} {id}");
            for (k, v) in &f.0 {
                show(v, 0, k);
            }
            if let Ok(dir) = std::env::var("AMANATSU_DUMP") {
                let blob = match f.get("compressedBlob") {
                    Some(Value::Bytes(b)) => b.clone(),
                    Some(Value::Array(a)) => a.iter().map(|x| match x { Value::Int(i) => *i as u8, _ => 0 }).collect(),
                    _ => Vec::new(),
                };
                let nums = |n: &str| -> Vec<Vec<i64>> {
                    f.array(n).unwrap_or(&[]).iter().map(|p| match p {
                        Value::Array(a) => a.iter().map(|x| match x { Value::Int(i) => *i, Value::Struct(s) => s.int("data").unwrap_or(0), _ => 0 }).collect(),
                        Value::Int(i) => vec![*i],
                        Value::Struct(s) => s.array("data").map(|a| a.iter().filter_map(|x| if let Value::Int(i) = x { Some(*i) } else { None }).collect()).unwrap_or_default(),
                        _ => Vec::new(),
                    }).collect()
                };
                let (off, cl, dl) = (nums("offsets"), nums("compressedLengths"), nums("decompressedLengths"));
                println!("blob {} offsets {off:?} c {cl:?} d {dl:?}", blob.len());
                for (p, ((o, c), d)) in off.iter().zip(&cl).zip(&dl).enumerate() {
                    for (j, ((o, c), d)) in o.iter().zip(c).zip(d).enumerate() {
                        let src = &blob[*o as usize..(*o + *c) as usize];
                        let out = lz4_block(src, *d as usize);
                        std::fs::write(format!("{dir}/{p}_{j}.bin"), out).unwrap();
                    }
                }
            }
            return;
        }
    }
    panic!("shader not found");
}

/// LZ4 block decompression (probe use).
fn lz4_block(src: &[u8], size: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(size);
    let mut i = 0;
    let len = |i: &mut usize, mut n: usize| {
        if n == 15 {
            loop {
                let b = src[*i];
                *i += 1;
                n += b as usize;
                if b != 255 {
                    break;
                }
            }
        }
        n
    };
    while i < src.len() {
        let token = src[i];
        i += 1;
        let lit = len(&mut i, (token >> 4) as usize);
        out.extend_from_slice(&src[i..i + lit]);
        i += lit;
        if i >= src.len() {
            break;
        }
        let off = u16::from_le_bytes([src[i], src[i + 1]]) as usize;
        i += 2;
        let m = len(&mut i, (token & 15) as usize) + 4;
        let start = out.len() - off;
        for k in 0..m {
            out.push(out[start + k]);
        }
    }
    out
}

/// Objects named `AMANATSU_ASSET` in `AMANATSU_BUNDLE`: class and fields
/// (byte arrays by length).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn object_fields() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    let name = std::env::var("AMANATSU_ASSET").unwrap();
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let objs: Vec<(i64, i32)> = u.file(&key).unwrap().objects.iter().map(|o| (o.path_id, o.class_id)).collect();
        for (id, class) in objs {
            let Ok(f) = u.decode(&key, id) else { continue };
            if f.string("m_Name") != Some(name.as_str()) {
                continue;
            }
            println!("== {key} {id} class {class}");
            for (k, v) in &f.0 {
                match v {
                    crysirror_unity::managed::Value::Bytes(b) => println!("  {k}: <{} bytes>", b.len()),
                    v => println!("  {k}: {}", format!("{v:?}").chars().take(300).collect::<String>()),
                }
            }
        }
    }
}

/// The slices of the `Texture2DArray` `AMANATSU_ASSET` in `AMANATSU_BUNDLE`
/// as PNGs in `AMANATSU_DUMP` (RGB and alpha separately).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn texture_array_dump() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    let name = std::env::var("AMANATSU_ASSET").unwrap();
    let dir = std::env::var("AMANATSU_DUMP").unwrap();
    let mut found = None;
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let ids: Vec<i64> = u.file(&key).unwrap().objects.iter().filter(|o| o.class_id == 187).map(|o| o.path_id).collect();
        for id in ids {
            if u.decode(&key, id).ok().and_then(|f| f.string("m_Name").map(|n| n == name)) == Some(true) {
                found = Some((key.clone(), id));
            }
        }
    }
    let (k, id) = found.unwrap();
    let (slices, srgb) = u.texture_array(&k, id).unwrap();
    println!("{} slices, srgb {srgb}", slices.len());
    for (i, s) in slices.iter().enumerate() {
        for c in 0..4 {
            let g: Vec<u8> = s.rgba.chunks_exact(4).flat_map(|p| [p[c], p[c], p[c], 255]).collect();
            let img = crysirror_image::ImageSurface { width: s.width, height: s.height, rgba: g };
            std::fs::write(format!("{dir}/{name}_{i}_{}.png", "rgba".as_bytes()[c] as char), crysirror_image::encode_png(&img).unwrap()).unwrap();
        }
    }
}

/// A card's `Custom` face values (`AMANATSU_CARD`, default `AL_F_00`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn card_face() {
    let mut u = unity();
    let path = std::env::var("AMANATSU_CARD").unwrap_or_else(|_| "DefaultData/0/chara/female/AL_F_00.png".into());
    let card = crysirror_amanatsu::card::Card::parse(&u.read_all(&path).unwrap()).unwrap();
    let custom = card.parts("Custom").unwrap();
    println!("{:?}", custom.first());
    println!("Graphic {:?} raw {:?}", card.graphic(), card.block("Graphic").map(|b| b.len()));
}

/// Every texture of a material (`AMANATSU_BUNDLE`, name `AMANATSU_FILTER`)
/// to `AMANATSU_DUMP/<property>.png` (2D arrays: `<property>_<slice>.png`).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn material_textures() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    let filter = std::env::var("AMANATSU_FILTER").unwrap();
    let dir = std::env::var("AMANATSU_DUMP").unwrap();
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let ids: Vec<i64> = u.file(&key).unwrap().objects.iter().filter(|o| o.class_id == 21).map(|o| o.path_id).collect();
        for id in ids {
            let f = u.decode(&key, id).unwrap();
            if f.string("m_Name").unwrap_or("") != filter {
                continue;
            }
            let m = crysirror_unity::world6::UnityMaterial::decode(&f, String::new());
            for (prop, env) in &m.textures {
                if env.texture.path_id == 0 {
                    continue;
                }
                let Ok((file, pid)) = u.target(&key, env.texture) else { continue };
                let class = u.file(&file).unwrap().object(pid).map(|o| o.class_id).unwrap_or(0);
                let srgb = u.decode(&file, pid).ok().and_then(|t| t.int("m_ColorSpace")).unwrap_or(-1);
                if class == 187 {
                    let (slices, _) = u.texture_array(&file, pid).unwrap();
                    for (i, s) in slices.iter().enumerate() {
                        std::fs::write(format!("{dir}/{prop}_{i}.png"), crysirror_image::encode_png(s).unwrap()).unwrap();
                    }
                    println!("{prop} array {} slices colorspace {srgb}", slices.len());
                } else if let Ok(img) = u.texture(&file, pid) {
                    std::fs::write(format!("{dir}/{prop}.png"), crysirror_image::encode_png(&img).unwrap()).unwrap();
                    println!("{prop} {}x{} colorspace {srgb}", img.width, img.height);
                } else {
                    println!("{prop}: class {class} not decoded");
                }
            }
        }
    }
}

/// Lights of a bundle's prefabs (`AMANATSU_BUNDLE`): node path, local and
/// world rotation, light fields and the scripts beside them.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn bundle_lights() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let scene = match u.scene(&key) {
            Ok(s) => s,
            Err(e) => {
                println!("{key}: {e:#}");
                continue;
            }
        };
        println!("{key}: {} nodes", scene.nodes.len());
        for (i, n) in scene.nodes.iter().enumerate() {
            let Some(l) = n.components.iter().find(|c| c.class_id == 108) else { continue };
            let m = crysirror_unity::world6::world_matrix(&scene, i);
            let scripts: Vec<&str> = n.components.iter().map(|c| c.script.as_str()).filter(|s| !s.is_empty()).collect();
            println!("{} active {} local rot {:?} world fwd {:?}", scene.path(i), n.active, n.xform.rotation, m.transform_vector3(glam::Vec3::Z).to_array());
            println!("    type {:?} color {:?} intensity {:?} mask {:?} scripts {scripts:?}", l.fields.int("m_Type"), l.fields.get("m_Color"), l.fields.float("m_Intensity"), l.fields.fields("m_CullingMask").and_then(|m| m.int("m_Bits")));
        }
        for (i, n) in scene.nodes.iter().enumerate() {
            for c in n.components.iter().filter(|c| c.script.contains("Light")) {
                println!("{} [{}] {:?}", scene.path(i), c.script, c.fields.0.iter().filter(|(k, _)| !k.starts_with("m_")).collect::<Vec<_>>());
            }
        }
    }
}

/// Nodes of a bundle's prefabs (`AMANATSU_BUNDLE`) whose local scale is not
/// one, with the world scale of each.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn bundle_scales() {
    let mut u = unity();
    let bundle = std::env::var("AMANATSU_BUNDLE").unwrap();
    for key in u.bundle_files().into_iter().filter(|k| k.starts_with(&format!("bundle:{bundle}#")) && !k.ends_with(".resS")) {
        let Ok(scene) = u.scene(&key) else { continue };
        for (i, n) in scene.nodes.iter().enumerate() {
            let renderer = n.components.iter().any(|c| c.class_id == 137);
            if n.xform.scale != [1., 1., 1.] || (renderer && n.parent.is_none()) {
                let m = crysirror_unity::world6::world_matrix(&scene, i);
                println!("{} local {:?} world {:?} renderer {renderer}", scene.path(i), n.xform.scale, m.to_scale_rotation_translation().0.to_array());
            }
        }
    }
}

/// Expression presets of a personality (`AMANATSU_ID`, default 0) with a
/// flush or tears.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn expression_flush_tears() {
    let mut u = unity();
    let p: i64 = std::env::var("AMANATSU_ID").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let list = crysirror_amanatsu::face::expression_list(&mut u, p).unwrap();
    let mut ids: Vec<_> = list.iter().filter(|(_, v)| v[6].is_some_and(|x| x > 0.) || v[7].is_some_and(|x| x > 0.)).map(|(id, v)| (*id, v[6], v[7])).collect();
    ids.sort_by_key(|x| x.0);
    println!("{} presets, {} with flush/tears: {ids:?}", list.len(), ids.len());
    assert!(list.values().all(|v| v[7].is_none_or(|t| (0. ..=3.).contains(&t))));
}
