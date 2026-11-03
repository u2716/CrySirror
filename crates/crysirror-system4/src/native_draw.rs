// GPL-2.0-or-later. Native signatures/policy: xsystem4 a85467d DrawGraph.c,
// draw.c; (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// Loader identity/return policy checked against owner DrawPluginManager.dll,
// see SOURCE.json. No original DLL, device, renderer or pixel loop is hosted.
mod filtered_copy;
mod font_state;
mod native_text;
mod normalized;

use crate::{
    adapter::{Bridge, Dispatch},
    native_cg::NativeCg,
    vm::{
        external::{Argument, HllReturn},
        ExternalCall, ExternalTarget, Machine,
    },
};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
    },
    service::{HostServices, ImageMetadata},
    state::ClientId,
};
use std::{collections::BTreeSet, sync::Arc};
struct Pending {
    owner: ClientId,
    call: ExternalCall,
    number: i32,
    job: ImageJob,
}
pub(crate) struct NativeDraw {
    owner: ClientId,
    // Exact native C-name identity, distinct from case-insensitive module lookup.
    names: BTreeSet<Vec<u8>>,
    images: Option<Arc<dyn ImageMetadata>>,
    pending: Option<Pending>,
    native_tables: bool,
    module_profile: Option<bool>,
    text_profile: Option<bool>,
    font: font_state::FontState,
    text_run: Option<native_text::Run>,
    filtered_run: Option<filtered_copy::Run>,
}
impl NativeDraw {
    pub(crate) fn cancel_pending_for_shutdown(&mut self) {
        self.pending = None;
        self.text_run = None;
        self.filtered_run = None;
    }
    pub(crate) fn new(owner: ClientId) -> Self {
        Self {
            owner,
            names: BTreeSet::new(),
            images: None,
            pending: None,
            native_tables: false,
            module_profile: None,
            text_profile: None,
            font: Default::default(),
            text_run: None,
            filtered_run: None,
        }
    }
    pub(crate) fn configure_text_profile(&mut self, verified: bool) {
        self.text_profile = Some(verified);
    }
    pub(crate) fn configure_original_profile(&mut self, verified: bool) {
        self.module_profile = Some(verified);
        self.native_tables = verified;
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"owner":self.owner.0,"interface_bound":self.images.is_some(),
            "loaded":self.names.iter().map(|n|String::from_utf8_lossy(n)).collect::<Vec<_>>(),
            "pending":self.pending.is_some()||self.text_run.is_some()||self.filtered_run.is_some(),"font_state":{"size":self.font.size,"weight":self.font.weight,"color":self.font.color,"underline":self.font.underline,"strikeout":self.font.strikeout},"text_module_profile":self.text_profile,"font_render_profile":"portable mapped outlines; explicit disk synthetic bold, original3x coverage table; GDI hinting identity not claimed","native_table_profile":self.module_profile,"implemented_family":"RGB/alpha/RGBA copy/fill, normalized fragment/blend families and resident pixel queries; remaining exports unsupported"})
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        cg: &mut NativeCg,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.pending.is_none() && self.text_run.is_none() && self.filtered_run.is_none(),
                "System4 changed pending DrawGraph target"
            );
            return Ok(None);
        };
        let program = machine.program.clone();
        let library = &program.libraries[library];
        let function = &library.functions[function];
        if library.name != b"DrawPluginManager" && library.name != b"DrawGraph" {
            ensure!(
                self.pending.is_none() && self.text_run.is_none() && self.filtered_run.is_none(),
                "System4 changed pending DrawGraph library"
            );
            return Ok(None);
        }
        ensure!(
            owner == self.owner,
            "System4 foreign native drawing service owner"
        );
        let arguments = machine.hll_arguments(call)?;
        if library.name == b"DrawPluginManager" {
            ensure!(
                self.pending.is_none() && self.text_run.is_none() && self.filtered_run.is_none(),
                "System4 plugin changed pending draw operation"
            );
            ensure!(
                function.return_kind == 10,
                "System4 plugin manager return ABI"
            );
            let [Argument::String { bytes, .. }] = arguments.as_slice() else {
                bail!("System4 plugin manager string ABI")
            };
            let name = bytes.split(|b| *b == 0).next().unwrap();
            ensure!(name.len() <= 4096, "System4 draw plugin name budget");
            let result = match function.name.as_slice() {
                b"IsLoad" => i32::from(self.names.contains(name)),
                b"Load" if self.names.contains(name) => 0,
                b"Load" => {
                    ensure!(
                        self.module_profile != Some(false),
                        "unsupported System4 native Graph/DrawGraph module arithmetic profile"
                    );
                    let folded = name.to_ascii_lowercase();
                    ensure!(
                        matches!(folded.as_slice(), b"drawgraph" | b"drawgraph.dll"),
                        "System4 native draw plugin is not mapped: {}",
                        String::from_utf8_lossy(name)
                    );
                    ensure!(
                        program
                            .libraries
                            .iter()
                            .any(|library| library.name == b"DrawGraph"),
                        "System4 DrawGraph original library declaration absent"
                    );
                    ensure!(self.names.len() < 64, "System4 draw plugin instance budget");
                    // This installs the actual scoped engine image interface
                    // used by the handlers below, not an always-success export.
                    // Unimplemented DrawGraph exports still fail at original PC.
                    let images = host.image_metadata(owner)?;
                    self.images = Some(images);
                    self.names.insert(name.to_vec());
                    1
                }
                _ => bail!(
                    "unsupported System4 DrawPluginManager export {}",
                    String::from_utf8_lossy(&function.name)
                ),
            };
            machine.complete_hll(call, HllReturn::Scalar(result))?;
            return Ok(Some(Dispatch::Completed));
        }
        if library.name != b"DrawGraph" {
            ensure!(
                self.pending.is_none() && self.text_run.is_none() && self.filtered_run.is_none(),
                "System4 changed pending DrawGraph library"
            );
            return Ok(None);
        }
        let images = self
            .images
            .as_ref()
            .context("System4 DrawGraph interface not bound by native Load")?
            .clone();
        if self.pending.is_none() && self.text_run.is_none() && self.filtered_run.is_none() {
            if let Some(done) = self.font.dispatch(
                machine,
                call,
                &function.name,
                function.return_kind,
                &arguments,
                self.native_tables,
            )? {
                return Ok(Some(done));
            }
        }
        if filtered_copy::Run::recognizes(&function.name) {
            ensure!(
                self.pending.is_none() && self.text_run.is_none() && function.return_kind == 0,
                "native filtered copy pending/return ABI"
            );
            if self.filtered_run.is_none() {
                self.filtered_run = Some(filtered_copy::Run::begin(
                    owner,
                    call,
                    &function.name,
                    &arguments,
                    bridge,
                    cg,
                    images.as_ref(),
                )?);
            }
            let result = self.filtered_run.as_mut().unwrap().poll(
                owner,
                call,
                machine,
                bridge,
                cg,
                host,
                images.as_ref(),
            );
            if result
                .as_ref()
                .is_ok_and(|r| matches!(r, Dispatch::Completed))
            {
                self.filtered_run = None;
            }
            return result.map(Some);
        }
        ensure!(
            self.filtered_run.is_none(),
            "native filtered copy target changed while pending"
        );
        if matches!(
            function.name.as_slice(),
            b"DrawTextToPMap" | b"DrawTextToAMap"
        ) {
            ensure!(
                self.pending.is_none() && function.return_kind == 0,
                "native DrawText pending/return ABI"
            );
            ensure!(
                self.text_profile != Some(false),
                "unsupported original DrawText2 module coverage profile"
            );
            if self.text_run.is_none() {
                let fonts = host.font_library(owner)?;
                self.text_run = Some(native_text::Run::prepare(
                    owner,
                    machine,
                    call,
                    &arguments,
                    &self.font,
                    bridge,
                    cg,
                    fonts.as_ref(),
                    function.name == b"DrawTextToAMap",
                )?);
            }
            let result = self.text_run.as_mut().unwrap().poll(
                owner,
                images.as_ref(),
                host,
                bridge,
                cg,
                machine,
                call,
            );
            if result
                .as_ref()
                .is_ok_and(|r| matches!(r, Dispatch::Completed))
            {
                self.text_run = None;
            }
            return result.map(Some);
        }
        ensure!(
            self.text_run.is_none(),
            "native DrawText target changed while pending"
        );
        if self.pending.is_none() {
            if let Some(dispatch) =
                crate::native_pixel::dispatch(owner, machine, call, bridge, cg, host)?
            {
                return Ok(Some(dispatch));
            }
        }
        ensure!(
            function.return_kind == 0,
            "System4 DrawGraph void return ABI"
        );
        if self.pending.is_none() {
            let values = arguments
                .iter()
                .map(|arg| match arg {
                    Argument::Scalar { kind: 10, bits } => Ok(*bits),
                    _ => bail!("System4 DrawGraph integer argument ABI"),
                })
                .collect::<Result<Vec<_>>>()?;
            let normalized = normalized::operation(
                owner,
                &function.name,
                &values,
                bridge,
                cg,
                self.native_tables,
            )?;
            let (number, input, needs_alpha) = if let Some(operation) = normalized {
                operation
            } else {
                let (number, input) = match (function.name.as_slice(), values.as_slice()) {
                    (
                        b"Copy" | b"CopyAMap" | b"CopyWithAlphaMap",
                        [dst, dx, dy, src, sx, sy, w, h],
                    ) => {
                        ensure!(
                            (1..=8192).contains(w) && (1..=8192).contains(h),
                            "System4 DrawGraph copy dimensions budget"
                        );
                        let math = match function.name.as_slice() {
                            b"Copy" => ImageCompositeMath::CopyRgb,
                            b"CopyAMap" => ImageCompositeMath::CopyAlpha,
                            _ => ImageCompositeMath::Copy,
                        };
                        (
                            *dst,
                            composite(
                                cg.sprite_input(owner, *dst, bridge)?,
                                cg.sprite_input(owner, *src, bridge)?,
                                [*sx, *sy, *w, *h],
                                [*dx, *dy],
                                math,
                            ),
                        )
                    }
                    (b"Fill", [dst, x, y, w, h, r, g, b]) => (
                        *dst,
                        fill(
                            cg.sprite_input(owner, *dst, bridge)?,
                            [*x, *y, *w, *h],
                            [byte(*r)?, byte(*g)?, byte(*b)?, 255],
                            ImageCompositeMath::CopyRgb,
                        )?,
                    ),
                    (b"FillAMap", [dst, x, y, w, h, a]) => (
                        *dst,
                        fill(
                            cg.sprite_input(owner, *dst, bridge)?,
                            [*x, *y, *w, *h],
                            [0, 0, 0, byte(*a)?],
                            ImageCompositeMath::CopyAlpha,
                        )?,
                    ),
                    (b"FillWithAlpha", [dst, x, y, w, h, r, g, b, a]) => (
                        *dst,
                        fill(
                            cg.sprite_input(owner, *dst, bridge)?,
                            [*x, *y, *w, *h],
                            [byte(*r)?, byte(*g)?, byte(*b)?, byte(*a)?],
                            ImageCompositeMath::Copy,
                        )?,
                    ),
                    _ => bail!(
                        "unsupported original DrawGraph export/ABI {}",
                        String::from_utf8_lossy(&function.name)
                    ),
                };
                let needs_alpha = matches!(
                    function.name.as_slice(),
                    b"CopyAMap" | b"CopyWithAlphaMap" | b"FillAMap" | b"FillWithAlpha"
                );
                (number, input, needs_alpha)
            };
            if needs_alpha {
                ensure!(
                    bridge
                        .sact
                        .as_ref()
                        .and_then(|s| s.sprites.as_ref())
                        .and_then(|s| s.get(number))
                        .is_some_and(|s| s.alpha),
                    "System4 DrawGraph alpha-less destination plane policy is unsupported"
                );
            }
            self.pending = Some(Pending {
                owner,
                call: call.clone(),
                number,
                job: images.begin_process(input, b"[]".as_slice().into())?,
            });
        }
        let pending = self.pending.as_mut().unwrap();
        ensure!(
            pending.owner == owner && pending.call == *call,
            "System4 stale DrawGraph completion owner/PC"
        );
        match pending.job.poll()? {
            ImageProcessState::Pending => {
                Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
            }
            ImageProcessState::Failed(error) => {
                bail!("System4 shared DrawGraph operation: {error}")
            }
            ImageProcessState::Ready(resource) => {
                cg.publish_draw_result(owner, pending.number, resource, bridge, host)?;
                machine.complete_hll(call, HllReturn::Void)?;
                self.pending = None;
                Ok(Some(Dispatch::Completed))
            }
        }
    }
}
fn byte(value: i32) -> Result<u8> {
    u8::try_from(value).context("System4 DrawGraph color outside byte profile")
}
fn fill(
    base: ImageInput,
    [x, y, w, h]: [i32; 4],
    color: [u8; 4],
    math: ImageCompositeMath,
) -> Result<ImageInput> {
    ensure!(
        (1..=8192).contains(&w) && (1..=8192).contains(&h),
        "System4 DrawGraph fill dimensions budget"
    );
    Ok(composite(
        base,
        ImageInput::Solid {
            size: [w as u32, h as u32],
            color,
        },
        [0, 0, w, h],
        [x, y],
        math,
    ))
}
fn composite(
    base: ImageInput,
    input: ImageInput,
    source: [i32; 4],
    destination: [i32; 2],
    math: ImageCompositeMath,
) -> ImageInput {
    ImageInput::Composite {
        base: Box::new(base),
        layers: vec![ImageCompositeLayer {
            input: Box::new(input),
            source,
            destination,
            clip: None,
            opacity: 255,
            math,
        }],
    }
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use crate::{
        ain::{Function, HllFunction, Library, Program},
        vm::{heap::Object, Step},
    };
    use crysirror_runtime::Runtime;
    use std::time::{Duration, Instant};
    fn machine(library: usize, name: &str, args: &[i32], returns: i32) -> (Machine, ExternalCall) {
        let mut code = 0x5au16.to_le_bytes().to_vec();
        code.extend((library as i32).to_le_bytes());
        code.extend(0i32.to_le_bytes());
        code.extend(0x2fu16.to_le_bytes());
        let mut libraries = vec![
            Library {
                name: b"DrawPluginManager".to_vec(),
                functions: vec![],
            },
            Library {
                name: b"DrawGraph".to_vec(),
                functions: vec![],
            },
        ];
        libraries[library].functions.push(HllFunction {
            name: name.as_bytes().to_vec(),
            return_kind: returns,
            arguments: args.iter().map(|k| (vec![], *k)).collect(),
        });
        let mut m = Machine::new(Arc::new(Program {
            version: 5,
            code,
            libraries,
            functions: vec![Function {
                name: b"main".to_vec(),
                address: 0,
                is_label: false,
                return_kind: 0,
                return_structure: -1,
                arguments: 0,
                variables: vec![],
                crc: 0,
            }],
            ..Default::default()
        }))
        .unwrap();
        while !m.entered_main {
            assert_eq!(m.step().unwrap(), Step::Advanced);
        }
        // External operands are supplied by the authored caller below.
        let call = ExternalCall {
            pc: 0,
            next: 10,
            target: ExternalTarget::Hll {
                library,
                function: 0,
            },
        };
        (m, call)
    }
    fn enter(m: &mut Machine, expected: &ExternalCall) {
        assert_eq!(m.step().unwrap(), Step::External(expected.clone()));
    }
    fn runtime() -> (tempfile::TempDir, Runtime, Bridge) {
        let root = tempfile::tempdir().unwrap();
        let p = serde_json::from_value(serde_json::json!({"schema":2,"name":"native draw binding",
            "content_revision":"1","canvas":[2,1],"assets":"assets",
            "clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one"}]}))
        .unwrap();
        let mut runtime = Runtime::with_asset_root(
            p,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        // Only declare the drawing precondition. Actual font/audio/Canvas SACT
        // initialization is separately accepted by the original owner test.
        let mut s = crate::sact::Sact::new([2, 1], ["unused".into(), "unused".into()]).unwrap();
        s.initialized = true;
        s.sprites = Some(crate::sprites::Sprites::new(ClientId(1), [2, 1]).unwrap());
        for (n, color) in [(0, [10, 20, 30, 61]), (1, [111, 122, 133, 202])] {
            s.sprites
                .as_mut()
                .unwrap()
                .create(ClientId(1), n, [2, 1], color, &mut runtime.host)
                .unwrap();
        }
        let mut bridge = Bridge::default();
        bridge.sact = Some(s);
        (root, runtime, bridge)
    }
    fn manager(
        draw: &mut NativeDraw,
        cg: &mut NativeCg,
        bridge: &mut Bridge,
        runtime: &mut Runtime,
        method: &str,
        name: &[u8],
    ) -> Result<i32> {
        let (mut m, call) = machine(0, method, &[12], 10);
        let slot = m.heap.alloc(Object::String(Arc::new(name.to_vec())))?;
        m.stack.push(slot);
        enter(&mut m, &call);
        ensure!(
            draw.dispatch(ClientId(1), &mut m, &call, bridge, cg, &mut runtime.host)?
                == Some(Dispatch::Completed),
            "manager incomplete"
        );
        let result = m.stack[0];
        for _ in 0..100 {
            if m.step()? == Step::Halted {
                break;
            }
        }
        ensure!(
            m.heap.live_slots() == 1,
            "manager leaked native string/frame"
        );
        Ok(result)
    }
    fn operation(
        draw: &mut NativeDraw,
        cg: &mut NativeCg,
        bridge: &mut Bridge,
        runtime: &mut Runtime,
        name: &str,
        values: &[i32],
    ) -> Result<()> {
        let (mut m, call) = machine(1, name, &vec![10; values.len()], 0);
        m.stack = values.to_vec();
        enter(&mut m, &call);
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match draw.dispatch(ClientId(1), &mut m, &call, bridge, cg, &mut runtime.host)? {
                Some(Dispatch::Completed) => break,
                Some(Dispatch::WaitUntil(_)) => {
                    ensure!(
                        m.pc == call.pc && m.stack == values,
                        "native draw acknowledged before publication"
                    );
                    ensure!(Instant::now() < until, "native draw timeout");
                    std::thread::yield_now();
                }
                _ => bail!("draw handler missing"),
            }
        }
        for _ in 0..100 {
            if m.step()? == Step::Halted {
                break;
            }
        }
        ensure!(
            m.stack.is_empty() && m.heap.live_slots() == 1,
            "draw frame cleanup"
        );
        Ok(())
    }
    fn pixels(
        cg: &NativeCg,
        bridge: &Bridge,
    ) -> Arc<dyn crysirror_core::image_processing::ImageResource> {
        let ImageInput::Resource(resource) = cg.sprite_input(ClientId(1), 0, bridge).unwrap()
        else {
            panic!("not engine-owned published resource")
        };
        resource
    }
    #[test]
    fn native_loader_tracks_real_binding_and_exact_name_repeated_load_policy() {
        let (_root, mut rt, mut bridge) = runtime();
        let mut cg = NativeCg::default();
        let mut draw = NativeDraw::new(ClientId(1));
        assert_eq!(
            manager(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "IsLoad",
                b"DrawGraph.dll"
            )
            .unwrap(),
            0
        );
        assert_eq!(
            manager(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "Load",
                b"DrawGraph.dll\0tail"
            )
            .unwrap(),
            1
        );
        assert!(draw.images.is_some());
        assert_eq!(
            manager(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "Load",
                b"DrawGraph.dll"
            )
            .unwrap(),
            0
        );
        assert_eq!(
            manager(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "IsLoad",
                b"DRAWGRAPH.dll"
            )
            .unwrap(),
            0
        );
        assert_eq!(
            manager(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "Load",
                b"DRAWGRAPH.dll"
            )
            .unwrap(),
            1
        );
        assert_eq!(draw.names.len(), 2);
        assert!(manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"Unknown.dll"
        )
        .is_err());
        assert_eq!(draw.names.len(), 2);
    }
    #[test]
    fn copy_rgb_alpha_and_rgba_are_real_scoped_engine_operations_before_void_ack() {
        for (method, expected) in [
            ("Copy", [111, 122, 133, 61]),
            ("CopyAMap", [10, 20, 30, 202]),
            ("CopyWithAlphaMap", [111, 122, 133, 202]),
        ] {
            let (_root, mut rt, mut bridge) = runtime();
            let mut cg = NativeCg::default();
            let mut draw = NativeDraw::new(ClientId(1));
            manager(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "Load",
                b"DrawGraph.dll",
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                method,
                &[0, 1, 0, 1, 0, 0, 1, 1],
            )
            .unwrap();
            let resource = pixels(&cg, &bridge);
            assert_eq!(resource.client(), ClientId(1));
            assert_eq!(resource.size(), [2, 1]);
            assert_eq!(&resource.rgba()[..4], &[10, 20, 30, 61]);
            assert_eq!(&resource.rgba()[4..], &expected);
            assert!(rt
                .host
                .engine
                .layers
                .get(ClientId(1), 0x4100_0000)
                .is_some());
            assert!(rt
                .host
                .engine
                .layers
                .get(ClientId(2), 0x4100_0000)
                .is_none());
        }
    }
    #[test]
    fn fill_channels_preserves_previous_engine_resource_and_never_mutates_shared_pixels() {
        let (_root, mut rt, mut bridge) = runtime();
        let mut cg = NativeCg::default();
        let mut draw = NativeDraw::new(ClientId(1));
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph.dll",
        )
        .unwrap();
        operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Fill",
            &[0, 0, 0, 1, 1, 3, 4, 5],
        )
        .unwrap();
        let old = pixels(&cg, &bridge);
        assert_eq!(old.rgba().as_ref(), &[3, 4, 5, 61, 10, 20, 30, 61]);
        operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "FillAMap",
            &[0, 0, 0, 2, 1, 0],
        )
        .unwrap();
        assert_eq!(
            pixels(&cg, &bridge).rgba().as_ref(),
            &[3, 4, 5, 0, 10, 20, 30, 0]
        );
        assert_eq!(old.rgba().as_ref(), &[3, 4, 5, 61, 10, 20, 30, 61]);
        operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "FillWithAlpha",
            &[0, 1, 0, 1, 1, 7, 8, 9, 10],
        )
        .unwrap();
        assert_eq!(
            pixels(&cg, &bridge).rgba().as_ref(),
            &[3, 4, 5, 0, 7, 8, 9, 10]
        );
    }
    #[test]
    fn unbound_unknown_and_alpha_less_operations_fail_at_original_native_pc() {
        let (_root, mut rt, mut bridge) = runtime();
        let mut cg = NativeCg::default();
        let mut draw = NativeDraw::new(ClientId(1));
        assert!(operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Fill",
            &[0, 0, 0, 1, 1, 3, 4, 5]
        )
        .is_err());
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph.dll",
        )
        .unwrap();
        assert!(operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "NotImplemented",
            &[]
        )
        .is_err());
        bridge
            .sact
            .as_mut()
            .unwrap()
            .sprites
            .as_mut()
            .unwrap()
            .create(ClientId(1), 0, [2, 1], [1, 2, 3, -1], &mut rt.host)
            .unwrap();
        assert!(operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "FillAMap",
            &[0, 0, 0, 1, 1, 99]
        )
        .is_err());
        let (mut m, call) = machine(1, "Fill", &[10; 8], 0);
        m.stack = vec![0, 0, 0, 1, 1, 9, 8, 7];
        enter(&mut m, &call);
        assert!(draw
            .dispatch(
                ClientId(2),
                &mut m,
                &call,
                &mut bridge,
                &mut cg,
                &mut rt.host
            )
            .is_err());
        assert_eq!(m.pc, call.pc);
        assert_eq!(m.stack, [0, 0, 0, 1, 1, 9, 8, 7]);
        assert!(draw.pending.is_none());
    }
    fn pixel_call(
        draw: &mut NativeDraw,
        cg: &mut NativeCg,
        bridge: &mut Bridge,
        rt: &mut Runtime,
        name: &str,
        values: [i32; 3],
        outputs: usize,
    ) -> Result<(i32, Vec<i32>)> {
        use crate::vm::heap::{Page, PageKind};
        let kinds = [vec![10; 3], vec![18; outputs]].concat();
        let (mut m, call) = machine(1, name, &kinds, 10);
        let sact = name.starts_with("SP_");
        if sact {
            Arc::make_mut(&mut m.program).libraries[1].name = b"SACT2".to_vec();
        }
        let page = m.heap.alloc(Object::Page(Some(Page {
            kind: PageKind::Global,
            values: vec![77; outputs],
        })))?;
        m.stack = values.to_vec();
        for field in 0..outputs {
            m.stack.extend([page, field as i32]);
        }
        enter(&mut m, &call);
        let original = m.stack.clone();
        let result = if sact {
            cg.dispatch(ClientId(1), &mut m, &call, bridge, &mut rt.host)
        } else {
            draw.dispatch(ClientId(1), &mut m, &call, bridge, cg, &mut rt.host)
        };
        if result.is_err() {
            ensure!(
                m.pc == call.pc && m.stack == original,
                "failed pixel read advanced original call"
            );
            ensure!(
                m.heap.page(page)?.values == vec![77; outputs],
                "failed pixel read partially changed refs"
            );
        }
        ensure!(
            result? == Some(Dispatch::Completed),
            "pixel read did not complete"
        );
        Ok((m.stack[0], m.heap.page(page)?.values.clone()))
    }
    #[test]
    fn original_pixel_query_family_reads_shared_resident_bytes_and_retains_native_missing_alpha_policy(
    ) {
        let (_root, mut rt, mut bridge) = runtime();
        let mut draw = NativeDraw::new(ClientId(1));
        let mut cg = NativeCg::default();
        assert!(pixel_call(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "GetPixelColor",
            [0, 0, 0],
            3
        )
        .is_err());
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph",
        )
        .unwrap();
        // This real worker operation publishes an immutable engine resource.
        operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "CopyWithAlphaMap",
            &[0, 0, 0, 1, 0, 0, 2, 1],
        )
        .unwrap();
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "GetPixelColor",
                [0, 1, 0],
                3
            )
            .unwrap(),
            (1, vec![111, 122, 133])
        );
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "GetAlphaColor",
                [0, 1, 0],
                1
            )
            .unwrap(),
            (1, vec![202])
        );
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SP_GetPixelValue",
                [0, 0, 0],
                3
            )
            .unwrap(),
            (1, vec![111, 122, 133])
        );
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SP_GetAMapValue",
                [0, 0, 0],
                0
            )
            .unwrap(),
            (202, vec![])
        );
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SP_GetPixelValue",
                [99, 0, 0],
                3
            )
            .unwrap(),
            (0, vec![77; 3])
        );
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SP_GetAMapValue",
                [99, 0, 0],
                0
            )
            .unwrap(),
            (-1, vec![])
        );
        assert!(bridge
            .sact
            .as_ref()
            .unwrap()
            .sprites
            .as_ref()
            .unwrap()
            .get(99)
            .is_none());
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "GetAlphaColor",
                [99, 0, 0],
                1
            )
            .unwrap(),
            (0, vec![77])
        );
        assert!(bridge
            .sact
            .as_ref()
            .unwrap()
            .sprites
            .as_ref()
            .unwrap()
            .get(99)
            .is_some());
        bridge
            .sact
            .as_mut()
            .unwrap()
            .sprites
            .as_mut()
            .unwrap()
            .create(ClientId(1), 2, [1, 1], [1, 2, 3, -1], &mut rt.host)
            .unwrap();
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SP_GetAMapValue",
                [2, 0, 0],
                0
            )
            .unwrap(),
            (-1, vec![])
        );
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "GetAlphaColor",
                [2, 0, 0],
                1
            )
            .unwrap(),
            (1, vec![-1])
        );
        assert_eq!(
            pixel_call(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "GetAlphaColor",
                [0, -1, 0],
                1
            )
            .unwrap(),
            (0, vec![77])
        );
        assert!(pixel_call(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "GetPixelColor",
            [0, 2, 0],
            3
        )
        .is_err());
    }
    #[test]
    fn invalid_pixel_ref_or_foreign_owner_cannot_change_any_output_or_pending_call() {
        use crate::vm::heap::{Page, PageKind};
        let (_root, mut rt, mut bridge) = runtime();
        let mut draw = NativeDraw::new(ClientId(1));
        let mut cg = NativeCg::default();
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph",
        )
        .unwrap();
        for foreign in [false, true] {
            let (mut m, call) = machine(1, "GetPixelColor", &[10, 10, 10, 18, 18, 18], 10);
            let page = m
                .heap
                .alloc(Object::Page(Some(Page {
                    kind: PageKind::Global,
                    values: vec![77; 3],
                })))
                .unwrap();
            m.stack = vec![
                0,
                0,
                0,
                page,
                0,
                page,
                1,
                page,
                if foreign { 2 } else { 99 },
            ];
            enter(&mut m, &call);
            let original = m.stack.clone();
            assert!(draw
                .dispatch(
                    ClientId(if foreign { 2 } else { 1 }),
                    &mut m,
                    &call,
                    &mut bridge,
                    &mut cg,
                    &mut rt.host
                )
                .is_err());
            assert_eq!(m.pc, call.pc);
            assert_eq!(m.stack, original);
            assert_eq!(m.heap.page(page).unwrap().values, vec![77; 3]);
        }
    }
    #[test]
    fn normalized_original_draw_family_publishes_shared_pixels_before_void_ack() {
        let (_root, mut rt, mut bridge) = runtime();
        let mut cg = NativeCg::default();
        let mut draw = NativeDraw::new(ClientId(1));
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph",
        )
        .unwrap();
        let cases: &[(&str, &[i32], [u8; 4])] = &[
            ("CopyBright", &[128], [56, 61, 67, 61]),
            ("CopyColorReverse", &[], [144, 133, 122, 61]),
            ("CopySprite", &[111, 122, 133], [10, 20, 30, 61]),
            ("SpriteCopyAMap", &[0], [10, 20, 30, 202]),
            ("CopyUseAMapUnder", &[201], [10, 20, 30, 61]),
            ("CopyUseAMapBorder", &[202], [111, 122, 133, 61]),
            ("CopyAMapMax", &[], [10, 20, 30, 202]),
            ("CopyAMapMin", &[], [10, 20, 30, 61]),
            ("Blend", &[128], [61, 71, 82, 61]),
            ("BlendSrcBright", &[128, 64], [19, 25, 32, 61]),
            ("BlendAddSatur", &[], [37, 49, 62, 61]),
            ("BlendAMap", &[], [90, 101, 112, 61]),
            ("BlendAMapSrcOnly", &[], [98, 117, 135, 61]),
            ("BlendAMapColor", &[200, 100, 50], [161, 83, 46, 61]),
            (
                "BlendAMapColorAlpha",
                &[200, 100, 50, 128],
                [86, 52, 38, 61],
            ),
            ("BlendAMapAlpha", &[128], [50, 61, 71, 61]),
            ("BlendAMapBright", &[128], [46, 53, 59, 61]),
            ("BlendAMapAlphaSrcBright", &[128, 64], [17, 24, 31, 61]),
            ("BlendUseAMapColor", &[200, 100, 50, 192], [81, 50, 37, 61]),
            ("BlendScreen", &[], [117, 132, 147, 61]),
            ("BlendMultiply", &[], [4, 10, 16, 61]),
            ("BlendScreenAlpha", &[128], [64, 76, 89, 61]),
            ("SaturDP_DPxSA", &[], [212, 222, 232, 61]),
            ("AddDA_DAxSA", &[], [10, 20, 30, 255]),
            ("BlendDA_DAxSA", &[], [10, 20, 30, 48]),
            ("SubDA_DAxSA", &[], [10, 20, 30, 0]),
        ];
        for (name, extra, expected) in cases {
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "FillWithAlpha",
                &[0, 0, 0, 2, 1, 10, 20, 30, 61],
            )
            .unwrap();
            let previous = pixels(&cg, &bridge);
            let mut args = vec![0, 0, 0, 1, 0, 0, 1, 1];
            args.extend_from_slice(extra);
            operation(&mut draw, &mut cg, &mut bridge, &mut rt, name, &args).unwrap();
            let result = pixels(&cg, &bridge);
            assert_eq!(&result.rgba()[..4], expected, "{name}");
            assert_eq!(&result.rgba()[4..], [10, 20, 30, 61], "{name}/outside");
            assert_eq!(&previous.rgba()[..4], [10, 20, 30, 61], "{name}/immutable");
        }
        let fills: &[(&str, &[i32], [u8; 4])] = &[
            ("FillAlphaColor", &[200, 100, 50, 128], [105, 60, 40, 61]),
            ("FillScreen", &[200, 100, 50], [202, 112, 74, 61]),
            ("FillMultiply", &[200, 100, 50], [8, 8, 6, 61]),
            ("BrightDestOnly", &[128], [5, 10, 15, 61]),
            ("FillAMapOverBorder", &[128, 61], [10, 20, 30, 128]),
            ("FillAMapUnderBorder", &[128, 61], [10, 20, 30, 128]),
        ];
        for (name, extra, expected) in fills {
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "FillWithAlpha",
                &[0, 0, 0, 2, 1, 10, 20, 30, 61],
            )
            .unwrap();
            let mut args = vec![0, 1, 0, 1, 1];
            args.extend_from_slice(extra);
            operation(&mut draw, &mut cg, &mut bridge, &mut rt, name, &args).unwrap();
            let result = pixels(&cg, &bridge);
            assert_eq!(&result.rgba()[4..], expected, "{name}");
            assert_eq!(&result.rgba()[..4], [10, 20, 30, 61], "{name}/outside");
        }
        let before = pixels(&cg, &bridge);
        for (name, args) in [
            ("ScreenDA_DAxSA", vec![0, 0, 0, 1, 0, 0, 1, 1]),
            ("Blend", vec![0, 0, 0, 1, 0, 0, 1, 1, 256]),
        ] {
            let (mut machine, call) = machine(1, name, &vec![10; args.len()], 0);
            machine.stack = args.clone();
            enter(&mut machine, &call);
            assert!(draw
                .dispatch(
                    ClientId(1),
                    &mut machine,
                    &call,
                    &mut bridge,
                    &mut cg,
                    &mut rt.host
                )
                .is_err());
            assert_eq!(machine.pc, call.pc);
            assert_eq!(machine.stack, args);
            assert!(Arc::ptr_eq(&before, &pixels(&cg, &bridge)));
        }
    }
    #[test]
    fn original_scalar_graph_profile_preserves_integer_tables_and_distinguishes_screen_from_add() {
        let (_root, mut rt, mut bridge) = runtime();
        let mut cg = NativeCg::default();
        let mut draw = NativeDraw::new(ClientId(1));
        draw.configure_original_profile(false);
        assert!(manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph"
        )
        .is_err());
        draw.configure_original_profile(true);
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph",
        )
        .unwrap();
        let cases: &[(&str, &[i32], [u8; 4])] = &[
            ("CopyBright", &[128], [55, 61, 66, 61]),
            ("Blend", &[128], [59, 70, 80, 61]),
            ("BlendSrcBright", &[128, 64], [17, 24, 30, 61]),
            ("BlendAddSatur", &[], [121, 142, 163, 61]),
            ("BlendAMap", &[], [89, 100, 111, 61]),
            ("BlendAMapSrcOnly", &[], [97, 116, 135, 61]),
            ("BlendAMapColor", &[200, 100, 50], [160, 83, 45, 61]),
            (
                "BlendAMapColorAlpha",
                &[200, 100, 50, 128],
                [85, 51, 37, 61],
            ),
            ("BlendAMapAlpha", &[128], [49, 60, 70, 61]),
            ("BlendAMapBright", &[128], [45, 52, 58, 61]),
            ("BlendAMapAlphaSrcBright", &[128, 64], [16, 23, 31, 61]),
            ("BlendUseAMapColor", &[200, 100, 50, 192], [80, 49, 36, 61]),
            ("BlendScreen", &[], [117, 133, 148, 61]),
            ("BlendMultiply", &[], [4, 9, 15, 61]),
            ("BlendScreenAlpha", &[128], [63, 77, 89, 61]),
            ("ScreenDA_DAxSA", &[], [10, 20, 30, 215]),
            ("AddDA_DAxSA", &[], [10, 20, 30, 255]),
        ];
        for (name, extra, expected) in cases {
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "FillWithAlpha",
                &[0, 0, 0, 2, 1, 10, 20, 30, 61],
            )
            .unwrap();
            let mut args = vec![0, 0, 0, 1, 0, 0, 1, 1];
            args.extend_from_slice(extra);
            operation(&mut draw, &mut cg, &mut bridge, &mut rt, name, &args).unwrap();
            assert_eq!(&pixels(&cg, &bridge).rgba()[..4], expected, "{name}");
        }
        // Independent parameter edge: zero opacity cannot add source brightness.
        for native in [false, true] {
            let mut draw = NativeDraw::new(ClientId(1));
            if native {
                draw.configure_original_profile(true);
            }
            manager(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "Load",
                b"DrawGraph",
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "FillWithAlpha",
                &[0, 0, 0, 2, 1, 10, 20, 30, 61],
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "BlendSrcBright",
                &[0, 0, 0, 1, 0, 0, 1, 1, 0, 255],
            )
            .unwrap();
            assert_eq!(&pixels(&cg, &bridge).rgba()[..4], [10, 20, 30, 61]);
        }
    }
    #[test]
    fn font_controls_preserve_raw_sizes_names_and_native_exact_one_flags() {
        let (_root, mut rt, mut bridge) = runtime();
        let mut draw = NativeDraw::new(ClientId(1));
        let mut cg = NativeCg::default();
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph",
        )
        .unwrap();
        for native in [false, true] {
            draw.native_tables = native;
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SetFontSize",
                &[-33],
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SetFontWeight",
                &[1234],
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SetFontSpace",
                &[-7],
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SetFontUnderline",
                &[2],
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SetFontStrikeOut",
                &[1],
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "SetFontColor",
                &[-1, 128, 256],
            )
            .unwrap();
            assert_eq!(
                (draw.font.size, draw.font.weight, draw.font.space),
                (-33, 1234, -7)
            );
            assert_eq!(draw.font.underline, !native);
            assert!(draw.font.strikeout);
            assert_eq!(draw.font.color, [0, 128, 255]);
            let (mut m, call) = machine(1, "SetFontName", &[12], 0);
            let slot = m
                .heap
                .alloc(Object::String(Arc::new(b"custom\0ignored".to_vec())))
                .unwrap();
            m.stack.push(slot);
            enter(&mut m, &call);
            assert_eq!(
                draw.dispatch(
                    ClientId(1),
                    &mut m,
                    &call,
                    &mut bridge,
                    &mut cg,
                    &mut rt.host
                )
                .unwrap(),
                Some(Dispatch::Completed)
            );
            assert_eq!(draw.font.name, b"custom");
            for (method, want) in [
                ("GetFontSize", -33),
                ("GetFontWeight", 1234),
                ("GetFontSpace", -7),
                ("GetFontUnderline", i32::from(!native)),
                ("GetFontStrikeOut", 1),
            ] {
                let (mut m, call) = machine(1, method, &[], 10);
                enter(&mut m, &call);
                assert_eq!(
                    draw.dispatch(
                        ClientId(1),
                        &mut m,
                        &call,
                        &mut bridge,
                        &mut cg,
                        &mut rt.host
                    )
                    .unwrap(),
                    Some(Dispatch::Completed)
                );
                assert_eq!(m.stack, [want]);
            }
        }
    }

    #[test]
    #[ignore = "runtime-installed outline font; no font or game bytes committed"]
    fn original_text_calls_publish_shared_cells_once_and_preserve_pmap_amap_semantics() {
        use crysirror_core::service::{FontLoadSource, FontLoadSpec, FontLoadState};
        let (_root, mut rt, mut bridge) = runtime();
        let owner = ClientId(1);
        let font_bytes = std::fs::read(std::env::var("CRYSIRROR_TEST_FONT").unwrap()).unwrap();
        let fonts = rt.host.font_library(owner).unwrap();
        let ticket = fonts
            .begin(FontLoadSpec {
                name: "native-outline".into(),
                source: FontLoadSource::Bytes(font_bytes.into()),
                face_index: 0,
            })
            .unwrap();
        let end = Instant::now() + Duration::from_secs(10);
        let info = loop {
            match fonts.status(ticket).unwrap() {
                FontLoadState::Ready(info) => break info,
                FontLoadState::Pending => {
                    assert!(Instant::now() < end);
                    std::thread::yield_now();
                }
                other => panic!("font failed: {other:?}"),
            }
        };
        fonts.release_job(ticket).unwrap();
        assert!(fonts.face_style(&info.alias).unwrap().unwrap().weight > 0);
        bridge.sact.as_mut().unwrap().font_aliases = [info.alias.clone(), info.alias.clone()];
        bridge
            .sact
            .as_mut()
            .unwrap()
            .sprites
            .as_mut()
            .unwrap()
            .create(owner, 0, [32, 16], [10, 20, 30, 61], &mut rt.host)
            .unwrap();
        let mut cg = NativeCg::default();
        let mut draw = NativeDraw::new(owner);
        draw.configure_original_profile(true);
        draw.configure_text_profile(true);
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph",
        )
        .unwrap();
        for (name, alpha) in [("DrawTextToPMap", false), ("DrawTextToAMap", true)] {
            let (mut m, call) = machine(1, name, &[10, 10, 10, 12], 0);
            let text = m
                .heap
                .alloc(Object::String(Arc::new(b"AA A".to_vec())))
                .unwrap();
            m.stack = vec![0, 0, 0, text];
            enter(&mut m, &call);
            let old = cg.sprite_input(owner, 0, &bridge).unwrap();
            let until = Instant::now() + Duration::from_secs(10);
            let mut released = false;
            loop {
                match draw
                    .dispatch(owner, &mut m, &call, &mut bridge, &mut cg, &mut rt.host)
                    .unwrap()
                {
                    Some(Dispatch::Completed) => break,
                    Some(Dispatch::WaitUntil(_)) => {
                        assert_eq!(m.pc, call.pc);
                        assert_eq!(m.stack, vec![0, 0, 0, text]);
                        if !alpha && !released {
                            fonts.release_font(info.handle).unwrap();
                            released = true;
                        }
                        match (&old, cg.sprite_input(owner, 0, &bridge).unwrap()) {
                            (ImageInput::Resource(a), ImageInput::Resource(b)) => {
                                assert!(Arc::ptr_eq(a, &b))
                            }
                            (ImageInput::Solid { .. }, ImageInput::Solid { .. }) => {}
                            _ => panic!("partial native text publication"),
                        }
                        assert!(Instant::now() < until);
                        std::thread::yield_now();
                    }
                    other => panic!("text missing {other:?}"),
                }
            }
            let result = pixels(&cg, &bridge);
            assert_eq!(result.size(), [32, 16]);
            if !alpha {
                assert!(result.rgba().chunks_exact(4).all(|p| p[3] == 61));
                assert!(result.rgba().chunks_exact(4).any(|p| p[0] > 10));
                // Restore an authorized alias for the next native call; the
                // already published PMap image has no live font dependency.
                if released {
                    let bytes =
                        std::fs::read(std::env::var("CRYSIRROR_TEST_FONT").unwrap()).unwrap();
                    let ticket = fonts
                        .begin(FontLoadSpec {
                            name: "second".into(),
                            source: FontLoadSource::Bytes(bytes.into()),
                            face_index: 0,
                        })
                        .unwrap();
                    let face = loop {
                        match fonts.status(ticket).unwrap() {
                            FontLoadState::Ready(i) => break i,
                            FontLoadState::Pending => {
                                assert!(Instant::now() < until);
                                std::thread::yield_now();
                            }
                            other => panic!("reload {other:?}"),
                        }
                    };
                    fonts.release_job(ticket).unwrap();
                    bridge.sact.as_mut().unwrap().font_aliases = [face.alias.clone(), face.alias];
                }
            } else {
                let ImageInput::Resource(before) = old else {
                    panic!("PMap publication missing")
                };
                for (a, b) in before
                    .rgba()
                    .chunks_exact(4)
                    .zip(result.rgba().chunks_exact(4))
                {
                    assert_eq!(&a[..3], &b[..3]);
                }
                assert!(result.rgba().chunks_exact(4).any(|p| p[3] > 0));
                for y in 0..16 {
                    for x in 16..24 {
                        assert_eq!(result.rgba()[(y * 32 + x) * 4 + 3], 0);
                    }
                }
            }
        }
    }
    #[test]
    fn mirrored_subrectangles_preserve_the_native_rgb_alpha_and_whole_pixel_write_masks() {
        let (_root, mut rt, mut bridge) = runtime();
        let owner = ClientId(1);
        for number in [0, 1] {
            bridge
                .sact
                .as_mut()
                .unwrap()
                .sprites
                .as_mut()
                .unwrap()
                .create(owner, number, [3, 2], [100, 110, 120, 61], &mut rt.host)
                .unwrap();
        }
        let mut draw = NativeDraw::new(owner);
        let mut cg = NativeCg::default();
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph",
        )
        .unwrap();
        for (y, row) in [[10, 20, 30], [40, 50, 60]].into_iter().enumerate() {
            for (x, n) in row.into_iter().enumerate() {
                operation(
                    &mut draw,
                    &mut cg,
                    &mut bridge,
                    &mut rt,
                    "FillWithAlpha",
                    &[1, x as i32, y as i32, 1, 1, n, n + 1, n + 2, n + 3],
                )
                .unwrap();
            }
        }
        for method in [
            "CopyReverseLR",
            "CopyReverseUD",
            "CopyReverseAMapLR",
            "CopyReverseAMapUD",
            "CopyReverseLRWithAMap",
            "CopyReverseUDWithAMap",
        ] {
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                "FillWithAlpha",
                &[0, 0, 0, 3, 2, 100, 110, 120, 61],
            )
            .unwrap();
            operation(
                &mut draw,
                &mut cg,
                &mut bridge,
                &mut rt,
                method,
                &[0, 0, 0, 1, 1, 0, 2, 2],
            )
            .unwrap();
            let got = pixels(&cg, &bridge).rgba();
            let expected = if method.ends_with("UD") || method == "CopyReverseUDWithAMap" {
                [[50, 60], [20, 30]]
            } else {
                [[30, 20], [60, 50]]
            };
            for y in 0..2 {
                for x in 0..3 {
                    let want = if x == 2 {
                        [100, 110, 120, 61]
                    } else {
                        let n = expected[y][x];
                        if method.contains("ReverseAMap") {
                            [100, 110, 120, n + 3]
                        } else if method.ends_with("WithAMap") {
                            [n, n + 1, n + 2, n + 3]
                        } else {
                            [n, n + 1, n + 2, 61]
                        }
                    };
                    assert_eq!(
                        &got[(y * 3 + x) * 4..(y * 3 + x + 1) * 4],
                        want,
                        "{method} at{x},{y}"
                    );
                }
            }
        }
    }
    #[test]
    fn mirrored_self_copy_reads_the_frozen_source_and_invalid_clip_retains_native_pc() {
        let (_root, mut rt, mut bridge) = runtime();
        let owner = ClientId(1);
        let mut draw = NativeDraw::new(owner);
        let mut cg = NativeCg::default();
        manager(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "Load",
            b"DrawGraph",
        )
        .unwrap();
        operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "FillWithAlpha",
            &[0, 0, 0, 1, 1, 1, 2, 3, 7],
        )
        .unwrap();
        operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "FillWithAlpha",
            &[0, 1, 0, 1, 1, 4, 5, 6, 9],
        )
        .unwrap();
        let old = pixels(&cg, &bridge);
        operation(
            &mut draw,
            &mut cg,
            &mut bridge,
            &mut rt,
            "CopyReverseLR",
            &[0, 0, 0, 0, 0, 0, 2, 1],
        )
        .unwrap();
        assert_eq!(&*pixels(&cg, &bridge).rgba(), &[4, 5, 6, 7, 1, 2, 3, 9]);
        assert_eq!(&*old.rgba(), &[1, 2, 3, 7, 4, 5, 6, 9]);
        let (mut m, call) = machine(1, "CopyReverseLR", &[10; 8], 0);
        m.stack = vec![0, 0, 0, 0, -1, 0, 2, 1];
        enter(&mut m, &call);
        let before = m.stack.clone();
        let pixels_before = pixels(&cg, &bridge);
        assert!(draw
            .dispatch(owner, &mut m, &call, &mut bridge, &mut cg, &mut rt.host)
            .is_err());
        assert_eq!(m.pc, call.pc);
        assert_eq!(m.stack, before);
        assert!(Arc::ptr_eq(&pixels_before, &pixels(&cg, &bridge)));
    }
}
