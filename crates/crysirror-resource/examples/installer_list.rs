//! List the logical installed files of a Windows Installer package without
//! installing it: `cargo run -p crysirror-resource --example installer_list -- PACKAGE.msi`.
use anyhow::{Context, Result};
use crysirror_core::{service::{AssetReader, MemoryAssetReader}, state::ClientId};
use crysirror_resource::{installer::InstallerCatalog, DecodeContext};
use std::sync::Arc;
fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("installer_list PACKAGE.msi")?;
    let mut cx = DecodeContext::new(ClientId(1));
    cx.limits.max_entry_size = u64::MAX / 2;
    let source: Arc<dyn AssetReader> = Arc::new(MemoryAssetReader(std::fs::read(&path)?.into()));
    for file in InstallerCatalog::open(source, &cx)?.files() {
        println!("{}\t{}", file.size, file.path);
    }
    Ok(())
}
