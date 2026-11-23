// SPDX-License-Identifier: MPL-2.0
//! The installed game's Unity data through client storage: player files of
//! `AmanatsuLocation_Data`, the UnityFS bundles of `lib/` and the
//! Addressables bundles, the type-tree registry (bundle trees plus the
//! generated MonoBehaviour trees of `data/managed_trees.json`), and lookups
//! the game logic and presentation share.
use anyhow::{bail, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{AssetReader, ClientStorage},
};
use crysirror_image::ImageSurface;
use crysirror_unity::{
    bundle::Bundle,
    managed::{Fields, Value},
    unity6, PPtr, SerializedFile, TreeRegistry,
};
use lru::LruCache;
use std::{collections::HashMap, num::NonZeroUsize};
use std::sync::Arc;

/// Generated trees: `{"scripts": {hex id: {"class", "tree"}}}`.
const MANAGED_TREES: &str = include_str!("../data/managed_trees.json");

/// A bundle file: `bundle:<path>#<node>`; a player file: its name.
pub struct Unity {
    storage: Arc<dyn ClientStorage>,
    pub data_dir: String,
    files: LruCache<String, Arc<SerializedFile>>,
    bundles: LruCache<String, Arc<Bundle>>,
    /// CAB node name → (bundle path, node path).
    cabs: HashMap<String, (String, String)>,
    pub registry: TreeRegistry,
    /// Generated script classes by script id.
    pub script_classes: HashMap<[u8; 16], String>,
    sprite_meta: HashMap<(String, i64), Option<([f32; 4], [f32; 2], f32)>>,
    meshes: HashMap<(String, i64), Arc<unity6::Mesh6>>,
}

