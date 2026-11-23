// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Original KAGParser dictionary names/layout through the shared
//! [`crysirror_kag::krkr_store`] (KAG3 profile). Store maps active call frames
//! and macro arguments; restoring active frames is refused, never faked.
use crate::natives::*;
use crysirror_kag::{
    KagParser, Krkr2ConditionState, ParserSnapshot,
    krkr_store::{self, Profile},
};
use crysirror_tjs2::{
    Result,
    runtime::{ObjectHandle, Variant},
};
pub(crate) fn conditions(rt: &mut Rt, h: ObjectHandle, c: &Krkr2ConditionState) {
    krkr_store::conditions(rt, h, c);
}
/// Native `tTJSNI_KAGParser::Store` layout (KAGParser c2269b2, KAGParser.cpp
/// 491-683): call frames record storage, nearest label and the line offset
/// from it (restore re-runs GoToLabel per frame).
pub fn capture(rt: &mut Rt, p: &KagParser) -> Result<Variant> {
    krkr_store::capture(rt, p, Profile::Kag3)
}
pub fn validate(rt: &Rt, h: ObjectHandle) -> Result<()> {
    match krkr_store::unrestorable(rt, h, Profile::Kag3)? {
        Some(capability) => Err(unsupported("KAGParser.restore", capability)),
        None => Ok(()),
    }
}
pub fn decode(rt: &Rt, p: &KagParser, h: ObjectHandle) -> Result<ParserSnapshot> {
    validate(rt, h)?;
    krkr_store::decode(rt, p, h, Profile::Kag3)
}

pub(crate) fn read_conditions(rt: &Rt, h: ObjectHandle) -> Result<Krkr2ConditionState> {
    krkr_store::read_conditions(rt, h)
}
