// SPDX-License-Identifier: MPL-2.0
use super::super::checkpoint::{put_short, put_word, short, word};
use super::*;

impl Sprite {
    // 0x41a830 rotates the enclosing square around the pivot before applying
    // the average scale. The unrotated path uses each axis' own scale.
    pub(in crate::adapter::cvns) fn damage_bounds(&self) -> [i32; 4] {
        let [x, y] = self.position;
        let [w, h] = self.size;
        let [ox, oy] = self.origin;
        let [sx, sy] = self.scale;
        if self.rotation == 0 {
            let l = x.wrapping_sub(sx.wrapping_mul(ox) / 1000);
            let t = y.wrapping_sub(sy.wrapping_mul(oy) / 1000);
            return [
                l,
                t,
                l.wrapping_add(sx.wrapping_mul(w) / 1000),
                t.wrapping_add(sy.wrapping_mul(h) / 1000),
            ];
        }
        let side = w.max(h);
        let dx = ox.wrapping_neg().wrapping_sub(side.wrapping_sub(w) / 2);
        let dy = oy.wrapping_neg().wrapping_sub(side.wrapping_sub(h) / 2);
        let theta = f64::from(self.rotation) * (1.0 / 180.0) * f64::from(std::f32::consts::PI);
        let (sin, cos) = theta.sin_cos();
        let (s, c) = ((sin * 1024.) as i32, (cos * 1024.) as i32);
        let mut b = [9999i32, 9999, 0, 0];
        for (u, v) in [(0, 0), (side, 0), (0, side), (side, side)] {
            let u = u.wrapping_add(dx);
            let v = v.wrapping_add(dy);
            let px = x.wrapping_add(
                c.wrapping_mul(u)
                    .wrapping_sub(s.wrapping_mul(v))
                    .wrapping_add(512)
                    >> 10,
            );
            let py = y.wrapping_add(
                s.wrapping_mul(u)
                    .wrapping_add(c.wrapping_mul(v))
                    .wrapping_add(512)
                    >> 10,
            );
            b = [b[0].min(px), b[1].min(py), b[2].max(px), b[3].max(py)];
        }
        let scale = sx.wrapping_add(sy) / 2;
        let l = x.wrapping_sub(x.wrapping_sub(b[0]).wrapping_mul(scale) / 1000);
        let t = y.wrapping_sub(y.wrapping_sub(b[1]).wrapping_mul(scale) / 1000);
        [
            l,
            t,
            l.wrapping_add(b[2].wrapping_sub(b[0]).wrapping_mul(scale) / 1000)
                .wrapping_add(1),
            t.wrapping_add(b[3].wrapping_sub(b[1]).wrapping_mul(scale) / 1000)
                .wrapping_add(1),
        ]
    }

    pub(in crate::adapter::cvns) fn record(&self, id: SpriteId, out: &mut Vec<u8>) -> Result<()> {
        let mut data = vec![0; 1338];
        let kind = match self.kind {
            Kind::Bitmap => 1,
            Kind::Fill => 6,
            Kind::Movie => 9,
            Kind::Part(_) => 30,
        };
        put_word(&mut data, 4, 80);
        put_word(&mut data, 8, kind);
        put_word(&mut data, 16, self.fill);
        put_word(&mut data, 24, self.tint.0);
        for (at, v) in [
            (20, self.hidden),
            (22, self.tint.1),
            (28, self.alpha as i16),
            (30, self.rotation as i16),
            (32, self.origin[0] as i16),
            (34, self.origin[1] as i16),
            (36, self.scale[0] as i16),
            (38, self.scale[1] as i16),
            (42, self.source),
            (52, self.position[0].wrapping_sub(self.origin[0]) as i16),
            (54, self.position[1].wrapping_sub(self.origin[1]) as i16),
            (56, if self.rule_blit { -200 } else { -1 }),
        ] {
            put_short(&mut data, at, v);
        }
        for (i, v) in self.crop.iter().enumerate() {
            put_short(&mut data, 44 + 2 * i, *v);
        }
        let old = self.last_bounds.unwrap_or([-1; 4]);
        let bounds = self.damage_bounds();
        let damage = if old == [-1; 4] {
            bounds
        } else {
            [
                old[0].min(bounds[0]),
                old[1].min(bounds[1]),
                old[2].max(bounds[2]),
                old[3].max(bounds[3]),
            ]
        };
        for (at, rect) in [(60, old), (128, damage), (144, bounds)] {
            for (i, v) in rect.into_iter().enumerate() {
                put_word(&mut data, at + 4 * i, v as u32);
            }
        }
        put_word(&mut data, 76, u32::from(self.last_bounds != Some(bounds)));
        for (i, v) in [
            self.enabled as u32,
            self.dead as u32,
            self.position[0] as u32,
            self.position[1] as u32,
            self.size[0] as u32,
            self.size[1] as u32,
            self.origin[0] as u32,
            self.origin[1] as u32,
            self.scale[0] as u32,
            self.scale[1] as u32,
            self.rotation as u32,
            self.layer,
        ]
        .into_iter()
        .enumerate()
        {
            put_word(&mut data, 80 + i * 4, v);
        }
        put_word(&mut data, 160, self.running as u32);
        put_short(&mut data, 164, self.slot);
        put_short(&mut data, 166, self.current);
        put_word(&mut data, 168, self.remaining as u32);
        for (i, start) in self.starts.iter().enumerate() {
            put_short(&mut data, 172 + 2 * i, *start);
        }
        for (frame, raw) in self.frames.iter().zip(data[184..1336].chunks_exact_mut(18)) {
            let [x, y, w, h] = frame.crop;
            let [ox, oy] = frame.origin;
            for (i, v) in [frame.bitmap, x, y, w, h, ox, oy, frame.duration, frame.next]
                .into_iter()
                .enumerate()
            {
                put_short(raw, 2 * i, v);
            }
        }
        if let Some(effect) = &self.effect {
            put_short(&mut data, 1336, 1);
            for v in effect.record() {
                data.extend_from_slice(&v.to_le_bytes());
            }
        }
        match id {
            SpriteId::Global(index) => {
                crate::vm::save_slot::csp1::append(out, 0x4c0, &[index], &data)
            }
            SpriteId::Part(w, p) => {
                crate::vm::save_slot::csp1::append(out, 0x4c1, &[w.into(), p.into()], &data)
            }
        }
    }

