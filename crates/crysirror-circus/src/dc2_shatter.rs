//! DC2 41C530 case13 selector14 -> 4188A0 "scrbrk" screen break. Native
//! policy only: 418040's per-type tile setup and rand() order, the 40ms
//! frame loop and 419B60(1..5) cleanup. Motion is the shared integer
//! `crysirror_image::tile_scatter`; frames are shared image composition.
use super::*;
use crate::dc2_bitmap::{Owner, Request};
use crate::machine::Event;
use crate::machine::resource_ops::{ResourceReply, ResourceRequest};
use crysirror_image::tile_scatter::TileScatter;

/// Engine image slot for the 800x32 `system\eff` crack strip (402FE0 surface).
pub(crate) const OVERLAY_SLOT: u8 = 48;
const COLUMNS: u32 = 25;
const ROWS: u32 = 19;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Shatter {
    pc: u32,
    owner: Owner,
    kind: i16,
    baked: bool,
    captured: bool,
    overlay: bool,
    scatter: Option<TileScatter>,
    due: u64,
    pending: Option<(bool, Vec<([i32; 2], i32)>, Vec<([i32; 2], [i32; 2], u8)>)>,
    finished: bool,
    released: u8,
}

/// 418040: shared base loop, then the selector-specific delays/velocities.
fn initialize(kind: i16, rand: &mut dyn FnMut() -> Result<i32>) -> Result<TileScatter> {
    let mut s = TileScatter::new(COLUMNS, ROWS, 32, [800, 600])?;
    s.acceleration = 1024;
    s.life_step = 1;
    s.time_step = 8192 / 100;
    for t in &mut s.tiles {
        let mut vx = 32 - (rand()? & 0x1f);
        let vy = (rand()? & 0x1f) + 32;
        if rand()? & 1 == 0 {
            vx = (rand()? & 0x1f) - 32;
        }
        t.life = rand()? % 32 + 16;
        t.delay = 0;
        t.velocity = [vx << 10, vy << 10];
        t.drift = 0;
        t.time = 0;
        t.alpha = 16;
    }
    match kind {
        4 => {
            s.acceleration = 0;
            for t in &mut s.tiles {
                t.delay = 0;
                rand()?;
                if rand()? & 1 == 0 {
                    t.velocity[0] = ((rand()? & 0x1f) - 32) << 10;
                }
                t.velocity[1] = (32 - (rand()? & 0x30)) << 10;
            }
        }
        0 => {
            s.acceleration = 0;
            for t in &mut s.tiles {
                t.delay = 0;
                let vx = 48 - (rand()? & 0x1f);
                let vy = 48 - (rand()? & 0x1f);
                t.velocity = [vx << 10, vy << 10];
            }
        }
        //Row-major tiles (25 per row, 19 rows); rand() order as in 418040.
        1 => {
            for (index, t) in s.tiles.iter_mut().enumerate() {
                let (row, column) = ((index / 25) as i32, (index % 25) as i32);
                t.delay = rand()? % 2 - row / 5 + column - 25;
                let vx = 32 - (rand()? & 0x1f);
                let vy = 32 - (rand()? & 0x30);
                t.velocity = [vx << 10, vy << 10];
            }
        }
        2 => {
            s.acceleration = 0;
            for (index, t) in s.tiles.iter_mut().enumerate() {
                t.delay = rand()? % 2 + (index / 25) as i32 - 19;
                let vx = 4 - (rand()? & 8);
                let vy = (rand()? & 0x1f) + 32;
                t.velocity = [vx << 10, vy << 10];
            }
        }
        3 => {
            s.acceleration = -1024;
            for (index, t) in s.tiles.iter_mut().enumerate() {
                let (row, column) = ((index / 25) as i32, (index % 25) as i32);
                t.delay = rand()? % 2 - row / 5 - column;
                let vx = (rand()? & 0x1f) - 32;
                let vy = 32 - (rand()? & 0x30);
                t.velocity = [vx << 10, vy << 10];
            }
        }
        5 => {
            s.acceleration = 0;
            for (index, t) in s.tiles.iter_mut().enumerate() {
                let (row, column) = ((index / 25) as i32, (index % 25) as i32);
                t.delay = -column - 2 * row;
                let vx = 2 - (rand()? & 8);
                let vy = (rand()? & 8) - 2;
                t.velocity = [vx << 10, vy << 10];
            }
        }
        6 => {
            s.acceleration = 0;
            for (index, t) in s.tiles.iter_mut().enumerate() {
                let (n, m) = (9 - (index / 25) as i32, 12 - (index % 25) as i32);
                t.delay = -(n.abs() + m.abs());
                let vx = 4 - (rand()? & 8);
                let vy = 4 - (rand()? & 8);
                t.velocity = [vx << 10, vy << 10];
            }
        }
        9 => {
            s.acceleration = 0;
            s.life_step = 4;
            for t in &mut s.tiles {
                t.delay = 0;
                let vx = rand()? % 16 - 8;
                let vy = -32 - rand()? % 32;
                t.velocity = [vx << 10, vy << 10];
            }
        }
        10 => {
            for t in &mut s.tiles {
                t.delay = -(rand()? % 20);
            }
        }
        //418040's switch has no other case: the shared base loop stands.
        _ => {}
    }
    Ok(s)
}

