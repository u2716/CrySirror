// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HAHEN setter resource/RNG dependency. Cache hits perform no file read or
//! random draw; procedural and external partitions keep one retained worker.
use crate::{
    native_hahen::HahenClass,
    native_hahen_grid,
    native_hahen_partition::{ExternalPreparation, Key},
    native_picture_reload::ContextData,
};
use anyhow::Result;
use crysirror_image::temporal_pattern::PatternRng;
#[derive(Clone)]
pub(crate) struct Ready {
    pub class: HahenClass,
    pub random: PatternRng,
}
enum Work {
    Grid(native_hahen_grid::Preparation),
    External(ExternalPreparation),
}
pub(crate) struct Preparation {
    candidate: HahenClass,
    parameters: [i32; 18],
    random: PatternRng,
    work: Option<Work>,
    ready: Option<Ready>,
    failure: Option<String>,
}
impl Preparation {
    pub(crate) fn begin(
        class: &HahenClass,
        parameters: [i32; 18],
        canvas: [u32; 2],
        random: PatternRng,
        context: &ContextData,
    ) -> Result<Self> {
        crate::native_hahen::validate(&parameters, parameters[0])?;
        random.validate()?;
        let key = Key::from_parameters(&parameters);
        let work = if !class.needs_partition(&parameters) {
            None
        } else if key.0[0] == 2 {
            Some(Work::External(ExternalPreparation::begin(
                &context.resources,
                &context.names,
                &context.pool,
                key,
                canvas,
            )?))
        } else {
            native_hahen_grid::Preparation::begin(&context.pool, key, canvas, random.clone())?
                .map(Work::Grid)
        };
        let mut result = Self {
            candidate: class.clone(),
            parameters,
            random,
            work,
            ready: None,
            failure: None,
        };
        if result.work.is_none() {
            // Unknown partition types have an explicit authored-default
            // fallback in the source, whereas a valid procedural miss MUST
            // have the real generation worker above (not an empty ACK).
            result.candidate.set(&result.parameters, None)?;
            result.ready = Some(Ready {
                class: result.candidate.clone(),
                random: result.random.clone(),
            });
        }
        Ok(result)
    }
    pub(crate) fn poll(&mut self) -> Result<Option<Ready>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN HAHEN setter: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        let result = self.advance();
        if let Err(error) = &result {
            self.failure = Some(format!("{error:#}"));
        }
        result
    }
    fn advance(&mut self) -> Result<Option<Ready>> {
        let generated = match self
            .work
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN HAHEN partition dependency absent"))?
        {
            Work::Grid(work) => {
                let Some(ready) = work.poll()? else {
                    return Ok(None);
                };
                self.random = ready.random;
                Some(ready.partition)
            }
            Work::External(work) => {
                let Some(partition) = work.poll()? else {
                    return Ok(None);
                };
                partition
            }
        };
        self.candidate.set(&self.parameters, generated)?;
        let ready = Ready {
            class: self.candidate.clone(),
            random: self.random.clone(),
        };
        self.ready = Some(ready.clone());
        self.work = None;
        Ok(Some(ready))
    }
}
