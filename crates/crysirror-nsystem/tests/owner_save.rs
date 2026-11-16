// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Owner-data check: `CRYSIRROR_NSYSTEM_SAVE=<GAMEnn.SAV>` (an original slot,
//! read-only) must decode and re-encode byte-identically. Skipped without it.
use crysirror_nsystem::{save::Slot, script::title_key};

#[test]
fn original_slot_round_trips() {
    let Some(path) = std::env::var_os("CRYSIRROR_NSYSTEM_SAVE") else { return };
    let bytes = std::fs::read(path).unwrap();
    let title = title_key("その花びらにくちづけを　にゅーじぇね！");
    let slot = Slot::decode(&bytes, &title).unwrap();
    assert!(!slot.screen.script.is_empty());
    // A slot taken outside camera effects carries the Reset camera.
    assert!(slot.screen.cameras.iter().all(|c| *c == crysirror_nsystem::save::default_camera()));
    assert_eq!(slot.encode(&title).unwrap(), bytes);
}

/// `CRYSIRROR_NSYSTEM_CONFIG=<CONFIG.SAV>` must round-trip byte-identically.
#[test]
fn original_config_round_trips() {
    let Some(path) = std::env::var_os("CRYSIRROR_NSYSTEM_CONFIG") else { return };
    let bytes = std::fs::read(path).unwrap();
    let system = crysirror_nsystem::system::System::decode(&bytes).unwrap();
    assert_eq!(system.encode().unwrap(), bytes);
}
