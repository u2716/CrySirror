// SPDX-License-Identifier: MPL-2.0
//! Font-independent styled cell streams with explicit moves and unit wrapping.
//! Integer placement follows the whole object-string layout in siglus_rs
//! 5f358ef1360b0439e92aed2b749e8f25d8721954, runtime/forms/stage.rs.
//! Native escape syntax, palettes and resource lifetimes are caller policy.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

pub const MAX_FLOW_ITEMS: usize = 65_536;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitFlowOptions {
    #[serde(default)]
    pub vertical: bool,
    /// Advance spacing and line/column spacing, in native pixel units.
    #[serde(default)]
    pub spacing: [i32; 2],
    /// One full cell consumes two units. Zero disables automatic wrapping.
    #[serde(default)]
    pub wrap_units: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UnitFlowItem {
    Glyph {
        character: char,
        size: u16,
        /// One half cell or two units for a full cell. Half advances round up.
        units: u8,
        /// Opaque style index, never a font/device pointer.
        #[serde(default)]
        style: u32,
    },
    LineBreak {
        size: u16,
    },
    Move {
        #[serde(default)]
        x: Option<i64>,
        #[serde(default)]
        y: Option<i64>,
        #[serde(default)]
        relative: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitFlowGlyph {
    pub character: char,
    /// Top-left logical cell origin. Raster ink bounds are a separate result.
    pub position: [i32; 2],
    pub size: u16,
    pub style: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitFlowLayout {
    pub glyphs: Vec<UnitFlowGlyph>,
    /// Native cell bounds include the zero origin and use full-cell extents
    /// even for half-width advances. They are not a measurement of raster ink.
    pub extent: [u32; 2],
    pub minimum: [i64; 2],
    pub maximum: [i64; 2],
    pub cursor: [i64; 2],
    pub units_on_line: u64,
}

/// ASCII and Unicode half-width katakana use one unit in a classic cell grid.
pub fn half_cell(character: char) -> bool {
    character.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&character)
}

/// Pure bounded layout. Explicit moves do not reset the wrap-unit counter;
/// line breaks do. Negative spacing/positions remain meaningful. No font
/// lookup, shaping, rasterization or native markup parsing happens here.
pub fn layout_unit_flow(
    items: &[UnitFlowItem],
    options: UnitFlowOptions,
) -> Result<UnitFlowLayout> {
    ensure!(items.len() <= MAX_FLOW_ITEMS, "unit flow item budget");
    // Validate the entire stream before any output, including unused breaks.
    for item in items {
        match item {
            UnitFlowItem::Glyph { size, units, .. } => {
                ensure!(
                    (1..=512).contains(size) && (1..=2).contains(units),
                    "unit flow cell size/units"
                );
            }
            UnitFlowItem::LineBreak { size } => {
                ensure!((1..=512).contains(size), "unit flow break size");
            }
            UnitFlowItem::Move { x, y, .. } => {
                ensure!(x.is_some() || y.is_some(), "unit flow empty move")
            }
        }
    }
    let mut glyphs = Vec::new();
    let mut cursor = [0i64; 2];
    let mut minimum = [0i64; 2];
    let mut maximum = [1i64; 2];
    let mut units_on_line = 0u64;
    let newline = |cursor: &mut [i64; 2], size: u16| {
        let advance = i64::from(size) + i64::from(options.spacing[1]);
        if options.vertical {
            cursor[0] = cursor[0].saturating_sub(advance);
            cursor[1] = 0;
        } else {
            cursor[0] = 0;
            cursor[1] = cursor[1].saturating_add(advance);
        }
    };
    for item in items {
        match *item {
            UnitFlowItem::Move { x, y, relative } => {
                for (axis, value) in [x, y].into_iter().enumerate() {
                    if let Some(value) = value {
                        cursor[axis] = if relative {
                            cursor[axis].saturating_add(value)
                        } else {
                            value
                        };
                    }
                }
            }
            UnitFlowItem::LineBreak { size } => {
                newline(&mut cursor, size);
                units_on_line = 0;
            }
            UnitFlowItem::Glyph {
                character,
                size,
                units,
                style,
            } => {
                if options.wrap_units > 0
                    && units_on_line > 0
                    && units_on_line.saturating_add(u64::from(units))
                        > u64::from(options.wrap_units)
                {
                    newline(&mut cursor, size);
                    units_on_line = 0;
                }
                glyphs.push(UnitFlowGlyph {
                    character,
                    size,
                    style,
                    position: cursor
                        .map(|v| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32),
                });
                for axis in 0..2 {
                    minimum[axis] = minimum[axis].min(cursor[axis]);
                    maximum[axis] = maximum[axis].max(cursor[axis].saturating_add(i64::from(size)));
                }
                units_on_line = units_on_line.saturating_add(u64::from(units));
                let advance = if units == 1 {
                    (i64::from(size) + 1) / 2
                } else {
                    i64::from(size)
                } + i64::from(options.spacing[0]);
                let axis = usize::from(options.vertical);
                cursor[axis] = cursor[axis].saturating_add(advance);
            }
        }
    }
    Ok(UnitFlowLayout {
        glyphs,
        minimum,
        maximum,
        cursor,
        units_on_line,
        extent: std::array::from_fn(|i| {
            maximum[i]
                .saturating_sub(minimum[i])
                .clamp(1, i64::from(u32::MAX)) as u32
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn glyph(character: char, size: u16) -> UnitFlowItem {
        UnitFlowItem::Glyph {
            character,
            size,
            units: if half_cell(character) { 1 } else { 2 },
            style: 7,
        }
    }
    #[test]
    fn odd_half_cells_wrap_by_units_after_size_and_absolute_move_changes() {
        let items = [
            glyph('A', 13),
            glyph('あ', 13),
            UnitFlowItem::Move {
                x: Some(80),
                y: None,
                relative: false,
            },
            glyph('B', 9),
            glyph('C', 20),
        ];
        let result = layout_unit_flow(
            &items,
            UnitFlowOptions {
                wrap_units: 4,
                spacing: [-2, 3],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            result.glyphs.iter().map(|g| g.position).collect::<Vec<_>>(),
            [[0, 0], [5, 0], [80, 0], [0, 23]]
        );
        assert_eq!(result.extent, [89, 43]);
        assert_eq!(result.units_on_line, 1);
        assert_eq!(result.glyphs[2].style, 7);
    }
    #[test]
    fn vertical_negative_spacing_explicit_breaks_and_saturated_moves_are_distinct() {
        let items = [
            glyph('ｱ', 13),
            glyph('あ', 13),
            UnitFlowItem::LineBreak { size: 20 },
            glyph('B', 9),
            UnitFlowItem::Move {
                x: Some(i64::MIN),
                y: Some(i64::MAX),
                relative: false,
            },
            glyph('Z', 12),
            UnitFlowItem::Move {
                x: Some(-1),
                y: Some(1),
                relative: true,
            },
            glyph('Y', 12),
        ];
        let result = layout_unit_flow(
            &items,
            UnitFlowOptions {
                vertical: true,
                spacing: [-8, 2],
                wrap_units: 0,
            },
        )
        .unwrap();
        assert_eq!(result.glyphs[1].position, [0, -1]);
        assert_eq!(result.glyphs[2].position, [-22, 0]);
        assert_eq!(result.glyphs[3].position, [i32::MIN, i32::MAX]);
        assert_eq!(result.glyphs[4].position, [i32::MIN, i32::MAX]);
        assert_eq!(result.extent, [u32::MAX, u32::MAX]);
        assert_eq!(result.units_on_line, 3);
    }
    #[test]
    fn malformed_streams_and_empty_output_have_explicit_bounded_results() {
        let empty = layout_unit_flow(&[], Default::default()).unwrap();
        assert_eq!(empty.extent, [1, 1]);
        for item in [
            UnitFlowItem::Glyph {
                character: 'A',
                size: 0,
                units: 1,
                style: 0,
            },
            UnitFlowItem::Glyph {
                character: 'A',
                size: 12,
                units: 0,
                style: 0,
            },
            UnitFlowItem::LineBreak { size: 513 },
            UnitFlowItem::Move {
                x: None,
                y: None,
                relative: false,
            },
        ] {
            assert!(layout_unit_flow(&[item], Default::default()).is_err());
        }
        assert!(
            layout_unit_flow(
                &vec![glyph('A', 12); MAX_FLOW_ITEMS + 1],
                Default::default()
            )
            .is_err()
        );
    }
}
