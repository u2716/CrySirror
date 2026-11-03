//! Header preparation and decode have separate, owner-polled admission fences.
use super::*;
struct EncodedJob {
    owner: ClientId,
    name: String,
    bytes: Arc<[u8]>,
    storage: Arc<dyn ClientStorage>,
    decoders: DecoderRegistry,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
    header: Option<Task<usize>>,
    decode: Option<Task<Arc<dyn ImageResource>>>,
    input: Option<Charge>,
    state: ImageProcessState,
}
pub(crate) fn begin(
    owner: ClientId,
    name: &str,
    bytes: Arc<[u8]>,
    storage: Arc<dyn ClientStorage>,
    decoders: DecoderRegistry,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<ImageJob> {
    crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Asset, name)?;
    if bytes.is_empty() || bytes.len() > 64 * 1024 * 1024 {
        return Err(Error::Limit("encoded image byte budget"));
    }
    let input = Charge::new(memory.clone(), bytes.len() + 1024 * 1024)?;
    let source = bytes.clone();
    let header = pool
        .reserve()
        .map_err(|e| Error::Io(e.to_string()))?
        .spawn(move |cancel| {
            cancel.check()?;
            let hint = crysirror_image::encoded_dimensions_hint(&source)
                .and_then(|size| crysirror_image::rgba_len(size[0], size[1]).ok())
                .unwrap_or(crysirror_image::MAX_DECODED_BYTES);
            cancel.check()?;
            Ok(hint)
        })
        .map_err(|e| Error::Io(e.to_string()))?;
    Ok(Box::new(EncodedJob {
        owner,
        name: name.into(),
        bytes,
        storage,
        decoders,
        pool,
        memory,
        header: Some(header),
        decode: None,
        input: Some(input),
        state: ImageProcessState::Pending,
    }))
}
impl EncodedJob {
    fn poll_inner(&mut self) -> anyhow::Result<()> {
        if let Some(header) = &mut self.header {
            let Some(size) = header.try_take()? else {
                return Ok(());
            };
            self.header = None;
            let mut output = Charge::new(self.memory.clone(), size)?;
            let scratch = Charge::new(self.memory.clone(), 3 * size)?;
            let owner = self.owner;
            let name = self.name.clone();
            let bytes = self.bytes.clone();
            let storage = self.storage.clone();
            let decoders = self.decoders.clone();
            let input = self.input.take();
            self.decode = Some(self.pool.reserve()?.spawn(move |cancel| {
                let _input = input;
                let _scratch = scratch;
                cancel.check()?;
                let context =
                    crysirror_resource::DecodeContext::new(owner).with_cancellation(cancel.clone());
                let source = |path: &str| {
                    let path = crysirror_core::path::VfsPath::new(
                        crysirror_core::path::Mount::Asset,
                        path,
                    )?;
                    storage.read(&path)
                };
                let image =
                    decoders.decode_with(&name, &bytes, Some(&source), Some(&context), None)?;
                image.validate()?;
                anyhow::ensure!(
                    image.rgba.len() <= size,
                    "encoded image exceeded its admitted dimensions"
                );
                cancel.check()?;
                output.shrink(image.rgba.len());
                resource_from_surface(owner, image, output, &|| cancel.check())
            })?);
        }
        if let Some(decode) = &mut self.decode {
            if let Some(image) = decode.try_take()? {
                self.decode = None;
                self.bytes = Arc::from([]);
                self.state = ImageProcessState::Ready(image);
            }
        }
        Ok(())
    }
}
impl ImageProcess for EncodedJob {
    fn poll(&mut self) -> Result<ImageProcessState> {
        if matches!(&self.state, ImageProcessState::Pending) {
            if let Err(error) = self.poll_inner() {
                self.header = None;
                self.decode = None;
                self.input = None;
                self.bytes = Arc::from([]);
                self.state = ImageProcessState::Failed(error.to_string());
            }
        }
        Ok(self.state.clone())
    }
}
