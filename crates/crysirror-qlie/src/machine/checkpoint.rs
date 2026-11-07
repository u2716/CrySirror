//! Logical reader checkpoint. This payload never contains GPU, audio-device
//! or host pointers. Immutable project programs remain in the current session.
use super::*;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    version: u32,
    game: ExecutionContext,
    globals: BTreeMap<String, Value>,
    object_types: BTreeMap<String, String>,
    arrays: BTreeMap<String, usize>,
    array_types: BTreeMap<String, String>,
    #[serde(default)]
    array_shapes: BTreeMap<String, Vec<usize>>,
    next_object: u32,
    next_transient: u64,
    programs: BTreeMap<String, Program>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SharedBank {
    values: BTreeMap<String, Value>,
    arrays: BTreeMap<String, usize>,
    array_types: BTreeMap<String, String>,
    array_shapes: BTreeMap<String, Vec<usize>>,
}
impl Machine {
    /// A fresh logical identity, used to rename restored saved objects.
    pub(crate) fn allocate_object_identity(&mut self) -> Result<String> {
        self.next_object = self
            .next_object
            .checked_add(1)
            .context("QLIE object identity exhausted")?;
        Ok(format!("object/{}", self.next_object))
    }
    /// The svar calculator bank persisted by SystemData (6C0F64/6C0EB0).
    pub(crate) fn shared_bank_state(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(SharedBank {
            values: self.shared_bank.values.clone(),
            arrays: self.shared_bank.arrays.clone(),
            array_types: self.shared_bank.array_types.clone(),
            array_shapes: self.shared_bank.array_shapes.clone(),
        })?)
    }
    /// sdat\i (6C0E74): frees every svar of the calculator bank.
    /// 707A30: fresh readers and banks except the shared svar bank, then a
    /// jump to `label` in `file`.
    pub(crate) fn reset_to(&mut self, file: &str, label: &str) -> Result<()> {
        let entry = self.frame.reader_path.clone();
        let mut fresh = Machine::new(self.programs.clone(), "startup.s")
            .or_else(|_| Machine::new(self.programs.clone(), &entry))?;
        fresh.shared_bank = std::mem::take(&mut self.shared_bank);
        fresh.next_object = self.next_object;
        *self = fresh;
        self.jump(label, Some(file), None, true)
    }
    pub(crate) fn clear_shared_bank(&mut self) {
        self.shared_bank = Default::default();
    }
    /// Loaded before startup.s: later svar declarations keep these values.
    pub(crate) fn restore_shared_bank(&mut self, state: serde_json::Value) -> Result<()> {
        let bank: SharedBank = serde_json::from_value(state)?;
        ensure!(
            bank.values.len() <= 131072 && bank.arrays.values().all(|n| *n <= 65536),
            "QLIE SystemData value budget"
        );
        for (name, shape) in &bank.array_shapes {
            ensure!(
                (1..=4).contains(&shape.len())
                    && bank.arrays.get(name) == Some(&Self::array_element_count(shape)?),
                "QLIE SystemData array shape"
            );
        }
        // Object identities are session-local; a persisted svar can only
        // retain scalar/string values, never a live object reference.
        for value in bank.values.values() {
            if let Value::Text(s) = value {
                ensure!(!s.starts_with("object/"), "QLIE SystemData object reference");
            }
        }
        self.shared_bank.values = bank.values;
        self.shared_bank.arrays = bank.arrays;
        self.shared_bank.array_types = bank.array_types;
        self.shared_bank.array_shapes = bank.array_shapes;
        Ok(())
    }
    pub(crate) fn game_checkpoint(&self, advance: bool) -> Result<Vec<u8>> {
        let mut game = match self.thread_kind() {
            ThreadKind::System => self
                .suspended_game
                .clone()
                .context("QLIE no paused game checkpoint")?,
            ThreadKind::Game => ExecutionContext {
                frame: self.frame.clone(),
                stack: self.stack.clone(),
                scope: self.scope.clone(),
                transient: self.transient.clone(),
            },
        };
        if advance && self.thread_kind() == ThreadKind::Game {
            game.frame.pc = game
                .frame
                .pc
                .checked_add(1)
                .context("QLIE checkpoint resume PC overflow")?;
        }
        let programs = game
            .transient
            .iter()
            .map(|path| {
                Ok((
                    path.clone(),
                    self.programs
                        .get(path)
                        .context("QLIE checkpoint transient program absent")?
                        .as_ref()
                        .clone(),
                ))
            })
            .collect::<Result<_>>()?;
        let checkpoint = Checkpoint {
            version: 1,
            game,
            globals: self.globals.clone(),
            object_types: self.object_types.clone(),
            arrays: self.arrays.clone(),
            array_types: self.array_types.clone(),
            array_shapes: self.array_shapes.clone(),
            next_object: self.next_object,
            next_transient: self.next_transient,
            programs,
        };
        let bytes = serde_json::to_vec(&checkpoint)?;
        ensure!(
            bytes.len() <= 32 * 1024 * 1024,
            "QLIE logical checkpoint byte budget"
        );
        Ok(bytes)
    }
    /// Restore GameThread without replacing a SystemThread load UI's stack.
    /// Its thrd,0 subsequently resumes the restored game continuation.
    pub(crate) fn restore_game_checkpoint(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(
            bytes.len() <= 32 * 1024 * 1024,
            "QLIE logical checkpoint byte budget"
        );
        let checkpoint: Checkpoint = serde_json::from_slice(bytes)?;
        ensure!(
            checkpoint.version == 1,
            "unsupported QLIE logical checkpoint version"
        );
        ensure!(
            checkpoint.game.frame.thread == ThreadKind::Game && checkpoint.game.stack.len() <= 1024,
            "QLIE logical checkpoint game context"
        );
        ensure!(
            checkpoint.globals.len() <= 1_000_000
                && checkpoint.object_types.len() <= 65536
                && checkpoint.arrays.values().all(|v| *v <= 65536),
            "QLIE logical checkpoint value budget"
        );
        ensure!(
            checkpoint.programs.len() <= 1024,
            "QLIE logical checkpoint transient program budget"
        );
        for (name, shape) in &checkpoint.array_shapes {
            ensure!(
                (1..=4).contains(&shape.len())
                    && shape.iter().all(|v| (1..=65536).contains(v))
                    && checkpoint.arrays.get(name) == Some(&Self::array_element_count(shape)?),
                "QLIE checkpoint array shape"
            );
        }
        let mut programs = self.programs.clone();
        for (path, program) in &checkpoint.programs {
            ensure!(
                checkpoint.game.transient.contains(path)
                    && program.path == *path
                    && program.code.len() <= 1_000_000,
                "QLIE logical checkpoint transient identity"
            );
            programs.insert(path.clone(), Arc::new(program.clone()));
        }
        for frame in std::iter::once(&checkpoint.game.frame).chain(checkpoint.game.stack.iter()) {
            let program = programs
                .get(&frame.path)
                .context("QLIE saved program absent")?;
            ensure!(
                frame.pc <= program.code.len()
                    && frame.locals.len() <= 65536
                    && frame.selection.len() <= 16
                    && frame
                        .call_results
                        .as_ref()
                        .is_none_or(|values| values.len() <= 128)
                    && frame.arrays.values().all(|v| *v <= 65536),
                "QLIE saved reader bounds"
            );
            for (name, shape) in &frame.array_shapes {
                ensure!(
                    (1..=4).contains(&shape.len())
                        && shape.iter().all(|v| (1..=65536).contains(v))
                        && frame.arrays.get(name) == Some(&Self::array_element_count(shape)?),
                    "QLIE checkpoint local array shape"
                );
            }
            ensure!(
                frame.loops.iter().all(|pc| *pc < program.code.len())
                    && frame
                        .cases
                        .iter()
                        .all(|(_, _, pc)| *pc > 0 && *pc <= program.code.len()),
                "QLIE saved control bounds"
            );
        }
        self.programs = programs;
        self.globals = checkpoint.globals;
        // The paused game's restore does not discard current System gvar or
        // shared svar object references. Logical identities are monotonic;
        // canonical collection retires any old Game identities no longer held.
        self.object_types.extend(checkpoint.object_types);
        self.arrays = checkpoint.arrays;
        self.array_types = checkpoint.array_types;
        self.array_shapes = checkpoint.array_shapes;
        // System UI IDs cannot collide with objects allocated after loading
        // an older game continuation.
        self.next_object = self.next_object.max(checkpoint.next_object);
        self.next_transient = self.next_transient.max(checkpoint.next_transient);
        if self.thread_kind() == ThreadKind::System {
            self.suspended_game = Some(checkpoint.game);
        } else {
            self.frame = checkpoint.game.frame;
            self.stack = checkpoint.game.stack;
            self.scope = checkpoint.game.scope;
            self.transient = checkpoint.game.transient;
        }
        Ok(())
    }
}
