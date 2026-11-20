// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! The dictionary of a native `tTJSNI_KAGParser::Store` as TJS values, and
//! its decoding back into a [`ParserSnapshot`]. Two native layouts exist:
//! KAG3's KAGParser (c2269b2, KAGParser.cpp 491-683) with
//! `ExcludeLevelStack`/`IfLevelExecutedStack`, and the CM3D2 plug-in's
//! older parser that records only `ExcludeLevel`/`IfLevel`. Call frames
//! record storage, nearest label and the line offset from it; the native
//! Restore re-runs `GoToLabel`, so the top-level cursor resumes at the label.
use crate::{Krkr2ConditionState, KagParser, ParserSnapshot};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{ObjectHandle, Runtime, TjsHost, Variant},
};

/// Which native `Store` layout a parser dictionary follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// KiriKiri2 KAG3 `KAGParser`: condition stacks, active frames are not
    /// restored.
    Kag3,
    /// The CM3D2 plug-in `KAGParser` (cm3d2_x64.dll `0x18003f170`): levels
    /// only; call frames are restored at their recorded line and position.
    Cm3d2,
}

fn put<H: TjsHost + 'static>(rt: &mut Runtime<H>, h: ObjectHandle, name: &str, v: impl Into<Variant>) {
    rt.set_object_member(h, name, v.into());
}

/// Condition members of a dictionary in the KAG3 layout.
pub fn conditions<H: TjsHost + 'static>(rt: &mut Runtime<H>, h: ObjectHandle, c: &Krkr2ConditionState) {
    put(rt, h, "ExcludeLevel", Variant::Integer(c.exclude_level));
    put(rt, h, "IfLevel", Variant::Integer(c.if_level));
    put(rt, h, "ExcludeLevelStack", Variant::String(c.exclude_level_stack.iter().map(|v| format!("{:08x}", *v as i32 as u32)).collect()));
    put(rt, h, "IfLevelExecutedStack", Variant::String(c.if_level_executed_stack.iter().map(|v| if *v { '1' } else { '0' }).collect()));
}

fn levels<H: TjsHost + 'static>(rt: &mut Runtime<H>, h: ObjectHandle, c: &Krkr2ConditionState, profile: Profile) {
    match profile {
        Profile::Kag3 => conditions(rt, h, c),
        Profile::Cm3d2 => {
            put(rt, h, "ExcludeLevel", Variant::Integer(c.exclude_level));
            put(rt, h, "IfLevel", Variant::Integer(c.if_level));
        }
    }
}

/// The `Store` dictionary of `p`.
pub fn capture<H: TjsHost + 'static>(rt: &mut Runtime<H>, p: &KagParser, profile: Profile) -> Result<Variant> {
    let h = rt.alloc_dictionary_object();
    let macros = rt.alloc_dictionary_object();
    for (n, s) in p.macro_definitions() {
        put(rt, macros, n, Variant::String(s.into()));
    }
    put(rt, h, "macros", Variant::Object(macros));
    // Macro arguments of the expansion in progress (one native stack level).
    let mut args = Vec::new();
    if !p.macro_params().is_empty() {
        let d = rt.alloc_dictionary_object();
        for a in p.macro_params() {
            if let crate::Attribute::Named { name, value } = a {
                let v = match value {
                    crate::AttributeValue::Literal(s) => Variant::String(s.clone()),
                    crate::AttributeValue::Void => Variant::Void,
                    _ => return Err(TjsError::runtime("unresolved KAG macro argument")),
                };
                put(rt, d, name, v);
            }
        }
        args.push(Variant::Object(d));
    }
    let depth = args.len() as i64;
    let a = rt.alloc_array_object(args);
    put(rt, h, "macroArgs", Variant::Object(a));
    let mut frames = Vec::new();
    for f in p.call_stack() {
        let d = rt.alloc_dictionary_object();
        let storage = f.storage().to_owned();
        let label = f.current_label().unwrap_or("").to_owned();
        let (line, pos) = p.line_pos_for_offset(&storage, f.offset()).unwrap_or((0, 0));
        let base = if label.is_empty() { 0 } else { p.label_line(&storage, &label).unwrap_or(0) };
        put(rt, d, "storage", Variant::String(storage));
        put(rt, d, "label", Variant::String(label));
        put(rt, d, "offset", Variant::Integer(line.saturating_sub(base) as i64));
        put(rt, d, "orgLineStr", Variant::String(f.line_text().into()));
        put(rt, d, "lineBuffer", Variant::String(String::new()));
        put(rt, d, "pos", Variant::Integer(pos as i64));
        for n in ["lineBufferUsing", "macroArgStackBase", "macroArgStackDepth"] {
            put(rt, d, n, Variant::Integer(0));
        }
        levels(rt, d, &f.krkr2_condition_state(), profile);
        frames.push(Variant::Object(d));
    }
    let a = rt.alloc_array_object(frames);
    put(rt, h, "callStack", Variant::Object(a));
    let storage = p.cur_storage().unwrap_or("");
    put(rt, h, "storageName", Variant::String(storage.into()));
    put(rt, h, "storageShortName", Variant::String(storage.rsplit(['/', '\\', '>']).next().unwrap_or(storage).into()));
    put(rt, h, "curLabel", Variant::String(p.cur_label().unwrap_or("").into()));
    put(rt, h, "curLine", Variant::Integer(p.cur_line().unwrap_or(1).saturating_sub(1) as i64));
    put(rt, h, "curPos", Variant::Integer(p.cur_pos().unwrap_or(0) as i64));
    put(rt, h, "lineBuffer", Variant::String(String::new()));
    put(rt, h, "lineBufferUsing", Variant::Integer(0));
    put(rt, h, "macroArgStackBase", Variant::Integer(0));
    put(rt, h, "macroArgStackDepth", Variant::Integer(depth));
    levels(rt, h, &p.store().krkr2_condition_state(), profile);
    Ok(Variant::Object(h))
}

