// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! KCS natives acknowledged without effect until their capability exists: save
//! containers (238..254), post-effect
//! classes and resource bookkeeping. The first call of each id is logged, so a running
//! game shows what is still missing. Any other unknown native fails visibly in the adapter.
/// `Some(slot)` for a listed native; a value slot answers 0.
pub(crate) fn return_slot(id: u32) -> Option<Option<u32>> {
    match id {
        111 | 267 | 315 | 316 | 365 | 368 | 520 | 598 | 633 | 741 | 903 | 909 => Some(Some(0)),
        168 | 344 | 347 | 348 | 402 | 426 | 527 | 528 | 530 | 531 | 542 | 568 | 904 | 907 | 908 | 931 | 991 | 1036 => Some(None),
        _ => None,
    }
}
