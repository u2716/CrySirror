// Copyright (c) 2026 CrySirror contributors. MIT.
//! Native Majiro picture/sprite semantics (419500..41BD30, 43BF70..43FD00).
//!
//! Pages are VM-visible 8-bit indexed or 24-bit BGR pictures with an optional
//! linked transparency page. Scripts read pixels back (screen captures for
//! transitions, `grp_point_get`), and sprites write *destructively* into their
//! destination pictures during the native dirty-tile redraw (43C790), so this
//! state is retained exactly as the original data model. Every pixel kernel is
//! the shared `crysirror_image::transparency_planes` capability; the primary
//! picture is presented through an engine image resource and layer.
use super::*;
use crysirror_core::image_processing::ImageResource;
use crysirror_image::transparency_planes::{self as planes, Plane, PlaneFormat};

pub(super) const PAGE_BASE: u32 = 0x0010_0000;
pub(super) const SPRITE_BASE: u32 = 0x0100_0000;
const TILE_W: i32 = 160;
const TILE_H: i32 = 40;
const PRIMARY_LAYER: u32 = 1;

#[derive(Clone)]
pub(super) struct Page {
    pub plane: Plane,
    /// Linked transparency page (picture offset 28).
    pub alpha: Option<u32>,
    /// Whether this page owns/release its alpha page (offset 20).
    pub owns_alpha: bool,
    /// page_drawmode: 1 draws additively (offset 24).
    pub mode: i32,
    pub version: u64,
    /// Allocated during `$init@GLOBAL` (4403F0): survives a reset.
    pub persistent: bool,
}
struct Screen {
    cols: i32,
    rows: i32,
    tiles: Vec<bool>,
    dirty: bool,
    refs: i32,
}
/// Keyframe step engine shared by cell, transparency and zoom sequences.
#[derive(Clone, Default)]
struct Track {
    active: u8,
    base: u32,
    duration: i32,
    index: i32,
    previous: f64,
    entries: Vec<(f64, i32)>,
}
impl Track {
    fn define(&mut self, entries: Vec<(f64, i32)>, previous: f64, now: u32) {
        self.entries = entries;
        self.previous = previous;
        self.active = 1;
        self.duration = 0;
        self.index = -1;
        self.base = now;
    }
    /// Advance with catch-up; returns elapsed time within the current entry.
    fn step(&mut self, now: u32) -> i32 {
        if self.active == 0 || self.entries.is_empty() {
            return 0;
        }
        let mut elapsed = (now.wrapping_sub(self.base) as i32).wrapping_abs();
        if elapsed >= self.duration {
            let mut n = 0;
            loop {
                elapsed -= self.duration;
                if self.index >= 0 {
                    self.previous = self.entries[self.index as usize].0;
                }
                self.index += 1;
                if self.index as usize >= self.entries.len() {
                    self.index = 0;
                    self.active = 2;
                }
                let next = self.entries[self.index as usize].1;
                if next < 0 {
                    self.active = 0;
                    elapsed = 0;
                    break;
                }
                self.duration = next;
                n += 1;
                if !(elapsed >= next && n < 100) {
                    break;
                }
            }
            self.base = now.wrapping_sub(elapsed.max(0) as u32);
        }
        elapsed
    }
    fn value(&self, elapsed: i32) -> f64 {
        let Some(&(value, duration)) = self.entries.get(self.index.max(0) as usize) else {
            return self.previous;
        };
        if duration <= 0 {
            value
        } else {
            let e = f64::from(elapsed.clamp(0, duration));
            (e * value + self.previous * (f64::from(duration) - e)) / f64::from(duration)
        }
    }
}
#[derive(Clone)]
pub(super) struct Sprite {
    pub page: u32,
    pub src: [i32; 4],
    base_src: [i32; 2],
    pub dest: u32,
    pub pos: [i32; 2],
    pub transparency: u8,
    group: u32,
    hidden: i32,
    pub owns_page: bool,
    moving: u8,
    move_start: u32,
    move_duration: i32,
    move_from: [i32; 2],
    move_delta: [i32; 2],
    cells: Track,
    alpha: Track,
    transformed: bool,
    temp: Option<Plane>,
    temp_mask: Option<Plane>,
    temp_valid: bool,
    temp_pos: [i32; 2],
    angle: f32,
    angle_mode: u8,
    angle_from: f32,
    angle_to: f32,
    angle_start: u32,
    angle_duration: i32,
    zoom_x: f32,
    zoom_y: f32,
    zoom_x_track: Track,
    zoom_y_track: Track,
    pub persistent: bool,
}
#[derive(Default)]
pub(super) struct Gfx {
    pub pages: BTreeMap<u32, Page>,
    next_page: u32,
    pub primary: Option<u32>,
    screens: BTreeMap<u32, Screen>,
    pub sprites: BTreeMap<u32, Sprite>,
    /// Top (head, drawn last) to bottom (tail, drawn first).
    pub order: Vec<u32>,
    next_sprite: u32,
    pub lock: i32,
    presented: Option<(u32, u64, String)>,
    pub viewport: [i32; 2],
    presented_viewport: [i32; 2],
    pub client_size: [i32; 2],
    pub warnings: u64,
}
#[derive(Debug)]
struct PrimaryImage {
    client: ClientId,
    size: [u32; 2],
    rgba: Arc<[u8]>,
}
impl ImageResource for PrimaryImage {
    fn client(&self) -> ClientId {
        self.client
    }
    fn size(&self) -> [u32; 2] {
        self.size
    }
    fn rgba(&self) -> Arc<[u8]> {
        self.rgba.clone()
    }
    fn opaque(&self) -> bool {
        true
    }
    fn pixel_at(&self, point: [u32; 2]) -> crysirror_core::Result<Option<[u8; 4]>> {
        crysirror_core::image_processing::rgba_pixel_at(self.size, &self.rgba, point)
    }
}

