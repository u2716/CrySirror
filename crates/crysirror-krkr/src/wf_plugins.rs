// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! `wfBasicEffect.dll` and `wfTypicalDSP.dll`: WaveSoundBuffer filter
//! classes whose DSP runs as the shared `crysirror_audio::wave_filters`
//! stages of a transformed audio resource.
//!
//! Native findings (IDA 9.0 on the RiddleJoker `plugin/` copies;
//! wfBasicEffect 226 816 B, wfTypicalDSP 1 183 232 B):
//!
//! * Both register their classes as members of the `WaveSoundBuffer` class
//!   object (`WaveSoundBuffer.StkFreeVerb`, `.GraphicEqualizer`,
//!   `.DelayEffect`, `.WaveDSPFilter`); every class has a read-only
//!   `interface` property (the `iTVPBasicWaveFilter*` the engine connects at
//!   open). Games reach them through `sysscn/voiceeffect.tjs`
//!   `VoiceEffectFactory`: `Delay/Verb/EQ(...)` construct with the ini
//!   arguments, `DSP(type, family, ...)` constructs
//!   `WaveDSPFilter(type, family)` and calls `setParams(void, ...)`.
//! * DelayEffect (C++ `WaveDelay`): constructor 0x10008090 stores extend
//!   1000 ms and zero time/feedback/mix; `init` 0x10002360 needs at least
//!   three arguments (time integer, feedback real, mix real; feedback and
//!   mix kept as floats) and an optional fourth (extend integer). Sample loop
//!   0x10005EA0; line sizing 0x1000A260 (`time·rate/1000`, never shrinks);
//!   extend frames 0x10008D20.
//! * StkFreeVerb: `stk::FreeVerb` with `Stk::sampleRate` left at 44100
//!   (0x10034350 is only read), properties `effectMix`, `roomSize`,
//!   `damping`, `width`, `mode`, `extend`; the filter connects as stereo
//!   (0x10004F20) and only processes two-channel frames (0x10002E70).
//! * GraphicEqualizer: gains at +0xC4 default 1.0; `setGain(band, gain)`
//!   0x10002250 ignores bands above 9, `getGain(band)` 0x100022F0 answers 0
//!   for them. Coefficients 0x100078B0, per-sample 0x10002A60.
//! * WaveDSPFilter: DSPFilters `SmoothedFilterDesign<…, 1|2 channels, State>`
//!   (RTTI). Constructor 0x100020F0 parses `(type, family, structure,
//!   transitionSamples)` through the name tables at 0x100CBD38/0x100CBDA0/
//!   0x100CBDF8, defaults AllPass/RBJ/DirectFormII/1024 (0x10002003).
//!   `setParams` 0x10002A30 writes argument i to parameter i for i below
//!   the design's parameter count (void or missing: the design default),
//!   zero above it, and answers the count; parameter 0 is then replaced by
//!   the stream rate (0x10002630, 44100 while unconnected). `getParamInfo(i)`
//!   0x10002740 answers `{label, name, defaultValue, currentValue, min,
//!   max}` for i < count. The filter is recreated at connect, so its first
//!   parameters apply without the 1024-sample transition; only changes made
//!   while connected ramp.
//!
//! Filters apply when the WaveSoundBuffer opens (`crate::sound`), as the
//! native chain is rebuilt then. Changes made while the buffer is open take
//! effect at its next open.
use crate::natives::{Rt, methods, this_of, unsupported};
use crysirror_core::service::{IirFamily, IirKind, IirStructure, Oscillator, WaveFilter, iir_default_params, iir_params};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{ObjectHandle, Variant},
};

