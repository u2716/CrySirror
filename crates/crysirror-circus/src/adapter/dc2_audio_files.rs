//! DC2 426760/427BA0 file policy over scoped engine storage. A VOICE name
//! first uses the shared multivolume archive; loose files replace their final
//! extension and fall back from PCM to WAV only when opening PCM would fail.
use super::*;

pub(super) fn prepare(
    storage: Arc<dyn ClientStorage>,
    client: ClientId,
    cached: Option<Arc<dyn Archive>>,
    name: &str,
) -> Result<Prepared> {
    let native = name.replace('\\', "/");
    ensure!(!native.is_empty(), "DC2 empty native audio filename");
    let context = DecodeContext::new(client);
    let packed = native
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("VOICE/"));
    let archive = if packed {
        if let Some(archive) = cached {
            Some(archive)
        } else {
            let index = VfsPath::new(Mount::Asset, "Advdata/VOICE.DAT")?;
            if storage.metadata(Mount::Asset, index.relative())?.is_some() {
                Some(dc2_voice::open_archive(storage.clone(), client)?)
            } else {
                None
            }
        }
    } else {
        cached
    };
    if packed {
        //426690 strips exactly the first VOICE component, preserving SYS and
        //other subdirectories inside the native64-byte index name.
        let entry = replace_extension(&native[6..], "PCM");
        if let Some((archive, id)) = archive
            .as_ref()
            .and_then(|archive| archive.resolve(&entry).map(|id| (archive, id)))
        {
            let reader = archive.open(id, &context)?;
            let bytes =
                crysirror_resource::read_all(reader.as_ref(), context.limits.max_entry_size)?;
            return Ok(Prepared::Audio(bytes.into(), Some(archive.clone())));
        }
    }
    //427BA0 tries .PCM, then .WAV, including for non-VOICE paths. Resource
    //category remains a property of the caller; a voice may live under SE/.
    let path = loose_path(storage.as_ref(), &native)?;
    let reader = storage.open_read(&path)?;
    let bytes = crysirror_resource::read_all(reader.as_ref(), context.limits.max_entry_size)?;
    Ok(Prepared::Audio(bytes.into(), archive))
}

pub(super) fn loose_path(storage: &dyn ClientStorage, native: &str) -> Result<VfsPath> {
    let pcm = VfsPath::new(
        Mount::Asset,
        &format!("Advdata/{}", replace_extension(&native, "PCM")),
    )?;
    Ok(
        if storage.metadata(Mount::Asset, pcm.relative())?.is_some() {
            pcm
        } else {
            VfsPath::new(
                Mount::Asset,
                &format!("Advdata/{}", replace_extension(&native, "WAV")),
            )?
        },
    )
}

fn replace_extension(name: &str, extension: &str) -> String {
    //4278A0 remembers the last separator, colon or period. A period replaces
    //the extension only when no later path separator followed it.
    let at = name
        .rfind(['/', ':', '.'])
        .filter(|&index| name.as_bytes()[index] == b'.')
        .unwrap_or(name.len());
    format!("{}.{}", &name[..at], extension)
}