impl Gfx {
    pub fn warn(&mut self, message: String) {
        self.warnings += 1;
        if self.warnings <= 200 {
            eprintln!("[majiro] {message}");
        }
    }
    pub fn new(client_size: [i32; 2]) -> Self {
        Self {
            next_page: PAGE_BASE,
            next_sprite: SPRITE_BASE,
            client_size,
            ..Default::default()
        }
    }
    pub fn alloc_page(&mut self, plane: Plane) -> Result<u32> {
        ensure!(self.pages.len() < 8192, "Majiro page budget");
        let id = self.next_page;
        self.next_page = self.next_page.checked_add(1).context("Majiro page ids")?;
        self.pages.insert(
            id,
            Page {
                plane,
                alpha: None,
                owns_alpha: false,
                mode: 0,
                version: 0,
                persistent: false,
            },
        );
        Ok(id)
    }
    pub fn page(&self, id: i32) -> Option<&Page> {
        self.pages.get(&(id as u32))
    }
    pub fn valid(&self, id: i32) -> bool {
        self.pages.contains_key(&(id as u32))
    }
    pub fn touch(&mut self, id: u32) {
        if let Some(page) = self.pages.get_mut(&id) {
            page.version = page.version.wrapping_add(1);
        }
    }
    /// The primary picture always has a screen (43C260/401B70).
    pub fn set_primary(&mut self, id: Option<u32>) {
        if let Some(old) = self.primary {
            if let Some(screen) = self.screens.get_mut(&old) {
                screen.refs -= 1;
            }
        }
        self.primary = id;
        if let Some(id) = id {
            self.screen_ref(id);
            let [w, h] = self.size(id);
            self.invalidate(id, [0, 0, w, h]);
        }
        self.prune_screens();
    }
    fn size(&self, id: u32) -> [i32; 2] {
        self.pages
            .get(&id)
            .map_or([0, 0], |p| [p.plane.width as i32, p.plane.height as i32])
    }
    fn screen_ref(&mut self, page: u32) {
        let [w, h] = self.size(page);
        let screen = self.screens.entry(page).or_insert_with(|| {
            let cols = (w + TILE_W - 1) / TILE_W;
            let rows = (h + TILE_H - 1) / TILE_H;
            Screen {
                cols,
                rows,
                tiles: vec![false; (cols * rows).max(0) as usize],
                dirty: false,
                refs: 0,
            }
        });
        screen.refs += 1;
    }
    fn prune_screens(&mut self) {
        let primary = self.primary;
        self.screens
            .retain(|id, screen| screen.refs > 0 || Some(*id) == primary);
    }
    /// 43C590: mark dirty tiles and propagate to sprites sourcing this page.
    pub fn invalidate(&mut self, page: u32, rect: [i32; 4]) {
        self.invalidate_depth(page, rect, 0);
    }
    fn invalidate_depth(&mut self, page: u32, [x, y, w, h]: [i32; 4], depth: u32) {
        if self.lock != 0 || depth > 16 {
            return;
        }
        self.touch(page);
        if let Some(screen) = self.screens.get_mut(&page) {
            screen.dirty = true;
            let x0 = (x / TILE_W).max(0);
            let y0 = (y / TILE_H).max(0);
            let x1 = ((x + w - 1) / TILE_W).min(screen.cols - 1);
            let y1 = ((y + h - 1) / TILE_H).min(screen.rows - 1);
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    screen.tiles[(ty * screen.cols + tx) as usize] = true;
                }
            }
        }
        let alpha_of = |pages: &BTreeMap<u32, Page>, id: u32| pages.get(&id).and_then(|p| p.alpha);
        let mut targets = Vec::new();
        for sprite in self.sprites.values_mut() {
            if sprite.dest == page {
                continue;
            }
            let sources = sprite.page == page || alpha_of(&self.pages, sprite.page) == Some(page);
            if !sources {
                continue;
            }
            if sprite.transformed {
                sprite.temp_valid = false;
                targets.push((sprite.dest, [0, 0, 9999, 9999]));
            } else {
                targets.push((
                    sprite.dest,
                    [x + sprite.pos[0] - sprite.src[0], y + sprite.pos[1] - sprite.src[1], w, h],
                ));
            }
        }
        for (dest, rect) in targets {
            self.invalidate_depth(dest, rect, depth + 1);
        }
    }
    pub fn invalidate_all(&mut self, page: u32) {
        let [w, h] = self.size(page);
        self.invalidate(page, [0, 0, w, h]);
    }
    /// Borrow two pages mutably/immutably even when they are the same id.
    fn with_pair<R>(
        &mut self,
        src: u32,
        dst: u32,
        f: impl FnOnce(&Plane, &mut Plane, Option<&Plane>, Option<&mut Plane>) -> R,
    ) -> Option<R> {
        let source = self.pages.get(&src)?.clone();
        let src_alpha = source.alpha.and_then(|a| self.pages.get(&a)).map(|p| p.plane.clone());
        let dst_alpha_id = self.pages.get(&dst)?.alpha;
        let mut dst_alpha = dst_alpha_id.and_then(|a| self.pages.get(&a)).map(|p| p.plane.clone());
        let page = self.pages.get_mut(&dst)?;
        let result = f(&source.plane, &mut page.plane, src_alpha.as_ref(), dst_alpha.as_mut());
        if let (Some(id), Some(plane)) = (dst_alpha_id, dst_alpha) {
            if let Some(alpha) = self.pages.get_mut(&id) {
                alpha.plane = plane;
            }
        }
        Some(result)
    }
    /// 43CBA0: draw one sprite region with its transparency.
    fn draw(&mut self, src: u32, rect: [i32; 4], dst: u32, at: [i32; 2], t: u8) {
        let additive = self.pages.get(&src).is_some_and(|p| p.mode == 1);
        let src_has_alpha = self.pages.get(&src).and_then(|p| p.alpha).is_some();
        self.with_pair(src, dst, |s, d, sa, da| {
            if additive {
                planes::additive(s, rect, d, at, t);
                return;
            }
            match (sa, da) {
                (Some(mask), dm) => planes::masked_blend(s, mask, rect, d, dm, at, t),
                (None, Some(dm)) => {
                    if t == 0 {
                        planes::copy(s, rect, d, at);
                    } else {
                        planes::blend(s, rect, d, at, t);
                    }
                    dm.fill_blend([at[0], at[1], rect[2], rect[3]], 0, u16::from(t));
                }
                (None, None) => {
                    if t == 0 {
                        planes::copy(s, rect, d, at);
                    } else {
                        planes::blend(s, rect, d, at, t);
                    }
                }
            }
        });
        let _ = src_has_alpha;
        self.touch(dst);
    }
    fn redraw(&mut self, page: u32) {
        let Some(screen) = self.screens.get_mut(&page) else {
            return;
        };
        if !screen.dirty {
            return;
        }
        screen.dirty = false;
        let (cols, rows) = (screen.cols, screen.rows);
        let tiles = std::mem::replace(&mut screen.tiles, vec![false; (cols * rows).max(0) as usize]);
        let [w, h] = self.size(page);
        let bottom_up: Vec<u32> = self.order.iter().rev().copied().collect();
        for ty in 0..rows {
            for tx in 0..cols {
                if !tiles[(ty * cols + tx) as usize] {
                    continue;
                }
                let tile = [tx * TILE_W, ty * TILE_H, (w - tx * TILE_W).min(TILE_W), (h - ty * TILE_H).min(TILE_H)];
                for id in &bottom_up {
                    let Some(s) = self.sprites.get(id) else { continue };
                    if s.dest != page || s.hidden != 0 || s.transparency == 255 {
                        continue;
                    }
                    let (src, rect, at) = if s.transformed {
                        if s.temp.is_some() {
                            let temp = s.temp.as_ref().unwrap();
                            (u32::MAX, [0, 0, temp.width as i32, temp.height as i32], s.temp_pos)
                        } else {
                            (s.page, [0, 0, s.src[2], s.src[3]], [s.pos[0] - s.src[2] / 2, s.pos[1] - s.src[3] / 2])
                        }
                    } else {
                        let Some(src_page) = self.pages.get(&s.page) else { continue };
                        let Some(r) = src_page.plane.clip(s.src) else { continue };
                        let at = [s.pos[0] + (r[0] - s.src[0]), s.pos[1] + (r[1] - s.src[1])];
                        (s.page, r, at)
                    };
                    // clip to tile
                    let mut r = rect;
                    let mut a = at;
                    if a[0] < tile[0] {
                        r[0] += tile[0] - a[0];
                        r[2] -= tile[0] - a[0];
                        a[0] = tile[0];
                    }
                    if a[1] < tile[1] {
                        r[1] += tile[1] - a[1];
                        r[3] -= tile[1] - a[1];
                        a[1] = tile[1];
                    }
                    r[2] = r[2].min(tile[0] + tile[2] - a[0]);
                    r[3] = r[3].min(tile[1] + tile[3] - a[1]);
                    if r[2] <= 0 || r[3] <= 0 {
                        continue;
                    }
                    let t = s.transparency;
                    if src == u32::MAX {
                        let temp = s.temp.clone().unwrap();
                        let mask = s.temp_mask.clone();
                        if let Some(page) = self.pages.get_mut(&page) {
                            match &mask {
                                Some(m) => planes::masked_blend(&temp, m, r, &mut page.plane, None, a, t),
                                None => planes::blend(&temp, r, &mut page.plane, a, t),
                            }
                        }
                    } else {
                        self.draw(src, r, page, a, t);
                    }
                }
            }
        }
        self.touch(page);
    }
    /// 43D660 linear move.
    fn update_move(&mut self, id: u32, now: u32) {
        let Some(s) = self.sprites.get_mut(&id) else { return };
        if s.moving != 1 {
            return;
        }
        let mut e = now.wrapping_sub(s.move_start) as i32;
        if e >= s.move_duration {
            s.moving = 2;
            e = s.move_duration;
        }
        let new = if s.move_duration != 0 {
            [
                (f64::from(s.move_delta[0]) * f64::from(e) / f64::from(s.move_duration) + f64::from(s.move_from[0])) as i32,
                (f64::from(s.move_delta[1]) * f64::from(e) / f64::from(s.move_duration) + f64::from(s.move_from[1])) as i32,
            ]
        } else {
            [s.move_from[0] + s.move_delta[0], s.move_from[1] + s.move_delta[1]]
        };
        if new != s.pos {
            let old = s.pos;
            s.pos = new;
            let (dest, w, h, transformed) = (s.dest, s.src[2], s.src[3], s.transformed);
            if transformed {
                s.temp_valid = false;
                let r = self.transformed_rect(id);
                self.invalidate(dest, r);
            } else {
                self.invalidate(dest, [old[0], old[1], w, h]);
                self.invalidate(dest, [new[0], new[1], w, h]);
            }
        }
    }
    fn sprite_rect(&self, id: u32) -> Option<(u32, [i32; 4])> {
        let s = self.sprites.get(&id)?;
        Some(if s.transformed {
            (s.dest, self.transformed_rect(id))
        } else {
            (s.dest, [s.pos[0], s.pos[1], s.src[2], s.src[3]])
        })
    }
    fn transformed_rect(&self, id: u32) -> [i32; 4] {
        let Some(s) = self.sprites.get(&id) else { return [0; 4] };
        match &s.temp {
            Some(t) => [s.temp_pos[0], s.temp_pos[1], t.width as i32, t.height as i32],
            None => [s.pos[0] - s.src[2] / 2, s.pos[1] - s.src[3] / 2, s.src[2], s.src[3]],
        }
    }
    /// 43CDC0 per-frame sprite update (move, cells, transparency, transform).
    fn update_sprite(&mut self, id: u32, now: u32) {
        self.update_move(id, now);
        let Some(s) = self.sprites.get_mut(&id) else { return };
        let mut invalidate = false;
        if s.cells.active != 0 {
            let old = s.src;
            s.cells.step(now);
            let cell = s.cells.entries.get(s.cells.index.max(0) as usize).map_or(-2.0, |e| e.0) as i32;
            if cell == -2 {
                s.src[0] = s.base_src[0];
                s.src[1] = s.base_src[1];
            } else if s.src[2] > 0 && s.src[3] > 0 {
                let page_w = self.pages.get(&s.page).map_or(1, |p| p.plane.width as i32);
                let cols = (page_w / s.src[2]).max(1);
                s.src[0] = s.src[2] * (cell % cols);
                s.src[1] = s.src[3] * (cell / cols);
            }
            if s.src != old {
                s.temp_valid = false;
                invalidate = true;
            }
        }
        if s.alpha.active != 0 {
            let e = s.alpha.step(now);
            let value = s.alpha.value(e).clamp(0.0, 255.0) as u8;
            if value != s.transparency {
                s.transparency = value;
                invalidate = true;
            }
        }
        if s.transformed {
            if s.angle_mode != 0 {
                let mut e = now.wrapping_sub(s.angle_start) as i32;
                e = e.wrapping_abs();
                if s.angle_mode == 1 && e >= s.angle_duration {
                    e = s.angle_duration;
                    s.angle_mode = 0;
                }
                let value = if s.angle_duration > 0 {
                    (s.angle_to - s.angle_from) / s.angle_duration as f32 * e as f32 + s.angle_from
                } else {
                    s.angle_to
                };
                if value != s.angle {
                    s.angle = value;
                    s.temp_valid = false;
                }
            }
            for (track, zoom) in [(&mut s.zoom_x_track, &mut s.zoom_x), (&mut s.zoom_y_track, &mut s.zoom_y)] {
                if track.active != 0 {
                    let e = track.step(now);
                    let value = track.value(e) as f32;
                    if value != *zoom {
                        *zoom = value;
                        s.temp_valid = false;
                    }
                }
            }
        }
        let transformed = s.transformed && !s.temp_valid && s.transparency != 255 && s.hidden == 0;
        if invalidate {
            if let Some((dest, rect)) = self.sprite_rect(id) {
                self.invalidate(dest, rect);
            }
        }
        if transformed {
            self.render_transformed(id);
        }
    }
    /// 448750 rotate/zoom into the sprite's temporary picture (nearest).
    fn render_transformed(&mut self, id: u32) {
        let Some(s) = self.sprites.get(&id) else { return };
        let Some(src) = self.pages.get(&s.page) else { return };
        let src_alpha = src.alpha.and_then(|a| self.pages.get(&a)).map(|p| p.plane.clone());
        let src_plane = src.plane.clone();
        let (w, h) = (s.src[2], s.src[3]);
        let old = s.temp.as_ref().map(|t| [s.temp_pos[0], s.temp_pos[1], t.width as i32, t.height as i32]);
        let radius = (f64::from((h / 2) * (h / 2) + (w / 2) * (w / 2))).sqrt();
        let r = ((s.zoom_x.abs() as f64 * radius) as i32 + 1).max((s.zoom_y.abs() as f64 * radius) as i32 + 1);
        let size = (2 * r).max(1) as u32;
        let Ok(mut temp) = Plane::new(size, size, PlaneFormat::Bgr24) else { return };
        let Ok(mut mask) = Plane::new(size, size, PlaneFormat::Index8) else { return };
        mask.pixels.fill(255);
        let (sin, cos) = (f64::from(s.angle).to_radians().sin(), f64::from(s.angle).to_radians().cos());
        let (zx, zy) = (f64::from(s.zoom_x), f64::from(s.zoom_y));
        let (cx, cy) = (f64::from(s.src[0]) + f64::from(w) / 2.0, f64::from(s.src[1]) + f64::from(h) / 2.0);
        if zx.abs() > 1e-6 && zy.abs() > 1e-6 {
            for y in 0..size as i32 {
                for x in 0..size as i32 {
                    let dx = f64::from(x - r) + 0.5;
                    let dy = f64::from(y - r) + 0.5;
                    let ux = (dx * cos + dy * sin) / zx;
                    let uy = (-dx * sin + dy * cos) / zy;
                    let sx = (cx + ux).floor() as i32;
                    let sy = (cy + uy).floor() as i32;
                    if sx < s.src[0] || sy < s.src[1] || sx >= s.src[0] + w || sy >= s.src[1] + h {
                        continue;
                    }
                    let Some(bgr) = src_plane.bgr_at(sx, sy) else { continue };
                    let at = (y as usize * size as usize + x as usize) * 3;
                    temp.pixels[at..at + 3].copy_from_slice(&bgr);
                    mask.pixels[y as usize * size as usize + x as usize] =
                        src_alpha.as_ref().and_then(|a| a.index_at(sx, sy)).unwrap_or(0);
                }
            }
        }
        let pos = [s.pos[0] - r, s.pos[1] - r];
        let dest = s.dest;
        let s = self.sprites.get_mut(&id).unwrap();
        s.temp = Some(temp);
        s.temp_mask = Some(mask);
        s.temp_pos = pos;
        s.temp_valid = true;
        if let Some(old) = old {
            self.invalidate(dest, old);
        }
        self.invalidate(dest, [pos[0], pos[1], size as i32, size as i32]);
    }
    /// 43CCB0 frame render: non-primary destinations first, then primary.
    pub fn render(&mut self, now: u32) {
        let primary = self.primary;
        let ids: Vec<u32> = self.order.clone();
        for id in &ids {
            if self.sprites.get(id).is_some_and(|s| Some(s.dest) != primary) {
                self.update_sprite(*id, now);
            }
        }
        if self.lock == 0 {
            let screens: Vec<u32> = self.screens.keys().copied().filter(|p| Some(*p) != primary).collect();
            for page in screens {
                self.redraw(page);
            }
        }
        for id in &ids {
            if self.sprites.get(id).is_some_and(|s| Some(s.dest) == primary) {
                self.update_sprite(*id, now);
            }
        }
        if self.lock == 0 {
            if let Some(primary) = primary {
                self.redraw(primary);
            }
        }
    }
    /// Publish the primary picture when its content or identity changed.
    pub fn present(&mut self, client: ClientId, host: &mut dyn HostServices) -> Result<()> {
        let Some(primary) = self.primary else { return Ok(()) };
        let Some(page) = self.pages.get(&primary) else { return Ok(()) };
        let version = page.version;
        if self
            .presented
            .as_ref()
            .is_some_and(|(id, v, _)| *id == primary && *v == version)
            && self.presented_viewport == self.viewport
        {
            return Ok(());
        }
        let size = [page.plane.width, page.plane.height];
        let rgba: Arc<[u8]> = page.plane.to_rgba(None).into();
        let upload = host
            .publish_image_resource(client, PRIMARY_LAYER, Arc::new(PrimaryImage { client, size, rgba }))
            .map_err(core_error)?;
        let path = upload.path.relative().to_owned();
        let [w, h] = self.client_size;
        let ox = if size[0] > 0 { self.viewport[0].rem_euclid(size[0] as i32) } else { 0 };
        let oy = if size[1] > 0 { self.viewport[1].rem_euclid(size[1] as i32) } else { 0 };
        let mut layers = vec![json!({"op":"put","layer":{"id":PRIMARY_LAYER,"rect":[-ox,-oy,size[0],size[1]],
            "order":[0,0],"paint":{"kind":"image","asset":path}}})];
        if ox != 0 || oy != 0 {
            for (i, (dx, dy)) in [(size[0] as i32, 0), (0, size[1] as i32), (size[0] as i32, size[1] as i32)].iter().enumerate() {
                layers.push(json!({"op":"put","layer":{"id":PRIMARY_LAYER+1+i as u32,"rect":[dx-ox,dy-oy,size[0],size[1]],
                    "order":[0,1+i],"paint":{"kind":"image","asset":path}}}));
            }
        } else if self.presented_viewport != [0, 0] {
            for i in 0..3u32 {
                layers.push(json!({"op":"remove","id":PRIMARY_LAYER+1+i}));
            }
        }
        let _ = (w, h);
        host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":layers}))?)
            .map_err(core_error)?;
        if let Some((_, _, old)) = self.presented.replace((primary, version, path)) {
            host.release_image_resource(client, &old).map_err(core_error)?;
        }
        self.presented_viewport = self.viewport;
        Ok(())
    }
    pub fn new_sprite(&mut self, page: u32, src: [i32; 4], dest: u32, now: u32) -> Result<u32> {
        ensure!(self.sprites.len() < 8192, "Majiro sprite budget");
        let id = self.next_sprite;
        self.next_sprite = self.next_sprite.checked_add(1).context("Majiro sprite ids")?;
        self.sprites.insert(
            id,
            Sprite {
                page,
                src,
                base_src: [src[0], src[1]],
                dest,
                pos: [100000, 100000],
                transparency: 0,
                group: id,
                hidden: 0,
                owns_page: false,
                moving: 0,
                move_start: now,
                move_duration: 0,
                move_from: [0, 0],
                move_delta: [0, 0],
                cells: Track::default(),
                alpha: Track::default(),
                transformed: false,
                temp: None,
                temp_mask: None,
                temp_valid: false,
                temp_pos: [0, 0],
                angle: 0.0,
                angle_mode: 0,
                angle_from: 0.0,
                angle_to: 0.0,
                angle_start: now,
                angle_duration: 0,
                zoom_x: 1.0,
                zoom_y: 1.0,
                zoom_x_track: Track::default(),
                zoom_y_track: Track::default(),
                persistent: false,
            },
        );
        self.order.insert(0, id);
        self.screen_ref(dest);
        Ok(id)
    }
    pub fn release_sprite(&mut self, id: u32) {
        let Some((dest, rect)) = self.sprite_rect(id) else { return };
        self.invalidate(dest, rect);
        let s = self.sprites.remove(&id).unwrap();
        self.order.retain(|x| *x != id);
        if let Some(screen) = self.screens.get_mut(&s.dest) {
            screen.refs -= 1;
        }
        self.prune_screens();
        if s.owns_page {
            self.release_page(s.page, true);
        }
    }
    pub fn page_in_use(&self, id: u32) -> bool {
        self.sprites.values().any(|s| s.page == id || s.dest == id)
    }
    pub fn release_page(&mut self, id: u32, force: bool) -> bool {
        if !force && self.page_in_use(id) {
            return false;
        }
        if let Some(page) = self.pages.remove(&id) {
            if page.owns_alpha {
                if let Some(alpha) = page.alpha {
                    self.pages.remove(&alpha);
                }
            }
        }
        true
    }
    /// 43C220/4403F0 after the global initializer.
    pub fn mark_persistent(&mut self) {
        for s in self.sprites.values_mut() {
            s.persistent = true;
        }
        for p in self.pages.values_mut() {
            p.persistent = true;
        }
    }
    /// 43C260 + 440380: release transient sprites/pages, re-arm persistent
    /// sprites hidden at the native 100000 origin.
    pub fn reset(&mut self) {
        let transient: Vec<u32> = self.sprites.iter().filter(|(_, s)| !s.persistent).map(|(id, _)| *id).collect();
        for id in transient {
            self.release_sprite(id);
        }
        for s in self.sprites.values_mut() {
            s.moving = 0;
            s.cells.active = 0;
            s.alpha.active = 0;
            s.angle_mode = 0;
            s.zoom_x_track.active = 0;
            s.zoom_y_track.active = 0;
            s.hidden = 0;
            s.temp = None;
            s.temp_mask = None;
            s.temp_valid = false;
            s.pos = [100000, 100000];
        }
        let used: std::collections::BTreeSet<u32> = self
            .sprites
            .values()
            .flat_map(|s| [s.page, s.dest])
            .chain(self.primary)
            .collect();
        let alphas: std::collections::BTreeSet<u32> = self.pages.values().filter(|p| p.persistent).filter_map(|p| p.alpha).collect();
        self.pages.retain(|id, p| p.persistent || used.contains(id) || alphas.contains(id));
        self.lock = 0;
        self.viewport = [0, 0];
        if let Some(primary) = self.primary {
            self.invalidate_all(primary);
        }
    }
    fn reorder(&mut self, id: u32) {
        if let Some((dest, rect)) = self.sprite_rect(id) {
            self.invalidate(dest, rect);
        }
    }
}

