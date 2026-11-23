// SPDX-License-Identifier: MPL-2.0
//! Reads every object class with a reader in an installed Unity 4 player data
//! directory. Ignored by default: `UNITY_DATA=<..._Data> cargo test -- --ignored`.
use crysirror_core::service::AssetReader;
use crysirror_unity::{class_id as c, classes::*, SerializedFile};
use std::{collections::BTreeMap, sync::{Arc, Mutex}};

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

#[test]
#[ignore = "needs UNITY_DATA"]
fn every_supported_object_reads() {
    let dir = std::path::PathBuf::from(std::env::var_os("UNITY_DATA").unwrap());
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with("level") || n == "mainData" || (n.ends_with(".assets")))
        .collect();
    names.sort();
    let mut counts: BTreeMap<i32, usize> = BTreeMap::new();
    let mut fonts = 0;
    for name in &names {
        let file = SerializedFile::open(Arc::new(File(Mutex::new(std::fs::File::open(dir.join(name)).unwrap())))).unwrap();
        for info in file.objects.clone() {
            let bytes = file.bytes(&info).unwrap();
            let what = format!("{name} #{} class {}", info.path_id, info.class_id);
            let ok = match info.class_id {
                c::GAME_OBJECT => GameObject::read(&bytes).map(|_| ()),
                c::TRANSFORM => Transform::read(&bytes).map(|t| assert_eq!(t.children.len() * 8 + 60, bytes.len(), "{what}")),
                c::MONO_BEHAVIOUR => MonoBehaviour::read(&bytes).map(|_| ()),
                c::MONO_SCRIPT => MonoScript::read(&bytes).map(|_| ()),
                c::MATERIAL => Material::read(&bytes).map(|_| ()),
                c::CAMERA => Camera::read(&bytes).map(|_| ()),
                c::TEXTURE_2D => Texture2D::read(&bytes).and_then(|t| if t.width == 0 { Ok(()) } else { crysirror_unity::texture::decode(&t).map(|_| ()) }),
                c::FONT => Font::read(&bytes).map(|f| {
                    if !f.font_data.is_empty() {
                        fonts += 1;
                        let magic = &f.font_data[..4];
                        assert!(magic == [0, 1, 0, 0] || magic == b"OTTO" || magic == b"true" || magic == b"ttcf", "{what}: {magic:?}");
                    }
                }),
                c::MESH => Mesh::read(&bytes).map(|m| {
                    for s in &m.submeshes {
                        assert!(s.first_byte as usize / 2 + s.index_count as usize <= m.indices.len(), "{what}");
                    }
                    assert!(m.indices.iter().all(|i| (*i as usize) < m.positions.len()), "{what}");
                }),
                c::MESH_RENDERER => Renderer::read(&bytes).map(|_| ()),
                c::MESH_FILTER => mesh_filter(&bytes).map(|_| ()),
                c::SHADER => shader_name(&bytes).map(|_| ()),
                c::RESOURCE_MANAGER => resource_container(&bytes).map(|v| eprintln!("{name}: {} resources", v.len())),
                c::BUILD_SETTINGS => build_levels(&bytes).map(|v| eprintln!("levels: {v:?}")),
                _ => Ok(()),
            };
            ok.unwrap_or_else(|e| panic!("{what}: {e:#}"));
            *counts.entry(info.class_id).or_default() += 1;
        }
    }
    eprintln!("{} files, {fonts} embedded fonts, {counts:?}", names.len());
}

