// SPDX-License-Identifier: MPL-2.0
//! Native EFEX queue policy; integer interpolation is a shared scene capability.
use super::*;
#[derive(Clone)]
pub(super) struct Move {
    id: usize,
    kind: i32,
    from: [i32; 2],
    to: [i32; 2],
}
impl SugarPotVm {
    pub(super) fn efex(&mut self, h: &mut dyn HostServices, args: &[String]) -> Result<bool> {
        let selector = self.number(args, 1, 0)?;
        match selector {
            11 | 12 | 13 => {
                let id = usize::try_from(self.number(args, 0, 0)?)?;
                if selector == 12 {
                    let load = vec![
                        args[0].clone(),
                        args.get(4).context("SugarPot efex image")?.clone(),
                        args.get(5).cloned().unwrap_or("0".into()),
                        args.get(6).cloned().unwrap_or("0".into()),
                        "0".into(),
                        "0".into(),
                    ];
                    if !self.visual(h, "chon", &load)? {
                        return Ok(false);
                    }
                }
                let sprite = self.sprites.get(id).context("SugarPot efex sprite")?;
                let width = sprite.image.as_ref().map_or(0, |i| i.size[0] as i32);
                let x = self.number(args, 2, 0)?
                    - match sprite.origin {
                        1 => width / 2,
                        2 => width,
                        _ => 0,
                    };
                let y = self.number(args, 3, 0)?;
                // 43BBF0 reserves slots1..7; an eighth request has no free slot.
                if self.moves.len() < 7 {
                    self.moves.push(Move {
                        id,
                        kind: selector,
                        from: sprite.position,
                        to: [x, y],
                    });
                }
            }
            1 | 2 | 3 => {
                let duration = self.number(args, 2, 0)?;
                //Portable repair: a nonpositive native duration would divide
                //by zero or wrap the timer. Commit the final queued positions.
                let duration = duration.max(0) as u64;
                for window in &mut self.windows {
                    window.visibility = 2;
                }
                self.publish(h, None)?;
                self.wait = Some(Wait::Motion {
                    start: h.now_ms(),
                    duration,
                    kind: selector,
                });
                if duration == 0 {
                    self.motion_frame(h, 1, 1, selector)?;
                    self.wait = None;
                } else {
                    self.motion_frame(h, 0, duration, selector)?;
                }
            }
            _ => bail!(
                "SugarPot efex selector {selector} has no reviewed portable execution profile"
            ),
        }
        Ok(true)
    }
    pub(super) fn motion_frame(
        &mut self,
        h: &mut dyn HostServices,
        elapsed: u64,
        duration: u64,
        kind: i32,
    ) -> Result<()> {
        use crysirror_scene::animation::sample_integer_line;
        let mut layers = vec![];
        let mut used = BTreeSet::new();
        for movement in &self.moves {
            if !used.insert(movement.id) {
                continue;
            }
            let sprite = self
                .sprites
                .get(movement.id)
                .context("SugarPot queued movement sprite")?;
            if !sprite.visible || sprite.system != 0 {
                continue;
            }
            let p: [i32; 2] = std::array::from_fn(|axis| {
                sample_integer_line(
                    elapsed.min(duration) as i64,
                    0,
                    movement.from[axis] as i64,
                    duration as i64,
                    movement.to[axis] as i64,
                ) as i32
            });
            let Some(image) = &sprite.image else {
                continue;
            };
            let alpha = match kind {
                2 => sample_integer_line(elapsed.min(duration) as i64, 0, 0, duration as i64, 255),
                3 => sample_integer_line(elapsed.min(duration) as i64, 0, 255, duration as i64, 0),
                _ => 255,
            };
            let alpha = Self::sprite_opacity(sprite, movement.id == 0, alpha as i32);
            layers.push(json!({"op":"set_rect","id":movement.id as u32+1,"rect":[p[0],p[1],image.size[0],image.size[1]/sprite.divisions]}));
            layers.push(json!({"op":"set_opacity","id":movement.id as u32+1,"opacity":alpha}));
        }
        if !layers.is_empty() {
            self.submit(h, layers, vec![])?;
        }
        if elapsed >= duration {
            let mut used = BTreeSet::new();
            for movement in self.moves.drain(..) {
                if !used.insert(movement.id) {
                    continue;
                }
                let sprite = self
                    .sprites
                    .get_mut(movement.id)
                    .context("SugarPot queued movement sprite")?;
                if sprite.visible && sprite.system == 0 {
                    sprite.position = movement.to.map(|v| v as i16 as i32);
                    if movement.kind == 13 {
                        sprite.visible = false;
                    }
                }
            }
            // Native completion reconstructs the scene with opaque surviving
            // sprites; fade-out removes only explicitly queued selector13s.
            self.publish(h, None)?;
        }
        Ok(())
    }
}

