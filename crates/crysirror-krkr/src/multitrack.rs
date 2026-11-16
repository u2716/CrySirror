// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! wumultitrack.dll multi-track music (game-kr `plugin/wumultitrack.dll`,
//! IDA). The description and instance policy live here; stems, lockstep
//! mixing and gain ramps are the shared `AudioResourceTransform::MultiTrack`
//! and `AudioRequest::SetTrackGains`.
//!
//! - `.mtrk` (RenderMixerBunch 0x1000C120): a name `dir/a|b|ID.mtrk` names
//!   the track files `dir/a`, `dir/b` and the instance `ID.mtrk`; any other
//!   name is a TAB list file of `name, path, flag` lines and names the
//!   instance by its file name. Tracks are registered under their explicit
//!   name, the file name without extension and the file name.
//! - `.mchx` (RenderMixerGather 0x1000C7A0): TAB lines `channel N`,
//!   `source PATH [flag]` (relative to the description; also the instance
//!   ID), `track NAME ch...` and `volume NAME gain`; `#` lines are comments.
//! - Masks (0x10003C10/0x1000FD40/0x1000E380) retarget track gains linearly
//!   over `time` ms of source time; a zero time or an unchanged gain switches
//!   at once. Delayed masks (0x10003390, 0x1000EAC0, 0x1000F970) wait for the
//!   first listed source frame.
use crate::natives::{Rt, unsupported};
use crysirror_core::service::{AudioRequest, AudioTrackGain, MultiTrackStem};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{NativeArgCount, Variant},
};
use std::collections::BTreeMap;

const AUDIO_EXTENSIONS: &[&str] = &[".wav", ".ogg", ".opus", ".flac", ".mp3"];

/// A parsed description, ready for resource preparation.
pub struct Description {
    /// Resolved asset path of the resource's own asset (the first stem's).
    pub main: String,
    pub stems: Vec<MultiTrackStem>,
    pub id: String,
    /// Lower-cased track names, in registration order, with track index.
    pub names: Vec<(String, u32)>,
}
/// Open instances: id → WaveSoundBuffer object, and each buffer's names and
/// current target gains.
#[derive(Default)]
pub struct Registry {
    instances: BTreeMap<String, usize>,
    owned: BTreeMap<usize, Owned>,
}
struct Owned {
    id: String,
    names: Vec<(String, u32)>,
    gains: Vec<f32>,
}
impl Registry {
    /// A buffer that reopens or retires leaves its instance id.
    pub fn retire(&mut self, object: usize) {
        if let Some(owned) = self.owned.remove(&object) {
            if self.instances.get(&owned.id) == Some(&object) {
                self.instances.remove(&owned.id);
            }
        }
    }
}

/// `_wtoi`: optional sign and leading digits, otherwise 0.
fn wtoi(s: &str) -> i64 {
    let t = s.trim_start();
    let (neg, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let n = digits.bytes().take_while(u8::is_ascii_digit).fold(0i64, |n, d| n.saturating_mul(10).saturating_add(i64::from(d - b'0')));
    if neg { -n } else { n }
}
fn err(text: impl std::fmt::Display) -> TjsError {
    TjsError::runtime(format!("WuMultiTrack: {text}"))
}
fn split_dir(name: &str) -> (&str, &str) {
    match name.rfind(['/', '\\', '>']) {
        Some(i) => name.split_at(i + 1),
        None => ("", name),
    }
}
fn strip_ext(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if !name[i..].contains(['/', '\\']) => &name[..i],
        _ => name,
    }
}
/// Whether `name` is a multi-track description this plug-in opens.
pub fn handles(rt: &Rt, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    rt.host().loaded_plugins.contains("wumultitrack.dll") && (lower.ends_with(".mtrk") || lower.ends_with(".mchx"))
}
fn resolve_audio(rt: &Rt, path: &str) -> Result<String> {
    let storage = rt.host().storage()?;
    storage
        .resolve(path)
        .or_else(|| storage.resolve_with(path, AUDIO_EXTENSIONS))
        .ok_or_else(|| err(format!("source not found: {path}")))
}
fn read_lines(rt: &Rt, name: &str) -> Result<Vec<String>> {
    let storage = rt.host().storage()?;
    let path = storage.resolve(name).ok_or_else(|| err(format!("failed to load track list: {name}")))?;
    let bytes = storage.read(&path).map_err(|e| err(format!("{name}: {e}")))?;
    let text = crate::storage::decode_text(&bytes).map_err(|e| err(format!("{name}: {e}")))?;
    Ok(text.lines().map(|l| l.trim_end_matches('\r').to_owned()).collect())
}
fn add_names(names: &mut Vec<(String, u32)>, index: u32, candidates: &[&str]) {
    for name in candidates {
        if !name.is_empty() {
            names.push((name.to_lowercase(), index));
        }
    }
}
fn stem(asset: &str, pick: Option<[Option<u16>; 2]>, gain: f32) -> Result<MultiTrackStem> {
    let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, asset).map_err(|e| err(e))?;
    Ok(MultiTrackStem { asset: Some(path), pick, gain })
}

