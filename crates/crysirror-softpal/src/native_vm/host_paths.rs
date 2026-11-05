//! Owner-thread storage context. Native save names/formats remain unchanged;
//! the desktop supplies a distinct per-client writable directory.
use std::path::{Path, PathBuf};
std::thread_local! {
    static SAVE_ROOT: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}
pub(crate) fn install(root: PathBuf) {
    SAVE_ROOT.with(|slot| *slot.borrow_mut() = Some(root));
}
pub(crate) fn save_directory(game_root: &Path) -> PathBuf {
    SAVE_ROOT
        .with(|slot| slot.borrow().clone())
        .unwrap_or_else(|| game_root.join("save"))
}
pub(crate) fn writable_copy(path: &Path) -> std::io::Result<PathBuf> {
    let Some(root) = SAVE_ROOT.with(|slot| slot.borrow().clone()) else {
        return Ok(path.into());
    };
    if path.starts_with(&root) {
        return Ok(path.into());
    }
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "native save filename")
    })?;
    std::fs::create_dir_all(&root)?;
    let destination = root.join(name);
    if !destination.exists() {
        std::fs::copy(path, &destination)?;
    }
    Ok(destination)
}
