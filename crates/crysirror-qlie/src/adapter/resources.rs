//! Resource work is isolated from VM/scene ownership. Decoders and archive
//! transforms are the same shared services used by authored engine projects.
use super::*;
use crysirror_core::service::{AssetReader, ClientStorage};
pub(super) struct Prepared {
    pub programs: BTreeMap<String, Arc<Program>>,
    pub text: BTreeMap<String, String>,
}
pub(super) fn prepare(
    storage: Arc<dyn ClientStorage>,
    assets: &BTreeMap<String, String>,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Prepared> {
    let mut programs = BTreeMap::new();
    let mut text = BTreeMap::new();
    let mut used = 0usize;
    for (logical, path) in assets {
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
    Ok(Prepared { programs, text })
}
pub(super) struct Member {
    pub name: String,
    pub bytes: Arc<[u8]>,
    pub metadata: serde_json::Value,
    pub slot: usize,
    pub nested: Option<Box<Bundle>>,
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
            images.push(Member {
                name: e.path.clone(),
                bytes,
                metadata: e.metadata.clone(),
                slot,
                nested,
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