pub fn describe(rt: &Rt, name: &str) -> Result<Description> {
    let (dir, file) = split_dir(name);
    if file.to_ascii_lowercase().ends_with(".mchx") {
        return gather(rt, name, dir);
    }
    let mut tracks: Vec<(String, Option<String>)> = Vec::new();
    let id;
    if file.contains('|') {
        let fields: Vec<&str> = file.split('|').collect();
        id = fields[fields.len() - 1].to_owned();
        for f in &fields[..fields.len() - 1] {
            tracks.push((format!("{dir}{f}"), None));
        }
    } else {
        id = file.to_owned();
        for line in read_lines(rt, name)? {
            if line.is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let path = fields.get(1).copied().unwrap_or("");
            if path.is_empty() {
                continue;
            }
            let explicit = fields.first().copied().filter(|s| !s.is_empty()).map(str::to_owned);
            tracks.push((path.to_owned(), explicit));
        }
        if tracks.is_empty() {
            return Err(err(format!("failed to load track list: {name}")));
        }
    }
    if tracks.is_empty() {
        return Err(err(format!("empty tracks: {name}")));
    }
    if tracks.len() > 64 {
        return Err(unsupported("WuMultiTrack more than 64 tracks", "the shared multi-track voice"));
    }
    let mut stems = Vec::new();
    let mut names = Vec::new();
    let mut main = String::new();
    for (i, (path, explicit)) in tracks.iter().enumerate() {
        let resolved = resolve_audio(rt, path)?;
        if i == 0 {
            main = resolved.clone();
        }
        stems.push(stem(&resolved, None, 1.)?);
        let (_, base) = split_dir(path);
        add_names(&mut names, i as u32, &[explicit.as_deref().unwrap_or(""), strip_ext(base), base]);
    }
    Ok(Description { main, stems, id, names })
}

