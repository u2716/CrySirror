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
