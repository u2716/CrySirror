// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Ordered mask-resource completion between native Calcu and the first Print.
//! Calcu already consumed scalar/RNG policy in slot order; this owner neither
//! recomputes a recipe nor acknowledges an unfinished shared image operation.
use crate::{native_bath_mask, native_bath_window::Update, native_staff_bank::Bank};
use anyhow::{Context, Result, ensure};
use crysirror_core::{image_processing::ImageResource, service::HostServices, state::ClientId};
use std::{
    collections::{BTreeSet, VecDeque},
    sync::Arc,
};
#[derive(Clone)]
pub(crate) struct Mutation {
    pub bank: crate::native_picture_bank::FramePictureBank,
    pub bank_limit: i32,
    pub slot: i32,
    pub update: Update,
}
#[derive(Clone)]
pub(crate) struct Ready {
    pub bank: Bank,
    pub touched: BTreeSet<i32>,
}
pub(crate) struct Preparation {
    client: ClientId,
    atlas: Arc<dyn ImageResource>,
    candidate: Bank,
    queue: VecDeque<Mutation>,
    pending: Option<(i32, native_bath_mask::Preparation)>,
    touched: BTreeSet<i32>,
    ready: Option<Ready>,
    failure: Option<String>,
    pub api_calls: u64,
}
impl Preparation {
    pub(crate) fn begin(
        client: ClientId,
        atlas: Arc<dyn ImageResource>,
        bank: Bank,
        mutations: Vec<Mutation>,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0
                && atlas.client() == client
                && atlas.size() == [128, 384]
                && mutations.len() <= 32
                && mutations.iter().all(|m| (0..32).contains(&m.slot)),
            "NNN bath mutation owner/count"
        );
        ensure!(
            bank.values().all(|p| p.client() == client),
            "NNN bath mutation bank scope"
        );
        Ok(Self {
            client,
            atlas,
            candidate: bank,
            queue: mutations.into(),
            pending: None,
            touched: Default::default(),
            ready: None,
            failure: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Ready>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN bath mutation sequence: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self.advance(host);
        if let Err(error) = &result {
            self.failure = Some(format!("{error:#}"));
        }
        result
    }
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<Option<Ready>> {
        loop {
            if let Some((slot, work)) = self.pending.as_mut() {
                let before = work.api_calls;
                let result = work.poll(host);
                self.api_calls += work.api_calls - before;
                let Some(ready) = result? else {
                    return Ok(None);
                };
                let cache = self
                    .candidate
                    .get_mut(slot)
                    .context("NNN bath pending picture identity")?;
                ensure!(cache.mask_exists, "NNN bath admitted mask fact changed");
                cache.storage = Some(ready.picture);
                if ready.flush_filename {
                    cache.flush()?;
                }
                self.touched.insert(*slot);
                self.pending = None;
            }
            let Some(mutation) = self.queue.pop_front() else {
                let ready = Ready {
                    bank: self.candidate.clone(),
                    touched: self.touched.clone(),
                };
                self.ready = Some(ready.clone());
                return Ok(Some(ready));
            };
            let cache = self
                .candidate
                .get(&mutation.slot)
                .context("NNN bath native picture cache absent")?;
            // No loadOk or assignment prerequisite: Calcu uses GetMaskPic,
            // which can refer to retained allocation after a failed reload.
            ensure!(
                cache.mask_exists,
                "NNN bath mutation without native mask-exists flag"
            );
            let mut picture = cache
                .storage
                .as_ref()
                .context("NNN bath native mask buffer absent")?
                .clone();
            picture.has_mask = cache.mask_exists;
            let work = native_bath_mask::Preparation::begin(
                self.client,
                picture,
                self.atlas.clone(),
                mutation.update,
                host,
            )?;
            self.api_calls += work.api_calls;
            self.pending = Some((mutation.slot, work));
        }
    }
}
