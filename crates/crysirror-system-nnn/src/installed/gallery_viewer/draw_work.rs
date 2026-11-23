// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! One admitted native CG Print. Source views, wipe parameters, RNG and the
//! post-Print counter remain candidates until the caller publishes the ready
//! engine image. Completion pumps never re-run Calcu/Print or sample input.
use super::*;
use crate::{
    native_cg_composition::{self, CgTile},
    native_cg_difference::{DifferencePrint, DifferenceState},
};
use crysirror_image::temporal_pattern::PatternRng;
use std::collections::BTreeMap;
#[derive(Clone)]
pub(super) struct CgDrawResult {
    pub image: Arc<dyn ImageResource>,
    pub difference: DifferenceState,
    pub random: Option<PatternRng>,
    pub tables: crate::native_simple_wipe::Tables,
}
pub(super) struct CgDrawWork {
    client: ClientId,
    canvas: [u32; 2],
    base: Option<ImageInput>,
    tiles: Vec<CgTile>,
    columns: u32,
    position: [i64; 2],
    difference: DifferenceState,
    random: Option<PatternRng>,
    tables: crate::native_simple_wipe::Tables,
    old_job: Option<ImageJob>,
    old_ready: Option<Arc<dyn ImageResource>>,
    new_jobs: Vec<(usize, ImageJob)>,
    new_ready: BTreeMap<usize, Arc<dyn ImageResource>>,
    draw_job: Option<ImageJob>,
    after: Option<(
        DifferenceState,
        Option<PatternRng>,
        crate::native_simple_wipe::Tables,
    )>,
    ready: Option<CgDrawResult>,
}
impl CgDrawWork {
    pub(super) fn begin(
        client: ClientId,
        canvas: [u32; 2],
        base: ImageInput,
        tiles: Vec<CgTile>,
        columns: u32,
        position: [i64; 2],
        difference: DifferenceState,
        old: Option<Arc<dyn ImageResource>>,
        random: Option<PatternRng>,
        tables: crate::native_simple_wipe::Tables,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        ensure!(
            columns > 0
                && columns <= 16
                && !tiles.is_empty()
                && tiles.len() <= 256
                && tiles.len() % columns as usize == 0,
            "NNN admitted CG Print tile bounds"
        );
        let wipe = matches!(difference.plan(0), DifferencePrint::Wipe { .. });
        let old_job = if wipe {
            Some(native_cg_composition::prepare_viewport(
                host, client, canvas, old,
            )?)
        } else {
            None
        };
        let mut new_jobs = Vec::new();
        if wipe {
            for index in (0..tiles.len()).step_by(columns as usize) {
                new_jobs.push((
                    index,
                    native_cg_composition::prepare_viewport(
                        host,
                        client,
                        canvas,
                        Some(tiles[index].image.clone()),
                    )?,
                ));
            }
        }
        Ok(Self {
            client,
            canvas,
            base: Some(base),
            tiles,
            columns,
            position,
            difference,
            random,
            tables,
            old_job,
            old_ready: None,
            new_jobs,
            new_ready: Default::default(),
            draw_job: None,
            after: None,
            ready: None,
        })
    }
    pub(super) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<CgDrawResult>> {
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        if let Some(job) = self.old_job.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN CG difference old viewport: {error}")
                }
                ImageProcessState::Ready(image) => {
                    self.old_ready = Some(image);
                    self.old_job = None;
                }
            }
        }
        let mut pending = Vec::new();
        for (index, mut job) in self.new_jobs.drain(..) {
            match job.poll()? {
                ImageProcessState::Pending => pending.push((index, job)),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN CG difference new viewport: {error}")
                }
                ImageProcessState::Ready(image) => {
                    self.new_ready.insert(index, image);
                }
            }
        }
        self.new_jobs = pending;
        if !self.new_jobs.is_empty() {
            return Ok(None);
        }
        if self.draw_job.is_none() {
            let plan = native_cg_composition::plan_print(
                self.client,
                self.canvas,
                self.base
                    .take()
                    .context("NNN CG Print base consumed twice")?,
                &self.tiles,
                self.columns,
                self.position,
                &self.difference,
                self.old_ready.clone(),
                &self.new_ready,
                self.random.as_ref(),
                &self.tables,
            )?;
            self.after = Some((plan.difference_after, plan.random_after, plan.tables_after));
            self.draw_job = Some(
                host.image_metadata(self.client)?
                    .begin_process(plan.input, b"[]".as_slice().into())?,
            );
        }
        match self.draw_job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => anyhow::bail!("NNN CG shared Print graph: {error}"),
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.client && image.size() == self.canvas,
                    "NNN CG Print ready image scope"
                );
                let (difference, random, tables) = self
                    .after
                    .take()
                    .context("NNN CG Print after-state absent")?;
                let ready = CgDrawResult {
                    image,
                    difference,
                    random,
                    tables,
                };
                self.ready = Some(ready.clone());
                Ok(Some(ready))
            }
        }
    }
}
