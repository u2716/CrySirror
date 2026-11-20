// SPDX-License-Identifier: MPL-2.0
//! The face of a maid (`Maid.Update` face part and `TMorph`): blend-set
//! expressions with a timed transition, the `FaceBlend` overlay, blinking,
//! lip sync and the mouth flags, resolved to morph weights of the head.
//!
//! - `FaceAnime(tag, t = 1)`: from the current expression to `tag`
//!   (`tag〓通常` when that set exists) with weight
//!   `COSS2(FaceTime^0.4, 4)`, `FaceTime` starting at `1/(60 t)` and growing
//!   by `dt / t`.
//! - Per frame: `ClearBlendValues`, `MulBlendValues(FaceName, 1)`,
//!   `MulBlendValues(FaceName2, ease)`, `AddBlendValues(FaceName3, 1)`
//!   (clamped to 1), the blink (`EyeMabataki = sin(v²π)`, `v` falling 2/s,
//!   the next one 0.5 + rnd(80)·0.1 s later while no transition runs), then
//!   `FixBlendValues_Face`.
use std::collections::BTreeMap;

/// `UTY.COSS(v)`: `(1 − cos(vπ)) / 2`.
pub fn coss(v: f32) -> f32 {
    (1. - (v * std::f32::consts::PI).cos()) * 0.5
}

/// `UTY.COSS2(v, n)`: `COSS` applied `n` times.
pub fn coss2(v: f32, n: f32) -> f32 {
    let mut x = v;
    for _ in 0..n as i32 {
        x = coss(x);
    }
    x
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FaceState {
    /// `FaceName`, `FaceName2` (transition target), `FaceName3` (blend).
    pub name: String,
    pub name2: String,
    pub name3: String,
    pub time: f32,
    pub plus: f32,
    pub anime: bool,
    /// `MabatakiVal`, next blink time (s).
    pub blink: f32,
    pub next_blink: f32,
    /// `boMouthOpen` (drives `boBallGAG`).
    pub mouth_open: bool,
    /// `boLipsync` and its phase accumulators.
    pub lipsync: bool,
    lipsync_old: bool,
    lipsync_t: (f32, f32),
    /// `LipSync1..3` and `boLipSync`.
    pub lip: [f32; 3],
    pub lip_on: bool,
    /// `m_baKuchipakuPattern` with its offset time (s).
    pub kuchipaku: Option<(f32, Vec<u8>)>,
    rng: u32,
}

impl FaceState {
    pub fn new() -> Self {
        Self { name: String::new(), rng: 0x1234_5678, ..Default::default() }
    }

    /// `Maid.FaceAnime(tag, 1)`.
    pub fn anime(&mut self, tag: &str, sets: &BTreeMap<String, BTreeMap<String, f32>>) {
        let mut tag = tag.to_owned();
        let normal = format!("{tag}〓通常");
        if sets.contains_key(&normal) {
            tag = normal;
        }
        self.anime = true;
        if !self.name2.is_empty() {
            self.name = std::mem::take(&mut self.name2);
        }
        if self.name.is_empty() {
            self.name = "デフォ".into();
        }
        self.name2 = tag;
        self.plus = 1.;
        self.time = self.plus / 60.;
    }

    /// `Maid.FaceBlend`.
    pub fn blend(&mut self, tag: &str) {
        self.name3 = if tag == "無し" { String::new() } else { tag.to_owned() };
    }

    /// `UTY.GetRnd(n)`.
    fn rnd(&mut self, n: u32) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng % n.max(1)
    }

    /// One `Maid.Update` face step; `t` is `Time.time`, `audio` the voice
    /// playback position (s) when a voice plays. Returns the morph weights.
    pub fn update(&mut self, dt: f32, t: f32, audio: Option<f32>, sets: &BTreeMap<String, BTreeMap<String, f32>>) -> BTreeMap<String, f32> {
        // Voice-driven kuchipaku pattern (FoceKuchipakuUpdate at 30 Hz).
        if let (Some((offset, pattern)), Some(at)) = (&self.kuchipaku, audio) {
            let i = ((offset + at) * 30.) as i64;
            let n = (pattern.len() / 3) as i64;
            if 0 < i && i < n {
                let i = i as usize;
                self.lip_on = true;
                for k in 0..3 {
                    self.lip[k] = self.lip[k] * 0.15 + pattern[i * 3 + k] as f32 / 255. * 0.85 * 3.;
                }
            } else {
                self.lip_on = false;
            }
        }
        if self.lipsync {
            self.lip_on = true;
            self.lipsync_t.0 += (t * 10.).sin() * 0.5 + (t * 5.55).cos() * 0.1 * 0.2;
            self.lipsync_t.1 += (t * 5.).sin() * 0.1 + (t * 5.55).cos() * 0.1 * 0.2;
            let a = (1. + (t * 4. + self.lipsync_t.0).sin()) * 0.5 * 0.98 + 0.1;
            let b = (self.lipsync_t.0.sin() + 1.) * 0.5;
            let c = (self.lipsync_t.1.sin() + 1.) * 0.5;
            self.lip = [a * b * c * 0.5, a * (1. - c) * 0.5, a * (1. - b) * c * 0.5];
        } else if self.lipsync_old {
            self.lip = [0.; 3];
        }
        self.lipsync_old = self.lipsync;
        // boMabataki (default true).
        let mut values: BTreeMap<String, f32> = BTreeMap::new();
        let mul = |values: &mut BTreeMap<String, f32>, set: &str, w: f32| {
            let Some(s) = sets.get(set) else { return };
            for v in values.values_mut() {
                *v *= 1. - w;
            }
            for (k, x) in s {
                let e = values.entry(k.clone()).or_insert(0.);
                *e += x * w;
            }
        };
        if !self.name.is_empty() {
            mul(&mut values, &self.name.clone(), 1.);
        }
        if !self.name2.is_empty() {
            mul(&mut values, &self.name2.clone(), coss2(self.time.powf(0.4), 4.));
            self.time += dt * self.plus;
            if self.time >= 1. {
                self.anime = false;
                self.name = std::mem::take(&mut self.name2);
            }
        }
        if !self.name3.is_empty() {
            if let Some(s) = sets.get(&self.name3) {
                for (k, x) in s {
                    let e = values.entry(k.clone()).or_insert(0.);
                    *e = (*e + x).min(1.);
                }
            }
        }
        let blink = if self.blink > 0. { (self.blink * self.blink * std::f32::consts::PI).sin() } else { 0. };
        self.blink -= dt * 2.;
        fix_face(&mut values, blink, self.lip_on.then_some(self.lip), self.mouth_open);
        if !self.anime && self.next_blink < t {
            let wait = 0.5 + self.rnd(80) as f32 * 0.1;
            self.next_blink = t + wait;
            self.blink = 1.;
        }
        values
    }
}

