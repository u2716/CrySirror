// SPDX-License-Identifier: MPL-2.0
//! Owner-corpus checks over an installed CM3D2 1.00 `GameData`. Ignored by
//! default; run with `CM3D2_ROOT=<game dir> cargo test --release -- --ignored`.
use crysirror_core::{service::AssetReader, state::ClientId};
use crysirror_resource::{read_all, Archive, DecodeContext, FormatRegistry};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

struct File(Mutex<std::fs::File>);
impl AssetReader for File {
    fn len(&self) -> crysirror_core::Result<u64> {
        let f = self.0.lock().unwrap();
        f.metadata().map(|m| m.len()).map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        use std::io::{Read, Seek, SeekFrom};
        let mut f = self.0.lock().unwrap();
        f.seek(SeekFrom::Start(at))
            .and_then(|_| f.read(out))
            .map_err(|e| crysirror_core::Error::Io(e.to_string()))
    }
}

fn root() -> Option<PathBuf> {
    std::env::var_os("CM3D2_ROOT").map(PathBuf::from)
}

fn open(name: &str, cx: &DecodeContext) -> Box<dyn Archive> {
    open_path(root().unwrap().join("GameData").join(name), cx)
}

fn open_path(path: PathBuf, cx: &DecodeContext) -> Box<dyn Archive> {
    let mut registry = FormatRegistry::default();
    crysirror_formats_arc::register(&mut registry).unwrap();
    let reader = Arc::new(File(Mutex::new(std::fs::File::open(&path).unwrap())));
    registry.open(reader, Some("kiss/warc"), cx).unwrap().1
}

fn context() -> DecodeContext {
    let mut cx = DecodeContext::new(ClientId(1));
    cx.limits.max_entry_size = 256 * 1024 * 1024;
    cx
}

/// Flat file-name lookup, as `FileSystemArchive` resolves game files.
fn by_name(archive: &dyn Archive) -> HashMap<String, u32> {
    archive
        .entries()
        .iter()
        .map(|e| (e.path.rsplit('/').next().unwrap().to_ascii_lowercase(), e.id))
        .collect()
}

