// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Pinned sprite_save_cg/SACT2.c and complete native
// SACT2.B740 -> SactEngine.6890 -> EncodeCG QNT interface were read statically.
//! Only native surface/file policy. Shared Image workers and QNT encoder own
//! all pixel preparation, prediction/interleaving and compression mechanisms.
use crate::{
    adapter::{Bridge, Dispatch},
    native_cg::NativeCg,
    native_files::NativeFiles,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
    },
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    image_processing::{ImageJob, ImageProcessState},
    path::{Mount, VfsPath},
    service::{ClientStorage, HostServices},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
enum Phase {
    Pixels(ImageJob),
    Writing(Task<bool>),
}
struct Pending {
    call: ExternalCall,
    path: VfsPath,
    alpha: bool,
    storage: Arc<dyn ClientStorage>,
    phase: Phase,
}
pub(crate) struct NativeSaveCg {
    owner: ClientId,
    pool: TaskPool,
    pending: Option<Pending>,
}
impl NativeSaveCg {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Self {
        Self {
            owner,
            pool,
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
        bridge: &Bridge,
        cg: &NativeCg,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(self.pending.is_none(), "native SaveCG target changed");
            return Ok(None);
        };
        let program = m.program.clone();
        let library = &program.libraries[library];
        let f = &library.functions[function];
        if library.name != b"SACT2" || f.name != b"SP_SaveCG" {
            ensure!(self.pending.is_none(), "native SaveCG HLL changed");
            return Ok(None);
        }
        ensure!(
            owner == self.owner && f.return_kind == 10,
            "native SaveCG owner/return ABI"
        );
        let args = m.hll_arguments(call)?;
        let [
            Argument::Scalar {
                kind: 10,
                bits: number,
            },
            Argument::String { bytes, .. },
        ] = args.as_slice()
        else {
            anyhow::bail!("native SaveCG argument ABI")
        };
        if self.pending.is_none() {
            let sact = bridge
                .sact
                .as_ref()
                .filter(|s| s.initialized)
                .context("native SaveCG before Init")?;
            let Some(sprite) = sact
                .sprites
                .as_ref()
                .context("native SaveCG sprite service")?
                .get(*number)
                .filter(|s| *number >= 1 && s.surface.is_some())
            else {
                m.complete_hll(call, HllReturn::Scalar(0))?;
                return Ok(Some(Dispatch::Completed));
            };
            let (mount, path) = files.hll_path(bytes, Mount::Save)?;
            ensure!(mount == Mount::Save, "native SaveCG writable capability");
            let path = VfsPath::new(Mount::Save, &path)?;
            let storage = host.storage(owner)?;
            let images = host.image_metadata(owner)?;
            let pixels = images.begin_process(
                cg.sprite_input(owner, *number, bridge)?,
                b"[]".as_slice().into(),
            )?;
            self.pending = Some(Pending {
                call: call.clone(),
                path,
                alpha: sprite.alpha,
                storage,
                phase: Phase::Pixels(pixels),
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(pending.call == *call, "native SaveCG stale receipt");
        if let Phase::Pixels(job) = &mut pending.phase {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(Some(wait(host))),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("native SaveCG pixel preparation: {error}")
                }
                ImageProcessState::Ready(resource) => {
                    ensure!(resource.client() == owner, "native SaveCG foreign image");
                    let permit = self.pool.reserve()?;
                    let storage = pending.storage.clone();
                    let path = pending.path.clone();
                    let alpha = pending.alpha;
                    pending.phase=Phase::Writing(permit.spawn(move|cancel|{
                    cancel.check()?;let cx=crysirror_resource::DecodeContext::new(owner).with_cancellation(cancel.clone());
                    // Original native SaveCG binds QNT by GUID, regardless of
                    // filename extension. Plane presence comes from the
                    // original surface, never from normalized pixel opacity.
                    let bytes=crysirror_formats_arc::qnt::encode_rgba_with(resource.size(),&resource.rgba(),
                        crysirror_formats_arc::qnt::QntEncodeOptions{version:2,origin:[0,0],color_plane:true,alpha_plane:alpha},&cx)?;
                    cancel.check()?;
                    let path=match crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage.as_ref(),&path){
                        Ok(Some(path))=>path,Ok(None)=>path,Err(error)=>{
                            if error.downcast_ref::<Error>().is_some_and(|e|matches!(e,Error::NotFound|Error::Io(_))){return Ok(false)}
                            return Err(error)
                        }};
                    cancel.check()?;
                    match storage.write_atomic_existing_parent(&path,&bytes){
                        Ok(())=>Ok(true),Err(Error::Io(_)|Error::NotFound|Error::ReadOnly)=>Ok(false),Err(error)=>Err(error.into())}
                })?);
                }
            }
        }
        let Phase::Writing(task) = &mut pending.phase else {
            unreachable!()
        };
        let Some(ok) = task.try_take()? else {
            return Ok(Some(wait(host)));
        };
        m.complete_hll(call, HllReturn::Scalar(i32::from(ok)))?;
        self.pending = None;
        Ok(Some(Dispatch::Completed))
    }
}
fn wait(host: &dyn HostServices) -> Dispatch {
    Dispatch::WaitUntil(host.now_ms().saturating_add(1))
}