const KINDS: &[(&str, IirKind)] = &[
    ("LowPass", IirKind::LowPass),
    ("HighPass", IirKind::HighPass),
    ("BandPass", IirKind::BandPass),
    ("BandPass1", IirKind::BandPass),
    ("BandPass2", IirKind::BandPass2),
    ("BandStop", IirKind::BandStop),
    ("LowShelf", IirKind::LowShelf),
    ("HighShelf", IirKind::HighShelf),
    ("BandShelf", IirKind::BandShelf),
    ("AllPass", IirKind::AllPass),
    ("TwoPole", IirKind::TwoPole),
    ("OnePole", IirKind::OnePole),
];
const FAMILIES: &[(&str, IirFamily)] = &[
    ("RBJ", IirFamily::Rbj),
    ("Butterworth", IirFamily::Butterworth),
    ("ChebyshevI", IirFamily::ChebyshevI),
    ("Chebyshev1", IirFamily::ChebyshevI),
    ("ChebyshevII", IirFamily::ChebyshevII),
    ("Chebyshev2", IirFamily::ChebyshevII),
    ("Elliptic", IirFamily::Elliptic),
    ("Bessel", IirFamily::Bessel),
    ("Legendre", IirFamily::Legendre),
    ("Custom", IirFamily::Custom),
];
const STRUCTURES: &[(&str, IirStructure)] = &[
    ("DirectFormI", IirStructure::DirectFormI),
    ("DirectForm1", IirStructure::DirectFormI),
    ("DirectFormII", IirStructure::DirectFormII),
    ("DirectForm2", IirStructure::DirectFormII),
    ("TransposedDirectFormI", IirStructure::TransposedDirectFormI),
    ("TransposedDirectForm1", IirStructure::TransposedDirectFormI),
    ("TransposedDirectFormII", IirStructure::TransposedDirectFormII),
    ("TransposedDirectForm2", IirStructure::TransposedDirectFormII),
];

fn err(e: impl std::fmt::Display) -> TjsError {
    TjsError::runtime(e.to_string())
}
fn f32r(v: f64) -> Variant {
    Variant::Real(v as f32 as f64)
}
fn real_of(rt: &Rt, h: ObjectHandle, key: &str) -> f64 {
    rt.object_member(h, key).to_real().unwrap_or(0.)
}
fn int_of(rt: &Rt, h: ObjectHandle, key: &str) -> i64 {
    rt.object_member(h, key).to_integer().unwrap_or(0)
}
fn given(args: &[Variant], i: usize) -> Option<&Variant> {
    args.get(i).filter(|v| !matches!(v, Variant::Void))
}
fn instance(rt: &mut Rt, this: Option<ObjectHandle>, class: &str) -> ObjectHandle {
    let h = this
        .map(|h| rt.bound_this(h).unwrap_or(h))
        .filter(|h| *h != rt.global_handle())
        .unwrap_or_else(|| rt.alloc_ordinary_object());
    rt.add_object_class_info(h, class.to_string());
    h
}
fn wave_class(rt: &mut Rt) -> Result<ObjectHandle> {
    rt.object_member(rt.global_handle(), "WaveSoundBuffer")
        .object_handle()
        .ok_or_else(|| err("wave filter plug-in linked before WaveSoundBuffer exists"))
}
/// A class object installed as `WaveSoundBuffer.<name>`.
fn class(rt: &mut Rt, name: &'static str, init: fn(&mut Rt, ObjectHandle, &[Variant]) -> Result<()>) -> Result<ObjectHandle> {
    let wave = wave_class(rt)?;
    let c = rt.alloc_native_constructor(move |rt: &mut Rt, this: Option<ObjectHandle>, args: Vec<Variant>| {
        let h = instance(rt, this, name);
        // Instances resolve the class's properties and methods through it.
        let class = wave_class(rt)?;
        if let Some(class) = rt.object_member(class, name).object_handle()
            && rt.object_super_class(h).is_none()
            && class != h
        {
            rt.set_object_super_class(h, class);
        }
        init(rt, h, &args)?;
        Ok(Variant::Object(h))
    });
    rt.add_object_class_info(c, name);
    rt.register_object_native_property(
        c,
        "interface",
        |rt: &mut Rt, t| Ok(Variant::Integer(this_of(rt, t, "wave filter interface")?.0 as i64)),
        crate::natives::read_only,
    );
    rt.register_object_native(c, "finalize", |_: &mut Rt, _, _| Ok(Variant::Void));
    rt.set_object_member(wave, name, Variant::Object(c));
    Ok(c)
}
/// A real-valued property backed by the instance member `key`.
fn real_property(rt: &mut Rt, c: ObjectHandle, name: &'static str, key: &'static str, set: fn(f64) -> f64) {
    rt.register_object_native_property(
        c,
        name,
        move |rt: &mut Rt, t| Ok(rt.object_member(this_of(rt, t, name)?, key)),
        move |rt: &mut Rt, t, v: Variant| {
            let h = this_of(rt, t, name)?;
            let v = set(v.to_real()?);
            rt.set_object_member(h, key, Variant::Real(v));
            Ok(())
        },
    );
}