    pub(in crate::adapter::cvns) fn from_record(
        id: SpriteId,
        raw: &[u8],
        random: &mut impl FnMut() -> i32,
    ) -> Result<Self> {
        ensure!(raw.len() >= 1338, "truncated CSP1 sprite");
        let has_effect = short(raw, 1336) != 0;
        ensure!(
            raw.len() == 1338 + 40 * usize::from(has_effect),
            "invalid CSP1 sprite effect length"
        );
        let kind = match word(raw, 8) {
            1 => Kind::Bitmap,
            6 => Kind::Fill,
            9 => Kind::Movie,
            30 => match id {
                SpriteId::Part(window, _) => Kind::Part(window),
                _ => bail!("CSP1 ADV sprite outside its owner"),
            },
            n => bail!("unknown CSP1 sprite kind {n}"),
        };
        let mut sprite = Self::new(kind);
        sprite.fill = word(raw, 16);
        sprite.hidden = short(raw, 20);
        sprite.tint = (word(raw, 24), short(raw, 22));
        sprite.alpha = short(raw, 28) as u16;
        sprite.source = short(raw, 42);
        sprite.crop = std::array::from_fn(|i| short(raw, 44 + 2 * i));
        sprite.rule_blit = short(raw, 56) == -200;
        let old = std::array::from_fn(|i| word(raw, 60 + 4 * i) as i32);
        sprite.last_bounds = (old != [-1; 4]).then_some(old);
        sprite.enabled = word(raw, 80) != 0;
        sprite.dead = word(raw, 84) != 0;
        sprite.position = [word(raw, 88) as i32, word(raw, 92) as i32];
        sprite.size = [word(raw, 96) as i32, word(raw, 100) as i32];
        sprite.origin = [word(raw, 104) as i32, word(raw, 108) as i32];
        sprite.scale = [word(raw, 112) as i32, word(raw, 116) as i32];
        sprite.rotation = word(raw, 120) as i32;
        sprite.layer = word(raw, 124);
        ensure!(
            sprite.layer < 128,
            "CSP1 sprite layer outside native buckets"
        );
        sprite.running = word(raw, 160) != 0;
        sprite.slot = short(raw, 164);
        sprite.current = short(raw, 166);
        sprite.remaining = word(raw, 168) as i32;
        ensure!(
            !sprite.running || ((0..64).contains(&sprite.current) && (0..6).contains(&sprite.slot)),
            "invalid running CSP1 animation"
        );
        sprite.starts = std::array::from_fn(|i| short(raw, 172 + 2 * i));
        for (frame, bytes) in sprite
            .frames
            .iter_mut()
            .zip(raw[184..1336].chunks_exact(18))
        {
            frame.bitmap = short(bytes, 0);
            frame.crop = std::array::from_fn(|i| short(bytes, 2 + 2 * i));
            frame.origin = [short(bytes, 10), short(bytes, 12)];
            frame.duration = short(bytes, 14);
            frame.next = short(bytes, 16);
        }
        if has_effect {
            let fx = &raw[1338..];
            sprite.effect = Some(super::super::fx::Effect::new(
                word(fx, 0),
                word(fx, 16),
                word(fx, 24) as i32,
                word(fx, 28) as i32,
                word(fx, 32) as i32,
                sprite.base(),
                random,
            )?);
        }
        Ok(sprite)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tinted_adv_part_keeps_its_enclosing_owner_on_restore() -> Result<()> {
        let id = SpriteId::Part(1, 3);
        let mut part = Sprite::new(Kind::Part(1));
        part.tint = (0xff0000, 0);
        let mut records = Vec::new();
        part.record(id, &mut records)?;
        crate::vm::save_slot::csp1::visit(&records, |tag, keys, bytes| {
            assert_eq!((tag, keys), (0x4c1, &[1u16, 3][..]));
            let restored = Sprite::from_record(id, bytes, &mut || 0)?;
            assert_eq!(restored.kind, Kind::Part(1));
            assert_eq!(restored.tint, part.tint);
            Ok(())
        })
    }
}
