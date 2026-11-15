// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! STAFFROLL's sequential CPicture recycling transaction. Resource reads and
//! raster jobs complete before the next page observes the mutated bank. Only
//! the completed result may replace live pictures and presentation together.
use crate::{
    image_bridge::NativePictureViews,
    native_picture_cache::PictureCache,
    native_picture_reload::{ContextData, Reload},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone)]
pub(crate) struct Completed {
    pub png_headers:Vec<crysirror_image::png_header::PngHeader>,
    pub bank: BTreeMap<i32, PictureCache<NativePictureViews>>,
    pub touched: BTreeSet<i32>,
    pub image: Arc<dyn ImageResource>,
}

pub(crate) struct Pages {
    png_headers:Vec<crysirror_image::png_header::PngHeader>,
    client: ClientId,
    context: ContextData,
    owner: u64,
    parameters: [i32; 16],
    own: i32,
    bank_limit: i32,
    point: [i32; 2],
    canvas: [u32; 2],
    bank: BTreeMap<i32, PictureCache<NativePictureViews>>,
    current: Arc<dyn ImageResource>,
    touched: BTreeSet<i32>,
    delta: i32,
    page: Option<(i32, u32)>,
    read: Option<Reload>,
    paint: Option<ImageJob>,
    ready: Option<Completed>,
    failure: Option<String>,
    pub api_calls: u64,
}
impl Pages {
    pub(crate) fn begin(
        context: ContextData,
        owner: u64,
        parameters: [i32; 16],
        own: i32,
        bank_limit: i32,
        assigned: bool,
        point: [i32; 2],
        canvas: [u32; 2],
        bank: BTreeMap<i32, PictureCache<NativePictureViews>>,
        current: Arc<dyn ImageResource>,
    ) -> Result<Self> {
        let client = context.resources.client();
        ensure!(
            current.client() == client && current.size() == canvas,
            "NNN staff presentation client/extent"
        );
        ensure!(
            matches!(bank_limit, 16 | 32) && (0..bank_limit).contains(&own),
            "NNN staff physical bank profile"
        );
        ensure!(
            (1..=4096).contains(&parameters[12])
                && bank
                    .iter()
                    .all(|(slot, picture)| (0..bank_limit).contains(slot)
                        && picture.client() == client
                        && !picture.indexed),
            "NNN staff page budget/bank scope"
        );
        let delta = if assigned { 0 } else { parameters[12] };
        Ok(Self {
            png_headers:Vec::new(),
            client,
            context,
            owner,
            parameters,
            own,
            bank_limit,
            point,
            canvas,
            bank,
            current,
            touched: BTreeSet::new(),
            delta,
            page: None,
            read: None,
            paint: None,
            ready: None,
            failure: None,
            api_calls: 0,
        })
    }
    pub(crate) fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Completed>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("NNN staff pages: {error}");
        }
        if let Some(result) = &self.ready {
            return Ok(Some(result.clone()));
        }
        let result = self.advance(host);
        if let Err(error) = &result {
            self.failure = Some(format!("{error:#}"));
        }
        result
    }
    fn advance(&mut self, host: &mut dyn HostServices) -> Result<Option<Completed>> {
        // A cached/off-screen sequence may have no I/O. Bound one poll's work
        // without admitting another page or repeating its LoadDWQ on retry.
        for _ in 0..64 {
            if let Some(paint) = self.paint.as_mut() {
                match paint.poll()? {
                    ImageProcessState::Pending => return Ok(None),
                    ImageProcessState::Failed(error) => anyhow::bail!("staff raster: {error}"),
                    ImageProcessState::Ready(image) => {
                        ensure!(
                            image.client() == self.client && image.size() == self.canvas,
                            "NNN staff raster result scope"
                        );
                        self.current = image;
                        self.paint = None;
                        self.delta += 1;
                        continue;
                    }
                }
            }
            if let Some(read) = self.read.as_mut() {
                let before = read.api_calls;
                let result = read.poll(host);
                self.api_calls += read.api_calls - before;
                let Some(result) = result? else {
                    return Ok(None);
                };
                let (slot, height) = self.page.take().context("NNN staff pending page")?;
                if let Some(header)=result.png_header{self.png_headers.push(header);}
                self.bank.insert(slot, result.cache);
                self.touched.insert(slot);
                self.read = None;
                if result.returned {
                    self.paint_page(slot, height, host)?;
                } else {
                    // A failed new name still replaces the cache identity and
                    // loadOk flag. The next same-name call returns TRUE and
                    // may draw the retained older RGB/mask allocation.
                    self.delta += 1;
                }
                continue;
            }
            if self.delta >= self.parameters[12] {
                let ready = Completed {
                    png_headers:self.png_headers.clone(),
                    bank: self.bank.clone(),
                    touched: self.touched.clone(),
                    image: self.current.clone(),
                };
                self.ready = Some(ready.clone());
                return Ok(Some(ready));
            }
            let base_slot = self.own.clamp(0, self.bank_limit - 1);
            let Some(base) = self.bank.get(&base_slot) else {
                self.delta = self.parameters[12];
                continue;
            };
            // CPicture::InitWork sets size1x1 even with a NULL RGB buffer.
            let height = base.storage.as_ref().map_or(1, |v| v.raw.size()[1]);
            let Some(slot) = crate::native_staff_roll::page_slot(
                &self.parameters,
                self.own,
                self.delta,
                self.bank_limit,
            )?
            else {
                self.delta += 1;
                continue;
            };
            if self.delta == 0 {
                self.paint_page(slot, height, host)?;
                continue;
            }
            // Re-read the base name and height for EVERY page: clamping the
            // literal slot16 into a16-entry bank can alias and reload the base.
            let name = crate::native_picture_name::staff_frame(&base.filename, self.delta)?;
            let current = if let Some(picture) = self.bank.get(&slot) {
                picture.clone()
            } else {
                PictureCache::empty(self.client)?
            };
            self.read = Some(Reload::begin(&self.context, &current, self.owner, &name)?);
            self.page = Some((slot, height));
        }
        Ok(None)
    }
    fn paint_page(
        &mut self,
        slot: i32,
        base_height: u32,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        let picture = self.bank.get(&slot).and_then(|p| p.storage.as_ref());
        let input = if let Some(picture) = picture {
            crate::native_staff_roll::page_input(
                &self.parameters,
                self.point,
                base_height,
                self.delta,
                picture,
                self.canvas,
                ImageInput::Resource(self.current.clone()),
            )?
        } else {
            None
        };
        if let Some(input) = input {
            self.paint = Some(
                host.image_metadata(self.client)?
                    .begin_process(input, b"[]".as_slice().into())?,
            );
            self.api_calls += 1;
        } else {
            self.delta += 1;
        }
        Ok(())
    }
}
