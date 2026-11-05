//! Native filename/CRM selection policy. Shared archive and image crates own
//! indexing, range readers, raster decoding and differential composition.
use super::*;
use crysirror_image::crxd::Dependency;

pub(super) fn load_image(
    storage: &dyn ClientStorage,
    client: ClientId,
    name: &str,
) -> Result<ImageSurface> {
    ensure!(
        !name.is_empty() && name.len() < 260 && !name.contains('\0'),
        "Circus image resource name"
    );
    let (variant, container) = name
        .split_once(',')
        .map_or((name, None), |(variant, container)| {
            (variant, Some(container))
        });
    ensure!(
        !variant.is_empty() && !container.is_some_and(|v| v.is_empty() || v.contains(',')),
        "Circus compound image declaration"
    );
    //44FBA0 strips one native modifier immediately before the final extension
    //or string terminator before standalone/CRM lookup. Save names stay raw.
    let (variant,modifier)=native_image_variant(variant)?;
    let variant=variant.as_str();
    let path_variant=variant.rsplit_once('.').filter(|(_,extension)|extension.eq_ignore_ascii_case("crx")).map_or(variant,|(stem,_)|stem);
    let path = image_path(path_variant)?;
    let context = DecodeContext::new(client);
    let archive: Option<Box<dyn Archive>> = if storage
        .metadata(Mount::Asset, path.relative())?
        .is_some()
    {
        None
    } else if let Some(container) = container {
        let container = container.replace('\\', "/");
        let path = VfsPath::new(Mount::Asset, &format!("Advdata/GRP/{container}.crm"))?;
        Some(crysirror_resource::circus_crm::CircusCrm.open(storage.open_read(&path)?, &context)?)
    } else {
        None
    };
    let read_entry = |id: u32| -> Result<Vec<u8>> {
        let arc = archive
            .as_ref()
            .context("Circus CRM resource capability absent")?;
        crysirror_resource::read_all(arc.open(id, &context)?.as_ref(), 64 * 1024 * 1024)
    };
    let resolve_name = |name: &str| -> Result<Vec<u8>> {
        if let Some(archive) = &archive {
            let basename = name
                .rsplit(['\\', '/'])
                .next()
                .context("Circus CRM dependency filename")?;
            if let Some(id) = archive
                .resolve(basename)
                .or_else(|| archive.resolve(&format!("{basename}.crx")))
            {
                return read_entry(id);
            }
        }
        let native = name.replace('\\', "/");
        let native = native
            .strip_suffix(".CRX")
            .or_else(|| native.strip_suffix(".crx"))
            .unwrap_or(&native);
        let parent = variant
            .rsplit_once(['\\', '/'])
            .map_or("", |(parent, _)| parent);
        let relative = if native.contains('/') || parent.is_empty() {
            native.to_owned()
        } else {
            format!("{parent}/{native}")
        };
        let reader = storage.open_read(&image_path(&relative)?)?;
        crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)
    };
    let bytes = if let Some(archive) = &archive {
        let basename = variant
            .rsplit(['\\', '/'])
            .next()
            .context("Circus CRM variant basename")?;
        let id = archive
            .resolve(basename)
            .or_else(|| archive.resolve(&format!("{basename}.crx")))
            .context("Circus CRM variant absent")?;
        read_entry(id)?
    } else {
        let reader = storage.open_read(&path)?;
        crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)?
    };
    let mut pixel_format=crx_pixel_format(&bytes);
    let decoded = crysirror_image::crxd::decode_with_optional_offsets(&bytes, &mut |dependency| {
        let data=match dependency {
        Dependency::Name(name) => resolve_name(name).map(Some),
        Dependency::ArchiveOffset(offset) => {
            let Some(archive)=archive.as_ref() else{return Ok(None)};
            let entry = archive
                .entries()
                .iter()
                .find(|entry| entry.metadata["offset"].as_u64() == Some(u64::from(*offset)))
                .context("Circus CRXD offset is not a directory entry")?;
            read_entry(entry.id).map(Some)
        }
        }?;
        if let Some(data)=&data {
            if let Some(format)=crx_pixel_format(data).filter(|_|matches!(modifier,b'@'|b'*')){
                ensure!(pixel_format.is_none_or(|old|old==format),"Circus modifier resource pixel formats disagree");
                pixel_format=Some(format);
            }
        }
        Ok(data)
    })?;
    let surface=decoded.surface;
    //44F1D0@44F4AA..44F4CD dispatches only mode1(@) and2(*): mode3(?)
    //strips for resource identity but requests no colour transform.
    if !matches!(modifier,b'@'|b'*'){return Ok(surface)}
    //44F1D0 finishes differential composition before44E130/44E260. The
    //VM supplies exact encoded-byte lookup tables; the shared image worker
    //owns pixel traversal, bounds, immutable output and transparency policy.
    use crysirror_image::rgb_lut::{RgbLookup,RgbLookupProfile};
    let profile=match pixel_format.context("Circus image modifier lacks source pixel format")? {
        0=>RgbLookupProfile::StraightColorKey{color:[255,0,255]},
        1=>RgbLookupProfile::StraightNonzeroAlpha,
        other=>bail!("Circus image modifier indexed native format {other} needs an explicit palette-conversion policy"),
    };
    let (weight,divisor,offset)=if modifier==b'@'{(3u16,4u16,[0u16,0,16])}else{(7,8,[32,8,0])};
    let table=|offset:u16|(0u16..=255).map(|value|((value*weight/divisor)+offset) as u8).collect();
    RgbLookup{red:table(offset[0]),green:table(offset[1]),blue:table(offset[2]),profile}.apply(&surface,&||Ok(()))
}
fn native_image_variant(name:&str)->Result<(String,u8)> {
    let at=name.rfind(['.','\\','/']).filter(|at|name.as_bytes()[*at]==b'.').unwrap_or(name.len());
    let modifier=at.checked_sub(1).map(|at|name.as_bytes()[at]).filter(|byte|matches!(*byte,b'@'|b'*'|b'?'));
    let mut result=name.to_owned();
    if let Some(modifier)=modifier {
        result.remove(at-1);
        ensure!(!result.is_empty(),"Circus image modifier has no resource name");
        Ok((result,modifier))
    }else{Ok((result,0))}
}
fn crx_pixel_format(bytes:&[u8])->Option<u16> {
    let bytes=if bytes.starts_with(b"CRXG"){bytes}else if bytes.starts_with(b"CRXD"){bytes.get(32..)?}else{return None};
    bytes.starts_with(b"CRXG").then(||bytes.get(16..18).map(|raw|u16::from_le_bytes(raw.try_into().unwrap()))).flatten()
}
#[cfg(test)]
mod tests {
    use super::*;
    struct ReadOnly(std::path::PathBuf);
    impl ClientStorage for ReadOnly {
        fn read(&self, path: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            ensure_asset(path)?;
            std::fs::read(self.0.join(path.relative()))
                .map_err(|e| crysirror_core::Error::Io(e.to_string()))
        }
        fn open_read(
            &self,
            path: &VfsPath,
        ) -> crysirror_core::Result<Arc<dyn crysirror_core::service::AssetReader>> {
            Ok(Arc::new(crysirror_core::service::MemoryAssetReader(
                self.read(path)?.into(),
            )))
        }
        fn metadata(
            &self,
            mount: Mount,
            relative: &str,
        ) -> crysirror_core::Result<Option<crysirror_core::service::StorageMetadata>> {
            let path = VfsPath::new(mount, relative)?;
            ensure_asset(&path)?;
            match std::fs::metadata(self.0.join(path.relative())) {
                Ok(value) => Ok(Some(crysirror_core::service::StorageMetadata {
                    is_directory: value.is_dir(),
                    size: Some(value.len()),
                    created_ms: None,
                    modified_ms: None,
                    accessed_ms: None,
                })),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(crysirror_core::Error::Io(error.to_string())),
            }
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
        fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
            Err(crysirror_core::Error::Unsupported("test enumeration"))
        }
    }
    fn ensure_asset(path: &VfsPath) -> crysirror_core::Result<()> {
        if path.mount() == Mount::Asset {
            Ok(())
        } else {
            Err(crysirror_core::Error::ReadOnly)
        }
    }
    #[test]
    #[ignore = "requires owner's licensed DC4PH assets via CIRCUS_ASSET_ROOT"]
    fn installed_crm_variants_decode_through_shared_differential_resources() {
        let root = std::env::var_os("CIRCUS_ASSET_ROOT").expect("CIRCUS_ASSET_ROOT");
        let storage = ReadOnly(root.into());
        for name in [
            "BST/skr_a1_a01,BST/skr_a",
            "BST/skr_a1_a06,BST/skr_a",
            "BST/skr_a3_a01,BST/skr_a",
        ] {
            let image = load_image(&storage, ClientId(77), name).unwrap();
            image.validate().unwrap();
            assert!(image.width > 0 && image.height > 0);
            assert!(image.rgba.chunks_exact(4).any(|p| p[3] != 0));
            eprintln!("{name}: {}x{}", image.width, image.height);
        }
    }
}