fn delay_init(rt: &mut Rt, h: ObjectHandle, args: &[Variant]) -> Result<()> {
    if args.len() > 2 {
        let time = args[0].to_integer()?;
        rt.set_object_member(h, "__wf_time", Variant::Integer(time));
        rt.set_object_member(h, "__wf_feedback", f32r(args[1].to_real()?));
        rt.set_object_member(h, "__wf_mix", f32r(args[2].to_real()?));
    }
    if args.len() > 3 {
        let extend = args[3].to_integer()?;
        rt.set_object_member(h, "__wf_extend", Variant::Integer(extend));
    }
    Ok(())
}

/// `Plugins.link("wfBasicEffect.dll")`.
pub fn install_basic(rt: &mut Rt) -> Result<()> {
    let delay = class(rt, "DelayEffect", |rt, h, args| {
        rt.set_object_member(h, "__wf", Variant::String("delay".into()));
        rt.set_object_member(h, "__wf_time", Variant::Integer(0));
        rt.set_object_member(h, "__wf_feedback", Variant::Real(0.));
        rt.set_object_member(h, "__wf_mix", Variant::Real(0.));
        rt.set_object_member(h, "__wf_extend", Variant::Integer(1000));
        delay_init(rt, h, args)
    })?;
    methods(
        rt,
        delay,
        &[("init", |rt, t, a| {
            let h = this_of(rt, t, "DelayEffect.init")?;
            delay_init(rt, h, &a)?;
            Ok(Variant::Void)
        })],
    );

    let verb = class(rt, "StkFreeVerb", |rt, h, args| {
        rt.set_object_member(h, "__wf", Variant::String("verb".into()));
        let defaults = [0.75, 0.75, 0.25, 1.0, 0., 1000.];
        let keys = ["__wf_mix", "__wf_room", "__wf_damp", "__wf_width", "__wf_mode", "__wf_extend"];
        for (i, (key, default)) in keys.iter().zip(defaults).enumerate() {
            let v = match given(args, i) {
                Some(v) => v.to_real()?,
                None => default,
            };
            let v = match i {
                0 => v.clamp(0., 1.),
                4 => f64::from(v != 0.),
                5 => v.trunc(),
                _ => v,
            };
            rt.set_object_member(h, *key, Variant::Real(v));
        }
        Ok(())
    })?;
    real_property(rt, verb, "effectMix", "__wf_mix", |v| v.clamp(0., 1.));
    real_property(rt, verb, "roomSize", "__wf_room", |v| v);
    real_property(rt, verb, "damping", "__wf_damp", |v| v);
    real_property(rt, verb, "width", "__wf_width", |v| v);
    real_property(rt, verb, "mode", "__wf_mode", |v| f64::from(v != 0.));
    real_property(rt, verb, "extend", "__wf_extend", f64::trunc);

    let eq = class(rt, "GraphicEqualizer", |rt, h, args| {
        rt.set_object_member(h, "__wf", Variant::String("eq".into()));
        for band in 0..10 {
            let g = match given(args, band) {
                Some(v) => v.to_real()?,
                None => 1.,
            };
            rt.set_object_member(h, format!("__wf_g{band}"), f32r(g));
        }
        Ok(())
    })?;
    methods(
        rt,
        eq,
        &[
            ("setGain", |rt, t, a| {
                let h = this_of(rt, t, "GraphicEqualizer.setGain")?;
                let gain = a.get(1).ok_or_else(|| err("GraphicEqualizer.setGain(band, gain)"))?.to_real()?;
                let band = a.first().ok_or_else(|| err("GraphicEqualizer.setGain(band, gain)"))?.to_integer()?;
                if (0..=9).contains(&band) {
                    rt.set_object_member(h, format!("__wf_g{band}"), f32r(gain));
                }
                Ok(Variant::Void)
            }),
            ("getGain", |rt, t, a| {
                let h = this_of(rt, t, "GraphicEqualizer.getGain")?;
                let band = a.first().ok_or_else(|| err("GraphicEqualizer.getGain(band)"))?.to_integer()?;
                Ok(if (0..=9).contains(&band) { rt.object_member(h, &format!("__wf_g{band}")) } else { Variant::Real(0.) })
            }),
        ],
    );
    install_extended(rt)
}

