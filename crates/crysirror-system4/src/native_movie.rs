// Copyright (C)2019 Nunuhara Cabbage,2023 kichikuou;
// Rust (C)2026 CrySirror contributors. GPL-2.0-or-later.
//! PlayMovie native HLL policy over shared Media workers and retained Layers.
//! xsystem4 a85467d PlayMovie.c/movie_{ffmpeg,plmpeg}.c were read completely.
//! The reviewed Rance7 PlayMovie.dll (see load_movie profile) supplies the real
//! Stop and integer aspect-fit semantics missing from the public warning stub.
//! No DirectShow, original DLL, native decoder or alternate renderer is hosted.
use crate::{
    adapter::{Bridge, Dispatch},
    vm::{
        external::{Argument, HllReturn},
        ExternalCall, ExternalTarget, Machine,
    },
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AssetReader, HostServices, MediaPhase, MediaProbeJob, MediaProbeState, MediaRequest,
        MediaSourceOptions,
    },
    state::ClientId,
    Error,
};
use crysirror_tasks::{Task, TaskPool};
use serde_json::json;
use std::sync::Arc;
const MEDIA: u32 = 1023;
const LAYER: u32 = 0x53ff_0001;
struct Source {
    path: VfsPath,
    reader: Arc<dyn AssetReader>,
}
struct Loaded {
    source: Source,
    size: [u32; 2],
}
enum Preparation {
    Opening(Task<Option<Source>>),
    Probe { source: Source, job: MediaProbeJob },
}
struct Pending {
    call: ExternalCall,
    phase: Preparation,
}
pub(crate) struct NativeMovie {
    owner: ClientId,
    pool: TaskPool,
    reviewed: bool,
    initialized: bool,
    pending: Option<Pending>,
    loaded: Option<Loaded>,
    media_owned: bool,
    layer_owned: bool,
    running: bool,
}
impl NativeMovie {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Self {
        Self {
            owner,
            pool,
            reviewed: false,
            initialized: false,
            pending: None,
            loaded: None,
            media_owned: false,
            layer_owned: false,
            running: false,
        }
    }
    pub(crate) fn configure(&mut self, reviewed: bool) {
        self.reviewed = reviewed;
    }
    pub(crate) fn has_state(&self) -> bool {
        self.initialized
            || self.pending.is_some()
            || self.loaded.is_some()
            || self.media_owned
            || self.layer_owned
    }
    fn release_media(&mut self, host: &mut dyn HostServices) -> Result<()> {
        // Flags retire only after that authority was actually released. A
        // failed host call can be retried during orderly/fatal shutdown.
        if self.media_owned {
            host.media_request(self.owner, MediaRequest::Remove { id: MEDIA })?;
            self.media_owned = false;
        }
        if self.layer_owned {
            host.submit_engine(
                self.owner,
                1,
                &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":LAYER}]}))?,
            )?;
            self.layer_owned = false;
        }
        self.loaded = None;
        self.running = false;
        Ok(())
    }
    pub(crate) fn close(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.pending = None;
        self.release_media(host)?;
        self.initialized = false;
        Ok(())
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        json!({"reviewed":self.reviewed,"initialized":self.initialized,"preparing":self.pending.is_some(),"loaded":self.loaded.as_ref().map(|l|json!({"path":l.source.path.relative(),"size":l.size})),"media_owned":self.media_owned,"layer_owned":self.layer_owned,"running":self.running})
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending PlayMovie target"
            );
            return Ok(None);
        };
        let program = machine.program.clone();
        let library = &program.libraries[library];
        let function = &library.functions[function];
        if library.name != b"PlayMovie" {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending PlayMovie library"
            );
            return Ok(None);
        }
        ensure!(
            owner == self.owner && self.reviewed,
            "System4 unknown PlayMovie native module profile"
        );
        let name = function.name.as_slice();
        ensure!(
            matches!(
                name,
                b"Init" | b"Load" | b"Play" | b"Stop" | b"IsPlay" | b"Release"
            ),
            "System4 unsupported PlayMovie native export"
        );
        let args = machine.hll_arguments(call)?;
        let scalar = matches!(name, b"Init" | b"Load" | b"Play" | b"IsPlay");
        ensure!(
            function.return_kind == if scalar { 10 } else { 0 },
            "System4 PlayMovie return ABI"
        );
        ensure!(
            if name == b"Load" {
                matches!(args.as_slice(), [Argument::String { .. }])
            } else {
                args.is_empty()
            },
            "System4 PlayMovie argument ABI"
        );
        ensure!(
            !scalar || name == b"Load" || machine.stack.len() < 65536,
            "System4 PlayMovie return stack budget"
        );
        if let Some(pending) = &self.pending {
            ensure!(
                pending.call == *call && name == b"Load",
                "System4 stale PlayMovie load"
            );
        }
        let result = match name {
            b"Init" => {
                // The engine must expose a media authority; a default Host
                // Unsupported response cannot become the public source's `1`.
                match host.media_info(owner, MEDIA) {
                    Err(Error::NotFound) => {}
                    Ok(_) if self.media_owned => {}
                    Ok(_) => anyhow::bail!("System4 PlayMovie media identity collision"),
                    Err(error) => return Err(error.into()),
                }
                self.initialized = true;
                Some(1)
            }
            b"Load" => {
                ensure!(self.initialized, "System4 PlayMovie.Load before Init");
                if self.pending.is_none() {
                    let Argument::String { bytes, .. } = &args[0] else {
                        unreachable!()
                    };
                    let raw = bytes.split(|b| *b == 0).next().unwrap();
                    ensure!(
                        !raw.is_empty() && raw.len() < 260,
                        "System4 PlayMovie native filename extent"
                    );
                    let name = crysirror_resource::text_stream::decode_shift_jis(raw, 4096)?
                        .replace('\\', "/");
                    let path = VfsPath::new(Mount::Asset, &name)?;
                    let storage = host.storage(owner)?;
                    let permit = self.pool.reserve()?;
                    self.release_media(host)?;
                    self.pending=Some(Pending{call:call.clone(),phase:Preparation::Opening(permit.spawn(move|cancel|{
                        cancel.check()?;
                        let Some(path)=crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage.as_ref(),&path)? else{return Ok(None)};
                        let reader=match storage.open_asset(&path){Ok(reader)=>reader,Err(Error::NotFound|Error::Io(_))=>return Ok(None),Err(e)=>return Err(e.into())};
                        ensure!(reader.len()?<=2*1024*1024*1024,"System4 PlayMovie input byte budget");
                        cancel.check()?;Ok(Some(Source{path,reader}))
                    })?)});
                }
                let pending = self.pending.as_mut().unwrap();
                if let Preparation::Opening(task) = &mut pending.phase {
                    let Some(opened) = task.try_take()? else {
                        return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                    };
                    let Some(source) = opened else {
                        self.pending = None;
                        machine.complete_hll(call, HllReturn::Scalar(0))?;
                        return Ok(Some(Dispatch::Completed));
                    };
                    let job = host.begin_media_probe(owner, &source.path)?;
                    ensure!(job.client() == owner, "System4 foreign PlayMovie probe");
                    pending.phase = Preparation::Probe { source, job };
                }
                let Preparation::Probe { source: _, job } = &mut pending.phase else {
                    unreachable!()
                };
                match job.poll()? {
                    MediaProbeState::Pending => {
                        return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
                    }
                    MediaProbeState::Failed(error) => {
                        anyhow::bail!("System4 PlayMovie decoder preparation: {error}")
                    }
                    MediaProbeState::Ready(info) => {
                        ensure!(
                            info.client == owner
                                && info.header_validated
                                && info.size.into_iter().all(|n| n > 0 && n <= 16384),
                            "System4 invalid PlayMovie probe receipt"
                        );
                        let Pending {
                            phase: Preparation::Probe { source, .. },
                            ..
                        } = self.pending.take().unwrap()
                        else {
                            unreachable!()
                        };
                        self.loaded = Some(Loaded {
                            source,
                            size: info.size,
                        });
                        Some(1)
                    }
                }
            }
            b"Play" => {
                if !self.initialized || self.loaded.is_none() {
                    Some(0)
                } else {
                    let loaded = self.loaded.as_ref().unwrap();
                    let canvas = bridge
                        .sact
                        .as_ref()
                        .context("System4 PlayMovie missing native canvas")?
                        .canvas;
                    let rect = aspect_fit(canvas, loaded.size)?;
                    let path = if self.media_owned {
                        host.media_request(owner, MediaRequest::Resume { id: MEDIA })?
                    } else {
                        let path = host.media_reader(
                            owner,
                            MEDIA,
                            loaded.source.reader.clone(),
                            MediaSourceOptions {
                                looping: false,
                                audio: true,
                                gain: 1.,
                            },
                        )?;
                        self.media_owned = true;
                        path
                    };
                    ensure!(
                        path.mount() == Mount::Asset,
                        "System4 PlayMovie foreign surface mount"
                    );
                    let batch = json!({"layers":[{"op":"put","layer":{"id":LAYER,"rect":rect,"paint":{"kind":"image","asset":path.relative()},"order":[i32::MAX,0]}}]});
                    host.submit_engine(owner, 1, &serde_json::to_vec(&batch)?)?;
                    self.layer_owned = true;
                    self.running = true;
                    Some(1)
                }
            }
            b"Stop" => {
                // Original IMediaControl::Stop preserves current media position
                // and last picture. Shared Stop discards them, so use retained
                // clock Pause. Play continues the same resource without rewind.
                if self.media_owned {
                    host.media_request(owner, MediaRequest::Pause { id: MEDIA })?;
                }
                self.running = false;
                None
            }
            b"IsPlay" => {
                if !self.initialized {
                    Some(0)
                } else if !self.media_owned {
                    // Reviewed IMediaEvent::GetEvent reports no terminal event
                    // yet, including before playback. Do not fabricate EOF.
                    Some(1)
                } else {
                    let info = host.media_info(owner, MEDIA)?;
                    ensure!(
                        info.phase != MediaPhase::Failed,
                        "System4 PlayMovie shared decode failed"
                    );
                    if info.phase == MediaPhase::Ended {
                        self.running = false;
                        Some(0)
                    } else {
                        Some(1)
                    }
                }
            }
            b"Release" => {
                self.close(host)?;
                None
            }
            _ => unreachable!(),
        };
        machine.complete_hll(
            call,
            if let Some(value) = result {
                HllReturn::Scalar(value)
            } else {
                HllReturn::Void
            },
        )?;
        Ok(Some(Dispatch::Completed))
    }
}
fn aspect_fit(canvas: [u32; 2], video: [u32; 2]) -> Result<[u32; 4]> {
    let [w, h] = canvas;
    let [vw, vh] = video;
    ensure!(
        w > 0 && h > 0 && vw > 0 && vh > 0,
        "System4 PlayMovie empty geometry"
    );
    let scaled = u64::from(w) * u64::from(vh) / u64::from(vw);
    let (ow, oh) = if scaled > u64::from(h) {
        (u64::from(h) * u64::from(vw) / u64::from(vh), u64::from(h))
    } else {
        (u64::from(w), scaled)
    };
    ensure!(
        ow > 0 && oh > 0,
        "System4 PlayMovie collapsed integer aspect fit"
    );
    let ow = u32::try_from(ow)?;
    let oh = u32::try_from(oh)?;
    Ok([(w - ow) / 2, (h - oh) / 2, ow, oh])
}
