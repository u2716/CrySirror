// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Pinned sact_Update/scene_render and native SactEngine
// retained scene policy. All snapshots, rendering and pixels are engine-owned.
use crate::{
    adapter::{Bridge, Dispatch},
    native_cg::NativeCg,
    vm::{ExternalCall, ExternalTarget, Machine, external::HllReturn},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState},
    service::HostServices,
    state::ClientId,
};
use serde_json::json;
struct Pending {
    call: ExternalCall,
    size: [u32; 2],
    job: ImageJob,
}
pub(crate) struct NativePresent {
    owner: ClientId,
    pending: Option<Pending>,
}
impl NativePresent {
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
    /// Capture the original retained sprite scene, independently of the last
    /// presented native WindowSurface. Root placement/hidden state is excluded
    /// by shared CopySubtree; each child's own visibility/order is retained.
    pub(crate) fn begin_frame(
        owner: ClientId,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<ImageJob> {
        let sact = bridge
            .sact
            .as_ref()
            .filter(|s| s.initialized)
            .context("native scene before SACT.Init")?;
        sact.sprites
            .as_ref()
            .context("native sprite scene absent")?
            .require_owner(owner)?;
        let [w, h] = sact.canvas;
        let batch = json!({"canvases":[{"op":"clear","id":1,"color":[0,0,0,255]},
            {"op":"copy_subtree","source":crate::sact::BACK_SCENE,"destination":1,"rect":[0,0,w,h],"position":[0,0]}]});
        host.submit_engine(owner, 1, &serde_json::to_vec(&batch)?)?;
        // This captures a CrySirror Canvas, not a legacy renderer's framebuffer.
        // Missing presenters are explicit Unsupported, never an Update ACK.
        Ok(host.begin_canvas_image(owner, 1)?)
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
            ensure!(self.pending.is_none(), "native present target changed");
            return Ok(None);
        };
        let program = m.program.clone();
        let library = &program.libraries[library];
        let f = &library.functions[function];
        if library.name != b"SACT2" || f.name != b"Update" {
            ensure!(self.pending.is_none(), "native present HLL changed");
            return Ok(None);
        }
        ensure!(owner == self.owner, "native present foreign owner");
        ensure!(
            f.return_kind == 10 && m.hll_arguments(call)?.is_empty(),
            "native Update ABI"
        );
        if self.pending.is_none() {
            let size = bridge
                .sact
                .as_ref()
                .context("native Update SACT absent")?
                .canvas;
            self.pending = Some(Pending {
                call: call.clone(),
                size,
                job: Self::begin_frame(owner, bridge, host)?,
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(pending.call == *call, "native Update stale receipt");
        match pending.job.poll()? {
            ImageProcessState::Pending => {
                Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
            }
            ImageProcessState::Failed(error) => anyhow::bail!("native scene capture: {error}"),
            ImageProcessState::Ready(resource) => {
                ensure!(
                    resource.client() == owner && resource.size() == pending.size,
                    "native scene capture owner/extent"
                );
                cg.present_window_result(owner, resource, bridge, host)?;
                m.complete_hll(call, HllReturn::Scalar(1))?;
                self.pending = None;
                Ok(Some(Dispatch::Presented))
            }
        }
    }
}