fn real_arg(a: &[Variant], i: usize, default: f64) -> Result<f64> {
    match given(a, i) {
        Some(v) => v.to_real(),
        None => Ok(default),
    }
}
fn set_real(rt: &mut Rt, h: ObjectHandle, key: &str, v: f64) {
    rt.set_object_member(h, key, Variant::Real(v));
}
/// An oscillator class: instances carry `__osc` and an `oscillator` property.
fn osc_class(rt: &mut Rt, name: &'static str, init: fn(&mut Rt, ObjectHandle, &[Variant]) -> Result<()>) -> Result<()> {
    let c = class(rt, name, init)?;
    rt.register_object_native_property(
        c,
        "oscillator",
        |rt: &mut Rt, t| Ok(Variant::Integer(this_of(rt, t, "oscillator")?.0 as i64)),
        crate::natives::read_only,
    );
    Ok(())
}
fn osc_arg(rt: &Rt, v: Option<&Variant>, what: &str) -> Result<ObjectHandle> {
    v.and_then(crate::natives::object_arg)
        .map(|h| rt.bound_this(h).unwrap_or(h))
        .filter(|h| matches!(rt.object_member(*h, "__osc"), Variant::String(_)))
        .ok_or_else(|| err(format!("{what}: an oscillator object is required")))
}
fn blit_init(rt: &mut Rt, h: ObjectHandle, a: &[Variant], kind: &str) -> Result<()> {
    if a.len() < 2 {
        return Err(TjsError::bad_param_count());
    }
    rt.set_object_member(h, "__osc", Variant::String(kind.into()));
    set_real(rt, h, "__osc_gain", real_arg(a, 0, 1.)?);
    set_real(rt, h, "__osc_freq", a[1].to_real()?);
    let harmonics = match given(a, 2) {
        Some(v) => v.to_integer()?,
        None => 0,
    };
    rt.set_object_member(h, "__osc_hm", Variant::Integer(harmonics));
    Ok(())
}
/// The classes the later wfBasicEffect builds add (game-kr 422 912 B; IDA:
/// GainLimit arguments 0x10007DD0 and samples 0x10008860, Math* samples
/// 0x100103D0/0x10010490/0x10010530, RingModulation 0x100185A0, Vocoder
/// constructor 0x10015300 with the inlined voclib 0x10018D30, oscillator
/// constructors 0x10014500..0x10014C70 and generators 0x10016FE0..0x100178C0;
/// oscillator frequencies are rescaled to the stream rate at connect,
/// 0x10017DC0). OscNoise without a seed draws engine random bits there; here
/// it uses a fixed seed. OscLoop takes wvdecoder's WaveSoundData.
fn install_extended(rt: &mut Rt) -> Result<()> {
    class(rt, "GainLimit", |rt, h, a| {
        rt.set_object_member(h, "__wf", Variant::String("limit".into()));
        let (mut gain, mut th, mut mode) = (1f64, 1f64, 0i64);
        if !a.is_empty() {
            gain = 10f64.powf(a[0].to_real()? / 20.);
            th = real_arg(a, 1, 1.)?.abs();
            mode = match a.get(2) {
                Some(v) => v.to_integer()?,
                None => 0,
            };
        }
        rt.set_object_member(h, "__wf_gain", f32r(gain));
        rt.set_object_member(h, "__wf_threshold", f32r(th));
        rt.set_object_member(h, "__wf_mode", Variant::Integer(mode.clamp(-1, 5)));
        Ok(())
    })?;
    class(rt, "MathAbs", |rt, h, _| {
        rt.set_object_member(h, "__wf", Variant::String("abs".into()));
        Ok(())
    })?;
    class(rt, "MathConstAdd", |rt, h, a| {
        rt.set_object_member(h, "__wf", Variant::String("add".into()));
        rt.set_object_member(h, "__wf_value", f32r(real_arg(a, 0, 0.)?));
        Ok(())
    })?;
    class(rt, "MathConstMul", |rt, h, a| {
        rt.set_object_member(h, "__wf", Variant::String("mul".into()));
        rt.set_object_member(h, "__wf_value", f32r(real_arg(a, 0, 1.)?));
        Ok(())
    })?;
    class(rt, "RingModulation", |rt, h, a| {
        let osc = osc_arg(rt, a.first(), "RingModulation")?;
        rt.set_object_member(h, "__wf", Variant::String("ring".into()));
        rt.set_object_member(h, "__wf_osc", Variant::Object(osc));
        Ok(())
    })?;
    let vocoder = class(rt, "Vocoder", |rt, h, a| {
        let osc = osc_arg(rt, a.first(), "Vocoder")?;
        rt.set_object_member(h, "__wf", Variant::String("vocoder".into()));
        rt.set_object_member(h, "__wf_osc", Variant::Object(osc));
        set_real(rt, h, "__wf_formant", real_arg(a, 1, 1.)? as f32 as f64);
        set_real(rt, h, "__wf_reaction", real_arg(a, 2, 0.03)? as f32 as f64);
        set_real(rt, h, "__wf_bands", real_arg(a, 3, 24.)?.trunc().clamp(4., 96.));
        set_real(rt, h, "__wf_fpb", real_arg(a, 4, 6.)?.trunc().clamp(1., 8.));
        Ok(())
    })?;
    real_property(rt, vocoder, "formantShift", "__wf_formant", |v| v as f32 as f64);
    real_property(rt, vocoder, "reactionTime", "__wf_reaction", |v| v as f32 as f64);
    real_property(rt, vocoder, "bands", "__wf_bands", |v| v.trunc().clamp(4., 96.));
    real_property(rt, vocoder, "filtersPerBand", "__wf_fpb", |v| v.trunc().clamp(1., 8.));

    osc_class(rt, "OscConst", |rt, h, a| {
        rt.set_object_member(h, "__osc", Variant::String("const".into()));
        set_real(rt, h, "__osc_gain", real_arg(a, 0, 1.)?);
        Ok(())
    })?;
    osc_class(rt, "OscSineWave", |rt, h, a| {
        if a.len() < 2 {
            return Err(TjsError::bad_param_count());
        }
        rt.set_object_member(h, "__osc", Variant::String("sine".into()));
        set_real(rt, h, "__osc_gain", real_arg(a, 0, 1.)?);
        set_real(rt, h, "__osc_freq", a[1].to_real()?);
        Ok(())
    })?;
    osc_class(rt, "OscNoise", |rt, h, a| {
        rt.set_object_member(h, "__osc", Variant::String("noise".into()));
        set_real(rt, h, "__osc_gain", real_arg(a, 0, 1.)?);
        let seed = match given(a, 1) {
            Some(v) => v.to_integer()?,
            None => 0,
        };
        rt.set_object_member(h, "__osc_seed", Variant::Integer(seed));
        Ok(())
    })?;
    osc_class(rt, "OscBlit", |rt, h, a| blit_init(rt, h, a, "blit"))?;
    osc_class(rt, "OscBlitSaw", |rt, h, a| blit_init(rt, h, a, "saw"))?;
    osc_class(rt, "OscBlitSquare", |rt, h, a| blit_init(rt, h, a, "square"))?;
    osc_class(rt, "OscModulate", |rt, h, a| {
        if a.len() < 4 {
            return Err(TjsError::bad_param_count());
        }
        rt.set_object_member(h, "__osc", Variant::String("modulate".into()));
        set_real(rt, h, "__osc_gain", real_arg(a, 0, 1.)?);
        set_real(rt, h, "__osc_freq", a[1].to_real()?);
        set_real(rt, h, "__osc_vgain", a[2].to_real()?);
        set_real(rt, h, "__osc_rgain", a[3].to_real()?);
        Ok(())
    })?;
    osc_class(rt, "OscMix", |rt, h, a| {
        rt.set_object_member(h, "__osc", Variant::String("mix".into()));
        let mut list = vec![];
        for v in a {
            list.push(Variant::Object(osc_arg(rt, Some(v), "OscMix")?));
        }
        let arr = rt.alloc_array_object(list);
        rt.set_object_member(h, "__osc_list", Variant::Object(arr));
        Ok(())
    })?;
    osc_class(rt, "OscFilter", |rt, h, a| {
        let source = osc_arg(rt, a.first(), "OscFilter")?;
        rt.set_object_member(h, "__osc", Variant::String("filter".into()));
        rt.set_object_member(h, "__osc_src", Variant::Object(source));
        let arr = rt.alloc_array_object(a.iter().skip(1).cloned().collect());
        rt.set_object_member(h, "__osc_list", Variant::Object(arr));
        Ok(())
    })?;
    // OscLoop(data, pitch): the samples are copied, so the script may
    // invalidate the WaveSoundData afterwards (as voiceeffect.tjs does).
    osc_class(rt, "OscLoop", |rt, h, a| {
        let source = a.first().and_then(crate::natives::object_arg).map(|o| rt.bound_this(o).unwrap_or(o));
        let samples = source
            .and_then(|o| crate::wv_decoder::mono(rt, o))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| err("OscLoop: an opened WaveSoundData is required"))?;
        rt.set_object_member(h, "__osc", Variant::String("loop".into()));
        set_real(rt, h, "__osc_freq", real_arg(a, 1, 1.)?);
        // The copy lives beside the WaveSoundData buffers, keyed by this object.
        rt.host_mut().wave_data.objects.insert(h.0, crate::wv_decoder::Data { channels: 1, frequency: 0, samples });
        Ok(())
    })?;
    Ok(())
}
fn children(rt: &Rt, h: ObjectHandle) -> Vec<ObjectHandle> {
    rt.object_member(h, "__osc_list")
        .object_handle()
        .map(|l| rt.array_elements(l).unwrap_or(&[]).iter().filter_map(crate::natives::object_arg).collect())
        .unwrap_or_default()
}
fn oscillator(rt: &Rt, h: ObjectHandle, depth: u32) -> Result<Oscillator> {
    if depth > 8 {
        return Err(err("oscillator nesting too deep"));
    }
    let kind = rt.object_member(h, "__osc").to_tjs_string()?;
    let gain = real_of(rt, h, "__osc_gain") as f32;
    let freq = real_of(rt, h, "__osc_freq") as f32;
    let harmonics = int_of(rt, h, "__osc_hm").clamp(0, 1 << 20) as u32;
    Ok(match kind.as_str() {
        "const" => Oscillator::Const { value: gain },
        "sine" => Oscillator::Sine { gain, freq },
        "noise" => {
            let seed = int_of(rt, h, "__osc_seed");
            Oscillator::Noise { gain, seed: (seed != 0).then_some(seed as u64) }
        }
        "blit" => Oscillator::Blit { gain, freq, harmonics },
        "saw" => Oscillator::BlitSaw { gain, freq, harmonics },
        "square" => Oscillator::BlitSquare { gain, freq, harmonics },
        "modulate" => Oscillator::Modulate {
            gain,
            vibrato_rate: freq,
            vibrato_gain: real_of(rt, h, "__osc_vgain"),
            random_gain: real_of(rt, h, "__osc_rgain"),
        },
        "mix" => Oscillator::Mix {
            sources: children(rt, h).into_iter().map(|c| oscillator(rt, c, depth + 1)).collect::<Result<_>>()?,
        },
        "filter" => {
            let source = rt.object_member(h, "__osc_src").object_handle().ok_or_else(|| err("OscFilter source"))?;
            let mut filters = vec![];
            for f in children(rt, h) {
                if let Some(s) = stage(rt, f, depth + 1)? {
                    filters.push(s);
                }
            }
            Oscillator::Filter { source: Box::new(oscillator(rt, source, depth + 1)?), filters }
        }
        "loop" => {
            let samples = crate::wv_decoder::mono(rt, h).ok_or_else(|| err("OscLoop samples"))?;
            Oscillator::Loop { samples: samples.into(), pitch: f64::from(freq) }
        }
        other => return Err(err(format!("unknown oscillator kind {other}"))),
    })
}
fn carrier(rt: &Rt, f: ObjectHandle, depth: u32) -> Result<Oscillator> {
    let osc = rt.object_member(f, "__wf_osc").object_handle().ok_or_else(|| err("wave filter carrier"))?;
    oscillator(rt, osc, depth + 1)
}

