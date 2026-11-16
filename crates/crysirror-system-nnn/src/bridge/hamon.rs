// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native HAMON setters retain the real delivery while engine field creation
//! is pending. Restore uses the same class-owned field, with native slot order.
use super::*;
pub(super) struct PendingSet {
    receipt: NativeCall,
    work: crate::native_hamon_vectors::Preparation,
}
impl PendingSet {
    pub(super) fn matches(&self, call: &NativeCall) -> bool {
        self.receipt == *call
    }
}
impl Bridge {
    pub(super) fn try_hamon_set(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        if self.pending_hamon.is_none() {
            let Payload::Words { count, words } = &call.payload else {
                bail!("NNN HAMON setter word ABI")
            };
            ensure!(
                *count as usize == words.len() && *count >= 2 && words[1] == 69,
                "NNN HAMON setter parameter span"
            );
            self.layer_id(words[0])?;
            self.freeze_effect_backing()?;
            let p = crate::native_hamon::parameters(&words[2..], self.bindings.size);
            crate::native_hamon::validate(&p)?;
            let key = crate::native_hamon::VectorKey {
                canvas: self.bindings.size,
                center: [p[2], p[3]],
                start_y: p[10],
            };
            let pool = self
                .images
                .as_ref()
                .context("NNN HAMON resource/task context absent")?
                .staff_context()
                .pool;
            let work = crate::native_hamon_vectors::Preparation::begin(
                self.client,
                key,
                self.hamon_field.as_ref(),
                &pool,
            )?;
            self.pending_hamon = Some(PendingSet {
                receipt: call.clone(),
                work,
            });
        }
        let pending = self.pending_hamon.as_mut().unwrap();
        ensure!(
            pending.receipt == *call,
            "NNN HAMON pending native receipt changed"
        );
        machine.validate_receipt(call)?;
        let before = pending.work.api_calls;
        let result = pending.work.poll(host);
        self.api_calls += pending.work.api_calls - before;
        let Some(field) = result? else {
            return Ok(false);
        };
        // No native state or PC changes on failed/pending field work. Common
        // commit replaces the real slot; then this infallible class lease swap
        // completes the same delivery. Skip may not cancel an admitted setter.
        self.commit(machine, call, host)?;
        self.hamon_field = Some(field);
        self.pending_hamon = None;
        Ok(true)
    }
    pub(crate) fn poll_saved_hamon(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        loop {
            let Some(&slot) = self.saved_hamon.first() else {
                return Ok(true);
            };
            if self.saved_hamon_work.is_none() {
                let state = self
                    .picture_effects
                    .get(&slot)
                    .context("NNN saved HAMON state absent")?;
                let crate::native_picture_effect::PictureEffectKind::Hamon { parameters: p } =
                    state.kind
                else {
                    bail!("NNN saved HAMON class changed")
                };
                let key = crate::native_hamon::VectorKey {
                    canvas: self.bindings.size,
                    center: [p[2], p[3]],
                    start_y: p[10],
                };
                let pool = self
                    .images
                    .as_ref()
                    .context("NNN saved HAMON task context absent")?
                    .staff_context()
                    .pool;
                self.saved_hamon_work = Some((
                    slot,
                    crate::native_hamon_vectors::Preparation::begin(
                        self.client,
                        key,
                        self.hamon_field.as_ref(),
                        &pool,
                    )?,
                ));
            }
            let (identity, work) = self.saved_hamon_work.as_mut().unwrap();
            ensure!(
                *identity == slot,
                "NNN saved HAMON initialization order changed"
            );
            let before = work.api_calls;
            let result = work.poll(host);
            self.api_calls += work.api_calls - before;
            let Some(field) = result? else {
                return Ok(false);
            };
            self.hamon_field = Some(field);
            self.saved_hamon.remove(&slot);
            self.saved_hamon_work = None;
        }
    }
}
