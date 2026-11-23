// SPDX-License-Identifier: MPL-2.0
use crate::{firefly::Fireflies, poll};
use anyhow::{Context, Result};
use crysirror_core::service::ClientStorage;
use crysirror_image::{ImageSurface, packed_stamps, temporal_pattern::PatternRng};
use std::sync::{
    Arc,
    mpsc::{self, Receiver, SyncSender},
};

struct Flake {
    fixed: [i32; 2],
    velocity: [i32; 2],
    sprite: usize,
    active: bool,
}

impl Flake {
    fn spawn(rng: &mut PatternRng, horizontal: bool, edge: bool, canvas: [i32; 2]) -> Result<Self> {
        let mut draw = || rng.next_draw().map(|v| v as i32);
        let x = if horizontal && edge {
            0
        } else {
            draw()? % (canvas[0] + 100)
        };
        let y = if !horizontal && edge {
            0
        } else {
            draw()? % (canvas[1] + 60)
        };
        let drift = draw()? % 8192;
        let drift = if draw()? > 16383 { drift } else { -drift };
        let speed = 2 * draw()? + 16384;
        Ok(Self {
            fixed: [x << 16, y << 16],
            velocity: if horizontal {
                [speed, drift]
            } else {
                [drift, speed]
            },
            sprite: usize::from(speed >= 0x5000) + usize::from(speed >= 0xa000),
            active: true,
        })
    }
}

enum RenderJob {
    Uniform(Vec<(usize, [i32; 2])>, u16),
    Weighted(Vec<(usize, [i32; 2], u8)>),
    Mapped(Vec<(usize,[i32;2])>),
    Snow2(Vec<(usize, [i32;2], u8)>,bool,bool),
    Lines(Vec<crysirror_image::integer_lines::IntegerLine>),
}
enum Field {
    Snow {
        mapped:bool,
        flakes: Vec<Flake>,
        horizontal: bool,
    },
    Firefly(Fireflies),
    Rain(crate::rain::Rain),
    Snow2(crate::snow2::Field),
}
pub(super) struct Weather {
    canvas: [i32; 2],
    field: Field,
    previous: Option<u64>,
    elapsed: u32,
    alpha: i32,
    fading: bool,
    pub retiring: bool,
    jobs: SyncSender<RenderJob>,
    frames: Receiver<Result<ImageSurface>>,
    pending: bool,
    pub published: bool,
}

