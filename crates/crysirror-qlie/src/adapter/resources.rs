//! Resource work is isolated from VM/scene ownership. Decoders and archive
//! transforms are the same shared services used by authored engine projects.
use super::*;
use crysirror_core::service::{AssetReader, ClientStorage};
pub(super) struct Prepared {
    pub programs: BTreeMap<String, Arc<Program>>,
    pub text: BTreeMap<String, String>,
    pub pack_slots: BTreeSet<u32>,
}
pub(super) fn prepare(
    storage: Arc<dyn ClientStorage>,
    assets: &BTreeMap<String, String>,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Prepared> {
    let mut programs = BTreeMap::new();
    let mut text = BTreeMap::new();
    let mut used = 0usize;
    let mut pack_slots = BTreeSet::new();
    let mut container_dirs = BTreeSet::from([String::new(), "GameData".into()]);
    for (logical, path) in assets {
        cancel.check()?;
        if let Some((source, _)) = storage.serving_container(&VfsPath::new(Mount::Asset, path)?) {
            if let Some((dir, _)) = source.rsplit_once('/') {
                container_dirs.insert(dir.into());
            }
            if let Some(slot) = pack_slot(&source) {
                pack_slots.insert(slot);
            }
        }
        if !logical.ends_with(".s") && !logical.ends_with(".txt") {
            continue;
        }
        cancel.check()?;
        let r = storage.open_asset(&VfsPath::new(Mount::Asset, path)?)?;
        let bytes = crysirror_resource::read_all(r.as_ref(), 16 * 1024 * 1024)?;
        used += bytes.len();
        ensure!(
            used <= 32 * 1024 * 1024,
            "QLIE script/text preparation budget"
        );
        if logical.ends_with(".s") {
            programs.insert(
                logical.clone(),
                Arc::new(
                    Program::parse(logical, &bytes)
                        .with_context(|| format!("QLIE source {logical}"))?,
                ),
            );
        } else {
            text.insert(logical.clone(), crate::program::decode_text(&bytes)?);
        }
    }
    cancel.check()?;
    // A mounted but fully overridden data archive still occupies its native
    // slot. Query retained mount metadata; do not reread a package's bytes.
    for directory in container_dirs {
        for slot in 0..32 {
            for name in [format!("data{slot}.pack"), format!("Data{slot}.pack")] {
                let source = if directory.is_empty() {
                    name
                } else {
                    format!("{directory}/{name}")
                };
                if storage.container_mount(&source)?.is_some() {
                    pack_slots.insert(slot);
                }
            }
        }
    }
    Ok(Prepared {
        programs,
        text,
        pack_slots,
    })
}
fn pack_slot(source: &str) -> Option<u32> {
    let source = source.rsplit('/').next()?.to_ascii_lowercase();
    let slot = source
        .strip_prefix("data")?
        .strip_suffix(".pack")?
        .parse::<u32>()
        .ok()?;
    (slot < 32).then_some(slot)
}
pub(super) struct Member {
    pub name: String,
    pub bytes: Arc<[u8]>,
    pub metadata: serde_json::Value,
    pub slot: usize,
    pub nested: Option<Box<Bundle>>,
    pub model: Option<Arc<crysirror_asset::tqmo::Model>>,
    pub scene_model: Option<Arc<crysirror_scene::legacy_mesh::Model>>,
}
pub(super) struct Bundle {
    pub id: usize,
    pub animation: animation::Animation,
    pub images: Vec<Member>,
    pub sounds: Vec<Member>,
}
pub(super) enum Loaded {
    Blob(Arc<[u8]>),
    Bundle(Bundle),
}
pub(super) struct Memory(pub Arc<[u8]>);
impl AssetReader for Memory {
    fn len(&self) -> crysirror_core::Result<u64> {
        Ok(self.0.len() as u64)
    }
    fn read_at(&self, at: u64, out: &mut [u8]) -> crysirror_core::Result<usize> {
        let at = usize::try_from(at).map_err(|_| Error::Invalid("QLIE memory offset"))?;
        if at >= self.0.len() {
            return Ok(0);
        }
        let n = out.len().min(self.0.len() - at);
        out[..n].copy_from_slice(&self.0[at..at + n]);
        Ok(n)
    }
}
pub(super) fn load(
    storage: Arc<dyn ClientStorage>,
    client: ClientId,
    path: &str,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Loaded> {
    cancel.check()?;
    let source = storage.open_asset(&VfsPath::new(Mount::Asset, path)?)?;
    let input: Arc<[u8]> = crysirror_resource::read_all(source.as_ref(), 64 * 1024 * 1024)?.into();
    if !path.to_ascii_lowercase().ends_with(".b") {
        return Ok(Loaded::Blob(input));
    }
    let mut slots = 0;
    Ok(Loaded::Bundle(bundle(
        input,
        client,
        cancel,
        0,
        &mut slots,
        usize::MAX,
    )?))
}
fn bundle(
    input: Arc<[u8]>,
    client: ClientId,
    cancel: &crysirror_tasks::Cancellation,
    depth: usize,
    slots: &mut usize,
    id: usize,
) -> Result<Bundle> {
    ensure!(depth < 8, "QLIE nested animation depth");
    let format = crysirror_formats_arc::legacy_archive("qlie/abmp10")
        .context("shared ABMP parser missing")?;
    let cx = crysirror_resource::DecodeContext::new(client).with_cancellation(cancel.clone());
    let archive = format.open(Arc::new(Memory(input)), &cx)?;
    let mut metadata = None;
    let mut wide = true;
    let mut images = Vec::new();
    let mut sounds = Vec::new();
    for e in archive.entries() {
        cancel.check()?;
        if e.path == "metadata.bin" || e.path == "unknown.dat" {
            wide = matches!(
                e.metadata["section"].as_str(),
                Some("abdata14" | "abdata15")
            );
            metadata = Some(crysirror_resource::read_all(
                archive.open(e.id, &cx)?.as_ref(),
                8 * 1024 * 1024,
            )?);
        } else if e.metadata["section"] == "image" {
            let slot = *slots;
            *slots += 1;
            ensure!(*slots <= 1024, "QLIE embedded image slot limit");
            let mut bytes: Arc<[u8]> =
                crysirror_resource::read_all(archive.open(e.id, &cx)?.as_ref(), 64 * 1024 * 1024)?
                    .into();
            let nested = if e.metadata["kind"] == 6 && !bytes.is_empty() {
                let child = bundle(bytes.clone(), client, cancel, depth + 1, slots, slot)?;
                bytes = Arc::from([]);
                Some(Box::new(child))
            } else {
                None
            };
            let model = if e.metadata["kind"] == 8 && !bytes.is_empty() {
                cancel.check()?;
                let mut model = crysirror_asset::tqmo::Model::parse(&bytes)
                    .with_context(|| format!("QLIE embedded model {}", e.path))?;
                model.upgrade_legacy()?;
                bytes = Arc::from([]);
                Some(Arc::new(model))
            } else { None };
            let scene_model = if let Some(model) = &model {
                let policies = model.materials.iter().map(|material| {
                    use crysirror_image::rgb_lut::ConstantByteBlend as Mode;
                    let mode = match material.native_render_flags[0].min(7) {
                        0 => Mode::Normal, 1 => Mode::Screen, 2 => Mode::Multiply,
                        3 => Mode::Difference, 4 => Mode::Lighten, 5 => Mode::Darken,
                        6 => Mode::Add, _ => Mode::Subtract,
                    };
                    let blend=crysirror_scene::layer::Blend::StoredByte {
                        mode,force_source_opaque:false,force_output_opaque:true,
                    };
                    // Native material sampler flags0/1 both select nearest;
                    // the ABMP track selector may override during evaluation.
                    Ok((blend,false))
                }).collect::<Result<Vec<_>>>()?;
                Some(Arc::new(model.to_scene_model(&policies)?))
            } else { None };
            images.push(Member {
                name: e.path.clone(),
                bytes,
                metadata: e.metadata.clone(),
                slot,
                nested,
                model,
                scene_model,
            });
        } else if e.metadata["section"] == "sound" {
            sounds.push(Member {
                name: e.path.clone(),
                bytes: crysirror_resource::read_all(
                    archive.open(e.id, &cx)?.as_ref(),
                    16 * 1024 * 1024,
                )?
                .into(),
                metadata: e.metadata.clone(),
                slot: usize::MAX,
                nested: None,
                model: None,
                scene_model: None,
            });
        } else if e.size > 0 {
            bail!("unsupported ABMP member {} ({})", e.path, e.metadata);
        }
    }
    let animation = animation::Animation::parse(
        &metadata.context("ABMP native animation metadata absent")?,
        wide,
    )?;
    cancel.check()?;
    Ok(Bundle {
        id,
        animation,
        images,
        sounds,
    })
}
impl Bundle {
    pub fn slot(&self, slot: usize) -> Option<&Member> {
        for member in &self.images {
            if member.slot == slot {
                return Some(member);
            }
            if let Some(child) = &member.nested {
                if let Some(value) = child.slot(slot) {
                    return Some(value);
                }
            }
        }
        None
    }
    pub fn slot_mut(&mut self, slot: usize) -> Option<&mut Member> {
        for member in &mut self.images {
            if member.slot == slot {
                return Some(member);
            }
            if let Some(child) = &mut member.nested {
                if let Some(value) = child.slot_mut(slot) {
                    return Some(value);
                }
            }
        }
        None
    }
}
