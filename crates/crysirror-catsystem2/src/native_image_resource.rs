// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native HG3 frame selection and two-stage resource preparation. The optional
//! GPL format plugin parses/decodes the container; shared Image workers own the
//! immutable pixels. This module has no pixel codec, renderer or composition.
//! Native FES completion remains the caller's responsibility after publication.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    path::{Mount, VfsPath},
    service::{ClientStorage, HostServices, ImageMetadata, MemoryAssetReader, UploadedImage},
    state::ClientId,
};
use crysirror_resource::DecodeContext;
use crysirror_tasks::{Cancellation, Task, TaskPool};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Geometry {
    pub frame_id: u32,
    pub size: [u32; 2],
    pub canvas: [u32; 2],
    pub origin: [i32; 2],
    pub base: Option<[i32; 2]>,
    pub color_bits: u32,
    pub flags: Option<u32>,
    pub image_mode: Option<u32>,
    pub compression_type: Option<u32>,
}
#[derive(Clone, Debug)]
struct Encoded {
    path: String,
    geometry: Geometry,
    bytes: Arc<[u8]>,
}
#[derive(Clone, Debug)]
enum Selection {
    MissingFile,
    MissingFrame,
    Frame(Encoded),
}
enum Request {
    Resolved(VfsPath),
    Named(String),
}
fn select_request(
    storage: &dyn ClientStorage,
    client: ClientId,
    request: Request,
    frame_id: u32,
    cancel: Cancellation,
) -> Result<Selection> {
    match request {
        Request::Resolved(path) => select(storage, client, &path, frame_id, cancel),
        Request::Named(name) => {
            cancel.check()?;
            let catalog = crate::native_resource_paths::Catalog::load(storage)?;
            for candidate in catalog.candidates(storage, "image", &name, &|| cancel.check())? {
                cancel.check()?;
                let Some(path) = crate::native_resource_paths::resolve(storage, &candidate)? else {
                    continue;
                };
                let selected = select(storage, client, &path, frame_id, cancel.clone())?;
                if !matches!(selected, Selection::MissingFile) {
                    return Ok(selected);
                }
            }
            Ok(Selection::MissingFile)
        }
    }
}

// The caller supplies the already resolved native archive/member path. LST,
// INT extension and loose-file precedence belong to the native catalog, not
// this worker and not the shared codec. No ambient host path is accepted.
fn select(
    storage: &dyn ClientStorage,
    client: ClientId,
    path: &VfsPath,
    frame_id: u32,
    cancel: Cancellation,
) -> Result<Selection> {
    cancel.check()?;
    let reader = match storage.open_asset(path) {
        Ok(reader) => reader,
        Err(crysirror_core::Error::NotFound) => return Ok(Selection::MissingFile),
        Err(error) => return Err(error.into()),
    };
    // Capture one bounded immutable source generation before indexing. A file
    // replacement cannot mix old metadata with newly read frame bytes.
    let bytes = crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)?;
    cancel.check()?;
    let mut context = DecodeContext::new(client).with_cancellation(cancel.clone());
    context.source_name = Some(path.relative().to_owned());
    context.parameters.insert(
        "cat-system/hg3".into(),
        serde_json::json!({"expand_canvas":false,"pixel_profile":"native"}),
    );
    let archive = crysirror_formats_arc::legacy_archive("cat-system/hg3")
        .context("CatSystem2 shared HG3 format unavailable")?
        .open(Arc::new(MemoryAssetReader(bytes.into())), &context)?;
    let mut selected = None;
    for entry in archive.entries() {
        cancel.check()?;
        let geometry: Geometry = serde_json::from_value(entry.metadata["hg3"].clone())
            .context("CatSystem2 shared HG3 geometry contract")?;
        if geometry.frame_id != frame_id {
            continue;
        }
        ensure!(
            selected.is_none(),
            "CatSystem2 ambiguous native HG3 frame ID"
        );
        ensure!(
            geometry.size.iter().all(|v| (1..=8192).contains(v))
                && u64::from(geometry.size[0]) * u64::from(geometry.size[1]) <= 16 * 1024 * 1024,
            "CatSystem2 native image extent budget"
        );
        ensure!(
            entry.metadata["hg3"]["expanded"] == false
                && entry.metadata["hg3"]["decoded_size"] == serde_json::json!(geometry.size),
            "CatSystem2 HG3 raw frame geometry mismatch"
        );
        selected = Some((entry.id, geometry));
    }
    let Some((id, geometry)) = selected else {
        return Ok(Selection::MissingFrame);
    };
    let bytes = crysirror_resource::read_all(
        archive.open(id, &context)?.as_ref(),
        context.limits.max_entry_size,
    )?;
    cancel.check()?;
    Ok(Selection::Frame(Encoded {
        path: path.relative().to_owned(),
        geometry,
        bytes: bytes.into(),
    }))
}

