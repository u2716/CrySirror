// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! wvdecoder.dll: `WaveSoundData.F32` / `WaveSoundData.S16`, whole decoded
//! sounds as script-readable sample buffers.
//!
//! Native findings (IDA on game-kr `plugin/wvdecoder.dll`): registration
//! 0x100078C0 (`WaveSoundData` with the `F32` and `S16` classes),
//! constructor 0x10007DC0 (an optional storage argument opens at once),
//! `open` 0x1000A9C0 (answers true, or throws `cannot file open: %1`,
//! `unknown file format: %1`, `cannot decode: %1`), `getSample(index,
//! channel)` 0x10009B70 (void outside the data), `setSample(index, value,
//! channel)` 0x1000B7F0 (answers the previous value; a void value stores 0),
//! `totalTime` 0x1000B9E0 (`samples·1000/frequency`, real), `format`
//! 0x100096A0 (`channels << 24 | 0x10020` for F32).
//!
//! `foreach` 0x100030A0, `getLipSync` 0x100036F0, `makeThumbnail`
//! 0x1000A440 (bucketing 0x10002720) and `save` 0x1000B0E0 (RIFF writer
//! 0x1000C110) follow the same image.
//!
//! Decoding goes through the shared audio resource and source-sample
//! services, so `open` suspends the script until the data is resident.
//! `rawdata` answers an opaque token where the native answers an address.
use crate::natives::{Rt, string, this_of, unsupported};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{AudioAsset, AudioPreparePhase, AudioResourceHandle, AudioSampleQuery, AudioSampleRequest, AudioSampleState, HostServices},
    state::ClientId,
};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{NativeArgCount, ObjectHandle, Variant},
};
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub(crate) struct Data {
    pub channels: u32,
    pub frequency: u32,
    /// Interleaved, in -1..1.
    pub samples: Vec<f32>,
}
enum Stage {
    Start(VfsPath),
    Resource(u64),
    Reading { resource: AudioResourceHandle, frames: u64, job: Option<Box<dyn AudioSampleQuery>>, data: Data },
}
#[derive(Default)]
pub(crate) struct State {
    pub objects: BTreeMap<usize, Data>,
    pending: Option<(usize, String, Stage)>,
}
impl State {
    pub(crate) fn invalidate(&mut self, object: usize) {
        self.objects.remove(&object);
    }
    pub(crate) fn roots(&self, out: &mut Vec<ObjectHandle>) {
        if let Some((object, ..)) = &self.pending {
            out.push(ObjectHandle(*object));
        }
    }
}
fn err(e: impl std::fmt::Display) -> TjsError {
    TjsError::runtime(e.to_string())
}
fn open(rt: &mut Rt, h: ObjectHandle, name: &str) -> Result<()> {
    if !rt.host().can_suspend {
        return Err(unsupported("WaveSoundData.open in this callback", "a resumable TJS boundary"));
    }
    if rt.host().wave_data.pending.is_some() {
        return Err(err("WaveSoundData.open already pending"));
    }
    let path = rt
        .host()
        .storage()?
        .resolve(name)
        .ok_or_else(|| err(format!("cannot file open: {name}")))?;
    let path = VfsPath::new(Mount::Asset, &path).map_err(err)?;
    rt.host_mut().wave_data.objects.remove(&h.0);
    rt.host_mut().wave_data.pending = Some((h.0, name.to_owned(), Stage::Start(path)));
    rt.request_suspend();
    Ok(())
}
/// Advances a pending open; false while the script must stay suspended.
pub(crate) fn poll(rt: &mut Rt, h: &mut dyn HostServices, client: ClientId) -> anyhow::Result<bool> {
    let Some((object, name, stage)) = rt.host_mut().wave_data.pending.take() else {
        return Ok(true);
    };
    let next = match stage {
        Stage::Start(path) => Stage::Resource(h.begin_audio_resource(client, AudioAsset { path, frame_limit: None }, true)?),
        Stage::Resource(ticket) => {
            let status = h.audio_resource_preparation(client, ticket)?;
            match status.phase {
                AudioPreparePhase::Pending => Stage::Resource(ticket),
                AudioPreparePhase::Failed => {
                    h.release_audio_resource_preparation(client, ticket)?;
                    anyhow::bail!("unknown file format: {name}: {}", status.error.unwrap_or_default());
                }
                AudioPreparePhase::Ready => {
                    h.release_audio_resource_preparation(client, ticket)?;
                    let resource = status.resource.ok_or_else(|| anyhow::anyhow!("cannot decode: {name}"))?;
                    let info = status.info.ok_or_else(|| anyhow::anyhow!("cannot decode: {name}"))?;
                    let frames = info.source_frames.ok_or_else(|| anyhow::anyhow!("cannot decode: {name}"))?;
                    anyhow::ensure!(frames <= 16 << 20, "WaveSoundData: {name} is longer than 16 Mi frames");
                    Stage::Reading { resource, frames, job: None, data: Data { frequency: info.source_rate, ..Default::default() } }
                }
            }
        }
        Stage::Reading { resource, frames, mut job, mut data } => {
            let done = if data.channels == 0 { 0 } else { (data.samples.len() / data.channels as usize) as u64 };
            if job.is_none() && (data.channels == 0 || done < frames) {
                job = Some(h.begin_audio_samples(client, resource, AudioSampleRequest { start_frame: done, max_bytes: 1 << 20 })?);
            }
            let mut finished = data.channels != 0 && done >= frames;
            if let Some(mut j) = job.take() {
                match j.poll()? {
                    AudioSampleState::Pending => job = Some(j),
                    AudioSampleState::Ready(receipt) => {
                        data.channels = u32::from(receipt.format.channels);
                        let wide = receipt.format.bits_per_sample == 16;
                        if wide {
                            data.samples.extend(receipt.bytes.chunks_exact(2).map(|s| f32::from(i16::from_le_bytes([s[0], s[1]])) / 32768.));
                        } else {
                            data.samples.extend(receipt.bytes.iter().map(|s| (f32::from(*s) - 128.) / 128.));
                        }
                        finished = receipt.frames == 0 || (data.samples.len() / data.channels.max(1) as usize) as u64 >= frames;
                    }
                }
            }
            if finished {
                h.release_audio_resource(client, resource)?;
                rt.host_mut().wave_data.objects.insert(object, data);
                return Ok(true);
            }
            Stage::Reading { resource, frames, job, data }
        }
    };
    rt.host_mut().wave_data.pending = Some((object, name, next));
    Ok(false)
}
fn data<'a>(rt: &'a Rt, h: ObjectHandle) -> Option<&'a Data> {
    rt.host().wave_data.objects.get(&h.0)
}
/// `index·channels + channel` when inside the data.
fn offset(rt: &Rt, h: ObjectHandle, a: &[Variant], channel_at: usize) -> Result<Option<usize>> {
    let Some(d) = data(rt, h) else { return Ok(None) };
    let index = match a.first() {
        Some(v) if !matches!(v, Variant::Void) => v.to_integer()?,
        _ => 0,
    };
    let channel = match a.get(channel_at) {
        Some(v) if !matches!(v, Variant::Void) => v.to_integer()?,
        _ => 0,
    };
    let frames = (d.samples.len() / d.channels.max(1) as usize) as i64;
    Ok((index >= 0 && index < frames && channel >= 0 && channel < i64::from(d.channels)).then(|| index as usize * d.channels as usize + channel as usize))
}
/// `(start, count)` in frames from optional arguments at `at`, as the
/// native normalises them: negative values count from the end.
fn range(a: &[Variant], at: usize, frames: usize) -> Result<(usize, usize)> {
    let total = frames as i64;
    let arg = |i: usize, default: i64| -> Result<i64> {
        match a.get(i) {
            Some(v) if !matches!(v, Variant::Void) => v.to_integer(),
            _ => Ok(default),
        }
    };
    let (s, c) = (arg(at, 0)?, arg(at + 1, total)?);
    let start = if s >= 0 { s } else { s + total };
    let count = if c >= 0 { c } else { c + total };
    let mut count = if count + start <= total { count } else { total - start };
    if start < 0 {
        count += start;
    }
    let start = start.clamp(0, total);
    Ok((start as usize, count.clamp(0, total - start) as usize))
}
fn sample_variant(float: bool, v: f32) -> Variant {
    if float { Variant::Real(f64::from(v)) } else { Variant::Integer(((v * 32768.).round() as i64).clamp(-32768, 32767)) }
}
fn class(rt: &mut Rt, owner: ObjectHandle, name: &'static str, float: bool) {
    let c = rt.alloc_native_constructor(move |rt: &mut Rt, this: Option<ObjectHandle>, a: Vec<Variant>| {
        let h = this
            .map(|h| rt.bound_this(h).unwrap_or(h))
            .filter(|h| *h != rt.global_handle())
            .unwrap_or_else(|| rt.alloc_ordinary_object());
        rt.add_object_class_info(h, "WaveSoundData");
        let class = rt
            .object_member(rt.global_handle(), "WaveSoundData")
            .object_handle()
            .and_then(|w| rt.object_member(w, name).object_handle());
        if let Some(class) = class.filter(|c| *c != h && rt.object_super_class(h).is_none()) {
            rt.set_object_super_class(h, class);
        }
        if let Some(v) = a.first().filter(|v| !matches!(v, Variant::Void)) {
            let file = v.to_tjs_string()?;
            open(rt, h, &file)?;
        }
        Ok(Variant::Object(h))
    });
    rt.add_object_class_info(c, "WaveSoundData");
    rt.register_object_native_with_arg_count(c, "open", NativeArgCount::AtLeast(1), |rt: &mut Rt, t, a: Vec<Variant>| {
        let h = this_of(rt, t, "WaveSoundData.open")?;
        let file = string(&a, 0, "WaveSoundData.open")?;
        open(rt, h, &file)?;
        Ok(Variant::Integer(1))
    });
    for name in ["close", "finalize"] {
        rt.register_object_native(c, name, |rt: &mut Rt, t, _| {
            let h = this_of(rt, t, "WaveSoundData.close")?;
            rt.host_mut().wave_data.objects.remove(&h.0);
            Ok(Variant::Void)
        });
    }
    rt.register_object_native(c, "getSample", move |rt: &mut Rt, t, a: Vec<Variant>| {
        let h = this_of(rt, t, "WaveSoundData.getSample")?;
        Ok(match offset(rt, h, &a, 1)? {
            Some(at) => sample_variant(float, data(rt, h).unwrap().samples[at]),
            None => Variant::Void,
        })
    });
    rt.register_object_native(c, "setSample", move |rt: &mut Rt, t, a: Vec<Variant>| {
        let h = this_of(rt, t, "WaveSoundData.setSample")?;
        let Some(at) = offset(rt, h, &a, 2)? else { return Ok(Variant::Void) };
        let new = match a.get(1) {
            Some(v) if !matches!(v, Variant::Void) => {
                if float { v.to_real()? as f32 } else { v.to_integer()?.clamp(-32768, 32767) as f32 / 32768. }
            }
            _ => 0.,
        };
        let d = rt.host_mut().wave_data.objects.get_mut(&h.0).unwrap();
        let old = std::mem::replace(&mut d.samples[at], new);
        Ok(sample_variant(float, old))
    });
    // foreach(func, start, count): func(frame, value, channel) per sample;
    // a non-void answer replaces the sample, an object answer stops the walk
    // and is returned.
    rt.register_object_native_with_arg_count(c, "foreach", NativeArgCount::AtLeast(1), move |rt: &mut Rt, t, a: Vec<Variant>| {
        let h = this_of(rt, t, "WaveSoundData.foreach")?;
        let Some(d) = data(rt, h).cloned() else { return Ok(Variant::Void) };
        let ch = d.channels.max(1) as usize;
        let (start, count) = range(&a, 1, d.samples.len() / ch)?;
        for i in start * ch..(start + count) * ch {
            let current = data(rt, h).and_then(|d| d.samples.get(i).copied()).unwrap_or(0.);
            let args = vec![Variant::Integer((i / ch) as i64), sample_variant(float, current), Variant::Integer((i % ch) as i64)];
            let r = rt.call_function(a[0].clone(), args)?;
            match r {
                Variant::Void => {}
                Variant::Object(_) | Variant::Closure(_) => return Ok(r),
                v => {
                    let new = if float { v.to_real()? as f32 } else { v.to_integer()?.clamp(-32768, 32767) as f32 / 32768. };
                    if let Some(s) = rt.host_mut().wave_data.objects.get_mut(&h.0).and_then(|d| d.samples.get_mut(i)) {
                        *s = new;
                    }
                }
            }
        }
        Ok(Variant::Void)
    });
    // getLipSync(func, step, start, count): func(peak², index) per `step`
    // frames (all channels); answers the number of calls.
    rt.register_object_native_with_arg_count(c, "getLipSync", NativeArgCount::AtLeast(2), move |rt: &mut Rt, t, a: Vec<Variant>| {
        let h = this_of(rt, t, "WaveSoundData.getLipSync")?;
        let Some(d) = data(rt, h).cloned() else { return Ok(Variant::Integer(0)) };
        let ch = d.channels.max(1) as usize;
        let step = a[1].to_integer()?;
        let (start, count) = range(&a, 2, d.samples.len() / ch)?;
        let scale = if float { 1f32 } else { 32768. };
        let (mut at, end, mut calls) = (start * ch, (start + count) * ch, 0i64);
        while at < end {
            let stop = if step > 0 { (at + step as usize * ch).min(end) } else { at };
            let peak = d.samples[at..stop].iter().fold(0f32, |m, v| m.max(v * scale * v * scale));
            // A non-positive step never advances; one call keeps that finite.
            at = if stop > at { stop } else { end };
            rt.call_function(a[0].clone(), vec![Variant::Real(f64::from(peak)), Variant::Integer(calls)])?;
            calls += 1;
        }
        Ok(Variant::Integer(calls))
    });
    // makeThumbnail(layer, color, channel = -1, start, count): one min/max
    // column per layer pixel column, drawn as a vertical run of `color`.
    rt.register_object_native_with_arg_count(c, "makeThumbnail", NativeArgCount::AtLeast(2), move |rt: &mut Rt, t, a: Vec<Variant>| {
        const M: &str = "WaveSoundData.makeThumbnail";
        let h = this_of(rt, t, M)?;
        let layer = a.first().and_then(crate::natives::object_arg).map(|o| rt.bound_this(o).unwrap_or(o)).ok_or_else(|| err(format!("{M}: a Layer is required")))?;
        let color = a[1].to_integer()? as u32;
        let Some(d) = data(rt, h).cloned() else { return Ok(Variant::Void) };
        let ch = d.channels.max(1) as usize;
        let channel = match a.get(2) {
            Some(v) if !matches!(v, Variant::Void) => v.to_integer()?,
            _ => -1,
        };
        let channel = if channel >= ch as i64 { -1 } else { channel };
        let (start, count) = range(&a, 3, d.samples.len() / ch)?;
        let (size, rgba) = crate::province_ex::pixels(rt, layer, M)?;
        let (w, hh) = (size[0] as usize, size[1] as usize);
        let cols = count.min(w);
        let mut out = rgba.to_vec();
        let px = [(color >> 16) as u8, (color >> 8) as u8, color as u8, (color >> 24) as u8];
        let (mut lo, mut hi, mut acc, mut x) = (0f32, 0f32, (count >> 1) as i64, 0usize);
        for f in start..start + count {
            let frame = &d.samples[f * ch..(f + 1) * ch];
            let picked = if channel >= 0 { &frame[channel as usize..channel as usize + 1] } else { frame };
            for v in picked {
                hi = hi.max(*v);
                lo = lo.min(*v);
            }
            acc -= cols as i64;
            if acc < 0 {
                acc += count as i64;
                let y0 = (((lo * 0.5 + 0.5) as f64 * hh as f64).round() as i64).max(0);
                let y1 = (((hi * 0.5 + 0.5) as f64 * hh as f64).round() as i64).min(hh as i64 - 1);
                if x < w && y1 >= y0 && y0 < hh as i64 {
                    for y in y0..=y1 {
                        let o = (y as usize * w + x) * 4;
                        out[o..o + 4].copy_from_slice(&px);
                    }
                }
                x += 1;
                lo = 0.;
                hi = 0.;
            }
        }
        let image = crysirror_core::image_processing::RgbaImage::new(ClientId(rt.host().client), size, out.into()).map_err(err)?;
        crate::image_api::queue_chain(rt, layer, crysirror_core::image_processing::ImageInput::Resource(std::sync::Arc::new(image)), vec![])
    });
    // save(storage, format): RIFF WAVE, 16-bit PCM unless the format is
    // true / "F32" (32-bit float).
    rt.register_object_native_with_arg_count(c, "save", NativeArgCount::AtLeast(1), move |rt: &mut Rt, t, a: Vec<Variant>| {
        let h = this_of(rt, t, "WaveSoundData.save")?;
        let name = string(&a, 0, "WaveSoundData.save")?;
        let as_float = match a.get(1) {
            None | Some(Variant::Void) => false,
            Some(Variant::String(s)) => match s.as_str() {
                "F32" => true,
                "S16" => false,
                other => return Err(err(format!("invalid save format: {other}"))),
            },
            Some(v) => v.to_integer()? != 0,
        };
        let d = data(rt, h).cloned().filter(|d| d.channels > 0).ok_or_else(|| err(format!("save failure: {name}")))?;
        let bytes_per = if as_float { 4u32 } else { 2 };
        let body: Vec<u8> = if as_float {
            d.samples.iter().flat_map(|v| v.to_le_bytes()).collect()
        } else {
            d.samples.iter().flat_map(|v| (((v * 32768.).round() as i32).clamp(-32768, 32767) as i16).to_le_bytes()).collect()
        };
        let mut out = Vec::with_capacity(body.len() + 44);
        out.extend(b"RIFF");
        out.extend((36 + body.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend((if as_float { 3u16 } else { 1 }).to_le_bytes());
        out.extend((d.channels as u16).to_le_bytes());
        out.extend(d.frequency.to_le_bytes());
        out.extend((d.frequency * d.channels * bytes_per).to_le_bytes());
        out.extend(((d.channels * bytes_per) as u16).to_le_bytes());
        out.extend(((bytes_per * 8) as u16).to_le_bytes());
        out.extend(b"data");
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        crate::plugin_ports::packinone::write_pack(rt, &name, out)?;
        Ok(Variant::Integer(1))
    });
    type Get = fn(&Data, bool) -> Variant;
    let props: [(&str, Get); 5] = [
        ("channels", |d, _| Variant::Integer(d.channels.into())),
        ("frequency", |d, _| Variant::Integer(d.frequency.into())),
        ("samples", |d, _| Variant::Integer((d.samples.len() / d.channels.max(1) as usize) as i64)),
        ("totalTime", |d, _| {
            let frames = d.samples.len() / d.channels.max(1) as usize;
            Variant::Real(if d.frequency == 0 { 0. } else { frames as f64 * 1000. / f64::from(d.frequency) })
        }),
        ("format", |d, float| Variant::Integer((i64::from(d.channels) << 24) | if float { 0x10020 } else { 0x10 })),
    ];
    for (prop, get) in props {
        rt.register_object_native_property(
            c,
            prop,
            move |rt: &mut Rt, t| {
                let h = this_of(rt, t, "WaveSoundData")?;
                Ok(get(&data(rt, h).cloned().unwrap_or_default(), float))
            },
            crate::natives::read_only,
        );
    }
    // The native answers the sample buffer's address for other native
    // plug-ins; here it is an opaque per-object token.
    rt.register_object_native_property(
        c,
        "rawdata",
        |rt: &mut Rt, t| Ok(Variant::Integer(this_of(rt, t, "WaveSoundData.rawdata")?.0 as i64)),
        crate::natives::read_only,
    );
    rt.set_object_member(owner, name, Variant::Object(c));
}
pub(crate) fn install(rt: &mut Rt) {
    if rt.global_member("WaveSoundData").object_handle().is_some() {
        return;
    }
    let owner = rt.alloc_ordinary_object();
    rt.add_object_class_info(owner, "WaveSoundData");
    rt.set_global_member("WaveSoundData", Variant::Object(owner));
    class(rt, owner, "F32", true);
    class(rt, owner, "S16", false);
}
/// Mono mix of a WaveSoundData object, for `OscLoop`.
pub(crate) fn mono(rt: &Rt, h: ObjectHandle) -> Option<Vec<f32>> {
    let d = data(rt, h)?;
    let ch = d.channels.max(1) as usize;
    Some(d.samples.chunks_exact(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect())
}
