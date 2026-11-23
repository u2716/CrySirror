// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Worker-owned label decoding with charged immutable resident plane views.
use crysirror_core::{
    Error, Result,
    path::{Mount, VfsPath},
    region_map::{RegionMap, RegionMapQuery, RegionMapState, RegionPlane},
    service::ClientStorage,
    state::ClientId,
};
use crysirror_image::label_image::LabelImage;
use crysirror_tasks::{Task, TaskPool};
use std::sync::{Arc, atomic::AtomicUsize};
struct Plane {
    owner: u32,
    source: LabelImage,
    size: [u32; 2],
    _charge: crate::image_processing::Charge,
}
impl std::fmt::Debug for Plane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegionPlane")
            .field("client", &self.owner)
            .field("size", &self.size)
            .finish()
    }
}
impl RegionPlane for Plane {
    fn client(&self) -> u32 {
        self.owner
    }
    fn size(&self) -> [u32; 2] {
        self.size
    }
    fn value_at(&self, [x, y]: [u32; 2]) -> Result<Option<u32>> {
        if x >= self.size[0] || y >= self.size[1] {
            return Ok(None);
        }
        let [w, h] = self.source.size;
        Ok(self
            .source
            .labels
            .get((y % h) as usize * w as usize + (x % w) as usize)
            .map(|v| u32::from(*v)))
    }
}
struct Query {
    owner: u32,
    task: Option<Task<RegionMap>>,
    state: RegionMapState,
}
impl RegionMapQuery for Query {
    fn client(&self) -> u32 {
        self.owner
    }
    fn poll(&mut self) -> Result<RegionMapState> {
        if let Some(task) = self.task.as_mut() {
            match task.try_take() {
                Ok(None) => (),
                Ok(Some(map)) => {
                    self.task = None;
                    self.state = RegionMapState::Ready(map);
                }
                Err(error) => {
                    self.task = None;
                    self.state = RegionMapState::Failed(
                        error
                            .downcast_ref::<Error>()
                            .cloned()
                            .unwrap_or_else(|| Error::Io(format!("{error:#}"))),
                    );
                }
            }
        }
        Ok(self.state.clone())
    }
}
pub(crate) fn begin(
    owner: ClientId,
    path: VfsPath,
    minimum: Option<[u32; 2]>,
    storage: Arc<dyn ClientStorage>,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<Box<dyn RegionMapQuery>> {
    if owner.0 == 0 || minimum.is_some_and(|size| size.iter().any(|v| *v == 0 || *v > 8192)) {
        return Err(Error::Invalid("region image identity/minimum extent"));
    }
    let permit = pool.reserve().map_err(|e| Error::Io(e.to_string()))?;
    let task = permit
        .spawn(move |cancel| {
            cancel.check()?;
            let source = if path.mount() == Mount::Asset {
                storage.open_asset(&path)?
            } else {
                storage.open_read(&path)?
            };
            let length = source.len()?;
            anyhow::ensure!(
                length > 0 && length <= crysirror_image::MAX_DECODED_BYTES as u64,
                "label image encoded byte budget"
            );
            let _input = crate::image_processing::Charge::new(memory.clone(), length as usize)?;
            let _scratch = crate::image_processing::Charge::new(
                memory.clone(),
                2 * crysirror_image::MAX_DECODED_BYTES,
            )?;
            let mut output = crate::image_processing::Charge::new(memory, 16 * 1024 * 1024)?;
            let mut bytes = vec![0; length as usize];
            for (index, chunk) in bytes.chunks_mut(64 * 1024).enumerate() {
                cancel.check()?;
                crysirror_resource::read_exact_at(
                    source.as_ref(),
                    (index * 64 * 1024) as u64,
                    chunk,
                )?;
            }
            anyhow::ensure!(source.len()? == length, "label image changed during read");
            let image = LabelImage::decode(&bytes, &|| cancel.check())?;
            output.shrink(image.labels.len());
            let mut size = image.size;
            if let Some(minimum) = minimum {
                size[0] = size[0].max(minimum[0]);
                size[1] = size[1].max(minimum[1]);
            }
            cancel.check()?;
            let plane = Arc::new(Plane {
                owner: owner.0,
                source: image,
                size,
                _charge: output,
            });
            Ok(RegionMap::from_plane(owner.0, plane)?)
        })
        .map_err(|e| Error::Io(e.to_string()))?;
    Ok(Box::new(Query {
        owner: owner.0,
        task: Some(task),
        state: RegionMapState::Pending,
    }))
}
