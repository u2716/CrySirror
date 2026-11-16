// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HAHEN retains its real native setter/save receipt until partition work has
//! completed. Pixel work remains a separate shared image service operation.
use super::*;
pub(super) struct PendingSet {
    receipt: NativeCall,
    work: crate::native_hahen_set::Preparation,
    pub(super) ready: Option<crate::native_hahen_set::Ready>,
}
impl PendingSet {
    pub(super) fn matches(&self, call: &NativeCall) -> bool {
        self.receipt == *call
    }
}
impl Bridge {
    pub(super) fn try_hahen_set(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        if self.pending_hahen.is_none() {
            let Payload::Words { count, words } = &call.payload else {
                bail!("NNN HAHEN setter ABI");
            };
            ensure!(
                *count as usize == words.len() && *count >= 2 && words[1] == 81,
                "NNN HAHEN setter span"
            );
            self.layer_id(words[0])?;
            self.freeze_effect_backing()?;
            let p = crate::native_hahen::parameters(&words[2..], self.bindings.size);
            let class = &self
                .particle_classes
                .as_ref()
                .context("NNN HAHEN constructor absent")?
                .hahen;
            let random = machine
                .native_variables_candidate(call)?
                .random_state()
                .context("NNN HAHEN random state")?;
            let context = self
                .images
                .as_ref()
                .context("NNN HAHEN task/resource context")?
                .staff_context();
            let work = crate::native_hahen_set::Preparation::begin(
                class,
                p,
                self.bindings.size,
                random,
                &context,
            )?;
            self.pending_hahen = Some(PendingSet {
                receipt: call.clone(),
                work,
                ready: None,
            });
        }
        let pending = self.pending_hahen.as_mut().unwrap();
        ensure!(pending.matches(call), "NNN HAHEN native receipt changed");
        machine.validate_receipt(call)?;
        if pending.ready.is_none() {
            let Some(ready) = pending.work.poll()? else {
                return Ok(false);
            };
            pending.ready = Some(ready);
        }
        self.commit(machine, call, host)?;
        self.pending_hahen = None;
        Ok(true)
    }
    pub(crate) fn next_saved_hahen(&self) -> Option<i32> {
        let slot = *self.saved_hahen.first()?;
        (self.next_saved_cpu_effect().is_none()
            && self
                .saved_suna
                .first_key_value()
                .is_none_or(|(s, _)| slot < *s))
        .then_some(slot)
    }
    pub(crate) fn poll_saved_hahen(
        &mut self,
        slot: i32,
        random: &PatternRng,
    ) -> Result<Option<PatternRng>> {
        ensure!(
            self.next_saved_hahen() == Some(slot),
            "NNN HAHEN saved init order changed"
        );
        if self.saved_hahen_work.is_none() {
            let state = self
                .picture_effects
                .get(&slot)
                .context("NNN HAHEN saved state")?;
            let crate::native_picture_effect::PictureEffectKind::Hahen { parameters } = state.kind
            else {
                bail!("NNN HAHEN saved class changed");
            };
            let class = &self
                .particle_classes
                .as_ref()
                .context("NNN saved HAHEN constructor absent")?
                .hahen;
            let context = self
                .images
                .as_ref()
                .context("NNN HAHEN saved task/resource context")?
                .staff_context();
            self.saved_hahen_work = Some((
                slot,
                crate::native_hahen_set::Preparation::begin(
                    class,
                    parameters,
                    self.bindings.size,
                    random.clone(),
                    &context,
                )?,
            ));
        }
        let (pending, work) = self.saved_hahen_work.as_mut().unwrap();
        ensure!(*pending == slot, "NNN HAHEN saved receipt changed");
        let Some(ready) = work.poll()? else {
            return Ok(None);
        };
        self.particle_classes
            .as_mut()
            .context("NNN HAHEN restore constructor absent")?
            .hahen = Arc::new(ready.class);
        self.saved_hahen.remove(&slot);
        self.saved_hahen_work = None;
        Ok(Some(ready.random))
    }
}
