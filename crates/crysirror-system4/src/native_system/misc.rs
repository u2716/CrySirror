// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Original Rance7 SACT2 E290/B1C0 (log + newline, return0), 112F0
//! (debugger pause) and 11310/C0E0 (function stack, trailing81A9).
//! SystemService1390 registers URL/title pairs through MainSystem; System40
//! 41DCA0 installs its Help/site entries and420AE0 opens a selected URL.
//! Shared engine logs, retained menus and async desktop requests own output.
use super::*;
use std::collections::VecDeque;
const BAR: u32 = 21316;
const FIRST_URL: u32 = 40026;
const PAUSE: u32 = 21317;
#[derive(Default)]
pub(super) struct Misc {
    urls: Vec<(String, String)>,
    opens: VecDeque<String>,
    opening: Option<Box<dyn DesktopRequestJob>>,
    pause: Option<ExternalCall>,
}
impl Misc {
    pub fn pending(&self) -> bool {
        self.opening.is_some() || !self.opens.is_empty() || self.pause.is_some()
    }
    pub fn cancel(&mut self) {
        self.opening = None;
        self.opens.clear();
    }
    pub fn close(&mut self, owner: ClientId, host: &mut dyn HostServices) -> Result<()> {
        let mut ui = vec![];
        if self.pause.is_some() {
            ui.push(json!({"op":"close_dialog","id":PAUSE}));
        }
        if !self.urls.is_empty() {
            ui.push(json!({"op":"remove_menu","id":BAR}));
        }
        if !ui.is_empty() {
            host.submit_engine(owner, 1, &serde_json::to_vec(&json!({"ui":ui}))?)?;
        }
        self.pause = None;
        self.urls.clear();
        self.cancel();
        Ok(())
    }
    pub fn poll_menu(&mut self, owner: ClientId, host: &mut dyn HostServices) -> Result<()> {
        if self.urls.is_empty() {
            return Ok(());
        }
        let events: Vec<serde_json::Value> = serde_json::from_slice(
            &host.query_engine(owner, crysirror_core::service::EngineQuery::MenuEvents)?,
        )?;
        for event in events {
            ensure!(
                event["menu_bar"].as_u64() == Some(u64::from(BAR)),
                "System4 unknown native menu bar activation"
            );
            let id = event["widget"]["id"]
                .as_u64()
                .context("System4 URL menu item absent")?;
            let index = id
                .checked_sub(u64::from(FIRST_URL))
                .and_then(|n| usize::try_from(n).ok())
                .context("System4 URL menu item bounds")?;
            let (_, url) = self
                .urls
                .get(index)
                .context("System4 stale URL menu activation")?;
            ensure!(self.opens.len() < 32, "System4 URL opening queue budget");
            self.opens.push_back(url.clone());
        }
        if self.opening.is_none() {
            if let Some(url) = self.opens.front() {
                self.opening =
                    Some(host.begin_desktop_request(
                        owner,
                        DesktopRequest::OpenUrl { url: url.clone() },
                    )?);
                self.opens.pop_front();
            }
        }
        if let Some(job) = self.opening.as_mut() {
            match job.poll()? {
                DesktopRequestState::Pending => (),
                DesktopRequestState::Ready(DesktopResponse::Opened) => self.opening = None,
                DesktopRequestState::Ready(_) => {
                    anyhow::bail!("System4 URL opening returned a different desktop response")
                }
            }
        }
        Ok(())
    }
    pub fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            return Ok(None);
        };
        let program = m.program.clone();
        let lib = &program.libraries[library];
        let function = &lib.functions[function];
        let name = function.name.as_slice();
        if !((lib.name == b"SACT2"
            && matches!(name, b"Error" | b"Debug_Pause" | b"Debug_GetFuncStack"))
            || (lib.name == b"SystemService" && name == b"AddURLMenu"))
        {
            return Ok(None);
        }
        let args = m.hll_arguments(call)?;
        if name == b"Debug_Pause" {
            ensure!(
                function.return_kind == 0 && args.is_empty(),
                "SACT Debug_Pause ABI"
            );
            if let Some(pending) = &self.pause {
                ensure!(*pending == *call, "SACT changed debugger pause receipt");
                let events: Vec<serde_json::Value> = serde_json::from_slice(
                    &host
                        .query_engine(owner, crysirror_core::service::EngineQuery::DialogEvents)?,
                )?;
                for event in events {
                    ensure!(
                        event["widget"]["owner"].as_u64() == Some(u64::from(owner.0))
                            && event["widget"]["id"].as_u64() == Some(u64::from(PAUSE))
                            && event["kind"] == "dialog_result"
                            && event["value"].as_f64() == Some(1.),
                        "SACT foreign debugger response"
                    );
                    m.complete_hll(call, HllReturn::Void)?;
                    self.pause = None;
                    return Ok(Some(Dispatch::Completed));
                }
            } else {
                let font = bridge
                    .sact
                    .as_ref()
                    .map(|s| s.font_aliases[0].as_str())
                    .unwrap_or("default");
                // Bind the native debugger run-control to the engine's modal
                // UI. The VM stays at the original receipt until Resume; no
                // host thread blocks and no foreign debugger code is loaded.
                host.submit_engine(owner,1,&serde_json::to_vec(&json!({"ui":[{"op":"open_dialog","dialog":{"id":PAUSE,"title":"System4 debugger","message":"Script execution is paused.","font":font,"buttons":[{"id":1,"label":"Resume"}],"default":1,"cancel":1}}]}))?)?;
                self.pause = Some(call.clone());
            }
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        }
        let mut candidate = m.clone();
        let mut log = None;
        let mut install = None;
        let result = match name {
            b"Error" => {
                ensure!(function.return_kind == 10, "SACT Error return ABI");
                let [Argument::String { bytes, .. }] = args.as_slice() else {
                    anyhow::bail!("SACT Error string ABI")
                };
                let mut text =
                    crysirror_resource::text_stream::decode_shift_jis(c_prefix(bytes), 1 << 20)?;
                text.push('\n');
                log = Some(text);
                HllReturn::Scalar(0)
            }
            b"Debug_GetFuncStack" => {
                ensure!(
                    function.return_kind == 0,
                    "SACT Debug_GetFuncStack return ABI"
                );
                let [Argument::HeapReference { kind: 20, slot }, Argument::Scalar {
                    kind: 10,
                    bits: nest,
                }] = args.as_slice()
                else {
                    anyhow::bail!("SACT Debug_GetFuncStack ABI")
                };
                m.heap.string(*slot)?;
                let bytes = function_stack(m, *nest)?;
                candidate.heap.set_string(*slot, Arc::new(bytes))?;
                HllReturn::Void
            }
            b"AddURLMenu" => {
                ensure!(
                    function.return_kind == 47,
                    "SystemService AddURLMenu return ABI"
                );
                let [Argument::String { bytes: title, .. }, Argument::String { bytes: url, .. }] =
                    args.as_slice()
                else {
                    anyhow::bail!("SystemService AddURLMenu string ABI")
                };
                let title =
                    crysirror_resource::text_stream::decode_shift_jis(c_prefix(title), 4096)?;
                let url = crysirror_resource::text_stream::decode_shift_jis(c_prefix(url), 8192)?;
                DesktopRequest::OpenUrl { url: url.clone() }.validate()?;
                ensure!(self.urls.len() < 64, "System4 URL menu budget");
                let mut urls = self.urls.clone();
                urls.push((title, url));
                let items = urls
                    .iter()
                    .enumerate()
                    .map(|(i, (title, _))| json!({"id":FIRST_URL+i as u32,"label":title}))
                    .collect::<Vec<_>>();
                let font = bridge
                    .sact
                    .as_ref()
                    .map(|s| s.font_aliases[0].as_str())
                    .unwrap_or("default");
                let width = bridge.sact.as_ref().map(|s| s.canvas[0]).unwrap_or(800);
                let menu = json!({"id":BAR,"rect":[0,0,width,24],"font":font,"items":[{"id":1,"label":"ヘルプ","children":[{"id":2,"label":"Webサイト","children":items}]}]});
                // Installation never opens a viewer. Only retained engine menu
                // activation admits the scoped asynchronous OpenUrl request.
                install = Some((menu, urls));
                HllReturn::Scalar(1)
            }
            _ => unreachable!(),
        };
        candidate.complete_hll(call, result)?;
        if let Some(text) = log {
            let receipt = host.append_log(owner, &text)?;
            ensure!(receipt.client == owner, "SACT foreign log receipt");
        }
        if let Some((menu, urls)) = install {
            host.submit_engine(
                owner,
                1,
                &serde_json::to_vec(&json!({"ui":[{"op":"set_menu","menu":menu}]}))?,
            )?;
            self.urls = urls;
        }
        *m = candidate;
        Ok(Some(Dispatch::Completed))
    }
}
fn c_prefix(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())]
}
fn function_stack(m: &Machine, nest: i32) -> Result<Vec<u8>> {
    let count = if nest < 0 {
        m.frames.len()
    } else {
        m.frames.len().min(usize::try_from(nest)?.saturating_add(1))
    };
    let mut out = Vec::new();
    for frame in m.frames.iter().rev().take(count) {
        let name = c_prefix(
            &m.program
                .functions
                .get(frame.function)
                .context("SACT debug frame function bounds")?
                .name,
        );
        ensure!(
            out.len()
                .checked_add(name.len())
                .and_then(|n| n.checked_add(2))
                .is_some_and(|n| n <= 1 << 20),
            "SACT debug stack string budget"
        );
        out.extend_from_slice(name);
        out.extend_from_slice(&[0x81, 0xa9]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ain::{Function, HllFunction, Library, Program, Variable},
        vm::Step,
    };
    use crysirror_core::service::{DesktopProvider, LogSnapshot};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    fn runtime() -> (tempfile::TempDir, crysirror_runtime::Runtime) {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("game")).unwrap();
        let project=serde_json::from_value(json!({"schema":2,"name":"native diagnostics/menu","content_revision":"1","assets":"game","canvas":[800,600],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        (root, rt)
    }
    fn call(
        lib: &[u8],
        name: &[u8],
        kind: i32,
        args: &[(i32, i32)],
        strings: Vec<Vec<u8>>,
        nested: bool,
    ) -> (Machine, ExternalCall) {
        let mut code = vec![];
        let op = |code: &mut Vec<u8>, kind: u16, args: &[i32]| {
            code.extend(kind.to_le_bytes());
            for arg in args {
                code.extend(arg.to_le_bytes());
            }
        };
        if nested {
            op(&mut code, 0x30, &[1]);
            op(&mut code, 0x2f, &[]);
        }
        let inner = code.len() as u32;
        for (kind, value) in args {
            if matches!(*kind, 19 | 51) {
                op(&mut code, 4, &[]);
            }
            op(
                &mut code,
                if matches!(*kind, 12 | 20) { 0x41 } else { 0 },
                &[*value],
            );
        }
        op(&mut code, 0x5a, &[0, 0]);
        op(&mut code, 0x2f, &[]);
        let function = |name: &str, address| Function {
            name: name.as_bytes().to_vec(),
            address,
            is_label: false,
            return_kind: 0,
            return_structure: -1,
            arguments: 0,
            crc: 0,
            variables: vec![],
        };
        let mut functions = vec![function("main", 0)];
        if nested {
            functions.push(function("inner", inner));
        }
        let program = Program {
            version: 5,
            code,
            strings,
            functions,
            globals: (0..args
                .iter()
                .filter(|(kind, _)| matches!(*kind, 19 | 51))
                .map(|(_, index)| index + 1)
                .max()
                .unwrap_or(0))
                .map(|_| Variable {
                    name: vec![],
                    kind: 10,
                    structure: -1,
                    rank: 0,
                    group: None,
                })
                .collect(),
            libraries: vec![Library {
                name: lib.to_vec(),
                functions: vec![HllFunction {
                    name: name.to_vec(),
                    return_kind: kind,
                    arguments: args.iter().map(|(kind, _)| (vec![], *kind)).collect(),
                }],
            }],
            ..Default::default()
        };
        let mut m = Machine::new(Arc::new(program)).unwrap();
        for _ in 0..100 {
            if let Step::External(call) = m.step().unwrap() {
                return (m, call);
            }
        }
        panic!("native call not reached")
    }
    #[test]
    fn native_error_commits_c_prefix_log_once_and_returns_the_original_zero() {
        let (_dir, mut rt) = runtime();
        let owner = ClientId(1);
        let (mut m, call) = call(
            b"SACT2",
            b"Error",
            10,
            &[(12, 0)],
            vec![b"message\0ignored".to_vec()],
            false,
        );
        let mut misc = Misc::default();
        let bridge = Bridge::default();
        assert_eq!(
            misc.dispatch(owner, &mut m, &call, &bridge, &mut rt.host)
                .unwrap(),
            Some(Dispatch::Completed)
        );
        assert_eq!(m.stack, [0]);
        let before: LogSnapshot = rt.host.log_snapshot(owner).unwrap();
        assert_eq!(before.records.len(), 1);
        assert_eq!(before.records[0].message, "message\n");
        assert!(misc
            .dispatch(owner, &mut m, &call, &bridge, &mut rt.host)
            .is_err());
        assert_eq!(rt.host.log_snapshot(owner).unwrap(), before);
    }
    #[test]
    fn native_absent_pad_queries_keep_float_bool_outputs_and_unacknowledged_enabled_calls() {
        let (_dir, mut rt) = runtime();
        let owner = ClientId(1);
        for (name, args) in [
            (
                b"Joypad_GetAnalogStickStatus".as_slice(),
                vec![(10, 0), (10, 0), (19, 0), (19, 1)],
            ),
            (
                b"Joypad_GetDigitalStickStatus".as_slice(),
                vec![(10, 0), (10, 0), (51, 0), (51, 1), (51, 2), (51, 3)],
            ),
        ] {
            let (mut m, call) = call(b"SACT2", name, 47, &args, vec![], false);
            let count = args.len() - 2;
            for i in 0..count {
                m.heap.set(0, i as i32, 91 + i as i32).unwrap();
            }
            let before = m.heap.page(0).unwrap().values.clone();
            let mut bridge = Bridge::default();
            bridge.joypad_enabled = Some(true);
            assert!(bridge
                .dispatch_for(owner, &mut m, &call, &mut rt.host)
                .is_err());
            assert_eq!(m.pc, call.pc);
            bridge.joypad_enabled = Some(false);
            assert_eq!(
                bridge
                    .dispatch_for(owner, &mut m, &call, &mut rt.host)
                    .unwrap(),
                Dispatch::Completed
            );
            assert_eq!(m.stack, [0]);
            assert_eq!(m.heap.page(0).unwrap().values, before);
        }
    }
    #[test]
    fn native_debug_stack_uses_native_depth_order_separator_and_engine_resume_receipt() {
        let (_dir, mut rt) = runtime();
        let owner = ClientId(1);
        let bridge = Bridge::default();
        let mut misc = Misc::default();
        for (nest, expected) in [
            (0, b"inner\x81\xa9".as_slice()),
            (-1, b"inner\x81\xa9main\x81\xa9".as_slice()),
            (100, b"inner\x81\xa9main\x81\xa9".as_slice()),
        ] {
            let (mut m, call) = call(
                b"SACT2",
                b"Debug_GetFuncStack",
                0,
                &[(20, 0), (10, nest)],
                vec![b"replace".to_vec()],
                true,
            );
            let Argument::HeapReference { slot, .. } = m.hll_arguments(&call).unwrap()[0] else {
                panic!()
            };
            misc.dispatch(owner, &mut m, &call, &bridge, &mut rt.host)
                .unwrap();
            assert_eq!(&**m.heap.string(slot).unwrap(), expected);
        }
        let (mut m, call) = call(b"SACT2", b"Debug_Pause", 0, &[], vec![], false);
        let now = rt.host.now_ms();
        assert!(matches!(
            misc.dispatch(owner, &mut m, &call, &bridge, &mut rt.host)
                .unwrap(),
            Some(Dispatch::WaitUntil(_))
        ));
        assert_eq!(m.pc, call.pc);
        assert!(misc.pending());
        rt.host
            .engine
            .ui
            .tick(
                crysirror_core::service::InputFrame {
                    buttons_pressed: crysirror_core::service::buttons::ENTER,
                    ..Default::default()
                },
                [800, 600],
            )
            .unwrap();
        assert_eq!(
            misc.dispatch(owner, &mut m, &call, &bridge, &mut rt.host)
                .unwrap(),
            Some(Dispatch::Completed)
        );
        assert!(!misc.pending());
        assert_eq!(rt.host.now_ms(), now);
    }
    struct Provider {
        calls: AtomicUsize,
        requests: Mutex<Vec<DesktopRequest>>,
    }
    struct Open(bool);
    impl DesktopRequestJob for Open {
        fn poll(&mut self) -> crysirror_core::Result<DesktopRequestState> {
            if !self.0 {
                self.0 = true;
                Ok(DesktopRequestState::Pending)
            } else {
                Ok(DesktopRequestState::Ready(DesktopResponse::Opened))
            }
        }
    }
    impl DesktopProvider for Provider {
        fn begin(
            &self,
            owner: ClientId,
            request: DesktopRequest,
        ) -> crysirror_core::Result<Box<dyn DesktopRequestJob>> {
            assert_eq!(owner, ClientId(1));
            self.calls.fetch_add(1, Ordering::Relaxed);
            self.requests.lock().unwrap().push(request);
            Ok(Box::new(Open(false)))
        }
    }
    #[test]
    fn native_url_registration_never_opens_until_real_shared_menu_activation() {
        let (_dir, mut rt) = runtime();
        let owner = ClientId(1);
        let bridge = Bridge::default();
        let mut misc = Misc::default();
        let provider = Arc::new(Provider {
            calls: AtomicUsize::new(0),
            requests: Mutex::new(vec![]),
        });
        rt.host.desktop_provider = Some(provider.clone());
        let (mut m, call) = call(
            b"SystemService",
            b"AddURLMenu",
            47,
            &[(12, 0), (12, 1)],
            vec![b"Website".to_vec(), b"https://example.com/".to_vec()],
            false,
        );
        misc.dispatch(owner, &mut m, &call, &bridge, &mut rt.host)
            .unwrap();
        assert_eq!(m.stack, [1]);
        misc.poll_menu(owner, &mut rt.host).unwrap();
        assert_eq!(provider.calls.load(Ordering::Relaxed), 0);
        rt.host
            .engine
            .ui
            .activate_menu_item(owner.0, BAR, FIRST_URL)
            .unwrap();
        misc.poll_menu(owner, &mut rt.host).unwrap();
        assert!(misc.pending());
        misc.poll_menu(owner, &mut rt.host).unwrap();
        assert!(!misc.pending());
        assert_eq!(provider.calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            &*provider.requests.lock().unwrap(),
            &[DesktopRequest::OpenUrl {
                url: "https://example.com/".into()
            }]
        );
    }
}