#[test]
#[ignore = "needs CM3D2_ROOT"]
fn every_model_and_animation_converts_and_loads() {
    let cx = context();
    let textures = open("texture.arc", &cx);
    let texture_names = by_name(textures.as_ref());
    let models = open("model.arc", &cx);
    let mut dropped = 0usize;
    for e in models.entries() {
        let bytes = read_all(models.open(e.id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap();
        let mut external = |name: &str| -> anyhow::Result<Vec<u8>> {
            let id = *texture_names
                .get(&name.to_ascii_lowercase())
                .ok_or_else(|| anyhow::anyhow!("no texture {name}"))?;
            read_all(textures.open(id, &cx)?.as_ref(), cx.limits.max_entry_size)
        };
        let glb = crysirror_asset::interchange::to_gltf(&bytes, &mut external)
            .unwrap_or_else(|err| panic!("{}: {err:#}", e.path));
        let model = crysirror_asset::pbr::load_pbr(&glb, |_| anyhow::bail!("external"))
            .unwrap_or_else(|err| panic!("{}: {err:#}", e.path));
        assert!(!model.meshes.is_empty(), "{}", e.path);
        dropped += model.dropped.len();
    }
    eprintln!("{} models, {dropped} dropped-feature notes", models.entries().len());
    let motions = open("motion.arc", &cx);
    for e in motions.entries() {
        let bytes = read_all(motions.open(e.id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap();
        let glb = crysirror_asset::interchange::to_gltf(&bytes, &mut |_| anyhow::bail!("none"))
            .unwrap_or_else(|err| panic!("{}: {err:#}", e.path));
        let model = crysirror_asset::pbr::load_pbr(&glb, |_| anyhow::bail!("external"))
            .unwrap_or_else(|err| panic!("{}: {err:#}", e.path));
        assert_eq!(model.animations.len(), 1, "{}", e.path);
    }
    eprintln!("{} animations", motions.entries().len());
}

#[test]
#[ignore = "needs CM3D2_ROOT"]
fn every_nei_table_decrypts() {
    let dll = std::fs::read(root().unwrap().join(crysirror_cm3d2::native_key::DLL_PATH)).unwrap();
    let keys = crysirror_cm3d2::native_key::NativeKeys::from_dll(&dll).unwrap();
    let cx = context();
    let csv = open("csv.arc", &cx);
    for e in csv.entries() {
        let bytes = read_all(csv.open(e.id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap();
        let table = crysirror_cm3d2::nei::Table::open(&bytes, &keys.table_key)
            .unwrap_or_else(|err| panic!("{}: {err:#}", e.path));
        assert!(table.columns > 0 && table.rows > 0, "{}", e.path);
        if let Ok(dir) = std::env::var("NEI_DUMP") {
            let mut out = String::new();
            for y in 0..table.rows {
                let row: Vec<String> = (0..table.columns).map(|x| table.string(x, y)).collect();
                out += &format!("{y}\t{}\n", row.join("\t").trim_end_matches('\t'));
            }
            let name = e.path.rsplit('/').next().unwrap();
            std::fs::write(format!("{dir}/{name}.txt"), out).unwrap();
        }
        if e.path.ends_with("schedule_define.nei") {
            eprintln!("{} {}x{}: {:?}", e.path, table.columns, table.rows,
                (0..table.columns.min(6)).map(|x| table.string(x, 0)).collect::<Vec<_>>());
        }
    }
}

struct Dir(PathBuf);
impl crysirror_cm3d2::unity_scene::UnityFiles for Dir {
    fn open(&mut self, name: &str) -> anyhow::Result<Arc<dyn AssetReader>> {
        Ok(Arc::new(File(Mutex::new(std::fs::File::open(self.0.join(name))?))))
    }
}

#[test]
#[ignore = "needs CM3D2_ROOT; prints CM3D2_SCENE (default mainData)"]
fn print_scene_hierarchy() {
    use crysirror_cm3d2::unity_scene::{Component, UnityData};
    let mut dir = Dir(root().unwrap().join("CM3D2x64_Data"));
    let mut data = UnityData::new().unwrap();
    let mut name = std::env::var("CM3D2_SCENE").unwrap_or_else(|_| "mainData".into());
    if !name.starts_with("level") && name != "mainData" {
        // A scene name: its build index through mainData's BuildSettings.
        let main = data.file(&mut dir, "mainData").unwrap();
        let info = *main.objects.iter().find(|o| o.class_id == crysirror_unity::class_id::BUILD_SETTINGS).unwrap();
        let levels = crysirror_unity::classes::build_levels(&main.bytes(&info).unwrap()).unwrap();
        let index = levels.iter().position(|l| l.rsplit('/').next().and_then(|f| f.strip_suffix(".unity")) == Some(name.as_str())).unwrap();
        name = if index == 0 { "mainData".into() } else { format!("level{}", index - 1) };
        eprintln!("scene file {name}");
    }
    let scene = match std::env::var("CM3D2_PREFAB") {
        Ok(path) => {
            let go = data.resource_of(&mut dir, &path, crysirror_unity::class_id::GAME_OBJECT).unwrap().expect("prefab resource");
            data.prefab(&mut dir, &go).unwrap()
        }
        Err(_) => data.scene(&mut dir, &name).unwrap(),
    };
    if let Ok(class) = std::env::var("CM3D2_SCRIPT") {
        for i in 0..scene.nodes.len() {
            if scene.script(i, &class).is_some() {
                eprintln!("{class} at {}", scene.path(i));
            }
        }
    }
    for (i, n) in scene.nodes.iter().enumerate() {
        let depth = scene.path(i).matches('/').count();
        let comps: Vec<String> = n.components.iter().map(|(_, c)| match c {
            Component::Transform => String::new(),
            Component::Camera(c) => format!("Camera(ortho={} size={} depth={} mask={:#x})", c.orthographic, c.orthographic_size, c.depth, c.culling_mask),
            Component::Script { class, fields, enabled } if std::env::var("CM3D2_FIELDS").is_ok_and(|c| c.split(',').any(|c| c == class)) => format!("{class}(enabled={enabled} {fields:?})"),
            Component::Script { class, fields, .. } if class == "UISprite" => format!(
                "UISprite({} type={} {}x{} color={:?} atlas={:?})",
                fields.string("mSpriteName").unwrap_or(""),
                fields.int("mType").unwrap_or(-1),
                fields.int("mWidth").unwrap_or(0),
                fields.int("mHeight").unwrap_or(0),
                fields.floats("mColor"),
                fields.pptr("mAtlas")
            ),
            Component::Script { class, fields, .. } if matches!(class.as_str(), "UILabel" | "UIWidget" | "UI2DSprite" | "UITexture") => {
                let a = |s: &str| fields.fields(s).and_then(|a| Some((a.pptr("target").filter(|t| !t.is_null())?.path_id, a.float("relative")?, a.int("absolute")?)));
                format!("{class}(pivot={} {}x{} L{:?} R{:?} B{:?} T{:?})", fields.int("mPivot").unwrap_or(-1), fields.int("mWidth").unwrap_or(0), fields.int("mHeight").unwrap_or(0), a("leftAnchor"), a("rightAnchor"), a("bottomAnchor"), a("topAnchor"))
            }
            Component::Script { class, .. } => class.clone(),
            Component::Other { class_id } => format!("#{class_id}"),
        }).filter(|s| !s.is_empty()).collect();
        let only = std::env::var("CM3D2_PATH").unwrap_or_default();
        if scene.path(i).contains(&only) && depth <= std::env::var("CM3D2_DEPTH").ok().and_then(|d| d.parse().ok()).unwrap_or(3) {
            eprintln!("{}{} [{}] {}{:?} pos={:?} {:?}", "  ".repeat(depth), n.name, n.key.path_id, if n.active { "" } else { "(inactive) " }, comps, n.position, n.scale);
        }
    }
}

/// Storage over the extracted archives is not available offline, so this
/// drives `Maid` through a minimal in-memory ClientStorage of all menus,
/// models and materials of the corpus.
#[test]
#[ignore = "needs CM3D2_ROOT"]
fn print_npc_maid_slots() {
    use crysirror_cm3d2::chara::Maid;
    struct Mem(std::collections::HashMap<String, Vec<u8>>);
    impl crysirror_core::service::ClientStorage for Mem {
        fn read(&self, p: &crysirror_core::path::VfsPath) -> crysirror_core::Result<Vec<u8>> {
            self.0.get(p.relative()).cloned().ok_or(crysirror_core::Error::NotFound)
        }
        fn write_atomic(&self, _: &crysirror_core::path::VfsPath, _: &[u8]) -> crysirror_core::Result<()> { Err(crysirror_core::Error::ReadOnly) }
        fn remove(&self, _: &crysirror_core::path::VfsPath) -> crysirror_core::Result<()> { Err(crysirror_core::Error::ReadOnly) }
        fn list(&self, _: crysirror_core::path::Mount, dir: &str) -> crysirror_core::Result<Vec<String>> {
            let prefix = format!("{dir}/");
            let mut v: Vec<String> = self.0.keys().filter_map(|k| k.strip_prefix(&prefix)).map(|r| r.split('/').next().unwrap().to_owned()).collect();
            v.sort();
            v.dedup();
            Ok(v)
        }
    }
    let cx = context();
    let mut map = std::collections::HashMap::new();
    for arc in ["menu", "material", "model"] {
        let a = open(&format!("{arc}.arc"), &cx);
        for e in a.entries() {
            map.insert(format!("__cm3d2/{arc}/{}", e.path), read_all(a.open(e.id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap());
        }
    }
    let storage = Mem(map);
    let files = crysirror_cm3d2::files::GameFiles::index(&storage, &["__cm3d2/menu".into(), "__cm3d2/material".into(), "__cm3d2/model".into()]).unwrap();
    // CM3D2_MENUS: comma-separated menus (default `_I_default.menu`);
    // CM3D2_MAN: build a man.
    let mut maid = Maid::new(std::env::var_os("CM3D2_MAN").is_some());
    for menu in std::env::var("CM3D2_MENUS").unwrap_or_else(|_| "_I_default.menu".into()).split(',') {
        maid.set_up_model(&files, &storage, menu).unwrap();
    }
    for (k, v) in &maid.slots {
        eprintln!("slot {k}: {:?} nodes={:?}", v.model, v.nodes);
    }
    eprintln!("visible {:?}", maid.visible_slots());
}

/// Bones of `CM3D2_MODEL` (in model.arc) and track paths of `CM3D2_ANM`
/// (in motion.arc), for skeleton/clip mismatches.
#[test]
#[ignore = "needs CM3D2_ROOT, CM3D2_MODEL and/or CM3D2_ANM"]
fn print_model_and_clip() {
    use crysirror_asset::interchange::cm3d2::{Animation, Mesh};
    let cx = context();
    if let Ok(name) = std::env::var("CM3D2_MODEL") {
        let a = open("model.arc", &cx);
        let id = by_name(a.as_ref())[&name.to_ascii_lowercase()];
        let mesh = Mesh::parse(&read_all(a.open(id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap()).unwrap();
        for (i, b) in mesh.bones.iter().enumerate() {
            eprintln!("bone {i} {} parent={:?}", b.name, b.parent);
        }
        eprintln!("skin {:?}", mesh.skin_bones);
        for m in &mesh.materials {
            eprintln!("material {} {} {:?}", m.name, m.shader, m.properties);
        }
    }
    if let Ok(name) = std::env::var("CM3D2_ANM") {
        let a = open("motion.arc", &cx);
        let id = by_name(a.as_ref())[&name.to_ascii_lowercase()];
        let anim = Animation::parse(&read_all(a.open(id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap()).unwrap();
        for t in &anim.tracks {
            eprintln!("track {}", t.path);
        }
    }
}

/// Every command head of every `.menu` with its count and one example.
#[test]
#[ignore = "needs CM3D2_ROOT"]
fn menu_command_census() {
    use crysirror_cm3d2::menu::MenuFile;
    let cx = context();
    let a = open("menu.arc", &cx);
    let mut count: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    for e in a.entries() {
        let bytes = read_all(a.open(e.id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap();
        let Ok(m) = MenuFile::parse(&bytes) else { continue };
        for c in &m.commands {
            let k = count.entry(c[0].clone()).or_insert((0, String::new()));
            k.0 += 1;
            if k.1.is_empty() {
                k.1 = format!("{}: {:?}", e.path, c);
            }
        }
    }
    for (k, (n, ex)) in count {
        eprintln!("{n:6} {k} — {ex}");
    }
}

/// The commands of `CM3D2_MENU` (in menu.arc), in order.
#[test]
#[ignore = "needs CM3D2_ROOT and CM3D2_MENU"]
fn print_menu() {
    use crysirror_cm3d2::menu::MenuFile;
    let cx = context();
    let a = open("menu.arc", &cx);
    let name = std::env::var("CM3D2_MENU").unwrap().to_ascii_lowercase();
    let names = by_name(a.as_ref());
    if let Some(cat) = name.strip_prefix("cat:") {
        for (n, id) in &names {
            let Ok(m) = MenuFile::parse(&read_all(a.open(*id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap()) else { continue };
            if m.category.eq_ignore_ascii_case(cat) {
                eprintln!("in {n}");
                for c in &m.commands { eprintln!("cmd {c:?}"); }
            }
        }
        return;
    }
    if !names.contains_key(&name) {
        let mut v: Vec<_> = names.keys().filter(|n| n.contains(&name)).collect();
        v.sort();
        eprintln!("matches {v:?}");
        return;
    }
    let id = names[&name];
    let m = MenuFile::parse(&read_all(a.open(id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap()).unwrap();
    eprintln!("category {} name {}", m.category, m.name);
    for c in &m.commands {
        eprintln!("cmd {c:?}");
    }
}

/// Static tables of the installed Assembly-CSharp (CM3D2_TYPE, default
/// SceneEditInfo) evaluated from their `.cctor`.
#[test]
#[ignore = "needs CM3D2_ROOT"]
fn print_managed_statics() {
    let path = root().unwrap().join("CM3D2x64_Data/Managed/Assembly-CSharp.dll");
    let asm = crysirror_dotnet::Assembly::parse(std::fs::read(path).unwrap()).unwrap();
    let ty = std::env::var("CM3D2_TYPE").unwrap_or_else(|_| "SceneEditInfo".into());
    let st = asm.eval_cctor(&ty).unwrap();
    for (name, v) in &st.fields {
        let n = st.dict(v).map(|d| d.len()).or_else(|| st.items(v).map(|l| l.len()));
        eprintln!("static {name}: {n:?} {:?}", st.object(v).map(|o| format!("{o:?}").chars().take(300).collect::<String>()));
    }
    for i in std::env::var("CM3D2_OBJECTS").unwrap_or_default().split(',').filter_map(|x| x.parse::<usize>().ok()) {
        eprintln!("object {i}: {:?}", st.heap.get(i));
    }
}

#[test]
#[ignore = "scratch: dump an atlas sprite"]
fn scratch_dump_sprite() {
    use crysirror_cm3d2::unity_scene::{Component, UnityData};
    let mut dir = Dir(root().unwrap().join("CM3D2x64_Data"));
    let mut data = UnityData::new().unwrap();
    let level = std::env::var("CM3D2_SCENE").unwrap_or_else(|_| "level4".into());
    let scene = data.scene(&mut dir, &level).unwrap();
    let n = scene.find(&std::env::var("CM3D2_NODE").unwrap_or_else(|_| "UI Root/FFNameDialog/Base".into())).unwrap();
    let want = std::env::var("CM3D2_SPRITE").unwrap_or_else(|_| "cm3d2_dialog_frame".into());
    let f = scene.script(n, "UISprite").unwrap().clone();
    let file = scene.nodes[n].key.file.clone();
    let ak = data.resolve(&mut dir, &file, f.pptr("mAtlas").unwrap()).unwrap().unwrap();
    let (_, _, _, af) = data.behaviour(&mut dir, &ak).unwrap();
    for s in af.array("mSprites").unwrap() {
        let crysirror_unity::managed::Value::Struct(s) = s else { continue };
        if s.string("name").is_some_and(|n| n.contains(want.as_str())) { eprintln!("{s:?}"); } else { eprintln!("name {:?}", s.string("name")); }
    }
    let mk = data.resolve(&mut dir, &ak.file, af.pptr("material").unwrap()).unwrap().unwrap();
    let (_, mb) = data.bytes(&mut dir, &mk).unwrap();
    let m = crysirror_unity::classes::Material::read(&mb).unwrap();
    let tk = data.resolve(&mut dir, &mk.file, m.texture("_MainTex").unwrap().texture).unwrap().unwrap();
    let (_, tb) = data.bytes(&mut dir, &tk).unwrap();
    let t = crysirror_unity::classes::Texture2D::read(&tb).unwrap();
    let img = crysirror_unity::texture::decode(&t).unwrap();
    eprintln!("tex {}x{} {:?} fmt={:?}", img.width, img.height, t.name, t.format);
    let out = std::env::var("OUT").unwrap();
    std::fs::write(&out, &img.rgba).unwrap();
    let _ = Component::Transform;
}

#[test]
#[ignore = "needs CM3D2_ROOT"]
fn game_tables_build_and_a_new_maid_initializes() {
    let dll = std::fs::read(root().unwrap().join(crysirror_cm3d2::native_key::DLL_PATH)).unwrap();
    let keys = crysirror_cm3d2::native_key::NativeKeys::from_dll(&dll).unwrap();
    let cx = context();
    let csv = open("csv.arc", &cx);
    let mut tables = HashMap::new();
    for e in csv.entries() {
        let bytes = read_all(csv.open(e.id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap();
        let name = e.path.rsplit('/').next().unwrap().to_ascii_lowercase();
        tables.insert(name, crysirror_cm3d2::nei::Table::open(&bytes, &keys.table_key).unwrap());
    }
    let data = crysirror_cm3d2::gamedata::GameData::from_tables(tables).unwrap();
    let skills: usize = data.skills.iter().map(|c| c.len()).sum();
    eprintln!(
        "trophies {} shop {} grades {} skills {} noon {} night {}",
        data.trophies.len(),
        data.shop.len(),
        data.salon_grades.len(),
        skills,
        data.schedule.noon.len(),
        data.schedule.night.len()
    );
    assert!(!data.trophies.is_empty() && !data.shop.is_empty() && skills > 0);
    assert_eq!(data.salon_grades.len(), 5);
    let y = &data.yotogi;
    let commands: usize = y.commands.values().map(Vec::len).sum();
    eprintln!("yotogi stages {} skills with commands {} commands {} wait motions {} undressing {}", y.stages.len(), y.commands.len(), commands, y.wait_motions.len(), y.undressing.len());
    assert_eq!(y.commands.len(), skills);
    assert!(!y.stages.is_empty() && y.undressing.len() >= 10);
    let mut player = crysirror_cm3d2::params::PlayerStatus::default();
    let mut pp = crysirror_cm3d2::maid_param::PlayerParam { st: &mut player, data: &data };
    pp.init();
    assert!(pp.st.have_item.len() > 100);
    let mut maid = crysirror_cm3d2::params::MaidStatus::default();
    let mut mp = crysirror_cm3d2::maid_param::MaidParam { st: &mut maid, data: &data };
    mp.initialize(&mut pp, "g".into(), "2026/11/20 1:02:03".into(), "名".into(), "姓".into()).unwrap();
    assert_eq!(mp.st.create_time_num, 2026112010203);
    assert!(mp.st.maid_class[0].is_have);
    assert!(mp.st.skill_data.len() > 0);
    eprintln!("skills at start {} classes {:?}", mp.st.skill_data.len(), mp.st.current_maid_class);
}

/// Boot the installed title and drive its actual colliders, with writable state outside the install.
#[test]
#[cfg(feature = "runtime")]
#[ignore = "needs CM3D2_ROOT"]
fn title_settings_and_load_cancel() {
    use crysirror_core::service::{buttons, InputFrame};
    use crysirror_runtime::{AdapterRegistry, Runtime};
    use serde_json::{json, Value};

    fn state(rt: &Runtime) -> Value { rt.inspect()["sessions"][0]["native"].clone() }
    fn hit(rt: &Runtime, path: &str) -> Option<[i32; 2]> {
        let state = state(rt);
        let matches = state["hits"].as_array()?.iter().find(|h| h["path"].as_str().is_some_and(|s| s.ends_with(path)))?;
        let rect = matches["rect"].as_array()?;
        Some([((rect[0].as_f64()? + rect[2].as_f64()?) * 0.5) as i32,
              ((rect[1].as_f64()? + rect[3].as_f64()?) * 0.5) as i32])
    }
    fn tick(rt: &mut Runtime, position: [i32; 2], pressed: u64, held: u64) {
        rt.tick(16, InputFrame {pointer_x: position[0], pointer_y: position[1], buttons_pressed: pressed, buttons_down: held}).unwrap();
    }
    fn idle(rt: &mut Runtime, count: usize) {
        for _ in 0..count { tick(rt, [960,540], 0, 0); }
    }

    let install = root().expect("CM3D2_ROOT");
    let mut names: Vec<_> = std::fs::read_dir(install.join("GameData")).unwrap().flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.ends_with(".arc")).collect();
    names.sort();
    let mounts: Vec<_> = names.iter().map(|name| format!("__cm3d2/{}", name.trim_end_matches(".arc"))).collect();
    let project = serde_json::from_value(json!({"schema":2, "name":"cm3d2-ui", "content_revision":"1",
        "canvas":[1920,1080], "assets":"game","clients":[{"name":"game","game":"local-installed",
        "adapter":"cm3d2","config":{"data_dir":"CM3D2x64_Data","archives":mounts}}]})).unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_cm3d2::register(&mut registry).unwrap();
    let writable = tempfile::tempdir().unwrap();
    let mut rt = Runtime::with_asset_root(project, &install, writable.path(), registry).unwrap();
    for (name, mount) in names.iter().zip(&mounts) {
        let mut cx = context();
        cx.limits.max_entry_size = 1024 * 1024 * 1024;
        let archive = crysirror_resource::ArchiveMount::new(open(name, &cx), cx).unwrap();
        rt.mount_provider(ClientId(1), mount, Arc::new(archive)).unwrap();
    }
    for frame in 0..640 {
        tick(&mut rt, [960,540], if frame == 90 || frame == 260 { buttons::MOUSE_LEFT } else {0}, 0);
    }
    let blank = [960, 540];
    for (key, selected) in [(buttons::UP, 3), (buttons::DOWN, 0), (buttons::DOWN, 1),
                            (buttons::DOWN, 2), (buttons::DOWN, 3)] {
        tick(&mut rt, blank, key, key);
        for _ in 0..10 { tick(&mut rt, blank, 0, key); }
        assert!(state(&rt)["active"].as_str().unwrap().contains(&format!("active: {selected},")));
        tick(&mut rt, blank, 0, 0);
    }
    // The physical cursor can retake focus, and keyboard focus survives a stationary cursor.
    let start = hit(&rt, "/StartGroup/Start").unwrap();
    tick(&mut rt, start, 0, 0);
    assert!(state(&rt)["active"].as_str().unwrap().contains("active: 0,"));
    tick(&mut rt, start, buttons::DOWN, 0);
    tick(&mut rt, start, 0, 0);
    assert!(state(&rt)["active"].as_str().unwrap().contains("active: 1,"));
    tick(&mut rt, [start[0]+1, start[1]], 0, 0);
    assert!(state(&rt)["active"].as_str().unwrap().contains("active: 0,"));
    let config = hit(&rt, "/ButtonGroup/ConfigGroup/Config").expect("title Config");
    tick(&mut rt, config, 0, 0);
    assert!(state(&rt)["active"].as_str().unwrap().contains("active: 2"));
    let ok = "/ConfigPanel/OK";
    tick(&mut rt, config, buttons::MOUSE_LEFT, buttons::MOUSE_LEFT);
    assert!(hit(&rt, ok).is_none(), "press alone opened settings");
    tick(&mut rt, [960,540], 0, buttons::MOUSE_LEFT);
    tick(&mut rt, config, 0, 0);
    assert!(hit(&rt, ok).is_none(), "drag activated settings");
    tick(&mut rt, config, buttons::MOUSE_LEFT, buttons::MOUSE_LEFT);
    tick(&mut rt, config, 0, 0);
    idle(&mut rt, 40);
    let close = hit(&rt, ok).expect("settings OK");
    tick(&mut rt, blank, buttons::DOWN | buttons::ENTER, 0);
    assert!(state(&rt)["active"].as_str().unwrap().contains("active: 2,"));
    assert_eq!(hit(&rt, ok), Some(close));
    // ConfigPanel/BG overlaps the title Start collider on this original layout.
    let behind_settings = hit(&rt, "/StartGroup/Start").expect("covered Start");
    tick(&mut rt, behind_settings, buttons::MOUSE_LEFT, 0);
    idle(&mut rt, 5);
    assert!(state(&rt)["active"].as_str().unwrap().contains("closing: None"));
    assert_eq!(hit(&rt, ok), Some(close));
    tick(&mut rt, close, buttons::ESCAPE, buttons::ESCAPE);
    tick(&mut rt, close, 0, 0);
    assert_eq!(hit(&rt, ok), Some(close));
    tick(&mut rt, close, buttons::MOUSE_LEFT, 0);
    idle(&mut rt, 65);
    assert!(hit(&rt, ok).is_none());
    let load = hit(&rt, "/ButtonGroup/ContinueGroup/Continue").expect("title Continue");
    tick(&mut rt, load, 0, 0);
    tick(&mut rt, load, buttons::ENTER, buttons::ENTER);
    tick(&mut rt, load, 0, 0);
    idle(&mut rt, 100);
    let cancel = hit(&rt, "/SaveAndLoadPanel/Cancel").expect("load Cancel");
    tick(&mut rt, cancel, buttons::MOUSE_LEFT, 0);
    idle(&mut rt, 100);
    assert!(hit(&rt, "/SaveAndLoadPanel/Cancel").is_none());
    assert!(state(&rt)["active"].as_str().unwrap().contains("panel: Shown"));
}

#[test]
#[ignore = "needs COM3D2_ROOT"]
fn com3d2_parts_use_shared_kiss_formats() {
    use crysirror_asset::interchange::cm3d2::{Animation, Material, Mesh};
    let root = PathBuf::from(std::env::var_os("COM3D2_ROOT").expect("COM3D2_ROOT"));
    let cx = context();
    let parts = open_path(root.join("GameData/parts.arc"), &cx);
    let names = by_name(parts.as_ref());
    let read = |name: &str| {
        read_all(parts.open(names[name], &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap()
    };
    let mesh = Mesh::parse(&read("dress237_shoe.model")).unwrap();
    assert_eq!(mesh.name, "dress237_shoe");
    assert!(!mesh.positions.is_empty() && !mesh.skin_bones.is_empty());
    let material = Material::parse_file(&read("dress237_z2_shoe.mate")).unwrap();
    assert!(!material.shader.is_empty() && !material.properties.is_empty());
    let menu = crysirror_cm3d2::menu::MenuFile::parse(&read("dress237_accsenaka_i_.menu")).unwrap();
    assert_eq!(menu.first("icons"), Some("Dress237_accSenaka_I_.tex"));
    let motion = Animation::parse(&read("accsenaka004.anm")).unwrap();
    assert!(motion.tracks.iter().any(|track| track.curves.iter().flatten().any(|c| !c.keys.is_empty())));
    for archive_name in ["parts.arc", "parts2.arc"] {
        let archive = open_path(root.join("GameData").join(archive_name), &cx);
        let mut revisions = std::collections::BTreeMap::new();
        for entry in archive.entries().iter().filter(|e| e.path.to_ascii_lowercase().ends_with(".model")) {
            let data = read_all(archive.open(entry.id, &cx).unwrap().as_ref(), cx.limits.max_entry_size).unwrap();
            let mesh = Mesh::parse(&data).unwrap_or_else(|e| panic!("{}: {e:#}", entry.path));
            assert!(mesh.bones.iter().all(|b| b.scale.iter().all(|n| n.is_finite())));
            *revisions.entry(mesh.version).or_insert(0usize) += 1;
        }
        assert!(revisions.contains_key(&2001));
        eprintln!("{archive_name} parsed mesh revisions {revisions:?}");
    }
    let tex = read("dress237_z4_accude_shadow.tex");
    let image = crysirror_image::decode_standard(crysirror_image::kiss_tex::payload(&tex).unwrap()).unwrap();
    assert_eq!((image.width, image.height), (512, 1024));
}
