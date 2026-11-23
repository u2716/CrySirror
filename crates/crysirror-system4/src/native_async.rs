// Native HLL dependency policy from xsystem4 Math.c a85467d78e.
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// GPL-2.0-or-later. Host jobs belong to NativeAdapter, not cloned VM state.
use crate::{
    adapter::Dispatch,
    vm::{ExternalCall, ExternalTarget, Machine},
};
use anyhow::{ensure, Result};
use crysirror_core::{
    service::{CivilTimeQuery, CivilTimeState, HostServices},
    state::ClientId,
};

struct CivilSeed {
    owner: ClientId,
    call: ExternalCall,
    query: Box<dyn CivilTimeQuery>,
}
#[derive(Default)]
pub(crate) struct NativeAsync {
    civil_seed: Option<CivilSeed>,
    pub(crate) civil_samples: u64,
}
impl NativeAsync {
    pub(crate) fn cancel_pending_for_shutdown(&mut self) {
        self.civil_seed = None;
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.civil_seed.is_none(),
                "System4 changed pending civil-time target"
            );
            return Ok(None);
        };
        let library = &machine.program.libraries[library];
        let function = &library.functions[function];
        if library.name != b"Math" || function.name != b"SetSeedByCurrentTime" {
            ensure!(
                self.civil_seed.is_none(),
                "System4 changed pending civil-time HLL"
            );
            return Ok(None);
        }
        ensure!(
            function.return_kind == 0 && machine.hll_arguments(call)?.is_empty(),
            "System4 Math.SetSeedByCurrentTime ABI"
        );
        if self.civil_seed.is_none() {
            let query = host.begin_civil_time(owner)?;
            ensure!(query.client() == owner, "System4 foreign civil-time query");
            self.civil_seed = Some(CivilSeed {
                owner,
                call: call.clone(),
                query,
            });
        }
        let pending = self.civil_seed.as_mut().unwrap();
        ensure!(
            pending.owner == owner && pending.call == *call && pending.query.client() == owner,
            "System4 stale civil-time dependency"
        );
        match pending.query.poll()? {
            CivilTimeState::Pending => {
                Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))))
            }
            CivilTimeState::Failed(error) => {
                anyhow::bail!("System4 host civil-time sample: {error}")
            }
            CivilTimeState::Ready(sample) => {
                ensure!(sample.client == owner, "System4 foreign civil-time receipt");
                // time(NULL) original policy uses Unix seconds, not the virtual
                // engine clock, timezone fields or a fabricated constant.
                machine.complete_math_civil_seed(call, sample.unix_ms)?;
                self.civil_seed = None;
                self.civil_samples += 1;
                Ok(Some(Dispatch::Completed))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ain::{Function, HllFunction, Library, Program},
        vm::Step,
    };
    use crysirror_core::{
        path::VfsPath,
        service::{CivilTime, FileSystem},
        state::Command,
        Error,
    };
    use std::{
        collections::VecDeque,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };
    struct Query {
        owner: ClientId,
        states: VecDeque<CivilTimeState>,
        dropped: Arc<AtomicUsize>,
    }
    impl Drop for Query {
        fn drop(&mut self) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    impl CivilTimeQuery for Query {
        fn client(&self) -> ClientId {
            self.owner
        }
        fn poll(&mut self) -> crysirror_core::Result<CivilTimeState> {
            Ok(self.states.pop_front().unwrap_or(CivilTimeState::Pending))
        }
    }
    struct Host {
        query: Option<Query>,
        begins: usize,
    }
    impl FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(Error::Unsupported("authored FS"))
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(Error::Unsupported("authored FS"))
        }
    }
    impl HostServices for Host {
        fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
            Err(Error::Unsupported("authored submit"))
        }
        fn now_ms(&self) -> u64 {
            50_000
        }
        fn begin_civil_time(
            &mut self,
            _: ClientId,
        ) -> crysirror_core::Result<Box<dyn CivilTimeQuery>> {
            self.begins += 1;
            self.query
                .take()
                .map(|q| Box::new(q) as Box<dyn CivilTimeQuery>)
                .ok_or(Error::Unsupported("authored unavailable civil clock"))
        }
    }
    fn machine() -> (Machine, ExternalCall) {
        let mut code = Vec::new();
        for function in [0i32, 1] {
            code.extend(0x5au16.to_le_bytes());
            code.extend(0i32.to_le_bytes());
            code.extend(function.to_le_bytes());
        }
        code.extend(0x2fu16.to_le_bytes());
        let p = Program {
            version: 5,
            code,
            functions: vec![Function {
                address: 0,
                name: b"main".to_vec(),
                is_label: false,
                return_kind: 0,
                return_structure: -1,
                arguments: 0,
                crc: 0,
                variables: vec![],
            }],
            libraries: vec![Library {
                name: b"Math".to_vec(),
                functions: vec![
                    HllFunction {
                        name: b"SetSeedByCurrentTime".to_vec(),
                        return_kind: 0,
                        arguments: vec![],
                    },
                    HllFunction {
                        name: b"Rand".to_vec(),
                        return_kind: 10,
                        arguments: vec![],
                    },
                ],
            }],
            ..Default::default()
        };
        let mut m = Machine::new(Arc::new(p)).unwrap();
        for _ in 0..100 {
            if let Step::External(call) = m.step().unwrap() {
                return (m, call);
            }
        }
        panic!("missing native host dependency")
    }
    fn sample(owner: ClientId) -> CivilTime {
        CivilTime {
            client: owner,
            unix_ms: 0,
            utc_offset_seconds: 0,
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0,
        }
    }
    fn host(owner: ClientId, states: Vec<CivilTimeState>) -> (Host, Arc<AtomicUsize>) {
        let dropped = Arc::new(AtomicUsize::new(0));
        (
            Host {
                query: Some(Query {
                    owner,
                    states: states.into(),
                    dropped: dropped.clone(),
                }),
                begins: 0,
            },
            dropped,
        )
    }
    #[test]
    fn real_host_receipt_seeds_original_seconds_after_wait_not_virtual_clock() {
        let owner = ClientId(7);
        let (mut machine, call) = machine();
        let (mut host, dropped) = host(
            owner,
            vec![
                CivilTimeState::Pending,
                CivilTimeState::Ready(sample(owner)),
            ],
        );
        let mut native = NativeAsync::default();
        assert_eq!(
            native
                .dispatch(owner, &mut machine, &call, &mut host)
                .unwrap(),
            Some(Dispatch::WaitUntil(50_001))
        );
        assert_eq!(machine.pc, 0);
        assert_eq!(machine.step().unwrap(), Step::External(call.clone()));
        assert_eq!(
            native
                .dispatch(owner, &mut machine, &call, &mut host)
                .unwrap(),
            Some(Dispatch::Completed)
        );
        assert_eq!(host.begins, 1);
        assert_eq!(native.civil_samples, 1);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
        for _ in 0..100 {
            if machine.step().unwrap() == Step::Halted {
                break;
            }
        }
        // unix seconds0 -> native odd seed1, discard1. The virtual clock50s
        // would produce another stream; no wall-clock access occurs in tests.
        assert_eq!(machine.stack, [(4182529786u32 & 0x7fff_ffff) as i32]);
    }
    #[test]
    fn unavailable_foreign_failed_and_dropped_dependencies_never_ack() {
        let owner = ClientId(7);
        let (mut machine, call) = machine();
        let mut native = NativeAsync::default();
        let mut unavailable = Host {
            query: None,
            begins: 0,
        };
        assert!(native
            .dispatch(owner, &mut machine, &call, &mut unavailable)
            .is_err());
        let (mut foreign, _) = host(ClientId(8), vec![CivilTimeState::Pending]);
        assert!(native
            .dispatch(owner, &mut machine, &call, &mut foreign)
            .is_err());
        let (mut failed, _) = host(owner, vec![CivilTimeState::Failed("real failure".into())]);
        assert!(native
            .dispatch(owner, &mut machine, &call, &mut failed)
            .is_err());
        assert_eq!(machine.pc, 0);
        assert_eq!(machine.step().unwrap(), Step::External(call.clone()));
        assert_eq!(native.civil_samples, 0);
        let mut native = NativeAsync::default();
        let (mut pending, dropped) = host(owner, vec![CivilTimeState::Pending]);
        native
            .dispatch(owner, &mut machine, &call, &mut pending)
            .unwrap();
        drop(native);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
        assert_eq!(machine.step().unwrap(), Step::External(call));
    }
}
