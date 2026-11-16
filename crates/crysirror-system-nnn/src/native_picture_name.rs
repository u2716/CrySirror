// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CPicture file identity differs from a script's LOADDWQ basename. Source
//! GameCallBack11361 prefixes the first two original bytes and a backslash;
//! TaihiAllEffect14339 saves CPicture's resulting logical filename verbatim.
//! Profile selection must be explicit until each executable ABI is reviewed.
use crate::native_resources::{ImageLookup, ResourceNames};
use anyhow::{Result, ensure};

#[derive(Clone, Copy)]
pub(crate) enum SavedNameProfile {
    LegacyBasename,
    NativeLogical,
}

pub(crate) fn script_native_name(basename: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        basename.len() >= 2
            && !basename.contains(&0)
            && !basename.contains(&b'\\')
            && !basename.contains(&b'/'),
        "NNN script picture basename"
    );
    // CPicture's unchecked filename copy and the original save's doubleNUL
    // record both have finite capacity. Reject the formerly overflowing input.
    ensure!(
        basename.len() <= 251,
        "NNN native prefixed picture filename capacity"
    );
    let mut name = Vec::with_capacity(basename.len() + 3);
    name.extend_from_slice(&basename[..2]);
    name.push(b'\\');
    name.extend_from_slice(basename);
    Ok(name)
}
pub(crate) fn saved_lookup(
    names: &ResourceNames,
    profile: SavedNameProfile,
    name: &[u8],
    levels: u32,
) -> Result<ImageLookup> {
    match profile {
        SavedNameProfile::LegacyBasename => names.script_image(name, levels),
        SavedNameProfile::NativeLogical => names.picture(name, levels),
    }
}
pub(crate) fn staff_frame(base_native_name: &[u8], delta: i32) -> Result<Vec<u8>> {
    ensure!(delta > 0, "NNN staff frame number");
    let mut name = base_native_name.to_vec();
    name.push(b'_');
    name.extend_from_slice(delta.wrapping_add(1).to_string().as_bytes());
    // Original wsprintf targets1024, but CPicture then stores the same name in
    // its much smaller name member. Keep the stricter defined storage bound.
    ensure!(
        name.len() <= 254 && !name.contains(&0),
        "NNN staff CPicture filename capacity"
    );
    Ok(name)
}
