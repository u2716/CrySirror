// SPDX-License-Identifier: MPL-2.0
//! Font-independent cell layout for terminals and fixed-cell message windows.
//! Callers supply their line-break and indentation policy. Glyph rasterization
//! and reveal clocks remain separate engine services.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellLayoutOptions {
    pub bounds: [i32; 2],
    pub size: i32,
    pub spacing: [i32; 2],
    /// ASCII and half-width katakana occupy half a cell before spacing.
    pub half_width: bool,
    /// Apply spacing before halving (useful for independently sized name runs).
    pub half_spacing: bool,
    /// Maximum extra width allowed for forbidden line-start punctuation.
    pub hanging: i32,
    pub forbidden_line_start: String,
    pub indent_pairs: Vec<[char; 2]>,
    /// Explicit cell widths (before spacing) for listed scalars, such as
    /// inline picture glyphs; they replace the full/half-width rule.
    #[serde(default)]
    pub cell_widths: Vec<(char, i32)>,
}
impl CellLayoutOptions {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.bounds.iter().all(|v| (1..=32767).contains(v)),
            "cell layout bounds"
        );
        ensure!((1..=512).contains(&self.size), "cell layout size");
        ensure!(
            self.spacing.iter().all(|v| (-511..=4096).contains(v))
                && self.size + self.spacing[1] > 0,
            "cell layout spacing"
        );
        ensure!(
            (0..=512).contains(&self.hanging)
                && self.forbidden_line_start.len() <= 4096
                && self.indent_pairs.len() <= 32,
            "cell layout break policy"
        );
        ensure!(
            self.cell_widths.len() <= 65536
                && self.cell_widths.iter().all(|(_, w)| (0..=4096).contains(w)),
            "cell layout width overrides"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellCursor {
    pub position: [i32; 2],
    pub indent: i32,
    pub opening: Option<char>,
    pub nesting: u32,
    pub line_head: bool,
}
impl CellCursor {
    pub fn at_start() -> Self {
        Self {
            line_head: true,
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellGlyph {
    pub character: char,
    pub position: [i32; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellLayout {
    pub glyphs: Vec<CellGlyph>,
    /// UTF-8 byte boundary of the first scalar which did not fit.
    pub consumed: usize,
}

/// One indivisible cell with explicit glyph offsets and an independent
/// signed cursor advance. A two-character cell can keep its internal pitch
/// while applying spacing once to the complete group. Offsets are local to
/// the current cursor; no shaping or byte-offset interpretation occurs here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellGroup {
    pub glyphs: Vec<CellGlyph>,
    pub advance: i32,
    /// Optional local cursor positioning before this group (for example a
    /// terminal address or an independently positioned name/header run).
    pub reset_before: Option<[i32; 2]>,
    /// Request an ordinary bounds-based wrap before this complete group.
    /// False lets the caller provide explicit language-specific break hints.
    pub automatic_wrap: bool,
    pub break_before: bool,
    pub break_after: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupedCellLayout {
    pub glyphs: Vec<CellGlyph>,
    /// Number of complete input groups admitted; never a byte/glyph index.
    pub consumed_groups: usize,
}

/// Place caller-defined cells with bounded offsets/advances, explicit break
/// hints and optional cyclic vertical bounds. Empty groups may request a line
/// break. Input is visited once, including zero/negative advances; neither can
/// cause a retry loop. Existing scalar cell-layout APIs keep their semantics.
pub fn layout_cell_groups(
    groups: &[CellGroup],
    options: &CellLayoutOptions,
    cursor: &mut CellCursor,
    cyclic: bool,
) -> Result<GroupedCellLayout> {
    options.validate()?;
    ensure!(groups.len() <= 65536, "grouped cell group budget");
    let mut total = 0usize;
    for group in groups {
        total = total
            .checked_add(group.glyphs.len())
            .context("grouped cell glyph count")?;
        ensure!(
            total <= 65536
                && (-32767..=32767).contains(&group.advance)
                && group
                    .glyphs
                    .iter()
                    .all(|g| g.position.iter().all(|v| (-32767..=32767).contains(v))),
            "grouped cell glyph/advance bounds"
        );
        ensure!(
            group
                .reset_before
                .is_none_or(|p| p.iter().all(|v| (-32767..=32767).contains(v))),
            "grouped cell cursor reset bounds"
        );
    }
    ensure!(
        cursor.position.iter().all(|v| (-32767..=32767).contains(v))
            && (0..=32767).contains(&cursor.indent),
        "grouped cell cursor bounds"
    );
    if cyclic && cursor.position[1] >= options.bounds[1] {
        cursor.position[1] = 0;
    }
    let newline = |cursor: &mut CellCursor| -> Result<()> {
        let y = cursor.position[1]
            .checked_add(options.size + options.spacing[1])
            .context("grouped cell vertical overflow")?;
        cursor.position = [
            cursor.indent,
            if cyclic && y >= options.bounds[1] {
                0
            } else {
                y
            },
        ];
        cursor.line_head = true;
        Ok(())
    };
    let mut glyphs = Vec::with_capacity(total);
    for (index, group) in groups.iter().enumerate() {
        if let Some(position) = group.reset_before {
            cursor.position = position;
            cursor.line_head = true;
        }
        let next = cursor.position[0]
            .checked_add(group.advance)
            .context("grouped cell advance overflow")?;
        if group.break_before
            || (group.automatic_wrap
                && cursor.position[0] > cursor.indent
                && next > options.bounds[0].saturating_add(options.hanging))
        {
            newline(cursor)?;
        }
        if cursor.position[1] >= options.bounds[1] {
            return Ok(GroupedCellLayout {
                glyphs,
                consumed_groups: index,
            });
        }
        for glyph in &group.glyphs {
            let position = [
                cursor.position[0].checked_add(glyph.position[0]),
                cursor.position[1].checked_add(glyph.position[1]),
            ];
            let position = [
                position[0].context("grouped glyph horizontal overflow")?,
                position[1].context("grouped glyph vertical overflow")?,
            ];
            ensure!(
                position.iter().all(|v| (-65535..=65535).contains(v)),
                "grouped glyph coordinate bounds"
            );
            glyphs.push(CellGlyph {
                character: glyph.character,
                position,
            });
        }
        cursor.position[0] = cursor.position[0]
            .checked_add(group.advance)
            .context("grouped cell horizontal overflow")?;
        ensure!(
            (-32767..=32767).contains(&cursor.position[0]),
            "grouped cell horizontal bounds"
        );
        if !group.glyphs.is_empty() {
            cursor.line_head = false;
        }
        if group.break_after {
            newline(cursor)?;
        }
    }
    Ok(GroupedCellLayout {
        glyphs,
        consumed_groups: groups.len(),
    })
}

/// Append cells from an immutable policy and a caller-owned logical cursor.
/// Returns overflow explicitly rather than truncating the caller's text.
/// ASCII BEL is an explicit line break which suppresses hanging indentation.
pub fn layout_cells(
    text: &str,
    options: &CellLayoutOptions,
    cursor: &mut CellCursor,
) -> Result<CellLayout> {
    layout_cells_with_breaks(text,options,cursor,"")
}

/// Fixed-cell layout with an explicit set of characters forbidden at the end
/// of an automatically wrapped line. Existing callers retain their profile.
pub fn layout_cells_with_breaks(text:&str,options:&CellLayoutOptions,cursor:&mut CellCursor,forbidden_line_end:&str)->Result<CellLayout>{
    layout_cells_impl(text, options, cursor, forbidden_line_end, false)
}

/// Append fixed cells, returning to the first row when the vertical cursor
/// reaches the bottom. Older glyphs remain in draw order and may be covered
/// by subsequent cells; this does not clear, rasterize or scroll a surface.
/// Uses the same CR/LF, width, indentation and caller-supplied forbidden-break
/// policy as the non-cyclic path. The existing 1MiB/65536-scalar bounds apply.
pub fn layout_cells_cyclic(
    text: &str,
    options: &CellLayoutOptions,
    cursor: &mut CellCursor,
    forbidden_line_end: &str,
) -> Result<CellLayout> {
    layout_cells_impl(text, options, cursor, forbidden_line_end, true)
}

fn layout_cells_impl(
    text: &str,
    options: &CellLayoutOptions,
    cursor: &mut CellCursor,
    forbidden_line_end: &str,
    cyclic: bool,
) -> Result<CellLayout> {
    ensure!(forbidden_line_end.len()<=4096,"cell line-end policy limit");
    options.validate()?;
    ensure!(
        text.len() <= 1024 * 1024 && text.chars().count() <= 65536,
        "cell text limit"
    );
    ensure!(
        cursor.position.iter().all(|v| *v >= 0)
            && (0..=32767).contains(&cursor.indent)
            && cursor.nesting <= 65536,
        "cell cursor limit"
    );
    if cyclic && cursor.position[1] >= options.bounds[1] {
        cursor.position[1] = 0;
    }
    if cyclic {
        ensure!(cursor.position[0] <= 32767, "cyclic cell horizontal cursor limit");
    }
    let newline = |state: &mut CellCursor, indented: bool| -> Result<()> {
        let next_y = if cyclic {
            state.position[1].checked_add(options.size + options.spacing[1])
                .context("cyclic cell vertical cursor overflow")?
        } else {
            state.position[1].saturating_add(options.size + options.spacing[1])
        };
        state.position = [
            if indented { state.indent } else { 0 },
            if cyclic && next_y >= options.bounds[1] { 0 } else { next_y },
        ];
        state.line_head = true;
        Ok(())
    };
    let mut glyphs = Vec::new();
    let mut input=text.char_indices().peekable();
    while let Some((byte,character))=input.next() {
        if character == '\r' {
            continue;
        }
        if character == '\n' || character == '\u{7}' {
            newline(cursor, character == '\n')?;
            continue;
        }
        let half = options.half_width
            && (character.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&character));
        let explicit = options
            .cell_widths
            .iter()
            .find(|(c, _)| *c == character)
            .map(|(_, w)| *w);
        let cell = if let Some(width) = explicit {
            width
        } else if half {
            (options.size / 2).max(1)
        } else {
            options.size
        };
        let advance = if explicit.is_none() && half && options.half_spacing {
            (options.size + options.spacing[0]) / 2
        } else {
            cell + options.spacing[0]
        };
        let next = if cyclic {
            cursor.position[0].checked_add(advance)
                .context("cyclic cell horizontal cursor overflow")?
        } else {
            cursor.position[0].saturating_add(advance)
        };
        let end_wrap=cursor.position[0]>0 && forbidden_line_end.contains(character) && input.peek().is_some_and(|(_,next_character)| {
            let next_half=options.half_width&&(next_character.is_ascii()||('｡'..='ﾟ').contains(next_character));
            let next_advance=if next_half{(options.size/2).max(1)}else{options.size}+options.spacing[0];
            next.saturating_add(next_advance)>options.bounds[0]
        });
        let wrap = end_wrap || next > options.bounds[0].saturating_add(options.hanging)
            || (next > options.bounds[0] && !options.forbidden_line_start.contains(character));
        if wrap {
            newline(cursor, true)?;
            if character == ' ' || character == '\u{3000}' {
                continue;
            }
        }
        if cursor.position[1] >= options.bounds[1] {
            return Ok(CellLayout {
                glyphs,
                consumed: byte,
            });
        }
        glyphs.push(CellGlyph {
            character,
            position: cursor.position,
        });
        cursor.position[0] = if cyclic {
            cursor.position[0].checked_add(advance.max(1))
                .context("cyclic cell horizontal cursor overflow")?
        } else {
            cursor.position[0].saturating_add(advance.max(1))
        };
        if let Some(pair) = options
            .indent_pairs
            .iter()
            .find(|pair| pair[0] == character)
        {
            if cursor.line_head {
                cursor.indent = cursor.position[0];
                cursor.opening = Some(pair[0]);
                cursor.nesting = 1;
            } else if cursor.opening == Some(pair[0]) {
                cursor.nesting = cursor.nesting.saturating_add(1);
            }
        }
        if cursor.opening.is_some_and(|open| {
            options
                .indent_pairs
                .iter()
                .any(|pair| pair == &[open, character])
        }) {
            cursor.nesting = cursor.nesting.saturating_sub(1);
            if cursor.nesting == 0 {
                cursor.indent = 0;
                cursor.opening = None;
            }
        }
        cursor.line_head = false;
    }
    Ok(CellLayout {
        glyphs,
        consumed: text.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> CellLayoutOptions {
        CellLayoutOptions { cell_widths: Vec::new(),
            bounds: [39, 60],
            size: 20,
            spacing: [-1, 10],
            half_width: true,
            half_spacing: false,
            hanging: 20,
            forbidden_line_start: "。、」".into(),
            indent_pairs: vec![['「', '」']],
        }
    }
    #[test]
    fn half_cells_hanging_and_append_share_the_same_cursor() {
        let mut cursor = CellCursor::at_start();
        let a = layout_cells("Aあ。", &options(), &mut cursor).unwrap();
        assert_eq!(
            a.glyphs.iter().map(|g| g.position).collect::<Vec<_>>(),
            [[0, 0], [9, 0], [28, 0]]
        );
        let b = layout_cells("う", &options(), &mut cursor).unwrap();
        assert_eq!(b.glyphs[0].position, [0, 30]);
    }
    #[test]
    fn nested_hanging_indent_and_explicit_breaks() {
        let mut cursor = CellCursor::at_start();
        let result = layout_cells(
            "「あ\nい」\u{7}う",
            &CellLayoutOptions { cell_widths: Vec::new(),
                bounds: [100, 100],
                ..options()
            },
            &mut cursor,
        )
        .unwrap();
        assert_eq!(result.glyphs[2].position, [19, 30]);
        assert_eq!(result.glyphs[4].position, [0, 60]);
        assert_eq!(cursor.indent, 0);
    }
    #[test]
    fn overflow_is_a_valid_utf8_suffix_and_does_not_emit_that_scalar() {
        let mut cursor = CellCursor::at_start();
        let input = "あいうえお";
        let result = layout_cells(
            input,
            &CellLayoutOptions { cell_widths: Vec::new(),
                bounds: [39, 30],
                ..options()
            },
            &mut cursor,
        )
        .unwrap();
        assert_eq!(result.consumed, 6);
        assert_eq!(&input[result.consumed..], "うえお");
        assert_eq!(result.glyphs.len(), 2);
    }
}

/// Native code-page cell placement: one encoded byte advances half a cell,
/// any multi-byte scalar one whole cell. Glyph width and spacing stay integer.
pub fn encoded_cell_run(
    text: &str,
    origin: [i32; 2],
    size: i32,
    encoding: &'static encoding_rs::Encoding,
) -> Result<(Vec<CellGlyph>, i32)> {
    ensure!(
        (1..=512).contains(&size) && text.len() <= 1024 * 1024 && text.chars().count() <= 65536,
        "encoded cell run budget"
    );
    let mut x = origin[0];
    let mut glyphs = Vec::with_capacity(text.chars().count());
    for character in text.chars() {
        glyphs.push(CellGlyph {
            character,
            position: [x, origin[1]],
        });
        let mut buffer = [0; 4];
        let bytes = encoding.encode(character.encode_utf8(&mut buffer)).0;
        let step = if bytes.len() > 1 { size } else { size / 2 };
        x = x
            .checked_add(step)
            .ok_or_else(|| anyhow::anyhow!("encoded cell advance overflow"))?;
    }
    Ok((glyphs, x - origin[0]))
}

/// Two encoded fixed-cell runs centered in a common advance box. This is a
/// font-independent placement operation; native delimiters, relative line
/// offsets, colors and reveal order remain the caller's semantic policy.
pub fn encoded_ruby_run(
    base: &str,
    reading: &str,
    origin: [i32; 2],
    base_size: i32,
    reading_size: i32,
    reading_offset_y: i32,
    encoding: &'static encoding_rs::Encoding,
) -> Result<(Vec<CellGlyph>, Vec<CellGlyph>, i32)> {
    ensure!(!base.is_empty() && !reading.is_empty(), "encoded ruby requires base and reading");
    ensure!(!base.contains(['\n','\r']) && !reading.contains(['\n','\r']), "encoded ruby requires single lines");
    let (mut base_cells, base_width) = encoded_cell_run(base, origin, base_size, encoding)?;
    let reading_y = origin[1].checked_add(reading_offset_y).ok_or_else(|| anyhow::anyhow!("encoded ruby vertical overflow"))?;
    let (mut reading_cells, reading_width) = encoded_cell_run(reading, [origin[0],reading_y], reading_size, encoding)?;
    let advance = base_width.max(reading_width);
    for (cells, width) in [(&mut base_cells,base_width),(&mut reading_cells,reading_width)] {
        let offset = (advance-width)/2;
        for cell in cells {
            cell.position[0] = cell.position[0].checked_add(offset).ok_or_else(|| anyhow::anyhow!("encoded ruby horizontal overflow"))?;
        }
    }
    Ok((base_cells,reading_cells,advance))
}

#[cfg(test)]
mod ruby_tests {
    use super::*;
    #[test]
    fn encoded_ruby_centers_shorter_run_and_preserves_odd_integer_remainder() {
        let (base,reading,advance)=encoded_ruby_run("A","あいう",[4,20],22,12,-9,encoding_rs::SHIFT_JIS).unwrap();
        assert_eq!(advance,36);
        assert_eq!(base[0].position,[16,20]);
        assert_eq!(reading[0].position,[4,11]);
        assert_eq!(reading[2].position,[28,11]);
    }
    #[test]
    fn encoded_ruby_rejects_bad_geometry_and_keeps_halfwidth_encoding() {
        assert!(encoded_ruby_run("","x",[0,0],22,12,-9,encoding_rs::SHIFT_JIS).is_err());
        assert!(encoded_ruby_run("a\nb","x",[0,0],22,12,-9,encoding_rs::SHIFT_JIS).is_err());
        assert!(encoded_ruby_run("a","x",[0,i32::MIN],22,12,-9,encoding_rs::SHIFT_JIS).is_err());
        let (_,reading,advance)=encoded_ruby_run("あ","ｱA",[0,0],22,12,-9,encoding_rs::SHIFT_JIS).unwrap();
        assert_eq!(advance,22);assert_eq!(reading[1].position[0]-reading[0].position[0],6);
    }
}
