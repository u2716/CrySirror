// Native SACT2/sprite resource policy, xsystem4 a85467d78e,
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. Parsing, decoding, immutable pixels and publication are
// shared services; no VM-private image/compositor algorithm exists here.
mod window;
use crate::{
    adapter::{Bridge, Dispatch},
    assets::Class,
    sprites::Surface,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
        heap::PageKind,
    },
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_encoding::{ImageEncodingInfoQuery, ImageEncodingInfoReceipt, ImageEncodingInfoState},
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    path::{Mount, VfsPath},
    service::{HostServices, ImageMetadata},
    state::ClientId,
};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy)]
enum Operation {
    Metrics { page: i32 },
    SetSprite { number: i32 },
}
enum Phase {
    Header(Box<dyn ImageEncodingInfoQuery>),
    Pixels {
        header: ImageEncodingInfoReceipt,
        job: ImageJob,
    },
}
struct Pending {
    owner: ClientId,
    call: ExternalCall,
    path: VfsPath,
    fallback: Option<VfsPath>,
    operation: Operation,
    metadata: Arc<dyn ImageMetadata>,
    phase: Phase,
}
#[derive(Clone)]
struct Published {
    path: String,
    _resource: Arc<dyn ImageResource>,
}
#[derive(Default)]
pub(crate) struct NativeCg {
    pending: Option<Pending>,
    images: BTreeMap<i32, Published>,
    current_window: Option<Published>,
    presented: Option<Published>,
    retired: Vec<Published>,
}
impl NativeCg {
    pub(crate) fn published_paths(&self) -> Vec<String> {
        let paths = self
            .images
            .values()
            .chain(self.current_window.iter())
            .chain(self.presented.iter())
            .chain(self.retired.iter())
            .map(|image| image.path.clone())
            .collect::<std::collections::BTreeSet<_>>();
        paths.into_iter().collect()
    }

