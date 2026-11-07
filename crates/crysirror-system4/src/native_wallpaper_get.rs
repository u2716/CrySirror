// GPL-2.0-or-later. Native E3F0→SactEngine64D0/5750/05810, full C/asm
//and dirty-rectangle reset read statically. Original modules are never run.
//! Complete backdrop snapshot policy over scoped immutable Image services.
use crate::{
    adapter::{Bridge, Dispatch},
    native_cg::NativeCg,
    native_scene::NativeScene,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
    },
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
        ImageResource,
    },
    service::{HostServices, ImageMetadata},
    state::ClientId,
};
use std::sync::Arc;
enum Phase {
    Raw(ImageJob),
    Target {
        raw: Arc<dyn ImageResource>,
        job: ImageJob,
    },
}
struct Pending {
    call: ExternalCall,
    number: i32,
    size: [u32; 2],
    images: Arc<dyn ImageMetadata>,
    phase: Phase,
}
pub(crate) struct NativeWallpaperGet {
    owner: ClientId,
    pending: Option<Pending>,
}
impl NativeWallpaperGet {
    pub(crate) fn new(owner: ClientId) -> Self {
        Self {
            owner,
            pending: None,
        }
    }
    pub(crate) fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn cancel(&mut self) {
        self.pending = None;
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        scene: &NativeScene,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(self.pending.is_none(), "native WP_GetSP target changed");
            return Ok(None);
        };
        let p = m.program.clone();
        let l = &p.libraries[library];
        let f = &l.functions[function];
        if l.name != b"SACT2" || f.name != b"WP_GetSP" {
            ensure!(self.pending.is_none(), "native WP_GetSP HLL changed");
            return Ok(None);
        }
        ensure!(
            owner == self.owner && f.return_kind == 10,
            "native WP_GetSP owner/return ABI"
        );
        if self.pending.is_none() {
            let args = m.hll_arguments(call)?;
            let [
                Argument::Scalar {
                    kind: 10,
                    bits: number,
                },
            ] = args.as_slice()
            else {
                bail!("native WP_GetSP argument ABI")
            };
            ensure!(
                (-1..65536).contains(number),
                "native WP_GetSP sprite budget"
            );
            let sact = bridge
                .sact
                .as_ref()
                .filter(|s| s.initialized)
                .context("native WP_GetSP before Init")?;
            sact.sprites
                .as_ref()
                .context("native WP_GetSP sprite service")?
                .require_owner(owner)?;
            let size = sact.canvas;
            let images = host.image_metadata(owner)?;
            let input = scene.apply_backdrop(owner, size, cg.sprite_input(owner, -1, bridge)?)?;
            let job = images.begin_process(input, b"[]".as_slice().into())?;
            self.pending = Some(Pending {
                call: call.clone(),
                number: *number,
                size,
                images,
                phase: Phase::Raw(job),
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(pending.call == *call, "native WP_GetSP stale receipt");
        let (job, raw) = match &mut pending.phase {
            Phase::Raw(job) => (job, None),
            Phase::Target { raw, job } => (job, Some(raw.clone())),
        };
        match job.poll()? {
            ImageProcessState::Pending => {}
            ImageProcessState::Failed(error) => bail!("native WP_GetSP image: {error}"),
            ImageProcessState::Ready(resource) => {
                ensure!(
                    resource.client() == owner && resource.size() == pending.size,
                    "native WP_GetSP resource scope/extent"
                );
                if let Some(raw) = raw {
                    cg.install_wallpaper_snapshot(
                        owner,
                        pending.number,
                        raw,
                        resource,
                        bridge,
                        host,
                    )?;
                    //Original clears its engine dirty vector then inserts the
                    //whole window rect. This adapter always captures the full
                    //scene on Update, so the same full invalidation is retained.
                    m.complete_hll(call, HllReturn::Scalar(1))?;
                    self.pending = None;
                    return Ok(Some(Dispatch::Completed));
                }
                let size = pending.size;
                let target = ImageInput::Solid {
                    size,
                    color: [0, 0, 0, 255],
                }
                .with_composite_layer(ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(resource.clone())),
                    source: [0, 0, size[0] as i32, size[1] as i32],
                    destination: [0, 0],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::CopyRgb,
                });
                pending.phase = Phase::Target {
                    raw: resource,
                    job: pending
                        .images
                        .begin_process(target, b"[]".as_slice().into())?,
                };
            }
        }
        Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
    }
}
