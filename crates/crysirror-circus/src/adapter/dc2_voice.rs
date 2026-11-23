//! Native DC2 volume naming over the shared indexed range archive. The worker
//! opens only referenced read-only volumes and never reads a whole2GB DAT.
use super::*;
pub(super) fn open_archive(
    storage: Arc<dyn ClientStorage>,
    client: ClientId,
) -> Result<Arc<dyn Archive>> {
    let path = VfsPath::new(Mount::Asset, "Advdata/VOICE.DAT")?;
    let source = storage.open_read(&path)?;
    let mut context = DecodeContext::new(client);
    context.source_name = Some(path.relative().into());
    context.resolver = Some(Arc::new(
        move |name: &str| -> Result<Arc<dyn crysirror_core::service::AssetReader>> {
            Ok(storage.open_read(&VfsPath::new(Mount::Asset, name)?)?)
        },
    ));
    Ok(Arc::from(
        crysirror_resource::circus_voice::CircusVoice.open(source, &context)?,
    ))
}