    pub(crate) fn cancel_pending_for_shutdown(&mut self) {
        self.pending = None;
    }
    ///Native6630/66D0 call052A0 before attempting a new CG lookup. The
    ///pre-clear is not an ACK and is not delayed until a successful decode.
    pub(crate) fn sprite_clear_target(
        &self,
        m: &Machine,
        call: &ExternalCall,
    ) -> Result<Option<i32>> {
        if self.pending.is_some() {
            return Ok(None);
        }
        let ExternalTarget::Hll { library, function } = call.target else {
            return Ok(None);
        };
        let library = &m.program.libraries[library];
        let function = &library.functions[function];
        if library.name != b"SACT2"
            || !matches!(function.name.as_slice(), b"SP_SetCG" | b"SP_SetCGFromFile")
        {
            return Ok(None);
        }
        ensure!(function.return_kind == 10, "native CG preclear return ABI");
        let args = m.hll_arguments(call)?;
        let number = match (function.name.as_slice(), args.as_slice()) {
            (
                b"SP_SetCG",
                [
                    Argument::Scalar {
                        kind: 10,
                        bits: number,
                    },
                    Argument::Scalar { kind: 10, bits: id },
                ],
            ) => {
                if *number < 1 || *id < 1 {
                    return Ok(None);
                }
                *number
            }
            (
                b"SP_SetCGFromFile",
                [
                    Argument::Scalar {
                        kind: 10,
                        bits: number,
                    },
                    Argument::String { .. },
                ],
            ) => *number,
            _ => anyhow::bail!("native CG preclear argument ABI"),
        };
        Ok(Some(number))
    }
    /// Native sprite-number policy resolves only engine-owned immutable input.
    /// Shared workers perform every raster operation; no pixel buffer is kept
    /// in the adapter. Snapshots retain the exact resource across async work.
    pub(crate) fn sprite_input(
        &self,
        owner: ClientId,
        number: i32,
        bridge: &Bridge,
    ) -> Result<ImageInput> {
        validate_sprite(bridge, number)?;
        if number == -1 {
            let sprites = bridge.sact.as_ref().unwrap().sprites.as_ref().unwrap();
            sprites.require_owner(owner)?;
            if let Some(window) = &self.current_window {
                ensure!(
                    window._resource.client() == owner,
                    "native raw WindowSurface foreign lease"
                );
                return Ok(ImageInput::Resource(window._resource.clone()));
            }
            let window = sprites
                .graphics(-1)
                .context("native raw window metadata absent")?;
            let Some(Surface::Solid(color)) = window.surface else {
                anyhow::bail!("native raw WindowSurface binding absent")
            };
            return Ok(ImageInput::Solid {
                size: window.size,
                color,
            });
        }
        self.scene_sprite_input(owner, number, bridge)
    }
    ///SactEngine's actual map lookup is distinct from GetSurface7A60's raw
    ///window shortcut. In particular a FromFile/Create negative entry does
    ///not replace physical WindowSurface bytes.
    pub(crate) fn scene_sprite_input(
        &self,
        owner: ClientId,
        number: i32,
        bridge: &Bridge,
    ) -> Result<ImageInput> {
        validate_sprite(bridge, number)?;
        let sprites = bridge.sact.as_ref().unwrap().sprites.as_ref().unwrap();
        sprites.require_owner(owner)?;
        let sprite = sprites
            .get(number)
            .context("native sprite-map source absent")?;
        Ok(
            match sprite
                .surface
                .as_ref()
                .context("native sprite has no base surface")?
            {
                Surface::Solid(color) => ImageInput::Solid {
                    size: sprite.size,
                    color: *color,
                },
                Surface::Image(path) => {
                    let image = self
                        .images
                        .get(&number)
                        .filter(|p| p.path == *path)
                        .context("native sprite-map resource binding absent")?;
                    ensure!(
                        image._resource.client() == owner && image._resource.size() == sprite.size,
                        "native sprite-map stale/foreign lease"
                    );
                    ImageInput::Resource(image._resource.clone())
                }
            },
        )
    }

    pub(crate) fn publish_draw_result(
        &mut self,
        owner: ClientId,
        number: i32,
        resource: Arc<dyn ImageResource>,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        if number == -1 {
            return self.write_window_result(owner, resource, bridge, host);
        }
        self.publish_result(owner, number, resource, bridge, host, false)
    }
    fn publish_result(
        &mut self,
        owner: ClientId,
        number: i32,
        resource: Arc<dyn ImageResource>,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
        present: bool,
    ) -> Result<()> {
        validate_sprite(bridge, number)?;
        let sprites = bridge.sact.as_mut().unwrap().sprites.as_mut().unwrap();
        let sprite = sprites
            .get(number)
            .context("System4 DrawGraph destination absent")?;
        let (size, alpha) = (sprite.size, sprite.alpha);
        ensure!(
            resource.client() == owner && resource.size() == size,
            "System4 DrawGraph foreign/stale destination result"
        );
        let publication = host.publish_image_resource(
            owner,
            if number < 0 {
                16
            } else {
                0x4100_0000 + number as u32
            },
            resource.clone(),
        )?;
        let path = publication.path.relative().to_owned();
        let next_path = path.clone();
        if let Err(error) = sprites.update(owner, number, host, move |sprite| {
            sprite.size = size;
            sprite.alpha = alpha;
            sprite.surface = Some(Surface::Image(next_path));
            if present {
                sprite.visible = true;
            }
            Ok(())
        }) {
            host.release_image_resource(owner, &path)
                .context("System4 DrawGraph rollback publication")?;
            return Err(error);
        }
        if let Some(old) = self.images.insert(
            number,
            Published {
                path,
                _resource: resource,
            },
        ) {
            self.retired.push(old);
        }
        self.drain_retired(owner, host)
    }