//448530 ->4383B0 cases39..45. These records retain the original sprite
//coordinates until the final native commit; intermediate Layer rectangles are
//presentation only, so a slot never mistakes an interpolated coordinate for XY.
#[derive(Clone, Debug)]
pub(super) struct EfMovement {
    id: usize,
    selector: i32,
    from: [i32; 2],
    to: [i32; 2],
    start: u64,
    duration: u64,
    distance: i32,
}
impl SugarPotVm {
    fn ef_movement(
        &mut self,
        h: &mut dyn HostServices,
        args: &[String],
        selector: i32,
    ) -> Result<bool> {
        let id = usize::try_from(self.number(args, 0, 0)?)?;
        //Selector6 loads the image at its separate initial XY (arguments6/7),
        //with cell/division zero; arguments2/3 remain its final destination.
        if matches!(selector, 6 | 8) {
            let load = vec![
                args[0].clone(),
                args.get(4).context("SugarPot EF6 image")?.clone(),
                args.get(if selector == 6 { 6 } else { 2 })
                    .cloned()
                    .unwrap_or("0".into()),
                args.get(if selector == 6 { 7 } else { 3 })
                    .cloned()
                    .unwrap_or("0".into()),
                "0".into(),
                "0".into(),
            ];
            if !self.visual(h, "chon", &load)? {
                return Ok(false);
            }
        }
        let sprite = self.sprites.get(id).context("SugarPot EF sprite ID")?;
        let mut position = [self.number(args, 2, 0)?, self.number(args, 3, 0)?];
        let width = sprite.image.as_ref().map_or(0, |i| i.size[0] as i32);
        position[0] = position[0].wrapping_sub(match sprite.origin {
            1 => width / 2,
            2 => width,
            _ => 0,
        });
        let distance = self.number(args, if matches!(selector, 8 | 9) { 6 } else { 4 }, 0)?;
        let duration = self.number(args, 5, 1000)?;
        let duration = if duration == 0 { 1000 } else { duration };
        //The native signed duration can feed an unsigned division. Negative
        //durations are repaired portably as an immediate final commit, rather
        //than waiting for a wrapped multi-week timer or dividing by zero.
        let duration = duration.max(0) as u64;
        let (from, to) = if matches!(selector, 8 | 9) {
            (sprite.position, sprite.position)
        } else if selector <= 4 {
            let mut to = position;
            let axis = usize::from(selector >= 3);
            to[axis] = if matches!(selector, 1 | 3) {
                to[axis].wrapping_add(distance)
            } else {
                to[axis].wrapping_sub(distance)
            };
            (position, to)
        } else {
            (sprite.position, position)
        };
        for window in &mut self.windows {
            window.visibility = 2;
        }
        self.publish(h, None)?;
        let motion = EfMovement {
            id,
            selector,
            from,
            to,
            start: h.now_ms(),
            duration,
            distance,
        };
        let finished = self.ef_movement_frame(h, &motion, false)?;
        if !finished {
            self.wait = Some(Wait::EfMovement(motion));
        }
        Ok(true)
    }
    pub(super) fn ef_movement_frame(
        &mut self,
        h: &mut dyn HostServices,
        motion: &EfMovement,
        skip: bool,
    ) -> Result<bool> {
        use crysirror_scene::animation::sample_integer_line;
        if h.now_ms() < self.native_effect_next {
            return Ok(false);
        }
        self.native_effect_next = h.now_ms().saturating_add(33);
        let elapsed = h.now_ms().saturating_sub(motion.start);
        let finished = skip
            || self.settings.header[4] != 0
            || elapsed >= motion.duration
            || (motion.selector <= 4 && motion.distance <= 0);
        self.native_effect_index = if finished {
            0
        } else if motion.selector <= 4 {
            motion.distance.wrapping_mul(elapsed as i32) / motion.duration as i32
        } else {
            let phase = (255u32.wrapping_mul(elapsed as u32) as i32) / motion.duration as i32;
            if matches!(motion.selector, 7 | 9) {
                255 - phase
            } else {
                phase
            }
        };
        let sprite = self
            .sprites
            .get(motion.id)
            .context("SugarPot active EF sprite")?;
        //4383B0 only paints/commits active, non-system sprites. A rejected
        //target still owns the effect duration, matching the original loop.
        if sprite.visible && sprite.system == 0 {
            if finished {
                let sprite = &mut self.sprites[motion.id];
                if motion.selector <= 4 {
                    //4383B0 cases39..42 write only their moving axis. The
                    //other argument positions this presentation but does not
                    //overwrite the sprite's orthogonal saved coordinate.
                    let axis = usize::from(motion.selector >= 3);
                    sprite.position[axis] = motion.to[axis] as i16 as i32;
                    let layer=sprite.image.as_ref().map(|image|json!({"op":"set_rect","id":motion.id as u32+1,
                        "rect":[motion.to[0],motion.to[1],image.size[0],image.size[1]/sprite.divisions]}));
                    if let Some(layer) = layer {
                        self.submit(h, vec![layer], vec![])?;
                    }
                } else {
                    sprite.position = motion.to.map(|v| v as i16 as i32);
                    if matches!(motion.selector, 7 | 9) {
                        sprite.visible = false;
                    }
                    self.publish(h, None)?;
                }
            } else if matches!(motion.selector, 8 | 9) {
                //448934/4489A2 ->4383B0 cases46/47 ->433960: draw the
                //same native sprite twice, plus displacement first and minus
                //second, at its ordinary depth among the other sprites.
                //These are retained engine image layers, not a VM compositor.
                let phase = (255u32.wrapping_mul(elapsed as u32) as i32) / motion.duration as i32;
                let travel = motion.distance.wrapping_mul(elapsed as i32) / motion.duration as i32;
                let separation = if motion.selector == 8 {
                    motion.distance.wrapping_sub(travel)
                } else {
                    travel
                };
                let opacity = if motion.selector == 8 {
                    phase
                } else {
                    255 - phase
                };
                let opacity = Self::sprite_opacity(sprite, motion.id == 0, opacity);
                let order = if motion.id == 0 {
                    0
                } else {
                    100 + motion.id as i32
                };
                let mut layers = Vec::new();
                for (ordinal, delta) in [separation, separation.wrapping_neg()]
                    .into_iter()
                    .enumerate()
                {
                    let mut moved = sprite.clone();
                    moved.position[0] = motion.from[0].wrapping_add(delta);
                    let id = if ordinal == 0 {
                        motion.id as u32 + 1
                    } else {
                        65001
                    };
                    if let Some(mut layer) = Self::sprite_layer(id, &moved, order)? {
                        if motion.id == 0 && sprite.native_depth == 24 {
                            layer["layer"]["blend"] =
                                json!(Self::native_image_blend(24, sprite.attribute as u8 as i32));
                        }
                        layer["layer"]["order"] = json!([order, ordinal]);
                        layer["layer"]["opacity"] = json!(opacity);
                        layers.push(layer);
                    }
                }
                self.submit(h, layers, vec![])?;
                if !self.published_layers.contains(&65001) {
                    self.published_layers.push(65001);
                }
            } else if let Some(image) = &sprite.image {
                let position: [i32; 2] = std::array::from_fn(|axis| {
                    sample_integer_line(
                        elapsed as i64,
                        0,
                        motion.from[axis] as i64,
                        motion.duration as i64,
                        motion.to[axis] as i64,
                    ) as i32
                });
                let opacity = match motion.selector {
                    6 => sample_integer_line(elapsed as i64, 0, 0, motion.duration as i64, 255),
                    7 => sample_integer_line(elapsed as i64, 0, 255, motion.duration as i64, 0),
                    _ => 255,
                };
                let opacity = Self::sprite_opacity(sprite, motion.id == 0, opacity as i32);
                self.submit(h, vec![
                    json!({"op":"set_rect","id":motion.id as u32+1,"rect":[position[0],position[1],image.size[0],image.size[1]/sprite.divisions]}),
                    json!({"op":"set_opacity","id":motion.id as u32+1,"opacity":opacity})], vec![])?;
            }
        }
        Ok(finished)
    }
}

