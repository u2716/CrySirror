use crate::native_asset::error::{AssetError, Result};
use crate::native_asset::nls::Nls;

pub fn parse_archive_dat_paths(bytes: &[u8], nls: Nls) -> Result<Vec<String>> {
    parse_archive_dat_profile(bytes, nls, 0)
}

/// Select a comma-delimited archive profile, then split its pipe-delimited
/// paths. Newer Game.exe uses set_system_path(index) to replace this profile.
/// Decode before splitting so a Shift-JIS trail byte cannot become a delimiter.
pub fn parse_archive_dat_profile(bytes: &[u8], nls: Nls, profile: usize) -> Result<Vec<String>> {
    if bytes.is_empty() {
        return Err(AssetError::InvalidArchiveDat {
            reason: "file is empty".to_owned(),
        });
    }

    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let text = nls.decode(&bytes[..end])?;
    let selected = text.split(',').nth(profile).ok_or_else(|| AssetError::InvalidArchiveDat {
        reason: format!("archive profile {profile} is absent"),
    })?;
    let compact: String = selected.chars()
        .filter(|c| !matches!(c, '\r' | '\n' | ' ' | '\t'))
        .collect();

    let mut paths = Vec::new();
    for item in compact.split('|') {
        if item.is_empty() {
            continue;
        }
        paths.push(item.replace('/', "\\"));
    }

    if paths.is_empty() {
        return Err(AssetError::InvalidArchiveDat {
            reason: "no path entries".to_owned(),
        });
    }
    Ok(paths)
}
