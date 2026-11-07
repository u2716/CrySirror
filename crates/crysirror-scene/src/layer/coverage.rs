//! Client-scoped ordered coverage targets. Logical layer declarations retain
//! commands; renderers rebuild the transient coverage plane for each frame.
use super::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Operation {
    #[default]
    None,
    Write,
    Reset {
        value: u8,
    },
}
/// Resolve a retained glyph view without duplicating its reveal clock. Every
/// view observes the source's current engine-owned run at extraction time.
pub(super) fn payload(
    owner: &BTreeMap<u32, LayerSpec>,
    paint: &Paint,
    freeze: bool,
) -> Result<(Paint, Operation)> {
    let mut paint = paint.clone();
    let operation = match paint {
        Paint::CoverageWrite { paint: inner } => {
            paint = *inner;
            Operation::Write
        }
        Paint::CoverageReset { value } => {
            return Ok((Paint::Solid { color: [0; 4] }, Operation::Reset { value }));
        }
        _ => Operation::None,
    };
    if let Paint::GlyphView { source, offset } = &paint {
        let source = owner
            .get(source)
            .context("glyph view source layer missing")?;
        let mut value = source.paint.clone();
        let Paint::GlyphRun { glyphs, reveal, .. } = &mut value else {
            bail!("glyph view source is not a positioned glyph run")
        };
        let translation = [source.rect[0] + offset[0], source.rect[1] + offset[1]];
        if freeze {
            if let Some(phase) = reveal.take() {
                glyphs.truncate(phase.status(glyphs.len()).visible);
            }
        }
        for glyph in glyphs {
            for axis in 0..2 {
                glyph.baseline[axis] += translation[axis];
            }
            if let Some(crop) = &mut glyph.crop {
                for axis in 0..2 {
                    crop[axis] += translation[axis];
                }
            }
        }
        paint = value;
    }
    Ok((paint, operation))
}
