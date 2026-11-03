// GPL-2.0-or-later. Native MsgSkip storage/HLL policy; xsystem4 a85467d
// MsgSkip.c plus reviewed owner 2004 MsgSkip.dll. (C) 2019 Nunuhara Cabbage;
// Rust (C) 2026 CrySirror contributors. See SOURCE.json for ABI differences.
// Bit operations use shared Core integer_fields, IO uses scoped engine workers.
use crate::{
    adapter::Dispatch,
    native_files::NativeFiles,
    vm::{
        external::{Argument, HllReturn},
        ExternalCall, ExternalTarget, Machine,
    },
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    integer_fields,
    path::{Mount, VfsPath},
    service::{ClientStorage, HostServices, StorageWrite, StorageWriteState},
    state::ClientId,
    Error,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
struct Flags {
    path: VfsPath,
    messages: usize,
    words: Vec<i64>,
    set: u32,
}
impl Flags {
    fn get(&self, index: i32) -> bool {
        if index < 0 || index as usize >= self.messages {
            return false;
        }
        integer_fields::get(&self.words, 1, native_bit(index) as i64) == Some(1)
    }
    fn mark(&mut self, index: i32) {
        if index < 0 || index as usize >= self.messages || self.get(index) {
            return;
        }
        let ok = integer_fields::set(&mut self.words, 1, native_bit(index) as i64, 1);
        debug_assert!(ok);
        self.set += 1;
    }
    fn encoded(&self) -> Arc<[u8]> {
        let bytes = self.messages.div_ceil(8);
        let mut out = Vec::with_capacity(bytes + 4);
        out.extend((self.messages as u32).to_le_bytes());
        for word in &self.words {
            out.extend((*word as u32).to_le_bytes());
        }
        out.truncate(4 + bytes);
        out.into()
    }
}
fn native_bit(index: i32) -> usize {
    let index = index as usize;
    (index / 8) * 8 + 7 - (index % 8)
}
enum Phase {
    Flush(Box<dyn StorageWrite>),
    Read(Task<Flags>),
}
struct Pending {
    owner: ClientId,
    call: ExternalCall,
    path: VfsPath,
    original: VfsPath,
    messages: usize,
    phase: Phase,
}
pub(crate) struct NativeMsgSkip {
    owner: ClientId,
    pool: TaskPool,
    flags: Option<Flags>,
    pending: Option<Pending>,
    use_flag: bool,
    enabled: bool,
    state: bool,
    exit_write: Option<Box<dyn StorageWrite>>,
    exit_flushed: bool,
}
impl NativeMsgSkip {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Self {
        Self {
            owner,
            pool,
            flags: None,
            pending: None,
            use_flag: true,
            enabled: false,
            state: false,
            exit_write: None,
            exit_flushed: false,
        }
    }
    pub(crate) fn pending_init(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn abandon_failed_init(&mut self) {
        self.pending = None;
    }
    pub(crate) fn has_state(&self) -> bool {
        self.flags.is_some() || self.pending.is_some()
    }
    pub(crate) fn needs_flush(&self) -> bool {
        self.flags.is_some() && self.use_flag && !self.exit_flushed
    }
    pub(crate) fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"initialized":self.flags.is_some(),"pending":self.pending.is_some(),
            "messages":self.flags.as_ref().map_or(0,|f|f.messages),"flagged_bits_including_padding":self.flags.as_ref().map_or(0,|f|f.set),
            "use_flag":self.use_flag,"enabled":self.enabled,"state":self.state,"exit_flushed":self.exit_flushed})
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        machine: &mut Machine,
        call: &ExternalCall,
        files: &NativeFiles,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending MsgSkip target"
            );
            return Ok(None);
        };
        let program = machine.program.clone();
        let library = &program.libraries[library];
        let function = &library.functions[function];
        if library.name != b"MsgSkip" {
            ensure!(
                self.pending.is_none(),
                "System4 changed pending MsgSkip library"
            );
            return Ok(None);
        }
        ensure!(owner == self.owner, "System4 foreign MsgSkip owner");
        let args = machine.hll_arguments(call)?;
        if function.name == b"Init" {
            ensure!(
                function.return_kind == 10,
                "System4 MsgSkip.Init return ABI"
            );
            let [Argument::String { bytes, .. }] = args.as_slice() else {
                anyhow::bail!("System4 MsgSkip.Init filename ABI")
            };
            if self.pending.is_none() {
                let (path, original) = files.save_path(bytes)?;
                let messages = program.messages.len();
                ensure!(
                    messages <= 1_000_000,
                    "System4 MsgSkip message count budget"
                );
                let storage = host.storage(owner)?;
                let phase = if self.use_flag && self.flags.is_some() {
                    let flags = self.flags.as_ref().unwrap();
                    Phase::Flush(storage.begin_write_atomic(&flags.path, flags.encoded())?)
                } else {
                    Phase::Read(start_read(
                        &self.pool,
                        storage,
                        path.clone(),
                        original.clone(),
                        messages,
                    )?)
                };
                self.pending = Some(Pending {
                    owner,
                    call: call.clone(),
                    path,
                    original,
                    messages,
                    phase,
                });
            }
            let pending = self.pending.as_mut().unwrap();
            ensure!(
                pending.owner == owner && pending.call == *call,
                "System4 stale MsgSkip Init receipt"
            );
            loop {
                match &mut pending.phase {
                    Phase::Flush(query) => match query.poll()? {
                        StorageWriteState::Pending => break,
                        StorageWriteState::Failed(error) => {
                            anyhow::bail!("System4 MsgSkip prior-file write: {error}")
                        }
                        StorageWriteState::Ready => {
                            pending.phase = Phase::Read(start_read(
                                &self.pool,
                                host.storage(owner)?,
                                pending.path.clone(),
                                pending.original.clone(),
                                pending.messages,
                            )?)
                        }
                    },
                    Phase::Read(task) => {
                        let Some(flags) = task.try_take()? else { break };
                        self.flags = Some(flags);
                        self.exit_flushed = false;
                        machine.complete_hll(call, HllReturn::Scalar(1))?;
                        self.pending = None;
                        return Ok(Some(Dispatch::Completed));
                    }
                }
            }
            return Ok(Some(Dispatch::WaitUntil(host.now_ms().saturating_add(1))));
        }
        ensure!(
            self.pending.is_none(),
            "System4 changed MsgSkip call before initialization completed"
        );
        let number = || -> Result<i32> {
            let [Argument::Scalar { kind: 10, bits }] = args.as_slice() else {
                anyhow::bail!("System4 MsgSkip integer ABI")
            };
            Ok(*bits)
        };
        let initialized = self.flags.is_some();
        let result = match function.name.as_slice() {
            b"UseFlag" | b"SetEnable" | b"SetState" | b"SetFlag" => {
                ensure!(
                    function.return_kind == 0,
                    "System4 MsgSkip setter return ABI"
                );
                let value = number()?;
                if initialized {
                    match function.name.as_slice() {
                        b"UseFlag" => {
                            self.use_flag = value != 0;
                            self.exit_flushed = false
                        }
                        b"SetEnable" => self.enabled = value != 0,
                        b"SetState" => self.state = value != 0,
                        _ => {
                            self.flags.as_mut().unwrap().mark(value);
                            self.exit_flushed = false
                        }
                    }
                }
                HllReturn::Void
            }
            b"GetFlag" => {
                ensure!(
                    function.return_kind == 10,
                    "System4 MsgSkip getter return ABI"
                );
                let index = number()?;
                HllReturn::Scalar(i32::from(self.flags.as_ref().is_some_and(|f| {
                    index >= 0 && (index as usize) < f.messages && (!self.use_flag || f.get(index))
                })))
            }
            b"GetEnable" | b"GetState" | b"GetNumofMsg" | b"GetNumofFlag" => {
                ensure!(
                    function.return_kind == 10 && args.is_empty(),
                    "System4 MsgSkip query ABI"
                );
                let value = match function.name.as_slice() {
                    b"GetEnable" => i32::from(initialized && self.enabled),
                    b"GetState" => i32::from(initialized && self.state),
                    b"GetNumofMsg" => self.flags.as_ref().map_or(0, |f| f.messages as i32),
                    _ => self.flags.as_ref().map_or(0, |f| f.set as i32),
                };
                HllReturn::Scalar(value)
            }
            _ => anyhow::bail!(
                "unsupported original MsgSkip export {}",
                String::from_utf8_lossy(&function.name)
            ),
        };
        machine.complete_hll(call, result)?;
        Ok(Some(Dispatch::Completed))
    }
    pub(crate) fn flush(
        &mut self,
        owner: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<Dispatch> {
        ensure!(
            owner == self.owner,
            "System4 foreign read-history flush owner"
        );
        ensure!(
            self.pending.is_none(),
            "System4 read-history flush before admitted Init completed"
        );
        if !self.needs_flush() {
            return Ok(Dispatch::Completed);
        }
        if self.exit_write.is_none() {
            let f = self.flags.as_ref().unwrap();
            self.exit_write = Some(
                host.storage(owner)?
                    .begin_write_atomic(&f.path, f.encoded())?,
            );
        }
        match self.exit_write.as_mut().unwrap().poll()? {
            StorageWriteState::Pending => Ok(Dispatch::WaitUntil(host.now_ms().saturating_add(1))),
            StorageWriteState::Failed(error) => {
                anyhow::bail!("System4 read-history atomic save failed: {error}")
            }
            StorageWriteState::Ready => {
                self.exit_write = None;
                self.exit_flushed = true;
                Ok(Dispatch::Completed)
            }
        }
    }
}
fn start_read(
    pool: &TaskPool,
    storage: Arc<dyn ClientStorage>,
    path: VfsPath,
    original: VfsPath,
    messages: usize,
) -> Result<Task<Flags>> {
    pool.spawn(move |cancel| {
        cancel.check()?;
        let bytes = messages.div_ceil(8);
        let expected = 4 + bytes;
        let mut flags = Flags {
            path: path.clone(),
            messages,
            words: vec![0; bytes.div_ceil(4)],
            set: 0,
        };
        let found = crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
            storage.as_ref(),
            &path,
        )?;
        let found = if found.is_some() {
            found
        } else {
            crysirror_resource::capability_lookup::resolve_ascii_case_insensitive(
                storage.as_ref(),
                &original,
            )?
        };
        let Some(found) = found else { return Ok(flags) };
        let metadata = storage
            .metadata(found.mount(), found.relative())?
            .context("System4 MsgSkip source vanished")?;
        if metadata.is_directory || metadata.size == Some(0) {
            return Ok(flags);
        }
        let size = metadata
            .size
            .context("System4 MsgSkip file size unavailable")?;
        if size != expected as u64 {
            reset_file(storage.as_ref(), &found)?;
            return Ok(flags);
        }
        let data = storage.read(&found)?;
        cancel.check()?;
        ensure!(
            data.len() == expected,
            "System4 MsgSkip source length changed during read"
        );
        if u32::from_le_bytes(data[..4].try_into().unwrap()) as usize != messages {
            reset_file(storage.as_ref(), &found)?;
            return Ok(flags);
        }
        if found.mount() == Mount::Save {
            flags.path = found;
        }
        for (index, chunk) in data[4..].chunks(4).enumerate() {
            if index % 1024 == 0 {
                cancel.check()?;
            }
            let mut word = [0; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            let word = u32::from_le_bytes(word);
            flags.words[index] = i64::from(word as i32);
            flags.set += word.count_ones();
        }
        cancel.check()?;
        Ok(flags)
    })
}
fn reset_file(storage: &dyn ClientStorage, path: &VfsPath) -> Result<()> {
    ensure!(path.mount()==Mount::Save,"System4 corrupt original read-only MsgSkip needs explicit import/reset; original game data is not modified");
    match storage.remove(path) {
        Ok(()) | Err(Error::NotFound) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;
    use crate::{
        ain::{Function, HllFunction, Library, Program},
        vm::{heap::Object, Step},
        worker_adapter::{Config, NativeAdapter},
    };
    use crysirror_core::{
        service::InputFrame,
        vm::{VmAdapter, VmYield},
    };
    use crysirror_runtime::Runtime;
    use std::time::{Duration, Instant};
    fn runtime() -> (tempfile::TempDir, Runtime) {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("assets")).unwrap();
        let project=serde_json::from_value(serde_json::json!({"schema":2,"name":"MsgSkip native IO","content_revision":"1",
            "canvas":[2,2],"assets":"assets","clients":[{"name":"one","adapter":"crysirror-script-v1","game":"one"}]})).unwrap();
        let rt = Runtime::with_asset_root(
            project,
            &root.path().join("assets"),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        (root, rt)
    }
    fn pending(
        name: &str,
        value: Option<i32>,
        filename: Option<&[u8]>,
        messages: usize,
    ) -> (Machine, ExternalCall) {
        let ret = if matches!(
            name,
            "Init" | "GetFlag" | "GetState" | "GetEnable" | "GetNumofMsg" | "GetNumofFlag"
        ) {
            10
        } else {
            0
        };
        let args = if filename.is_some() {
            vec![(vec![], 12)]
        } else if value.is_some() {
            vec![(vec![], 10)]
        } else {
            vec![]
        };
        let mut code = 0x5au16.to_le_bytes().to_vec();
        code.extend(0i32.to_le_bytes());
        code.extend(0i32.to_le_bytes());
        code.extend(0x2fu16.to_le_bytes());
        let mut m = Machine::new(Arc::new(Program {
            version: 5,
            code,
            messages: vec![b"authored".to_vec(); messages],
            functions: vec![Function {
                address: 0,
                name: b"main".to_vec(),
                is_label: false,
                return_kind: 0,
                return_structure: -1,
                arguments: 0,
                variables: vec![],
                crc: 0,
            }],
            libraries: vec![Library {
                name: b"MsgSkip".to_vec(),
                functions: vec![HllFunction {
                    name: name.as_bytes().to_vec(),
                    return_kind: ret,
                    arguments: args,
                }],
            }],
            ..Default::default()
        }))
        .unwrap();
        if let Some(bytes) = filename {
            let slot = m
                .heap
                .alloc(Object::String(Arc::new(bytes.to_vec())))
                .unwrap();
            m.stack.push(slot);
        }
        if let Some(value) = value {
            m.stack.push(value);
        }
        for _ in 0..100 {
            if let Step::External(call) = m.step().unwrap() {
                return (m, call);
            }
        }
        panic!("missing original MsgSkip HLL")
    }
    fn call(
        skip: &mut NativeMsgSkip,
        files: &NativeFiles,
        rt: &mut Runtime,
        name: &str,
        value: Option<i32>,
        filename: Option<&[u8]>,
    ) -> Result<Vec<i32>> {
        let (mut m, call) = pending(name, value, filename, 5);
        let original = m.stack.clone();
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match skip.dispatch(ClientId(1), &mut m, &call, files, &mut rt.host)? {
                Some(Dispatch::Completed) => break,
                Some(Dispatch::WaitUntil(_)) => {
                    ensure!(
                        m.pc == call.pc && m.stack == original,
                        "MsgSkip acknowledged before actual IO"
                    );
                    ensure!(Instant::now() < until, "MsgSkip timeout");
                    std::thread::yield_now();
                }
                _ => anyhow::bail!("wrong MsgSkip dispatch"),
            }
        }
        for _ in 0..100 {
            if m.step()? == Step::Halted {
                break;
            }
        }
        ensure!(
            m.heap.live_slots() == 1,
            "MsgSkip leaked native string/local page"
        );
        Ok(m.stack)
    }
    fn flush(skip: &mut NativeMsgSkip, rt: &mut Runtime) -> Result<()> {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            match skip.flush(ClientId(1), &mut rt.host)? {
                Dispatch::Completed => return Ok(()),
                Dispatch::WaitUntil(_) => {
                    ensure!(Instant::now() < until, "read-history flush timeout");
                    std::thread::yield_now();
                }
                _ => anyhow::bail!("unexpected presentation"),
            }
        }
    }
    #[test]
    fn native_read_flags_preserve_msb_padding_count_and_real_use_flag_policy() {
        let (_root, mut rt) = runtime();
        let files = NativeFiles::new(ClientId(1)).unwrap();
        let mut skip = NativeMsgSkip::new(ClientId(1), files.io_pool());
        let path = VfsPath::new(Mount::Save, "Seen.asd").unwrap();
        rt.host
            .storage(ClientId(1))
            .unwrap()
            .write_atomic(&path, &[5, 0, 0, 0, 0x81])
            .unwrap();
        assert_eq!(
            call(
                &mut skip,
                &files,
                &mut rt,
                "Init",
                None,
                Some(b"SaveData\\Seen.asd")
            )
            .unwrap(),
            [1]
        );
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetNumofMsg", None, None).unwrap(),
            [5]
        );
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetNumofFlag", None, None).unwrap(),
            [2]
        );
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetFlag", Some(0), None).unwrap(),
            [1]
        );
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetFlag", Some(4), None).unwrap(),
            [0]
        );
        call(&mut skip, &files, &mut rt, "UseFlag", Some(0), None).unwrap();
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetFlag", Some(4), None).unwrap(),
            [1]
        );
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetFlag", Some(7), None).unwrap(),
            [0]
        );
        call(&mut skip, &files, &mut rt, "SetFlag", Some(2), None).unwrap();
        flush(&mut skip, &mut rt).unwrap();
        assert_eq!(
            rt.host.storage(ClientId(1)).unwrap().read(&path).unwrap(),
            [5, 0, 0, 0, 0x81]
        );
        call(&mut skip, &files, &mut rt, "UseFlag", Some(1), None).unwrap();
        flush(&mut skip, &mut rt).unwrap();
        assert_eq!(
            rt.host.storage(ClientId(1)).unwrap().read(&path).unwrap(),
            [5, 0, 0, 0, 0xa1]
        );
        call(&mut skip, &files, &mut rt, "SetState", Some(7), None).unwrap();
        call(&mut skip, &files, &mut rt, "SetEnable", Some(-1), None).unwrap();
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetState", None, None).unwrap(),
            [1]
        );
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetEnable", None, None).unwrap(),
            [1]
        );
    }
    #[test]
    fn native_reinit_flushes_previous_file_then_loads_new_and_bad_exact_header_is_reset() {
        let (_root, mut rt) = runtime();
        let files = NativeFiles::new(ClientId(1)).unwrap();
        let mut skip = NativeMsgSkip::new(ClientId(1), files.io_pool());
        call(&mut skip, &files, &mut rt, "Init", None, Some(b"First.asd")).unwrap();
        call(&mut skip, &files, &mut rt, "SetFlag", Some(1), None).unwrap();
        let second = VfsPath::new(Mount::Save, "Second.asd").unwrap();
        rt.host
            .storage(ClientId(1))
            .unwrap()
            .write_atomic(&second, &[6, 0, 0, 0, 0xff])
            .unwrap();
        call(
            &mut skip,
            &files,
            &mut rt,
            "Init",
            None,
            Some(b"Second.asd"),
        )
        .unwrap();
        assert_eq!(
            rt.host
                .storage(ClientId(1))
                .unwrap()
                .read(&VfsPath::new(Mount::Save, "First.asd").unwrap())
                .unwrap(),
            [5, 0, 0, 0, 0x40]
        );
        assert!(matches!(
            rt.host.storage(ClientId(1)).unwrap().read(&second),
            Err(Error::NotFound)
        ));
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetNumofFlag", None, None).unwrap(),
            [0]
        );
        flush(&mut skip, &mut rt).unwrap();
        assert_eq!(
            rt.host.storage(ClientId(1)).unwrap().read(&second).unwrap(),
            [5, 0, 0, 0, 0]
        );
    }
    #[test]
    fn original_read_only_flags_import_without_writing_game_data_and_foreign_init_never_acks() {
        let (root, mut rt) = runtime();
        std::fs::create_dir(root.path().join("assets/SaveData")).unwrap();
        let original = root.path().join("assets/SaveData/Old.asd");
        std::fs::write(&original, [5, 0, 0, 0, 8]).unwrap();
        let files = NativeFiles::new(ClientId(1)).unwrap();
        let mut skip = NativeMsgSkip::new(ClientId(1), files.io_pool());
        let (mut m, p) = pending("Init", None, Some(b"Old.asd"), 5);
        assert!(skip
            .dispatch(ClientId(2), &mut m, &p, &files, &mut rt.host)
            .is_err());
        assert_eq!(m.pc, p.pc);
        call(&mut skip, &files, &mut rt, "Init", None, Some(b"Old.asd")).unwrap();
        assert_eq!(
            call(&mut skip, &files, &mut rt, "GetFlag", Some(4), None).unwrap(),
            [1]
        );
        flush(&mut skip, &mut rt).unwrap();
        assert_eq!(std::fs::read(&original).unwrap(), [5, 0, 0, 0, 8]);
        assert_eq!(
            rt.host
                .storage(ClientId(1))
                .unwrap()
                .read(&VfsPath::new(Mount::Save, "Old.asd").unwrap())
                .unwrap(),
            [5, 0, 0, 0, 8]
        );
    }
    fn fixture_ain(end: u16) -> Vec<u8> {
        let mut code = Vec::new();
        let op = |code: &mut Vec<u8>, opcode: u16, args: &[i32]| {
            code.extend(opcode.to_le_bytes());
            for a in args {
                code.extend(a.to_le_bytes());
            }
        };
        op(&mut code, 0x41, &[0]);
        op(&mut code, 0x5a, &[0, 0]);
        op(&mut code, 1, &[]);
        op(&mut code, 0, &[1]);
        op(&mut code, 0x5a, &[0, 1]);
        if end == 0x63 {
            op(&mut code, 0, &[100000]);
            op(&mut code, 0x63, &[21]);
            op(&mut code, 0x2f, &[]);
        } else if end == 0x5a {
            op(&mut code, 0x5a, &[0, 2]);
            op(&mut code, 0x2f, &[]);
        } else {
            op(&mut code, end, &[]);
        }
        let mut out = b"VERS".to_vec();
        let word = |out: &mut Vec<u8>, n: u32| out.extend(n.to_le_bytes());
        word(&mut out, 5);
        out.extend(b"CODE");
        word(&mut out, code.len() as u32);
        out.extend(code);
        out.extend(b"FUNC");
        word(&mut out, 1);
        word(&mut out, 0);
        out.extend(b"main\0");
        for n in [0, 0, u32::MAX, 0, 0, 0] {
            word(&mut out, n)
        }
        out.extend(b"MAIN");
        word(&mut out, 0);
        out.extend(b"STR0");
        word(&mut out, 1);
        out.extend(b"SaveData\\Seen.asd\0");
        out.extend(b"MSG0");
        word(&mut out, 5);
        for _ in 0..5 {
            out.extend(b"authored\0")
        }
        out.extend(b"HLL0");
        word(&mut out, 1);
        out.extend(b"MsgSkip\0");
        word(&mut out, 3);
        out.extend(b"Init\0");
        word(&mut out, 10);
        word(&mut out, 1);
        out.extend(b"name\0");
        word(&mut out, 12);
        out.extend(b"SetFlag\0");
        word(&mut out, 0);
        word(&mut out, 1);
        out.extend(b"index\0");
        word(&mut out, 10);
        out.extend(b"AuthoredMissing\0");
        word(&mut out, 0);
        word(&mut out, 0);
        out
    }
    fn adapter(root: &std::path::Path, end: u16) -> NativeAdapter {
        std::fs::write(root.join("assets/test.ain"), fixture_ain(end)).unwrap();
        NativeAdapter::new(
            ClientId(1),
            Config {
                program: "test.ain".into(),
                ini: None,
                volumes: vec![],
                canvas: None,
                font_aliases: ["unused".into(), "unused".into()],
                reviewed_output_log_dll: None,
            },
        )
        .unwrap()
    }
    #[test]
    fn native_vm_halt_waits_for_actual_read_flag_atomic_commit() {
        let (root, mut rt) = runtime();
        let mut vm = adapter(root.path(), 0x2f);
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if vm.step(&mut rt.host, InputFrame::default(), 100).unwrap() == VmYield::Halted {
                break;
            }
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert_eq!(
            rt.host
                .storage(ClientId(1))
                .unwrap()
                .read(&VfsPath::new(Mount::Save, "Seen.asd").unwrap())
                .unwrap(),
            [5, 0, 0, 0, 0x40]
        );
    }
    #[test]
    fn close_saves_read_flags_without_advancing_sleep_deadline_or_acknowledging_wait() {
        let (root, mut rt) = runtime();
        let mut vm = adapter(root.path(), 0x63);
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            vm.step(&mut rt.host, InputFrame::default(), 100).unwrap();
            let d: serde_json::Value = serde_json::from_slice(&vm.diagnostics()).unwrap();
            if d["native_pending"]["target"] == "System(21)" {
                break;
            }
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        let before: serde_json::Value = serde_json::from_slice(&vm.diagnostics()).unwrap();
        assert!(vm.request_close());
        loop {
            if vm.step(&mut rt.host, InputFrame::default(), 100).unwrap() == VmYield::Halted {
                break;
            }
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        let after: serde_json::Value = serde_json::from_slice(&vm.diagnostics()).unwrap();
        assert_eq!(after["native_pending"], before["native_pending"]);
        assert_eq!(rt.host.now_ms(), 0);
        assert_eq!(
            rt.host
                .storage(ClientId(1))
                .unwrap()
                .read(&VfsPath::new(Mount::Save, "Seen.asd").unwrap())
                .unwrap(),
            [5, 0, 0, 0, 0x40]
        );
    }
    #[test]
    fn unsupported_native_call_is_exposed_after_read_history_commit_without_ack() {
        let (root, mut rt) = runtime();
        // An unknown layout is correctly rejected before startup validation,
        // before flags exist. This well-formed authored unknown export tests
        // the runtime/native error path after real initialization instead.
        let mut vm = adapter(root.path(), 0x5a);
        let until = Instant::now() + Duration::from_secs(5);
        let error = loop {
            match vm.step(&mut rt.host, InputFrame::default(), 100) {
                Err(error) => break error,
                _ => {
                    assert!(Instant::now() < until);
                    std::thread::yield_now();
                }
            }
        };
        assert!(error
            .to_string()
            .contains("unsupported original MsgSkip export AuthoredMissing"));
        assert_eq!(
            rt.host
                .storage(ClientId(1))
                .unwrap()
                .read(&VfsPath::new(Mount::Save, "Seen.asd").unwrap())
                .unwrap(),
            [5, 0, 0, 0, 0x40]
        );
        let d: serde_json::Value = serde_json::from_slice(&vm.diagnostics()).unwrap();
        assert_eq!(d["message_skip"]["exit_flushed"], true);
        assert_eq!(d["native_pending"]["function"], "AuthoredMissing");
    }
}
