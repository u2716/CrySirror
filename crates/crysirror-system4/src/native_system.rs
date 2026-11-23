// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d hll/SystemService.c and util.c.
//! Native platform policy over scoped engine UI/input/civil-time services.
//! Public warning stubs, hardcoded trial keys and missing platform functions
//! are deliberately not acknowledged as successful SystemService exports.
use crate::{
    adapter::{Bridge, Dispatch},
    bootstrap::Bootstrap,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
    },
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::{
        CivilTimeQuery, CivilTimeState, DesktopRequest, DesktopRequestJob, DesktopRequestState,
        DesktopResponse, HostServices,
    },
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use serde_json::json;
use std::sync::Arc;
mod misc;
enum Pending {
    Web {
        call: ExternalCall,
        job: Box<dyn DesktopRequestJob>,
    },
    DisplayPulse {
        call: ExternalCall,
        tick: u32,
        job: Box<dyn DesktopRequestJob>,
    },
    Civil {
        call: ExternalCall,
        date: bool,
        outputs: Vec<(i32, i32)>,
        query: Box<dyn CivilTimeQuery>,
    },
    Fullscreen {
        call: ExternalCall,
        enabled: bool,
        deadline: u64,
    },
    Manual {
        call: ExternalCall,
        open: bool,
        task: Task<Option<VfsPath>>,
    },
}
pub(crate) struct NativeSystem {
    owner: ClientId,
    pool: TaskPool,
    game_name: Option<Vec<u8>>,
    pending: Option<Pending>,
    cursor_config: i32,
    last_display_pulse: u32,
    misc: misc::Misc,
}
impl NativeSystem {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Self {
        Self {
            owner,
            pool,
            game_name: None,
            pending: None,
            cursor_config: 0,
            last_display_pulse: 0,
            misc: Default::default(),
        }
    }
    pub(crate) fn configure(&mut self, bootstrap: &Bootstrap) -> Result<()> {
        let (encoded, _, bad) = encoding_rs::SHIFT_JIS.encode(&bootstrap.game_name);
        ensure!(!bad, "System4 game name CP932 binding");
        self.game_name = Some(encoded.into_owned());
        Ok(())
    }
    pub(crate) fn has_pending(&self) -> bool {
        self.pending.is_some() || self.misc.pending()
    }
    pub(crate) fn close(&mut self,host:&mut dyn HostServices)->Result<()> {
        self.misc.close(self.owner,host)?;self.pending=None;Ok(())
    }
    pub(crate) fn poll_menu(&mut self,owner:ClientId,host:&mut dyn HostServices)->Result<()> {
        ensure!(owner==self.owner,"System4 foreign native menu owner");
        self.misc.poll_menu(owner,host)
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        ensure!(owner==self.owner,"System4 foreign native system owner");
        if let Some(dispatch)=self.misc.dispatch(owner,m,call,bridge,host)? {return Ok(Some(dispatch));}
        if matches!(call.target, ExternalTarget::System(11)) {
            ensure!(owner == self.owner, "System4 foreign OpenWeb owner");
            let slots = m.validate_system_owned_strings(call, 1)?;
            if self.pending.is_none() {
                let raw = m.heap.string(slots[0])?;
                let url = crysirror_resource::text_stream::decode_shift_jis(
                    raw.split(|b| *b == 0).next().unwrap(),
                    8192,
                )?;
                let request = DesktopRequest::OpenUrl { url };
                request.validate()?;
                let job = host.begin_desktop_request(owner, request)?;
                self.pending = Some(Pending::Web {
                    call: call.clone(),
                    job,
                });
            }
            let Some(Pending::Web { call: pending, job }) = &mut self.pending else {
                anyhow::bail!("System4 changed OpenWeb dependency")
            };
            ensure!(*pending == *call, "System4 stale OpenWeb receipt");
            match job.poll()? {
                DesktopRequestState::Pending => {
                    return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                }
                DesktopRequestState::Ready(DesktopResponse::Opened) => {
                    m.complete_system_owned_strings(call, 1, None)?;
                    self.pending = None;
                    return Ok(Some(Dispatch::Completed));
                }
                DesktopRequestState::Ready(_) => {
                    anyhow::bail!("System4 mismatched OpenWeb response")
                }
            }
        }
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending platform target"
            );
            return Ok(None);
        };
        let program = m.program.clone();
        let lib = &program.libraries[library];
        let function = &lib.functions[function];
        // Original SACT2 11200/11240 call GetLocalTime once and expose four
        // output refs (including milliseconds). SystemService.GetTime has
        // three refs. Both use the same scoped, worker-sampled civil clock.
        let sact_civil = lib.name == b"SACT2"
            && matches!(
                function.name.as_slice(),
                b"System_GetDate" | b"System_GetTime"
            );
        let name = match (sact_civil, function.name.as_slice()) {
            (true, b"System_GetDate") => b"GetDate".as_slice(),
            (true, b"System_GetTime") => b"GetTime".as_slice(),
            _ => function.name.as_slice(),
        };
        if !(lib.name == b"SystemService" || sact_civil)
            || !matches!(
                name,
                b"IsResetOnce"
                    | b"RestrainScreensaver"
                    | b"GetGameVersion"
                    | b"GetGameName"
                    | b"GetViewWidth"
                    | b"GetViewHeight"
                    | b"IsFullScreen"
                    | b"ChangeNormalScreen"
                    | b"ChangeFullScreen"
                    | b"MoveMouseCursorPosImmediately"
                    | b"SetHideMouseCursorByGame"
                    | b"GetDate"
                    | b"GetTime"
                    | b"IsExistPlayingManual"
                    | b"OpenPlayingManual"
                    | b"SetMouseCursorConfig"
                    | b"GetMouseCursorConfig"
            )
        {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending platform HLL"
            );
            return Ok(None);
        }
        ensure!(owner == self.owner, "System4 foreign platform owner");
        let args = m.hll_arguments(call)?;
        let void = matches!(
            name,
            b"GetGameName"
                | b"GetDate"
                | b"GetTime"
                | b"OpenPlayingManual"
                | b"RestrainScreensaver"
        );
        let result_kind = if void {
            0
        } else if matches!(name, b"GetGameVersion" | b"GetViewWidth" | b"GetViewHeight") {
            10
        } else {
            47
        };
        ensure!(
            function.return_kind == result_kind,
            "System4 platform result ABI"
        );
        if let Some(pending) = &self.pending {
            let pending = match pending {
                Pending::Web { call, .. }
                | Pending::Civil { call, .. }
                | Pending::Fullscreen { call, .. }
                | Pending::Manual { call, .. }
                | Pending::DisplayPulse { call, .. } => call,
            };
            ensure!(pending == call, "System4 changed pending platform receipt");
        }
        let words = args
            .iter()
            .map(|a| {
                if matches!(a, Argument::Reference { .. }) {
                    2
                } else {
                    1
                }
            })
            .sum::<usize>();
        ensure!(
            void || m.stack.len().checked_sub(words).is_some_and(|n| n < 65536),
            "System4 platform return stack budget"
        );
        let result = match name {
            b"IsResetOnce" => {
                ensure!(args.is_empty(), "System4 reset-once query ABI");
                Some(i32::from(m.reset_once))
            }

            b"RestrainScreensaver" => {
                ensure!(args.is_empty(), "System4 display pulse ABI");
                if self.pending.is_none() {
                    let tick = host.now_ms() as u32;
                    // Reviewed SystemService10001430 uses native unsigned32
                    // addition/comparison, including its original wrap branch.
                    if self.last_display_pulse.wrapping_add(5000) > tick {
                        m.complete_hll(call, HllReturn::Void)?;
                        return Ok(Some(Dispatch::Completed));
                    }
                    let job =
                        host.begin_desktop_request(owner, DesktopRequest::PulseDisplayActivity)?;
                    self.pending = Some(Pending::DisplayPulse {
                        call: call.clone(),
                        tick,
                        job,
                    });
                }
                let Some(Pending::DisplayPulse {
                    call: pending,
                    tick,
                    job,
                }) = &mut self.pending
                else {
                    anyhow::bail!("System4 changed display pulse dependency")
                };
                ensure!(*pending == *call, "System4 stale display pulse receipt");
                match job.poll()? {
                    DesktopRequestState::Pending => {
                        return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                    }
                    DesktopRequestState::Ready(DesktopResponse::DisplayActivityPulsed) => {
                        self.last_display_pulse = *tick;
                        self.pending = None;
                        None
                    }
                    DesktopRequestState::Ready(_) => {
                        anyhow::bail!("System4 mismatched display pulse response")
                    }
                }
            }

            b"GetGameVersion" => {
                ensure!(args.is_empty(), "System4 game version ABI");
                Some(program.game_version)
            }
            b"GetGameName" => {
                let [Argument::HeapReference { kind: 20, slot }] = args.as_slice() else {
                    anyhow::bail!("System4 game name ABI")
                };
                let name = self
                    .game_name
                    .as_ref()
                    .context("System4 missing game name bootstrap")?;
                m.heap.set_string(*slot, Arc::new(name.clone()))?;
                None
            }
            b"GetViewWidth" | b"GetViewHeight" => {
                ensure!(args.is_empty(), "System4 view dimension ABI");
                let canvas = bridge
                    .sact
                    .as_ref()
                    .context("System4 missing native view metadata")?
                    .canvas;
                Some(i32::try_from(
                    canvas[usize::from(name == b"GetViewHeight")],
                )?)
            }
            b"IsFullScreen" => {
                ensure!(args.is_empty(), "System4 fullscreen query ABI");
                Some(i32::from(
                    host.window_fullscreen(owner)?
                        .context("System4 fullscreen observation unavailable")?,
                ))
            }
            b"ChangeNormalScreen" | b"ChangeFullScreen" => {
                ensure!(args.is_empty(), "System4 fullscreen setter ABI");
                let enabled = name == b"ChangeFullScreen";
                if self.pending.is_none() {
                    // Require a physical or explicitly virtual observation;
                    // request acceptance is not confirmation of window state.
                    host.window_fullscreen(owner)?
                        .context("System4 fullscreen observation unavailable")?;
                    let deadline = host
                        .now_ms()
                        .checked_add(5000)
                        .context("System4 fullscreen deadline overflow")?;
                    host.submit_engine(
                        owner,
                        1,
                        &serde_json::to_vec(
                            &json!({"ui":[{"op":"set_fullscreen","enabled":enabled}]}),
                        )?,
                    )?;
                    self.pending = Some(Pending::Fullscreen {
                        call: call.clone(),
                        enabled,
                        deadline,
                    });
                }
                let Some(Pending::Fullscreen {
                    call: pending,
                    enabled: desired,
                    deadline,
                }) = &self.pending
                else {
                    anyhow::bail!("System4 changed fullscreen dependency")
                };
                ensure!(
                    *pending == *call && *desired == enabled,
                    "System4 stale fullscreen receipt"
                );
                if host.window_fullscreen(owner)? != Some(enabled) {
                    ensure!(
                        host.now_ms() < *deadline,
                        "System4 fullscreen host request unconfirmed"
                    );
                    return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                }
                self.pending = None;
                Some(1)
            }
            b"MoveMouseCursorPosImmediately" => {
                let [
                    Argument::Scalar { kind: 10, bits: x },
                    Argument::Scalar { kind: 10, bits: y },
                ] = args.as_slice()
                else {
                    anyhow::bail!("System4 pointer warp ABI")
                };
                host.warp_pointer(owner, *x, *y)?;
                Some(1)
            }
            b"SetHideMouseCursorByGame" => {
                let [
                    Argument::Scalar {
                        kind: 47,
                        bits: hide,
                    },
                ] = args.as_slice()
                else {
                    anyhow::bail!("System4 cursor visibility ABI")
                };
                host.submit_engine(
                    owner,
                    1,
                    &serde_json::to_vec(
                        &json!({"ui":[{"op":"set_cursor_visible","visible":*hide==0}]}),
                    )?,
                )?;
                Some(1)
            }
            b"SetMouseCursorConfig" => {
                let [
                    Argument::Scalar {
                        kind: 10,
                        bits: index,
                    },
                    Argument::Scalar {
                        kind: 10,
                        bits: value,
                    },
                ] = args.as_slice()
                else {
                    anyhow::bail!("System4 cursor config setter ABI")
                };
                if *index != 0 {
                    Some(0)
                } else {
                    self.cursor_config = *value;
                    Some(1)
                }
            }
            b"GetMouseCursorConfig" => {
                let [
                    Argument::Scalar {
                        kind: 10,
                        bits: which,
                    },
                    Argument::Reference {
                        kind: 18,
                        page,
                        index,
                    },
                ] = args.as_slice()
                else {
                    anyhow::bail!("System4 cursor config getter ABI")
                };
                if *which != 0 {
                    Some(0)
                } else {
                    m.heap
                        .set(*page, *index, i32::from(self.cursor_config != 0))?;
                    Some(1)
                }
            }
            b"GetDate" | b"GetTime" => {
                let date = name == b"GetDate";
                ensure!(
                    args.len() == if date || sact_civil { 4 } else { 3 },
                    "System4 civil output count"
                );
                let outputs = args
                    .iter()
                    .map(|arg| {
                        let Argument::Reference {
                            kind: 18,
                            page,
                            index,
                        } = arg
                        else {
                            anyhow::bail!("System4 civil output ABI")
                        };
                        m.heap.get(*page, *index)?;
                        Ok((*page, *index))
                    })
                    .collect::<Result<Vec<_>>>()?;
                if self.pending.is_none() {
                    let query = host.begin_civil_time(owner)?;
                    ensure!(query.client() == owner, "System4 foreign civil query");
                    self.pending = Some(Pending::Civil {
                        call: call.clone(),
                        date,
                        outputs: outputs.clone(),
                        query,
                    });
                }
                let Some(Pending::Civil {
                    call: pending,
                    date: kind,
                    outputs: old,
                    query,
                }) = &mut self.pending
                else {
                    anyhow::bail!("System4 changed civil dependency")
                };
                ensure!(
                    *pending == *call
                        && *kind == date
                        && *old == outputs
                        && query.client() == owner,
                    "System4 stale civil receipt"
                );
                let sample = match query.poll()? {
                    CivilTimeState::Pending => {
                        return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                    }
                    CivilTimeState::Failed(e) => anyhow::bail!("System4 civil clock: {e}"),
                    CivilTimeState::Ready(sample) => sample,
                };
                ensure!(
                    sample.client == owner
                        && sample.hour < 24
                        && sample.minute < 60
                        && sample.second < 60
                        && sample.millisecond < 1000,
                    "System4 invalid civil receipt"
                );
                let mut values = if date {
                    vec![
                        sample.year,
                        i32::from(sample.month),
                        i32::from(sample.day),
                        i32::from(sample.weekday().context("System4 invalid sampled date")?),
                    ]
                } else {
                    vec![
                        i32::from(sample.hour),
                        i32::from(sample.minute),
                        i32::from(sample.second),
                    ]
                };
                if !date && sact_civil {
                    values.push(i32::from(sample.millisecond));
                }
                // All refs/types/receipt fields are validated before any write.
                // Aliased outputs retain the native left-to-right assignment.
                for ((page, index), value) in outputs.into_iter().zip(values) {
                    m.heap.set(page, index, value)?;
                }
                self.pending = None;
                None
            }
            b"IsExistPlayingManual" | b"OpenPlayingManual" => {
                ensure!(args.is_empty(), "System4 manual operation ABI");
                let open = name == b"OpenPlayingManual";
                if self.pending.is_none() {
                    let storage = host.storage(owner)?;
                    let path = VfsPath::new(Mount::Asset, "Manual/index.html")?;
                    self.pending=Some(Pending::Manual{call:call.clone(),open,task:self.pool.spawn(move|cancel|{
                        cancel.check()?;let found=crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(storage.as_ref(),&path)?;
                        let found=if let Some(path)=found {match storage.open_asset(&path){Ok(_)=>Some(path),Err(Error::NotFound|Error::Io(_))=>None,Err(e)=>return Err(e.into())}}else{None};cancel.check()?;Ok(found)
                    })?});
                }
                let Some(Pending::Manual {
                    call: pending,
                    open: kind,
                    task,
                }) = &mut self.pending
                else {
                    anyhow::bail!("System4 changed manual dependency")
                };
                ensure!(
                    *pending == *call && *kind == open,
                    "System4 stale manual receipt"
                );
                let Some(path) = task.try_take()? else {
                    return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
                };
                if open {
                    if let Some(path) = path {
                        host.open_document(owner, &path)?;
                    }
                    self.pending = None;
                    None
                } else {
                    self.pending = None;
                    Some(i32::from(path.is_some()))
                }
            }
            _ => unreachable!(),
        };
        m.complete_hll(
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

#[cfg(test)]
mod civil_tests {
    use super::*;
    use crate::{
        ain::{Function, HllFunction, Library, Program, Variable},
        vm::Step,
    };
    use crysirror_core::{
        service::{CivilTime, FileSystem},
        state::Command,
    };
    struct Query {
        owner: ClientId,
        pending: bool,
    }
    impl CivilTimeQuery for Query {
        fn client(&self) -> ClientId {
            self.owner
        }
        fn poll(&mut self) -> crysirror_core::Result<CivilTimeState> {
            if self.pending {
                self.pending = false;
                return Ok(CivilTimeState::Pending);
            }
            Ok(CivilTimeState::Ready(CivilTime {
                client: self.owner,
                unix_ms: 0,
                utc_offset_seconds: 0,
                year: 2026,
                month: 11,
                day: 9,
                hour: 12,
                minute: 34,
                second: 56,
                millisecond: 789,
            }))
        }
    }
    struct Host {
        begins: usize,
    }
    impl FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(Error::Unsupported("test FS"))
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(Error::Unsupported("test FS"))
        }
    }
    impl HostServices for Host {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Err(Error::Unsupported("test submit"))
        }
        fn now_ms(&self) -> u64 {
            50
        }
        fn begin_civil_time(
            &mut self,
            owner: ClientId,
        ) -> crysirror_core::Result<Box<dyn CivilTimeQuery>> {
            self.begins += 1;
            Ok(Box::new(Query {
                owner,
                pending: true,
            }))
        }
    }
    fn pending(name: &[u8]) -> (Machine, ExternalCall) {
        let mut code = Vec::new();
        for n in 0i32..4 {
            code.extend(4u16.to_le_bytes());
            code.extend(0u16.to_le_bytes());
            code.extend(n.to_le_bytes());
        }
        code.extend(0x5au16.to_le_bytes());
        code.extend(0i32.to_le_bytes());
        code.extend(0i32.to_le_bytes());
        code.extend(0x2fu16.to_le_bytes());
        let mut m = Machine::new(Arc::new(Program {
            version: 5,
            code,
            globals: (0..4)
                .map(|_| Variable {
                    name: vec![],
                    kind: 10,
                    structure: -1,
                    rank: 0,
                    group: None,
                })
                .collect(),
            functions: vec![Function {
                name: b"main".to_vec(),
                address: 0,
                is_label: false,
                return_kind: 0,
                return_structure: -1,
                arguments: 0,
                crc: 0,
                variables: vec![],
            }],
            libraries: vec![Library {
                name: b"SACT2".to_vec(),
                functions: vec![HllFunction {
                    name: name.to_vec(),
                    return_kind: 0,
                    arguments: vec![(vec![], 18); 4],
                }],
            }],
            ..Default::default()
        }))
        .unwrap();
        for _ in 0..100 {
            if let Step::External(call) = m.step().unwrap() {
                return (m, call);
            }
        }
        panic!("no civil call")
    }
    #[test]
    fn sact_civil_outputs_wait_for_one_real_receipt_and_include_milliseconds() {
        for (name, expected) in [
            (b"System_GetDate".as_slice(), [2026, 11, 9, 1]),
            (b"System_GetTime".as_slice(), [12, 34, 56, 789]),
        ] {
            let owner = ClientId(1);
            let (mut m, call) = pending(name);
            let mut host = Host { begins: 0 };
            let mut system =
                NativeSystem::new(owner, TaskPool::new("sact-civil-test", 1, 2).unwrap());
            let bridge = Bridge::default();
            assert!(matches!(
                system
                    .dispatch(owner, &mut m, &call, &bridge, &mut host)
                    .unwrap(),
                Some(Dispatch::WaitUntil(_))
            ));
            assert_eq!(m.pc, call.pc);
            assert_eq!(m.heap.page(0).unwrap().values, [0; 4]);
            assert_eq!(
                system
                    .dispatch(owner, &mut m, &call, &bridge, &mut host)
                    .unwrap(),
                Some(Dispatch::Completed)
            );
            assert_eq!(m.heap.page(0).unwrap().values, expected);
            assert_eq!(host.begins, 1);
        }
    }
}