    pub(crate) fn surface_replacement(
        machine: &Machine,
        call: &ExternalCall,
    ) -> Result<Option<i32>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            return Ok(None);
        };
        let library = &machine.program.libraries[library];
        if library.name != b"SACT2"
            || !matches!(
                library.functions[function].name.as_slice(),
                b"SP_Create" | b"SP_CreatePixelOnly" | b"SP_Delete"
            )
        {
            return Ok(None);
        }
        Ok(match machine.hll_arguments(call)?.first() {
            Some(Argument::Scalar { kind: 10, bits }) => Some(*bits),
            _ => None,
        })
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        self.dispatch_inner(owner, machine, call, bridge, None, host)
    }
    pub(crate) fn dispatch_with_files(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        files: &crate::native_files::NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        self.dispatch_inner(owner, machine, call, bridge, Some(files), host)
    }
    fn dispatch_inner(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        files: Option<&crate::native_files::NativeFiles>,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(self.pending.is_none(), "System4 changed pending CG target");
            return Ok(None);
        };
        if self.pending.is_none() {
            if let Some(dispatch) =
                crate::native_pixel::dispatch(owner, machine, call, bridge, self, host)?
            {
                return Ok(Some(dispatch));
            }
        }
        let program = machine.program.clone();
        let library = &program.libraries[library];
        let function = &library.functions[function];
        if library.name != b"SACT2"
            || !matches!(
                function.name.as_slice(),
                b"CG_GetMetrics" | b"CG_IsExist" | b"SP_SetCG" | b"SP_SetCGFromFile"
            )
        {
            ensure!(self.pending.is_none(), "System4 changed pending CG HLL");
            return Ok(None);
        }
        ensure!(function.return_kind == 10, "System4 SACT CG return ABI");
        let arguments = machine.hll_arguments(call)?;
        if self.pending.is_none() {
            let mut file_paths = None;
            let (operation, path) = match (function.name.as_slice(), arguments.as_slice()) {
                (b"CG_IsExist", [Argument::Scalar { kind: 10, bits: id }]) => {
                    let path = bridge
                        .assets
                        .as_ref()
                        .context("System4 original numbered assets absent")?
                        .path(Class::Cg, *id)?;
                    machine.complete_hll(call, HllReturn::Scalar(i32::from(path.is_some())))?;
                    return Ok(Some(Dispatch::Completed));
                }
                (
                    b"CG_GetMetrics",
                    [
                        Argument::Scalar { kind: 10, bits: id },
                        Argument::HeapReference { kind: 21, slot },
                    ],
                ) => {
                    validate_metrics(machine, *slot)?;
                    (
                        Operation::Metrics { page: *slot },
                        bridge
                            .assets
                            .as_ref()
                            .context("System4 original numbered assets absent")?
                            .path(Class::Cg, *id)?,
                    )
                }
                (
                    b"SP_SetCG",
                    [
                        Argument::Scalar {
                            kind: 10,
                            bits: number,
                        },
                        Argument::Scalar { kind: 10, bits: id },
                    ],
                ) => {
                    if *number < 1 || *id < 1 {
                        machine.complete_hll(call, HllReturn::Scalar(0))?;
                        return Ok(Some(Dispatch::Completed));
                    }
                    validate_sprite(bridge, *number)?;
                    (
                        Operation::SetSprite { number: *number },
                        bridge
                            .assets
                            .as_ref()
                            .context("System4 original numbered assets absent")?
                            .path(Class::Cg, *id)?,
                    )
                }
                (
                    b"SP_SetCGFromFile",
                    [
                        Argument::Scalar {
                            kind: 10,
                            bits: number,
                        },
                        Argument::String { bytes, .. },
                    ],
                ) => {
                    validate_sprite(bridge, *number)?;
                    let prefix = bytes.split(|b| *b == 0).next().unwrap();
                    if prefix.is_empty() {
                        //66D0's native empty filename cannot open an image;
                        //052A0 has still already cleared the existing record.
                        machine.complete_hll(call, HllReturn::Scalar(0))?;
                        return Ok(Some(Dispatch::Completed));
                    }
                    let native = encoding_rs::SHIFT_JIS
                        .decode_without_bom_handling_and_without_replacement(prefix)
                        .context("System4 CG filename encoding")?;
                    let path = VfsPath::new(Mount::Asset, &native.replace('\\', "/"))?;
                    if let Some(files) = files {
                        let (mount, relative) = files.hll_path(bytes, Mount::Asset)?;
                        file_paths = Some(if mount == Mount::Save {
                            files.save_path(bytes)?
                        } else {
                            (
                                VfsPath::new(Mount::Save, &relative)?,
                                VfsPath::new(Mount::Asset, &relative)?,
                            )
                        });
                    }
                    (
                        Operation::SetSprite { number: *number },
                        Some(path.relative().to_owned()),
                    )
                }
                _ => anyhow::bail!("System4 native CG argument ABI"),
            };
            let Some(path) = path else {
                machine.complete_hll(call, HllReturn::Scalar(0))?;
                return Ok(Some(Dispatch::Completed));
            };
            let metadata = host.image_metadata(owner)?;
            // Native files written into the isolated writable root shadow
            // their original loose-file counterpart. Numbered archive CGs
            // keep their own resource lookup policy.
            let (path, fallback) = file_paths.map_or(
                (VfsPath::new(Mount::Asset, &path)?, None),
                |(saved, original)| (saved, Some(original)),
            );
            let query = metadata.begin_scoped_encoding_info(&path)?;
            ensure!(
                query.client() == owner,
                "System4 foreign encoded image query"
            );
            self.pending = Some(Pending {
                owner,
                call: call.clone(),
                path,
                fallback,
                operation,
                metadata,
                phase: Phase::Header(query),
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(
            pending.owner == owner && pending.call == *call,
            "System4 stale native CG dependency"
        );
        match &mut pending.phase {
            Phase::Header(query) => {
                ensure!(
                    query.client() == owner,
                    "System4 foreign encoded image query"
                );
                match query.poll()? {
                    ImageEncodingInfoState::Pending => {}
                    ImageEncodingInfoState::Failed(crysirror_core::Error::NotFound) => {
                        if let Some(original) = pending.fallback.take() {
                            let query = pending.metadata.begin_scoped_encoding_info(&original)?;
                            ensure!(
                                query.client() == owner,
                                "System4 foreign fallback CG header"
                            );
                            pending.path = original;
                            pending.phase = Phase::Header(query);
                            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                        }
                        //Original lookup failure returns0. Metrics remain
                        //unchanged; SetCG's earlier052A0 already cleared its
                        //old base and retained glyphs before this lookup.
                        machine.complete_hll(call, HllReturn::Scalar(0))?;
                        self.pending = None;
                        return Ok(Some(Dispatch::Completed));
                    }
                    ImageEncodingInfoState::Failed(error) => {
                        anyhow::bail!("System4 shared CG header: {error}")
                    }
                    ImageEncodingInfoState::Ready(header) => {
                        ensure!(
                            header.client == owner
                                && matches!(header.asset.mount(), Mount::Asset | Mount::Save)
                                && header.asset == pending.path,
                            "System4 foreign/stale encoded image receipt"
                        );
                        match pending.operation {
                            Operation::Metrics { page } => {
                                validate_metrics(machine, page)?;
                                let info = &header.encoding;
                                let values = [
                                    i32::try_from(info.size[0])?,
                                    i32::try_from(info.size[1])?,
                                    i32::from(
                                        info.color_bits.context("CG color depth unavailable")?,
                                    ),
                                    i32::from(
                                        info.has_color_plane
                                            .context("CG color-plane declaration unavailable")?,
                                    ),
                                    i32::from(
                                        info.has_alpha_plane
                                            .context("CG alpha-plane declaration unavailable")?,
                                    ),
                                    i32::try_from(
                                        info.color_row_bytes
                                            .context("CG color row pitch unavailable")?,
                                    )?,
                                    i32::from(
                                        info.alpha_sample_bytes
                                            .context("CG alpha sample pitch unavailable")?,
                                    ),
                                ];
                                for (index, value) in values.into_iter().enumerate() {
                                    machine.heap.set(page, index as i32, value)?;
                                }
                                machine.complete_hll(call, HllReturn::Scalar(1))?;
                                self.pending = None;
                                return Ok(Some(Dispatch::Completed));
                            }
                            Operation::SetSprite { .. } => {
                                header
                                    .encoding
                                    .has_alpha_plane
                                    .context("CG alpha-plane declaration unavailable")?;
                                let input = if pending.path.mount() == Mount::Asset {
                                    ImageInput::Asset {
                                        path: pending.path.relative().to_owned(),
                                        size: header.encoding.size,
                                        fingerprint: Some(header.fingerprint),
                                    }
                                } else {
                                    ImageInput::ScopedAsset {
                                        path: pending.path.clone(),
                                        size: header.encoding.size,
                                        fingerprint: Some(header.fingerprint),
                                    }
                                };
                                let job = pending
                                    .metadata
                                    .begin_process(input, b"[]".as_slice().into())?;
                                pending.phase = Phase::Pixels { header, job };
                            }
                        }
                    }
                }
            }
            Phase::Pixels { header, job } => match job.poll()? {
                ImageProcessState::Pending => {}
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("System4 shared CG preparation: {error}")
                }
                ImageProcessState::Ready(resource) => {
                    ensure!(
                        resource.client() == owner && resource.size() == header.encoding.size,
                        "System4 foreign/stale prepared image"
                    );
                    let Operation::SetSprite { number } = pending.operation else {
                        unreachable!()
                    };
                    let published = host.publish_image_resource(
                        owner,
                        if number < 0 {
                            16
                        } else {
                            0x4100_0000 + number as u32
                        },
                        resource.clone(),
                    )?;
                    let path = published.path.relative().to_owned();
                    let sprites = bridge
                        .sact
                        .as_mut()
                        .and_then(|s| s.sprites.as_mut())
                        .context("System4 CG sprite service absent")?;
                    if let Err(error) = sprites.image(
                        owner,
                        number,
                        path.clone(),
                        header.encoding.size,
                        header.encoding.has_alpha_plane.unwrap(),
                        host,
                    ) {
                        host.release_image_resource(owner, &path)
                            .context("System4 rollback prepared CG publication")?;
                        return Err(error);
                    }
                    if let Some(old) = self.images.insert(
                        number,
                        Published {
                            path,
                            _resource: resource,
                        },
                    ) {
                        self.retired.push(old);
                    }
                    self.drain_retired(owner, host)?;
                    machine.complete_hll(call, HllReturn::Scalar(1))?;
                    self.pending = None;
                    return Ok(Some(Dispatch::Completed));
                }
            },
        }
        Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
    }
    /// Retire only our immutable publications when native delete/create changes
    /// the sprite. Frozen frame retention/reclamation remain shared services.
    pub(crate) fn reconcile(
        &mut self,
        owner: ClientId,
        number: i32,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        let sprites = bridge.sact.as_ref().and_then(|s| s.sprites.as_ref());
        if self.images.get(&number).is_some_and(|image| {
            !sprites.and_then(|s| s.get(number)).is_some_and(
                |sprite| matches!(&sprite.surface,Some(Surface::Image(path))if path==&image.path),
            )
        }) {
            self.retired.push(self.images.remove(&number).unwrap());
        }
        self.drain_retired(owner, host)?;
        Ok(())
    }
}
fn validate_sprite(bridge: &Bridge, number: i32) -> Result<()> {
    ensure!(
        (-1..65536).contains(&number),
        "System4 CG sprite index budget"
    );
    ensure!(
        bridge
            .sact
            .as_ref()
            .is_some_and(|s| s.initialized && s.sprites.is_some()),
        "System4 CG before real SACT init"
    );
    Ok(())
}
fn validate_metrics(machine: &Machine, slot: i32) -> Result<()> {
    let page = machine.heap.page(slot)?;
    let PageKind::Structure(index) = page.kind else {
        anyhow::bail!("System4 CG metrics expected original structure")
    };
    let members = &machine.program.structures[index].members;
    ensure!(
        page.values.len() >= 7 && members.len() >= 7,
        "System4 CG metrics structure length"
    );
    for (index, member) in members[..7].iter().enumerate() {
        ensure!(
            member.kind == 10 || ((index == 3 || index == 4) && member.kind == 47),
            "System4 CG metrics member type"
        );
    }
    Ok(())
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use crate::{
        ain::{Function, HllFunction, Library, Program, Structure, Variable},
        vm::Step,
    };
    use crysirror_core::service::MemoryAssetReader;
    use crysirror_formats_arc::ald_catalog::{AldCatalog, AldVolume};
    use crysirror_resource::{ArchiveMount, DecodeContext};
    use crysirror_runtime::Runtime;
    use std::time::{Duration, Instant};
    fn qnt() -> Vec<u8> {
        let mut bytes = b"QNT\0".to_vec();
        for value in [2u32, 68, 0, 0, 1, 1, 24, 1, 0, 10] {
            bytes.extend(value.to_le_bytes())
        }
        bytes.resize(68, 0);
        bytes.extend([120, 156, 251, 207, 0, 0, 2, 0, 1, 0]);
        bytes
    }
    fn runtime() -> (tempfile::TempDir, Runtime) {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("assets")).unwrap();
        let project = root.path().join("project.json");
        std::fs::write(&project,br#"{"schema":2,"name":"native CG","content_revision":"1","canvas":[1,1],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one"}]}"#).unwrap();
        let runtime = Runtime::load(&project, &root.path().join("state")).unwrap();
        (root, runtime)
    }
    fn op(code: &mut Vec<u8>, opcode: u16, args: &[i32]) {
        code.extend(opcode.to_le_bytes());
        for a in args {
            code.extend(a.to_le_bytes())
        }
    }
    fn program(name: &str, args: &[i32], metrics: bool) -> Program {
        let mut program = Program {
            version: 5,
            ..Default::default()
        };
        if metrics {
            op(&mut program.code, 0, &[1]);
            op(&mut program.code, 0x5d, &[0]);
            program.globals = vec![Variable {
                name: vec![],
                kind: 13,
                structure: 0,
                rank: 0,
                group: None,
            }];
            program.structures = vec![Structure {
                name: b"metrics".to_vec(),
                constructor: -1,
                destructor: -1,
                members: (0..7)
                    .map(|_| Variable {
                        name: vec![],
                        kind: 10,
                        structure: -1,
                        rank: 0,
                        group: None,
                    })
                    .collect(),
            }];
        } else {
            op(&mut program.code, 0, &[0]);
            op(&mut program.code, 0x41, &[0]);
            program.strings = vec![b"picture.qnt".to_vec()];
        }
        op(&mut program.code, 0x5a, &[0, 0]);
        op(&mut program.code, 0x2f, &[]);
        program.functions = vec![Function {
            address: 0,
            name: b"main".to_vec(),
            is_label: false,
            return_kind: 10,
            return_structure: -1,
            arguments: 0,
            crc: 0,
            variables: vec![],
        }];
        program.libraries = vec![Library {
            name: b"SACT2".to_vec(),
            functions: vec![HllFunction {
                name: name.as_bytes().to_vec(),
                return_kind: 10,
                arguments: args.iter().map(|k| (vec![], *k)).collect(),
            }],
        }];
        program
    }
    fn pending(program: Program) -> (Machine, ExternalCall) {
        let mut machine = Machine::new(Arc::new(program)).unwrap();
        for _ in 0..100 {
            if let Step::External(call) = machine.step().unwrap() {
                return (machine, call);
            }
        }
        panic!("missing original CG call")
    }
    fn advance(
        cg: &mut NativeCg,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        runtime: &mut Runtime,
    ) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match cg.dispatch(ClientId(1), machine, call, bridge, &mut runtime.host)? {
                Some(Dispatch::Completed) => return Ok(()),
                Some(Dispatch::WaitUntil(_)) => {
                    ensure!(Instant::now() < deadline, "CG task timeout");
                    ensure!(
                        machine.pc == call.pc,
                        "CG native PC advanced before real resource publication"
                    );
                    std::thread::yield_now()
                }
                _ => anyhow::bail!("wrong CG dependency"),
            }
        }
    }
    fn bridge_with_sprite_fixture() -> Bridge {
        // Logical precondition for CG tests only. Actual SACT font/audio Init
        // acceptance is separately verified; no font API is called here.
        let mut sact = crate::sact::Sact::new([1, 1], ["unused".into(), "unused".into()]).unwrap();
        sact.initialized = true;
        sact.sprites = Some(crate::sprites::Sprites::new(ClientId(1), [1, 1]).unwrap());
        let mut bridge = Bridge::default();
        bridge.sact = Some(sact);
        bridge
    }
    #[test]
    fn original_set_cg_waits_for_shared_pixels_and_preserves_declared_opaque_alpha() {
        let (root, mut runtime) = runtime();
        std::fs::write(root.path().join("assets/picture.qnt"), qnt()).unwrap();
        let (mut machine, call) = pending(program("SP_SetCGFromFile", &[10, 12], false));
        let mut bridge = bridge_with_sprite_fixture();
        let mut cg = NativeCg::default();
        advance(&mut cg, &mut machine, &call, &mut bridge, &mut runtime).unwrap();
        let sprite = bridge
            .sact
            .as_ref()
            .unwrap()
            .sprites
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap();
        assert!(sprite.alpha);
        assert_eq!(sprite.size, [1, 1]);
        let layer = runtime
            .host
            .engine
            .layers
            .get(ClientId(1), 0x4100_0000)
            .unwrap();
        assert_eq!(layer.rect, [0.0, 0.0, 1.0, 1.0]);
        assert!(
            runtime
                .host
                .engine
                .layers
                .get(ClientId(2), 0x4100_0000)
                .is_none()
        );
        assert!(cg.images[&0]._resource.opaque());
        assert_eq!(cg.images[&0]._resource.rgba().as_ref(), [0, 0, 0, 255]);
        for _ in 0..100 {
            if machine.step().unwrap() == Step::Halted {
                break;
            }
        }
        assert_eq!(machine.stack, [1]);
        bridge
            .sact
            .as_mut()
            .unwrap()
            .sprites
            .as_mut()
            .unwrap()
            .delete(ClientId(1), 0, &mut runtime.host)
            .unwrap();
        cg.reconcile(ClientId(1), 0, &bridge, &mut runtime.host)
            .unwrap();
        assert!(cg.images.is_empty());
    }
    #[test]
    fn corrupt_original_image_payload_retains_call_without_layer_or_success() {
        let (root, mut runtime) = runtime();
        let mut bytes = qnt();
        let last = bytes.len() - 1;
        bytes[last] ^= 7;
        std::fs::write(root.path().join("assets/picture.qnt"), bytes).unwrap();
        let (mut machine, call) = pending(program("SP_SetCGFromFile", &[10, 12], false));
        let stack = machine.stack.clone();
        let mut bridge = bridge_with_sprite_fixture();
        let mut cg = NativeCg::default();
        assert!(advance(&mut cg, &mut machine, &call, &mut bridge, &mut runtime).is_err());
        assert_eq!(machine.pc, call.pc);
        assert_eq!(machine.stack, stack);
        assert_eq!(machine.step().unwrap(), Step::External(call));
        assert!(
            runtime
                .host
                .engine
                .layers
                .get(ClientId(1), 0x4100_0000)
                .is_none()
        );
        assert!(cg.images.is_empty());
    }
    #[test]
    fn missing_original_file_returns_native_zero_without_parsing_error_strings() {
        let (_root, mut runtime) = runtime();
        let (mut machine, call) = pending(program("SP_SetCGFromFile", &[10, 12], false));
        let mut bridge = bridge_with_sprite_fixture();
        let mut cg = NativeCg::default();
        advance(&mut cg, &mut machine, &call, &mut bridge, &mut runtime).unwrap();
        assert!(
            bridge
                .sact
                .as_ref()
                .unwrap()
                .sprites
                .as_ref()
                .unwrap()
                .get(0)
                .is_none()
        );
        for _ in 0..100 {
            if machine.step().unwrap() == Step::Halted {
                break;
            }
        }
        assert_eq!(machine.stack, [0]);
        assert!(cg.images.is_empty());
        assert!(
            runtime
                .host
                .engine
                .layers
                .get(ClientId(1), 0x4100_0000)
                .is_none()
        );
    }
    #[test]
    fn original_numbered_cg_metrics_use_shared_ald_header_query_and_native_fields() {
        let (root, mut runtime) = runtime();
        let mut bytes = vec![0; 768];
        for (index, offset) in [256u32, 512, 768].into_iter().enumerate() {
            bytes[index * 3..index * 3 + 3].copy_from_slice(&[
                (offset >> 8) as u8,
                (offset >> 16) as u8,
                (offset >> 24) as u8,
            ]);
        }
        bytes[256..259].copy_from_slice(&[1, 1, 0]);
        let payload = qnt();
        bytes[512..516].copy_from_slice(&32u32.to_le_bytes());
        bytes[516..520].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes[528..539].copy_from_slice(b"picture.qnt");
        bytes[544..544 + payload.len()].copy_from_slice(&payload);
        std::fs::write(root.path().join("assets/gameGA.ald"), &bytes).unwrap();
        let cx = DecodeContext::new(ClientId(1));
        let catalog = AldCatalog::load(
            vec![AldVolume {
                number: 0,
                source: Arc::new(MemoryAssetReader(bytes.into())),
            }],
            &cx,
        )
        .unwrap();
        runtime
            .mount_provider(
                ClientId(1),
                "__cg",
                Arc::new(ArchiveMount::new(Box::new(catalog), cx).unwrap()),
            )
            .unwrap();
        let storage = runtime.host.storage(ClientId(1)).unwrap();
        let assets = crate::assets::Assets::load(
            ClientId(1),
            storage.as_ref(),
            &[crate::assets::Volume {
                class: Class::Cg,
                volume: 0,
                source: "gameGA.ald".into(),
                mount: "__cg".into(),
            }],
            Default::default(),
        )
        .unwrap();
        let (mut machine, call) = pending(program("CG_GetMetrics", &[10, 21], true));
        let slot = machine.heap.get(0, 0).unwrap();
        let sequence = machine.heap.slot(slot).unwrap().sequence;
        let mut bridge = Bridge::default();
        bridge.assets = Some(Arc::new(assets));
        let mut cg = NativeCg::default();
        advance(&mut cg, &mut machine, &call, &mut bridge, &mut runtime).unwrap();
        assert_eq!(
            machine.heap.page(slot).unwrap().values,
            [1, 1, 24, 0, 1, 3, 1]
        );
        assert_eq!(machine.heap.slot(slot).unwrap().sequence, sequence);
        assert!(cg.images.is_empty());
        assert!(
            runtime
                .host
                .engine
                .layers
                .get(ClientId(1), 0x4100_0000)
                .is_none()
        );
    }
}
