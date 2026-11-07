// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Reviewed Mugen2 legacy save records. This is the original native binary
//! envelope, not the framework snapshot format. Opaque status/effect sections
//! retain their bytes; full VM/resource restoration requires separate mapping.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, StorageWrite},
};
use std::sync::Arc;
mod fresh;
mod live;
pub(crate) use live::LiveExecution;
const HEADER: usize = 32;
const LIMIT: usize = 64 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveBlock {
    pub code: u32,
    pub label: [u8; 16],
    pub payload: Vec<u8>,
}
impl SaveBlock {
    pub fn new(code: u32, label: &[u8], payload: Vec<u8>) -> Result<Self> {
        ensure!(
            label.len() <= 15 && !label.contains(&0),
            "NNN native save label"
        );
        let mut out = [0; 16];
        out[..label.len()].copy_from_slice(label);
        Ok(Self {
            code,
            label: out,
            payload,
        })
    }
    fn word(&self, index: usize) -> Result<u32> {
        let at = index.checked_mul(4).context("NNN save word overflow")?;
        Ok(u32::from_le_bytes(
            self.payload
                .get(at..at + 4)
                .context("NNN save word bounds")?
                .try_into()?,
        ))
    }
}
#[derive(Clone, Debug)]
pub struct LegacySaveProfile {
    pub header_name: Vec<u8>,
    pub version: u32,
    pub extension_sizes: Vec<u32>,
    pub omake: bool,
    pub cutin: bool,
}
impl LegacySaveProfile {
    /// Compute this reviewed native envelope policy on a preparation worker.
    /// The original game configuration owns identity/version/declared sizes;
    /// do not infer a profile from whichever untrusted save happens to arrive.
    pub(crate) fn for_configuration(
        config: &crate::native_bootstrap::NativeConfiguration,
    ) -> Result<Self> {
        ensure!(
            config.module_sha256 == crate::native_bootstrap::MUGEN2_MODULE_SHA256
                && config.profile == crate::expression::VariableProfile::Legacy1000,
            "NNN original save unreviewed native module/variable layout"
        );
        let setup = &config.setup;
        let header_name = setup
            .parameter(b"saveHeaderName")
            .unwrap_or(b"SAVEDATA")
            .to_vec();
        ensure!(
            header_name.len() <= 128 && !header_name.contains(&0),
            "NNN native save header name"
        );
        let version = [b"highVersion".as_slice(), b"middleVersion", b"lowVersion"]
            .map(|key| setup.integer(key).map(|value| value.unwrap_or(0)));
        let [high, middle, low] = version;
        let version =
            (high? as u32).wrapping_shl(16) | (middle? as u32).wrapping_shl(8) | (low? as u32);
        // Actual legacy constructor only has autoExtDataMode, not newer
        // source autoExtDataSub/face/okikae auto classes. Its mode sizes come
        // from the real mode actor; do not replace those consumers with0.
        for index in 1..=256 {
            let key = format!("autoExtDataMode{index}");
            let Some(name) = setup.parameter(key.as_bytes()) else {
                break;
            };
            let mode = crate::native_mode::number(name)?;
            ensure!(
                mode <= 0,
                "NNN native auto-extension mode {mode} requires its original state-size/data consumer"
            );
        }
        let count = usize::try_from(setup.integer(b"extSaveDataNumber")?.unwrap_or(0))
            .context("NNN native negative save extension count")?;
        ensure!(count <= 256, "NNN save extension count budget");
        let mut extension_sizes = Vec::with_capacity(count);
        let mut total = 0usize;
        for index in 1..=count {
            let key = format!("extSaveDataSize{index}");
            let bytes = u32::try_from(setup.integer(key.as_bytes())?.unwrap_or(0))
                .context("NNN native negative save extension size")?;
            total = total
                .checked_add(bytes as usize)
                .context("NNN native extension byte overflow")?;
            ensure!(total <= LIMIT, "NNN save extension byte budget");
            extension_sizes.push(bytes);
        }
        Ok(Self {
            header_name,
            version,
            extension_sizes,
            omake: setup.integer(b"useExtClass")?.unwrap_or(0) != 0,
            cutin: setup.integer(b"useCutinFlag")?.unwrap_or(0) != 0,
        })
    }
}
#[derive(Clone, Debug)]
pub struct NativeSaveFile {
    pub blocks: Vec<SaveBlock>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeSavePosition {
    pub run_mode: i32,
    pub script_number: i32,
    pub script_sub_number: i32,
    pub script_sub_sub_number: i32,
    pub last_message_id: i32,
    pub message_rows: u32,
    pub message_mode: u32,
    pub last_select_id: i32,
    /// Original total selection rows, including supplemental message rows.
    pub choice_rows: u32,
    pub extra_rows: u32,
    pub music_number: i32,
    pub music_repeat_count: i32,
}
impl NativeSaveFile {
    pub fn parse(bytes: &[u8], profile: &LegacySaveProfile) -> Result<Self> {
        ensure!(
            (96..=LIMIT).contains(&bytes.len()),
            "NNN native save byte budget"
        );
        let mut blocks = Vec::new();
        let mut at = 0;
        while at < bytes.len() {
            ensure!(blocks.len() < 1024, "NNN native save block budget");
            let head = bytes
                .get(at..at + HEADER)
                .context("NNN truncated native save block header")?;
            let size = u32::from_le_bytes(head[..4].try_into()?) as usize;
            ensure!(
                size >= HEADER && size <= LIMIT,
                "NNN native save block length"
            );
            let end = at
                .checked_add(size)
                .context("NNN native save block overflow")?;
            let raw = bytes
                .get(at..end)
                .context("NNN truncated native save block")?;
            ensure!(
                head[8..16] == [0; 8],
                "NNN unknown native save header flags"
            );
            let code = u32::from_le_bytes(head[4..8].try_into()?);
            let label = head[16..32].try_into()?;
            blocks.push(SaveBlock {
                code,
                label,
                payload: raw[HEADER..].to_vec(),
            });
            at = end;
        }
        let out = Self { blocks };
        out.validate(profile)?;
        Ok(out)
    }
    pub fn validate(&self, profile: &LegacySaveProfile) -> Result<()> {
        ensure!(
            profile.header_name.len() <= 128 && profile.extension_sizes.len() <= 256,
            "NNN save profile budget"
        );
        let n = 8
            + usize::from(profile.omake)
            + usize::from(profile.cutin)
            + profile.extension_sizes.len();
        ensure!(self.blocks.len() == n, "NNN native save block count");
        let header = &self.blocks[0];
        ensure!(
            header.code == 0 && header.payload.is_empty(),
            "NNN native save HEADER block"
        );
        let mut expected = [0; 16];
        let name = &profile.header_name[..profile.header_name.len().min(15)];
        expected[..name.len()].copy_from_slice(name);
        ensure!(
            header.label == expected,
            "NNN original save belongs to another game"
        );
        let info = &self.blocks[1];
        ensure!(
            info.code == 1 && info.payload.len() == 32,
            "NNN legacy save INFO size"
        );
        ensure!(
            info.word(0)? as usize == n
                // Actual Mugen2 4d9ae0 accepts older same-high versions.
                // Later source's middle-version/message restriction is absent.
                && info.word(1)? <= profile.version
                && ((info.word(1)? >> 16)&255)>=((profile.version>>16)&255)
                && info.word(2)? as usize == profile.extension_sizes.len()
                && info.word(3)? == u32::from(profile.omake)
                && info.word(4)? == u32::from(profile.cutin),
            "NNN native save version/extension profile"
        );
        // These sizes come from complete legacy Save* static functions. Modern
        // GAMEMESSAGE is larger and must not be substituted for this ABI.
        for (at, code, size) in [
            (2, 2, 7088),
            (4, 4, 4128),
            (5, 5, 16416),
            (6, 6, 4128),
            (7, 7, 16416),
        ] {
            ensure!(
                self.blocks[at].code == code && self.blocks[at].payload.len() + HEADER == size,
                "NNN legacy save section {code} size/order"
            );
        }
        let mini = &self.blocks[3];
        ensure!(
            mini.code == 3 && mini.payload.len() >= 16,
            "NNN save MINICG header"
        );
        let (w, h) = (mini.word(0)?, mini.word(1)?);
        ensure!(
            w > 0 && h > 0 && w <= 8192 && h <= 8192,
            "NNN native save thumbnail dimensions"
        );
        ensure!(
            u64::from(w) * u64::from(h) * 4 + 16 == mini.payload.len() as u64,
            "NNN native save thumbnail length"
        );
        let mut at = 8;
        if profile.omake {
            ensure!(
                self.blocks[at].code == 8 && self.blocks[at].payload.len() + HEADER == 12832,
                "NNN legacy OMAKE block"
            );
            at += 1;
        }
        if profile.cutin {
            ensure!(
                self.blocks[at].code == 9 && self.blocks[at].payload.len() + HEADER == 1056,
                "NNN legacy CUTIN block"
            );
            at += 1;
        }
        for (i, size) in profile.extension_sizes.iter().enumerate() {
            let b = &self.blocks[at + i];
            ensure!(
                b.code == 100 + i as u32 && b.payload.len() == *size as usize,
                "NNN native save EXT block"
            );
        }
        let p = self.position()?;
        ensure!(
            p.message_rows <= 64 && p.choice_rows <= 64 && p.extra_rows <= p.choice_rows,
            "NNN saved message count"
        );
        Ok(())
    }
    /// Native variable callback writes exactly1000 values, although the record
    /// reserves1024 slots. The remaining96 padding bytes are preserved.
    pub fn variables(&self) -> Result<Vec<i32>> {
        let b = self.blocks.get(4).context("NNN variable block absent")?;
        ensure!(
            b.code == 4 && b.payload.len() == 4096,
            "NNN legacy variable record"
        );
        (0..1000).map(|i| Ok(b.word(i)? as i32)).collect()
    }
    pub fn replace_variables(&mut self, values: &[i32]) -> Result<()> {
        ensure!(
            values.len() == 1000,
            "NNN legacy save needs1000 native variables"
        );
        let b = self
            .blocks
            .get_mut(4)
            .context("NNN variable block absent")?;
        ensure!(
            b.code == 4 && b.payload.len() == 4096,
            "NNN legacy variable record"
        );
        for (i, v) in values.iter().enumerate() {
            b.payload[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        Ok(())
    }
    pub fn position(&self) -> Result<NativeSavePosition> {
        let b = self.blocks.get(2).context("NNN status block absent")?;
        ensure!(
            b.code == 2 && b.payload.len() == 7056,
            "NNN legacy status record"
        );
        let word = |native: usize| b.word(native - 8);
        Ok(NativeSavePosition {
            run_mode: word(24)? as i32,
            script_number: word(26)? as i32,
            script_sub_number: word(27)? as i32,
            script_sub_sub_number: word(31)? as i32,
            last_message_id: word(28)? as i32,
            message_rows: word(29)?,
            message_mode: word(30)?,
            last_select_id: word(32)? as i32,
            choice_rows: word(33)?,
            extra_rows: word(34)?,
            music_number: word(36)? as i32,
            music_repeat_count: word(37)? as i32,
        })
    }
    pub fn messages(&self, count: usize) -> Result<Vec<Vec<u8>>> {
        ensure!(count <= 64, "NNN saved message row count");
        let b = self.blocks.get(7).context("NNN message block absent")?;
        ensure!(
            b.code == 7 && b.payload.len() == 16384,
            "NNN legacy message record"
        );
        b.payload
            .chunks_exact(256)
            .take(count)
            .map(|row| {
                let len = row
                    .iter()
                    .position(|v| *v == 0)
                    .context("NNN unterminated saved message")?;
                ensure!(
                    len <= 254 && row[len + 1] == 0,
                    "NNN saved message native double-NUL"
                );
                Ok(row[..len].to_vec())
            })
            .collect()
    }
    /// Serialize only after exact original profile validation. This preserves
    /// untouched fields; it cannot manufacture missing VM/effect/save state.
    pub fn encode(&self, profile: &LegacySaveProfile) -> Result<Vec<u8>> {
        self.validate(profile)?;
        let mut out = Vec::new();
        for b in &self.blocks {
            let size = b
                .payload
                .len()
                .checked_add(HEADER)
                .context("NNN save block size")?;
            ensure!(out.len() + size <= LIMIT, "NNN native save byte budget");
            out.extend(u32::try_from(size)?.to_le_bytes());
            out.extend(b.code.to_le_bytes());
            out.extend([0; 8]);
            out.extend(b.label);
            out.extend(&b.payload);
        }
        Ok(out)
    }
    /// Admission on capability storage; the returned completion must become
    /// Ready before native save UI/VM policy can acknowledge the write.
    pub fn begin_write(
        &self,
        profile: &LegacySaveProfile,
        storage: &dyn ClientStorage,
        slot: u32,
        trial: bool,
    ) -> Result<Box<dyn StorageWrite>> {
        ensure!(slot < 10000, "NNN original save slot budget");
        let name = format!("{}data{slot}.sav", if trial { "t" } else { "" });
        Ok(storage.begin_write_atomic(
            &VfsPath::new(Mount::Save, &name)?,
            Arc::from(self.encode(profile)?),
        )?)
    }
}

/// Native saved execution metadata only. Original images/effects/text/audio
/// must be prepared and validated separately before a live import is admitted.
#[derive(Clone, Debug)]
struct SavedContext {
    pc: u32,
    depth: usize,
    calls: [u32; 255],
}
fn continuation_next(program: &crate::Program, table: usize, id: u32) -> Result<u32> {
    let tables = program
        .call_tables
        .context("NNN saved continuation table absent")?;
    let entry = usize::try_from(tables[table])?
        .checked_add(usize::try_from(id)?)
        .context("NNN saved continuation index overflow")?;
    let target = *program
        .words
        .get(entry)
        .context("NNN saved continuation table bounds")?;
    let record = program.instruction(target)?;
    let expected = match table {
        0 => record.identify == 0x66660002 && record.code == 0x55560009,
        2 => record.identify == 0x66660003,
        3 => record.identify == 0x66660002 && matches!(record.code, 0x5556000a | 0x55560016),
        _ => false,
    };
    ensure!(
        expected,
        "NNN native continuation table does not identify its original instruction family"
    );
    crate::control::target(program, record.next)
}
impl SavedContext {
    fn prepare(
        &self,
        program: &crate::Program,
        variables: crate::expression::Variables,
        adjusted: bool,
        parent: bool,
        game_mode: i32,
        cancel: &crysirror_tasks::Cancellation,
    ) -> Result<crate::machine::Machine> {
        cancel.check()?;
        let mut returns = Vec::with_capacity(self.depth);
        if adjusted {
            for &id in self.calls.iter().take(self.depth) {
                cancel.check()?;
                returns.push(crate::machine::ReturnFrame {
                    pc: continuation_next(program, 0, id)?,
                    native_id: id,
                });
            }
        } else if self.depth > 0 {
            // One bounded pass reconstructs native CALL IDs from raw return
            // addresses, without executing the original bytecode or rescanning
            // the whole program once per stack entry.
            let requested: std::collections::BTreeSet<_> =
                self.calls[..self.depth].iter().copied().collect();
            let last = *requested.last().unwrap();
            ensure!(
                last as usize <= program.word_count(),
                "NNN saved raw return address bounds"
            );
            let mut found = std::collections::BTreeMap::new();
            let mut pc = 0;
            let mut records = 0usize;
            while pc < last {
                cancel.check()?;
                records += 1;
                ensure!(records <= 1_000_000, "NNN saved return scan budget");
                let record = program.instruction(pc)?;
                if requested.contains(&record.next) {
                    ensure!(
                        record.identify == 0x66660002 && record.code == 0x55560009,
                        "NNN saved raw return does not follow an original CALL"
                    );
                    found.insert(
                        record.next,
                        *record
                            .operands
                            .get(1)
                            .context("NNN saved CALL native ID absent")?,
                    );
                }
                pc = record.next;
            }
            for &address in self.calls.iter().take(self.depth) {
                returns.push(crate::machine::ReturnFrame {
                    pc: address,
                    native_id: *found
                        .get(&address)
                        .context("NNN saved return is not an instruction boundary")?,
                });
            }
        }
        let pc = if adjusted && parent {
            continuation_next(program, 3, self.calls[253])?
        } else if adjusted && !matches!(game_mode, 4 | 9 | 10) {
            continuation_next(program, 2, self.calls[254])?
        } else {
            self.pc
        };
        crate::machine::Machine::from_native_saved(
            program,
            pc,
            returns,
            [Some(self.calls[253]), Some(self.calls[254])],
            variables,
        )
    }
}
impl NativeSaveFile {
    /// Original SetVarByLoad preserves live persistent200..299. A detached
    /// candidate also keeps current native RNG/calculated bindings; a game
    /// slot is not a framework snapshot of those global services.
    pub(crate) fn prepare_stored_variables(
        &self,
        current: &crate::expression::Variables,
    ) -> Result<crate::expression::Variables> {
        ensure!(
            current.stored_word_count() == 1000,
            "NNN saved/current variable bank ABI mismatch"
        );
        let saved = self.variables()?;
        let mut out = current.clone();
        for index in (0..200).chain(300..1000) {
            out.write_stored(index as u16, saved[index]);
        }
        out.finish_native_instruction();
        Ok(out)
    }
    /// Worker-only four-context restore preparation. This does not replace a
    /// live adapter, acknowledge a receipt, publish an image or claim complete
    /// original save support. The caller still needs an atomic resource plan.
    pub(crate) fn prepare_script_contexts(
        &self,
        profile: &LegacySaveProfile,
        configuration: &crate::native_bootstrap::NativeConfiguration,
        current: &crate::expression::Variables,
        mut load: impl FnMut(&[u8], &crysirror_tasks::Cancellation) -> Result<Arc<crate::Program>>,
        cancel: &crysirror_tasks::Cancellation,
    ) -> Result<crate::native_control::ScriptSession> {
        cancel.check()?;
        self.validate(profile)?;
        ensure!(
            configuration.profile == crate::expression::VariableProfile::Legacy1000,
            "NNN native saved variable ABI"
        );
        let position = self.position()?;
        ensure!(
            (0..=3).contains(&position.run_mode),
            "NNN saved active script context bounds"
        );
        let status = &self.blocks[2];
        let word = |native: usize| status.word(native - 8);
        let adjusted = word(22)?;
        ensure!(adjusted <= 1, "NNN unknown saved script-adjust version");
        let game_mode = word(20)? as i32;
        let variables = self.prepare_stored_variables(current)?;
        let script_indices = [
            None,
            Some(position.script_number),
            Some(position.script_sub_number),
            Some(position.script_sub_sub_number),
        ];
        let mut contexts = Vec::with_capacity(position.run_mode as usize + 1);
        for context in 0..=position.run_mode as usize {
            cancel.check()?;
            let name = if context == 0 {
                b"sys".to_vec()
            } else {
                configuration
                    .project
                    .tokens
                    .get(usize::try_from(script_indices[context].unwrap())?)
                    .context("NNN saved script number absent from original project")?
                    .clone()
            };
            configuration.resources.script(&name)?;
            let stack_word = 488 + 256 * context;
            let depth = word(stack_word)?;
            ensure!(depth <= 249, "NNN saved native subroutine stack exceeds249");
            let mut calls = [0u32; 255];
            for (index, value) in calls.iter_mut().enumerate() {
                *value = word(stack_word + 1 + index)?;
            }
            let native = SavedContext {
                pc: word(472 + context)?,
                depth: depth as usize,
                calls,
            };
            contexts.push((name, native));
        }
        // Check all fixed native fields/names before authorizing any loader.
        // A corrupt later context cannot trigger unrelated earlier resource I/O.
        let mut frames = Vec::with_capacity(contexts.len());
        for (context, (name, native)) in contexts.into_iter().enumerate() {
            cancel.check()?;
            let program = load(&name, cancel)?;
            cancel.check()?;
            let machine = native.prepare(
                &program,
                variables.clone(),
                adjusted == 1,
                context < position.run_mode as usize,
                game_mode,
                cancel,
            )?;
            frames.push((name, program, machine));
        }
        crate::native_control::ScriptSession::from_saved_contexts(frames)
    }
}

#[cfg(test)]
mod script_context_tests {
    use super::*;
    use crate::{
        Program, XorProfile,
        config::Names,
        expression::{VariableProfile, Variables},
        machine::{Payload, Step},
        native_bootstrap::NativeConfiguration,
        native_resources::ResourceNames,
    };
    fn program() -> Arc<Program> {
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend([5, 0x66660002, 0x55560009, 0, 0]); //32 CALLSUB0,id0 ->37
        words.extend([5, 0x66660002, 0x5556000a, 0, 0]); //37 SCRIPT a,id0 ->42
        words.extend([5, 0x66660003, 7, 0, 0]); //42 command7,noargs,id0 ->47
        words.extend([5, 0x66660007, 0x33330052, 1, 0]); //47 ->52
        words.extend([3, 0x66660002, 0x55560005]); //52 END
        words.extend([3, 0x66660002, 0x55560007]); //55 RETURN
        words.extend([55, 32, 42, 37, 63, u32::from_le_bytes(*b"a\0\0\0")]);
        words[8] = 1;
        words[9] = 58;
        words[20] = 59;
        words[22] = 60;
        words[23] = 61;
        words[6] = 1;
        words[7] = 62;
        words[18] = 64;
        let encoded: Vec<_> = words
            .into_iter()
            .flat_map(|word| (!word).to_le_bytes())
            .collect();
        Arc::new(Program::parse(&encoded, XorProfile::default()).unwrap())
    }
    fn fixture() -> (
        NativeSaveFile,
        LegacySaveProfile,
        NativeConfiguration,
        Variables,
    ) {
        let profile = LegacySaveProfile {
            header_name: b"context-fixture".to_vec(),
            version: 1,
            extension_sizes: vec![],
            omake: false,
            cutin: false,
        };
        let mut info = vec![0u8; 32];
        info[..4].copy_from_slice(&8u32.to_le_bytes());
        info[4..8].copy_from_slice(&1u32.to_le_bytes());
        let mut mini = vec![0; 20];
        mini[..4].copy_from_slice(&1u32.to_le_bytes());
        mini[4..8].copy_from_slice(&1u32.to_le_bytes());
        let mut file = NativeSaveFile {
            blocks: vec![
                SaveBlock::new(0, b"context-fixture", vec![]).unwrap(),
                SaveBlock::new(1, b"INFO", info).unwrap(),
                SaveBlock::new(2, b"GAMESTATUS", vec![0; 7056]).unwrap(),
                SaveBlock::new(3, b"MINICG", mini).unwrap(),
                SaveBlock::new(4, b"GAMEVAR", vec![0; 4096]).unwrap(),
                SaveBlock::new(5, b"EFFECT", vec![0; 16384]).unwrap(),
                SaveBlock::new(6, b"EFFECTFILENAME", vec![0; 4096]).unwrap(),
                SaveBlock::new(7, b"MESSAGE", vec![0; 16384]).unwrap(),
            ],
        };
        status(&mut file, 20, 10);
        for context in 0..4 {
            status(&mut file, 472 + context, 47);
        }
        let empty = Names::parse(b"", false).unwrap();
        let cfg = NativeConfiguration {
            resources: ResourceNames::new("").unwrap(),
            setup: empty.clone(),
            main_control: None,
            variable_names: empty.clone(),
            project: Names::parse(b"a b c", false).unwrap(),
            commands: empty.clone(),
            functions: empty,
            initial: None,
            module_sha256: "authored-reviewed-legacy".into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        };
        let mut current = Variables::fresh(VariableProfile::Legacy1000);
        current.write_stored(200, 99);
        current.write_stored(299, 51);
        current
            .replace_random_state(crysirror_image::temporal_pattern::PatternRng::msvc(123))
            .unwrap();
        let mut saved = vec![0; 1000];
        saved[0] = 10;
        saved[200] = -1;
        saved[299] = -2;
        saved[300] = 333;
        saved[999] = 444;
        file.replace_variables(&saved).unwrap();
        (file, profile, cfg, current)
    }
    fn status(file: &mut NativeSaveFile, native: usize, value: u32) {
        let at = (native - 8) * 4;
        file.blocks[2].payload[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    #[test]
    fn adjusted_four_contexts_recover_native_return_and_parent_script_ids_without_losing_persistent_bank()
     {
        let (mut file, profile, cfg, current) = fixture();
        status(&mut file, 22, 1);
        status(&mut file, 24, 3);
        status(&mut file, 26, 0);
        status(&mut file, 27, 1);
        status(&mut file, 31, 2);
        for context in 0..4 {
            status(&mut file, 488 + 256 * context, 1);
            status(&mut file, 472 + context, u32::MAX);
        }
        status(&mut file, 475, 47); // Parent rawPCs are obsolete; ID adjustment owns their destinations.
        let before = file.encode(&profile).unwrap();
        let program = program();
        let mut names = vec![];
        let mut restored = file
            .prepare_script_contexts(
                &profile,
                &cfg,
                &current,
                |name, _| {
                    names.push(name.to_vec());
                    Ok(program.clone())
                },
                &Default::default(),
            )
            .unwrap();
        assert_eq!(
            names,
            [b"sys".to_vec(), b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]
        );
        assert_eq!(restored.depth(), 3);
        let contexts: Vec<_> = restored.contexts_for_save().collect();
        for (_, _, machine) in &contexts[..3] {
            assert_eq!(machine.pc(), 42);
        }
        for (_, _, machine) in &contexts {
            assert_eq!(machine.return_frames()[0].pc, 37);
            assert_eq!(machine.return_frames()[0].native_id, 0);
        }
        let values = restored.machine().variables().unwrap();
        assert_eq!(
            (
                values.read_stored(0),
                values.read_stored(200),
                values.read_stored(299),
                values.read_stored(300),
                values.read_stored(999)
            ),
            (10, 99, 51, 333, 444)
        );
        assert_eq!(current.read_stored(0), 0);
        assert_eq!(
            values.random_state(),
            current.random_state(),
            "original game slots do not rewind the current global RNG"
        );
        assert_eq!(file.encode(&profile).unwrap(), before);
        let Step::Native(call) = restored.poll(1).unwrap() else {
            panic!()
        };
        assert_eq!(call.pc, 47);
        assert_eq!(call.code, 0x33330052);
    }
    #[test]
    fn raw_returns_recover_call_ids_and_typed_ids_advance_only_on_original_acknowledgement() {
        let (mut file, profile, cfg, current) = fixture();
        status(&mut file, 488, 1);
        status(&mut file, 489, 37);
        status(&mut file, 489 + 253, 7);
        status(&mut file, 489 + 254, 8);
        status(&mut file, 472, 37);
        let program = program();
        let mut restored = file
            .prepare_script_contexts(
                &profile,
                &cfg,
                &current,
                |_, _| Ok(program.clone()),
                &Default::default(),
            )
            .unwrap();
        let machine = restored.machine();
        assert_eq!(machine.return_frames()[0].native_id, 0);
        assert_eq!(machine.native_continuations(), [Some(7), Some(8)]);
        let Step::Native(script) = restored.poll(1).unwrap() else {
            panic!()
        };
        assert!(matches!(
            script.payload,
            Payload::Control {
                resume_id: Some(0),
                ..
            }
        ));
        assert_eq!(
            restored.machine().native_continuations(),
            [Some(7), Some(8)]
        );
        restored.acknowledge(&script).unwrap();
        assert_eq!(
            restored.machine().native_continuations(),
            [Some(0), Some(8)]
        );
        let Step::Native(command) = restored.poll(1).unwrap() else {
            panic!()
        };
        assert!(matches!(
            command.payload,
            Payload::UserCall {
                resume_id: Some(0),
                ..
            }
        ));
        restored.acknowledge(&command).unwrap();
        assert_eq!(
            restored.machine().native_continuations(),
            [Some(0), Some(0)]
        );
    }
    #[test]
    fn corrupt_or_cancelled_context_preparation_never_mutates_live_variables_or_save_bytes() {
        let (mut file, profile, cfg, current) = fixture();
        let program = program();
        for (field, value) in [(24, 4), (488, 250), (22, 2), (489, 38)] {
            let original = file.clone();
            status(&mut file, field, value);
            if field == 489 {
                status(&mut file, 488, 1);
            }
            let bytes = file.encode(&profile).unwrap();
            assert!(
                file.prepare_script_contexts(
                    &profile,
                    &cfg,
                    &current,
                    |_, _| Ok(program.clone()),
                    &Default::default()
                )
                .is_err()
            );
            assert_eq!(file.encode(&profile).unwrap(), bytes);
            assert_eq!(current.read_stored(200), 99);
            file = original;
        }
        let cancel = crysirror_tasks::Cancellation::default();
        cancel.cancel();
        let mut reads = 0;
        assert!(
            file.prepare_script_contexts(
                &profile,
                &cfg,
                &current,
                |_, _| {
                    reads += 1;
                    Ok(program.clone())
                },
                &cancel
            )
            .is_err()
        );
        assert_eq!(reads, 0);
        status(&mut file, 22, 1);
        status(&mut file, 488, 1);
        let mut bad = (*program).clone();
        bad.words[59] = 0;
        assert!(
            file.prepare_script_contexts(
                &profile,
                &cfg,
                &current,
                |_, _| Ok(Arc::new(bad.clone())),
                &Default::default()
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod context_admission_tests {
    use super::*;
    #[test]
    fn prepared_context_bank_matches_native_profile_and_general_command_adjustment_is_exact() {
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words.extend([5, 0x66660003, 7, 0, 0]);
        words.extend([3, 0x66660002, 0x55560005]);
        words.push(32);
        words[22] = 40;
        words[18] = 41;
        let program = crate::Program::parse(
            &words
                .into_iter()
                .flat_map(|word| (!word).to_le_bytes())
                .collect::<Vec<_>>(),
            crate::XorProfile::default(),
        )
        .unwrap();
        let native = SavedContext {
            pc: u32::MAX,
            depth: 0,
            calls: [0; 255],
        };
        let variables =
            crate::expression::Variables::fresh(crate::expression::VariableProfile::Legacy1000);
        let machine = native
            .prepare(&program, variables, true, false, 14, &Default::default())
            .unwrap();
        assert_eq!(
            machine.pc(),
            37,
            "general-mode saved command ID chooses after the original command; obsolete saved rawPC is not used"
        );
    }
}

/// Complete original1024-byte EFFECT record. Unknown command/parameter values
/// are retained as native data; this is not an acknowledgement or shader host.
#[derive(Clone, Debug)]
pub(crate) struct SavedEffect {
    pub words: [i32; 256],
    pub filename: Vec<u8>,
}
impl SavedEffect {
    pub fn enabled(&self) -> bool {
        self.words[0] != 0
    }
    pub fn rectangles(&self) -> crate::native_geometry::NativeRects {
        let rect = |at: usize| self.words[at..at + 4].try_into().unwrap();
        crate::native_geometry::NativeRects {
            source: rect(3),
            destination: rect(7),
            source_from: rect(11),
            destination_from: rect(15),
            source_to: rect(19),
            destination_to: rect(23),
        }
    }
}
impl NativeSaveFile {
    pub(crate) fn replace_effect_records(&mut self, records: &[SavedEffect]) -> Result<()> {
        ensure!(records.len() == 16, "NNN native save sixteen effects");
        ensure!(
            self.blocks
                .get(5)
                .is_some_and(|block| block.code == 5 && block.payload.len() == 16384)
                && self
                    .blocks
                    .get(6)
                    .is_some_and(|block| block.code == 6 && block.payload.len() == 4096),
            "NNN native save EFFECT/FILENAME ABI"
        );
        let mut effects = Vec::with_capacity(16384);
        let mut names = vec![0; 4096];
        for (slot, record) in records.iter().enumerate() {
            ensure!(
                record.filename.len() <= 254 && !record.filename.contains(&0),
                "NNN native save filename double-NUL ABI"
            );
            for word in record.words {
                effects.extend(word.to_le_bytes());
            }
            let start = slot * 256;
            names[start..start + record.filename.len()].copy_from_slice(&record.filename);
        }
        self.blocks[5].payload = effects;
        self.blocks[6].payload = names;
        Ok(())
    }
    pub(crate) fn effect_records(&self) -> Result<Vec<SavedEffect>> {
        let effects = self
            .blocks
            .get(5)
            .context("NNN saved EFFECT record absent")?;
        let names = self
            .blocks
            .get(6)
            .context("NNN saved EFFECTFILENAME record absent")?;
        ensure!(
            effects.code == 5
                && effects.payload.len() == 16384
                && names.code == 6
                && names.payload.len() == 4096,
            "NNN original16-slot effect/name record ABI"
        );
        let mut out = Vec::with_capacity(16);
        for slot in 0..16 {
            let mut words = [0i32; 256];
            for (index, word) in words.iter_mut().enumerate() {
                let at = slot * 1024 + index * 4;
                *word = i32::from_le_bytes(effects.payload[at..at + 4].try_into()?);
            }
            let raw = &names.payload[slot * 256..slot * 256 + 256];
            let end = raw
                .iter()
                .position(|byte| *byte == 0)
                .context("NNN unterminated saved picture name")?;
            ensure!(
                end <= 254 && raw[end + 1] == 0,
                "NNN original saved picture double-NUL"
            );
            out.push(SavedEffect {
                words,
                filename: raw[..end].to_vec(),
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod original_profile_tests {
    use super::*;
    use crate::{
        XorProfile,
        config::Names,
        expression::VariableProfile,
        native_bootstrap::{MUGEN2_MODULE_SHA256, NativeConfiguration},
        native_resources::ResourceNames,
    };
    fn config(text: &str) -> NativeConfiguration {
        let empty = Names::parse(b"", false).unwrap();
        NativeConfiguration {
            resources: ResourceNames::new("").unwrap(),
            setup: Names::parse(text.as_bytes(), false).unwrap(),
            main_control: None,
            variable_names: empty.clone(),
            project: empty.clone(),
            commands: empty.clone(),
            functions: empty,
            initial: None,
            module_sha256: MUGEN2_MODULE_SHA256.into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        }
    }
    #[test]
    fn authenticated_native_configuration_owns_default_identity_version_and_extension_layout() {
        let defaults = LegacySaveProfile::for_configuration(&config("")).unwrap();
        assert_eq!(defaults.header_name, b"SAVEDATA");
        assert_eq!(defaults.version, 0);
        assert!(defaults.extension_sizes.is_empty());
        let profile=LegacySaveProfile::for_configuration(&config("saveHeaderName owner highVersion 1 middleVersion 2 lowVersion 3 extSaveDataNumber 2 extSaveDataSize1 12 extSaveDataSize2 0 useExtClass 1 useCutinFlag 1")).unwrap();
        assert_eq!(profile.header_name, b"owner");
        assert_eq!(profile.version, 0x010203);
        assert_eq!(profile.extension_sizes, [12, 0]);
        assert!(profile.omake && profile.cutin);
        assert!(
            LegacySaveProfile::for_configuration(&config("autoExtDataMode1 printmessage")).is_err()
        );
        assert!(
            LegacySaveProfile::for_configuration(&config("autoExtDataMode1 unknown")).is_ok(),
            "original unknown lookup falls back to atoi0 and contributes no auto block"
        );
        for text in [
            "extSaveDataNumber -1",
            "extSaveDataNumber 257",
            "extSaveDataNumber 1 extSaveDataSize1 -1",
            "extSaveDataNumber 1 extSaveDataSize1 67108865",
        ] {
            assert!(
                LegacySaveProfile::for_configuration(&config(text)).is_err(),
                "{text}"
            );
        }
        let mut wrong = config("");
        wrong.module_sha256 = "unreviewed".into();
        assert!(LegacySaveProfile::for_configuration(&wrong).is_err());
    }
}
