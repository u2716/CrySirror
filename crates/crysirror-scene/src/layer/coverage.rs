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
    if let Paint::GlyphView {
        source,
        offset,
        range,
    } = &paint
    {
        let source = owner
            .get(source)
            .context("glyph view source layer missing")?;
        let mut value = source.paint.clone();
        let Paint::GlyphRun { glyphs, reveal, .. } = &mut value else {
            bail!("glyph view source is not a positioned glyph run")
        };
        let [start, end] = range.unwrap_or([0, glyphs.len()]);
        ensure!(
            start <= end && end <= glyphs.len(),
            "glyph view range exceeds source run"
        );
        let translation = [source.rect[0] + offset[0], source.rect[1] + offset[1]];
        if freeze {
            if let Some(phase) = reveal.take() {
                glyphs.truncate(phase.status(glyphs.len()).visible);
            }
        } else if range.is_some() {
            if let Some(phase) = reveal {
                let original_initial = phase.initial_visible;
                phase.initial_visible =
                    phase.initial_visible.saturating_sub(start).min(end - start);
                if let Some(steps) = phase.prefix_steps.as_mut() {
                    for step in steps {
                        *step = step.saturating_sub(start).min(end - start);
                    }
                } else {
                    // A subrange must still spend the source's earlier ticks;
                    // independent row clocks would reveal every row at once.
                    phase.prefix_steps = Some(
                        (original_initial + 1..=end)
                            .map(|n| n.saturating_sub(start).min(end - start))
                            .collect(),
                    );
                }
            }
        }
        glyphs.truncate(end.min(glyphs.len()));
        glyphs.drain(..start.min(glyphs.len()));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn glyph_range_views_follow_source_ticks_instead_of_starting_a_row_clock() {
        let mut source: LayerSpec = serde_json::from_value(
            serde_json::json!({"id":1,"rect":[2,3,100,100],"visible":false,
            "paint":{"kind":"glyph_run","font":"fixture","size":12,"glyphs":[
                {"character":"a","baseline":[0,0]},{"character":"b","baseline":[10,0]},
                {"character":"c","baseline":[0,20]},{"character":"d","baseline":[10,20]}],
                "reveal":{"interval_ms":0,"rate_per_second":10}}}),
        )
        .unwrap();
        let view = Paint::GlyphView {
            source: 1,
            offset: [1., 2.],
            range: Some([2, 4]),
        };
        for (elapsed, visible) in [(0, 0), (100, 0), (200, 0), (300, 1), (400, 2)] {
            let Paint::GlyphRun {
                reveal: Some(reveal),
                ..
            } = &mut source.paint
            else {
                panic!()
            };
            reveal.elapsed_ms = elapsed;
            let owner = BTreeMap::from([(1, source.clone())]);
            let (
                Paint::GlyphRun {
                    glyphs,
                    reveal: Some(phase),
                    ..
                },
                _,
            ) = payload(&owner, &view, false).unwrap()
            else {
                panic!()
            };
            assert_eq!(phase.visible_count(glyphs.len()), visible);
            phase.validate(glyphs.len()).unwrap();
            assert_eq!(glyphs[0].character, 'c');
            assert_eq!(glyphs[0].baseline, [3., 25.]);
            let (
                Paint::GlyphRun {
                    glyphs,
                    reveal: None,
                    ..
                },
                _,
            ) = payload(&owner, &view, true).unwrap()
            else {
                panic!()
            };
            assert_eq!(glyphs.len(), visible);
        }
        let owner = BTreeMap::from([(1, source)]);
        assert!(
            payload(
                &owner,
                &Paint::GlyphView {
                    source: 1,
                    offset: [0.; 2],
                    range: Some([2, 5])
                },
                false
            )
            .is_err()
        );
    }
    #[test]
    fn glyph_range_preserves_cluster_steps_and_previously_visible_prefix() {
        let source:LayerSpec=serde_json::from_value(serde_json::json!({"id":1,"rect":[0,0,100,100],
            "paint":{"kind":"glyph_run","font":"fixture","size":12,"glyphs":[
                {"character":"a","baseline":[0,0]},{"character":"b","baseline":[0,0]},
                {"character":"c","baseline":[0,0]},{"character":"d","baseline":[0,0]}],
                "reveal":{"interval_ms":0,"rate_per_second":10,"initial_visible":1,"elapsed_ms":100,"prefix_steps":[3,4]}}})).unwrap();
        let owner = BTreeMap::from([(1, source)]);
        let view = Paint::GlyphView {
            source: 1,
            offset: [0.; 2],
            range: Some([2, 4]),
        };
        let (
            Paint::GlyphRun {
                glyphs,
                reveal: Some(phase),
                ..
            },
            _,
        ) = payload(&owner, &view, false).unwrap()
        else {
            panic!()
        };
        assert_eq!(phase.prefix_steps, Some(vec![1, 2]));
        assert_eq!(phase.visible_count(glyphs.len()), 1);
    }
}