/// Every MonoBehaviour decodes under `UNITY_SCHEMA` and consumes its bytes.
#[test]
#[ignore = "needs UNITY_DATA and UNITY_SCHEMA"]
fn every_mono_behaviour_matches_the_schema() {
    let dir = std::path::PathBuf::from(std::env::var_os("UNITY_DATA").unwrap());
    let schema = crysirror_unity::managed::Schema::parse(
        &std::fs::read_to_string(std::env::var_os("UNITY_SCHEMA").unwrap()).unwrap(),
    )
    .unwrap();
    let open = |name: &str| SerializedFile::open(Arc::new(File(Mutex::new(std::fs::File::open(dir.join(name)).unwrap())))).unwrap();
    let mut files: BTreeMap<String, SerializedFile> = BTreeMap::new();
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with("level") || n == "mainData" || n.ends_with(".assets"))
        .collect();
    names.sort();
    for n in &names {
        files.insert(n.clone(), open(n));
    }
    let (mut ok, mut missing, mut failed) = (0, BTreeMap::<String, usize>::new(), Vec::new());
    for n in &names {
        let file = &files[n];
        for info in &file.objects {
            if info.class_id != c::MONO_BEHAVIOUR {
                continue;
            }
            let bytes = file.bytes(info).unwrap();
            let mb = MonoBehaviour::read(&bytes).unwrap();
            if mb.script.is_null() {
                continue;
            }
            let (owner, pid) = if mb.script.file_id == 0 {
                (file, mb.script.path_id)
            } else {
                let ext = file.external(mb.script.file_id).unwrap();
                let base = ext.rsplit('/').next().unwrap();
                match files.get(base) {
                    Some(f) => (f, mb.script.path_id),
                    None => continue,
                }
            };
            let script = MonoScript::read(&owner.bytes_of(pid).unwrap()).unwrap();
            let class = script.full_name();
            if !schema.scripts.contains_key(&class) {
                *missing.entry(class).or_default() += 1;
                continue;
            }
            match schema.decode(&class, &bytes[mb.body..]) {
                Ok(_) => ok += 1,
                Err(e) => failed.push(format!("{n} #{} {class}: {e:#}", info.path_id)),
            }
        }
    }
    failed.sort();
    failed.dedup_by(|a, b| a.split(' ').nth(2) == b.split(' ').nth(2));
    eprintln!("{ok} decoded; missing schemas {missing:?}");
    for f in &failed {
        eprintln!("FAIL {f}");
    }
    assert!(failed.is_empty());
}

/// The 5.6 player uses 64-bit object IDs, 32-bit object offsets and streamed textures.
#[test]
#[ignore = "requires COM3D2_DATA pointing to a Unity 5.6.4p2 installation"]
fn com3d2_boot_scene_and_streamed_logo() {
    let root = std::path::PathBuf::from(std::env::var_os("COM3D2_DATA").unwrap());
    let open = |name: &str| -> Arc<dyn AssetReader> {
        Arc::new(File(Mutex::new(
            std::fs::File::open(root.join(name)).unwrap(),
        )))
    };
    let main = SerializedFile::open(open("globalgamemanagers")).unwrap();
    assert_eq!((main.format, main.unity_version.as_str()), (17, "5.6.4p2"));
    let info = main
        .objects
        .iter()
        .find(|o| o.class_id == c::BUILD_SETTINGS)
        .unwrap();
    let levels = build_levels(&main.bytes(info).unwrap()).unwrap();
    assert_eq!(levels[6], "Assets/Scene/SceneLogo.unity");
    let resources = SerializedFile::open(open("resources.assets")).unwrap();
    let info = resources
        .objects
        .iter()
        .find(|o| {
            o.class_id == c::TEXTURE_2D && {
                let bytes = resources.bytes(o).unwrap();
                crysirror_unity::Reader::new(&bytes).string().unwrap() == "kiss_logo"
            }
        })
        .unwrap();
    let mut sibling = String::new();
    let img = crysirror_unity::texture::decode_56(&resources.bytes(info).unwrap(), |p| {
        sibling = p.to_owned();
        Ok(open(p))
    })
    .unwrap();
    assert_eq!((img.width, img.height), (1024, 484));
    assert_eq!(sibling, "resources.assets.resS");
    assert!(img.rgba.chunks_exact(4).any(|p| p[3] == 0));
    assert!(
        img.rgba
            .chunks_exact(4)
            .any(|p| p[3] == 255 && p[..3] != [255; 3])
    );
}