fn gather(rt: &Rt, name: &str, dir: &str) -> Result<Description> {
    let mut channels = 0i64;
    let mut source: Option<String> = None;
    let mut tracks: Vec<(String, Vec<Option<u16>>)> = Vec::new();
    let mut volumes: Vec<(String, f32)> = Vec::new();
    for line in read_lines(rt, name)? {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "channel" => {
                let n = f.get(1).filter(|s| !s.is_empty()).map_or(0, |s| wtoi(s));
                if n <= 0 {
                    return Err(err("invalid channel count"));
                }
                if channels != 0 {
                    return Err(err("duplicated channel found"));
                }
                channels = n;
            }
            "source" => {
                let path = f.get(1).copied().filter(|s| !s.is_empty()).ok_or_else(|| err("invalid source"))?;
                if source.is_some() {
                    return Err(err("multi source found"));
                }
                source = Some(path.to_owned());
            }
            "track" => {
                let track = f.get(1).copied().filter(|s| !s.is_empty()).ok_or_else(|| err("no track name"))?;
                let map = f[2..]
                    .iter()
                    .map(|s| {
                        let n = wtoi(s);
                        (!s.is_empty() && (0..32).contains(&n)).then_some(n as u16)
                    })
                    .collect();
                tracks.push((track.to_owned(), map));
            }
            "volume" => {
                let track = f.get(1).copied().filter(|s| !s.is_empty()).ok_or_else(|| err("no volume name"))?;
                let gain = f.get(2).filter(|s| !s.is_empty()).map_or(1., |s| s.trim().parse::<f32>().unwrap_or(0.));
                volumes.push((track.to_owned(), gain));
            }
            _ => {}
        }
    }
    if channels == 0 {
        return Err(err("no channel"));
    }
    if tracks.is_empty() {
        return Err(err("no track"));
    }
    let source = source.ok_or_else(|| err("no source"))?;
    if channels > 2 {
        return Err(unsupported(&format!("WuMultiTrack {channels}-channel output"), "mono or stereo multi-track output"));
    }
    if tracks.len() > 64 {
        return Err(unsupported("WuMultiTrack more than 64 tracks", "the shared multi-track voice"));
    }
    let resolved = resolve_audio(rt, &format!("{dir}{source}"))?;
    let mut stems = Vec::new();
    let mut names = Vec::new();
    for (i, (track, map)) in tracks.iter().enumerate() {
        let bound: Vec<Option<u16>> = (0..channels as usize).map(|k| map.get(k).copied().flatten()).collect();
        if bound.iter().all(Option::is_none) {
            return Err(err(format!("track empty bind:{track}")));
        }
        let pick = if channels == 1 { [bound[0], bound[0]] } else { [bound[0], bound[1]] };
        // `volume` lines apply to the last track of that name.
        let gain = volumes.iter().rev().find(|(n, _)| n == track).map_or(1., |v| v.1);
        if !gain.is_finite() || !(0.0..=16.).contains(&gain) {
            return Err(err(format!("volume out of range: {track}")));
        }
        stems.push(MultiTrackStem { asset: None, pick: Some(pick), gain });
        add_names(&mut names, i as u32, &[track]);
    }
    Ok(Description { main: resolved, stems, id: source, names })
}

/// Register a newly opened description for `object` and run the open
/// callbacks (0x100041A0): `(id, storage, trackCount, names, aliases)`.
pub fn opened(rt: &mut Rt, object: usize, storage: &str, d: &Description) -> Result<()> {
    rt.host_mut().multitrack.retire(object);
    if !d.id.is_empty() {
        if let Some(&other) = rt.host().multitrack.instances.get(&d.id) {
            if other != object {
                let line = format!("WuMultiTrack: instance conflict:{}", d.id);
                rt.host_mut().push_log(line);
            }
        }
        rt.host_mut().multitrack.instances.insert(d.id.clone(), object);
    }
    let gains = d.stems.iter().map(|s| s.gain).collect();
    rt.host_mut()
        .multitrack
        .owned
        .insert(object, Owned { id: d.id.clone(), names: d.names.clone(), gains });
    let Some(wave) = rt.global_member("WaveSoundBuffer").object_handle() else { return Ok(()) };
    let Some(list) = rt.object_member(wave, "multiTrackOpenCallbacks").object_handle() else { return Ok(()) };
    let callbacks: Vec<Variant> = rt.array_elements(list).map(|e| e.to_vec()).unwrap_or_default();
    if callbacks.is_empty() {
        return Ok(());
    }
    let dict = rt.alloc_dictionary_object();
    let mut aliases = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (name, index) in &d.names {
        if seen.insert(*index) {
            rt.set_object_member(dict, name, Variant::Integer(i64::from(*index)));
        } else {
            aliases.push(Variant::String(name.as_str().into()));
            aliases.push(Variant::Integer(i64::from(*index)));
        }
    }
    let aliases = rt.alloc_array_object(aliases);
    let args = vec![
        Variant::String(d.id.as_str().into()),
        Variant::String(storage.into()),
        Variant::Integer(d.stems.len() as i64),
        Variant::Object(dict),
        Variant::Object(aliases),
    ];
    for callback in callbacks {
        if matches!(callback, Variant::Object(_) | Variant::Closure(_)) {
            rt.call_function(callback, args.clone())?;
        }
    }
    Ok(())
}

