// SPDX-License-Identifier: MPL-2.0
//! Native file-cache policy: Totono trial 410A50/410B90/442350/4425B0.
//! Shared storage retains encoded bytes; shared image jobs prepare pinned images.
use super::*;
use crysirror_core::image_processing::{ImageJob, ImageProcessState};

pub(super) struct Cached {
    path: String,
    pub image: Option<Image>,
    encoded_bytes: u64,
    decoded_bytes: u64,
}
pub(super) struct Loading {
    path: String,
    task: Option<Task<(Arc<[u8]>, Option<[u32; 2]>)>>,
    prepared: Option<(Arc<[u8]>, Option<[u32; 2]>)>,
    decode: Option<ImageJob>,
}
const ENCODED_BUDGET: u64 = if cfg!(target_os = "vita") {
    16 << 20
} else {
    64 << 20
};
const DECODED_BUDGET: u64 = if cfg!(target_os = "vita") {
    32 << 20
} else {
    64 << 20
};
impl N2SystemVm {
    pub(super) fn clear_preloaded(&mut self, host: &mut dyn HostServices) -> Result<()> {
        for cached in std::mem::take(&mut self.preloaded).into_values() {
            self.storage
                .as_ref()
                .unwrap()
                .release_asset_bytes(Some(&VfsPath::new(Mount::Asset, &cached.path)?))?;
            if let Some(image) = cached.image {
                host.release_image_resource(
                    self.client,
                    image.paint["asset"]
                        .as_str()
                        .context("N2System cached image identity")?,
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn native_preload(
        &mut self,
        host: &mut dyn HostServices,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        // The original holds its file-cache gate throughout Load/UnloadFile.
        // A waiting owner keeps its PC; another script thread cannot unload a
        // partially prepared entry and then have that entry reappear later.
        if self.threads.values().any(|e| e.preload.is_some()) {
            return Ok(false);
        }
        if call.function == Builtin::UnloadFile {
            let path = if matches!(args.value(0), Value::Null) {
                None
            } else {
                Some(resource::asset_key(&args.string(0)?)?)
            };
            let paths: Vec<_> = self
                .preloaded
                .keys()
                .filter(|key| path.as_ref().is_none_or(|path| path == *key))
                .cloned()
                .collect();
            for key in paths {
                let cached = self.preloaded.remove(&key).unwrap();
                let path = VfsPath::new(Mount::Asset, &cached.path)?;
                self.storage
                    .as_ref()
                    .unwrap()
                    .release_asset_bytes(Some(&path))?;
                if let Some(image) = cached.image {
                    host.release_image_resource(
                        self.client,
                        image.paint["asset"]
                            .as_str()
                            .context("cached image identity")?,
                    )?;
                }
            }
            self.machine.complete(&mut state.thread, Value::Null)?;
            return Ok(true);
        }
        if state.preload.is_none() {
            let path = resource::resolve(self.catalog.as_ref().unwrap(), &args.string(0)?)?;
            let key = resource::asset_key(&path)?;
            if self.preloaded.contains_key(&key) {
                // Native duplicate hits return before considering the decode flag.
                self.machine.complete(&mut state.thread, Value::Null)?;
                return Ok(true);
            }
            ensure!(self.preloaded.len() < 1024, "N2System preloaded file count");
            let remaining = ENCODED_BUDGET
                .saturating_sub(self.preloaded.values().map(|c| c.encoded_bytes).sum());
            let decoded_remaining = DECODED_BUDGET
                .saturating_sub(self.preloaded.values().map(|c| c.decoded_bytes).sum());
            let storage = self.storage.as_ref().unwrap().clone();
            let images = self.images.as_ref().unwrap().clone();
            // Native parser accepts only a literal true spelling for this flag.
            let decode = args.string(1)?.eq_ignore_ascii_case("true");
            let source = path.clone();
            let task = match self.pool.as_ref().unwrap().spawn(move |cancel| {
                cancel.check()?;
                let reader = storage.open_read(&VfsPath::new(Mount::Asset, &source)?)?;
                let bytes: Arc<[u8]> = Arc::from(crysirror_resource::io::read_all_checked(
                    reader.as_ref(),
                    remaining,
                    &|| cancel.check(),
                )?);
                cancel.check()?;
                let size = if decode {
                    let size = images.size(&source)?;
                    ensure!(
                        u64::from(size[0]) * u64::from(size[1]) * 4 <= decoded_remaining,
                        "N2System preloaded decoded-image budget"
                    );
                    Some(size)
                } else {
                    None
                };
                cancel.check()?;
                Ok((bytes, size))
            }) {
                Ok(task) => task,
                Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                Err(e) => return Err(e),
            };
            state.preload = Some(Loading {
                path,
                task: Some(task),
                prepared: None,
                decode: None,
            });
            return Ok(false);
        }
        let loading = state.preload.as_mut().unwrap();
        if let Some(task) = &mut loading.task {
            let Some(prepared) = task.try_take()? else {
                return Ok(false);
            };
            loading.prepared = Some(prepared);
            loading.task = None;
        }
        let (bytes, size) = loading
            .prepared
            .as_ref()
            .context("N2System preloaded bytes")?;
        let image = if let Some(size) = size {
            if loading.decode.is_none() {
                match self
                    .images
                    .as_ref()
                    .unwrap()
                    .begin_encoded(&loading.path, bytes.clone())
                {
                    Ok(job) => loading.decode = Some(job),
                    Err(Error::Limit("image preparation task capacity")) => return Ok(false),
                    Err(e) => return Err(e.into()),
                }
            }
            match loading.decode.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("N2System LoadFile image: {error}"),
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == self.client && image.size() == *size,
                        "N2System preloaded image identity"
                    );
                    Some(image)
                }
            }
        } else {
            None
        };
        let loading = state.preload.take().unwrap();
        let (bytes, _) = loading.prepared.unwrap();
        let path = VfsPath::new(Mount::Asset, &loading.path)?;
        let encoded_bytes = bytes.len() as u64;
        self.storage
            .as_ref()
            .unwrap()
            .retain_asset_bytes(&path, bytes, ENCODED_BUDGET)?;
        let decoded_bytes = image.as_ref().map_or(0, |image| {
            let [w, h] = image.size();
            u64::from(w) * u64::from(h) * 4
        });
        let published = if let Some(image) = image {
            match host.publish_image_resource(self.client, 0x7ffe_0001, image) {
                Ok(uploaded) => Some(Image {
                    paint: json!({"kind":"image","asset":uploaded.path.relative()}),
                    size: uploaded.size,
                }),
                Err(e) => {
                    self.storage
                        .as_ref()
                        .unwrap()
                        .release_asset_bytes(Some(&path))?;
                    return Err(e.into());
                }
            }
        } else {
            None
        };
        self.preloaded.insert(
            resource::asset_key(&loading.path)?,
            Cached {
                path: loading.path,
                image: published,
                encoded_bytes,
                decoded_bytes,
            },
        );
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
}
