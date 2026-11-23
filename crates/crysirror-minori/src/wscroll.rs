// SPDX-License-Identifier: MPL-2.0
use super::{Sprite, asset, characters::Characters, remove, sprite};
use crate::vm::{EffectRequest, Vm};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::{Value, json};

pub(super) const LAYER: u32 = 510;
pub(super) const LAST: u32 = LAYER + 6;
pub(super) const SECOND: u32 = 520;

struct Plane {
    art: Sprite,
    fixed: bool,
}

fn plane(h: &mut dyn HostServices, c: ClientId, stem: &str, suffix: &str) -> Result<Option<Plane>> {
    let storage = h.storage(c)?;
    for (modifier, fixed) in [("", false), ("_fixed", true)] {
        let name = format!("{stem}_{suffix}{modifier}.png");
        for bank in ["bg", "st"] {
            if storage.exists(&asset(bank, &name)?) {
                return Ok(Some(Plane {
                    art: sprite(h, c, &[bank], &name)?,
                    fixed,
                }));
            }
        }
    }
    Ok(None)
}

fn samples(bytes: &[u8]) -> Vec<i32> {
    bytes
        .split(|&b| b == b'\n')
        .filter_map(|line| {
            let sign = usize::from(line.first() == Some(&b'-'));
            let n = line[sign..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count();
            std::str::from_utf8(&line[..sign + n]).ok()?.parse().ok()
        })
        .collect()
}

pub(super) struct WalkingScroll {
    layer: u32,
    planes: [Option<Plane>; 2],
    overlay: Option<Sprite>,
    prefix: String,
    curve: Vec<i32>,
    speed: i32,
    period: i32,
    ratio: i32,
    start: u64,
    last: Option<u64>,
    char_height: Option<i32>,
    pub retiring: bool,
}

impl WalkingScroll {
    pub fn load(
        h: &mut dyn HostServices,
        c: ClientId,
        vm: &Vm,
        request: &EffectRequest,
        layer: u32,
    ) -> Result<Self> {
        let mut args = request.resource.split(',');
        let stem = args.next().context("minori WScrollST stem")?;
        let prefix = args.next().unwrap_or_default().to_owned();
        let path = args.next().unwrap_or_default();
        let storage = h.storage(c)?;
        let mut curve = Vec::new();
        if !path.is_empty() {
            let path = asset("st", path)?;
            if storage.exists(&path) {
                let reader = storage.open_asset(&path)?;
                curve = samples(&crysirror_resource::read_all(reader.as_ref(), 1 << 20)?);
            }
        }
        let planes = [plane(h, c, stem, "f")?, plane(h, c, stem, "n")?];
        let mut overlay = None;
        for suffix in ["sc", "ov", "ml", "ad"] {
            let name = format!("{stem}_{suffix}.png");
            if storage.exists(&asset("bg", &name)?) {
                overlay = Some(sprite(h, c, &["bg"], &name)?);
                break;
            }
        }
        let ratio = vm
            .local("wscroll2Ratio")
            .or_else(|| vm.global("wscroll2Ratio"))
            .unwrap_or(5);
        ensure!(ratio != 0, "minori WScrollST zero plane ratio");
        Ok(Self {
            layer,
            planes,
            overlay,
            prefix,
            curve,
            speed: request.second.wrapping_neg(),
            period: request.first,
            ratio,
            start: h.now_ms() / 10,
            last: None,
            char_height: None,
            retiring: false,
        })
    }

    pub fn draw(
        &mut self,
        now: u64,
        canvas: [i32; 2],
        chars: &mut Characters,
        ops: &mut Vec<Value>,
    ) {
        let tick = now / 10;
        if self.last.is_some_and(|prev| tick.saturating_sub(prev) < 10) {
            return;
        }
        let initial = self.last.is_none();
        self.last = Some(tick);
        let elapsed = tick.saturating_sub(self.start) as u32;
        let near = i64::from(self.speed) * i64::from(elapsed) / 10;
        // The native compositor allocates three adjacent copies of each plane.
        // Fixed resources retain their initial origin throughout the effect.
        if self.planes[0].is_some() {
            for (i, plane) in self.planes.iter().enumerate() {
                let Some(plane) = plane else { continue };
                if plane.fixed && !initial {
                    continue;
                }
                let width = plane.art.rect[2];
                let displacement = if plane.fixed {
                    0
                } else if i == 0 {
                    near / i64::from(self.ratio)
                } else {
                    near
                };
                let offset = (displacement % i64::from(width)) as i32;
                for copy in 0..3 {
                    let mut art = plane.art.clone();
                    art.rect[0] = offset.saturating_add((copy - 1) * width);
                    let mut op = art.layer(self.layer + i as u32 * 3 + copy as u32, 255);
                    op["layer"]["order"] = json!([39, self.layer - LAYER + i as u32 * 3 + copy as u32]);
                    ops.push(op);
                }
            }
        }
        if initial && let Some(overlay) = &self.overlay {
            let mut op = overlay.layer(self.layer + 6, 255);
            op["layer"]["order"] = json!([49, self.layer - LAYER]);
            op["layer"]["blend"] = if overlay.asset.ends_with("_sc.png") {
                json!("screen")
            } else if overlay.asset.ends_with("_ml.png") {
                json!("multiply")
            } else if overlay.asset.ends_with("_ad.png") {
                json!("add")
            } else {
                json!({"stored_byte":{"mode":"overlay"}})
            };
            ops.push(op);
        }
        if !self.prefix.is_empty() {
            let height = *self
                .char_height
                .get_or_insert_with(|| chars.scroll_height(&self.prefix).unwrap_or(canvas[1]));
            let excess = height.saturating_sub(canvas[1]).max(0);
            let offset = if excess == 0 {
                0
            } else if let Some(y) = chars.scroll_sample(&self.prefix, &self.curve, now) {
                y
            } else if self.period != 0 {
                let half = excess / 2;
                let phase =
                    f64::from(i64::from(elapsed).wrapping_rem(i64::from(self.period)) as i32)
                        / f64::from(self.period);
                half + (f64::from(half) * (phase * std::f64::consts::PI).sin()) as i32
            } else {
                0
            };
            chars.scroll_offset(&self.prefix, offset, ops);
        }
    }

    pub fn remove(self, chars: &mut Characters, ops: &mut Vec<Value>) {
        ops.extend((self.layer..=self.layer+6).map(remove));
        if !self.prefix.is_empty() {
            chars.scroll_offset(&self.prefix, 0, ops);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_curve_reads_leading_signed_decimal_until_first_non_digit() {
        assert_eq!(
            super::samples(b";comment\r\n-5\r\n7 extra\n 12\n0\0\n"),
            [-5, 7, 0]
        );
    }
}

// The walk advances the near plane in whole pixels, carrying the signed
// remainder into the far plane. Its bob phase advances once per accepted draw.
pub(super) struct StageWalk {
    period: i32,
    second_period: Option<i32>,
    step_phase: bool,
    speed: i32,
    tick: Option<u64>,
    elapsed: i32,
    position: i32,
    remainder: i32,
    distant: i32,
    phase: i32,
    curve: Vec<i32>,
    pub retiring: bool,
}

impl StageWalk {
    pub fn load(h: &mut dyn HostServices, c: ClientId, request: &EffectRequest, late: bool) -> Result<Self> {
        let mut curve = Vec::new();
        let dual = request.kind == "WScroll3";
        if !dual && let Some(name) = request.resource.strip_prefix("sync:") {
            let path = asset("st", name)?;
            let storage = h.storage(c)?;
            if storage.exists(&path) {
                let reader = storage.open_asset(&path)?;
                curve = samples(&crysirror_resource::read_all(reader.as_ref(), 1 << 20)?);
            }
        } else if !dual {
            ensure!(matches!(request.resource.as_str(), "" | "*"), "unsupported WScroll2 resource {}", request.resource);
        }
        Ok(Self {
            period: request.first, second_period: dual.then_some(if request.second > 0 { request.second } else { request.first }),
            step_phase: late && !dual, speed: if dual { request.third } else { request.second },
            tick: None, elapsed: 1,
            position: 0, remainder: 0, distant: 0, phase: 0, curve, retiring: false,
        })
    }

    pub fn advance(&mut self, now: u64, paused: bool, ratio: i32) -> Result<bool> {
        let tick = now / 10;
        if paused && self.second_period.is_none() { self.tick = Some(tick); return Ok(false); }
        let dt = self.tick.map_or(1, |last| tick.saturating_sub(last));
        if self.tick.is_some() && dt < 10 { return Ok(false); }
        self.tick = Some(tick);
        self.elapsed = self.elapsed.wrapping_add(dt as i32);
        let position = self.elapsed.wrapping_mul(self.speed) / 10;
        let delta = position.wrapping_sub(self.position);
        if delta.unsigned_abs() < 2 { return Ok(false); }
        let ratio = if self.second_period.is_some() { 5 } else { ratio };
        ensure!(ratio != 0, "minori WScroll2 zero plane ratio");
        self.position = position;
        self.remainder = self.remainder.wrapping_add(delta);
        if i64::from(self.remainder).abs() > i64::from(ratio) {
            self.distant = self.distant.wrapping_add(self.remainder.wrapping_div(ratio));
            self.remainder = self.remainder.wrapping_rem(ratio);
        }
        if self.step_phase && self.period != 0 {
            self.phase = self.phase.wrapping_sub(1).wrapping_rem(self.period);
        } else {
            self.phase = self.phase.wrapping_add(delta);
        }
        Ok(true)
    }

    pub fn draw(&self, world: &[Sprite], layer: u32, canvas: [i32; 2], ops: &mut Vec<Value>) {
        for (i, art) in world.iter().take(2).enumerate() {
            let width = art.rect[2];
            let position = if i == 0 { self.distant } else { self.position.wrapping_add(width / 2) };
            let mut left = -position.rem_euclid(width);
            for copy in 0..2 {
                let mut tile = art.clone();
                tile.rect[0] = left;
                tile.rect[1] = 0;
                let id = if copy == 0 { i as u32 + 1 } else { layer + i as u32 };
                let mut op = tile.layer(id, 255);
                op["layer"]["order"] = json!([i + 1, copy]);
                ops.push(op);
                left = left.saturating_add(width);
                if copy == 0 && left >= canvas[0] { ops.push(remove(layer + i as u32)); break; }
            }
        }
    }

    pub fn secondary_offset(&self, art: &Sprite, canvas: [i32; 2]) -> Option<[i32; 2]> {
        self.second_period.map(|period| self.offset(art, canvas, period, None))
    }

    pub fn foreground_offset(&self, art: &Sprite, canvas: [i32; 2], frame: Option<u32>) -> [i32; 2] {
        self.offset(art, canvas, self.period, frame)
    }

    fn offset(&self, art: &Sprite, canvas: [i32; 2], period: i32, frame: Option<u32>) -> [i32; 2] {
        let wave = if period == 0 { 0. } else {
            (f64::from(self.phase) / f64::from(period) * std::f64::consts::TAU).sin()
        };
        let amplitude = [0, 1].map(|i| (f64::from((art.rect[i+2] - canvas[i]).max(0)) * wave) as i32);
        let y = frame.filter(|_| !self.curve.is_empty())
            .map_or(amplitude[1], |f| self.curve[f as usize % self.curve.len()]);
        [amplitude[0].saturating_abs(), y.saturating_abs()]
    }
}
