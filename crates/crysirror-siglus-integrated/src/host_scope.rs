// SPDX-License-Identifier: MPL-2.0
use std::{cell::RefCell, path::PathBuf};
#[derive(Clone)]
pub struct Scope {
    pub resource_root: PathBuf,
    pub user_root: PathBuf,
    pub font: Option<PathBuf>,
    pub font_face_index: u32,
    pub material: crate::native_assets::key_toml::KeyTomlConfig,
}
thread_local! {static SCOPE:RefCell<Option<Scope>>=const {RefCell::new(None)};}
pub fn install(scope: Scope) {
    SCOPE.with(|s| *s.borrow_mut() = Some(scope));
}
pub fn user_root() -> Option<PathBuf> {
    SCOPE.with(|s| s.borrow().as_ref().map(|s| s.user_root.clone()))
}
pub fn font() -> Option<PathBuf> {
    SCOPE.with(|s| s.borrow().as_ref().and_then(|s| s.font.clone()))
}
pub fn material() -> Option<crate::native_assets::key_toml::KeyTomlConfig> {
    SCOPE.with(|s| s.borrow().as_ref().map(|s| s.material.clone()))
}
pub fn admit_read(path: &std::path::Path) -> anyhow::Result<()> {
    SCOPE.with(|scope| {
        if let Some(scope) = scope.borrow().as_ref() {
            let path = path.canonicalize()?;
            anyhow::ensure!(
                path.starts_with(&scope.resource_root)
                    || path.starts_with(&scope.user_root)
                    || scope.font.as_ref().is_some_and(|font| font == &path),
                "Siglus resource escapes its host roots"
            );
        }
        Ok(())
    })
}

pub fn font_face_index() -> u32 {
    SCOPE.with(|s| s.borrow().as_ref().map_or(0, |s| s.font_face_index))
}
