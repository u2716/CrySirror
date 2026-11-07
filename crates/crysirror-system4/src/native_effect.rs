// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Pinned effect.c/SACT2.c and complete native B290,
// GLEffectCopy dispatch44B0/basic1..5/52xx..57xx/19F0/1B80/2390/2450,
// input7820/7830 and SactEngine6080/61B0 inspected statically. No DLL runs.
//! Native transition timing/return policy over immutable shared image jobs.
mod blind;
mod block;
mod blur;
mod feedback;
mod geometry;
mod mask;
mod quake;
mod radial;
mod rotation;
use crate::{
    adapter::{Bridge, Dispatch},
    native_cg::NativeCg,
    native_scene_present::NativePresent,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
    },
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    image_blend::{
        RgbaBlendArithmetic as A, RgbaBlendComponent as C, RgbaBlendEquation as E,
        RgbaBlendFactor as F, RgbaBlendState as S, RgbaByteRounding as Q,
    },
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
        ImageResource,
    },
    service::{HostServices, ImageMetadata},
    state::ClientId,
};
use crysirror_vfx::falling_atlas::SequenceRng;
use serde_json::json;
use std::sync::Arc;
struct Capture {
    old_job: ImageJob,
    new_job: ImageJob,
    old: Option<Arc<dyn ImageResource>>,
    new: Option<Arc<dyn ImageResource>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Publication {
    Active,
    Endpoint,
    Final,
}
enum Phase {
    Capture(Capture),
    Running {
        old: Arc<dyn ImageResource>,
        new: Arc<dyn ImageResource>,
        previous: Arc<dyn ImageResource>,
        start: u32,
        frame: Option<(ImageJob, Publication)>,
        next: u64,
    },
}
struct Run {
    call: ExternalCall,
    size: [u32; 2],
    kind: i32,
    duration: u32,
    key: bool,
    images: Arc<dyn ImageMetadata>,
    phase: Phase,
    skip: bool,
    return_void: bool,
    immediate: bool,
    endpoint_seen: bool,
    quake: quake::Quake,
    mask: Option<Arc<dyn ImageResource>>,
    feedback_anchor: Option<Arc<dyn ImageResource>>,
    block_mask: Option<block::BlockMask>,
}
pub(crate) struct NativeEffect {
    owner: ClientId,
    native_profile: Option<bool>,
    run: Option<Run>,
    mask: mask::Mask,
    quake: quake::Quake,
    rng: SequenceRng,
}
impl NativeEffect {
    pub(crate) fn new(owner: ClientId) -> Self {
        Self {
            owner,
            native_profile: None,
            run: None,
            mask: Default::default(),
            quake: Default::default(),
            rng: SequenceRng::msvc(1),
        }
    }
    pub(crate) fn configure_original_profile(&mut self, verified: bool) {
        self.native_profile = Some(verified);
    }
    pub(crate) fn has_pending(&self) -> bool {
        self.run.is_some() || self.mask.has_pending()
    }
    pub(crate) fn cancel(&mut self) {
        self.run = None;
        self.mask.cancel();
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(self.run.is_none(), "native Effect changed target");
            return Ok(None);
        };
        let p = m.program.clone();
        let l = &p.libraries[library];
        let f = &l.functions[function];
        if l.name != b"SACT2" {
            ensure!(!self.has_pending(), "native Effect changed HLL");
            return Ok(None);
        }
        ensure!(self.owner == owner, "native Effect foreign owner");
        if matches!(f.name.as_slice(), b"EffectSetMask" | b"EffectSetMaskSP") {
            ensure!(
                self.run.is_none() && self.native_profile == Some(true),
                "native mask requires reviewed idle arithmetic profile"
            );
            return self.mask.dispatch(owner, m, call, bridge, cg, host);
        }
        ensure!(!self.mask.has_pending(), "native mask changed pending HLL");
        if matches!(
            f.name.as_slice(),
            b"QUAKE_SET_CROSS" | b"QUAKE_SET_ROTATION"
        ) {
            ensure!(
                self.run.is_none() && self.native_profile == Some(true),
                "native quake configuration requires reviewed idle profile"
            );
            ensure!(f.return_kind == 0, "native quake setter return ABI");
            let args = m.hll_arguments(call)?;
            let [
                Argument::Scalar { kind: 10, bits: a },
                Argument::Scalar { kind: 10, bits: b },
            ] = args.as_slice()
            else {
                bail!("native quake setter argument ABI")
            };
            let (a, b) = (*a, *b);
            m.complete_hll(call, HllReturn::Void)?;
            if f.name == b"QUAKE_SET_CROSS" {
                self.quake.set_cross(a, b)
            } else {
                self.quake.set_rotation(a, b)
            }
            return Ok(Some(Dispatch::Completed));
        }
        let return_void = f.name == b"QuakeScreen";
        if f.name != b"Effect" && !return_void {
            ensure!(self.run.is_none(), "native Effect changed HLL");
            return Ok(None);
        }
        ensure!(
            f.return_kind == if return_void { 0 } else { 10 },
            "native Effect return ABI"
        );
        if self.run.is_none() {
            let args = m.hll_arguments(call)?;
            let (kind, duration, key, quake) = if return_void {
                ensure!(
                    self.native_profile == Some(true),
                    "native QuakeScreen requires reviewed timing profile"
                );
                let [
                    Argument::Scalar { kind: 10, bits: x },
                    Argument::Scalar { kind: 10, bits: y },
                    Argument::Scalar {
                        kind: 10,
                        bits: duration,
                    },
                    Argument::Scalar {
                        kind: 10 | 47,
                        bits: key,
                    },
                ] = args.as_slice()
                else {
                    bail!("native QuakeScreen argument ABI")
                };
                (27, *duration, *key, quake::Quake::cross(*x, *y))
            } else {
                let [
                    Argument::Scalar {
                        kind: 10,
                        bits: kind,
                    },
                    Argument::Scalar {
                        kind: 10,
                        bits: duration,
                    },
                    Argument::Scalar {
                        kind: 10 | 47,
                        bits: key,
                    },
                ] = args.as_slice()
                else {
                    bail!("native Effect argument ABI")
                };
                (*kind, *duration, *key, self.quake)
            };
            //B290's zero type OR zero time is real Update, not an unknown-type
            //fallback. Reject unsupported families before capturing/mutating.
            let fast = kind == 0 || duration == 0;
            if kind == 27 && duration > 0 {
                quake.validate()?;
            }
            let mask = if matches!(kind, 18 | 42) && !fast {
                Some(
                    self.mask
                        .resource
                        .as_ref()
                        .context("native Effect requires initialized alpha mask")?
                        .clone(),
                )
            } else {
                None
            };
            ensure!(
                fast || (1..=5).contains(&kind)
                    || self.native_profile == Some(true)
                        && (matches!(kind, 12 | 13 | 18 | 24 | 27 | 42 | 45 | 46 | 53)
                            || blur::recognizes(kind)
                            || matches!(kind, 7 | 8 | 9)
                            || geometry::recognizes(kind)
                            || rotation::recognizes(kind)
                            || feedback::recognizes(kind)),
                "native Effect {} is not implemented; original call remains pending",
                kind
            );
            ensure!(
                fast || (1..=3_600_000).contains(&duration),
                "native Effect duration outside bounded clock profile"
            );
            let size = bridge
                .sact
                .as_ref()
                .filter(|s| s.initialized)
                .context("native Effect before SACT.Init")?
                .canvas;
            if !fast && matches!(kind, 7 | 8 | 9) {
                blind::validate(kind, size, duration as u32)?;
            }
            if !fast && rotation::recognizes(kind) {
                rotation::validate(kind, size, duration as u32)?;
            }
            if !fast && kind == 53 {
                radial::validate(duration as u32)?;
            }
            if !fast && geometry::recognizes(kind) {
                geometry::validate(kind, size, duration as u32)?;
            }
            if !fast && blur::recognizes(kind) {
                blur::validate(kind, size, duration as u32)?;
            }
            if matches!(kind, 12 | 13 | 45 | 46) && !fast {
                let length = size[usize::from(matches!(kind, 12 | 13))];
                ensure!(
                    (u64::from(length) + 254) * (duration as u64) <= i32::MAX as u64,
                    "native directional Effect signed timeline multiply exceeds defined profile"
                );
            }
            let images = host.image_metadata(owner)?;
            let old_input = if return_void {
                ImageInput::Solid {
                    size,
                    color: [0, 0, 0, 255],
                }
            } else {
                cg.sprite_input(owner, -1, bridge)?
            };
            let old_job = images.begin_process(old_input, b"[]".as_slice().into())?;
            let new_job = NativePresent::begin_frame(owner, bridge, host)?;
            //Original ICheckClickGLE clears key flags at effect begin, even
            //when cancellation is disabled. The physical input journal remains
            //host-owned; no replacement OS event loop is introduced.
            if !fast || return_void {
                bridge.input.clear(false);
            }
            self.run = Some(Run {
                call: call.clone(),
                size,
                kind: if fast && !return_void { 0 } else { kind },
                duration: if fast { 0 } else { duration as u32 },
                key: key != 0,
                images,
                phase: Phase::Capture(Capture {
                    old_job,
                    new_job,
                    old: None,
                    new: None,
                }),
                skip: false,
                return_void,
                immediate: !fast && blur::immediate(kind, duration as u32),
                endpoint_seen: false,
                quake,
                mask,
                feedback_anchor: None,
                block_mask: None,
            });
        }
        let run = self.run.as_mut().unwrap();
        ensure!(run.call == *call, "native Effect stale receipt");
        if let Phase::Capture(c) = &mut run.phase {
            if c.old.is_none() {
                c.old = poll(&mut c.old_job, owner, run.size)?;
            }
            if c.new.is_none() {
                c.new = poll(&mut c.new_job, owner, run.size)?;
            }
            if c.old.is_none() || c.new.is_none() {
                return Ok(Some(wait(host)));
            }
            let old = c.old.take().unwrap();
            let new = c.new.take().unwrap();
            if matches!(run.kind, 24 | 27) {
                self.rng = SequenceRng::msvc(host.now_ms() as u32);
            }
            if run.kind == 24 {
                run.block_mask = Some(block::BlockMask::new(run.size, &mut self.rng)?);
            }
            run.phase = Phase::Running {
                previous: old.clone(),
                old,
                new,
                start: host.now_ms() as u32,
                frame: None,
                next: host.now_ms(),
            };
        }
        let native = self.native_profile.unwrap_or(false);
        let Phase::Running {
            old,
            new,
            previous,
            start,
            frame,
            next,
        } = &mut run.phase
        else {
            unreachable!()
        };
        if let Some((job, final_frame)) = frame.as_mut() {
            let Some(resource) = poll(job, owner, run.size)? else {
                return Ok(Some(wait(host)));
            };
            let publication = *final_frame;
            cg.present_window_result(owner, resource.clone(), bridge, host)?;
            //Original stateful kernels and the scale<.01 early-return keep
            //the last completed destination, not a recaptured visible screen.
            *previous = resource;
            *frame = None;
            if publication == Publication::Final {
                //Native B290 returns the out-result (0 normal/1 key skip),
                //then61B0 invalidates and redraws the complete new scene.
                //The public C implementation instead returns1 on success.
                let value = if native && run.kind != 0 {
                    i32::from(run.skip)
                } else {
                    1
                };
                m.complete_hll(
                    call,
                    if run.return_void {
                        HllReturn::Void
                    } else {
                        HllReturn::Scalar(value)
                    },
                )?;
                self.run = None;
            } else {
                if publication == Publication::Endpoint {
                    run.endpoint_seen = true;
                }
                *next = host.now_ms().saturating_add(1);
            }
            return Ok(Some(Dispatch::Presented));
        }
        if host.now_ms() < *next {
            return Ok(Some(Dispatch::WaitUntil(*next)));
        }
        let elapsed = (host.now_ms() as u32).wrapping_sub(*start);
        let final_frame = run.kind == 0 || run.immediate || elapsed >= run.duration;
        if !final_frame && run.key {
            run.skip = [13, 17, 1].into_iter().try_fold(false, |found, key| {
                Ok::<_, anyhow::Error>(found || bridge.input.key(key)? != 0)
            })?;
        }
        if run.kind == 0 || run.skip || final_frame && run.endpoint_seen {
            //61B0's actual scene redraw follows an expired endpoint or a key
            //skip. A skip omits the endpoint, as the native loop does.
            *frame = Some((
                run.images
                    .begin_process(ImageInput::Resource(new.clone()), b"[]".as_slice().into())?,
                Publication::Final,
            ));
        } else if final_frame {
            //The native renderer presents the effect endpoint and Sleep(1)
            //before61B0 redraws the scene. In particular fadeout/TVoff must
            //show black, and whiteout white, before the final scene appears.
            let endpoint = match run.kind {
                2 | 32 => ImageInput::Solid {
                    size: run.size,
                    color: [0, 0, 0, 255],
                },
                4 => ImageInput::Solid {
                    size: run.size,
                    color: [255; 4],
                },
                _ => ImageInput::Resource(new.clone()),
            };
            *frame = Some((
                run.images
                    .begin_process(endpoint, b"[]".as_slice().into())?,
                Publication::Endpoint,
            ));
        } else {
            let input = if matches!(run.kind, 7 | 8 | 9) {
                blind::sample(run.kind, elapsed, run.duration, old.clone(), new.clone())?
            } else if run.kind == 24 {
                run.block_mask
                    .as_ref()
                    .context("native block random table absent")?
                    .sample(elapsed, run.duration, old.clone(), new.clone())?
            } else if run.kind == 53 {
                radial::sample(elapsed, run.duration, old.clone(), new.clone())?
            } else if run.kind == 52 {
                if elapsed >= run.duration * 80 / 100 && run.feedback_anchor.is_none() {
                    run.feedback_anchor = Some(previous.clone());
                }
                feedback::sample_double(
                    elapsed,
                    run.duration,
                    new.clone(),
                    previous.clone(),
                    run.feedback_anchor.clone(),
                )?
            } else if feedback::recognizes(run.kind) {
                feedback::sample(
                    run.kind,
                    elapsed,
                    run.duration,
                    new.clone(),
                    previous.clone(),
                )?
            } else if rotation::recognizes(run.kind) {
                rotation::sample(
                    run.kind,
                    elapsed,
                    run.duration,
                    old.clone(),
                    new.clone(),
                    previous.clone(),
                )?
            } else if geometry::recognizes(run.kind) {
                geometry::sample(run.kind, elapsed, run.duration, old.clone(), new.clone())?
            } else if blur::recognizes(run.kind) {
                blur::sample(run.kind, elapsed, run.duration, old.clone(), new.clone())?
            } else if run.kind == 27 {
                run.quake.sample(
                    elapsed,
                    run.duration,
                    old.clone(),
                    new.clone(),
                    &mut self.rng,
                )?
            } else if matches!(run.kind, 18 | 42) {
                mask::sample(
                    run.kind,
                    elapsed,
                    run.duration,
                    old.clone(),
                    new.clone(),
                    run.mask
                        .as_ref()
                        .context("native Effect mask lease lost")?
                        .clone(),
                )?
            } else {
                sample(
                    run.kind,
                    elapsed,
                    run.duration,
                    old.clone(),
                    new.clone(),
                    native,
                )?
            };
            *frame = Some((
                run.images.begin_process(input, b"[]".as_slice().into())?,
                Publication::Active,
            ));
        }
        Ok(Some(wait(host)))
    }
}
fn wait(host: &dyn HostServices) -> Dispatch {
    Dispatch::WaitUntil(host.now_ms().saturating_add(1))
}
fn poll(
    job: &mut ImageJob,
    owner: ClientId,
    size: [u32; 2],
) -> Result<Option<Arc<dyn ImageResource>>> {
    match job.poll()? {
        ImageProcessState::Pending => Ok(None),
        ImageProcessState::Failed(error) => bail!("native Effect image: {error}"),
        ImageProcessState::Ready(image) => {
            ensure!(
                image.client() == owner && image.size() == size,
                "native Effect image scope/extent"
            );
            Ok(Some(image))
        }
    }
}
fn layer(
    base: ImageInput,
    input: ImageInput,
    rect: [i32; 4],
    position: [i32; 2],
    math: ImageCompositeMath,
) -> ImageInput {
    base.with_composite_layer(ImageCompositeLayer {
        input: Box::new(input),
        source: rect,
        destination: position,
        clip: None,
        opacity: 255,
        math,
    })
}
fn fixed(
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    weight: u32,
    bits: u8,
) -> Result<ImageInput> {
    Ok(ImageInput::Process{input:Box::new(ImageInput::Transition{from:old,to:new,mask:None}),
        effects:serde_json::to_vec(&json!([{"kind":"transition","spec":{"progress":0.0,"softness":0.0,"blur":0.0,
            "fixed_weighted":{"left_weight":(1u32<<bits)-weight,"right_weight":weight,"fractional_bits":bits,"rounding":"sum_then_shift","alpha":"opaque"}}}]))?.into()})
}
fn sample(
    kind: i32,
    elapsed: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    native: bool,
) -> Result<ImageInput> {
    ensure!(
        elapsed < duration && duration > 0,
        "native Effect active sample clock"
    );
    let size = old.size();
    ensure!(new.size() == size, "native Effect frame extents");
    let rect = [0, 0, size[0] as i32, size[1] as i32];
    let alpha = (255u64 * u64::from(elapsed) / u64::from(duration)) as u32;
    if kind == 1 {
        if native {
            //Original default MMX groups four RGB24 pixels and quantizes
            //alpha>>1 toQ7; scalar remainder usesQ8. No actual SIMD code is
            //executed; both profiles are ordinary shared fixed-mix jobs.
            let bulk = size[0] / 4 * 4;
            if bulk == 0 {
                return fixed(old, new, alpha, 8);
            }
            let input = fixed(old.clone(), new.clone(), alpha >> 1, 7)?;
            if bulk == size[0] {
                return Ok(input);
            }
            let tail = fixed(old, new, alpha, 8)?;
            return Ok(layer(
                input,
                tail,
                [bulk as i32, 0, (size[0] - bulk) as i32, size[1] as i32],
                [bulk as i32, 0],
                ImageCompositeMath::CopyRgb,
            ));
        }
        return Ok(ImageInput::Process{input:Box::new(ImageInput::Transition{from:old,to:new,mask:None}),effects:serde_json::to_vec(&json!([{"kind":"transition","spec":{"progress":elapsed as f32/duration as f32,"softness":0.0,"blur":0.0}}]))?.into()});
    }
    if matches!(kind, 12 | 13 | 45 | 46) {
        ensure!(
            native,
            "native directional Effect requires reviewed arithmetic profile"
        );
        return axis_mix(kind, elapsed, duration, old, new);
    }
    ensure!((2..=5).contains(&kind), "native Effect sampler family");
    let source = if kind == 2 || kind == 4 { old } else { new };
    let weight = if kind == 2 || kind == 4 {
        255 - alpha
    } else {
        alpha
    };
    let white = kind == 4 || kind == 5;
    let mut state = S {
        write_mask: [true, true, true, false],
        ..Default::default()
    };
    state.constant = [weight as f32 / 255.; 4];
    state.color = C {
        source: F::ConstantAlpha,
        destination: if white {
            F::OneMinusConstantAlpha
        } else {
            F::Zero
        },
        equation: E::Add,
    };
    if native {
        state.arithmetic = A::Byte255 {
            source: Q::Nearest,
            destination: Q::Nearest,
        };
    }
    let base = ImageInput::Solid {
        size,
        color: if white { [255; 4] } else { [0, 0, 0, 255] },
    };
    Ok(layer(
        base,
        ImageInput::Resource(source),
        rect,
        [0, 0],
        ImageCompositeMath::Normalized(state),
    ))
}
/// Native12/13/45/46 use a moving254-sample band, with one nearest /255
/// lookup per input contribution. Compose one cropped source plus a shared
/// alpha ramp, instead of making a complete image job for every scanline.
fn axis_mix(
    kind: i32,
    elapsed: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    let vertical = kind == 12 || kind == 13;
    let reverse = kind == 13 || kind == 46;
    let size = old.size();
    let axis = usize::from(vertical);
    let length = i64::from(size[axis]);
    //The original signed-multiply/IDIV path is only defined without overflow.
    //Use the exact quotient in a wide integer after the signed-range preflight.
    let phase = if reverse { duration - elapsed } else { elapsed };
    let origin = (length + 254) * i64::from(phase) / i64::from(duration) - 254;
    let at = origin.max(0).min(length);
    let count = (origin + 254).min(length).saturating_sub(at).max(0);
    let mut output = ImageInput::Resource(old);
    let copy_start = if reverse { at + count } else { 0 };
    let copy_count = if reverse { length - copy_start } else { at };
    let rectangle = |start: i64, count: i64| {
        if vertical {
            [0, start as i32, size[0] as i32, count as i32]
        } else {
            [start as i32, 0, count as i32, size[1] as i32]
        }
    };
    if copy_count > 0 {
        let rect = rectangle(copy_start, copy_count);
        output = layer(
            output,
            ImageInput::Resource(new.clone()),
            rect,
            [rect[0], rect[1]],
            ImageCompositeMath::CopyRgb,
        );
    }
    if count <= 0 {
        return Ok(output);
    }
    let old_start = if reverse {
        if origin < 0 { origin + 254 } else { 254 }
    } else {
        if origin < 0 { 1 - origin } else { 1 }
    };
    let new_start = 255 - old_start;
    let new_end = if reverse {
        new_start + count
    } else {
        new_start - count
    };
    ensure!(
        (0..=255).contains(&new_start) && (0..=255).contains(&new_end),
        "native directional Effect ramp profile"
    );
    let rect = rectangle(at, count);
    let crop_size = [rect[2] as u32, rect[3] as u32];
    let cropped = layer(
        ImageInput::Solid {
            size: crop_size,
            color: [0, 0, 0, 255],
        },
        ImageInput::Resource(new),
        rect,
        [0, 0],
        ImageCompositeMath::CopyRgb,
    );
    let ramp=ImageInput::Process{input:Box::new(cropped),effects:serde_json::to_vec(&json!([{"kind":"linear_gradient","spec":{
        "vertical":vertical,"start":[0,0,0,new_start],"end":[0,0,0,new_end],"write_mask":[false,false,false,true],"sample":"start_truncate"}}]))?.into()};
    let state = S {
        arithmetic: A::Byte255 {
            source: Q::Nearest,
            destination: Q::Nearest,
        },
        write_mask: [true, true, true, false],
        color: C {
            source: F::SourceAlpha,
            destination: F::OneMinusSourceAlpha,
            equation: E::Add,
        },
        ..Default::default()
    };
    Ok(layer(
        output,
        ramp,
        [0, 0, rect[2], rect[3]],
        [rect[0], rect[1]],
        ImageCompositeMath::Normalized(state),
    ))
}
