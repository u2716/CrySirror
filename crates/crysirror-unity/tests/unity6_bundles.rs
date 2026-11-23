// SPDX-License-Identifier: MPL-2.0
//! Opens UnityFS bundles of an installed Unity 6 player and decodes every
//! object through its type tree. Ignored by default:
//! `UNITY6_BUNDLES=<dir> cargo test --release -p crysirror-unity --test unity6_bundles -- --ignored --nocapture`
//! (`UNITY6_LIMIT` caps the bundle count).
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
        } else if p.extension().is_some_and(|e| e == "unity3d" || e == "bundle") {
            out.push(p);
        }
    }
}

#[test]
#[ignore = "needs UNITY6_BUNDLES"]
fn every_bundle_object_decodes() {
    let root = PathBuf::from(std::env::var_os("UNITY6_BUNDLES").unwrap());
    let limit: usize = std::env::var("UNITY6_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut classes: BTreeMap<i32, usize> = BTreeMap::new();
    let (mut bundles, mut objects, mut failures) = (0, 0, Vec::new());
    for path in files.into_iter().take(limit) {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
        let bundle = match Bundle::open(source) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("{}: {e:#}", path.display()));
                continue;
            }
        };
        bundles += 1;
        for node in bundle.nodes.iter().filter(|n| !n.path.ends_with(".resS") && !n.path.ends_with(".resource")) {
            let file = match SerializedFile::open(bundle.reader(node)) {
                Ok(f) => f,
                Err(e) => {
                    failures.push(format!("{}/{}: {e:#}", path.display(), node.path));
                    continue;
                }
            };
            for info in &file.objects {
                *classes.entry(info.class_id).or_default() += 1;
                objects += 1;
                if let Err(e) = file.decode(info) {
                    failures.push(format!("{}/{} #{} class {}: {e:#}", path.display(), node.path, info.path_id, info.class_id));
                }
            }
        }
    }
    eprintln!("{bundles} bundles, {objects} objects; classes {classes:?}");
    let show: Vec<&String> = failures.iter().filter(|f| std::env::var("UNITY6_SHOW").map_or(true, |c| f.contains(&format!("class {c}:")))).collect();
    for f in show.iter().take(40) {
        eprintln!("{f}");
    }
    assert!(failures.is_empty(), "{} failures", failures.len());
}

/// `UNITY6_DUMP=<bundle> [UNITY6_CLASS=<id>] [UNITY6_FULL=1]`: prints each
/// object's class, name and (truncated) fields.
#[test]
#[ignore = "needs UNITY6_DUMP"]
fn dump_bundle() {
    let path = PathBuf::from(std::env::var_os("UNITY6_DUMP").unwrap());
    let class: Option<i32> = std::env::var("UNITY6_CLASS").ok().and_then(|v| v.parse().ok());
    let full = std::env::var_os("UNITY6_FULL").is_some();
    let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
    let bundle = Bundle::open(source).unwrap();
    for node in &bundle.nodes {
        eprintln!("node {} {} bytes", node.path, node.size);
        if node.path.ends_with(".resS") || node.path.ends_with(".resource") {
            continue;
        }
        let file = SerializedFile::open(bundle.reader(node)).unwrap();
        eprintln!("externals {:?}", file.externals);
        for info in &file.objects {
            if class.is_some_and(|c| c != info.class_id) {
                continue;
            }
            let f = file.decode(info).unwrap();
            let name = f.string("m_Name").unwrap_or("").to_owned();
            let text = format!("{f:?}");
            let cut = if full { text.len() } else { text.len().min(600) };
            eprintln!("#{} class {} {name}: {}", info.path_id, info.class_id, &text[..text.floor_char_boundary(cut)]);
        }
    }
}

