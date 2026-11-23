// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CPicture's semantic ClipEtc and two-cell clip-union sequence. This scalar
//! adapter policy deliberately differs from ordinary source-window cropping:
//! a negative source origin shrinks the extent without moving destination.
use crate::native_picture_clip::CharaClip;
use anyhow::{Result, ensure};
pub(crate) fn clip_etc(
    destination: &mut [i64; 2],
    source: &mut [i64; 2],
    size: &mut [i64; 2],
    picture: [u32; 2],
    screen: [i64; 4],
) -> bool {
    let ends = [destination[0] + size[0], destination[1] + size[1]];
    if ends[0] <= screen[0]
        || ends[1] <= screen[1]
        || destination[0] > screen[2]
        || destination[1] > screen[3]
    {
        return false;
    }
    for a in 0..2 {
        if destination[a] < screen[a] {
            let delta = screen[a] - destination[a];
            destination[a] += delta;
            source[a] += delta;
            size[a] -= delta;
        }
        if source[a] < 0 {
            size[a] += source[a];
            source[a] = 0;
        }
        if ends[a] > screen[a + 2] {
            size[a] -= ends[a] - screen[a + 2];
        }
        if source[a] + size[a] > i64::from(picture[a]) {
            size[a] = i64::from(picture[a]) - source[a];
        }
    }
    size.iter().all(|n| *n > 0)
        && source
            .iter()
            .zip(picture)
            .all(|(n, max)| *n < i64::from(max))
}
pub(crate) struct PairGeometry {
    pub destination: [i32; 2],
    pub sources: [[i32; 4]; 2],
}
pub(crate) fn pair(
    destination: [i32; 2],
    origins: [[i32; 2]; 2],
    extent: [i32; 2],
    sizes: [[u32; 2]; 2],
    clips: [CharaClip; 2],
    screen: [i32; 4],
) -> Result<Option<PairGeometry>> {
    ensure!(
        destination
            .iter()
            .chain(origins.iter().flatten())
            .chain(&extent)
            .chain(&screen)
            .all(|n| n.unsigned_abs() <= 1_000_000),
        "NNN paired native coordinates"
    );
    ensure!(
        sizes.iter().flatten().all(|n| (1..=16384).contains(n)),
        "NNN paired native source size"
    );
    if extent.iter().any(|n| *n <= 0) {
        return Ok(None);
    }
    let mut trimmed = [None; 2];
    for i in 0..2 {
        let s = [origins[i][0], origins[i][1], extent[0], extent[1]];
        let Some(mapping) = clips[i].source_clip(s, destination)? else {
            return Ok(None);
        };
        trimmed[i] = Some(mapping);
    }
    let [Some((a, da)), Some((b, db))] = trimmed else {
        unreachable!()
    };
    let at = [da[0].min(db[0]), da[1].min(db[1])];
    let ends = [
        (da[0] + a[2]).max(db[0] + b[2]),
        (da[1] + a[3]).max(db[1] + b[3]),
    ];
    let mut target = at.map(i64::from);
    let mut size = [i64::from(ends[0] - at[0]), i64::from(ends[1] - at[1])];
    let mut starts = origins.map(|s| {
        [
            i64::from(s[0]) + target[0] - i64::from(destination[0]),
            i64::from(s[1]) + target[1] - i64::from(destination[1]),
        ]
    });
    for current in 0..2 {
        let before = target;
        if !clip_etc(
            &mut target,
            &mut starts[current],
            &mut size,
            sizes[current],
            screen.map(i64::from),
        ) {
            return Ok(None);
        }
        for a in 0..2 {
            starts[1 - current][a] += target[a] - before[a];
        }
    }
    Ok(Some(PairGeometry {
        destination: target.map(|n| n as i32),
        sources: starts.map(|s| [s[0] as i32, s[1] as i32, size[0] as i32, size[1] as i32]),
    }))
}
