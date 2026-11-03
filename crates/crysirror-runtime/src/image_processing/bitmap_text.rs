//! Scoped font and image leases execute together on a bounded engine worker.
use super::*;
use crysirror_text::TextSystem;
pub(crate) fn begin(
    owner: ClientId,
    input: ImageInput,
    spec: BitmapTextSpec,
    text: TextSystem,
    storage: Arc<dyn ClientStorage>,
    decoders: DecoderRegistry,
    pool: TaskPool,
    memory: Arc<AtomicUsize>,
) -> Result<ImageJob> {
    crysirror_text::bitmap_raster::validate(&spec).map_err(|e| Error::Io(e.to_string()))?;
    if matches!(
        input,
        ImageInput::Raster { .. } | ImageInput::Ternary { .. } | ImageInput::Transition { .. }
    ) {
        return Err(Error::Invalid("bitmap text pair input"));
    }
    let mut nodes = 0;
    let mut dependencies = 0;
    let size = input_size(owner, &input, 0, &mut nodes, &mut dependencies)?;
    let bytes =
        crysirror_image::rgba_len(size[0], size[1]).map_err(|e| Error::Io(e.to_string()))?;
    let text = text
        .for_client(owner)
        .frozen_font(&spec.font)
        .map_err(|e| Error::Io(e.to_string()))?;
    let spread = spec
        .shadow
        .map_or([0; 2], |s| s.offset.map(|n| usize::from(n) * 2));
    let record = (usize::from(spec.size) * 4 + spread[0] + 16)
        .checked_mul(usize::from(spec.size) * 4 + spread[1] + 16)
        .and_then(|n| n.checked_mul(12))
        .ok_or(Error::Limit("bitmap text scratch"))?;
    let output = Charge::new(memory.clone(), bytes)?;
    let scratch = Charge::new(
        memory.clone(),
        bytes * 3
            + dependencies
            + record.max(if spec.shadow.is_some() {
                144 * 1024 * 1024
            } else {
                16 * 1024 * 1024
            })
            + spec.glyphs.len() * 64,
    )?;
    let task = pool
        .reserve()
        .map_err(|e| Error::Io(e.to_string()))?
        .spawn(move |cancel| {
            let _scratch = scratch;
            cancel.check()?;
            let mut surface =
                load_inner(owner, &input, &storage, &decoders, &cancel, Some(&memory))?;
            text.rasterize_bitmap_records(&mut surface, &spec, &|| cancel.check())?;
            resource_from_surface(owner, surface, output, &|| cancel.check())
        })
        .map_err(|e| Error::Io(e.to_string()))?;
    Ok(Box::new(Job {
        task: Some(task),
        state: ImageProcessState::Pending,
    }))
}
