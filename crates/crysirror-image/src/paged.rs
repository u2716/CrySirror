// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Bounded sparse page layouts and immutable image generations. This module
//! plans work; ordinary engine ImageJobs own every pixel allocation/operation.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageResource},
    state::ClientId,
};
use std::{collections::BTreeMap, sync::Arc};

pub const MAX_PAGED_IMAGE_TILES: usize = 4096;
pub const MAX_PAGED_IMAGE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PagedImageLayout {
    tile_size: [u32; 2],
    grid_size: [u32; 2],
    allocated: Vec<bool>,
}
impl PagedImageLayout {
    /// Regions are [column,row,column_count,row_count]. Overlaps allocate once;
    /// holes remain absent, independently of a page's not-yet-defined pixels.
    pub fn from_tile_regions(
        tile_size: [u32; 2],
        grid_size: [u32; 2],
        regions: &[[u32; 4]],
    ) -> Result<Self> {
        crate::rgba_len(tile_size[0], tile_size[1])?;
        let count = u64::from(grid_size[0]) * u64::from(grid_size[1]);
        ensure!(
            grid_size.iter().all(|n| *n > 0) && count <= MAX_PAGED_IMAGE_TILES as u64,
            "paged image grid budget"
        );
        for i in 0..2 {
            ensure!(
                u64::from(tile_size[i]) * u64::from(grid_size[i]) <= 1_048_576,
                "paged image logical extent"
            );
        }
        ensure!(
            regions.len() <= MAX_PAGED_IMAGE_TILES,
            "paged image allocation region budget"
        );
        let mut allocated = vec![false; count as usize];
        for &[x, y, w, h] in regions {
            ensure!(
                x.checked_add(w).is_some_and(|n| n <= grid_size[0])
                    && y.checked_add(h).is_some_and(|n| n <= grid_size[1]),
                "paged image allocation region bounds"
            );
            for row in y..y + h {
                for col in x..x + w {
                    allocated[(row * grid_size[0] + col) as usize] = true;
                }
            }
        }
        let bytes = allocated.iter().filter(|b| **b).count() as u64
            * u64::from(tile_size[0])
            * u64::from(tile_size[1])
            * 4;
        ensure!(
            bytes <= MAX_PAGED_IMAGE_BYTES,
            "paged image total logical byte budget"
        );
        Ok(Self {
            tile_size,
            grid_size,
            allocated,
        })
    }
    pub fn tile_size(&self) -> [u32; 2] {
        self.tile_size
    }
    pub fn grid_size(&self) -> [u32; 2] {
        self.grid_size
    }
    pub fn logical_size(&self) -> [u32; 2] {
        [
            self.tile_size[0] * self.grid_size[0],
            self.tile_size[1] * self.grid_size[1],
        ]
    }
    pub fn allocated_slots(&self) -> impl Iterator<Item = u32> + '_ {
        self.allocated
            .iter()
            .enumerate()
            .filter_map(|(i, b)| b.then_some(i as u32))
    }
    pub fn allocated_bytes(&self) -> u64 {
        self.allocated_slots().count() as u64
            * u64::from(self.tile_size[0])
            * u64::from(self.tile_size[1])
            * 4
    }
    /// Signed [x,y,width,height], half-open clipping, stable row-major slots.
    pub fn intersect(
        &self,
        rect: [i64; 4],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<PagedImageRegion>> {
        check()?;
        ensure!(
            rect[2] >= 0 && rect[3] >= 0,
            "paged image negative region extent"
        );
        let right = rect[0]
            .checked_add(rect[2])
            .context("paged image region x overflow")?;
        let bottom = rect[1]
            .checked_add(rect[3])
            .context("paged image region y overflow")?;
        let mut out = Vec::new();
        for slot in self.allocated_slots() {
            check()?;
            let origin = [
                slot % self.grid_size[0] * self.tile_size[0],
                slot / self.grid_size[0] * self.tile_size[1],
            ];
            let x = rect[0].max(i64::from(origin[0]));
            let y = rect[1].max(i64::from(origin[1]));
            let r = right.min(i64::from(origin[0]) + i64::from(self.tile_size[0]));
            let b = bottom.min(i64::from(origin[1]) + i64::from(self.tile_size[1]));
            if r > x && b > y {
                out.push(PagedImageRegion {
                    slot,
                    origin,
                    rect: [
                        (x - i64::from(origin[0])) as u32,
                        (y - i64::from(origin[1])) as u32,
                        (r - x) as u32,
                        (b - y) as u32,
                    ],
                });
            }
        }
        check()?;
        Ok(out)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PagedImageRegion {
    pub slot: u32,
    pub origin: [u32; 2],
    /// Page-local [x,y,width,height].
    pub rect: [u32; 4],
}
#[derive(Debug)]
struct PageSet {
    pages: Vec<Option<Arc<dyn ImageResource>>>,
    changed: Vec<u64>,
}
#[derive(Clone, Debug)]
pub struct PagedImage {
    client: ClientId,
    generation: u64,
    layout: Arc<PagedImageLayout>,
    set: Arc<PageSet>,
}
#[derive(Clone, Debug)]
pub struct PagedClearPlan {
    client: ClientId,
    generation: u64,
    base: Arc<PageSet>,
    regions: Vec<PagedImageRegion>,
    inputs: Vec<ImageInput>,
}
impl PagedClearPlan {
    pub fn regions(&self) -> &[PagedImageRegion] {
        &self.regions
    }
    pub fn requests(&self) -> impl Iterator<Item = (u32, &ImageInput)> {
        self.regions
            .iter()
            .zip(&self.inputs)
            .map(|(r, i)| (r.slot, i))
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}
impl PagedImage {
    /// Allocated pages start explicitly undefined. A full-page clear can define
    /// them; a partial write may not invent values for their remaining pixels.
    pub fn new(client: ClientId, layout: PagedImageLayout) -> Self {
        let count = layout.allocated.len();
        Self {
            client,
            generation: 0,
            layout: Arc::new(layout),
            set: Arc::new(PageSet {
                pages: vec![None; count],
                changed: vec![0; count],
            }),
        }
    }
    pub fn client(&self) -> ClientId {
        self.client
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn layout(&self) -> &PagedImageLayout {
        &self.layout
    }
    pub fn page(&self, slot: u32) -> Option<&Arc<dyn ImageResource>> {
        self.set.pages.get(slot as usize).and_then(Option::as_ref)
    }
    pub fn defined_pages(&self) -> usize {
        self.set.pages.iter().filter(|p| p.is_some()).count()
    }
    pub fn dirty_since(&self, generation: u64) -> Result<Vec<u32>> {
        ensure!(
            generation <= self.generation,
            "paged image future dirty generation"
        );
        Ok(self
            .set
            .changed
            .iter()
            .enumerate()
            .filter_map(|(i, g)| (*g > generation).then_some(i as u32))
            .collect())
    }
    pub fn clear_plan(
        &self,
        rect: [i64; 4],
        color: [u8; 4],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<PagedClearPlan> {
        let regions = self.layout.intersect(rect, check)?;
        let mut inputs = Vec::with_capacity(regions.len());
        for r in &regions {
            check()?;
            let [w, h] = self.layout.tile_size;
            let input = if r.rect == [0, 0, w, h] {
                ImageInput::Solid {
                    size: [w, h],
                    color,
                }
            } else {
                let base = self
                    .page(r.slot)
                    .context("paged image partial clear reads undefined page")?
                    .clone();
                ImageInput::Composite {
                    base: Box::new(ImageInput::Resource(base)),
                    layers: vec![ImageCompositeLayer {
                        input: Box::new(ImageInput::Solid {
                            size: [r.rect[2], r.rect[3]],
                            color,
                        }),
                        source: [0, 0, r.rect[2] as i32, r.rect[3] as i32],
                        destination: [r.rect[0] as i32, r.rect[1] as i32],
                        clip: None,
                        opacity: 255,
                        math: ImageCompositeMath::Copy,
                    }],
                }
            };
            inputs.push(input);
        }
        check()?;
        Ok(PagedClearPlan {
            client: self.client,
            generation: self.generation,
            base: self.set.clone(),
            regions,
            inputs,
        })
    }
    /// Validate a complete owner-polled ImageJob result batch before publishing
    /// a new generation. Clones/snapshots keep their old resources and order.
    pub fn complete_clear(
        &self,
        plan: &PagedClearPlan,
        completed: &[(u32, Arc<dyn ImageResource>)],
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        check()?;
        ensure!(
            plan.client == self.client
                && plan.generation == self.generation
                && Arc::ptr_eq(&plan.base, &self.set),
            "paged image stale/foreign clear plan"
        );
        ensure!(
            completed.len() == plan.regions.len(),
            "paged image incomplete clear batch"
        );
        let mut ready = BTreeMap::new();
        for (slot, image) in completed {
            check()?;
            ensure!(
                image.client() == self.client && image.size() == self.layout.tile_size,
                "paged image result owner/size mismatch"
            );
            ensure!(
                image.rgba().len() == crate::rgba_len(image.size()[0], image.size()[1])?,
                "paged image malformed result layout"
            );
            ensure!(
                ready.insert(*slot, image.clone()).is_none(),
                "paged image duplicate clear result"
            );
        }
        ensure!(
            plan.regions.iter().all(|r| ready.contains_key(&r.slot)),
            "paged image result slot mismatch"
        );
        if plan.regions.is_empty() {
            check()?;
            return Ok(self.clone());
        }
        let generation = self
            .generation
            .checked_add(1)
            .context("paged image generation overflow")?;
        let mut pages = self.set.pages.clone();
        let mut changed = self.set.changed.clone();
        for (slot, image) in ready {
            pages[slot as usize] = Some(image);
            changed[slot as usize] = generation;
        }
        check()?;
        Ok(Self {
            client: self.client,
            generation,
            layout: self.layout.clone(),
            set: Arc::new(PageSet { pages, changed }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug)]
    struct TestImage {
        client: ClientId,
        size: [u32; 2],
        bytes: Arc<[u8]>,
    }
    impl ImageResource for TestImage {
        fn client(&self) -> ClientId {
            self.client
        }
        fn size(&self) -> [u32; 2] {
            self.size
        }
        fn rgba(&self) -> Arc<[u8]> {
            self.bytes.clone()
        }
        fn opaque(&self) -> bool {
            false
        }
    }
    fn image(client: u32, size: [u32; 2], byte: u8) -> Arc<dyn ImageResource> {
        Arc::new(TestImage {
            client: ClientId(client),
            size,
            bytes: vec![byte; (size[0] * size[1] * 4) as usize].into(),
        })
    }
    #[test]
    fn original_sized_page_budget_stays_bounded_without_a_large_surface() {
        let layout = PagedImageLayout::from_tile_regions(
            [256, 256],
            [12, 26],
            &[
                [0, 0, 12, 13],
                [0, 13, 12, 3],
                [0, 16, 10, 4],
                [0, 20, 10, 6],
            ],
        )
        .unwrap();
        assert_eq!(layout.logical_size(), [3072, 6656]);
        assert_eq!(layout.allocated_slots().count(), 292);
        assert_eq!(layout.allocated_bytes(), 76_546_048);
        assert!(crate::rgba_len(3072, 6656).is_err());
        assert!(
            PagedImageLayout::from_tile_regions([4096, 4096], [3, 3], &[[0, 0, 3, 3]]).is_err()
        );
        assert!(PagedImageLayout::from_tile_regions([1, 1], [65, 65], &[]).is_err());
        assert!(PagedImageLayout::from_tile_regions([2, 2], [2, 2], &[[1, 1, 2, 1]]).is_err());
    }
    #[test]
    fn clipped_half_open_regions_skip_holes_and_cancel() {
        let layout = PagedImageLayout::from_tile_regions(
            [2, 2],
            [3, 2],
            &[[0, 0, 1, 2], [2, 0, 1, 1], [0, 0, 1, 1]],
        )
        .unwrap();
        let regions = layout.intersect([-1, -1, 6, 4], &|| Ok(())).unwrap();
        assert_eq!(
            regions.iter().map(|r| r.slot).collect::<Vec<_>>(),
            [0, 2, 3]
        );
        assert_eq!(regions[2].rect, [0, 0, 2, 1]);
        assert_eq!(regions[1].rect, [0, 0, 1, 2]);
        assert!(
            layout
                .intersect([2, 0, 0, 4], &|| Ok(()))
                .unwrap()
                .is_empty()
        );
        assert!(layout.intersect([i64::MAX, 0, 1, 1], &|| Ok(())).is_err());
        assert!(
            layout
                .intersect([0, 0, 6, 4], &|| anyhow::bail!("cancel"))
                .is_err()
        );
    }
    #[test]
    fn generations_keep_frozen_pages_dirty_history_and_owner_checks() {
        let layout = PagedImageLayout::from_tile_regions([2, 2], [2, 1], &[[0, 0, 2, 1]]).unwrap();
        let empty = PagedImage::new(ClientId(1), layout.clone());
        assert!(empty.clear_plan([0, 0, 1, 1], [0; 4], &|| Ok(())).is_err());
        let plan = empty.clear_plan([0, 0, 4, 2], [0; 4], &|| Ok(())).unwrap();
        assert!(
            plan.requests()
                .all(|(_, i)| matches!(i, ImageInput::Solid { .. }))
        );
        let ready = vec![(0, image(1, [2, 2], 0)), (1, image(1, [2, 2], 0))];
        assert!(
            empty
                .complete_clear(&plan, &ready[..1], &|| Ok(()))
                .is_err()
        );
        assert!(
            empty
                .complete_clear(
                    &plan,
                    &[(0, image(2, [2, 2], 0)), ready[1].clone()],
                    &|| Ok(())
                )
                .is_err()
        );
        assert!(
            empty
                .complete_clear(
                    &plan,
                    &[(0, image(1, [1, 2], 0)), ready[1].clone()],
                    &|| Ok(())
                )
                .is_err()
        );
        assert!(
            empty
                .complete_clear(&plan, &[ready[0].clone(), ready[0].clone()], &|| Ok(()))
                .is_err()
        );
        assert!(
            PagedImage::new(ClientId(1), layout)
                .complete_clear(&plan, &ready, &|| Ok(()))
                .is_err()
        );
        assert!(
            empty
                .complete_clear(&plan, &ready, &|| anyhow::bail!("cancel"))
                .is_err()
        );
        assert_eq!(empty.defined_pages(), 0);
        let first = empty.complete_clear(&plan, &ready, &|| Ok(())).unwrap();
        let frozen = first.clone();
        assert_eq!(first.dirty_since(0).unwrap(), [0, 1]);
        assert_eq!(first.generation(), 1);
        assert!(first.complete_clear(&plan, &ready, &|| Ok(())).is_err());
        let partial = first.clear_plan([1, 0, 1, 1], [9; 4], &|| Ok(())).unwrap();
        assert!(matches!(
            partial.requests().next().unwrap().1,
            ImageInput::Composite { .. }
        ));
        let second = first
            .complete_clear(&partial, &[(0, image(1, [2, 2], 9))], &|| Ok(()))
            .unwrap();
        assert_eq!(second.dirty_since(1).unwrap(), [0]);
        assert_eq!(second.page(0).unwrap().rgba()[0], 9);
        assert_eq!(frozen.page(0).unwrap().rgba()[0], 0);
        assert_eq!(frozen.generation(), 1);
        assert!(second.dirty_since(3).is_err());
        let no_change = second
            .clear_plan([10, 0, 2, 2], [0; 4], &|| Ok(()))
            .unwrap();
        assert_eq!(
            second
                .complete_clear(&no_change, &[], &|| Ok(()))
                .unwrap()
                .generation(),
            2
        );
    }
}