fn integer<H: TjsHost + 'static>(rt: &Runtime<H>, h: ObjectHandle, n: &str) -> Result<i64> {
    rt.object_member(h, n).to_integer()
}

fn string_field<H: TjsHost + 'static>(rt: &Runtime<H>, h: ObjectHandle, n: &str) -> Result<String> {
    rt.object_member(h, n).to_tjs_string()
}

fn array<H: TjsHost + 'static>(rt: &Runtime<H>, h: ObjectHandle, n: &str) -> Result<Vec<Variant>> {
    let Some(a) = rt.object_member(h, n).object_handle() else {
        return Err(TjsError::runtime("native parser save array missing"));
    };
    Ok(rt.array_elements(a).map(<[Variant]>::to_vec).unwrap_or_default())
}

/// Parser state of a dictionary that this profile cannot restore: an active
/// line buffer or macro argument stack, or (KAG3) active call frames or
/// macro arguments. Returns the missing capability.
pub fn unrestorable<H: TjsHost + 'static>(rt: &Runtime<H>, h: ObjectHandle, profile: Profile) -> Result<Option<&'static str>> {
    for n in ["lineBufferUsing", "macroArgStackBase", "macroArgStackDepth"] {
        if integer(rt, h, n)? != 0 {
            return Ok(Some("active native macro/line-buffer save mapping"));
        }
    }
    let active = match profile {
        Profile::Kag3 => !array(rt, h, "callStack")?.is_empty() || !array(rt, h, "macroArgs")?.is_empty(),
        Profile::Cm3d2 => !array(rt, h, "macroArgs")?.is_empty(),
    };
    Ok(active.then_some("native active call/macro save mapping"))
}

/// Storages a dictionary's call frames refer to (to load before decoding).
pub fn frame_storages<H: TjsHost + 'static>(rt: &Runtime<H>, h: ObjectHandle) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for f in array(rt, h, "callStack")? {
        let d = f.object_handle().ok_or_else(|| TjsError::runtime("native saved call frame"))?;
        out.push(string_field(rt, d, "storage")?);
    }
    Ok(out)
}