// ---------------------------------------------------------------- pictures
fn basename(bytes: &[u8]) -> Result<String> {
    let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
    ensure!(!bytes.is_empty() && bytes.len() <= 4096, "Majiro picture filename extent");
    let name = encoding_rs::SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(bytes)
        .context("Majiro picture filename CP932")?;
    unicode_picture_stem(&name)
}
/// Resource readers already return decoded Unicode names. Keep that boundary
/// separate from native VM CP932 byte arguments (notably differential RCT01).
fn unicode_picture_stem(name: &str) -> Result<String> {
    // A CP932 input byte can occupy up to three UTF8 bytes after decoding.
    ensure!(!name.is_empty() && name.len() <= 3 * 4096, "Majiro decoded picture filename extent");
    let base = name.rsplit(['/', '\\']).next().unwrap();
    let stem = base.rsplit_once('.').map_or(base, |(stem, _)| stem);
    ensure!(!stem.is_empty() && !stem.contains(':'), "Majiro picture basename");
    Ok(stem.to_owned())
}
pub(super) struct LoadedPicture {
    pub plane: Plane,
}
impl MajiroVm {
    fn picture_path(&self, stem: &str) -> Option<String> {
        //4182C0 ignores the supplied extension and searches this exact order.
        for ext in ["png", "bmp", "rct", "rc8"] {
            if let Some(paths) = self.module_catalog.assets.get(&format!("{stem}.{ext}").to_ascii_lowercase()) {
                return paths.last().cloned();
            }
        }
        None
    }
    pub(super) fn picture_exists(&self, name: &[u8]) -> bool {
        basename(name).ok().and_then(|stem| self.picture_path(&stem)).is_some()
    }
    fn read_asset(&self, host: &mut dyn HostServices, path: &str) -> Result<Vec<u8>> {
        let storage = host.storage(self.client).map_err(core_error)?;
        let reader = storage.open_asset(&VfsPath::new(Mount::Asset, path)?)?;
        crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)
    }
    /// 418900: decode a picture file into a native plane.
    pub(super) fn load_picture(&mut self, host: &mut dyn HostServices, stem: &str) -> Result<LoadedPicture> {
        if let Some(cached) = self.picture_cache.get(&stem.to_ascii_lowercase()) {
            return Ok(LoadedPicture { plane: cached.clone() });
        }
        let path = self.picture_path(stem).with_context(|| format!("Majiro picture {stem:?} is absent"))?;
        let bytes = self.read_asset(host, &path)?;
        let plane = if bytes.starts_with(b"\x98\x5a\x92\x9aT") {
            let key = self.picture_key.clone();
            let (w, h, pixels, base) = match &key {
                Some(key) => key.with_bytes(|k| crysirror_formats_arc::images_c::majiro::rct_bgr(&bytes, Some(k)))?,
                None => crysirror_formats_arc::images_c::majiro::rct_bgr(&bytes, None)?,
            };
            let mut plane = Plane::from_pixels(w, h, PlaneFormat::Bgr24, pixels, vec![])?;
            if let Some(base) = base {
                // rct_bgr has decoded this CP932 metadata once already;
                // interpreting its UTF8 bytes as CP932 corrupts Japanese names.
                let base_stem = unicode_picture_stem(&base)?;
                if let Ok(base) = self.load_picture(host, &base_stem) {
                    // Differential RCT: key colour (R=255, G=0) shows the base.
                    for i in 0..(w * h) as usize {
                        let p = &plane.pixels[i * 3..i * 3 + 3];
                        if p[2] == 255 && p[1] == 0 {
                            let x = (i % w as usize) as i32;
                            let y = (i / w as usize) as i32;
                            if let Some(b) = base.plane.bgr_at(x, y) {
                                plane.pixels[i * 3..i * 3 + 3].copy_from_slice(&b);
                            }
                        }
                    }
                }
            }
            plane
        } else if bytes.starts_with(b"\x98\x5a\x92\x9a8") {
            let (w, h, pixels, palette) = crysirror_formats_arc::images_c::majiro::rc8_indexed(&bytes)?;
            Plane::from_pixels(w, h, PlaneFormat::Index8, pixels, palette)?
        } else {
            let mut registry = crysirror_image::DecoderRegistry::default();
            crysirror_formats_arc::register_images(&mut registry)?;
            let image = registry.decode_with(&path, &bytes, None, None, None)?;
            let mut pixels = Vec::with_capacity((image.width * image.height * 3) as usize);
            for p in image.rgba.chunks_exact(4) {
                pixels.extend_from_slice(&[p[2], p[1], p[0]]);
            }
            Plane::from_pixels(image.width, image.height, PlaneFormat::Bgr24, pixels, vec![])?
        };
        if self.picture_cache.len() >= 64 {
            let first = self.picture_cache.keys().next().cloned().unwrap();
            self.picture_cache.remove(&first);
        }
        self.picture_cache.insert(stem.to_ascii_lowercase(), plane.clone());
        Ok(LoadedPicture { plane })
    }
    /// Picture page plus its `<stem>_` transparency companion when present.
    fn load_page(&mut self, host: &mut dyn HostServices, name: &[u8], with_alpha: bool) -> Result<u32> {
        let stem = basename(name)?;
        let picture = match self.load_picture(host, &stem) {
            Ok(p) => p.plane,
            Err(error) => {
                self.warn(format!("Majiro picture {stem:?}: {error:#}"));
                Plane::new(1, 1, PlaneFormat::Bgr24)?
            }
        };
        let id = self.gfx.alloc_page(picture)?;
        if with_alpha {
            let mask_stem = format!("{stem}_");
            if self.picture_path(&mask_stem).is_some() {
                let mask = self.load_picture(host, &mask_stem)?.plane;
                let page = &self.gfx.pages[&id];
                if mask.format == PlaneFormat::Index8
                    && mask.width == page.plane.width
                    && mask.height == page.plane.height
                {
                    let alpha = self.gfx.alloc_page(mask)?;
                    let page = self.gfx.pages.get_mut(&id).unwrap();
                    page.alpha = Some(alpha);
                    page.owns_alpha = true;
                } else {
                    self.warn(format!("Majiro mask {mask_stem:?} is not an equally sized indexed picture"));
                }
            }
        }
        Ok(id)
    }
    pub(super) fn warn(&mut self, message: String) {
        self.gfx.warn(message);
    }
}