/// Instance lookup (0x10004980); a missing one logs and answers -1.
fn instance(rt: &mut Rt, id: &Variant) -> Result<std::result::Result<usize, Variant>> {
    let Variant::String(id) = id else {
        return Err(err("the track id must be a string"));
    };
    let id = id.to_string();
    if id.is_empty() {
        return Err(TjsError::runtime("invalid parameter"));
    }
    let found = rt
        .host()
        .multitrack
        .instances
        .get(&id)
        .copied()
        .filter(|o| rt.host().multitrack.owned.contains_key(o) && crate::sound::channel(rt, *o).is_some());
    match found {
        Some(o) => Ok(Ok(o)),
        None => {
            rt.host_mut().push_log(format!("WuMultiTrack: instance not found:{id}"));
            Ok(Err(Variant::Integer(-1)))
        }
    }
}
/// Track selection of a mask: void (all), a name, or an array of names.
fn selection(rt: &mut Rt, object: usize, tracks: &Variant) -> Result<Option<Vec<u32>>> {
    let names: Vec<String> = match tracks {
        Variant::Void => return Ok(None),
        Variant::String(s) => vec![s.to_string()],
        Variant::Object(h) => rt
            .array_elements(*h)
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.to_tjs_string().map(|s| s.to_string()))
            .collect::<Result<_>>()?,
        _ => return Err(TjsError::runtime("invalid parameter")),
    };
    let owned = &rt.host().multitrack.owned[&object];
    let mut out = Vec::new();
    let mut missing = Vec::new();
    for name in names {
        if name.is_empty() {
            return Ok(None);
        }
        match owned.names.iter().find(|(n, _)| *n == name.to_lowercase()) {
            Some((_, i)) => out.push(*i),
            None => missing.push(name),
        }
    }
    for name in missing {
        rt.host_mut().push_log(format!("WuMultiTrack: track not found: {name}"));
    }
    Ok(Some(out))
}
fn record(rt: &mut Rt, object: usize, change: &AudioTrackGain) {
    if let Some(owned) = rt.host_mut().multitrack.owned.get_mut(&object) {
        let all: Vec<u32> = (0..owned.gains.len() as u32).collect();
        for t in change.tracks.as_ref().unwrap_or(&all) {
            if let Some(g) = owned.gains.get_mut(*t as usize) {
                *g = change.gain;
            }
        }
    }
}
fn gain_arg(v: Option<&Variant>) -> Result<f32> {
    let g = v.map_or(Ok(0.), |v| v.to_real())? as f32;
    if !g.is_finite() || !(0.0..=16.).contains(&g) {
        return Err(err("track gain out of range"));
    }
    Ok(g)
}
fn time_arg(v: Option<&Variant>) -> Result<u32> {
    Ok(v.map_or(Ok(0), |v| v.to_integer())?.clamp(0, 600_000) as u32)
}
fn submit(rt: &mut Rt, object: usize, positions: Option<Vec<u64>>, changes: Vec<AudioTrackGain>) -> Result<()> {
    let Some(channel) = crate::sound::channel(rt, object) else { return Ok(()) };
    crate::sound::control(rt, AudioRequest::SetTrackGains { channel, positions, changes })
}
/// The gains a multi-track buffer starts playing with (after `play`).
pub fn replay(rt: &mut Rt, object: usize) -> Result<()> {
    let Some(owned) = rt.host().multitrack.owned.get(&object) else { return Ok(()) };
    let changes = owned
        .gains
        .iter()
        .enumerate()
        .map(|(i, g)| AudioTrackGain { tracks: Some(vec![i as u32]), gain: *g, fade_ms: 0 })
        .collect();
    submit(rt, object, None, changes)
}