/// Decode a dictionary whose storages are loaded in `p`. Callers check
/// [`unrestorable`] first.
pub fn decode<H: TjsHost + 'static>(rt: &Runtime<H>, p: &KagParser, h: ObjectHandle, profile: Profile) -> Result<ParserSnapshot> {
    let storage = string_field(rt, h, "storageName")?;
    let label = string_field(rt, h, "curLabel")?;
    let line = if label.is_empty() {
        0
    } else {
        p.label_line(&storage, &label).ok_or_else(|| TjsError::runtime("native parser saved label absent"))?
    };
    // The original Restore reloads storage and GoToLabel; curLine/curPos are
    // inspection fields, not an exact runtime cursor restoration contract.
    let offset = p.offset_for_line_pos(&storage, line, 0).map_err(|e| TjsError::runtime(e.to_string()))?;
    let macros = match rt.object_member(h, "macros") {
        Variant::Void => p.macro_definitions().map(|(n, s)| (n.to_owned(), s.to_owned())).collect::<Vec<_>>(),
        Variant::Object(m) => rt
            .object_members(m)
            .into_iter()
            .map(|(n, v)| if let Variant::String(s) = v { Ok((n, s)) } else { Err(TjsError::runtime("native saved macro body must be a string")) })
            .collect::<Result<Vec<_>>>()?,
        _ => return Err(TjsError::runtime("native saved macro dictionary")),
    };
    let (condition, frames) = match profile {
        Profile::Kag3 => (read_conditions(rt, h)?, Vec::new()),
        Profile::Cm3d2 => {
            let mut frames = Vec::new();
            for f in array(rt, h, "callStack")? {
                let d = f.object_handle().ok_or_else(|| TjsError::runtime("native saved call frame"))?;
                let storage = string_field(rt, d, "storage")?;
                let label = string_field(rt, d, "label")?;
                let base = if label.is_empty() { 0 } else { p.label_line(&storage, &label).ok_or_else(|| TjsError::runtime("native saved call frame label absent"))? };
                let line = base + usize::try_from(integer(rt, d, "offset")?).map_err(|_| TjsError::runtime("native saved call frame offset"))?;
                let pos = usize::try_from(integer(rt, d, "pos")?).map_err(|_| TjsError::runtime("native saved call frame position"))?;
                let at = p.offset_for_line_pos(&storage, line, pos).map_err(|e| TjsError::runtime(e.to_string()))?;
                // The parser's own line text: native `orgLineStr` keeps the
                // raw line, which this parser's normalized lines need not equal.
                let text = p.line_text_for_offset(&storage, at).unwrap_or("").to_owned();
                frames.push((storage, at, text, (!label.is_empty()).then_some(label), cm3d2_levels(rt, d)?));
            }
            (cm3d2_levels(rt, h)?, frames)
        }
    };
    Ok(ParserSnapshot::from_krkr2_compatible(Some(storage), offset, (!label.is_empty()).then_some(label), frames, macros, condition))
}

/// CM3D2 levels: only the outside-of-`if` state is representable without
/// the executed-branch stack.
fn cm3d2_levels<H: TjsHost + 'static>(rt: &Runtime<H>, h: ObjectHandle) -> Result<Krkr2ConditionState> {
    let (exclude, level) = (integer(rt, h, "ExcludeLevel")?, integer(rt, h, "IfLevel")?);
    if level != 0 {
        return Err(TjsError::runtime("CM3D2 KAGParser save inside an if block has no executed-branch stack"));
    }
    Ok(Krkr2ConditionState { exclude_level: exclude, if_level: 0, exclude_level_stack: Vec::new(), if_level_executed_stack: Vec::new() })
}

/// KAG3 condition members.
pub fn read_conditions<H: TjsHost + 'static>(rt: &Runtime<H>, h: ObjectHandle) -> Result<Krkr2ConditionState> {
    let hex = string_field(rt, h, "ExcludeLevelStack")?;
    let flags = string_field(rt, h, "IfLevelExecutedStack")?;
    let level = integer(rt, h, "IfLevel")?;
    if !(0..=128).contains(&level) || hex.len() != level as usize * 8 || flags.len() != level as usize || !flags.bytes().all(|v| v == b'0' || v == b'1') {
        return Err(TjsError::runtime("native saved condition stack bounds"));
    }
    let stack = hex
        .as_bytes()
        .chunks_exact(8)
        .map(|c| std::str::from_utf8(c).ok().and_then(|s| u32::from_str_radix(s, 16).ok()).map(|v| v as i32 as i64).ok_or_else(|| TjsError::runtime("native saved condition hex")))
        .collect::<Result<Vec<_>>>()?;
    Ok(Krkr2ConditionState { exclude_level: integer(rt, h, "ExcludeLevel")?, if_level: level, exclude_level_stack: stack, if_level_executed_stack: flags.bytes().map(|v| v == b'1').collect() })
}
