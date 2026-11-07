// SPDX-License-Identifier: MPL-2.0
//! Upstream bootstrap policy over caller-owned resource bytes, not OS paths.
//! No game directory writes or key-bearing diagnostics/launch-plan fields.
use crate::{key_recovery, key_toml::KeyTomlConfig, keys};
use anyhow::{Result, anyhow, ensure};
use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
};

// Immutable, content-scoped results avoid repeating detection's recovery when
// the adapter boots. Bound retained material independently of session count.
type Cache = VecDeque<([u8; 32], [u8; 16])>;
static RECOVERED: OnceLock<Mutex<Cache>> = OnceLock::new();

pub fn apply_defaults(material: &mut KeyTomlConfig) {
    material
        .game_angou_code
        .get_or_insert_with(|| keys::GAMEEXE_KEY.to_vec());
    material
        .scene_angou_code
        .get_or_insert_with(|| keys::SCENE_KEY.to_vec());
}

/// Preserve configured keys accepted by the original loader, recover missing
/// or mismatched keys using the upstream resource-pair rules, and reject
/// non-converging/ambiguous results. Call on a boot/resource worker.
pub fn resolve(game: &[u8], scene: &[u8], mut material: KeyTomlConfig) -> Result<KeyTomlConfig> {
    ensure!(
        game.len() <= 16 * 1024 * 1024 && scene.len() <= 64 * 1024 * 1024,
        "Siglus bootstrap resource budget"
    );
    apply_defaults(&mut material);
    // Text INI inputs have no Gameexe encryption header. The packed scene's
    // independent encryption flag still controls whether material is needed.
    let game_text =
        crate::gameexe::decode_gameexe_dat_bytes(game, &game_options(&material)).is_ok();
    let scene_encrypted = scene.get(84..88).is_some_and(|b| b != [0, 0, 0, 0]);
    if game_text && !scene_encrypted {
        return Ok(material);
    }
    if let Some(key) = material.exe_key16 {
        match key_recovery::check_key(game, scene, &key) {
            Ok(key_recovery::KeyStatus::Accepted | key_recovery::KeyStatus::Unverifiable) => {
                return Ok(material);
            }
            // Explicit custom tables aren't candidates for the standard-table
            // recovery profile. Keep their configured semantics for the decoder.
            _ if material.game_angou_code.as_deref() != Some(keys::GAMEEXE_KEY.as_slice())
                || material.scene_angou_code.as_deref() != Some(keys::SCENE_KEY.as_slice()) =>
            {
                return Ok(material);
            }
            _ => {}
        }
    }
    if !key_recovery::scene_pack_is_compressed(scene).map_err(|e| anyhow!(e))? {
        return Ok(material); // Upstream easy-link policy: no futile recovery.
    }
    let mut digest = blake3::Hasher::new();
    digest.update(&(game.len() as u64).to_le_bytes());
    digest.update(game);
    digest.update(scene);
    let identity = *digest.finalize().as_bytes();
    let cache = RECOVERED.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Some((_, key)) = cache
        .lock()
        .map_err(|_| anyhow!("Siglus recovery cache poisoned"))?
        .iter()
        .find(|(id, _)| *id == identity)
        .copied()
    {
        material.exe_key16 = Some(key);
        return Ok(material);
    }
    let key = key_recovery::recover_key_from_resources(game, scene).map_err(|e| anyhow!(e))?;
    let mut cache = cache
        .lock()
        .map_err(|_| anyhow!("Siglus recovery cache poisoned"))?;
    if cache.len() == 16 {
        cache.pop_front();
    }
    cache.push_back((identity, key));
    material.exe_key16 = Some(key);
    Ok(material)
}

pub fn game_options(material: &KeyTomlConfig) -> crate::gameexe::GameexeDecodeOptions {
    crate::gameexe::GameexeDecodeOptions {
        exe_key16: material.exe_key16,
        base_angou_code: material.base_angou_code.clone(),
        game_angou_code: material
            .game_angou_code
            .clone()
            .or_else(|| Some(keys::GAMEEXE_KEY.to_vec())),
        chain_order: material
            .chain_order
            .clone()
            .unwrap_or_else(|| crate::gameexe::GameexeDecodeOptions::default().chain_order),
        ..Default::default()
    }
}