/// `UNITY6_DATA=<..._Data>`: every serialized player file (`level*`,
/// `*.assets`, `globalgamemanagers`) decodes under its type trees.
#[test]
#[ignore = "needs UNITY6_DATA"]
fn every_player_file_decodes() {
    let dir = PathBuf::from(std::env::var_os("UNITY6_DATA").unwrap());
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with("level") || n == "globalgamemanagers" || n.ends_with(".assets"))
        .collect();
    names.sort();
    let mut script_names: BTreeMap<[u8; 16], String> = BTreeMap::new();
    // Trees of every bundle under UNITY6_BUNDLES (optional).
    let mut registry = crysirror_unity::TreeRegistry::default();
    if let Some(root) = std::env::var_os("UNITY6_BUNDLES") {
        let mut files = Vec::new();
        walk(std::path::Path::new(&root), &mut files);
        for path in files {
            let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
            let bundle = Bundle::open(source).unwrap();
            for node in bundle.nodes.iter().filter(|n| !n.path.ends_with(".resS") && !n.path.ends_with(".resource")) {
                registry.add(&SerializedFile::open(bundle.reader(node)).unwrap());
            }
        }
        eprintln!("registry {:?}", registry.len());
    }
    // Trees generated from managed metadata (UNITY6_ASSEMBLIES: a directory
    // of assemblies) for scripts of player objects without one.
    if let Some(asm_dir) = std::env::var_os("UNITY6_ASSEMBLIES") {
        let mut asms = Vec::new();
        for e in std::fs::read_dir(&asm_dir).unwrap().filter_map(|e| e.ok()) {
            if e.path().extension().is_some_and(|x| x == "dll") {
                asms.push(crysirror_dotnet::Assembly::parse(std::fs::read(e.path()).unwrap()).unwrap());
            }
        }
        let ggm: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(dir.join("globalgamemanagers.assets")).unwrap())));
        let scripts_file = SerializedFile::open(ggm).unwrap();
        let mut wanted: Vec<([u8; 16], String)> = Vec::new();
        for name in &names {
            let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(dir.join(name)).unwrap())));
            let file = SerializedFile::open(source).unwrap();
            for info in file.objects.iter().filter(|i| i.class_id == 114) {
                let ty = &file.types[info.type_index.unwrap()];
                let Some(id) = ty.script_id else { continue };
                if registry.lookup(114, Some(id)).is_some() || wanted.iter().any(|w| w.0 == id) {
                    continue;
                }
                let b = file.bytes(info).unwrap();
                let path_id = i64::from_le_bytes(b[20..28].try_into().unwrap());
                let ext = i32::from_le_bytes(b[16..20].try_into().unwrap());
                let script_src = if ext == 0 { &file } else { &scripts_file };
                let Some(f) = script_src.object(path_id).and_then(|i| script_src.decode_using(i, &registry).ok()) else { continue };
                let (ns, class) = (f.string("m_Namespace").unwrap_or(""), f.string("m_ClassName").unwrap_or(""));
                let full = if ns.is_empty() { class.to_owned() } else { format!("{ns}.{class}") };
                wanted.push((id, full));
            }
        }
        let generated: Vec<_> = {
            let g = crysirror_unity::managed_gen::Generator::new(asms, &registry).unwrap();
            if let Ok(c) = std::env::var("UNITY6_TREE") {
                for n in g.mono_behaviour(&c).unwrap().nodes {
                    eprintln!("{}{} {} {:#x}{}", "  ".repeat(n.level as usize), n.type_name, n.name, n.meta_flag, if n.is_array { " []" } else { "" });
                }
            }
            wanted.iter().map(|(id, class)| (*id, class.clone(), g.mono_behaviour(class))).collect()
        };
        for (id, class, tree) in generated {
            script_names.insert(id, class.clone());
            match tree {
                Ok(t) => registry.insert_script(id, t),
                Err(e) => eprintln!("generate {class}: {e:#}"),
            }
        }
        eprintln!("generated {} script trees", wanted.len());
    }
    let (mut objects, mut failures) = (0, Vec::new());
    for name in &names {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(dir.join(name)).unwrap())));
        let file = SerializedFile::open(source).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        for info in &file.objects {
            objects += 1;
            if let Err(e) = file.decode_using(info, &registry) {
                let script = if info.class_id == 114 {
                    let b = file.bytes(info).unwrap();
                    let path_id = i64::from_le_bytes(b[20..28].try_into().unwrap());
                    let ext = i32::from_le_bytes(b[16..20].try_into().unwrap());
                    let id = file.types[info.type_index.unwrap()].script_id;
                    format!(" script {ext}/{path_id} {}", id.and_then(|i| script_names.get(&i).cloned()).unwrap_or_default())
                } else {
                    String::new()
                };
                failures.push(format!("{name} #{} class {}:{script} {e:#}", info.path_id, info.class_id));
            }
        }
    }
    eprintln!("{} files, {objects} objects", names.len());
    // Scripts of the MonoBehaviours without trees (header: m_GameObject,
    // m_Enabled + align, m_Script).
    let ggm: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(dir.join("globalgamemanagers.assets")).unwrap())));
    let scripts_file = SerializedFile::open(ggm).unwrap();
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    for name in &names {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(dir.join(name)).unwrap())));
        let file = SerializedFile::open(source).unwrap();
        for info in file.objects.iter().filter(|i| i.class_id == 114) {
            if file.decode_using(info, &registry).is_ok() {
                continue;
            }
            let b = file.bytes(info).unwrap();
            let path_id = i64::from_le_bytes(b[20..28].try_into().unwrap());
            let script = scripts_file.object(path_id).and_then(|i| scripts_file.decode_using(i, &registry).ok());
            let label = script.map(|f| format!("{}.{} ({})", f.string("m_Namespace").unwrap_or(""), f.string("m_ClassName").unwrap_or(""), f.string("m_AssemblyName").unwrap_or(""))).unwrap_or(format!("script #{path_id}"));
            *missing.entry(label).or_default() += 1;
        }
    }
    eprintln!("scripts without trees: {missing:#?}");
    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    for f in &failures {
        if let Some((file, rest)) = f.split_once(" #") {
            let class = rest.split("class ").nth(1).and_then(|c| c.split(':').next()).unwrap_or("?");
            *by_class.entry(format!("{class} {}", if file.starts_with("level") { "level" } else { file })).or_default() += 1;
        }
    }
    eprintln!("without trees: {by_class:?}");
    if let Some((fname, pid)) = std::env::var("UNITY6_HEX").ok().as_deref().and_then(|v| v.split_once(':')) {
        let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(dir.join(fname)).unwrap())));
        let file = SerializedFile::open(source).unwrap();
        let b = file.bytes_of(pid.parse().unwrap()).unwrap();
        let info = *file.object(pid.parse().unwrap()).unwrap();
        let ty = &file.types[info.type_index.unwrap()];
        eprintln!("type {:?} script {:?} own tree {}", ty.class_id, ty.script_id, ty.tree.is_some());
        if let Some(t) = registry.lookup(114, ty.script_id) {
            for n in &t.nodes {
                eprintln!("  reg {}{} {} {:#x}", "  ".repeat(n.level as usize), n.type_name, n.name, n.meta_flag);
            }
        }
        eprintln!("decode {:?}", file.decode_using(&info, &registry).map(|f| f.0.len()));
        for (i, c) in b.chunks(16).enumerate() {
            eprintln!("{:04x}: {}", i * 16, c.iter().map(|x| format!("{x:02x}")).collect::<Vec<_>>().join(" "));
        }
    }
    let show: Vec<&String> = failures.iter().filter(|f| std::env::var("UNITY6_SHOW").map_or(true, |c| f.contains(&format!("class {c}:")))).collect();
    for f in show.iter().take(60) {
        eprintln!("{f}");
    }
    assert!(failures.is_empty(), "{} failures", failures.len());
}