// ---------------------------------------------------------------- syscalls
fn int(a: &[Value], i: usize) -> i32 {
    a.get(i).and_then(|v| v.integer().ok()).unwrap_or(0)
}
fn opt(a: &[Value], i: usize, default: i32) -> i32 {
    a.get(i).and_then(|v| v.integer().ok()).unwrap_or(default)
}
fn float(a: &[Value], i: usize) -> f32 {
    match a.get(i) {
        Some(Value::Float(f)) => *f,
        Some(Value::Integer(n)) => *n as f32,
        _ => 0.0,
    }
}
/// Outcome of a graphics syscall: a value, or "retry next frame" (wait).
pub(super) enum GfxOut {
    Value(i32),
    Wait,
}
impl MajiroVm {
    pub(super) fn try_gfx(
        &mut self,
        hash: u32,
        a: &[Value],
        host: &mut dyn HostServices,
    ) -> Result<Option<GfxOut>> {
        use GfxOut::Value as V;
        let now = host.now_ms() as u32;
        let g = &mut self.gfx;
        let out = match hash {
            // ---- pages
            0x83d81f59 | 0xf94b3586 => {
                let (w, h) = (int(a, 0).max(1) as u32, int(a, 1).max(1) as u32);
                let format = if opt(a, 2, 1) != 0 { PlaneFormat::Bgr24 } else { PlaneFormat::Index8 };
                let id = g.alloc_page(Plane::new(w, h, format)?)?;
                if hash == 0xf94b3586 {
                    let alpha = g.alloc_page(Plane::new(w, h, PlaneFormat::Index8)?)?;
                    let page = g.pages.get_mut(&id).unwrap();
                    page.alpha = Some(alpha);
                    page.owns_alpha = true;
                }
                V(id as i32)
            }
            0xdad96289 | 0x57efa275 => {
                let name = a.first().context("Majiro page_create_file name")?.bytes()?.to_vec();
                V(self.load_page(host, &name, hash == 0x57efa275)? as i32)
            }
            0xc50dfd06 => {
                let id = int(a, 0) as u32;
                if !g.valid(id as i32) {
                    g.warn(format!("page_release of invalid page {id:#x}"));
                } else if !g.release_page(id, false) {
                    g.warn(format!("page_release of page {id:#x} used by a sprite"));
                }
                V(0)
            }
            0x8d84b83f => V(i32::from(g.valid(int(a, 0)))),
            0xf8fd08f6 | 0x65f2e980 => {
                let size = g.page(int(a, 0)).map_or(0, |p| {
                    (if hash == 0xf8fd08f6 { p.plane.width } else { p.plane.height }) as i32
                });
                V(size)
            }
            0x539b07bc | 0xce94e6ca => {
                let name = a.first().context("Majiro pic_len name")?.bytes()?.to_vec();
                let stem = basename(&name)?;
                match self.load_picture(host, &stem) {
                    Ok(p) => V((if hash == 0x539b07bc { p.plane.width } else { p.plane.height }) as i32),
                    Err(_) => V(0),
                }
            }
            0xd3c06820 => {
                let name = a.first().context("Majiro picture existence name")?.bytes()?.to_vec();
                V(i32::from(self.picture_exists(&name)))
            }
            0x5e4e3063 => {
                let id = int(a, 0) as u32;
                if let Some(page) = g.pages.get_mut(&id) {
                    if page.plane.format == PlaneFormat::Bgr24 && page.mode != int(a, 1) {
                        page.mode = int(a, 1);
                        g.invalidate_all(id);
                    }
                }
                V(0)
            }
            0x3e11f2c5 => V(g.primary.map_or(0, |p| p as i32)),
            0xf9705332 => {
                let (id, alpha) = (int(a, 0) as u32, int(a, 1) as u32);
                let ok = match (g.pages.get(&id), g.pages.get(&alpha)) {
                    (Some(p), Some(m)) => {
                        m.plane.format == PlaneFormat::Index8
                            && p.plane.width == m.plane.width
                            && p.plane.height == m.plane.height
                    }
                    _ => false,
                };
                if ok {
                    let page = g.pages.get_mut(&id).unwrap();
                    let old = if page.owns_alpha { page.alpha } else { None };
                    page.alpha = Some(alpha);
                    page.owns_alpha = false;
                    if let Some(old) = old.filter(|old| *old != alpha) {
                        g.pages.remove(&old);
                    }
                    g.invalidate_all(id);
                } else {
                    g.warn("SetAlphaPage requires an equally sized indexed page".into());
                }
                V(0)
            }
            0x70612db5 => {
                let id = int(a, 0) as u32;
                let page = g.sprites.get(&id).map_or(id, |s| s.page);
                V(g.pages.get(&page).and_then(|p| p.alpha).map_or(0, |x| x as i32))
            }
            0x4c310b4b => {
                let page = g.page(int(a, 0));
                V(page
                    .and_then(|p| p.plane.bgr_at(int(a, 1), int(a, 2)))
                    .map_or(-1, |[b, gg, r]| i32::from(r) | i32::from(gg) << 8 | i32::from(b) << 16))
            }
            0x7e277952 => {
                let id = int(a, 0) as u32;
                if let Some(page) = g.pages.get_mut(&id) {
                    let (x, y) = (int(a, 1), int(a, 2));
                    page.plane.fill([x, y, 1, 1], int(a, 3) as u32);
                    g.invalidate(id, [x, y, 1, 1]);
                }
                V(0)
            }
            0xd01be374 | 0xfddf6c40 | 0x61bae53c => {
                let id = int(a, 0) as u32;
                let rect = [int(a, 1), int(a, 2), int(a, 3), int(a, 4)];
                let color = int(a, 5) as u32;
                if let Some(page) = g.pages.get_mut(&id) {
                    match hash {
                        0xd01be374 => page.plane.fill(rect, color),
                        0xfddf6c40 => page.plane.fill_blend(rect, color, (int(a, 6) + 1).clamp(1, 256) as u16),
                        _ => {
                            if page.plane.format == PlaneFormat::Bgr24 {
                                if let Some([x, y, w, h]) = page.plane.clip(rect) {
                                    let stride = page.plane.stride();
                                    let m = [(color >> 16) & 255, (color >> 8) & 255, color & 255];
                                    for row in y..y + h {
                                        let s = row as usize * stride + x as usize * 3;
                                        for px in page.plane.pixels[s..s + w as usize * 3].chunks_exact_mut(3) {
                                            for c in 0..3 {
                                                px[c] = (((m[c] + 1) * u32::from(px[c])) >> 8) as u8;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    g.invalidate(id, rect);
                } else {
                    g.warn(format!("fill of invalid page {id:#x}"));
                }
                V(0)
            }
            0xfd128a61 => {
                let id = int(a, 0) as u32;
                let rect = [int(a, 1), int(a, 2), int(a, 3), int(a, 4)];
                let weight = a.get(7).map(|_| (int(a, 7) + 1).clamp(1, 256) as u16);
                if let Some(page) = g.pages.get_mut(&id) {
                    planes::duotone(&mut page.plane, rect, int(a, 5) as u32, int(a, 6) as u32, weight);
                    g.invalidate(id, rect);
                }
                V(0)
            }
            0x56bbba3a | 0x87113971 | 0x30636d6e => {
                // These leaves unconditionally read src,x,y,w,h,dst. There
                // is no whole-page two-argument overload in422D40/422E80/
                // 423000; reject missing required values instead of reading
                // the native stack's uninitialized preceding records.
                ensure!(a.len()>=6,"Majiro bitmap copy/swap requires src,x,y,w,h,dst");
                for value in a.iter().take(if hash==0x30636d6e{9}else{8}){value.integer().context("Majiro bitmap copy/swap integer argument")?;}
                let (src, dst) = (int(a, 0) as u32, int(a, 5) as u32);
                let rect = [int(a, 1), int(a, 2), int(a, 3), int(a, 4)];
                // YUKIIRO422D40/422E80/423000 default each omitted
                // destination coordinate to its source coordinate. Six/seven
                // argument calls patch a picture in place; zero defaults move
                // route-selection buttons to the origin while their hitboxes
                // correctly remain at the original script coordinates.
                let at = [opt(a, 6, rect[0]), opt(a, 7, rect[1])];
                let t = opt(a, 8, 0).clamp(0, 255) as u8;
                let (Some(s), Some(d)) = (g.pages.get(&src), g.pages.get(&dst)) else {
                    g.warn(format!("bitmap copy with invalid page {src:#x}/{dst:#x}"));
                    return Ok(Some(V(0)));
                };
                let Some((r, at)) = planes::clip_pair(
                    [s.plane.width, s.plane.height],
                    rect,
                    [d.plane.width, d.plane.height],
                    at,
                ) else {
                    return Ok(Some(V(0)));
                };
                match hash {
                    0x56bbba3a => {
                        // 41A630: colour copy, then alpha planes when both own one.
                        let (sa, da) = (s.alpha, d.alpha);
                        g.with_pair(src, dst, |s, d, _, _| planes::copy(s, r, d, at));
                        if let (Some(sa), Some(da)) = (sa, da) {
                            g.with_pair(sa, da, |s, d, _, _| planes::copy(s, r, d, at));
                            g.touch(da);
                        }
                    }
                    0x87113971 => {
                        let (sa, da) = (s.alpha, d.alpha);
                        let mut a_page = g.pages[&src].plane.clone();
                        if src == dst {
                            let mut b = a_page.clone();
                            planes::swap(&mut a_page, r, &mut b, at);
                            // apply the destination-side changes too
                            let page = g.pages.get_mut(&src).unwrap();
                            page.plane = b;
                            planes::copy(&a_page, r, &mut page.plane, [r[0], r[1]]);
                        } else {
                            let mut b = g.pages[&dst].plane.clone();
                            planes::swap(&mut a_page, r, &mut b, at);
                            g.pages.get_mut(&src).unwrap().plane = a_page;
                            g.pages.get_mut(&dst).unwrap().plane = b;
                        }
                        if let (Some(sa), Some(da)) = (sa, da) {
                            if sa != da {
                                let mut x = g.pages[&sa].plane.clone();
                                let mut y = g.pages[&da].plane.clone();
                                planes::swap(&mut x, r, &mut y, at);
                                g.pages.get_mut(&sa).unwrap().plane = x;
                                g.pages.get_mut(&da).unwrap().plane = y;
                            }
                        }
                        g.invalidate(src, r);
                    }
                    _ => {
                        // 41A8C0: transparency copy (masked when the source has alpha).
                        if d.plane.format == PlaneFormat::Index8 && (s.plane.format!=PlaneFormat::Index8 || s.alpha.is_some()) {
                            g.warn("grp_extcopy into an indexed page".into());
                            return Ok(Some(V(0)));
                        }
                        g.with_pair(src, dst, |s, d, sa, _| match sa {
                            Some(mask) => planes::masked_blend(s, mask, r, d, None, at, t),
                            None => planes::blend(s, r, d, at, t),
                        });
                    }
                }
                g.invalidate(dst, [at[0], at[1], r[2], r[3]]);
                V(0)
            }
            // YUKIIRO423150/458480: direct indexed/alpha-plane multiply.
            // Page+16 is its Index8 flag (452D30), not page+28's linked alpha
            // pointer. Callers already pass GetAlphaPage handles explicitly.
            0xa361d9f7 => {
                ensure!((6..=8).contains(&a.len()) && a.iter().all(|v|matches!(v,Value::Integer(_))),"Majiro alpha multiply ABI");
                let(src,dst)=(int(a,0)as u32,int(a,5)as u32);
                let rect=[int(a,1),int(a,2),int(a,3),int(a,4)];
                let at=[opt(a,6,rect[0]),opt(a,7,rect[1])];
                let(Some(s),Some(d))=(g.pages.get(&src),g.pages.get(&dst))else{
                    g.warn("alpha multiply with invalid page".into());return Ok(Some(V(0)));
                };
                if s.plane.format==PlaneFormat::Index8 && d.plane.format==PlaneFormat::Index8{
                    if src==dst{
                        if let Some(page)=g.pages.get_mut(&src){planes::alpha_multiply_within(&mut page.plane,rect,at);}
                    }else{g.with_pair(src,dst,|s,d,_,_|planes::alpha_multiply(s,rect,d,at));}
                    g.invalidate(dst,[at[0],at[1],rect[2],rect[3]]);
                }else{g.warn("alpha multiply requires indexed pages".into());}
                V(0)
            }
            0xda675785 => {
                let (src, dst) = (int(a, 0) as u32, int(a, 5) as u32);
                let sr = [int(a, 1), int(a, 2), int(a, 3), int(a, 4)];
                let dr = [int(a, 6), int(a, 7), int(a, 8), int(a, 9)];
                if !g.pages.contains_key(&src) || !g.pages.contains_key(&dst) {
                    g.warn("grp_mod_copy with invalid page".into());
                    return Ok(Some(V(0)));
                }
                let (sa, da) = (g.pages[&src].alpha, g.pages[&dst].alpha);
                g.with_pair(src, dst, |s, d, _, _| planes::stretch(s, sr, d, dr));
                if let (Some(sa), Some(da)) = (sa, da) {
                    g.with_pair(sa, da, |s, d, _, _| planes::stretch(s, sr, d, dr));
                }
                g.invalidate(dst, dr);
                V(0)
            }
            0x9ac11f47 => {
                let id = int(a, 0) as u32;
                let rect = [int(a, 1), int(a, 2), int(a, 3), int(a, 4)];
                if let Some(page) = g.pages.get_mut(&id) {
                    planes::mosaic(&mut page.plane, rect, int(a, 5));
                    g.invalidate(id, rect);
                }
                V(0)
            }
            0x3cad3884 => {
                let (src, dst) = (int(a, 0) as u32, int(a, 1) as u32);
                let table = match a.get(2) {
                    Some(Value::Array(array)) => {
                        let data = array.lock();
                        data.elements
                            .iter()
                            .take(256)
                            .map(|v| v.integer().unwrap_or(0).clamp(0, 255) as u8)
                            .collect::<Vec<_>>()
                    }
                    _ => vec![],
                };
                let ok = table.len() == 256
                    && g.pages.get(&src).is_some_and(|p| p.plane.format == PlaneFormat::Index8)
                    && g.pages.get(&dst).is_some_and(|p| p.plane.format == PlaneFormat::Index8);
                if ok {
                    let source = g.pages[&src].plane.clone();
                    let page = g.pages.get_mut(&dst).unwrap();
                    let n = page.plane.pixels.len().min(source.pixels.len());
                    for i in 0..n {
                        page.plane.pixels[i] = table[source.pixels[i] as usize];
                    }
                    g.invalidate_all(dst);
                } else {
                    g.warn("make_alfa_table requires indexed pages and a 256-entry table".into());
                }
                V(0)
            }
            0x5eb974f7 => {
                let id = int(a, 0) as u32;
                let name = a.get(1).context("Majiro pic_unpack name")?.bytes()?.to_vec();
                let at = [opt(a, 2, 0), opt(a, 3, 0)];
                if !g.pages.contains_key(&id) {
                    g.warn("pic_unpack into invalid page".into());
                    return Ok(Some(V(0)));
                }
                let stem = basename(&name)?;
                let picture = match self.load_picture(host, &stem) {
                    Ok(p) => p.plane,
                    Err(error) => {
                        self.gfx.warn(format!("pic_unpack {stem:?}: {error:#}"));
                        return Ok(Some(V(0)));
                    }
                };
                let mask_stem = format!("{stem}_");
                let mask = if self.picture_path(&mask_stem).is_some() {
                    self.load_picture(host, &mask_stem).ok().map(|m| m.plane)
                } else {
                    None
                };
                let g = &mut self.gfx;
                let page = g.pages.get_mut(&id).unwrap();
                let Some((r, at)) = planes::clip_pair(
                    [picture.width, picture.height],
                    [0, 0, picture.width as i32, picture.height as i32],
                    [page.plane.width, page.plane.height],
                    at,
                ) else {
                    return Ok(Some(V(0)));
                };
                match &mask {
                    Some(m) if m.format == PlaneFormat::Index8 => {
                        planes::masked_blend(&picture, m, r, &mut page.plane, None, at, 0)
                    }
                    _ => planes::copy(&picture, r, &mut page.plane, at),
                }
                g.invalidate(id, [at[0], at[1], r[2], r[3]]);
                V(0)
            }
            0xdfd5599e => {
                let id = int(a, 0) as u32;
                if let Some(page) = g.pages.get(&id) {
                    let rect = if a.len() >= 5 {
                        [int(a, 1), int(a, 2), int(a, 3), int(a, 4)]
                    } else {
                        [0, 0, page.plane.width as i32, page.plane.height as i32]
                    };
                    let lock = std::mem::take(&mut g.lock);
                    g.invalidate(id, rect);
                    g.lock = lock;
                }
                V(0)
            }
            0x171aee22 => {
                g.viewport = [int(a, 0), int(a, 1)];
                V(0)
            }
            0xcf2b1e50 => {
                g.lock += 1;
                V(0)
            }
            0x6de6cdfc => {
                g.lock -= 1;
                V(0)
            }
            // ---- sprites
            0xef4581dc | 0xf6a05538 => {
                let page = int(a, 0) as u32;
                if !g.pages.contains_key(&page) {
                    g.warn(format!("sprite_create from invalid page {page:#x}"));
                    return Ok(Some(V(0)));
                }
                let p = &g.pages[&page].plane;
                let (mut x, mut y, mut w, mut h) = if a.len() >= 5 {
                    (int(a, 1), int(a, 2), int(a, 3), int(a, 4))
                } else {
                    (0, 0, p.width as i32, p.height as i32)
                };
                if w <= 0 || h <= 0 {
                    w = x.max(1);
                    h = y.max(1);
                    x = 0;
                    y = 0;
                }
                let dest = if a.len() >= 6 {
                    let d = int(a, 5) as u32;
                    if !g.pages.contains_key(&d) {
                        g.warn("sprite_create into invalid destination".into());
                        return Ok(Some(V(0)));
                    }
                    d
                } else {
                    match g.primary {
                        Some(p) => p,
                        None => {
                            g.warn("sprite_create without a primary picture".into());
                            return Ok(Some(V(0)));
                        }
                    }
                };
                let id = g.new_sprite(page, [x, y, w, h], dest, now)?;
                if hash == 0xf6a05538 {
                    g.sprites.get_mut(&id).unwrap().transformed = true;
                }
                V(id as i32)
            }
            0xc2c3c4f4 | 0xcd71fe7a => {
                let name = a.first().context("Majiro sprite_create_file name")?.bytes()?.to_vec();
                let page = self.load_page(host, &name, true)?;
                let g = &mut self.gfx;
                let p = &g.pages[&page].plane;
                let src = if a.len() >= 5 {
                    [int(a, 1), int(a, 2), int(a, 3), int(a, 4)]
                } else {
                    [0, 0, p.width as i32, p.height as i32]
                };
                let dest = if a.len() >= 6 { int(a, 5) as u32 } else { g.primary.unwrap_or(0) };
                if !g.pages.contains_key(&dest) {
                    g.warn("sprite_create_file without a valid destination".into());
                    g.release_page(page, true);
                    return Ok(Some(V(0)));
                }
                let id = g.new_sprite(page, src, dest, now)?;
                let s = g.sprites.get_mut(&id).unwrap();
                s.owns_page = true;
                if hash == 0xcd71fe7a {
                    s.transformed = true;
                    if g.pages[&page].alpha.is_none() {
                        let (w, h) = (g.pages[&page].plane.width, g.pages[&page].plane.height);
                        let alpha = g.alloc_page(Plane::new(w, h, PlaneFormat::Index8)?)?;
                        let p = g.pages.get_mut(&page).unwrap();
                        p.alpha = Some(alpha);
                        p.owns_alpha = true;
                    }
                }
                V(id as i32)
            }
            0x58b31737 => {
                let id = int(a, 0) as u32;
                if g.sprites.contains_key(&id) {
                    g.release_sprite(id);
                } else {
                    g.warn(format!("sprite_release of invalid sprite {id:#x}"));
                }
                V(0)
            }
            0xdcc706ef => V(i32::from(g.sprites.contains_key(&(int(a, 0) as u32)))),
            // YUKIIRO44F9E0/44FA20 return the stored sprite width/height,
            // not the current rotated/zoomed temporary bitmap dimensions.
            0x63f660a2|0xfef981d4=>{
                ensure!(a.len()==1,"Majiro sprite extent ABI");
                let axis=if hash==0x63f660a2{2}else{3};
                let id=int(a,0)as u32;
                if let Some(s)=g.sprites.get(&id){V(s.src[axis])}else{g.warn("sprite extent for invalid sprite".into());V(0)}
            }
            // YUKIIRO44F920 enables per-cell cross-fade scratch (+916),
            // distinct from affine rendering (+940,44DE00). 44C2E0 blends
            // adjacent animation cells through its own timed scratch. Do not
            // silently substitute rotation/zoom state for this separate API.
            0x3f34d519=>{
                ensure!(a.len()==1,"Majiro cell cross-fade enable ABI");
                bail!("Majiro sprite cell cross-fade scratch/50ms update policy is not connected");
            }
            // YUKIIRO44DF60 sets sprite+32: release owns its source page
            // (44DE70), independently of visibility nesting/transform state.
            0x8838fd56=>{
                ensure!(a.len()==1,"Majiro sprite page ownership ABI");
                if let Some(s)=g.sprites.get_mut(&(int(a,0)as u32)){s.owns_page=true;}else{g.warn("page ownership for invalid sprite".into());}
                V(0)
            }
            // YUKIIRO422730 complements the three colour channels only.
            0x053fac99=>{
                ensure!(a.len()==5,"Majiro bitmap invert ABI");
                let id=int(a,0)as u32;let rect=[int(a,1),int(a,2),int(a,3),int(a,4)];
                if let Some(page)=g.pages.get_mut(&id){planes::invert_bgr(&mut page.plane,rect);g.invalidate(id,rect);}else{g.warn("invert for invalid page".into());}
                V(0)
            }
            0x265e07d7 => V(g.sprites.get(&(int(a, 0) as u32)).map_or(0, |s| s.page as i32)),
            0x6b4a7873 | 0xf6459905 => {
                let axis = usize::from(hash == 0xf6459905);
                V(g.sprites.get(&(int(a, 0) as u32)).map_or(0, |s| {
                    if s.move_duration != 0 {
                        s.pos[axis]
                    } else if s.moving != 0 {
                        s.move_from[axis] + s.move_delta[axis]
                    } else {
                        s.pos[axis]
                    }
                }))
            }
            0x68ef2987 => V(i32::from(g.sprites.get(&(int(a, 0) as u32)).is_some_and(|s| s.moving == 1))),
            0xe119d5ba => {
                let id = int(a, 0) as u32;
                let Some(s) = g.sprites.get(&id) else {
                    g.warn(format!("sprite_move of invalid sprite {id:#x}"));
                    return Ok(Some(V(0)));
                };
                let current = if s.moving == 0 || s.move_duration != 0 {
                    s.pos
                } else {
                    [s.move_from[0] + s.move_delta[0], s.move_from[1] + s.move_delta[1]]
                };
                let rect = g.sprite_rect(id);
                let s = g.sprites.get_mut(&id).unwrap();
                s.move_delta = [int(a, 1) - current[0], int(a, 2) - current[1]];
                s.move_duration = opt(a, 3, 0);
                s.moving = 1;
                s.move_start = now;
                s.move_from = current;
                let _ = rect;
                g.update_move(id, now);
                V(0)
            }
            0x40258a56 | 0xd7771b72 => {
                let busy = g.sprites.get(&(int(a, 0) as u32)).is_some_and(|s| {
                    if hash == 0x40258a56 { s.moving == 1 } else { s.alpha.active == 1 }
                });
                if busy {
                    return Ok(Some(GfxOut::Wait));
                }
                V(0)
            }
            0xff7c52e6 => {
                let id = int(a, 0) as u32;
                let alpha = f64::from(int(a, 1));
                let duration = opt(a, 2, 0);
                let Some(s) = g.sprites.get_mut(&id) else {
                    g.warn(format!("sprite_alfa_set of invalid sprite {id:#x}"));
                    return Ok(Some(V(0)));
                };
                if duration != 0 {
                    let previous = f64::from(s.transparency);
                    s.alpha.define(vec![(alpha, duration), (alpha, -1)], previous, now);
                } else {
                    s.alpha.active = 0;
                    s.alpha.entries = vec![(alpha, 0), (alpha, -1)];
                    s.transparency = alpha.clamp(0.0, 255.0) as u8;
                    if let Some((dest, rect)) = g.sprite_rect(id) {
                        g.invalidate(dest, rect);
                    }
                }
                V(0)
            }
            0xf80bc9b6 | 0xc29b30e3 | 0x52819912 | 0xdfdba8e4 => {
                let id = int(a, 0) as u32;
                let Some(s) = g.sprites.get_mut(&id) else {
                    g.warn(format!("sprite sequence for invalid sprite {id:#x}"));
                    return Ok(Some(V(0)));
                };
                let values: Vec<i32> = a[1..].iter().map(|v| v.integer().unwrap_or(0)).collect();
                let mut entries: Vec<(f64, i32)> = values
                    .chunks(2)
                    .map(|c| (f64::from(c[0]), c.get(1).copied().unwrap_or(-1)))
                    .collect();
                entries.truncate(64);
                match hash {
                    0xf80bc9b6 => {
                        let previous = f64::from(s.transparency);
                        if let Some(last) = entries.last().copied() {
                            if last.1 >= 0 {
                                entries.push((last.0, -1));
                                // native: appended hold entry is not counted
                                entries.pop();
                            }
                        }
                        s.alpha.define(entries, previous, now);
                    }
                    0xc29b30e3 => {
                        if entries.is_empty() {
                            s.cells.active = 0;
                            s.cells.entries.clear();
                        } else {
                            let single = entries.len() == 1;
                            s.cells.define(entries, 0.0, now);
                            if single {
                                if let Some((d, r)) = g.sprite_rect(id) {
                                    g.invalidate(d, r);
                                }
                            }
                        }
                    }
                    0x52819912 => {
                        let total: i32 = entries.iter().map(|e| e.1.max(0)).sum::<i32>().max(1);
                        s.cells.define(entries, 0.0, now);
                        s.cells.base = total.wrapping_mul((now as i32) / total) as u32;
                    }
                    _ => {
                        let empty = s.cells.entries.is_empty();
                        s.cells.entries.extend(entries);
                        s.cells.entries.truncate(64);
                        if empty {
                            s.cells.active = 1;
                            s.cells.duration = 0;
                            s.cells.index = -1;
                            s.cells.base = now;
                        }
                    }
                }
                V(0)
            }
            0x69073588 | 0x4a02d664 | 0x7bffc5b1 | 0x5548ef5e => {
                let id = int(a, 0) as u32;
                if !g.sprites.contains_key(&id) {
                    g.warn(format!("sprite priority of invalid sprite {id:#x}"));
                    return Ok(Some(V(0)));
                }
                let high = matches!(hash, 0x4a02d664 | 0x5548ef5e);
                let single = matches!(hash, 0x5548ef5e | 0x7bffc5b1);
                let mut order = g.order.clone();
                order.retain(|x| *x != id);
                let at = if a.len() >= 2 {
                    let reference = int(a, 1) as u32;
                    let Some(target) = g.sprites.get(&reference) else {
                        g.warn("sprite priority reference invalid".into());
                        return Ok(Some(V(0)));
                    };
                    if reference == id {
                        return Ok(Some(V(0)));
                    }
                    let group = target.group;
                    let mut at = order.iter().position(|x| *x == reference).unwrap_or(0);
                    if !single {
                        if high {
                            while at > 0 && g.sprites[&order[at - 1]].group == group {
                                at -= 1;
                            }
                        } else {
                            while at + 1 < order.len() && g.sprites[&order[at + 1]].group == group {
                                at += 1;
                            }
                        }
                    }
                    g.sprites.get_mut(&id).unwrap().group = group;
                    if high { at } else { at + 1 }
                } else if high {
                    0
                } else {
                    order.len()
                };
                order.insert(at.min(order.len()), id);
                g.order = order;
                g.reorder(id);
                V(0)
            }
            0x721fa5b7 | 0x63c341b1 => {
                let push = hash == 0x721fa5b7;
                let ids: Vec<u32> = if a.is_empty() {
                    g.order.clone()
                } else {
                    vec![int(a, 0) as u32]
                };
                for id in ids {
                    let Some(s) = g.sprites.get_mut(&id) else { continue };
                    if push {
                        s.hidden -= 1;
                    } else {
                        s.hidden = (s.hidden + 1).min(0);
                        if s.hidden == 0 {
                            let (page, src) = (s.page, s.src);
                            g.invalidate(page, src);
                        }
                    }
                    if push {
                        if let Some((d, r)) = g.sprite_rect(id) {
                            g.invalidate(d, r);
                        }
                    }
                }
                V(0)
            }
            0x4ab0caef => {
                let (id, dst) = (int(a, 0) as u32, int(a, 1) as u32);
                if !g.sprites.contains_key(&id) || !g.pages.contains_key(&dst) {
                    g.warn("sprite_paste with invalid sprite/page".into());
                    return Ok(Some(V(0)));
                }
                g.update_move(id, now);
                let s = g.sprites[&id].clone();
                if s.hidden == 0 && s.transparency != 255 {
                    if s.transformed {
                        if !s.temp_valid {
                            g.render_transformed(id);
                        }
                        let s = g.sprites[&id].clone();
                        if let (Some(temp), Some(mask)) = (&s.temp, &s.temp_mask) {
                            let d = &g.pages[&dst].plane;
                            if let Some((r, at)) = planes::clip_pair(
                                [temp.width, temp.height],
                                [0, 0, temp.width as i32, temp.height as i32],
                                [d.width, d.height],
                                s.temp_pos,
                            ) {
                                let page = g.pages.get_mut(&dst).unwrap();
                                planes::masked_blend(temp, mask, r, &mut page.plane, None, at, s.transparency);
                                g.invalidate(dst, [at[0], at[1], r[2], r[3]]);
                            }
                        }
                    } else if let (Some(sp), Some(dp)) = (g.pages.get(&s.page), g.pages.get(&dst)) {
                        if let Some(src) = sp.plane.clip(s.src) {
                            let at = [s.pos[0] + src[0] - s.src[0], s.pos[1] + src[1] - s.src[1]];
                            if let Some((r, at)) = planes::clip_pair(
                                [sp.plane.width, sp.plane.height],
                                src,
                                [dp.plane.width, dp.plane.height],
                                at,
                            ) {
                                g.draw(s.page, r, dst, at, s.transparency);
                                g.invalidate(dst, [at[0], at[1], r[2], r[3]]);
                            }
                        }
                    }
                }
                V(0)
            }
            0x661afb43 => {
                let id = int(a, 0) as u32;
                let angle = float(a, 1);
                let duration = opt(a, 2, 0);
                let looping = opt(a, 3, 0) != 0;
                let Some(s) = g.sprites.get_mut(&id) else {
                    return Ok(Some(V(0)));
                };
                s.transformed = true;
                s.angle_from = s.angle;
                s.angle_to = angle;
                s.angle_start = now;
                s.angle_duration = duration;
                s.angle_mode = if looping { 2 } else { 1 };
                if duration == 0 {
                    s.angle_mode = 0;
                    if s.angle != angle {
                        s.temp_valid = false;
                    }
                    s.angle = angle;
                    s.angle_from = angle;
                }
                V(0)
            }
            0x01b4517c | 0x6f384a3d => {
                let id = int(a, 0) as u32;
                let Some(s) = g.sprites.get_mut(&id) else { return Ok(Some(V(0))) };
                let values: Vec<f64> = a[1..]
                    .iter()
                    .map(|v| match v {
                        Value::Float(f) => f64::from(*f),
                        Value::Integer(n) => f64::from(*n),
                        _ => 0.0,
                    })
                    .collect();
                let mut entries: Vec<(f64, i32)> = values
                    .chunks(2)
                    .map(|c| (c[0], c.get(1).map_or(-1, |d| *d as i32)))
                    .collect();
                entries.truncate(64);
                s.transformed = true;
                let previous = f64::from(if hash == 0x01b4517c { s.zoom_x } else { s.zoom_y });
                let track = if hash == 0x01b4517c { &mut s.zoom_x_track } else { &mut s.zoom_y_track };
                track.define(entries, previous, now);
                V(0)
            }
            0x3b1d5537 | 0xba383010 => {
                let id = int(a, 0) as u32;
                let value = float(a, 1);
                let duration = opt(a, 2, 0);
                let Some(s) = g.sprites.get_mut(&id) else { return Ok(Some(V(0))) };
                s.transformed = true;
                let x = hash == 0x3b1d5537;
                let current = if x { s.zoom_x } else { s.zoom_y };
                let track = if x { &mut s.zoom_x_track } else { &mut s.zoom_y_track };
                if duration != 0 {
                    track.define(vec![(f64::from(value), duration), (f64::from(value), -1)], f64::from(current), now);
                } else {
                    track.active = 0;
                    if current != value {
                        s.temp_valid = false;
                    }
                    if x { s.zoom_x = value } else { s.zoom_y = value }
                }
                V(0)
            }
            _ => return Ok(None),
        };
        Ok(Some(out))
    }
}