impl Machine {
    pub(super) fn execute_shatter(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if !(instruction.opcode == 0x0d && a[0] == 14) {
            return Ok(None);
        }
        if self.state.shatter.is_none() {
            //52B3CC (preference prefix20): disabled effects use process13.
            ensure!(
                self.state.settings.prefix(20)?.unwrap_or(0) != 0,
                "DC2 screen-break disabled-effect process13 path is not connected"
            );
            self.state.shatter = Some(Shatter {
                pc: instruction.offset,
                owner: self.state.resources.capture_owner.clone().unwrap_or_else(|| Owner {
                    scene: self.state.scene.clone(),
                    pc: instruction.offset,
                }),
                kind: a[1],
                baked: !self.state.draw.native_composition_pending,
                captured: false,
                overlay: false,
                scatter: None,
                due: now,
                pending: None,
                finished: false,
                released: 0,
            });
        }
        let mut s = self.state.shatter.clone().unwrap();
        ensure!(s.pc == instruction.offset && s.owner.scene == self.state.scene, "DC2 screen-break owner");
        let pending = |m: &mut Self, s: Shatter| {
            m.state.shatter = Some(s);
            Ok(Some(Outcome::Pending))
        };
        if !s.baked {
            //41ADE0 composes pending display records into native0 first.
            let Some((_, draw)) = self.default_composition(host, false)? else { return pending(self, s) };
            if !host.bitmap_stage(&Request::Bake { owner: s.owner.clone(), slot: 1, draw })? {
                return pending(self, s);
            }
            self.state.draw.planes[0].clear();
            self.state.resources.display_base.clear();
            self.state.resources.native0_composed = true;
            s.baked = true;
        }
        if !s.captured {
            //403CB0("scrbrk") copies the presented screen.
            if self.state.resources.capture_owner.is_none() {
                if !host.bitmap_stage(&Request::Capture(s.owner.clone()))? {
                    return pending(self, s);
                }
                self.state.resources.capture_owner = Some(s.owner.clone());
            }
            s.captured = true;
        }
        if !s.overlay {
            if !host.emit(&Event::LoadImage { slot: OVERLAY_SLOT, name: "system\\eff".into() })? {
                return pending(self, s);
            }
            s.overlay = true;
        }
        if s.scatter.is_none() {
            let mut result = None;
            let kind = s.kind;
            if !host.random_batch(&mut |rand| {
                result = Some(initialize(kind, rand)?);
                Ok(())
            })? {
                return pending(self, s);
            }
            s.scatter = result;
            s.due = now;
        }
        while !s.finished {
            if s.pending.is_none() {
                if now < s.due {
                    self.state.shatter = Some(s.clone());
                    return Ok(Some(Outcome::WaitUntil(s.due)));
                }
                let scatter = s.scatter.as_mut().unwrap();
                let done = scatter.step();
                let width = scatter.tile;
                let mut cracks = Vec::new();
                let mut tiles = Vec::new();
                for (index, t) in scatter.tiles.iter().enumerate() {
                    if t.life <= 0 {
                        continue;
                    }
                    let column = (index as u32 % COLUMNS) as i32;
                    let row = (index as u32 / COLUMNS) as i32;
                    let origin = [column * width, row * width];
                    if t.delay >= 0 && t.delay * width < 800 {
                        cracks.push((origin, t.delay * width));
                    }
                    //418570 draws tiles whose life is positive after the step.
                    let alpha = t.alpha.clamp(0, 16) as u8;
                    if alpha > 0 {
                        tiles.push((origin, scatter.pixel(t), alpha));
                    }
                }
                s.pending = Some((done, cracks, tiles));
                //4188A0 measures each40ms frame from its own start.
                s.due = now + 40;
            }
            let (done, cracks, tiles) = s.pending.clone().unwrap();
            let fill = self.state.palette.get(&0).copied().unwrap_or([0; 3]);
            if !host.bitmap_stage(&Request::Shatter {
                owner: s.owner.clone(),
                overlay_slot: OVERLAY_SLOT,
                fill: [fill[1], fill[0], fill[2]],
                cracks,
                tiles,
                finalize: done,
            })? {
                return pending(self, s);
            }
            s.pending = None;
            s.finished = done;
            if !done {
                self.state.shatter = Some(s.clone());
                return Ok(Some(Outcome::WaitUntil(s.due)));
            }
        }
        //418870 frees scrbrk/eff; 419B60(1..5) releases native1..5.
        self.state.resources.capture_owner = None;
        while s.released < 6 {
            let slot = if s.released == 5 { OVERLAY_SLOT } else { portrait::host_slot(s.released + 1) };
            match host.resource(&ResourceRequest::Unload { slot })? {
                None => return pending(self, s),
                Some(ResourceReply::Done) => {}
                Some(_) => bail!("DC2 screen-break release foreign receipt"),
            }
            s.released += 1;
        }
        for index in 1..self.state.primary_names.len() {
            self.state.primary_names[index] = None;
            self.state.primary_ids[index] = -1;
        }
        self.state.native5_name = None;
        self.state.draw.native_composition_pending = false;
        self.state.shatter = None;
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
}
