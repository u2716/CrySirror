//! Reusable wrapping grid navigation, independent of widget rendering.
use anyhow::{ensure, Result};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridDirection {
    Up,
    Down,
    Left,
    Right,
}
/// Row-major traversal skips the empty tail cells of the last row. -1 is an
/// explicit unselected state; signed row/column arithmetic remains meaningful.
pub fn grid_neighbor(
    current: i32,
    count: u32,
    columns: u32,
    direction: GridDirection,
) -> Result<i32> {
    ensure!(
        count > 0 && count <= 65536 && columns > 0 && columns <= 65536,
        "selection grid dimensions"
    );
    ensure!(
        current >= -1 && current < count as i32,
        "selection grid current item"
    );
    let columns = columns as i32;
    let count = count as i32;
    let mut col = current % columns;
    let mut row = current / columns;
    let rows = (count + columns - 1) / columns;
    let vertical = matches!(direction, GridDirection::Up | GridDirection::Down);
    for _ in 0..if vertical { rows } else { columns } {
        if vertical {
            row = (row
                + if direction == GridDirection::Up {
                    -1
                } else {
                    1
                })
            .rem_euclid(rows);
        } else {
            col = (col
                + if direction == GridDirection::Left {
                    -1
                } else {
                    1
                })
            .rem_euclid(columns);
        }
        let next = col + row * columns;
        if next < count {
            return Ok(next);
        }
    }
    Ok(current)
}
