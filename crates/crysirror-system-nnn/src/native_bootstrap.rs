// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Worker-side original bootstrap. Capability reads are supplied by the host;
//! this module never opens a process path, executes a PE, or decodes pictures.
use crate::{
    Program, XorProfile,
    config::{self, Names},
    expression::VariableProfile,
    machine::Machine,
    native_control::ScriptSession,
    native_resources::ResourceNames,
};
use anyhow::{Context, Result, ensure};
use crysirror_tasks::Cancellation;
use std::sync::Arc;

pub const MUGEN2_MODULE_SHA256: &str =
    "e85e92ec538d50eae08d27286177149972d443a96d1415277c1ae1f9ee108bdb";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootEntry {
    Controller,
    NewGame,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomSeedPolicy {
    WallClockSeconds,
    Fixed(u32),
    PreserveStartup,
}
/// Immutable native policy may be shared with resource workers and the owner;
/// the mutable ScriptSession and its RNG transaction remain single-owned.
#[derive(Clone, Debug)]
pub struct NativeConfiguration {
    pub resources: ResourceNames,
    pub setup: Names,
    /// Distinct native MainControl configuration; absence means its original defaults.
    pub main_control: Option<Names>,
    pub variable_names: Names,
    pub project: Names,
    pub commands: Names,
    pub functions: Names,
    pub initial: Option<Names>,
    pub module_sha256: String,
    pub profile: VariableProfile,
    pub scripts: XorProfile,
}
#[derive(Clone, Debug)]
pub struct PreparedBoot {
    pub session: ScriptSession,
    pub resources: ResourceNames,
    pub setup: Names,
    /// Distinct native MainControl configuration; absence means its original defaults.
    pub main_control: Option<Names>,
    pub variable_names: Names,
    pub project: Names,
    pub commands: Names,
    pub functions: Names,
    pub initial: Option<Names>,
    pub module_sha256: String,
    pub profile: VariableProfile,
    pub scripts: XorProfile,
}
impl PreparedBoot {
    pub fn configuration(&self) -> NativeConfiguration {
        NativeConfiguration {
            resources: self.resources.clone(),
            setup: self.setup.clone(),
            main_control: self.main_control.clone(),
            variable_names: self.variable_names.clone(),
            project: self.project.clone(),
            commands: self.commands.clone(),
            functions: self.functions.clone(),
            initial: self.initial.clone(),
            module_sha256: self.module_sha256.clone(),
            profile: self.profile,
            scripts: self.scripts,
        }
    }
    pub fn random_seed_policy(&self) -> Result<RandomSeedPolicy> {
        let seed = self
            .main_control
            .as_ref()
            .map(|table| table.integer(b"initRandomSeed"))
            .transpose()?
            .flatten()
            .unwrap_or(-1);
        Ok(match seed {
            -1 => RandomSeedPolicy::WallClockSeconds,
            value if value > 0 => RandomSeedPolicy::Fixed(value as u32),
            _ => RandomSeedPolicy::PreserveStartup,
        })
    }
    /// Original 0x52d130 uses host wall-clock seconds for -1, positive fixed
    /// srand, and no srand for other values. No clock/default seed is invented.
    pub fn initialize_random_state(
        &mut self,
        wall_clock_seconds: Option<u32>,
        startup: Option<crysirror_image::temporal_pattern::PatternRng>,
    ) -> Result<()> {
        use crysirror_image::temporal_pattern::PatternRng;
        let state = match self.random_seed_policy()? {
            RandomSeedPolicy::WallClockSeconds => PatternRng::msvc(
                wall_clock_seconds
                    .context("System-NNN original random seed needs host wall-clock seconds")?,
            ),
            RandomSeedPolicy::Fixed(seed) => PatternRng::msvc(seed),
            RandomSeedPolicy::PreserveStartup => startup.context(
                "System-NNN srand disabled; original startup shared random state required",
            )?,
        };
        let mut variables = self
            .session
            .machine()
            .variables()
            .context("System-NNN native variable bank absent")?
            .clone();
        variables.replace_random_state(state)?;
        self.session.machine_mut().replace_variables(variables)
    }
    /// `read` distinguishes a missing path from a malformed/inaccessible one.
    /// Run this only after CPU/resource task admission. Results are detached
    /// logical VM data suitable for transfer to the Core VM worker.
    pub fn mugen2(
        mut read: impl FnMut(&str, u64) -> Result<Option<Vec<u8>>>,
        root: &str,
        module: &str,
        entry: BootEntry,
        cancel: &Cancellation,
    ) -> Result<Self> {
        let resources = ResourceNames::new(root)?;
        let mut required = |path: &str, limit: u64| -> Result<Vec<u8>> {
            cancel.check()?;
            let path = resources.path(path)?;
            let bytes = read(&path, limit)?
                .with_context(|| format!("System-NNN required original resource absent: {path}"))?;
            ensure!(
                bytes.len() as u64 <= limit,
                "System-NNN bootstrap read exceeded byte limit"
            );
            Ok(bytes)
        };
        let module_bytes = required(module, 32 << 20)?;
        let module_sha256 = crysirror_resource::pe::PeData::parse(&module_bytes)?
            .sha256()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        ensure!(
            module_sha256 == MUGEN2_MODULE_SHA256,
            "System-NNN unreviewed original callback module {module_sha256}"
        );
        drop(required);
        cancel.check()?;
        let setup = if let Some(bytes) = read(&resources.path("init2/game.fxf")?, 1 << 20)? {
            Names::parse(&bytes, true)?
        } else {
            let path = resources.path("init/game.xtx")?;
            Names::parse(
                &read(&path, 1 << 20)?
                    .with_context(|| format!("System-NNN original config absent: {path}"))?,
                false,
            )?
        };
        cancel.check()?;
        // CMainControl owns a separate name table. A missing host config uses
        // the original defaults; a present malformed table is not ignored.
        let main_control =
            if let Some(bytes) = read(&resources.path("init2/mainControl.fxf")?, 1 << 20)? {
                Some(Names::parse(&bytes, true)?)
            } else {
                read(&resources.path("init/mainControl.xtx")?, 1 << 20)?
                    .map(|bytes| Names::parse(&bytes, false))
                    .transpose()?
            };
        ensure!(
            setup.integer(b"noScriptFlag")?.unwrap_or(0) == 0,
            "System-NNN no-script bootstrap is not a VM profile"
        );
        let mut names = |relative: &str, cipher: bool| -> Result<Names> {
            cancel.check()?;
            let path = resources.path(relative)?;
            Names::parse(
                &read(&path, 1 << 20)?
                    .with_context(|| format!("System-NNN original name table absent: {path}"))?,
                cipher,
            )
        };
        let profile = VariableProfile::Legacy1000;
        let variable_names = names("spt/var.fxf", true)?;
        ensure!(
            variable_names.tokens.len() == 1000,
            "System-NNN reviewed legacy variable ABI mismatch"
        );
        let project = names("spt/project.xtx", false)?;
        let commands = names("spt/command.xtx", false)?;
        let functions = names("spt/function.xtx", false)?;
        ensure!(
            commands.tokens.len() % 2 == 0 && functions.tokens.len() % 2 == 0,
            "System-NNN command/function declaration pairs"
        );
        let initial = if setup.integer(b"varInitFlag")?.unwrap_or(0) != 0 {
            Some(names("spt/varinitdata.xtx", false)?)
        } else {
            None
        };
        drop(names);
        let mut variables = config::configured_variables(profile, &variable_names, &setup)?;
        if variables.native_holidays_pending() {
            cancel.check()?;
            let path = resources.path("nya/saijitsu.xtx")?;
            // Missing CNameList input produces zero tuples in the original;
            // malformed/policy/I/O errors still propagate from the capability.
            let bytes = read(&path, 1 << 20)?.unwrap_or_default();
            variables.prepare_native_holidays(&Names::parse(&bytes, false)?)?;
        }
        if entry == BootEntry::NewGame {
            config::initialize_new_game(&mut variables, profile, initial.as_ref())?;
        }
        cancel.check()?;
        let path = resources.script(b"sys")?;
        let bytes = read(&path, 16 << 20)?
            .with_context(|| format!("System-NNN original controller absent: {path}"))?;
        let scripts = XorProfile::default();
        let controller = Arc::new(Program::parse(&bytes, scripts)?);
        cancel.check()?;
        Ok(Self {
            session: ScriptSession::new(controller, Machine::with_variables(variables)),
            resources,
            setup,
            main_control,
            variable_names,
            project,
            commands,
            functions,
            initial,
            module_sha256,
            profile,
            scripts,
        })
    }
    /// Read exactly the requested original script after a pending SCRIPT call.
    /// Project membership is native metadata, not a synthesized replacement.
    pub fn load_script(
        &self,
        name: &[u8],
        mut read: impl FnMut(&str, u64) -> Result<Option<Vec<u8>>>,
        cancel: &Cancellation,
    ) -> Result<Arc<Program>> {
        cancel.check()?;
        // 538530 loads this original filename independently of project index;
        // an absent name only changes native metadata's index to zero.
        let path = self.resources.script(name)?;
        let bytes = read(&path, 16 << 20)?
            .with_context(|| format!("System-NNN original script absent: {path}"))?;
        let program = Arc::new(Program::parse(&bytes, self.scripts)?);
        cancel.check()?;
        Ok(program)
    }
}

#[cfg(test)]
mod host_configuration_tests {
    use super::*;
    fn prepared() -> PreparedBoot {
        let mut words = vec![0u32; 32];
        words[0] = 32;
        words[1] = 0x66660001;
        words[2] = 0x55550001;
        words[18] = 32;
        let bytes: Vec<_> = words
            .into_iter()
            .flat_map(|word| (!word).to_le_bytes())
            .collect();
        let empty = Names::parse(b"", false).unwrap();
        PreparedBoot {
            session: ScriptSession::new(
                Arc::new(Program::parse(&bytes, XorProfile::default()).unwrap()),
                Machine::with_variables(crate::expression::Variables::fresh(
                    VariableProfile::Legacy1000,
                )),
            ),
            resources: ResourceNames::new("").unwrap(),
            setup: Names::parse(b"initRandomSeed 77", false).unwrap(),
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
    fn original_host_seed_namespace_is_distinct_from_game_and_preserved_in_configuration() {
        let mut boot = prepared();
        assert_eq!(
            boot.random_seed_policy().unwrap(),
            RandomSeedPolicy::WallClockSeconds
        );
        boot.main_control = Some(Names::parse(b"initRandomSeed 42", false).unwrap());
        assert_eq!(
            boot.random_seed_policy().unwrap(),
            RandomSeedPolicy::Fixed(42)
        );
        assert_eq!(
            boot.configuration()
                .main_control
                .unwrap()
                .integer(b"initRandomSeed")
                .unwrap(),
            Some(42)
        );
        boot.main_control = Some(Names::parse(b"initRandomSeed nope", false).unwrap());
        assert_eq!(
            boot.random_seed_policy().unwrap(),
            RandomSeedPolicy::PreserveStartup,
            "original atoi no-digit token is0; a syntactically valid table is not a malformed file"
        );
    }
    #[test]
    fn no_srand_branch_requires_actual_startup_state_and_missing_clock_is_not_zero_seed() {
        let mut boot = prepared();
        assert!(boot.initialize_random_state(None, None).is_err());
        boot.main_control = Some(Names::parse(b"initRandomSeed 0", false).unwrap());
        assert_eq!(
            boot.random_seed_policy().unwrap(),
            RandomSeedPolicy::PreserveStartup
        );
        assert!(boot.initialize_random_state(Some(99), None).is_err());
        boot.initialize_random_state(
            None,
            Some(crysirror_image::temporal_pattern::PatternRng::msvc(123)),
        )
        .unwrap();
        boot.main_control = Some(Names::parse(b"initRandomSeed 42", false).unwrap());
        boot.initialize_random_state(None, None).unwrap();
    }
}