/// `TMorph.FixBlendValues_Face` over named weights.
fn fix_face(v: &mut BTreeMap<String, f32>, blink: f32, lip: Option<[f32; 3]>, gag: bool) {
    let get = |v: &BTreeMap<String, f32>, k: &str| v.get(k).copied().unwrap_or(0.);
    if blink > 0. {
        let mut sum: f32 = (0..10).map(|i| get(v, &if i == 0 { "eyeclose".to_owned() } else { format!("eyeclose{i}") })).sum();
        if sum > 1. {
            sum = 1.;
        }
        let x = get(v, "eyeclose") + (1. - sum) * blink;
        if x > 0. {
            v.insert("eyeclose".into(), x);
        }
    }
    if let Some([l1, l2, l3]) = lip {
        v.insert("moutha".into(), l1);
        v.insert("mouthc".into(), l3);
        v.insert("toothoff".into(), l3);
        v.insert("mouths".into(), l2 * 0.8);
        v.insert("mouthdw".into(), l2);
        // BlendDataIdx_LipSyncTh is also `toothoff`.
        v.insert("toothoff".into(), 1.);
    }
    if gag {
        v.insert("moutha".into(), 1.);
        v.insert("mouths".into(), 0.);
        v.insert("mouthc".into(), 0.);
        v.insert("toothoff".into(), 0.);
    }
    let mut t3 = get(v, "tear3");
    let t2 = get(v, "tear2");
    if t2 + t3 > 1. {
        v.insert("tear2".into(), 1. - t3);
        t3 = 1.;
    } else {
        t3 += t2;
    }
    if get(v, "tear1") + t3 > 1. {
        v.insert("tear1".into(), 1. - t3);
    }
    v.insert("nosefook".into(), 0.);
    for k in ["hoho", "hoho2"] {
        if let Some(x) = v.get_mut(k) {
            *x = if *x < 0.5 { 0. } else { 1. };
        }
    }
    // Weights under 0.01 do not move vertices.
    for x in v.values_mut() {
        if *x < 0.01 {
            *x = 0.;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_reaches_the_target_after_one_second() {
        let mut sets = BTreeMap::new();
        sets.insert("通常".to_owned(), BTreeMap::from([("mouthup".to_owned(), 0.5)]));
        sets.insert("笑顔".to_owned(), BTreeMap::from([("mouthup".to_owned(), 1.0)]));
        let mut f = FaceState::new();
        f.anime("通常", &sets);
        for _ in 0..70 {
            f.update(1. / 60., 0., None, &sets);
        }
        assert_eq!(f.name, "通常");
        f.anime("笑顔", &sets);
        let first = f.update(1. / 60., 0., None, &sets)["mouthup"];
        assert!(first < 0.51, "{first}");
        for _ in 0..70 {
            f.update(1. / 60., 0., None, &sets);
        }
        assert_eq!(f.update(1. / 60., 0., None, &sets)["mouthup"], 1.0);
    }

    #[test]
    fn mouth_open_forces_the_gag_shape() {
        let sets = BTreeMap::new();
        let mut f = FaceState::new();
        f.mouth_open = true;
        let w = f.update(0.016, 0., None, &sets);
        assert_eq!((w["moutha"], w["mouths"], w["toothoff"]), (1., 0., 0.));
    }
}