fn hex16(s: &str) -> Option<[u8; 16]> {
    let mut out = [0u8; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

impl Unity {
    pub fn new(storage: Arc<dyn ClientStorage>, data_dir: &str) -> Result<Self> {
        let mut u = Self {
            storage,
            data_dir: data_dir.to_owned(),
            // Serialized files can retain descriptors after bundle eviction.
            // Bound both owners, including loose player files.
            files: LruCache::new(NonZeroUsize::new(32).unwrap()),
            bundles: LruCache::new(NonZeroUsize::new(16).unwrap()),
            cabs: HashMap::new(),
            registry: TreeRegistry::default(),
            script_classes: HashMap::new(),
            sprite_meta: HashMap::new(),
            meshes: HashMap::new(),
        };
        let mut paths = Vec::new();
        u.walk("lib", &mut paths);
        u.walk(&format!("{data_dir}/StreamingAssets/aa"), &mut paths);
        for p in paths.iter().filter(|p| p.ends_with(".unity3d") || p.ends_with(".bundle")) {
            // Discovery retains names and type trees, releasing each source
            // before indexing the next bundle.
            let source = u.reader(p).with_context(|| format!("index bundle {p}"))?;
            let Ok(bundle) = Bundle::open(source) else { continue };
            for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
                if let Ok(f) = SerializedFile::open(bundle.reader(node)) {
                    u.registry.add(&f);
                }
                u.cabs.insert(node.path.clone(), (p.clone(), node.path.clone()));
            }
        }
        let doc: serde_json::Value = serde_json::from_str(MANAGED_TREES)?;
        for (k, v) in doc["scripts"].as_object().context("managed_trees.json scripts")? {
            let id = hex16(k).context("managed tree script id")?;
            let tree: crysirror_unity::typetree::TypeTree = serde_json::from_value(v["tree"].clone())?;
            if u.registry.lookup(crysirror_unity::class_id::MONO_BEHAVIOUR, Some(id)).is_none() {
                u.registry.insert_script(id, tree);
            }
            u.script_classes.insert(id, v["class"].as_str().unwrap_or("").to_owned());
        }
        Ok(u)
    }

    fn walk(&self, dir: &str, out: &mut Vec<String>) {
        let Ok(names) = self.storage.list(Mount::Asset, dir) else { return };
        for n in names {
            let p = format!("{dir}/{n}");
            if self.storage.list(Mount::Asset, &p).is_ok() {
                self.walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }

    pub fn reader(&self, path: &str) -> Result<Arc<dyn AssetReader>> {
        Ok(self.storage.open_asset(&VfsPath::new(Mount::Asset, path)?)?)
    }

    /// File names in an installation directory (sorted).
    pub fn list_dir(&self, dir: &str) -> Vec<String> {
        let mut v = self.storage.list(Mount::Asset, dir).unwrap_or_default();
        v.sort();
        v
    }

    /// A whole installation file (cards, settings).
    pub fn read_all(&self, path: &str) -> Result<Vec<u8>> {
        let r = self.reader(path)?;
        let len = r.len().map_err(|e| anyhow::anyhow!("{path}: {e}"))? as usize;
        anyhow::ensure!(len <= 64 << 20, "{path} is larger than 64 MiB");
        let mut out = vec![0u8; len];
        let mut at = 0;
        while at < len {
            let n = r.read_at(at as u64, &mut out[at..]).map_err(|e| anyhow::anyhow!("{path}: {e}"))?;
            anyhow::ensure!(n > 0, "{path} truncated");
            at += n;
        }
        Ok(out)
    }

    pub fn bundle(&mut self, path: &str) -> Result<Arc<Bundle>> {
        if let Some(b) = self.bundles.get(path) {
            return Ok(b.clone());
        }
        let b = Arc::new(Bundle::open(self.reader(path)?).with_context(|| format!("bundle {path}"))?);
        self.bundles.put(path.to_owned(), b.clone());
        Ok(b)
    }

    /// A serialized file by key: a player file name or `bundle:<path>#<node>`.
    pub fn file(&mut self, key: &str) -> Result<Arc<SerializedFile>> {
        if let Some(f) = self.files.get(key) {
            return Ok(f.clone());
        }
        let f = if let Some(rest) = key.strip_prefix("bundle:") {
            let (path, node) = rest.split_once('#').context("bundle file key")?;
            let b = self.bundle(path)?;
            let n = b.node(node).with_context(|| format!("{node} not in {path}"))?;
            SerializedFile::open(b.reader(n))?
        } else {
            SerializedFile::open(self.reader(&format!("{}/{key}", self.data_dir))?).with_context(|| format!("player file {key}"))?
        };
        let f = Arc::new(f);
        self.files.put(key.to_owned(), f.clone());
        Ok(f)
    }

    /// The file key of an external reference of `file`.
    pub fn external_key(&mut self, file: &str, file_id: i32) -> Result<String> {
        let f = self.file(file)?;
        let name = f.external(file_id)?.to_owned();
        // `archive:/CAB-…/CAB-…` references resolve through the bundle index.
        if let Some(cab) = name.rsplit('/').next().filter(|n| n.starts_with("CAB-") || name.starts_with("archive:")) {
            let (b, n) = self.cabs.get(cab).cloned().with_context(|| format!("external {name}"))?;
            return Ok(format!("bundle:{b}#{n}"));
        }
        Ok(name.rsplit('/').next().unwrap_or(&name).to_owned())
    }

    /// (file key, path id) of a pointer of `file`.
    pub fn target(&mut self, file: &str, p: PPtr) -> Result<(String, i64)> {
        if p.file_id == 0 {
            Ok((file.to_owned(), p.path_id))
        } else {
            Ok((self.external_key(file, p.file_id)?, p.path_id))
        }
    }

    pub fn decode(&mut self, file: &str, path_id: i64) -> Result<Fields> {
        let f = self.file(file)?;
        let info = *f.object(path_id).with_context(|| format!("object {path_id} of {file}"))?;
        f.decode_using(&info, &self.registry)
    }

    pub fn texture(&mut self, file: &str, path_id: i64) -> Result<ImageSurface> {
        let f = self.decode(file, path_id)?;
        let resolve = |p: &str| -> Option<Arc<dyn AssetReader>> { self.stream_reader_ro(file, p) };
        unity6::texture_2d(&f, &resolve)
    }

    /// A `Texture2DArray`'s slices at level 0 and whether they are sRGB.
    pub fn texture_array(&mut self, file: &str, path_id: i64) -> Result<(Vec<ImageSurface>, bool)> {
        if let Some((bp, _)) = file.strip_prefix("bundle:").and_then(|r| r.split_once('#')) {
            self.bundle(bp)?;
        }
        let f = self.decode(file, path_id)?;
        let resolve = |p: &str| -> Option<Arc<dyn AssetReader>> { self.stream_reader_ro(file, p) };
        unity6::texture_2d_array(&f, &resolve)
    }

    fn stream_reader_ro(&self, file: &str, path: &str) -> Option<Arc<dyn AssetReader>> {
        if let Some(rest) = file.strip_prefix("bundle:") {
            let (bp, _) = rest.split_once('#')?;
            // A serialized file cache hit can outlive its bundle cache entry.
            let reopened;
            let b = match self.bundles.peek(bp) {
                Some(b) => b.as_ref(),
                None => {
                    reopened = Bundle::open(self.reader(bp).ok()?).ok()?;
                    &reopened
                }
            };
            let n = b.node(unity6::stream_node(path))?;
            return Some(b.reader(n));
        }
        self.reader(&format!("{}/{}", self.data_dir, path.rsplit('/').next().unwrap_or(path))).ok()
    }

    /// A sprite's pixels (its texture rectangle, top-down).
    pub fn sprite_image(&mut self, file: &str, path_id: i64) -> Result<Option<ImageSurface>> {
        let f = self.decode(file, path_id)?;
        let atlas = match f.get("m_SpriteAtlas") {
            Some(Value::PPtr(a)) if !a.is_null() => {
                let (k, id) = self.target(file, *a)?;
                Some((k.clone(), self.decode(&k, id)?))
            }
            _ => None,
        };
        let load = |_: PPtr| atlas.as_ref().map(|a| a.1.clone());
        let Some((tex, rect, in_atlas)) = unity6::sprite_render(&f, &load) else { return Ok(None) };
        let base = if in_atlas { atlas.as_ref().map_or(file.to_owned(), |a| a.0.clone()) } else { file.to_owned() };
        let (tk, tid) = self.target(&base, tex)?;
        // Make the texture's bundle resident for streamed data.
        if let Some(rest) = tk.strip_prefix("bundle:") {
            if let Some((bp, _)) = rest.split_once('#') {
                self.bundle(bp)?;
            }
        }
        let img = self.texture(&tk, tid)?;
        // Texture rect is bottom-up in texture pixels.
        let (x, y, w, h) = (rect[0].round() as i64, rect[1].round() as i64, rect[2].round() as i64, rect[3].round() as i64);
        if w <= 0 || h <= 0 {
            return Ok(None);
        }
        let (iw, ih) = (img.width as i64, img.height as i64);
        let top = ih - (y + h);
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for row in 0..h {
            let sy = (top + row).clamp(0, ih - 1);
            for col in 0..w {
                let sx = (x + col).clamp(0, iw - 1);
                let i = ((sy * iw + sx) * 4) as usize;
                rgba.extend_from_slice(&img.rgba[i..i + 4]);
            }
        }
        Ok(Some(ImageSurface { width: w as u32, height: h as u32, rgba }))
    }

    /// Sprite borders (left, bottom, right, top), size and pixels per unit.
    pub fn sprite_meta(&mut self, file: &str, p: PPtr) -> Option<([f32; 4], [f32; 2], f32)> {
        if p.is_null() {
            return None;
        }
        let (k, id) = self.target(file, p).ok()?;
        if let Some(m) = self.sprite_meta.get(&(k.clone(), id)) {
            return *m;
        }
        let m = self.decode(&k, id).ok().and_then(|f| {
            let vec = |n: &str, keys: &[&str]| -> Option<Vec<f32>> {
                match f.get(n) {
                    Some(Value::Struct(s)) => Some(keys.iter().map(|k| s.float(k).unwrap_or(0.) as f32).collect()),
                    _ => None,
                }
            };
            let border = vec("m_Border", &["x", "y", "z", "w"]).unwrap_or(vec![0.; 4]);
            let rect = vec("m_Rect", &["x", "y", "width", "height"])?;
            Some(([border[0], border[1], border[2], border[3]], [rect[2], rect[3]], f.float("m_PixelsToUnits").unwrap_or(100.) as f32))
        });
        self.sprite_meta.insert((k, id), m);
        m
    }

    /// The TrueType data behind a TMP font asset: its `m_SourceFontFile`, or
    /// that of its first fallback with one.
    pub fn tmp_font_data(&mut self, file: &str, path_id: i64) -> Result<Option<(String, Vec<u8>)>> {
        let mut todo = vec![(file.to_owned(), path_id)];
        let mut seen = 0;
        while let Some((k, id)) = todo.pop() {
            seen += 1;
            if seen > 16 {
                break;
            }
            let f = self.decode(&k, id)?;
            if let Some(Value::PPtr(src)) = f.get("m_SourceFontFile") {
                if !src.is_null() {
                    let (fk, fid) = self.target(&k, *src)?;
                    let font = self.decode(&fk, fid)?;
                    let name = font.string("m_Name").unwrap_or("font").to_owned();
                    match font.get("m_FontData") {
                        Some(Value::Bytes(b)) if !b.is_empty() => return Ok(Some((name, b.clone()))),
                        Some(Value::Array(a)) if !a.is_empty() => {
                            return Ok(Some((name, a.iter().filter_map(|v| if let Value::Int(i) = v { Some(*i as u8) } else { None }).collect())));
                        }
                        _ => {}
                    }
                }
            }
            if let Some(Value::Array(fb)) = f.get("m_FallbackFontAssetTable") {
                for p in fb.iter().rev() {
                    if let Value::PPtr(p) = p {
                        if !p.is_null() {
                            todo.push(self.target(&k, *p)?);
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Build scene names by build index (`BuildSettings.scenes`, without
    /// path or extension).
    pub fn build_scenes(&mut self) -> Result<Vec<String>> {
        let ggm = self.file("globalgamemanagers")?;
        let info = ggm.objects.iter().find(|o| o.class_id == crysirror_unity::class_id::BUILD_SETTINGS).context("BuildSettings")?;
        let levels = crysirror_unity::classes::build_levels(&ggm.bytes(info)?)?;
        Ok(levels.iter().map(|l| l.rsplit('/').next().unwrap_or(l).trim_end_matches(".unity").to_owned()).collect())
    }

    /// The `m_Resource` bytes (an FSB5 bank) of a decoded AudioClip of
    /// `file`: a sibling node of its bundle or a player `.resource` file.
    pub fn audio_clip_bytes(&mut self, file: &str, clip: &Fields) -> Result<Vec<u8>> {
        let res = clip.fields("m_Resource").context("AudioClip m_Resource")?;
        let source = res.string("m_Source").unwrap_or("");
        let offset = res.int("m_Offset").unwrap_or(0) as u64;
        let size = res.int("m_Size").unwrap_or(0) as usize;
        anyhow::ensure!(size <= 256 << 20, "AudioClip resource of {size} bytes");
        let node = source.rsplit('/').next().unwrap_or(source);
        let reader = match file.strip_prefix("bundle:").and_then(|r| r.split_once('#')) {
            Some((bundle, _)) => {
                let b = self.bundle(bundle)?;
                let n = b.node(node).with_context(|| format!("{node} not in {bundle}"))?;
                b.reader(n)
            }
            None => self.reader(&format!("{}/{node}", self.data_dir))?,
        };
        let mut out = vec![0u8; size];
        let mut at = 0;
        while at < size {
            let n = reader.read_at(offset + at as u64, &mut out[at..])?;
            anyhow::ensure!(n > 0, "AudioClip resource truncated");
            at += n;
        }
        Ok(out)
    }

    /// `m_Script` bytes of a TextAsset (read raw: the type-tree string is
    /// not UTF-8).
    pub fn text_asset(&mut self, file: &str, path_id: i64) -> Result<Vec<u8>> {
        let f = self.file(file)?;
        let info = *f.object(path_id).with_context(|| format!("TextAsset {path_id} of {file}"))?;
        let b = f.bytes(&info)?;
        let name_len = u32::from_le_bytes(b.get(..4).context("TextAsset")?.try_into()?) as usize;
        let at = (4 + name_len + 3) & !3;
        let len = u32::from_le_bytes(b.get(at..at + 4).context("TextAsset m_Script")?.try_into()?) as usize;
        Ok(b.get(at + 4..at + 4 + len).context("TextAsset m_Script bytes")?.to_vec())
    }

    /// Objects of a bundle (`lib/…` path) by asset name (`AssetBundle`
    /// container path stem, case-insensitive): (file key, path id).
    pub fn bundle_asset(&mut self, bundle: &str, name: &str, class_id: i32) -> Result<Option<(String, i64)>> {
        let b = self.bundle(bundle)?;
        let nodes: Vec<String> = b.nodes.iter().filter(|n| n.flags & 4 != 0).map(|n| n.path.clone()).collect();
        for node in nodes {
            let key = format!("bundle:{bundle}#{node}");
            let f = self.file(&key)?;
            let Some(ab) = f.objects.iter().find(|o| o.class_id == crysirror_unity::class_id::ASSET_BUNDLE).copied() else { continue };
            let fields = f.decode_using(&ab, &self.registry)?;
            for e in fields.array("m_Container").unwrap_or(&[]) {
                let crysirror_unity::managed::Value::Struct(e) = e else { continue };
                let path = e.string("first").unwrap_or("");
                let stem = path.rsplit('/').next().unwrap_or(path);
                let stem = stem.rsplit_once('.').map_or(stem, |(s, _)| s);
                if !stem.eq_ignore_ascii_case(name) {
                    continue;
                }
                let Some(p) = e.fields("second").and_then(|s| s.pptr("asset")) else { continue };
                if f.object(p.path_id).is_some_and(|o| o.class_id == class_id) {
                    return Ok(Some((key, p.path_id)));
                }
            }
        }
        Ok(None)
    }

    /// The first MonoBehaviour of script class `class` in the bundles whose
    /// key contains `within` (e.g. the Addressables `PropertyData`).
    pub fn find_script_object(&mut self, within: &str, class: &str) -> Result<Option<Fields>> {
        let keys: Vec<String> = self.bundle_files().into_iter().filter(|k| k.contains(within)).collect();
        for key in keys {
            let file = self.file(&key)?;
            let ids: Vec<i64> = file.objects.iter().filter(|o| o.class_id == crysirror_unity::class_id::MONO_BEHAVIOUR).map(|o| o.path_id).collect();
            for id in ids {
                let Ok(f) = self.decode(&key, id) else { continue };
                if self.script_name(&key, f.pptr("m_Script").unwrap_or_default()) == class {
                    return Ok(Some(f));
                }
            }
        }
        Ok(None)
    }

    /// Keys of every serialized file of the loaded bundles.
    pub fn bundle_files(&self) -> Vec<String> {
        let mut v: Vec<String> = self.cabs.values().map(|(b, n)| format!("bundle:{b}#{n}")).collect();
        v.sort();
        v
    }

    /// `m_ClassName` of a MonoBehaviour's script.
    pub fn script_name(&mut self, file: &str, p: PPtr) -> String {
        if p.is_null() {
            return String::new();
        }
        let Ok((k, id)) = self.target(file, p) else { return String::new() };
        self.decode(&k, id).map(|f| crysirror_unity::scene6::script_class(&f)).unwrap_or_default()
    }

    /// A `Mesh` with its streamed vertex data.
    pub fn mesh(&mut self, file: &str, path_id: i64) -> Result<Arc<unity6::Mesh6>> {
        if let Some(m) = self.meshes.get(&(file.to_owned(), path_id)) {
            return Ok(m.clone());
        }
        let f = self.decode(file, path_id)?;
        let resolve = |p: &str| -> Option<Arc<dyn AssetReader>> { self.stream_reader_ro(file, p) };
        let m = Arc::new(unity6::mesh(&f, &resolve)?);
        self.meshes.insert((file.to_owned(), path_id), m.clone());
        Ok(m)
    }

    /// A level (`levelN`) or bundle file as a scene hierarchy.
    pub fn scene(&mut self, key: &str) -> Result<crysirror_unity::scene6::Scene6> {
        let file = self.file(key)?;
        // Script names first (MonoBehaviour header: m_GameObject PPtr,
        // m_Enabled + align, m_Script PPtr).
        let mut names: HashMap<PPtr, String> = HashMap::new();
        for info in file.objects.iter().filter(|o| o.class_id == crysirror_unity::class_id::MONO_BEHAVIOUR) {
            let b = file.bytes(info)?;
            if b.len() < 28 {
                continue;
            }
            let p = PPtr { file_id: i32::from_le_bytes(b[16..20].try_into()?), path_id: i64::from_le_bytes(b[20..28].try_into()?) };
            if !names.contains_key(&p) {
                let n = self.script_name(key, p);
                names.insert(p, n);
            }
        }
        crysirror_unity::scene6::Scene6::load(&file, &self.registry, &|p| names.get(&p).cloned().unwrap_or_default())
    }
}

#[allow(dead_code)]
fn unused() -> Result<()> {
    bail!("unused")
}

impl crysirror_unity::world6::SceneAssets for Unity {
    fn object(&mut self, file: &str, p: PPtr) -> Result<Option<(String, i64, i32, Fields)>> {
        if p.is_null() {
            return Ok(None);
        }
        let (k, id) = match self.target(file, p) {
            Ok(t) => t,
            // `Library/unity default resources` and other player built-ins.
            Err(_) => return Ok(None),
        };
        let Ok(f) = self.file(&k) else { return Ok(None) };
        let Some(info) = f.object(id).copied() else { return Ok(None) };
        let fields = match info.class_id {
            crysirror_unity::class_id::MATERIAL | crysirror_unity::world6::SHADER => f.decode_using(&info, &self.registry)?,
            _ => Fields::default(),
        };
        Ok(Some((k, id, info.class_id, fields)))
    }
    fn mesh(&mut self, file: &str, path_id: i64) -> Result<Arc<unity6::Mesh6>> {
        Unity::mesh(self, file, path_id)
    }
    fn texture_size(&mut self, file: &str, path_id: i64) -> Result<(u32, u32)> {
        let f = self.decode(file, path_id)?;
        Ok((f.int("m_Width").unwrap_or(0).max(1) as u32, f.int("m_Height").unwrap_or(0).max(1) as u32))
    }
    fn texture(&mut self, file: &str, path_id: i64) -> Result<ImageSurface> {
        // Make the texture's bundle resident for streamed data.
        if let Some((bp, _)) = file.strip_prefix("bundle:").and_then(|r| r.split_once('#')) {
            self.bundle(bp)?;
        }
        Unity::texture(self, file, path_id)
    }
}
