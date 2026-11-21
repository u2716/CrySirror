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