pub(super) struct Shaking {
    curves: Vec<(u32, [f32; 4], crysirror_vfx::shake::Shake)>,
}
impl SugarPotVm {
    pub(super) fn ef(&mut self, h: &mut dyn HostServices, args: &[String]) -> Result<bool> {
        let selector = self.number(args, 1, 0)?;
        if matches!(selector, 1..=9) {
            return self.ef_movement(h, args, selector);
        }
        if matches!(selector, 20..=28) {
            return self.begin_geometry(h, args, selector);
        }
        if selector == 30 {
            return self.begin_rotation(h, args);
        }
        if selector == 29 {
            return self.begin_warp(h, args);
        }
        ensure!(
            matches!(selector, 11 | 12 | 16 | 17 | 18 | 19),
            "SugarPot EF selector {selector} requires remaining native effect profile"
        );
        let target = self.number(args, 0, 0)?;
        let x = self.number(args, 2, 0)?;
        let y = self.number(args, 3, 0)?;
        let fixed = matches!(selector, 11 | 12 | 16 | 17);
        let duration = if fixed {
            if matches!(selector, 11 | 12) {
                198
            } else {
                132
            }
        } else {
            self.number(args, 4, 0)?
        };
        ensure!(
            (0..=600000).contains(&duration),
            "SugarPot EF shake duration"
        );
        let frames = duration / 33;
        //448530 explicitly writes current window.byte24=1 for11..19 and29,
        //even when the caller had previously hidden it with wndoff.
        if let Some(window) = self.windows.get_mut(self.current_window) {
            window.visibility = 1;
        }
        self.publish(h, None)?;
        // Portable repair for original duration<33: avoid its zero divisor and
        // finish at the unchanged origin instead of evaluating undefined motion.
        if frames == 0 {
            return Ok(true);
        }
        let mut curves = vec![];
        let mut visible = 0usize;
        for (id, sprite) in self.sprites.iter().enumerate() {
            if !sprite.visible {
                continue;
            }
            let ordinal = visible;
            visible += 1;
            if target != -1 && target != id as i32 {
                continue;
            }
            if target == -1 && sprite.system != 0 && !(fixed && id == 0) {
                continue;
            }
            let Some(image) = &sprite.image else {
                continue;
            };
            let mut steps = Vec::with_capacity(frames as usize);
            for index in 0..frames {
                if fixed {
                    //4383B0 cases48..51; both four-step tables are fully
                    //initialized jump tables (439A38/439A48), not the apparent
                    //uninitialized case3 produced by its decompiler merge.
                    let (dx, dy) = match selector {
                        11 => (if index % 2 == 0 { -8 } else { 8 }, 0),
                        12 => (0, if index % 2 == 0 { -8 } else { 8 }),
                        16 => [(0, 8), (8, 0), (0, -8), (-8, 0)][index as usize],
                        17 => [(0, 8), (-8, 0), (0, -8), (8, 0)][index as usize],
                        _ => unreachable!(),
                    };
                    steps.push((dx, dy, 33));
                    continue;
                }
                // 434190 stores these x87 expressions back to f32 before
                // signed truncation. The resulting table is native policy;
                // shared VFX Shake owns clocking and pattern playback.
                let ax = (x as f64 - x as f64 / frames as f64 * index as f64) as f32 as i32;
                let ay = (y as f64 - y as f64 / frames as f64 * index as f64) as f32 as i32;
                let (mut dx, mut dy) = if y > x {
                    match index % 4 {
                        0 => (ax.wrapping_neg(), ay.wrapping_neg()),
                        1 => (ax.wrapping_neg(), ay),
                        2 => (ax, ay.wrapping_neg()),
                        _ => (ax, ay),
                    }
                } else {
                    match index % 4 {
                        0 => (ax.wrapping_neg(), ay.wrapping_neg()),
                        1 => (ax, ay.wrapping_neg()),
                        2 => (ax.wrapping_neg(), ay),
                        _ => (ax, ay),
                    }
                };
                if target == -1 && selector == 19 {
                    match ordinal % 4 {
                        1 => dy = dy.wrapping_neg(),
                        2 => dx = dx.wrapping_neg(),
                        3 => {
                            dx = dx.wrapping_neg();
                            dy = dy.wrapping_neg();
                        }
                        _ => {}
                    }
                }
                steps.push((dx, dy, 33));
            }
            let curve = crysirror_vfx::shake::Shake::new(
                crysirror_vfx::shake::ShakeKind::Pattern(steps),
                h.now_ms(),
                1,
                0,
                crysirror_vfx::shake::Layers::ALL,
            );
            curves.push((
                id as u32 + 1,
                [
                    sprite.position[0] as f32,
                    sprite.position[1] as f32,
                    image.size[0] as f32,
                    (image.size[1] / sprite.divisions) as f32,
                ],
                curve,
            ));
        }
        self.shaking = Some(Shaking { curves });
        self.wait = Some(Wait::Shake(h.now_ms().saturating_add(frames as u64 * 33)));
        self.shake_frame(h, false)?;
        Ok(true)
    }
    pub(super) fn shake_frame(&mut self, h: &mut dyn HostServices, finished: bool) -> Result<()> {
        let Some(shaking) = &self.shaking else {
            return Ok(());
        };
        let layers = shaking
            .curves
            .iter()
            .map(|(id, rect, curve)| {
                let mut rect = *rect;
                if !finished {
                    if let Some(offset) = curve.offset(h.now_ms()) {
                        rect[0] += offset.dx as f32;
                        rect[1] += offset.dy as f32;
                    }
                }
                json!({"op":"set_rect","id":id,"rect":rect})
            })
            .collect();
        self.submit(h, layers, vec![])?;
        if finished {
            self.shaking = None;
        }
        Ok(())
    }
}
