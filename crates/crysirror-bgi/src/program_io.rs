//! BP module reads through the client's shared storage/archive capabilities.
use crate::Config;
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
    state::ClientId,
};
use crysirror_resource::{ArchiveFormat, DecodeContext};
use std::sync::Arc;

pub(crate) fn load(
    storage: Arc<dyn ClientStorage>,
    client: ClientId,
    config: &Config,
    archive: &str,
    file: &str,
) -> Result<Vec<u8>> {
    let bytes = read_raw(storage.as_ref(), client, config, archive, file)?
        .with_context(|| format!("BGI module missing: {archive}:{file}"))?;
    let bytes = if bytes.starts_with(b"DSC FORMAT 1.00") {
        crysirror_resource::dsc::decode_with_limit(&bytes, 16 * 1024 * 1024)?
    } else {
        bytes
    };
    ensure!(!bytes.is_empty(), "BGI module is empty");
    Ok(bytes)
}

pub(crate) fn relative(_config: &Config, file: &str) -> Result<String> {
    let file = file.replace('\\', "/");
    let file = file.strip_prefix("C:/CrySirror/").unwrap_or(&file);
    Ok(VfsPath::new(Mount::Save, file)?.relative().to_owned())
}

fn resolve(storage: &dyn ClientStorage, path: &VfsPath) -> Result<Option<VfsPath>> {
    match storage.metadata(path.mount(), path.relative()) {
        Ok(Some(metadata)) => return Ok((!metadata.is_directory).then_some(path.clone())),
        Ok(None) | Err(crysirror_core::Error::NotFound) => (),
        Err(crysirror_core::Error::Unsupported(_)) => (),
        Err(error) => return Err(error.into()),
    }
    match crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage, path) {
        Err(error)
            if matches!(
                error.downcast_ref::<crysirror_core::Error>(),
                Some(crysirror_core::Error::NotFound)
            ) =>
        {
            Ok(None)
        }
        result => result,
    }
}

pub(crate) fn read_raw(
    storage: &dyn ClientStorage,
    client: ClientId,
    config: &Config,
    archive: &str,
    file: &str,
) -> Result<Option<Vec<u8>>> {
    let file = relative(config, file)?;
    // The writable game namespace shadows the immutable installation, as
    // native save/config files do, without acquiring write access to assets.
    for path in [
        VfsPath::new(Mount::Save, &file)?,
        VfsPath::new(Mount::Asset, &config.asset(&file)?)?,
    ] {
        if let Some(path) = resolve(storage, &path)? {
            let bytes = if path.mount() == Mount::Asset {
                crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 64 * 1024 * 1024)?
            } else {
                storage.read(&path)?
            };
            ensure!(bytes.len() <= 64 * 1024 * 1024, "BGI file byte limit");
            return Ok(Some(bytes));
        }
    }
    if archive.is_empty() {
        if let Ok(mounts) = storage.list(Mount::Asset, "__bgi") {
            for mount in mounts {
                let path = VfsPath::new(Mount::Asset, &format!("__bgi/{mount}/{file}"))?;
                if let Some(path) = resolve(storage, &path)? {
                    return Ok(Some(crysirror_resource::read_all(
                        storage.open_asset(&path)?.as_ref(),
                        64 * 1024 * 1024,
                    )?));
                }
            }
        }
        return Ok(None);
    }
    let archive_path = config.asset(archive)?;
    for candidate in [archive_path.clone(), format!("{archive_path}.arc")] {
        if let Some(mount) = storage.container_mount(&candidate)? {
            let path = VfsPath::new(Mount::Asset, &format!("{mount}/{file}"))?;
            if let Some(path) = resolve(storage, &path)? {
                return Ok(Some(crysirror_resource::read_all(
                    storage.open_asset(&path)?.as_ref(),
                    64 * 1024 * 1024,
                )?));
            }
            continue;
        }
        let path = VfsPath::new(Mount::Asset, &candidate)?;
        let Some(path) = resolve(storage, &path)? else {
            continue;
        };
        let context = DecodeContext::new(client);
        let archive = crysirror_formats_arc::BgiArc.open(storage.open_asset(&path)?, &context)?;
        let name = file.to_lowercase();
        let Some(entry) = archive
            .entries()
            .iter()
            .find(|entry| entry.path.to_lowercase() == name)
        else {
            return Ok(None);
        };
        return Ok(Some(crysirror_resource::read_all(
            archive.open(entry.id, &context)?.as_ref(),
            64 * 1024 * 1024,
        )?));
    }
    Ok(None)
}

pub(crate) fn decode(bytes: Vec<u8>) -> Result<Vec<u8>> {
    if bytes.starts_with(b"DSC FORMAT 1.00") {
        crysirror_resource::dsc::decode(&bytes)
    } else if bytes.starts_with(b"CompressedBG___") {
        Ok(crysirror_image::cbg::decode_cbg_native_buffer(&bytes)?)
    } else {
        Ok(bytes)
    }
}

/// Runtime ABI selection from the original two code resources. This reads no
/// executable and does not consult any external source or manifest.
pub(crate) fn native_profile(
    storage: &dyn ClientStorage,
    client: ClientId,
    config: &Config,
    bootstrap: &[u8],
) -> Result<crysirror_bgi_vm::NativeAbiProfile> {
    fn code_id(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf29ce484222325, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        })
    }
    if bootstrap.len() == 10176 && code_id(bootstrap) == 0x9f5e9248ec3ad577 {
        if let Some(title) = read_raw(storage, client, config, "sysprg.arc", "title._bp")? {
            let title = if title.starts_with(b"DSC FORMAT 1.00") {
                crysirror_resource::dsc::decode_with_limit(&title, 16 * 1024 * 1024)?
            } else {
                title
            };
            if title.len() == 26272 && code_id(&title) == 0xc6bdd0d97d4f4893 {
                return Ok(crysirror_bgi_vm::NativeAbiProfile::Amachoco);
            }
        }
    }
    Ok(crysirror_bgi_vm::NativeAbiProfile::Tayutama)
}
