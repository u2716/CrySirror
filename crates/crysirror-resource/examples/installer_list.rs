//! List the logical installed files of a Windows Installer package without
//! installing it: `cargo run -p crysirror-resource --example installer_list -- PACKAGE.msi`.
//! With `LOGICAL_PATH OUTPUT`, the matching file (ASCII case-insensitive) is
//! extracted to OUTPUT instead.
use anyhow::{Context, Result};
use crysirror_core::{service::{AssetReader, MemoryAssetReader}, state::ClientId};
use crysirror_resource::{installer::InstallerCatalog, DecodeContext};
use std::sync::Arc;
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().context("installer_list PACKAGE.msi [LOGICAL_PATH OUTPUT]")?;
    let extract = args.next().zip(args.next());
    let mut cx = DecodeContext::new(ClientId(1));
    cx.limits.max_entry_size = u64::MAX / 2;
    let source: Arc<dyn AssetReader> = Arc::new(MemoryAssetReader(std::fs::read(&path)?.into()));
    let catalog = InstallerCatalog::open(source, &cx)?;
    let Some((want, output)) = extract else {
        for file in catalog.files() {
            println!("{}\t{}", file.size, file.path);
        }
        return Ok(());
    };
    let want = want.replace('\\', "/");
    let index = catalog
        .files()
        .iter()
        .position(|file| file.path.replace('\\', "/").eq_ignore_ascii_case(&want))
        .with_context(|| format!("{want} is not a logical file of the package"))?;
    let reader = catalog.read(index, &cx)?;
    let mut bytes = vec![0; usize::try_from(reader.len()?)?];
    let mut done = 0;
    while done < bytes.len() {
        let n = reader.read_at(done as u64, &mut bytes[done..])?;
        anyhow::ensure!(n > 0, "short installer read");
        done += n;
    }
    std::fs::write(&output, bytes)?;
    Ok(())
}