impl Weather {
    pub fn snow(
        storage: Arc<dyn ClientStorage>,
        rng: &mut PatternRng,
        canvas: [i32; 2],
        horizontal: bool,
        mapped:bool,
    ) -> Result<Self> {
        let mut flakes = (0..50)
            .map(|_| Flake::spawn(rng, false, false, canvas))
            .collect::<Result<Vec<_>>>()?;
        if horizontal {
            for flake in &mut flakes {
                *flake = Flake::spawn(rng, true, false, canvas)?;
            }
        }
        Self::worker(storage, canvas, "snow", Field::Snow { flakes, horizontal,mapped })
    }
    pub fn snow2(storage:Arc<dyn ClientStorage>,rng:&mut PatternRng,canvas:[i32;2],r:&crate::vm::EffectRequest)->Result<Self>{
        let field=crate::snow2::Field::new(rng,canvas,&r.kind,r.first,r.second)?;
        Self::worker(storage,canvas,&r.resource,Field::Snow2(field))
    }
    pub fn change_population(&mut self,count:i32)->Result<()>{
        if let Field::Snow2(field)=&mut self.field {field.retarget(count)?;}
        Ok(())
    }
    pub fn rain(storage: Arc<dyn ClientStorage>,rng:&mut PatternRng,canvas:[i32;2],horizontal:bool)->Result<Self>{
        Self::worker(storage,canvas,"",Field::Rain(crate::rain::Rain::new(rng,canvas,horizontal)?))
    }
    pub fn firefly(
        storage: Arc<dyn ClientStorage>,
        rng: &mut PatternRng,
        canvas: [i32; 2],
        stem: &str,
        count: i32,
        life: i32,
    ) -> Result<Self> {
        let field = Fireflies::new(rng, canvas, stem, count, life)?;
        Self::worker(storage, canvas, stem, Field::Firefly(field))
    }
    pub fn retarget(&mut self, stem: &str, count: i32, life: i32) -> Result<bool> {
        match &mut self.field {
            Field::Firefly(f) if !self.retiring && !self.fading => f.retarget(stem, count, life),
            _ => Ok(false),
        }
    }
    fn worker(
        storage: Arc<dyn ClientStorage>,
        canvas: [i32; 2],
        stem: &str,
        field: Field,
    ) -> Result<Self> {
        let mapped=matches!(field,Field::Snow{mapped:true,..});
        let paths = (!matches!(field,Field::Rain(_))).then(|| ["s", "m", "l"]
            .map(|size| crate::presentation::asset("sys", &format!("{stem}{size}.png"))));
        let (jobs, rx) = mpsc::sync_channel::<RenderJob>(1);
        let (tx, frames) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("minori-particles".into())
            .spawn(move || {
                let run = || -> Result<()> {
                    let mut images = Vec::new();
                    for path in paths.into_iter().flatten() {
                        let path = path?;
                        if !storage.exists(&path) {
                            images.push(ImageSurface::filled(1,1,[0;4])?);
                            continue;
                        }
                        let reader = storage.open_asset(&path)?;
                        let bytes = crysirror_resource::read_all(reader.as_ref(), 4 << 20)?;
                        images.push(crysirror_image::decode_standard(&bytes).with_context(
                            || format!("minori particle image {}", path.relative()),
                        )?);
                    }
                    let overlay=if mapped {
                        let path=crate::presentation::asset("sys","snowOverlay.png")?;
                        if storage.exists(&path){
                            let reader=storage.open_asset(&path)?;
                            Some(crysirror_image::decode_standard(&crysirror_resource::read_all(reader.as_ref(),64<<20)?)?)
                        }else{None}
                    }else{None};
                    let mut previous: Option<ImageSurface>=None;
                    for job in rx {
                        let size = canvas.map(|n| n as u32);
                        let frame = match job {
                            RenderJob::Uniform(stamps, alpha) => {
                                packed_stamps::maximum_argb(size, &images, &stamps, alpha)?
                            }
                            RenderJob::Mapped(stamps)=>if let Some(map)=&overlay {
                                packed_stamps::mapped_overlay(size,map,&images,&stamps)?
                            }else{ImageSurface::filled(size[0],size[1],[0;4])?},
                            RenderJob::Snow2(mut stamps,reverse,trails)=>{
                                if reverse {
                                    for (index,pos,_) in &mut stamps {
                                        pos[1]=canvas[1]-images[*index].height as i32-pos[1];
                                    }
                                }
                                let mut frame=packed_stamps::maximum_argb_weighted(size,&images,&stamps)?;
                                if trails {
                                    if let Some(prev)=&previous {packed_stamps::retain_maximum(&mut frame,prev,48)?;}
                                    previous=Some(frame.clone());
                                }
                                frame
                            }
                            RenderJob::Lines(lines) => ImageSurface::filled(size[0],size[1],[0;4])?.integer_lines(&lines,&||Ok(()))?,
                            RenderJob::Weighted(stamps) => {
                                packed_stamps::maximum_argb_weighted(size, &images, &stamps)?
                            }
                        };
                        if tx.send(Ok(frame)).is_err() {
                            break;
                        }
                    }
                    Ok(())
                };
                if let Err(e) = run() {
                    let _ = tx.send(Err(e));
                }
            })?;
        Ok(Self {
            canvas,
            field,
            previous: None,
            elapsed: 0,
            alpha: 0,
            fading: false,
            retiring: false,
            jobs,
            frames,
            pending: false,
            published: false,
        })
    }
    pub fn fade(&mut self) {
        if !self.fading {
            self.fading = true;
            self.elapsed = 0;
        }
    }
    pub fn reusable(&self) -> bool { !self.fading && !self.retiring }
    pub fn matches(&self, horizontal: bool) -> bool {
        matches!(self.field,Field::Snow{horizontal:h,..} if h==horizontal)
            && !self.retiring
            && !self.fading
    }
    pub fn frame(&mut self) -> Result<Option<ImageSurface>> {
        if !self.pending {
            return Ok(None);
        }
        let frame = poll(&self.frames)?;
        if frame.is_some() {
            self.pending = false;
        }
        Ok(frame)
    }
    /// One draw at the native effect cadence. Skipped time advances the fixed
    /// positions in one update, so RNG consumption does not depend on polling.
    pub fn advance(&mut self, now: u64, rng: &mut PatternRng, snow_interval: u32) -> Result<bool> {
        if self.pending {
            return Ok(true);
        }
        let tick = now / 10;
        let dt = self.previous.map_or(1, |t| {
            tick.saturating_sub(t).min(u64::from(u32::MAX)) as u32
        });
        let interval = if matches!(self.field, Field::Snow { mapped: false, .. }) { snow_interval } else { 10 };
        if self.previous.is_some() && (dt == 0 || dt < interval) {
            return Ok(true);
        }
        let dt = if matches!(self.field, Field::Firefly(_) | Field::Snow2(_)) && dt > 20 {
            10
        } else {
            dt
        };
        self.previous = Some(tick);
        let step=if matches!(&self.field, Field::Rain(r) if !r.horizontal) {1} else {dt};
        self.elapsed = self.elapsed.wrapping_add(step);
        let shift=if matches!(self.field,Field::Rain(_)){2}else{4};
        let change = (self.elapsed >> shift) as i32;
        self.alpha = if self.fading {
            self.alpha.saturating_sub(change)
        } else {
            self.alpha.saturating_add(change).min(256)
        };
        if self.fading && self.alpha <= 0 {
            return Ok(false);
        }
        let job = match &mut self.field {
            Field::Snow2(field)=>RenderJob::Snow2(field.advance(rng,self.canvas,dt as i32,self.elapsed,self.alpha)?,field.reverse,field.trails),
            Field::Rain(r)=>RenderJob::Lines(r.advance(rng,self.canvas,dt,self.alpha)?),
            Field::Firefly(f) => {
                RenderJob::Weighted(f.advance(rng, self.canvas, dt as i32, self.alpha)?)
            }
            Field::Snow { flakes, horizontal,mapped } => {
                let mut stamps = Vec::with_capacity(flakes.len());
                for f in flakes {
                    if !f.active {
                        *f = Flake::spawn(rng, *horizontal, true, self.canvas)?;
                        continue;
                    }
                    for (p, v) in f.fixed.iter_mut().zip(f.velocity) {
                        *p = p.wrapping_add(v.wrapping_mul(dt as i32));
                    }
                    let p = [f.fixed[0] / 65536 - 50, f.fixed[1] / 65536 - 30];
                    let axis = usize::from(!*horizontal);
                    f.active = p[axis] < self.canvas[axis];
                    if f.active {
                        stamps.push((f.sprite, p));
                    }
                }
                if *mapped {RenderJob::Mapped(stamps)}else{RenderJob::Uniform(stamps, self.alpha as u16)}
            }
        };
        self.jobs
            .try_send(job)
            .context("minori snow render worker")?;
        self.pending = true;
        Ok(true)
    }
}