fn lookup<T: Copy>(table: &[(&str, T)], name: &str) -> Option<usize> {
    table.iter().position(|(n, _)| *n == name)
}
fn dsp_design(rt: &Rt, h: ObjectHandle) -> (IirKind, IirFamily, IirStructure) {
    let k = KINDS[int_of(rt, h, "__wf_type") as usize].1;
    let f = FAMILIES[int_of(rt, h, "__wf_family") as usize].1;
    let s = STRUCTURES[int_of(rt, h, "__wf_structure") as usize].1;
    (k, f, s)
}
fn dsp_params(rt: &Rt, h: ObjectHandle) -> [f64; 8] {
    let mut p = [0.; 8];
    for (i, v) in p.iter_mut().enumerate() {
        *v = real_of(rt, h, &format!("__wf_p{i}"));
    }
    p
}

/// `Plugins.link("wfTypicalDSP.dll")`.
pub fn install_dsp(rt: &mut Rt) -> Result<()> {
    let c = class(rt, "WaveDSPFilter", |rt, h, args| {
        rt.set_object_member(h, "__wf", Variant::String("dsp".into()));
        let mut index = [
            lookup(KINDS, "AllPass").unwrap(),
            lookup(FAMILIES, "RBJ").unwrap(),
            lookup(STRUCTURES, "DirectFormII").unwrap(),
        ];
        let tables: [&dyn Fn(&str) -> Option<usize>; 3] =
            [&|n| lookup(KINDS, n), &|n| lookup(FAMILIES, n), &|n| lookup(STRUCTURES, n)];
        for (i, find) in tables.iter().enumerate() {
            if let Some(v) = given(args, i) {
                let name = v.to_tjs_string()?;
                // An unknown name keeps the previous selection.
                if let Some(found) = find(&name) {
                    index[i] = found;
                }
            }
        }
        let transition = match given(args, 3) {
            Some(v) => v.to_integer()?,
            None => 1024,
        };
        rt.set_object_member(h, "__wf_type", Variant::Integer(index[0] as i64));
        rt.set_object_member(h, "__wf_family", Variant::Integer(index[1] as i64));
        rt.set_object_member(h, "__wf_structure", Variant::Integer(index[2] as i64));
        rt.set_object_member(h, "__wf_transition", Variant::Integer(transition));
        let (kind, family, _) = dsp_design(rt, h);
        let defaults = iir_default_params(kind, family).ok_or_else(|| {
            err(format!("WaveDSPFilter: {kind:?} is not available in the {family:?} family"))
        })?;
        for (i, v) in defaults.iter().enumerate() {
            rt.set_object_member(h, format!("__wf_p{i}"), Variant::Real(*v));
        }
        Ok(())
    })?;
    methods(
        rt,
        c,
        &[
            ("setParams", |rt, t, a| {
                let h = this_of(rt, t, "WaveDSPFilter.setParams")?;
                let (kind, family, _) = dsp_design(rt, h);
                let info = iir_params(kind, family).unwrap_or_default();
                for i in 0..8 {
                    let v = match (i < info.len(), given(&a, i)) {
                        (false, _) => 0.,
                        (true, Some(v)) => v.to_real()?,
                        (true, None) => info[i].default,
                    };
                    rt.set_object_member(h, format!("__wf_p{i}"), Variant::Real(v));
                }
                Ok(Variant::Integer(info.len() as i64))
            }),
            ("getParamInfo", |rt, t, a| {
                let h = this_of(rt, t, "WaveDSPFilter.getParamInfo")?;
                let index = a.first().ok_or_else(|| err("WaveDSPFilter.getParamInfo(index)"))?.to_integer()?;
                let (kind, family, _) = dsp_design(rt, h);
                let info = iir_params(kind, family).unwrap_or_default();
                let Some(p) = usize::try_from(index).ok().filter(|i| *i <= 7).and_then(|i| info.get(i)) else {
                    return Ok(Variant::Void);
                };
                let current = if index == 0 { 44100. } else { real_of(rt, h, &format!("__wf_p{index}")) };
                let d = rt.alloc_dictionary_object();
                rt.set_object_member(d, "label", Variant::String(p.label.into()));
                rt.set_object_member(d, "name", Variant::String(p.name.into()));
                rt.set_object_member(d, "defaultValue", Variant::Real(p.default));
                rt.set_object_member(d, "currentValue", Variant::Real(current));
                rt.set_object_member(d, "min", Variant::Real(p.min));
                rt.set_object_member(d, "max", Variant::Real(p.max));
                Ok(Variant::Object(d))
            }),
        ],
    );
    Ok(())
}