/// `UNITY6_DATA`: the build scene list and the `Resources` table of
/// `globalgamemanagers` (no type trees) read by the fixed readers.
#[test]
#[ignore = "needs UNITY6_DATA"]
fn build_settings_and_resources_read() {
    use crysirror_unity::{class_id as c, classes};
    let dir = PathBuf::from(std::env::var_os("UNITY6_DATA").unwrap());
    let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(dir.join("globalgamemanagers")).unwrap())));
    let file = SerializedFile::open(source).unwrap();
    let build = file.objects.iter().find(|o| o.class_id == c::BUILD_SETTINGS).unwrap();
    let levels = classes::build_levels(&file.bytes(build).unwrap()).unwrap();
    eprintln!("levels {levels:?}");
    let rm = file.objects.iter().find(|o| o.class_id == c::RESOURCE_MANAGER).unwrap();
    let res = classes::resource_container_64(&file.bytes(rm).unwrap()).unwrap();
    eprintln!("{} resources, e.g. {:?}", res.len(), &res[..res.len().min(8)]);
    assert!(!levels.is_empty() && !res.is_empty());
}

/// `UNITY6_DATA` + `UNITY6_LEVEL=level1`: the scene hierarchy with each
/// GameObject's components (scripts by class name).
#[test]
#[ignore = "needs UNITY6_DATA and UNITY6_LEVEL"]
fn print_level() {
    let dir = PathBuf::from(std::env::var_os("UNITY6_DATA").unwrap());
    let level = std::env::var("UNITY6_LEVEL").unwrap();
    let mut registry = crysirror_unity::TreeRegistry::default();
    if let Some(root) = std::env::var_os("UNITY6_BUNDLES") {
        let mut files = Vec::new();
        walk(std::path::Path::new(&root), &mut files);
        for path in files {
            let source: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
            let bundle = Bundle::open(source).unwrap();
            for node in bundle.nodes.iter().filter(|n| n.flags & 4 != 0) {
                registry.add(&SerializedFile::open(bundle.reader(node)).unwrap());
            }
        }
    }
    let open = |n: &str| -> SerializedFile {
        let s: Arc<dyn AssetReader> = Arc::new(File(Mutex::new(std::fs::File::open(dir.join(n)).unwrap())));
        SerializedFile::open(s).unwrap()
    };
    let file = open(&level);
    let ggm = open("globalgamemanagers.assets");
    let script_name = |p: crysirror_unity::PPtr| -> String {
        let src = if p.file_id == 0 { &file } else { &ggm };
        src.object(p.path_id).and_then(|i| src.decode_using(i, &registry).ok()).map(|f| f.string("m_ClassName").unwrap_or("?").to_owned()).unwrap_or_else(|| format!("script {p:?}"))
    };
    // GameObject → (name, components); Transform → (gameobject, children).
    let mut go: BTreeMap<i64, (String, Vec<String>, bool)> = BTreeMap::new();
    let mut tf: BTreeMap<i64, (i64, Vec<i64>, i64)> = BTreeMap::new();
    for info in &file.objects {
        let Ok(f) = file.decode_using(info, &registry) else { continue };
        let pp = |n: &str| match f.get(n) {
            Some(crysirror_unity::managed::Value::PPtr(p)) => *p,
            _ => crysirror_unity::PPtr::default(),
        };
        match info.class_id {
            1 => {
                go.insert(info.path_id, (f.string("m_Name").unwrap_or("").to_owned(), Vec::new(), f.int("m_IsActive").unwrap_or(1) != 0));
            }
            4 | 224 => {
                let kids = match f.get("m_Children") {
                    Some(crysirror_unity::managed::Value::Array(a)) => a.iter().filter_map(|v| if let crysirror_unity::managed::Value::PPtr(p) = v { Some(p.path_id) } else { None }).collect(),
                    _ => Vec::new(),
                };
                tf.insert(info.path_id, (pp("m_GameObject").path_id, kids, pp("m_Father").path_id));
            }
            _ => {}
        }
    }
    for info in &file.objects {
        if matches!(info.class_id, 1 | 4 | 224) {
            continue;
        }
        let Ok(f) = file.decode_using(info, &registry) else { continue };
        let Some(crysirror_unity::managed::Value::PPtr(g)) = f.get("m_GameObject") else { continue };
        let label = if info.class_id == 114 {
            match f.get("m_Script") {
                Some(crysirror_unity::managed::Value::PPtr(p)) => script_name(*p),
                _ => "MonoBehaviour".into(),
            }
        } else {
            format!("#{}", info.class_id)
        };
        if let Some(e) = go.get_mut(&g.path_id) {
            e.1.push(label);
        }
    }
    fn show(t: i64, depth: usize, tf: &BTreeMap<i64, (i64, Vec<i64>, i64)>, go: &BTreeMap<i64, (String, Vec<String>, bool)>) {
        let Some((g, kids, _)) = tf.get(&t) else { return };
        if let Some((name, comps, active)) = go.get(g) {
            eprintln!("{}{name}{} {comps:?}", "  ".repeat(depth), if *active { "" } else { " (inactive)" });
        }
        for k in kids {
            show(*k, depth + 1, tf, go);
        }
    }
    for (t, (_, _, father)) in &tf {
        if *father == 0 {
            show(*t, 0, &tf, &go);
        }
    }
}
