// SPDX-License-Identifier: MPL-2.0
use super::{ClientId, HostServices, Result, Sprite, Value, asset, json, remove, sprite};
use crate::vm::EffectRequest;
use anyhow::ensure;

pub(super) struct SceneEffect {
    kind: Kind,
    tick: Option<u64>,
    elapsed: u64,
    pub retiring: bool,
}
enum Kind {
    Scroll {
        art: Option<Sprite>,
        initial: i32,
        period: i32,
        speed: i32,
    },
    Crossing {
        lamps: [Option<Sprite>; 2],
        alternation: Option<u64>,
        next: usize,
        fade: Option<(u64, u8)>,
    },
}
impl SceneEffect {
    pub fn load(
        h: &mut dyn HostServices,
        c: ClientId,
        r: &EffectRequest,
        bg: Option<&Sprite>,
    ) -> Result<Self> {
        let kind = match r.kind.as_str() {
            "WScroll" => {
                let (name, origin) = r.resource.split_once(':').unwrap_or((&r.resource, ""));
                let art = if name.is_empty() {
                    bg.cloned()
                } else if h.storage(c)?.exists(&asset("bg", name)?) {
                    Some(sprite(h, c, &["bg"], name)?)
                } else {
                    None
                };
                let mut initial = crate::vm::decimal_prefix(origin).unwrap_or(0);
                if let Some(a) = &art {
                    if !(0..a.rect[2]).contains(&initial) {
                        initial = 0;
                    }
                    ensure!(
                        r.first != 0
                            && r.second != 0
                            && 10_i64 * i64::from(a.rect[2]) / i64::from(r.second) != 0,
                        "invalid WScroll period or speed"
                    );
                }
                Kind::Scroll {
                    art,
                    initial,
                    period: r.first,
                    speed: r.second,
                }
            }
            "Crossing" => {
                let mut lamps = [None, None];
                for (i, lamp) in lamps.iter_mut().enumerate() {
                    let name = format!("crossing{}.png", i + 1);
                    if h.storage(c)?.exists(&asset("sys", &name)?) {
                        *lamp = Some(sprite(h, c, &["sys"], &name)?);
                    }
                }
                Kind::Crossing {
                    lamps,
                    alternation: None,
                    next: 0,
                    fade: None,
                }
            }
            _ => anyhow::bail!("unknown scene effect {}", r.kind),
        };
        Ok(Self {
            kind,
            tick: None,
            elapsed: 0,
            retiring: false,
        })
    }
    pub fn fade(&mut self) {
        if let Kind::Crossing { fade, .. } = &mut self.kind
            && fade.is_none()
        {
            *fade = Some((self.elapsed, (self.elapsed / 4).min(100) as u8));
        }
    }
    /// Returns the volume of a crossing bell when the two signal planes alternate.
    pub fn draw(
        &mut self,
        now: u64,
        slot: usize,
        canvas: [i32; 2],
        ops: &mut Vec<Value>,
    ) -> Option<(u32, u8)> {
        let tick = now / 10;
        let dt = self.tick.map_or(1, |prev| tick.saturating_sub(prev));
        if dt == 0 || (self.tick.is_some() && matches!(self.kind, Kind::Scroll { .. }) && dt < 10) {
            return None;
        }
        self.tick = Some(tick);
        self.elapsed = self.elapsed.saturating_add(dt);
        let base = Self::base(slot);
        match &mut self.kind {
            Kind::Scroll {
                art: Some(art),
                initial,
                period,
                speed,
            } => {
                let width = art.rect[2];
                let elapsed = self.elapsed as i64;
                let wrap = 10 * i64::from(width) / i64::from(*speed);
                let x = (i64::from(*initial) + i64::from(*speed) * (elapsed % wrap) / 10)
                    .rem_euclid(i64::from(width)) as i32;
                let amp = f64::from(((art.rect[3] - canvas[1]) / 2).abs());
                let y = (amp
                    * ((((elapsed % i64::from(*period)) as f64 / f64::from(*period))
                        * std::f64::consts::TAU)
                        .sin()
                        + 1.)) as i32;
                for copy in 0..2 {
                    let mut tile = art.clone();
                    tile.rect[0] = copy * width - x;
                    tile.rect[1] = -y.min(art.rect[3] - canvas[1]).max(0);
                    let mut op = tile.layer(base + copy as u32, 255);
                    op["layer"]["order"] = json!([39, slot as u32 * 10 + copy as u32]);
                    ops.push(op);
                }
            }
            Kind::Crossing {
                lamps,
                alternation,
                next,
                fade,
            } => {
                let phase = self.elapsed / 50;
                if *alternation != Some(phase) {
                    let index = *next;
                    *next ^= 1;
                    // The native callback toggles once per observed change, even after a long suspension.
                    *alternation = Some(phase);
                    let mut op = if let Some(art) = &lamps[index] {
                        let mut art = art.clone();
                        [art.rect[0], art.rect[1]] = [[620, 180], [584, 331]][index];
                        art.layer(base, 255)
                    } else {
                        remove(base)
                    };
                    if op["op"] == "put" {
                        op["layer"]["order"] = json!([49, 1 + slot * 2]);
                    }
                    ops.push(op);
                    let volume =
                        fade.map_or((self.elapsed / 4).min(100) as u8, |(start, alpha)| {
                            alpha.saturating_sub(((self.elapsed - start) / 4).min(255) as u8)
                        });
                    return Some((index as u32, volume));
                }
            }
            _ => {}
        }
        None
    }
    pub fn base(slot: usize) -> u32 {
        550 + 2 * slot as u32
    }
}
