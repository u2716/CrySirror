// Copyright (c) 2026 CrySirror contributors.
// Distributed under the GNU Affero General Public License, version 3 or
// (at your option) any later version, https://www.gnu.org/licenses/agpl-3.0.html.
//! Storage-media plug-ins that rewrite names onto the ordinary search.
//!
//! Behaviour of the shipped DLLs:
//!
//! * lzfs.dll registers media `lzfs`; `lzfs://./name` opens `name` through the
//!   engine's own search and serves a complete LZ4 frame decoded, anything
//!   else raw. No TJS members.
//! * yuzuex.dll (proxyfs) publishes the global Dictionary `ProxyStorageMap`
//!   and media `proxy`: the media-relative name (`./x`), ASCII lower-cased,
//!   is one exact key; only a string value maps, to a full storage name.
//!
//! CrySirror change: the storage index lives outside the TJS heap, so the
//! dictionary's string values are mirrored into it at every VM step and
//! before each `Storages` lookup (`sync_proxy`), instead of being read live.
use crate::natives::Rt;
use crysirror_tjs2::{Result, runtime::Variant};
use std::collections::BTreeMap;

pub(crate) fn install_lzfs(rt: &mut Rt) -> Result<()> {
    rt.host_mut().storage_mut()?.wrappers.lzfs = true;
    Ok(())
}

pub(crate) fn install_yuzuex(rt: &mut Rt) -> Result<()> {
    if rt.global_member("ProxyStorageMap").object_handle().is_none() {
        let map = rt.alloc_dictionary_object();
        rt.set_global_member("ProxyStorageMap", Variant::Object(map));
    }
    rt.host_mut().storage_mut()?.wrappers.proxy = Some(BTreeMap::new());
    sync_proxy(rt)
}

/// Mirror `ProxyStorageMap`'s string values into the storage's `proxy` media.
pub(crate) fn sync_proxy(rt: &mut Rt) -> Result<()> {
    if rt.host().storage.as_ref().is_none_or(|s| s.wrappers.proxy.is_none()) {
        return Ok(());
    }
    let mut map = BTreeMap::new();
    if let Some(dict) = rt.global_member("ProxyStorageMap").object_handle() {
        for (key, value) in rt.object_members(dict) {
            if let Variant::String(target) = value {
                map.insert(key, target);
            }
        }
    }
    rt.host_mut().storage_mut()?.wrappers.proxy = Some(map);
    Ok(())
}
