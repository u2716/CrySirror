//! DC2 424880's retained40ms feedback blend. This is native scheduling only:
//! Canvas captures, immutable bitmap composition and publication are shared
//! engine services, including the exact stored-byte /16 arithmetic.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner {
    pub scene: String,
    pub pc: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    ClearPrepared,
    ///41ADE0 writes display records into native0 itself, preserving offscreen
    ///rows (title_d is800x1200). It does not publish the screen.
    Bake {
        owner: Owner,
        slot: u8,
        draw: Vec<Draw>,
    },
    Prepare {
        owner: Owner,
        slot: u8,
        draw: Vec<Draw>,
    },
    Present {
        owner: Owner,
        slot: u8,
        draw: Vec<Draw>,
    },
    Capture(Owner),
    Darken {
        owner: Owner,
        source_weight: u8,
    },
    Pan {
        owner: Owner,
        slot: u8,
        position: [i32; 2],
        source_weight: u8,
        finalize: bool,
    },
    Mix {
        owner: Owner,
        slot: u8,
        source_weight: u8,
    },
    Commit {
        owner: Owner,
        slot: u8,
    },
    ///424CA0/40EAB0: native0 over the current screen with per-pixel /16
    ///weights `clamp(field(mode, x, y) + offset, 0, 16)`.
    Wipe {
        owner: Owner,
        slot: u8,
        mode: u8,
        offset: i32,
    },
    ///41F630 case3 "flash": show a40E300 palette fill (`Some`) or the
    ///frozen copy (`None`, 40B1E0). `finalize` drops the frozen screen;
    ///the live scene equals the restored copy.
    Flash {
        owner: Owner,
        fill: Option<[u8; 3]>,
        finalize: bool,
    },
    ///41F630 case4/6 "yure"/"tateyure": over the previous screen, 40E300
    ///fills `fill` ([x1,y1,x2,y2), exclusive) then 40B1E0 copies the frozen
    ///screen at `offset`.
    Shake {
        owner: Owner,
        offset: [i32; 2],
        fill: Option<([i32; 4], [u8; 3])>,
    },
    ///41F630 case1: 40CCD0 floor(new*weight/16) of the freshly composed
    ///scene (403E80 + 4258C0(15)) over black, weight 0..=16.
    FadeIn {
        owner: Owner,
        slot: u8,
        weight: u8,
    },
    ///40D0A0: `(new*t + old*(256-t)) >> 8` from the frozen old screen
    ///(41F630 case20 "fadechg2") to the composed native0, t in 0..=256.
    Crossfade {
        owner: Owner,
        slot: u8,
        weight: u16,
    },
    ///418630/418570 screen-break frame. `cracks` copy only FF00FF texels of the
    ///overlay strip into the persistent captured screen (40B9C0), then
    ///`tiles` draw it over a palette fill (40E300) with keyed /16 weights
    ///(40C360). `finalize` binds the last frame like a committed feedback.
    Shatter {
        owner: Owner,
        overlay_slot: u8,
        fill: [u8; 3],
        /// (tile pixel origin, overlay frame x).
        cracks: Vec<([i32; 2], i32)>,
        /// (source origin, destination, weight 1..=16).
        tiles: Vec<([i32; 2], [i32; 2], u8)>,
        finalize: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Draw {
    Image {
        slot: u8,
        source: [i32; 4],
        destination: [i32; 2],
    },
    Solid {
        rect: [i32; 4],
        color: [u8; 4],
    },
    ///40C360 (RGB24, FF00FF key) or 40C0E0 (ARGB32, alpha scaled by the
    ///weight): `(src*w + dst*(16-w)) / 16`.
    Blend {
        slot: u8,
        source: [i32; 4],
        destination: [i32; 2],
        weight: u8,
    },
    ///40E3C0 palette fill with the same /16 weight.
    SolidBlend {
        rect: [i32; 4],
        color: [u8; 3],
        weight: u8,
    },
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Transition {
    pub progress: u8,
    pub deadline: Option<u64>,
    pub mixing: bool,
    pub source_weight: u8,
}
