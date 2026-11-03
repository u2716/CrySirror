// SPDX-License-Identifier: MPL-2.0
//! Original DrawContext texture-page allocation/handle policy. Shared paged
//! Image plans clipping/dirty generations; engine ImageJobs own pixel work.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_processing::{ImageJob, ImageProcessState, ImageResource},
    service::{ClientStorage, HostServices, ImageMetadata},
    state::ClientId,
};
use crysirror_image::paged::{PagedClearPlan, PagedImage, PagedImageLayout};
use crysirror_resource::xml::XmlDocument;
use crysirror_tasks::{Task, TaskPool};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Debug)]
pub(crate) struct NativeTextureCache {
    pub pages: PagedImage,
    pub published: BTreeMap<u32, String>,
    pub bpp: u32,
    pub bpp16_row: u32,
}
fn load(
    storage: &dyn ClientStorage,
    client: ClientId,
    check: &dyn Fn() -> Result<()>,
) -> Result<NativeTextureCache> {
    check()?;
    let bytes = crate::native_settings::optional_asset(storage, "config/startup.xml")?
        .context("CatSystem2 texture cache startup missing")?;
    let xml = XmlDocument::parse_with_fallback(&bytes,encoding_rs::SHIFT_JIS)?;
    let number = |path: &str, default: u32| -> Result<u32> {
        Ok(xml
            .unique_child_path(path)?
            .map(|n| crate::native_values::integer(&n.text))
            .unwrap_or(default))
    };
    //71C670 defaults256/256,grid0/0,8area[-1;4],threshold0;
    //71D2F0 calls71D430+71E8E0 on this same startup document.
    let tile = [
        number("TEXTURE/size/x", 256)?,
        number("TEXTURE/size/y", 256)?,
    ];
    let grid = [number("TEXTURE/vt/x", 0)?, number("TEXTURE/vt/y", 0)?];
    let bpp = number("TEXTURE/bpp", 32)?;
    let bpp16_row = number("TEXTURE/bpp16y", 0)?;
    ensure!(bpp == 32, "CatSystem2 unsupported native texture cache bpp");
    ensure!(
        tile.iter().all(|v| v.is_power_of_two()),
        "CatSystem2 unsupported texture page non-power-of-two/device fallback"
    );
    let mut areas = Vec::new();
    for i in 0..8 {
        check()?;
        let key = format!("TEXTURE/area{i:02}");
        let x = number(&format!("{key}/sx"), u32::MAX)?;
        if x == u32::MAX {
            continue;
        }
        let y = number(&format!("{key}/sy"), u32::MAX)?;
        let w = number(&format!("{key}/dx"), u32::MAX)?;
        let h = number(&format!("{key}/dy"), u32::MAX)?;
        // Rows past `bpp16_row` were 16-bit in the original to save memory; pages here are RGBA.
        ensure!(y.checked_add(h).is_some(), "CatSystem2 texture area extent");
        areas.push([x, y, w, h]);
    }
    let layout = PagedImageLayout::from_tile_regions(tile, grid, &areas)?;
    check()?;
    Ok(NativeTextureCache {
        pages: PagedImage::new(client, layout),
        published: BTreeMap::new(),
        bpp,
        bpp16_row,
    })
}
impl NativeTextureCache {
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"logical_size":self.pages.layout().logical_size(),"tile_size":self.pages.layout().tile_size(),"grid_size":self.pages.layout().grid_size(),"allocated_pages":self.pages.layout().allocated_slots().count(),"allocated_rgba_bytes":self.pages.layout().allocated_bytes(),"defined_pages":self.pages.defined_pages(),"generation":self.pages.generation(),"published_pages":self.published.len(),"published_paths":self.published,"bpp":self.bpp,"bpp16_row":self.bpp16_row})
    }
}
pub(crate) struct PendingClear {
    pub native_pc: u32,
    rect: [i64; 4],
    config: Option<Task<NativeTextureCache>>,
    cache: Option<NativeTextureCache>,
    images: Arc<dyn ImageMetadata>,
    plan: Option<PagedClearPlan>,
    job: Option<(u32, ImageJob)>,
    ready: Vec<(u32, Arc<dyn ImageResource>)>,
    complete: Option<NativeTextureCache>,
    failed: Option<String>,
}
impl PendingClear {
    pub fn begin(
        pool: &TaskPool,
        host: &mut dyn HostServices,
        client: ClientId,
        pc: u32,
        args: &[u32],
        cache: Option<NativeTextureCache>,
    ) -> Result<Option<Self>> {
        ensure!(
            args.len() == 6 && args[0] == 0,
            "CatSystem2 unsupported explicit bitmap native117 target/profile"
        );
        let x = args[1] as i32;
        let y = args[2] as i32;
        let right = x.wrapping_add(args[3] as i32);
        let bottom = y.wrapping_add(args[4] as i32);
        let rect = if right <= x || bottom <= y {
            [0; 4]
        } else {
            [
                i64::from(x),
                i64::from(y),
                i64::from(right) - i64::from(x),
                i64::from(bottom) - i64::from(y),
            ]
        };
        let config = if cache.is_none() {
            let permit = match pool.reserve() {
                Ok(p) => p,
                Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(None),
                Err(e) => return Err(e),
            };
            let storage = host.storage(client)?;
            Some(permit.spawn(move |cancel| load(storage.as_ref(), client, &|| cancel.check()))?)
        } else {
            None
        };
        Ok(Some(Self {
            native_pc: pc,
            rect,
            config,
            cache,
            images: host.image_metadata(client)?,
            plan: None,
            job: None,
            ready: Vec::new(),
            complete: None,
            failed: None,
        }))
    }
    pub fn poll(&mut self) -> Result<Option<NativeTextureCache>> {
        if let Some(error) = &self.failed {
            anyhow::bail!("{error}");
        }
        match self.poll_inner() {
            Ok(v) => Ok(v),
            Err(e) => {
                let error = format!("{e:#}");
                self.failed = Some(error.clone());
                anyhow::bail!("{error}");
            }
        }
    }
    fn poll_inner(&mut self) -> Result<Option<NativeTextureCache>> {
        if let Some(cache) = &self.complete {
            return Ok(Some(cache.clone()));
        }
        if self.cache.is_none() {
            let Some(cache) = self
                .config
                .as_mut()
                .context("CatSystem2 texture cache worker missing")?
                .try_take()?
            else {
                return Ok(None);
            };
            self.cache = Some(cache);
            self.config = None;
        }
        let cache = self.cache.as_ref().unwrap();
        if self.plan.is_none() {
            self.plan = Some(cache.pages.clear_plan(self.rect, [0; 4], &|| Ok(()))?);
        }
        let plan = self.plan.as_ref().unwrap();
        if let Some((slot, job)) = &mut self.job {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("CatSystem2 texture cache shared clear failed: {error}")
                }
                ImageProcessState::Ready(image) => {
                    self.ready.push((*slot, image));
                    self.job = None;
                }
            }
        }
        if let Some((slot, input)) = plan.requests().nth(self.ready.len()) {
            self.job = Some((
                slot,
                self.images
                    .begin_process(input.clone(), Arc::from(b"[]".as_slice()))?,
            ));
            return Ok(None);
        }
        let mut proposed = cache.clone();
        proposed.pages = cache.pages.complete_clear(plan, &self.ready, &|| Ok(()))?;
        self.complete = Some(proposed.clone());
        Ok(Some(proposed))
    }
    /// Publish all completed pages as one native admission. A failure releases
    /// only these new paths; prior generations remain usable by their owners.
    pub fn publish(
        &self,
        host: &mut dyn HostServices,
        client: ClientId,
        mut proposed: NativeTextureCache,
    ) -> Result<NativeTextureCache> {
        let mut published = Vec::new();
        for (slot, image) in &self.ready {
            match host.publish_image_resource(client, 0x45000000 + *slot, image.clone()) {
                Ok(v) => published.push((*slot, v.path.relative().to_owned())),
                Err(error) => {
                    for (_, path) in &published {
                        host.release_image_resource(client, path)?;
                    }
                    return Err(error.into());
                }
            }
        }
        //No fallible native state write follows publication: KCS preflight and
        //all ImageJob/resource validation happened before reaching this point.
        for (slot, path) in published {
            if let Some(old) = proposed.published.insert(slot, path) {
                host.release_image_resource(client, &old)?;
            }
        }
        Ok(proposed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Storage(Vec<u8>);
    impl ClientStorage for Storage {
        fn read(&self, path: &crysirror_core::path::VfsPath) -> crysirror_core::Result<Vec<u8>> {
            if path.mount() == crysirror_core::path::Mount::Asset
                && path.relative() == "config/startup.xml"
            {
                Ok(self.0.clone())
            } else {
                Err(crysirror_core::Error::NotFound)
            }
        }
        fn remove(&self, _: &crysirror_core::path::VfsPath) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
        fn list(
            &self,
            _: crysirror_core::path::Mount,
            _: &str,
        ) -> crysirror_core::Result<Vec<String>> {
            Err(crysirror_core::Error::NotFound)
        }
        fn write_atomic(
            &self,
            _: &crysirror_core::path::VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
    }
    #[test]
    fn original_texture_allocation_policy_delegates_page_geometry_without_pixels() {
        let xml=b"<document><TEXTURE><size><x>2</x><y>2</y></size><vt><x>3</x><y>2</y></vt><area00><sx>0</sx><sy>0</sy><dx>3</dx><dy>1</dy></area00><area01><sx>0</sx><sy>1</sy><dx>2</dx><dy>1</dy></area01><bpp16y>2</bpp16y><bpp>32</bpp></TEXTURE></document>";
        let cache = load(&Storage(xml.to_vec()), ClientId(1), &|| Ok(())).unwrap();
        assert_eq!(cache.pages.layout().logical_size(), [6, 4]);
        assert_eq!(
            cache.pages.layout().allocated_slots().collect::<Vec<_>>(),
            [0, 1, 2, 3, 4]
        );
        assert_eq!(cache.pages.defined_pages(), 0);
        assert!(cache.published.is_empty());
        let plan = cache
            .pages
            .clear_plan([0, 0, 6, 4], [0; 4], &|| Ok(()))
            .unwrap();
        assert_eq!(plan.requests().count(), 5);
        let sixteen = String::from_utf8(xml.to_vec())
            .unwrap()
            .replace("<bpp>32</bpp>", "<bpp>16</bpp>");
        assert!(load(&Storage(sixteen.into_bytes()), ClientId(1), &|| Ok(())).is_err());
        let split = String::from_utf8(xml.to_vec())
            .unwrap()
            .replace("<bpp16y>2", "<bpp16y>1");
        // Grisaia's areas pass bpp16y: those rows stay RGBA here instead of converting.
        assert!(load(&Storage(split.into_bytes()), ClientId(1), &|| Ok(())).is_ok());
        assert!(load(&Storage(xml.to_vec()), ClientId(1), &|| anyhow::bail!(
            "cancel"
        ))
        .is_err());
    }
}
