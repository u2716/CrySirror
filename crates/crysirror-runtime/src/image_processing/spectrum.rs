// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! One retained worker owns source decoding, analysis and image publication.
use super::*;
use crysirror_core::service::*;
struct Query {
    task: Option<Task<SpectrumGraphReceipt>>,
    done: Option<SpectrumGraphReceipt>,
    error: Option<String>,
}
impl Drop for Query {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.cancel();
        }
    }
}
impl SpectrumGraphQuery for Query {
    fn poll(&mut self) -> Result<SpectrumGraphStatus> {
        if let Some(error) = &self.error {
            return Err(Error::Io(error.clone()));
        }
        if let Some(done) = &self.done {
            return Ok(SpectrumGraphStatus::Ready(done.clone()));
        }
        let Some(task) = self.task.as_mut() else {
            return Err(Error::Invalid("spectrum completion"));
        };
        match task.try_take() {
            Ok(None) => Ok(SpectrumGraphStatus::Pending),
            Ok(Some(done)) => {
                self.task = None;
                self.done = Some(done.clone());
                Ok(SpectrumGraphStatus::Ready(done))
            }
            Err(error) => {
                self.task = None;
                let error = format!("spectrum worker: {error:#}");
                self.error = Some(error.clone());
                Err(Error::Io(error))
            }
        }
    }
}
pub(crate) fn begin(
    owner: ClientId,
    sound: Option<Arc<crysirror_audio::resources::PreparedSound>>,
    position: u64,
    input: ImageInput,
    spec: SpectrumGraphSpec,
    state: SpectrumGraphState,
    storage: Arc<dyn ClientStorage>,
    decoders: DecoderRegistry,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<SpectrumGraphJob> {
    let mut nodes = 0;
    let mut graph_bytes = 0;
    let size = input_size(owner, &input, 0, &mut nodes, &mut graph_bytes)?;
    spec.validate(size)?;
    if state.levels.len() > 8192 || state.peaks.len() > 8192 || state.peak_age.len() > 8192 {
        return Err(Error::Limit("spectrum history"));
    }
    let bytes =
        crysirror_image::rgba_len(size[0], size[1]).map_err(|e| Error::Io(e.to_string()))?;
    let output = Charge::new(memory.clone(), bytes)?;
    let encoded_scratch=if has_encoded_dependencies(&input) && matches!(&input,ImageInput::Process{..}|ImageInput::Raster{..}|ImageInput::Composite{..}|ImageInput::Ternary{..}|ImageInput::FieldWarp{..}) {128*1024*1024}else{0};
    let scratch = Charge::new(
        memory.clone(),
        bytes
            .checked_mul(3)
            .and_then(|n| n.checked_add(graph_bytes + encoded_scratch + 8 * 1024 * 1024))
            .ok_or(Error::Limit("spectrum scratch"))?,
    )?;
    let task = pool
        .spawn(move |cancel| {
            let _scratch = scratch;
            cancel.check()?;
            let mut samples = Vec::new();
            if let Some(sound) = sound {
                let mut source = sound.open()?;
                cancel.check()?;
                if source.frame_count().is_none_or(|len| position < len) {
                    source.seek(position)?;
                    samples.resize(2048, [0.; 2]);
                    let mut read = 0;
                    while read < samples.len() {
                        cancel.check()?;
                        let n = source.read(&mut samples[read..])?;
                        anyhow::ensure!(
                            n <= samples.len() - read,
                            "invalid spectrum decoder count"
                        );
                        if n == 0 {
                            break;
                        }
                        read += n;
                    }
                    samples.truncate(read);
                }
            }
            let state = crysirror_audio::spectrum_graph::analyze(&samples, &spec, state, &|| {
                cancel.check()
            })?;
            let mut image = process_input(
                owner,
                &input,
                &[],
                &storage,
                &decoders,
                &cancel,
                Some(&memory),
            )?;
            crysirror_image::spectrum_graph::draw(&mut image, &spec, &state, &|| cancel.check())?;
            let image = resource_from_surface(owner, image, output, &|| cancel.check())?;
            Ok(SpectrumGraphReceipt {
                client: owner,
                image,
                state,
            })
        })
        .map_err(|e| Error::Io(e.to_string()))?;
    Ok(Box::new(Query {
        task: Some(task),
        done: None,
        error: None,
    }))
}