/// The shared stage of one `WaveSoundBuffer.filters` element, `None` for an
/// identity PhaseVocoder.
fn stage(rt: &Rt, f: ObjectHandle, depth: u32) -> Result<Option<WaveFilter>> {
    let kind = rt.object_member(f, "__wf");
    let kind = match &kind {
        Variant::String(s) => s.to_string(),
        _ => {
            if !matches!(rt.object_member(f, "__vocoder"), Variant::Integer(1)) {
                return Err(unsupported("WaveSoundBuffer filter", "a known KiriKiri WaveFilter object"));
            }
            let (pitch, time) = (real_of(rt, f, "pitch") as f32, real_of(rt, f, "time") as f32);
            if pitch == 1. && time == 1. {
                return Ok(None);
            }
            return Ok(Some(WaveFilter::PhaseVocoder {
                pitch,
                time,
                window: int_of(rt, f, "__window").clamp(0, 1 << 20) as u32,
                overlap: int_of(rt, f, "__overlap").clamp(0, 64) as u32,
            }));
        }
    };
    let ms = |key: &str| -> Result<u32> {
        u32::try_from(int_of(rt, f, key)).map_err(|_| err(format!("wave filter {key} out of range")))
    };
    Ok(Some(match kind.as_str() {
        "delay" => WaveFilter::Delay {
            time_ms: ms("__wf_time")?,
            feedback: real_of(rt, f, "__wf_feedback") as f32,
            mix: real_of(rt, f, "__wf_mix") as f32,
            extend_ms: ms("__wf_extend")?,
        },
        "verb" => WaveFilter::FreeVerb {
            effect_mix: real_of(rt, f, "__wf_mix"),
            room_size: real_of(rt, f, "__wf_room"),
            damping: real_of(rt, f, "__wf_damp"),
            width: real_of(rt, f, "__wf_width"),
            frozen: real_of(rt, f, "__wf_mode") != 0.,
            extend_ms: u32::try_from(real_of(rt, f, "__wf_extend") as i64).map_err(|_| err("StkFreeVerb.extend out of range"))?,
        },
        "eq" => {
            let mut gains = [1f32; 10];
            for (i, g) in gains.iter_mut().enumerate() {
                *g = real_of(rt, f, &format!("__wf_g{i}")) as f32;
            }
            WaveFilter::GraphicEq { gains }
        }
        "dsp" => {
            let (kind, family, structure) = dsp_design(rt, f);
            WaveFilter::Iir { kind, family, structure, params: dsp_params(rt, f) }
        }
        "limit" => WaveFilter::GainLimit {
            gain: real_of(rt, f, "__wf_gain") as f32,
            threshold: real_of(rt, f, "__wf_threshold") as f32,
            mode: int_of(rt, f, "__wf_mode") as i8,
        },
        "abs" => WaveFilter::Abs,
        "add" => WaveFilter::Add { value: real_of(rt, f, "__wf_value") as f32 },
        "mul" => WaveFilter::Mul { value: real_of(rt, f, "__wf_value") as f32 },
        "ring" => WaveFilter::RingMod { carrier: carrier(rt, f, depth)? },
        "vocoder" => WaveFilter::Vocoder {
            carrier: carrier(rt, f, depth)?,
            formant_shift: real_of(rt, f, "__wf_formant") as f32,
            reaction_time: real_of(rt, f, "__wf_reaction") as f32,
            bands: real_of(rt, f, "__wf_bands") as u8,
            filters_per_band: real_of(rt, f, "__wf_fpb") as u8,
        },
        other => return Err(err(format!("unknown wave filter kind {other}"))),
    }))
}

/// The shared chain of a WaveSoundBuffer's `filters` array.
pub(crate) fn chain(rt: &Rt, buffer: ObjectHandle) -> Result<Vec<WaveFilter>> {
    let Some(list) = rt.object_member(buffer, "filters").object_handle() else {
        return Ok(vec![]);
    };
    let mut out = vec![];
    for f in rt.array_elements(list).unwrap_or(&[]).to_vec() {
        let Some(f) = crate::natives::object_arg(&f) else { continue };
        if let Some(s) = stage(rt, f, 0)? {
            out.push(s);
        }
    }
    if out.len() > 16 {
        return Err(err("WaveSoundBuffer.filters: more than 16 active filters"));
    }
    Ok(out)
}
