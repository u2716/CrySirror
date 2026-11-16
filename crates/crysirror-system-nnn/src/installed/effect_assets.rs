// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Constructor effects have one retained asynchronous owner. All native
//! constructor RNG, resource fallback and shared image work finishes before
//! the live Bridge and variable bank receive the result together.
use crate::{
    native_constructor_preparation::{Preparation, Ready},
    native_effect_constructors::ConstructorProfile,
    native_resources::ResourceNames,
    resource_bridge::NativeResources,
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_image::temporal_pattern::PatternRng;
use crysirror_tasks::{Cancellation, TaskPool};
pub(super) struct Assets {
    resources: NativeResources,
    names: ResourceNames,
    levels: u32,
    // Boxed: the constructor state is ~290 KiB, and `Assets` moves by value
    // through preparation results and task channels. On console threads
    // (2 MiB stack) repeated by-value copies overflowed the stack.
    work: Option<Box<Preparation>>,
    ready: Option<Box<Ready>>,
}
impl Assets {
    pub(super) fn prepare(
        config: &crate::native_bootstrap::NativeConfiguration,
        resources: &NativeResources,
        cancel: &Cancellation,
    ) -> Result<Self> {
        cancel.check()?;
        let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
        ensure!(levels <= 1024, "NNN constructor picture levels");
        Ok(Self {
            resources: resources.clone(),
            names: config.resources.clone(),
            levels,
            work: None,
            ready: None,
        })
    }
    pub(super) fn needs_start(&self) -> bool {
        self.work.is_none() && self.ready.is_none()
    }
    pub(super) fn start(
        &mut self,
        client: ClientId,
        pool: &TaskPool,
        canvas: [u32; 2],
        random: PatternRng,
    ) -> Result<()> {
        ensure!(
            self.needs_start() && self.resources.client() == client,
            "NNN effect constructor admission"
        );
        // Installed bootstrap currently admits the explicit MUGEN2 Legacy ABI.
        // Source32's32-layer/expanded-RNG profile remains a separate constructor
        // contract; it cannot silently replace these reviewed native16 slots.
        self.work = Some(Box::new(Preparation::begin(
            ConstructorProfile::Mugen2Legacy,
            pool,
            canvas,
            self.resources.clone(),
            self.names.clone(),
            self.levels,
            random,
            Default::default(),
        )?));
        Ok(())
    }
    pub(super) fn poll(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.ready.is_some() {
            return Ok(true);
        }
        let Some(ready) = self
            .work
            .as_mut()
            .context("NNN effect constructor not admitted")?
            .poll(host)?
        else {
            return Ok(false);
        };
        self.ready = Some(Box::new(ready));
        self.work = None;
        Ok(true)
    }
    pub(super) fn finish(
        self,
        bridge: &mut crate::bridge::Bridge,
        machine: &mut crate::machine::Machine,
    ) -> Result<()> {
        let ready = self
            .ready
            .context("NNN effect constructor dependencies pending")?;
        let (classes, random) = crate::native_particle_classes::Classes::from_ready(*ready)?;
        let mut variables = machine
            .variables()
            .context("NNN constructor native variable bank")?
            .clone();
        variables.replace_random_state(random)?;
        bridge.configure_kira_picture(classes.kira.clone())?;
        bridge.configure_particle_classes(classes)?;
        machine.replace_variables(variables)
    }
}
