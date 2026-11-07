// SPDX-License-Identifier: MPL-2.0
//! Native CP932 two-byte cells and 439F00/43C010/44BEA0 break policy.
//! Only native grouping/break decisions live here: shared grid layout owns
//! positions, signed advances, bounds and cyclic cursor movement. Each group's
//! Unicode scalars maps directly to the returned shared CellGlyph entries;
//! source byte offsets never index shaped glyphs or reveal clocks.
use super::*;
use crysirror_text::grid::{CellGlyph, CellGroup};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Profile {
    Live,
    Redraw,
    Measure,
    Backlog,
    Body,
}

pub(crate) struct Groups {
    pub cells: Vec<CellGroup>,
    pub column: i32,
    pub row: i32,
    pub ordinary: bool,
    pub last_break: bool,
    pub headers: Vec<usize>,
}
fn punctuation(value: u16) -> bool {
    value.wrapping_add(32511) <= 254 && (value < 0x8150 || value < 0x817c && value & 1 == 0)
}
fn small(value: u16) -> bool {
    (value.wrapping_add(32097) <= 9 && value & 1 != 0)
        || (value.wrapping_add(32031) <= 2 && value & 1 != 0)
        || matches!(value, 0x82ec | 0x82c1 | 0x82e5)
        || (value.wrapping_add(31936) <= 8 && value & 1 == 0)
        || (value.wrapping_add(31869) <= 4 && value & 1 != 0)
        || matches!(value, 0x838e | 0x8395 | 0x8396 | 0x8362)
}
fn wrap(bytes: &[u8], at: usize, column: i32, columns: i32) -> bool {
    //Native reads four lookahead bytes even at its terminal NUL. Zero padding
    //defines those out-of-buffer reads without reading unrelated process data.
    let byte = |i| bytes.get(at + i).copied().unwrap_or(0);
    let next = u16::from_be_bytes([byte(0), byte(1)]);
    let following = u16::from_be_bytes([byte(2), byte(3)]);
    if column < columns {
        return column >= columns - 1
            && next.wrapping_add(32511) <= 254
            && next.wrapping_add(32411) <= 22
            && next & 1 != 0;
    }
    if matches!(next, 0x5c6e | 0x5c4e)
        || byte(0) == 0
        || (byte(0) == b'@' && column <= columns)
        || (punctuation(next) && column <= columns)
    {
        return false;
    }
    let keep = following == 0x5c6e || (!punctuation(following) && column <= columns);
    if (matches!(next, 0x8158 | 0x815b | 0x8163) || small(next)) && keep {
        return false;
    }
    true
}
fn line(cells: &mut Vec<CellGroup>, column: &mut i32, row: &mut i32, rows: i32) {
    cells.push(CellGroup {
        glyphs: vec![],
        advance: 0,
        reset_before: None,
        automatic_wrap: false,
        break_before: false,
        break_after: true,
    });
    *column = 0;
    *row = (*row + 1) % rows;
    if *row == 0 {
        reset(cells);
    }
}
fn reset(cells: &mut Vec<CellGroup>) {
    cells.push(CellGroup {
        glyphs: vec![],
        advance: 0,
        reset_before: Some([0, 0]),
        automatic_wrap: false,
        break_before: false,
        break_after: false,
    });
}
///43C560 consumes raw label pairs without body escape interpretation.
pub(crate) fn name_groups(name: &str, size: i32, spacing: i32) -> Result<Vec<CellGroup>> {
    let (bytes, _, invalid) = encoding_rs::SHIFT_JIS.encode(name);
    ensure!(
        !invalid && bytes.len() < 0x10000,
        "SugarPot name CP932/budget"
    );
    ensure!(
        !bytes.contains(&b'@'),
        "SugarPot FONT00/FONT01 bitmap glyph source not connected"
    );
    let mut at = 0;
    let mut groups = vec![];
    while at < bytes.len() {
        let width = if dbcs_lead(bytes[at]) {
            2
        } else if at + 1 < bytes.len() && !dbcs_lead(bytes[at + 1]) {
            2
        } else {
            1
        };
        ensure!(at + width <= bytes.len(), "SugarPot name DBCS boundary");
        let text = decode(&bytes[at..at + width])?;
        let glyphs = text
            .chars()
            .enumerate()
            .map(|(i, character)| CellGlyph {
                character,
                position: [i as i32 * (size / 2), 0],
            })
            .collect();
        groups.push(CellGroup {
            glyphs,
            advance: size + spacing,
            reset_before: None,
            automatic_wrap: false,
            break_before: false,
            break_after: false,
        });
        at += width;
    }
    Ok(groups)
}
pub(crate) fn groups(
    native: &str,
    limit: usize,
    size: i32,
    spacing: i32,
    columns: i32,
    rows: i32,
    profile: Profile,
    boxed_speaker: bool,
    initial_cell: [i32; 2],
) -> Result<Groups> {
    ensure!(
        (1..=512).contains(&size) && (1..=127).contains(&columns) && (1..=127).contains(&rows),
        "SugarPot native cell dimensions"
    );
    let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(native);
    ensure!(
        !invalid && encoded.len() < 0x20000 && limit <= encoded.len(),
        "SugarPot native cell source budget"
    );
    let bytes = encoded.as_ref();
    let history = profile != Profile::Live;
    let mut cells = Vec::new();
    let (mut at, mut column, mut row, mut ordinary) =
        (0usize, initial_cell[0], initial_cell[1], false);
    let mut last_break = false;
    let mut headers = vec![];
    ensure!(
        (0..=127).contains(&column) && (0..=127).contains(&row),
        "SugarPot initial native cell counters"
    );
    while at < limit {
        match bytes[at] {
            b'[' if matches!(profile, Profile::Live | Profile::Measure)
                || (at == 0 && matches!(profile, Profile::Redraw | Profile::Backlog)) =>
            {
                let start = at + 1;
                at = start;
                while at < limit && bytes[at] != b']' {
                    at += if dbcs_lead(bytes[at]) { 2 } else { 1 };
                }
                ensure!(at < limit, "SugarPot native grouped speaker boundary");
                let speaker = decode(&bytes[start..at])?;
                headers.push(cells.len());
                cells.push(CellGroup {
                    glyphs: vec![],
                    advance: 0,
                    reset_before: Some([0, 0]),
                    automatic_wrap: false,
                    break_before: false,
                    break_after: false,
                });
                if !boxed_speaker {
                    //43C560 surrounds an unframed name with the actual
                    //CP932 constants494768/49476C: 【 and 】. Its own cell
                    //loop wraps unconditionally, rather than body kinsoku.
                    let mut header_column = 0;
                    let mut header_row = 0;
                    for mut group in name_groups(&format!("【{speaker}】"), size, spacing)? {
                        header_column += 1;
                        if header_column >= columns {
                            group.break_after = true;
                            header_column = 0;
                            header_row += 1;
                        }
                        cells.push(group);
                        if header_row >= rows {
                            reset(&mut cells);
                            header_row = 0;
                        }
                    }
                    cells.push(CellGroup {
                        glyphs: vec![],
                        advance: 0,
                        reset_before: None,
                        automatic_wrap: false,
                        break_before: false,
                        break_after: true,
                    });
                }
                at += 1;
                column = 0;
                //439F00 resets this counter to0 after the name;44BEA0's
                //history measurement explicitly sets it to1 instead.
                row = if matches!(profile, Profile::Measure | Profile::Backlog) && !boxed_speaker {
                    1
                } else {
                    0
                };
                last_break = true;
            }
            b'\\' | b'@' => {
                let code = *bytes
                    .get(at + 1)
                    .context("SugarPot grouped escape boundary")?;
                match code {
                    b'f' | b'F' if matches!(profile, Profile::Live | Profile::Measure) => break,
                    b'f' | b'F' => at += 2,
                    b'n' | b'N' => {
                        at += 2;
                        line(&mut cells, &mut column, &mut row, rows);
                        ordinary = true;
                        last_break = true;
                    }
                    b'p' | b'P' | b'i' | b'I' => at += 2,
                    b'c' | b'C' => {
                        at += 2;
                        let start = at;
                        while at < bytes.len()
                            && at - start < 16
                            && !dbcs_lead(bytes[at])
                            && !matches!(bytes[at], b'\\' | b'@')
                            && (!history || bytes[at] != b'[')
                        {
                            at += 1;
                        }
                    }
                    b'0'..=b'9' => {
                        //40EBB0 sends @x to40E520's PE bitmap coverage path;
                        //it is one image glyph, not two ASCII scalars. Keep
                        //this visible until the shared bitmap-glyph seam is
                        //connected rather than publishing an incorrect '@0'.
                        ensure!(
                            bytes[at] != b'@',
                            "SugarPot FONT00/FONT01 bitmap glyph source not connected"
                        );
                        ensure!(at + 2 <= limit, "SugarPot grouped digit escape boundary");
                        let text = decode(&bytes[at..at + 2])?;
                        let glyphs = text
                            .chars()
                            .enumerate()
                            .map(|(i, character)| CellGlyph {
                                character,
                                position: [i as i32 * (size / 2), 0],
                            })
                            .collect();
                        at += 2;
                        column += 1;
                        ordinary = true;
                        let break_after = wrap(bytes, at, column, columns);
                        last_break = break_after;
                        cells.push(CellGroup {
                            glyphs,
                            advance: size + spacing,
                            reset_before: None,
                            automatic_wrap: false,
                            break_before: false,
                            break_after,
                        });
                        if break_after {
                            column = 0;
                            row = (row + 1) % rows;
                            if row == 0 {
                                reset(&mut cells);
                            }
                        }
                    }
                    _ => at += 1,
                }
            }
            byte => {
                let width = if dbcs_lead(byte) {
                    2
                } else if at + 1 < limit && !dbcs_lead(bytes[at + 1]) {
                    2
                } else {
                    1
                };
                ensure!(
                    at + width <= limit,
                    "SugarPot grouped DBCS segment boundary"
                );
                let text = decode(&bytes[at..at + width])?;
                let glyphs = text
                    .chars()
                    .enumerate()
                    .map(|(i, character)| CellGlyph {
                        character,
                        position: [i as i32 * (size / 2), 0],
                    })
                    .collect();
                at += width;
                column += 1;
                ordinary = true;
                ensure!(column <= 127, "SugarPot native cell column overflow");
                let break_after = wrap(bytes, at, column, columns);
                last_break = break_after;
                cells.push(CellGroup {
                    glyphs,
                    advance: size + spacing,
                    reset_before: None,
                    automatic_wrap: false,
                    break_before: false,
                    break_after,
                });
                if break_after {
                    column = 0;
                    row = (row + 1) % rows;
                    if row == 0 {
                        reset(&mut cells);
                    }
                }
            }
        }
        ensure!(cells.len() <= 65536, "SugarPot grouped text cell budget");
    }
    Ok(Groups {
        cells,
        column,
        row,
        ordinary,
        last_break,
        headers,
    })
}
