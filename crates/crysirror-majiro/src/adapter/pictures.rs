// Copyright (c) 2026 CrySirror contributors. MIT.
//! 4182C0/419620/41ADC0/41AF30 resource policy over shared format decoders,
//! immutable image leases and Canvas. No bitmap algorithm or device lives here.
use super::*;
use crysirror_core::{image_processing::ImageResource, service::ClientStorage};
use crysirror_image::{DecoderRegistry, ImageSurface};

pub(super) struct PendingPicture {
    event: Event,
    task: Task<PictureResult>,
}
enum PictureResult {
    Size([u32; 2]),
    Image(Arc<dyn ImageResource>),
}
struct PictureKeys {
    client: ClientId,
    key: Option<crysirror_resource::KeyMaterial>,
}
impl crysirror_resource::KeyProvider for PictureKeys {
    fn key(&self, client: ClientId, id: &str) -> Result<crysirror_resource::KeyMaterial> {
        ensure!(
            client == self.client && id == "majiro/rct",
            "Majiro picture key scope"
        );
        self.key
            .clone()
            .context("Majiro encrypted RCT requires its script-provided picture password")
    }
}
// Only an immutable ownership envelope. Decoding and coverage conversion are
// shared image operations, completed on the resource worker before publication.
struct PictureLease {
    client: ClientId,
    size: [u32; 2],
    pixels: Arc<[u8]>,
    _storage: Arc<dyn crysirror_core::service::AssetReader>,
}
impl std::fmt::Debug for PictureLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MajiroPictureLease")
            .field("size", &self.size)
            .finish()
    }
}
impl ImageResource for PictureLease {
    fn client(&self) -> ClientId {
        self.client
    }
    fn size(&self) -> [u32; 2] {
        self.size
    }
    fn rgba(&self) -> Arc<[u8]> {
        self.pixels.clone()
    }
    fn opaque(&self) -> bool {
        true
    }
    fn pixel_at(&self, point: [u32; 2]) -> crysirror_core::Result<Option<[u8; 4]>> {
        crysirror_core::image_processing::rgba_pixel_at(self.size, &self.pixels, point)
    }
}
fn basename(bytes: &[u8]) -> Result<String> {
    let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
    ensure!(
        !bytes.is_empty() && bytes.len() <= 4096,
        "Majiro picture filename extent"
    );
    let name = encoding_rs::SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .context("Majiro picture filename CP932")?;
    let base = name.rsplit(['/', '\\']).next().unwrap();
    let stem = base.rsplit_once('.').map_or(base, |(stem, _)| stem);
    let (encoded, _, errors) = encoding_rs::SHIFT_JIS.encode(stem);
    ensure!(
        !stem.is_empty()
            && !errors
            && encoded.len() <= 251
            && !stem.contains(':')
            && !stem.chars().any(char::is_control),
        "Majiro picture basename"
    );
    Ok(stem.to_owned())
}
fn provider<'a>(assets: &'a BTreeMap<String, Vec<String>>, name: &str) -> Result<Option<&'a str>> {
    let Some(paths) = assets.get(&name.to_ascii_lowercase()) else {
        return Ok(None);
    };
    ensure!(
        paths.len() == 1,
        "Majiro duplicate picture providers require native archive precedence: {name}"
    );
    Ok(Some(&paths[0]))
}
fn resolve(assets: &BTreeMap<String, Vec<String>>, stem: &str) -> Result<String> {
    //4182C0 ignores the supplied extension and searches this exact order.
    for ext in ["png", "bmp", "rct", "rc8"] {
        if let Some(path) = provider(assets, &format!("{stem}.{ext}"))? {
            return Ok(path.to_owned());
        }
    }
    bail!("Majiro picture is absent from scoped resources: {stem}")
}
fn read(storage: &dyn ClientStorage, path: &str) -> crysirror_core::Result<Vec<u8>> {
    let path = VfsPath::new(Mount::Asset, path)?;
    let reader = storage.open_asset(&path)?;
    crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)
        .map_err(|e| Error::Io(e.to_string()))
}
impl MajiroVm {
    pub(super) fn native_picture(
        &mut self,
        event: &Event,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        let Event::Native {
            hash,
            arguments,
            returns,
            ..
        } = event
        else {
            bail!("Majiro picture event")
        };
        ensure!(arguments.len() == 1, "Majiro picture ABI");
        let create = *hash == 0xdad96289;
        if let Some(pending) = &self.picture {
            ensure!(&pending.event == event, "Majiro stale picture completion");
        } else {
            if create {
                ensure!(
                    self.pages.len() < 256 && self.next_page <= i32::MAX as u32,
                    "Majiro page handle budget"
                );
            }
            let stem = basename(arguments[0].bytes()?)?;
            let assets = self.module_catalog.assets.clone();
            let path = resolve(&assets, &stem)?;
            ensure!(
                !create || path.to_ascii_lowercase().ends_with(".rct"),
                "Majiro file page currently requires true-colour RCT; indexed/BMP/PNG page policy is not connected"
            );
            let storage = host.storage(self.client).map_err(core_error)?;
            let client = self.client;
            let key = self.picture_key.clone();
            let context = self.picture_context.clone();
            let permit = match self.pool.reserve() {
                Ok(permit) => permit,
                Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => return Ok(None),
                Err(error) => return Err(error),
            };
            let task = permit.spawn(move |cancel| {
                cancel.check()?;
                let reader = storage.open_asset(&VfsPath::new(Mount::Asset, &path)?)?;
                let mut header = [0u8; 20];
                let mut count = 0;
                while count < header.len() {
                    cancel.check()?;
                    let n = reader.read_at(count as u64, &mut header[count..])?;
                    ensure!(
                        n <= header.len() - count,
                        "Majiro picture header reader extent"
                    );
                    if n == 0 {
                        break;
                    }
                    count += n;
                }
                if !create
                    && (header.starts_with(b"\x98\x5a\x92\x9aT")
                        || header.starts_with(b"\x98\x5a\x92\x9a8"))
                {
                    //Both native RC8/RCT headers expose unsigned width/height
                    //at8/12; metadata queries require neither pixels nor keys.
                    ensure!(count == 20, "Majiro truncated picture header");
                    ensure!(
                        (header[4] == b'T'
                            && matches!(header[5], b'C' | b'S')
                            && matches!(&header[6..8], b"00" | b"01"))
                            || &header[4..8] == b"8_00",
                        "Majiro unsupported picture header"
                    );
                    let size = [
                        u32::from_le_bytes(header[8..12].try_into().unwrap()),
                        u32::from_le_bytes(header[12..16].try_into().unwrap()),
                    ];
                    crysirror_image::rgba_len(size[0], size[1])?;
                    return Ok(PictureResult::Size(size));
                }
                ensure!(
                    !create || header.starts_with(b"\x98\x5a\x92\x9aT"),
                    "Majiro true-colour file page requires an RCT header"
                );
                let mut context = context.with_cancellation(cancel.clone());
                context.keys = Arc::new(PictureKeys { client, key });
                //A normal native page is BGR24. The separate withalfa syscall
                //loads the '_' companion; it must not leak into this call.
                context
                    .parameters
                    .insert("majiro/rct/load_mask".into(), json!(false));
                let decoded = if create {
                    ensure!(count == 20, "Majiro truncated RCT header");
                    crysirror_image::rgba_len(
                        u32::from_le_bytes(header[8..12].try_into().unwrap()),
                        u32::from_le_bytes(header[12..16].try_into().unwrap()),
                    )? as u64
                } else {
                    64 * 1024 * 1024
                };
                let encoded = reader.len()?;
                ensure!(
                    encoded <= 64 * 1024 * 1024,
                    "Majiro encoded picture byte budget"
                );
                //The shared context budget is common to all tasks, including
                //completed-but-interrupted frames. Queue slots alone cannot
                //bound retained results: worker completion frees those slots.
                context.with_temporary_bytes(encoded + decoded * 3, || {
                    let bytes = crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)?;
                    cancel.check()?;
                    let mut registry = DecoderRegistry::default();
                    crysirror_formats_arc::register_images(&mut registry)?;
                    let source = |requested: &str| {
                        cancel.check().map_err(|e| Error::Io(e.to_string()))?;
                        let name = requested.rsplit('/').next().ok_or(Error::NotFound)?;
                        let path = provider(&assets, name)
                            .map_err(|e| Error::Io(e.to_string()))?
                            .ok_or(Error::NotFound)?;
                        read(storage.as_ref(), path)
                    };
                    let image =
                        registry.decode_with(&path, &bytes, Some(&source), Some(&context), None)?;
                    if !create {
                        return Ok(PictureResult::Size([image.width, image.height]));
                    }
                    ensure!(
                        image.width <= 8192 && image.height <= 8192,
                        "Majiro page geometry profile"
                    );
                    let image: ImageSurface = image.apply_effect_cancellable(
                        &crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                        &|| cancel.check(),
                    )?;
                    cancel.check()?;
                    let pixels: Arc<[u8]> = image.rgba.into();
                    let storage = context.memory_shared(pixels.clone())?;
                    Ok(PictureResult::Image(Arc::new(PictureLease {
                        client,
                        size: [image.width, image.height],
                        pixels,
                        _storage: storage,
                    })))
                })
            })?;
            self.picture = Some(PendingPicture {
                event: event.clone(),
                task,
            });
            return Ok(None);
        }
        let Some(result) = self.picture.as_mut().unwrap().task.try_take()? else {
            return Ok(None);
        };
        let value = match result {
            PictureResult::Size(size) => size[usize::from(*hash == 0xce94e6ca)] as i32,
            PictureResult::Image(image) => {
                let size = image.size();
                let id = self.next_page;
                let upload = host
                    .publish_image_resource(self.client, id, image)
                    .map_err(core_error)?;
                let path = upload.path.relative().to_owned();
                let batch = json!({"canvases":[{"op":"create","id":id,"width":size[0],"height":size[1]},
                    {"op":"set","id":id,"layer":{"id":0,"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":path}}}]});
                if let Err(error) = host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)
                {
                    let _ = host.release_image_resource(self.client, &path);
                    return Err(core_error(error));
                }
                self.api_calls += 2;
                self.pages.insert(id, size);
                self.page_resources.insert(id, path);
                self.next_page += 1;
                id as i32
            }
        };
        self.picture = None;
        Ok(Some(if *returns {
            vec![Value::Integer(value)]
        } else {
            vec![]
        }))
    }
}