pub(crate) fn install(rt: &mut Rt) -> Result<()> {
    let Some(wave) = rt.global_member("WaveSoundBuffer").object_handle() else {
        return Err(TjsError::runtime("wumultitrack linked before WaveSoundBuffer exists"));
    };
    if rt.object_member(wave, "multiTrackOpenCallbacks").object_handle().is_some() {
        return Ok(());
    }
    let callbacks = rt.alloc_array_object(vec![]);
    rt.set_object_member(wave, "multiTrackOpenCallbacks", Variant::Object(callbacks));
    // setMultiTrackMask(id, tracks, gain, time = 0)
    rt.register_object_native_with_arg_count(wave, "setMultiTrackMask", NativeArgCount::AtLeast(1), |rt: &mut Rt, _, a: Vec<Variant>| {
        let object = match instance(rt, &a[0])? {
            Ok(o) => o,
            Err(v) => return Ok(v),
        };
        let tracks = selection(rt, object, a.get(1).unwrap_or(&Variant::Void))?;
        let change = AudioTrackGain { tracks, gain: gain_arg(a.get(2))?, fade_ms: time_arg(a.get(3))? };
        record(rt, object, &change);
        submit(rt, object, None, vec![change])?;
        Ok(Variant::Void)
    });
    // setDelayMultiTrackMask(id, positions, [tracks, gain, ...], time = 0)
    rt.register_object_native_with_arg_count(wave, "setDelayMultiTrackMask", NativeArgCount::AtLeast(1), |rt: &mut Rt, _, a: Vec<Variant>| {
        let Some(Variant::Object(masks)) = a.get(2).cloned() else {
            return Err(TjsError::runtime("invalid parameter"));
        };
        let object = match instance(rt, &a[0])? {
            Ok(o) => o,
            Err(v) => return Ok(v),
        };
        let time = time_arg(a.get(3))?;
        let entries: Vec<Variant> = rt.array_elements(masks).map(|e| e.to_vec()).unwrap_or_default();
        let mut changes = Vec::new();
        for pair in entries.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            let tracks = selection(rt, object, &pair[0])?;
            changes.push(AudioTrackGain { tracks, gain: gain_arg(Some(&pair[1]))?, fade_ms: time });
        }
        let positions: Vec<u64> = match a.get(1) {
            Some(Variant::Object(h)) => rt
                .array_elements(*h)
                .unwrap_or_default()
                .iter()
                .map(|v| v.to_integer().map(|n| n.max(0) as u64))
                .collect::<Result<_>>()?,
            Some(v) if !matches!(v, Variant::Void) => vec![v.to_integer()?.max(0) as u64],
            _ => vec![],
        };
        if changes.is_empty() {
            return Ok(Variant::Void);
        }
        for c in &changes {
            record(rt, object, c);
        }
        submit(rt, object, Some(positions), changes)?;
        Ok(Variant::Void)
    });
    // rebindMultiTrackID(oldId, newId) → 1 when rebound (0x100055D0/0x1000FB80)
    rt.register_object_native_with_arg_count(wave, "rebindMultiTrackID", NativeArgCount::AtLeast(2), |rt: &mut Rt, _, a: Vec<Variant>| {
        let (Variant::String(old), Variant::String(new)) = (&a[0], &a[1]) else {
            return Err(err("the track id must be a string"));
        };
        let (old, new) = (old.to_string(), new.to_string());
        if old.is_empty() || new.is_empty() {
            return Ok(Variant::Void);
        }
        let object = match instance(rt, &a[0])? {
            Ok(o) => o,
            Err(_) => return Ok(Variant::Void),
        };
        let registry = &mut rt.host_mut().multitrack;
        registry.instances.remove(&old);
        registry.instances.insert(new.clone(), object);
        if let Some(owned) = registry.owned.get_mut(&object) {
            owned.id = new;
        }
        Ok(Variant::Integer(1))
    });
    for (name, key) in [("setGainQueryCallback_mtbiOPUS", "__mt_gain_opus"), ("setGainQueryCallback_mtbiVORBIS", "__mt_gain_vorbis")] {
        rt.register_object_native(wave, name, move |rt: &mut Rt, _, a: Vec<Variant>| {
            let Some(wave) = rt.global_member("WaveSoundBuffer").object_handle() else { return Ok(Variant::Void) };
            rt.set_object_member(wave, key, a.into_iter().next().unwrap_or(Variant::Void));
            Ok(Variant::Void)
        });
    }
    Ok(())
}