#[derive(Clone, Debug)]
pub(crate) struct Prepared {
    pub source: String,
    pub geometry: Geometry,
    pub image: Arc<dyn ImageResource>,
}
#[derive(Clone, Debug)]
pub(crate) enum Outcome {
    MissingFile,
    MissingFrame,
    Ready(Prepared),
}
/// Worker completion and engine pixel completion are independent retained
/// phases. Repeated owner polls neither reread a file nor rerun image effects.
pub(crate) struct Pending {
    pub owner: u32,
    pub native_pc: u32,
    client: ClientId,
    selection: Option<Task<Selection>>,
    encoded: Option<Encoded>,
    images: Arc<dyn ImageMetadata>,
    image_job: Option<ImageJob>,
    outcome: Option<Outcome>,
    failure: Option<String>,
    published: Option<(u32, UploadedImage)>,
}
impl Pending {
    pub fn begin(
        pool: &TaskPool,
        host: &mut dyn HostServices,
        client: ClientId,
        owner: u32,
        native_pc: u32,
        path: &str,
        frame_id: u32,
    ) -> Result<Option<Self>> {
        Self::begin_request(
            pool,
            host,
            client,
            owner,
            native_pc,
            Request::Resolved(VfsPath::new(Mount::Asset, path)?),
            frame_id,
        )
    }
    pub fn begin_named(
        pool: &TaskPool,
        host: &mut dyn HostServices,
        client: ClientId,
        owner: u32,
        native_pc: u32,
        name: &str,
        frame_id: u32,
    ) -> Result<Option<Self>> {
        ensure!(
            !name.is_empty() && name.is_ascii() && name.len() < 260,
            "CatSystem2 native image filename profile"
        );
        VfsPath::new(Mount::Asset, &name.replace('\\', "/"))?;
        Self::begin_request(
            pool,
            host,
            client,
            owner,
            native_pc,
            Request::Named(name.to_owned()),
            frame_id,
        )
    }
    fn begin_request(
        pool: &TaskPool,
        host: &mut dyn HostServices,
        client: ClientId,
        owner: u32,
        native_pc: u32,
        request: Request,
        frame_id: u32,
    ) -> Result<Option<Self>> {
        let permit = match pool.reserve() {
            Ok(permit) => permit,
            Err(error) if error.is::<crysirror_tasks::CapacityExceeded>() => return Ok(None),
            Err(error) => return Err(error),
        };
        let storage = host.storage(client)?;
        let images = host.image_metadata(client)?;
        let selection = permit.spawn(move |cancel| {
            select_request(storage.as_ref(), client, request, frame_id, cancel)
        })?;
        Ok(Some(Self {
            owner,
            native_pc,
            client,
            selection: Some(selection),
            encoded: None,
            images,
            image_job: None,
            outcome: None,
            failure: None,
            published: None,
        }))
    }
    pub fn poll(&mut self) -> Result<Option<Outcome>> {
        if let Some(error) = &self.failure {
            anyhow::bail!("{error}");
        }
        match self.poll_inner() {
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                let error = format!("{error:#}");
                self.failure = Some(error.clone());
                anyhow::bail!("{error}");
            }
        }
    }
    fn poll_inner(&mut self) -> Result<Option<Outcome>> {
        if let Some(outcome) = &self.outcome {
            return Ok(Some(outcome.clone()));
        }
        if let Some(task) = &mut self.selection {
            let Some(selection) = task.try_take()? else {
                return Ok(None);
            };
            self.selection = None;
            match selection {
                Selection::MissingFile => self.outcome = Some(Outcome::MissingFile),
                Selection::MissingFrame => self.outcome = Some(Outcome::MissingFrame),
                Selection::Frame(frame) => self.encoded = Some(frame),
            }
            if let Some(outcome) = &self.outcome {
                return Ok(Some(outcome.clone()));
            }
        }
        let frame = self
            .encoded
            .as_ref()
            .context("CatSystem2 image selection phase")?;
        if self.image_job.is_none() {
            self.image_job = Some(self.images.begin_process(
                ImageInput::Encoded {
                    name: format!("{}#{}.png", frame.path, frame.geometry.frame_id),
                    bytes: frame.bytes.clone(),
                    size: frame.geometry.size,
                },
                Arc::from(b"[]".as_slice()),
            )?);
            return Ok(None);
        }
        match self.image_job.as_mut().unwrap().poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("CatSystem2 shared image worker: {error}")
            }
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.client && image.size() == frame.geometry.size,
                    "CatSystem2 shared image ownership/geometry receipt"
                );
                self.outcome = Some(Outcome::Ready(Prepared {
                    source: frame.path.clone(),
                    geometry: frame.geometry.clone(),
                    image,
                }));
                self.image_job = None;
                Ok(self.outcome.clone())
            }
        }
    }
    /// This produces a real engine resource receipt, not native ACK. The owner
    /// must commit its object state/Canvas operations before advancing the VM.
    pub fn publish(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        id: u32,
    ) -> Result<UploadedImage> {
        ensure!(
            client == self.client,
            "CatSystem2 foreign image publication"
        );
        if let Some((published_id, receipt)) = &self.published {
            ensure!(
                *published_id == id,
                "CatSystem2 image publication identity changed"
            );
            return Ok(receipt.clone());
        }
        let Some(Outcome::Ready(frame)) = &self.outcome else {
            anyhow::bail!("CatSystem2 image publication before preparation");
        };
        let receipt = host.publish_image_resource(client, id, frame.image.clone())?;
        self.published = Some((id, receipt.clone()));
        Ok(receipt)
    }
    /// Roll back this unpublished-to-native engine lease after a later atomic
    /// Scene admission fails. A retry reuses prepared pixels but gets a fresh
    /// engine receipt; old Scene generations retain their own immutable leases.
    pub fn rollback_publication(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        ensure!(client == self.client, "CatSystem2 foreign image rollback");
        if let Some((_, receipt)) = &self.published {
            host.release_image_resource(client, receipt.path.relative())?;
            self.published = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        time::{Duration, Instant},
    };
    fn compressed(bytes: &[u8]) -> Vec<u8> {
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        z.write_all(bytes).unwrap();
        z.finish().unwrap()
    }
    // Self-authored 1x1 HG3 with a deliberately nonordinal native frame ID.
    fn fixture() -> Vec<u8> {
        let mut out = b"HG-3".to_vec();
        for word in [12u32, 0x300, 0, 37] {
            out.extend(word.to_le_bytes());
        }
        out.extend(b"stdinfo\0");
        for word in [
            56u32,
            40,
            1,
            1,
            32,
            (-3i32) as u32,
            4,
            20,
            30,
            7,
            (-8i32) as u32,
            9,
        ] {
            out.extend(word.to_le_bytes());
        }
        out.extend(b"img0000\0");
        let pixels = compressed(&[255; 4]);
        // One nonzero run of 4 bytes, then 4 terminal gamma bits.
        let control = compressed(&[0x09, 0x01]);
        let length = 24 + pixels.len() + control.len();
        out.extend(0u32.to_le_bytes());
        out.extend((length as u32).to_le_bytes());
        for word in [0u32, 1, pixels.len() as u32, 4, control.len() as u32, 2] {
            out.extend(word.to_le_bytes());
        }
        out.extend(pixels);
        out.extend(control);
        out
    }
    fn runtime(root: &std::path::Path) -> Result<crysirror_runtime::Runtime> {
        let project = serde_json::from_value(
            serde_json::json!({"schema":2,"name":"authored native image worker","content_revision":"1","assets":"game","canvas":[8,8],"clients":[{"name":"game","adapter":"crysirror-script-v1","game":"unused"}]}),
        )?;
        Ok(crysirror_runtime::Runtime::with_asset_root(
            project,
            root,
            &root.join("state"),
            Default::default(),
        )?)
    }
    fn await_frame(pending: &mut Pending) -> Result<Outcome> {
        let start = Instant::now();
        loop {
            if let Some(outcome) = pending.poll()? {
                return Ok(outcome);
            }
            ensure!(
                start.elapsed() < Duration::from_secs(5),
                "authored image worker timeout"
            );
            std::thread::yield_now();
        }
    }
    #[test]
    fn authored_frame_uses_shared_pixel_worker_and_real_client_publication() -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::write(root.path().join("native.hg3"), fixture())?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("cat-hg3-frame", 1, 2)?;
        let client = ClientId(1);
        let mut pending =
            Pending::begin(&pool, &mut rt.host, client, 8, 0x1234, "native.hg3", 37)?.unwrap();
        assert_eq!((pending.owner, pending.native_pc), (8, 0x1234));
        assert!(pending.publish(&mut rt.host, client, 600).is_err());
        let Outcome::Ready(frame) = await_frame(&mut pending)? else {
            anyhow::bail!("frame not prepared")
        };
        assert_eq!(frame.source, "native.hg3");
        assert_eq!(frame.geometry.frame_id, 37);
        assert_eq!(frame.geometry.origin, [-3, 4]);
        assert_eq!(frame.geometry.canvas, [20, 30]);
        assert_eq!(frame.geometry.base, Some([-8, 9]));
        assert_eq!(frame.image.rgba().as_ref(), [128; 4]);
        // Preparation owns immutable source generation even after replacement.
        std::fs::write(root.path().join("native.hg3"), b"not an image")?;
        let Outcome::Ready(again) = pending.poll()?.unwrap() else {
            panic!("lost ready receipt")
        };
        assert!(Arc::ptr_eq(&frame.image, &again.image));
        assert!(pending.publish(&mut rt.host, ClientId(2), 600).is_err());
        let receipt = pending.publish(&mut rt.host, client, 600)?;
        assert_eq!(receipt.size, [1, 1]);
        assert!(!receipt.opaque);
        assert!(receipt.path.relative().starts_with("__image_resource/"));
        assert_eq!(
            pending.publish(&mut rt.host, client, 600)?.path,
            receipt.path
        );
        assert!(pending.publish(&mut rt.host, client, 601).is_err());
        assert!(pending
            .rollback_publication(&mut rt.host, ClientId(2))
            .is_err());
        pending.rollback_publication(&mut rt.host, client)?;
        let replacement = pending.publish(&mut rt.host, client, 600)?;
        assert_ne!(replacement.path, receipt.path);
        pending.rollback_publication(&mut rt.host, client)?;
        Ok(())
    }
    #[test]
    fn missing_file_missing_native_frame_and_malformed_data_remain_distinct() -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::write(root.path().join("native.hg3"), fixture())?;
        std::fs::write(root.path().join("bad.hg3"), b"not an image")?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("cat-hg3-errors", 1, 3)?;
        let client = ClientId(1);
        for (path, id, is_file) in [("absent.hg3", 37, true), ("native.hg3", 0, false)] {
            let mut pending =
                Pending::begin(&pool, &mut rt.host, client, 8, 0x1234, path, id)?.unwrap();
            let outcome = await_frame(&mut pending)?;
            assert!(matches!(
                (&outcome, is_file),
                (Outcome::MissingFile, true) | (Outcome::MissingFrame, false)
            ));
            assert!(pending.publish(&mut rt.host, client, 700).is_err());
            assert!(pending.poll()?.is_some());
        }
        let mut pending =
            Pending::begin(&pool, &mut rt.host, client, 8, 0x1234, "bad.hg3", 37)?.unwrap();
        let error = await_frame(&mut pending).unwrap_err().to_string();
        assert_eq!(pending.poll().unwrap_err().to_string(), error);
        assert!(Pending::begin(&pool, &mut rt.host, client, 8, 0x1234, "../escape", 0).is_err());
        Ok(())
    }
    #[test]
    fn bounded_task_admission_never_acknowledges_or_opens_an_image() -> Result<()> {
        let root = tempfile::tempdir()?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("cat-hg3-capacity", 1, 1)?;
        let permit = pool.reserve()?;
        assert!(
            Pending::begin(&pool, &mut rt.host, ClientId(1), 8, 0x1234, "absent.hg3", 0)?.is_none()
        );
        drop(permit);
        let mut pending =
            Pending::begin(&pool, &mut rt.host, ClientId(1), 8, 0x1234, "absent.hg3", 0)?.unwrap();
        assert!(matches!(await_frame(&mut pending)?, Outcome::MissingFile));
        Ok(())
    }
    #[test]
    fn configured_image_list_search_preserves_native_order_and_case_on_actual_storage() -> Result<()>
    {
        let root = tempfile::tempdir()?;
        for dir in ["config", "Image/BG", "Bg.INT", "Ev.INT"] {
            std::fs::create_dir_all(root.path().join(dir))?;
        }
        std::fs::write(root.path().join("config/startup.xml"),b"<root><INT><int00><name>image</name><int>image.lst</int><ext>hg3</ext></int00></INT></root>")?;
        std::fs::write(
            root.path().join("Image.LST"),
            b"image\\bg/bg.int\r\nimage\\ev/ev.int\r\n",
        )?;
        let file = fixture();
        std::fs::write(root.path().join("Image/BG/Logo.HG3"), &file)?;
        std::fs::write(
            root.path().join("Bg.INT/Logo.HG3"),
            b"must not read later shadowed payload",
        )?;
        let mut rt = runtime(root.path())?;
        let pool = TaskPool::new("cat-image-lst", 1, 2)?;
        let mut pending =
            Pending::begin_named(&pool, &mut rt.host, ClientId(1), 8, 0x1234, "logo", 37)?.unwrap();
        let Outcome::Ready(frame) = await_frame(&mut pending)? else {
            anyhow::bail!("native list frame not prepared")
        };
        assert_eq!(frame.source, "Image/BG/Logo.HG3");
        assert_eq!(frame.image.rgba().as_ref(), [128; 4]);
        // Once a native member exists, an absent frame is its own result. The
        // search must not continue to a shadowed archive and hide that result.
        let mut absent =
            Pending::begin_named(&pool, &mut rt.host, ClientId(1), 8, 0x1234, "logo", 0)?.unwrap();
        assert!(matches!(await_frame(&mut absent)?, Outcome::MissingFrame));
        std::fs::remove_file(root.path().join("Image/BG/Logo.HG3"))?;
        std::fs::write(root.path().join("Bg.INT/Logo.HG3"), file)?;
        let mut archive =
            Pending::begin_named(&pool, &mut rt.host, ClientId(1), 8, 0x1234, "logo", 37)?.unwrap();
        let Outcome::Ready(frame) = await_frame(&mut archive)? else {
            anyhow::bail!("archive fallback not prepared")
        };
        assert_eq!(frame.source, "Bg.INT/Logo.HG3");
        Ok(())
    }
}
