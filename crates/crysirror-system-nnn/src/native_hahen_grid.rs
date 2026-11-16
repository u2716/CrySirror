// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HAHEN's procedural partition subselectors and retained worker receipt.
//! Grid construction is reusable shared geometry; cache keys and RNG commit
//! belong to the native class, including the otherwise-unused fourth key word.
use crate::native_hahen_partition::{Key, Partition};
use anyhow::{Result, ensure};
use crysirror_image::{
    fragment_grid::{FragmentGrid, FragmentGridStyle},
    temporal_pattern::PatternRng,
};
use crysirror_tasks::{Task, TaskPool};
#[derive(Clone)]
pub(crate) struct Ready {
    pub partition: Partition,
    pub random: PatternRng,
}
pub(crate) fn spec(key: Key, canvas: [u32; 2]) -> Result<Option<FragmentGrid>> {
    if key.0[0] != 1 {
        return Ok(None);
    }
    let style = match key.0[1] {
        0 => FragmentGridStyle::StaggeredTriangles,
        1 => FragmentGridStyle::Quadrilaterals,
        2 => FragmentGridStyle::StaggeredStrips,
        _ => return Ok(None),
    };
    let y = (i64::from(key.0[2]).max(0) + 1) * 2;
    let x = y * 4 / 3;
    ensure!(x <= 1024 && y <= 1024, "NNN HAHEN subdivision domain");
    let spec = FragmentGrid {
        size: canvas,
        subdivisions: [x as u32, y as u32],
        style,
        jitter_tenths: key.0[3].clamp(0, 4) as u32,
    };
    spec.validate()?;
    Ok(Some(spec))
}
pub(crate) struct Preparation {
    task: Option<Task<Ready>>,
    ready: Option<Ready>,
    failure: Option<String>,
}
impl Preparation {
    pub(crate) fn begin(
        pool: &TaskPool,
        key: Key,
        canvas: [u32; 2],
        mut random: PatternRng,
    ) -> Result<Option<Self>> {
        let Some(spec) = spec(key, canvas)? else {
            return Ok(None);
        };
        let task = pool.spawn(move |cancel| {
            let mesh = spec.prepare(&mut random, &|| cancel.check())?;
            let partition = Partition {
                vertices: mesh.vertices,
                faces: mesh.faces,
            };
            partition.validate()?;
            Ok(Ready { partition, random })
        })?;
        Ok(Some(Self {
            task: Some(task),
            ready: None,
            failure: None,
        }))
    }
    pub(crate) fn poll(&mut self) -> Result<Option<Ready>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN HAHEN partition worker: {error}")
        }
        if let Some(ready) = &self.ready {
            return Ok(Some(ready.clone()));
        }
        match self
            .task
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("NNN HAHEN grid receipt absent"))?
            .try_take()
        {
            Ok(Some(ready)) => {
                self.task = None;
                self.ready = Some(ready.clone());
                Ok(Some(ready))
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.failure = Some(format!("{error:#}"));
                Err(error)
            }
        }
    }
}
