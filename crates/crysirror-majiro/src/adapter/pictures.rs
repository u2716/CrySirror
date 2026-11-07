// Copyright (c) 2026 CrySirror contributors. MIT.
//! 4182C0/419620/41ADC0/41AF30 resource policy over shared format decoders,
//! immutable image leases and Canvas. No bitmap algorithm or device lives here.
//! Unadmitted3E11F2C5 (401B60 ->440760) returns the existing primary-picture
//! global5725D8; it does not allocate/decode a picture. Its future mapping needs
//! the shared primary Canvas and presentation lifetime, not a fabricated page ID.
use super::*;
use crysirror_core::{image_processing::ImageResource, service::ClientStorage};
use crysirror_image::{DecoderRegistry, ImageSurface};

pub(super) struct PendingPicture {
    event: Event,
    task: Task<PictureResult>,
}
enum PictureResult {
    Size([u32; 2]),
    Image(Arc<dyn ImageResource>, bool, bool),
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
    opaque: bool,
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
        self.opaque
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
        let sprite = *hash == 0xc2c3c4f4;
        if sprite {
            self.sprite_preflight(arguments)?;
        } else {
            ensure!(arguments.len() == 1, "Majiro picture ABI");
        }
        let create = matches!(*hash, 0xdad96289 | 0x57efa275 | 0xc2c3c4f4);
        let masked = matches!(*hash, 0x57efa275 | 0xc2c3c4f4);
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
            let mut mask_path = None;
            if masked {
                for ext in ["png", "bmp", "rct", "rc8"] {
                    if let Some(path) = provider(&assets, &format!("{stem}_.{ext}"))? {
                        mask_path = Some(path.to_owned());
                        break;
                    }
                }
            }
            ensure!(
                mask_path
                    .as_ref()
                    .is_none_or(|path| path.to_ascii_lowercase().ends_with(".rc8")),
                "Majiro mask requires reviewed identity RC8; PNG/BMP/RCT mask profile is not connected"
            );
            let has_mask = mask_path.is_some();
            ensure!(
                !create
                    || [".rct", ".rc8"]
                        .iter()
                        .any(|ext| path.to_ascii_lowercase().ends_with(ext)),
                "Majiro file page requires RCT/identity-index RC8; BMP/PNG page policy is not connected"
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
            let task =
                permit.spawn(move |cancel| {
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
                        !create || header.starts_with(b"\x98\x5a\x92\x9aT") || header.starts_with(b"\x98\x5a\x92\x9a8_00"),
                        "Majiro file page requires a reviewed RCT/RC8 header"
                    );
                    ensure!(
                        !masked || &header[6..8] == b"00",
                        "Majiro differential RCT sprite masking profile is not connected"
                    );
                    let mut context = context.with_cancellation(cancel.clone());
                    context.keys = Arc::new(PictureKeys { client, key });
                    //A normal native page is BGR24. The separate withalfa syscall
                    //loads the '_' companion; it must not leak into this call.
                    context
                        .parameters
                        .insert("majiro/rct/load_mask".into(), json!(masked));
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
                        let bytes = read(storage.as_ref(), path)?;
                        if masked
                            && name
                                .rsplit_once('.')
                                .is_some_and(|(stem, _)| stem.ends_with('_'))
                        {
                            //Native43DD80 requires an indexed plane. The
                            //shared normalized mask profile is exact for an
                            //identity RC8 palette; other palette/index layouts
                            //remain explicit instead of treating RGB as indices.
                            if !(bytes.starts_with(b"\x98\x5a\x92\x9a8_00")
                                && bytes.len() >= 788
                                && bytes[20..788]
                                    .chunks_exact(3)
                                    .enumerate()
                                    .all(|(i, rgb)| rgb == [i as u8; 3]))
                            {
                                return Err(Error::Unsupported(
                                    "Majiro sprite mask requires the reviewed indexed RC8 palette",
                                ));
                            }
                        }
                        Ok(bytes)
                    };
                    let indexed=header.starts_with(b"\x98\x5a\x92\x9a8_00");
                    //440610/447200 retain identity indices independently of
                    //coverage. Shared RC8 RGBA is lossless for this profile:
                    //each stored RGB byte equals its original palette index.
                    ensure!(!create || !indexed || (bytes.len()>=788 && bytes[20..788]
                        .chunks_exact(3).enumerate().all(|(i,rgb)|rgb==[i as u8;3])),
                        "Majiro non-identity indexed palette requires a retained index/palette resource");
                    let mut image =
                        registry.decode_with(&path, &bytes, Some(&source), Some(&context), None)?;
                    if !create {
                        return Ok(PictureResult::Size([image.width, image.height]));
                    }
                    ensure!(
                        image.width <= 8192 && image.height <= 8192,
                        "Majiro page geometry profile"
                    );
                    if create && indexed {
                        if let Some(mask_path)=&mask_path {
                            let name=mask_path.rsplit('/').next().context("Majiro RC8 mask name")?;
                            let mask_bytes=source(name).map_err(core_error)?;
                            let mask=registry.decode_with(mask_path,&mask_bytes,Some(&source),Some(&context),None)?;
                            image=image.copy_channel(&mask,0,8,true,&||cancel.check())?;
                        }
                    }
                    let image: ImageSurface = if masked {
                        image
                    } else {
                        image.apply_effect_cancellable(
                            &crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                            &|| cancel.check(),
                        )?
                    };
                    cancel.check()?;
                    let pixels: Arc<[u8]> = image.rgba.into();
                    let storage = context.memory_shared(pixels.clone())?;
                    Ok(PictureResult::Image(Arc::new(PictureLease {
                        client,
                        size: [image.width, image.height],
                        pixels,
                        _storage: storage,
                        opaque: !has_mask,
                    }),has_mask,indexed))
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
            PictureResult::Image(image, has_mask, indexed) => {
                let size = image.size();
                let id = self.next_page;
                let upload = host
                    .publish_image_resource(self.client, id, image.clone())
                    .map_err(core_error)?;
                let path = upload.path.relative().to_owned();
                let mut batch = json!({"canvases":[{"op":"create","id":id,"width":size[0],"height":size[1]},
                    {"op":"set","id":id,"layer":{"id":0,"rect":[0,0,size[0],size[1]],"paint":{"kind":"image","asset":path}}}]});
                let sprite_state = if sprite {
                    let prepared = self.sprite_resource_layer(arguments, size, &path, id, true);
                    let (layer, state) = match prepared {
                        Ok(value) => value,
                        Err(error) => {
                            let _ = host.release_image_resource(self.client, &path);
                            return Err(error);
                        }
                    };
                    batch["layers"] = json!([layer]);
                    Some(state)
                } else {
                    None
                };
                if let Err(error) = host.submit_engine(self.client, 1, &serde_json::to_vec(&batch)?)
                {
                    let _ = host.release_image_resource(self.client, &path);
                    return Err(core_error(error));
                }
                self.api_calls += 2;
                self.pages.insert(id, size);
                self.page_bitmaps.insert(
                    id,
                    page_bitmaps::Bitmap {
                        image,
                        indexed,
                        alpha: if has_mask {
                            page_bitmaps::Alpha::Inline
                        } else {
                            page_bitmaps::Alpha::None
                        },
                    },
                );
                self.page_resources.insert(id, path);
                self.next_page += 1;
                if let Some(state) = sprite_state {
                    let handle = self.next_sprite;
                    self.next_sprite += 1;
                    self.sprites.insert(handle, state);
                    self.sprite_order.insert(0, handle);
                    handle as i32
                } else {
                    id as i32
                }
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
