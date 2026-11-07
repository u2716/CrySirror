// GPL-2.0-or-later. Native SACT B590/B650/E490/E500 and GLEffectCopy
// 2AA0/2CC0/9D50/9ED0 read statically; no module execution.
//! Native mask resource/lookup policy. Engine workers decode and blend bytes.
use super::*;
use crate::assets::Class;
use crysirror_core::{
    image_encoding::{ImageEncodingInfoQuery, ImageEncodingInfoState},
    path::Mount,
};
enum Phase {
    Header {
        query: Box<dyn ImageEncodingInfoQuery>,
        path: String,
    },
    Pixels(ImageJob),
    DecodeOnly {
        job: ImageJob,
        size: [u32; 2],
    },
}
struct Pending {
    call: ExternalCall,
    size: [u32; 2],
    images: Arc<dyn ImageMetadata>,
    phase: Phase,
}
#[derive(Default)]
pub(super) struct Mask {
    pub(super) resource: Option<Arc<dyn ImageResource>>,
    pending: Option<Pending>,
}
impl Mask {
    pub(super) fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(super) fn cancel(&mut self) {
        self.pending = None;
        self.resource = None;
    }
    pub(super) fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        cg: &NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(self.pending.is_none(), "native mask target changed");
            return Ok(None);
        };
        let p = m.program.clone();
        let l = &p.libraries[library];
        let f = &l.functions[function];
        if l.name != b"SACT2" || !matches!(f.name.as_slice(), b"EffectSetMask" | b"EffectSetMaskSP")
        {
            ensure!(self.pending.is_none(), "native mask HLL changed");
            return Ok(None);
        }
        ensure!(f.return_kind == 10, "native effect mask return ABI");
        if self.pending.is_none() {
            let args = m.hll_arguments(call)?;
            let [
                Argument::Scalar {
                    kind: 10,
                    bits: number,
                },
            ] = args.as_slice()
            else {
                bail!("native effect mask argument ABI")
            };
            let sact = bridge
                .sact
                .as_ref()
                .filter(|s| s.initialized)
                .context("native mask before SACT.Init")?;
            let images = host.image_metadata(owner)?;
            let phase = if f.name == b"EffectSetMaskSP" {
                let sprites = sact
                    .sprites
                    .as_ref()
                    .context("native mask sprite set absent")?;
                sprites.require_owner(owner)?;
                let Some(sprite) = sprites.get(*number).filter(|s| s.surface.is_some()) else {
                    m.complete_hll(call, HllReturn::Scalar(0))?;
                    return Ok(Some(Dispatch::Completed));
                };
                //E500 returns1 after B590 even when B590 diagnoses an absent
                //alpha plane and keeps the existing mask. This proven native
                //branch is not an ACK for a missing image decoder.
                if !sprite.alpha {
                    m.complete_hll(call, HllReturn::Scalar(1))?;
                    return Ok(Some(Dispatch::Completed));
                }
                Phase::Pixels(begin(
                    owner,
                    sact.canvas,
                    sprite.size,
                    cg.scene_sprite_input(owner, *number, bridge)?,
                    images.as_ref(),
                    self.resource.clone(),
                )?)
            } else {
                let path = bridge
                    .assets
                    .as_ref()
                    .context("native numbered mask assets absent")?
                    .path(Class::Cg, *number)?;
                let Some(path) = path else {
                    m.complete_hll(call, HllReturn::Scalar(0))?;
                    return Ok(Some(Dispatch::Completed));
                };
                let query = images.begin_encoding_info(&path)?;
                ensure!(query.client() == owner, "native mask header owner");
                Phase::Header { query, path }
            };
            self.pending = Some(Pending {
                call: call.clone(),
                size: sact.canvas,
                images,
                phase,
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(pending.call == *call, "native effect mask stale receipt");
        match &mut pending.phase {
            Phase::Header { query, path } => match query.poll()? {
                ImageEncodingInfoState::Pending => {}
                ImageEncodingInfoState::Failed(crysirror_core::Error::NotFound) => {
                    m.complete_hll(call, HllReturn::Scalar(0))?;
                    self.pending = None;
                    return Ok(Some(Dispatch::Completed));
                }
                ImageEncodingInfoState::Failed(error) => {
                    bail!("native effect mask header: {error}")
                }
                ImageEncodingInfoState::Ready(header) => {
                    ensure!(
                        header.client == owner
                            && header.asset.mount() == Mount::Asset
                            && header.asset.relative() == path.as_str(),
                        "native mask header scope/path"
                    );
                    let alpha = header
                        .encoding
                        .has_alpha_plane
                        .context("native mask alpha-plane metadata unavailable")?;
                    let input = ImageInput::Asset {
                        path: path.clone(),
                        size: header.encoding.size,
                        fingerprint: Some(header.fingerprint),
                    };
                    pending.phase = if alpha {
                        Phase::Pixels(begin(
                            owner,
                            pending.size,
                            header.encoding.size,
                            input,
                            pending.images.as_ref(),
                            self.resource.clone(),
                        )?)
                    } else {
                        //Native decoding precedes B590's absent-alpha branch.
                        //A valid header is not a completed payload decode.
                        Phase::DecodeOnly {
                            size: header.encoding.size,
                            job: pending
                                .images
                                .begin_process(input, b"[]".as_slice().into())?,
                        }
                    };
                }
            },
            Phase::DecodeOnly { job, size } => {
                if poll(job, owner, *size)?.is_some() {
                    m.complete_hll(call, HllReturn::Scalar(1))?;
                    self.pending = None;
                    return Ok(Some(Dispatch::Completed));
                }
            }
            Phase::Pixels(job) => {
                if let Some(resource) = poll(job, owner, pending.size)? {
                    self.resource = Some(resource);
                    m.complete_hll(call, HllReturn::Scalar(1))?;
                    self.pending = None;
                    return Ok(Some(Dispatch::Completed));
                }
            }
        }
        Ok(Some(wait(host)))
    }
}
fn begin(
    owner: ClientId,
    size: [u32; 2],
    source_size: [u32; 2],
    input: ImageInput,
    images: &dyn ImageMetadata,
    previous: Option<Arc<dyn ImageResource>>,
) -> Result<ImageJob> {
    //Native B590 preserves an existing allocation's uncovered coverage. On
    //its FIRST partial copy those bytes were uninitialized. Authorized repair:
    //initialize that otherwise-undefined tail to0; never claim the original
    //random bytes had this value. Defined source bytes and later tails retain
    //their actual native copy/keep semantics.
    let base = if let Some(previous) = previous {
        ensure!(
            previous.client() == owner && previous.size() == size,
            "native mask previous lease scope/extent"
        );
        ImageInput::Resource(previous)
    } else {
        ImageInput::Solid {
            size,
            color: [0; 4],
        }
    };
    ensure!(
        source_size.into_iter().all(|n| n > 0),
        "native mask source dimensions"
    );
    let graph = layer(
        base,
        input,
        [
            0,
            0,
            size[0].min(source_size[0]) as i32,
            size[1].min(source_size[1]) as i32,
        ],
        [0, 0],
        ImageCompositeMath::CopyAlpha,
    );
    Ok(images.begin_process(graph, b"[]".as_slice().into())?)
}
/// Bounded256-entry native control coefficients, not a pixel implementation.
/// Stored mask bytes and all per-pixel operations stay in shared Image workers.
pub(super) fn sample(
    kind: i32,
    time: u32,
    duration: u32,
    old: Arc<dyn ImageResource>,
    new: Arc<dyn ImageResource>,
    mask: Arc<dyn ImageResource>,
) -> Result<ImageInput> {
    let size = old.size();
    ensure!(
        size == new.size() && size == mask.size(),
        "native mask transition extents"
    );
    let threshold = (u64::from(if kind == 18 { 255u32 } else { 511u32 }) * u64::from(time)
        / u64::from(duration)) as i32;
    let table = (0..256)
        .map(|mask: i32| -> u8 {
            if kind == 18 {
                if mask <= threshold {
                    255
                } else if mask < threshold + 16 {
                    ((threshold + 16 - mask) * 16 - 1) as u8
                } else {
                    0
                }
            } else {
                (threshold - mask).clamp(0, 255) as u8
            }
        })
        .collect::<Vec<_>>();
    let weights = ImageInput::Process {
        input: Box::new(ImageInput::Resource(mask)),
        effects: serde_json::to_vec(&json!([{"kind":"alpha_lookup","table":table}]))?.into(),
    };
    let source = layer(
        ImageInput::Resource(new),
        weights,
        [0, 0, size[0] as i32, size[1] as i32],
        [0, 0],
        ImageCompositeMath::CopyAlpha,
    );
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
        ImageInput::Resource(old),
        source,
        [0, 0, size[0] as i32, size[1] as i32],
        [0, 0],
        ImageCompositeMath::Normalized(state),
    ))
}
