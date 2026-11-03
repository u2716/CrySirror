// Native SACT2 scene.c background policy from xsystem4 a85467d (GPL2+).
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// All pixels, decoding, copy operations and composition belong to CrySirror.
use crate::{
    adapter::{Bridge, Dispatch},
    assets::Class,
    native_cg::NativeCg,
    vm::{
        external::{Argument, HllReturn},
        ExternalCall, ExternalTarget, Machine,
    },
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_encoding::{ImageEncodingInfoQuery, ImageEncodingInfoState},
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
        ImageResource,
    },
    path::Mount,
    service::{HostServices, ImageMetadata},
    state::ClientId,
};
use serde_json::json;
use std::sync::Arc;
const WALLPAPER: u32 = 2;
enum Phase {
    Header {
        query: Box<dyn ImageEncodingInfoQuery>,
        path: String,
    },
    Pixels {
        job: ImageJob,
        size: [u32; 2],
    },
}
struct Pending {
    call: ExternalCall,
    images: Arc<dyn ImageMetadata>,
    phase: Phase,
}
struct Published {
    path: String,
    resource: Arc<dyn ImageResource>,
}
pub(crate) struct NativeScene {
    owner: ClientId,
    pending: Option<Pending>,
    wallpaper: Option<Published>,
}
impl NativeScene {
    pub(crate) fn new(owner: ClientId) -> Self {
        Self {
            owner,
            pending: None,
            wallpaper: None,
        }
    }
    pub(crate) fn cancel_pending_for_shutdown(&mut self) {
        self.pending = None;
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        json!({"pending": self.pending.is_some(), "wallpaper": self.wallpaper.as_ref().map(|p|json!({"path":p.path,"size":p.resource.size()}))})
    }
    fn remove_wallpaper(&mut self, host: &mut dyn HostServices) -> Result<()> {
        // The backdrop is independent of sprite handles. Native SetWP(0)
        // discards only this texture, preserving the selected clear colour.
        if self.wallpaper.is_none() {
            return Ok(());
        }
        host.submit_engine(
            self.owner,
            1,
            &serde_json::to_vec(&json!({"layers":[{"op":"remove","id":WALLPAPER}]}))?,
        )?;
        if let Some(old) = self.wallpaper.take() {
            host.release_image_resource(self.owner, &old.path)?;
        }
        Ok(())
    }
    fn publish(
        &mut self,
        resource: Arc<dyn ImageResource>,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        ensure!(
            resource.client() == self.owner,
            "System4 foreign wallpaper result"
        );
        let [w, h] = resource.size();
        let published = host.publish_image_resource(self.owner, WALLPAPER, resource.clone())?;
        let path = published.path.relative().to_owned();
        let layer = json!({"layers":[{"op":"put","layer":{"id":WALLPAPER,"rect":[0,0,w,h],
            "paint":{"kind":"image","asset":path,"tint":[255,255,255,255]},
            "order":[i32::MIN,i32::MIN+1]}}]});
        if let Err(error) = host.submit_engine(self.owner, 1, &serde_json::to_vec(&layer)?) {
            host.release_image_resource(self.owner, &path)
                .context("System4 wallpaper publication rollback")?;
            return Err(error.into());
        }
        if let Some(old) = self.wallpaper.replace(Published { path, resource }) {
            host.release_image_resource(self.owner, &old.path)?;
        }
        Ok(())
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        cg: &NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending scene target"
            );
            return Ok(None);
        };
        let p = machine.program.clone();
        let library = &p.libraries[library];
        let function = &library.functions[function];
        if library.name != b"SACT2"
            || !matches!(
                function.name.as_slice(),
                b"SetWP" | b"SetWP_Color" | b"WP_SetSP"
            )
        {
            ensure!(self.pending.is_none(), "System4 changed pending scene HLL");
            return Ok(None);
        }
        ensure!(owner == self.owner, "System4 foreign scene owner");
        let sact = bridge
            .sact
            .as_ref()
            .filter(|s| s.initialized)
            .context("System4 scene before real SACT init")?;
        sact.sprites
            .as_ref()
            .context("System4 scene sprites absent")?
            .require_owner(owner)?;
        ensure!(function.return_kind == 10, "System4 SACT scene return ABI");
        let arguments = machine.hll_arguments(call)?;
        let values = arguments
            .iter()
            .map(|a| match a {
                Argument::Scalar { kind: 10, bits } => Ok(*bits),
                _ => anyhow::bail!("System4 scene scalar ABI"),
            })
            .collect::<Result<Vec<_>>>()?;
        if self.pending.is_none() {
            match (function.name.as_slice(), values.as_slice()) {
                (b"SetWP_Color", [r, g, b]) => {
                    ensure!(
                        [*r, *g, *b].iter().all(|v| (0..=255).contains(v)),
                        "System4 clear colour outside reviewed byte profile"
                    );
                    let [w, h] = sact.canvas;
                    host.submit_engine(owner,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,w,h],
                        "paint":{"kind":"solid","color":[r,g,b,255]},"order":[i32::MIN,i32::MIN]}}]}))?)?;
                    machine.complete_hll(call, HllReturn::Scalar(1))?;
                    return Ok(Some(Dispatch::Completed));
                }
                (b"SetWP", [0]) => {
                    self.remove_wallpaper(host)?;
                    machine.complete_hll(call, HllReturn::Scalar(1))?;
                    return Ok(Some(Dispatch::Completed));
                }
                (b"SetWP", [number]) => {
                    let path = bridge
                        .assets
                        .as_ref()
                        .context("System4 numbered CG catalog absent")?
                        .path(Class::Cg, *number)?;
                    let Some(path) = path else {
                        machine.complete_hll(call, HllReturn::Scalar(0))?;
                        return Ok(Some(Dispatch::Completed));
                    };
                    let images = host.image_metadata(owner)?;
                    let query = images.begin_encoding_info(&path)?;
                    ensure!(
                        query.client() == owner,
                        "System4 foreign wallpaper header query"
                    );
                    self.pending = Some(Pending {
                        call: call.clone(),
                        images,
                        phase: Phase::Header { query, path },
                    });
                }
                (b"WP_SetSP", [number]) => {
                    let Some(sprite) = sact
                        .sprites
                        .as_ref()
                        .unwrap()
                        .get(*number)
                        .filter(|s| s.surface.is_some())
                    else {
                        machine.complete_hll(call, HllReturn::Scalar(0))?;
                        return Ok(Some(Dispatch::Completed));
                    };
                    let size = sprite.size;
                    // Original scene_set_wp_texture copies RGB into a new
                    // opaque texture. Visibility/position/opacity of the
                    // source sprite do not affect the immutable snapshot.
                    let input = ImageInput::Composite {
                        base: Box::new(ImageInput::Solid {
                            size,
                            color: [0, 0, 0, 255],
                        }),
                        layers: vec![ImageCompositeLayer {
                            input: Box::new(cg.sprite_input(owner, *number, bridge)?),
                            source: [0, 0, size[0] as i32, size[1] as i32],
                            destination: [0, 0],
                            clip: None,
                            opacity: 255,
                            math: ImageCompositeMath::CopyRgb,
                        }],
                    };
                    let images = host.image_metadata(owner)?;
                    let job = images.begin_process(input, b"[]".as_slice().into())?;
                    self.pending = Some(Pending {
                        call: call.clone(),
                        images,
                        phase: Phase::Pixels { job, size },
                    });
                }
                _ => anyhow::bail!("System4 scene argument ABI"),
            }
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(pending.call == *call, "System4 stale scene receipt");
        match &mut pending.phase {
            Phase::Header { query, path } => {
                ensure!(query.client() == owner, "System4 foreign wallpaper header");
                match query.poll()? {
                    ImageEncodingInfoState::Pending => {}
                    ImageEncodingInfoState::Failed(crysirror_core::Error::NotFound) => {
                        self.pending = None;
                        machine.complete_hll(call, HllReturn::Scalar(0))?;
                        return Ok(Some(Dispatch::Completed));
                    }
                    ImageEncodingInfoState::Failed(e) => {
                        anyhow::bail!("System4 wallpaper header: {e}")
                    }
                    ImageEncodingInfoState::Ready(header) => {
                        ensure!(
                            header.client == owner
                                && header.asset.mount() == Mount::Asset
                                && header.asset.relative() == path,
                            "System4 foreign/stale wallpaper header"
                        );
                        let size = header.encoding.size;
                        let job = pending.images.begin_process(
                            ImageInput::Asset {
                                path: path.clone(),
                                size,
                                fingerprint: Some(header.fingerprint),
                            },
                            b"[]".as_slice().into(),
                        )?;
                        pending.phase = Phase::Pixels { job, size };
                    }
                }
            }
            Phase::Pixels { job, size } => match job.poll()? {
                ImageProcessState::Pending => {}
                ImageProcessState::Failed(e) => anyhow::bail!("System4 wallpaper pixels: {e}"),
                ImageProcessState::Ready(resource) => {
                    ensure!(
                        resource.client() == owner && resource.size() == *size,
                        "System4 foreign/stale wallpaper pixels"
                    );
                    self.publish(resource, host)?;
                    self.pending = None;
                    machine.complete_hll(call, HllReturn::Scalar(1))?;
                    return Ok(Some(Dispatch::Completed));
                }
            },
        }
        Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
    }
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use crate::{
        ain::{Function, HllFunction, Library, Program},
        sact::Sact,
        sprites::Sprites,
        vm::Step,
    };
    use crysirror_runtime::Runtime;
    use std::time::{Duration, Instant};
    fn runtime() -> (tempfile::TempDir, Runtime, Bridge) {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir(t.path().join("assets")).unwrap();
        let project=serde_json::from_value(json!({"schema":2,"name":"scene backgrounds","content_revision":"1","canvas":[3,2],"assets":"assets",
            "clients":[{"name":"data","adapter":"crysirror-script-v1","game":"test"}]})).unwrap();
        let r = Runtime::with_asset_root(
            project,
            &t.path().join("assets"),
            &t.path().join("state"),
            Default::default(),
        )
        .unwrap();
        // Independent native scene precondition. Original Init itself is
        // separately verified with caller-selected OS fonts, never stubbed.
        let mut sact = Sact::new([3, 2], ["g".into(), "m".into()]).unwrap();
        sact.initialized = true;
        sact.sprites = Some(Sprites::new(ClientId(1), [3, 2]).unwrap());
        let mut bridge = Bridge::default();
        bridge.sact = Some(sact);
        (t, r, bridge)
    }
    fn pending(name: &str, args: &[i32]) -> (Machine, ExternalCall) {
        let mut code = Vec::new();
        for value in args {
            code.extend(0u16.to_le_bytes());
            code.extend(value.to_le_bytes());
        }
        code.extend(0x5au16.to_le_bytes());
        code.extend(0i32.to_le_bytes());
        code.extend(0i32.to_le_bytes());
        code.extend(0x2fu16.to_le_bytes());
        let mut m = Machine::new(Arc::new(Program {
            version: 5,
            code,
            functions: vec![Function {
                name: b"main".to_vec(),
                address: 0,
                is_label: false,
                return_kind: 10,
                return_structure: -1,
                arguments: 0,
                variables: vec![],
                crc: 0,
            }],
            libraries: vec![Library {
                name: b"SACT2".to_vec(),
                functions: vec![HllFunction {
                    name: name.as_bytes().to_vec(),
                    return_kind: 10,
                    arguments: args.iter().map(|_| (vec![], 10)).collect(),
                }],
            }],
            ..Default::default()
        }))
        .unwrap();
        for _ in 0..100 {
            if let Step::External(c) = m.step().unwrap() {
                return (m, c);
            }
        }
        panic!("scene call missing")
    }
    fn advance(
        scene: &mut NativeScene,
        cg: &NativeCg,
        bridge: &Bridge,
        r: &mut Runtime,
        name: &str,
        args: &[i32],
    ) -> Machine {
        let (mut m, c) = pending(name, args);
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match scene
                .dispatch(ClientId(1), &mut m, &c, bridge, cg, &mut r.host)
                .unwrap()
            {
                Some(Dispatch::Completed) => break,
                Some(Dispatch::WaitUntil(_)) => {
                    assert_eq!(m.pc, c.pc);
                    assert_eq!(m.stack, args);
                    assert!(Instant::now() < until);
                    std::thread::yield_now()
                }
                _ => panic!("missing scene dispatch"),
            }
        }
        m
    }
    #[test]
    fn native_clear_colour_and_texture_removal_use_independent_retained_backdrop() {
        let (_t, mut r, b) = runtime();
        let mut scene = NativeScene::new(ClientId(1));
        let cg = NativeCg::default();
        let m = advance(&mut scene, &cg, &b, &mut r, "SetWP", &[0]);
        assert_eq!(m.stack, [1]);
        let m = advance(&mut scene, &cg, &b, &mut r, "SetWP_Color", &[31, 52, 73]);
        assert_eq!(m.stack, [1]);
        let layer = r.host.engine.layers.get(ClientId(1), 1).unwrap();
        assert_eq!(layer.rect, [0., 0., 3., 2.]);
        assert_eq!(layer.order, [i32::MIN, i32::MIN]);
        assert!(format!("{:?}", layer.paint).contains("31, 52, 73, 255"));
        advance(&mut scene, &cg, &b, &mut r, "SetWP", &[0]);
        assert!(r.host.engine.layers.get(ClientId(1), 1).is_some());
        assert!(r.host.engine.layers.get(ClientId(1), 2).is_none());
        let (mut m, c) = pending("SetWP_Color", &[7, 8, 256]);
        let before = serde_json::to_value(&r.host.engine).unwrap();
        assert!(scene
            .dispatch(ClientId(1), &mut m, &c, &b, &cg, &mut r.host)
            .is_err());
        assert_eq!(m.pc, c.pc);
        assert_eq!(m.stack, [7, 8, 256]);
        assert_eq!(serde_json::to_value(&r.host.engine).unwrap(), before);
        assert!(scene
            .dispatch(ClientId(2), &mut m, &c, &b, &cg, &mut r.host)
            .is_err());
    }
    #[test]
    fn native_sprite_wallpaper_is_opaque_rgb_snapshot_independent_of_source_lifetime() {
        let (_t, mut r, mut b) = runtime();
        let mut scene = NativeScene::new(ClientId(1));
        let cg = NativeCg::default();
        let sprites = b.sact.as_mut().unwrap().sprites.as_mut().unwrap();
        sprites
            .create(ClientId(1), 7, [2, 1], [20, 40, 60, 3], &mut r.host)
            .unwrap();
        sprites
            .scalar(ClientId(1), b"SP_SetPos", &[7, 200, 300], &mut r.host)
            .unwrap();
        sprites
            .scalar(ClientId(1), b"SP_SetBlendRate", &[7, 0], &mut r.host)
            .unwrap();
        let m = advance(&mut scene, &cg, &b, &mut r, "WP_SetSP", &[7]);
        assert_eq!(m.stack, [1]);
        let resource = scene.wallpaper.as_ref().unwrap().resource.clone();
        assert!(resource.opaque());
        assert_eq!(resource.rgba().as_ref(), [20, 40, 60, 255, 20, 40, 60, 255]);
        let wallpaper = r.host.engine.layers.get(ClientId(1), 2).unwrap();
        assert_eq!(wallpaper.rect, [0., 0., 2., 1.]);
        assert_eq!(wallpaper.order, [i32::MIN, i32::MIN + 1]);
        let sprites = b.sact.as_mut().unwrap().sprites.as_mut().unwrap();
        sprites
            .create(ClientId(1), 7, [1, 1], [99, 88, 77, 255], &mut r.host)
            .unwrap();
        sprites.delete(ClientId(1), 7, &mut r.host).unwrap();
        assert_eq!(resource.rgba().as_ref(), [20, 40, 60, 255, 20, 40, 60, 255]);
        advance(&mut scene, &cg, &b, &mut r, "SetWP_Color", &[4, 5, 6]);
        assert!(r.host.engine.layers.get(ClientId(1), 2).is_some());
        let m = advance(&mut scene, &cg, &b, &mut r, "WP_SetSP", &[7]);
        assert_eq!(m.stack, [0]);
        assert!(scene.wallpaper.is_some());
        advance(&mut scene, &cg, &b, &mut r, "SetWP", &[0]);
        assert!(scene.wallpaper.is_none());
        assert!(r.host.engine.layers.get(ClientId(1), 2).is_none());
        assert_eq!(resource.rgba()[0], 20);
        assert!(r.host.engine.layers.get(ClientId(2), 2).is_none());
    }
    fn numbered(root: &tempfile::TempDir, r: &mut Runtime, bridge: &mut Bridge, corrupt: bool) {
        use crysirror_core::service::MemoryAssetReader;
        use crysirror_formats_arc::ald_catalog::{AldCatalog, AldVolume};
        use crysirror_resource::{ArchiveMount, DecodeContext};
        let mut payload = b"QNT\0".to_vec();
        for value in [2u32, 68, 0, 0, 1, 1, 24, 1, 0, 10] {
            payload.extend(value.to_le_bytes());
        }
        payload.resize(68, 0);
        payload.extend([120, 156, 251, 207, 0, 0, 2, 0, 1, 0]);
        if corrupt {
            let last = payload.len() - 1;
            payload[last] ^= 7;
        }
        let mut bytes = vec![0; 768];
        for (index, offset) in [256u32, 512, 768].into_iter().enumerate() {
            bytes[index * 3..index * 3 + 3].copy_from_slice(&[
                (offset >> 8) as u8,
                (offset >> 16) as u8,
                (offset >> 24) as u8,
            ]);
        }
        bytes[256..259].copy_from_slice(&[1, 1, 0]);
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
        r.mount_provider(
            ClientId(1),
            "__cg",
            Arc::new(ArchiveMount::new(Box::new(catalog), cx).unwrap()),
        )
        .unwrap();
        let storage = r.host.storage(ClientId(1)).unwrap();
        bridge.assets = Some(Arc::new(
            crate::assets::Assets::load(
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
            .unwrap(),
        ));
    }
    #[test]
    fn native_numbered_wallpaper_waits_for_shared_header_pixels_and_preserves_previous_on_missing()
    {
        let (t, mut r, mut b) = runtime();
        numbered(&t, &mut r, &mut b, false);
        let mut scene = NativeScene::new(ClientId(1));
        let cg = NativeCg::default();
        let m = advance(&mut scene, &cg, &b, &mut r, "SetWP", &[1]);
        assert_eq!(m.stack, [1]);
        let resource = scene.wallpaper.as_ref().unwrap().resource.clone();
        assert_eq!(resource.size(), [1, 1]);
        assert_eq!(resource.rgba().as_ref(), [0, 0, 0, 255]);
        assert_eq!(
            r.host.engine.layers.get(ClientId(1), 2).unwrap().rect,
            [0., 0., 1., 1.]
        );
        let m = advance(&mut scene, &cg, &b, &mut r, "SetWP", &[2]);
        assert_eq!(m.stack, [0]);
        assert!(Arc::ptr_eq(
            &scene.wallpaper.as_ref().unwrap().resource,
            &resource
        ));
    }
    #[test]
    fn corrupt_native_wallpaper_never_acknowledges_or_replaces_retained_scene() {
        let (t, mut r, mut b) = runtime();
        numbered(&t, &mut r, &mut b, true);
        let mut scene = NativeScene::new(ClientId(1));
        let cg = NativeCg::default();
        advance(&mut scene, &cg, &b, &mut r, "SetWP_Color", &[12, 34, 56]);
        let before = serde_json::to_value(&r.host.engine).unwrap();
        let (mut m, c) = pending("SetWP", &[1]);
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match scene.dispatch(ClientId(1), &mut m, &c, &b, &cg, &mut r.host) {
                Err(_) => break,
                Ok(Some(Dispatch::WaitUntil(_))) => {
                    assert!(Instant::now() < until);
                    std::thread::yield_now()
                }
                _ => panic!("corrupt wallpaper was acknowledged"),
            }
        }
        assert_eq!(m.pc, c.pc);
        assert_eq!(m.stack, [1]);
        assert_eq!(m.step().unwrap(), Step::External(c));
        assert!(scene.wallpaper.is_none());
        assert_eq!(serde_json::to_value(&r.host.engine).unwrap(), before);
    }
}
