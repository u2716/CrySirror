//! DC3WY ending credits: 408F00 selector 9 -> 42E1B0 + 42EDE0(a3), driven
//! by 4078C0 -> 42E4D0 once per process frame until it reports completion.
//!
//! 42E1B0: private objects (outside the 120 primary objects) plus "bg\\black"
//! (alpha 0) and "bg\\white" (alpha 1) overlays. 42EDE0: total time T =
//! 112000 ms (97000 when var(5,10) > 4); heroine packs ED\\ed???01.PCK (event
//! images) and ed???02.PCK (variants) by var(5,10); "ed\\edback" at (-1,-1);
//! period P = (T - 6550 | 5800) / frames; ED\\ed.PCK staff strips scroll
//! 576*strips/T px per ms on two objects at x = 740 - w/2 (source from x 20).
//! 42E4D0 per frame (times are wall ms since 42EDE0):
//!  * (e+1000)/P changes with no white transition pending -> white -> 1 over
//!    40 ticks; 300 ms after an image switch finished -> white -> 0 over 60.
//!  * at 70% of a period the variant of the current image fades in (30).
//!  * e/P changes -> the next image goes to whichever of objects 0/1 is not
//!    fully opaque, with the PANS row: position, scale 100, then a move to
//!    (end x, end y) or a zoom to end x % over P/1000*60+120 ticks (on the
//!    image and its variant).
//!  * strips: object 9 at -(y%576), object 10 at 576-(y%576); after the last
//!    strip wait for a click or 10 s, then black alpha +0.005 per frame to 1.
//! Not connected: the right-click skip (var(100,110)), which needs the
//! right-button edge in the step input.
use super::*;
use crate::machine::packed_ops::PackedRequest;
use crate::machine::sprite_ops::{SpriteCommand, SpriteRequest, SpriteResourceBinding, SpriteResourceRequest, linear};
use crysirror_scene::layer::queued::{DeltaSource, SequenceOperation};

///42E4D0 pan/zoom tables by heroine var(5,10): (x, y, end x | zoom %, end y, zoom).
///DC3WY.exe 0x49D8B0, 0x49D990, 0x49DA48, 0x49DB00, 0x49DB90, 0x49DC20,
///0x49DCE8, 0x49DDA0, 0x49DE68 (each ends at an x of -1).
pub(super) const PANS: [&[[i32; 5]]; 9] = [
    &[[-446, -55, -446, 140, 0], [-153, -55, 53, 112, 0], [-480, 120, -340, 120, 0], [-200, 40, 85, 40, 1], [-400, 120, -600, 120, 0], [-180, 120, 24, 120, 0], [-486, -55, -486, 140, 0], [-386, 40, -560, 140, 0], [-400, 80, -540, 80, 0], [0, 0, 0, 0, 1]],
    &[[-200, 0, 53, 140, 0], [-386, 40, -560, 140, 0], [-400, 64, -480, 148, 0], [-188, -40, -60, 20, 0], [-100, 0, 53, 100, 0], [-200, 120, -400, 120, 0], [-200, 0, 53, 140, 0], [-320, 64, -400, 128, 0]],
    &[[-200, 0, 73, 64, 0], [-160, 0, 53, 140, 0], [40, 120, 140, 120, 1], [-160, 53, -280, 53, 0], [-160, 64, 53, 128, 0], [-360, 140, -520, 140, 0], [-80, 120, 140, 0, 1], [-160, 0, 13, 120, 0]],
    &[[53, 40, -60, 40, 0], [-200, 80, 0, 120, 0], [-386, 40, -520, 120, 0], [-386, -40, -520, -148, 0], [53, 0, -100, 0, 0], [-400, 0, -300, 0, 0]],
    &[[53, 80, -140, 80, 0], [-200, 40, -20, 100, 0], [-386, 40, -460, 100, 0], [-386, 40, -520, 80, 0], [-200, 0, -40, 120, 0], [-200, 80, 85, 80, 1]],
    &[[-200, 40, -120, 120, 0], [-250, 80, -450, 80, 0], [-200, 40, -80, 80, 0], [-200, 40, -20, 70, 0], [-386, 40, -520, 120, 0], [-400, 10, -200, 20, 0], [-480, 40, -300, 40, 0], [-200, 0, 40, 120, 0], [-200, -20, -340, -20, 0]],
    &[[-240, 0, -380, 80, 0], [-340, 0, -440, 80, 0], [-200, 20, -140, 110, 0], [-340, 0, -440, 100, 0], [-240, 0, -380, 120, 0], [-300, 40, -100, 40, 0], [-320, 0, -500, 140, 0], [-200, 40, 75, 40, 1]],
    &[[53, 0, -100, 0, 0], [-150, -80, 0, 180, 0], [-200, -80, 40, 180, 0], [-280, 0, 50, 0, 1], [-340, 0, -440, 140, 0], [0, 0, -260, 0, 0], [-280, 0, -480, 140, 0], [-320, 0, -480, 140, 0], [-200, -80, -20, 140, 0]],
    &[[-300, -80, -120, 180, 0], [-100, 80, 53, 80, 0], [-200, 60, -460, 60, 0], [-400, 40, -560, 120, 0], [-200, -80, 53, 180, 0], [-100, 40, 53, 120, 0], [-180, 80, 80, 110, 1], [-260, -80, 0, 150, 0], [-360, 40, -520, 120, 0], [-100, 40, 23, 110, 0], [-252, 0, 65, 0, 1]],
];

const IMAGE: [u16; 2] = [120, 121];
const VARIANT: u16 = 122;
const BACK: u16 = 124;
const STRIP: [u16; 2] = [129, 130];
const BLACK: u16 = 137;
///407990 draws objects 0..16 with the white overlay right after object 3
///and black last; nothing else (the scene is not rendered, 418150).
const WHITE: u16 = 123;
///Backdrop standing in for the cleared back buffer under the credit objects.
const BACKDROP: u16 = 119;
const OBJECTS: [u16; 9] = [119, 120, 121, 122, 123, 124, 129, 130, 137];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credits {
    pub setup: u8,
    pub heroine: i32,
    pub total: i64,
    pub period: f32,
    pub frames: u32,
    pub strips: u32,
    pub start: u64,
    pub text_start: u64,
    pub speed: f32,
    ///this+724: current image object (0/1), 99 waiting, 100 fading, -1 done.
    pub state: i32,
    pub white: u8,
    pub settle: u64,
    pub switching: bool,
    pub image: i32,
    pub variant: i32,
    pub flash: i32,
    pub strip: i32,
    pub wait: u64,
    pub black: f32,
    #[serde(default)]
    pub strip_step: u8,
    ///The one in-flight frame decode (slot, object, frame), and the last
    ///completed one, so a retried step does not start a second job.
    #[serde(default)]
    pub frame_pending: Option<(u8, u16, u32)>,
    #[serde(default)]
    pub frame_done: Option<(u8, u16, u32)>,
}

impl Machine {
    fn credit_ops(&mut self, host: &mut dyn NativeHost, object: u16, operations: Vec<SequenceOperation>) -> Result<bool> {
        let request = SpriteRequest { commands: vec![SpriteCommand { seed: self.object_seed(object), operations }] };
        if !host.sprite(&request)? {
            return Ok(false);
        }
        self.state.sprites.entry(object).or_default().actor = true;
        Ok(true)
    }
    fn credit_alpha(&mut self, host: &mut dyn NativeHost, object: u16, alpha: f32) -> Result<bool> {
        self.credit_ops(host, object, vec![SequenceOperation::Set { channels: vec![2], values: vec![alpha], retained: true }])
    }
    ///43B470: alpha linear over `ticks` (ignored while that lane is busy).
    fn credit_fade(&mut self, host: &mut dyn NativeHost, object: u16, alpha: f32, ticks: u32) -> Result<bool> {
        let track = linear(vec![2], vec![alpha], ticks, vec![2], vec![2], DeltaSource::Retained, false, true);
        self.credit_ops(host, object, vec![SequenceOperation::Enqueue { lane: 3, bank: None, track }])
    }
    fn credit_place(&mut self, host: &mut dyn NativeHost, object: u16, position: [f32; 2]) -> Result<bool> {
        self.credit_ops(host, object, vec![SequenceOperation::Set { channels: vec![0, 1], values: position.to_vec(), retained: true }])
    }
    fn credit_image(&mut self, host: &mut dyn NativeHost, slot: u8, object: u16, name: &str, crop: Option<[i32; 4]>) -> Result<bool> {
        if self.state.image_names.get(&slot).map(String::as_str) != Some(name) {
            if !host.emit(&Event::LoadImage { slot, name: name.to_owned() })? {
                return Ok(false);
            }
            self.state.image_names.insert(slot, name.to_owned());
        }
        Ok(host.sprite_resources(&SpriteResourceRequest { bindings: vec![SpriteResourceBinding { slot, object, crop }] })?.is_some())
    }
    fn credit_frame(&mut self, host: &mut dyn NativeHost, slot: u8, object: u16, frame: u32) -> Result<bool> {
        let request = (slot, object, frame);
        let mut credits = self.state.credits.clone().context("DC3WY credits owner")?;
        if credits.frame_done == Some(request) {
            credits.frame_done = None;
            self.state.credits = Some(credits);
            return Ok(true);
        }
        if credits.frame_pending.is_some_and(|pending| pending != request) {
            return Ok(false);
        }
        let done = host.packed(&PackedRequest::Frame { slot, object, frame })?;
        credits.frame_pending = (!done).then_some(request);
        self.state.credits = Some(credits);
        Ok(done)
    }
    ///Finish the in-flight frame before any other step of this frame.
    fn credit_drain(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        let Some((slot, object, frame)) = self.state.credits.as_ref().and_then(|credits| credits.frame_pending) else { return Ok(true) };
        if !host.packed(&PackedRequest::Frame { slot, object, frame })? {
            if crate::trace_enabled() {
                eprintln!("DC3 credits drain blocked on frame {slot}/{object}/{frame}");
            }
            return Ok(false);
        }
        let credits = self.state.credits.as_mut().unwrap();
        credits.frame_pending = None;
        credits.frame_done = Some((slot, object, frame));
        Ok(true)
    }
    ///Write the poll's working copy back, keeping the frame-decode owner
    ///fields that credit_frame/credit_drain maintain on the stored state.
    fn store_credits(&mut self, mut c: Credits) {
        if let Some(stored) = &self.state.credits {
            c.frame_pending = stored.frame_pending;
            c.frame_done = stored.frame_done;
        }
        self.state.credits = Some(c);
    }
    fn credit_values(&self, host: &mut dyn NativeHost, object: u16) -> Result<Option<Vec<f32>>> {
        Ok(host.sprite_states()?.into_iter().find(|(candidate, _)| *candidate == object).map(|(_, status)| status.values))
    }

    ///408F00 selector 9.
    pub(super) fn begin_credits(&mut self, now: u64) -> Result<()> {
        let heroine = self.value(5, 10)?;
        ensure!((0..=8).contains(&heroine), "DC3WY credit heroine {heroine}");
        let total = if heroine > 4 { 97000 } else { 112000 };
        self.state.credits = Some(Credits {
            setup: 0,
            heroine,
            total,
            period: 0.,
            frames: 0,
            strips: 0,
            start: now,
            text_start: now,
            speed: 0.,
            state: 0,
            white: 0,
            settle: 0,
            switching: false,
            image: -1,
            variant: -1,
            flash: -1,
            strip: 0,
            wait: 0,
            black: 0.,
            strip_step: 0,
            frame_pending: None,
            frame_done: None,
        });
        Ok(())
    }

    fn credits_setup(&mut self, host: &mut dyn NativeHost, now: u64) -> Result<bool> {
        let mut credits = self.state.credits.clone().context("DC3WY credits owner")?;
        let packs = ["rcc", "srr", "hmn", "sra", "aoi", "ski", "mkt", "smm", "htn"];
        let pack = packs[credits.heroine as usize];
        while credits.setup < 13 {
            let done = match credits.setup {
                12 => self.credit_image(host, 41, BACKDROP, "BG\\BLACK", None)? && self.credit_alpha(host, BACKDROP, 1.)?,
                0 => self.credit_image(host, 41, BLACK, "BG\\BLACK", Some([0, 0, 1024, 576]))?,
                1 => self.credit_alpha(host, BLACK, 0.)?,
                2 => self.credit_image(host, 42, WHITE, "BG\\WHITE", Some([0, 0, 1024, 576]))?,
                3 => self.credit_alpha(host, WHITE, 1.)?,
                4 => self.credit_image(host, 40, BACK, "ED\\EDBACK", None)? && self.credit_place(host, BACK, [-1., -1.])?,
                5 => host.packed(&PackedRequest::Load { slot: 0, name: format!("ED\\ed{pack}01.PCK") })?,
                6 => host.packed(&PackedRequest::Load { slot: 1, name: format!("ED\\ed{pack}02.PCK") })?,
                7 => host.packed(&PackedRequest::Load { slot: 2, name: "ED\\ed.PCK".into() })?,
                8 => {
                    let (Some(frames), Some(strips)) = (host.packed_frame_count(0)?, host.packed_frame_count(2)?) else { return Ok(false) };
                    ensure!(frames > 0 && strips > 0, "DC3WY credit packs are empty");
                    credits.frames = frames;
                    credits.strips = strips;
                    let lead = if credits.heroine > 4 { 5800 } else { 6550 };
                    credits.period = (credits.total - lead) as f32 / frames as f32;
                    credits.speed = (576 * i64::from(strips)) as f32 / credits.total as f32;
                    true
                }
                //Object 1 shows image 0 and fades in over 30 ticks.
                9 => self.credit_frame(host, 0, IMAGE[1], 0)?
                    && self.credit_alpha(host, IMAGE[1], 0.)?
                    && self.credit_fade(host, IMAGE[1], 1., 30)?,
                10 => self.credit_frame(host, 2, STRIP[0], 0)? && self.credit_strip_x(host, STRIP[0], 0.)?,
                11 => self.credit_frame(host, 2, STRIP[1], 1)? && self.credit_strip_x(host, STRIP[1], 576.)?,
                _ => unreachable!(),
            };
            if crate::trace_enabled() {
                eprintln!("DC3 credits setup {} done={done}", credits.setup);
            }
            if !done {
                self.state.credits = Some(credits);
                return Ok(false);
            }
            credits.setup += 1;
            self.state.credits = Some(credits.clone());
        }
        credits.start = now;
        credits.text_start = now;
        credits.setup = 14;
        self.state.credits = Some(credits);
        Ok(true)
    }

    fn credit_strip_x(&mut self, host: &mut dyn NativeHost, object: u16, y: f32) -> Result<bool> {
        let Some(([_, _], [width, _])) = host.packed_dimensions(object)? else { return Ok(false) };
        self.credit_place(host, object, [740. - (width / 2) as f32 - 20., y])
    }

    ///4078C0/42E4D0 for one process frame; Some(outcome) while running.
    pub(super) fn poll_credits(&mut self, host: &mut dyn NativeHost, now: u64, pressed: bool) -> Result<Option<Outcome>> {
        let Some(credits) = self.state.credits.clone() else { return Ok(None) };
        if crate::trace_enabled() {
            eprintln!("DC3 credits poll setup={} state={} image={}", credits.setup, credits.state, credits.image);
        }
        if credits.setup < 14 {
            if !self.credits_setup(host, now)? {
                return Ok(Some(Outcome::Pending));
            }
            return Ok(Some(Outcome::WaitUntil(now.saturating_add(16))));
        }
        if !self.credit_drain(host)? {
            return Ok(Some(Outcome::Pending));
        }
        let mut c = self.state.credits.clone().context("DC3WY credits owner")?;
        let next = Outcome::WaitUntil(now.saturating_add(16));
        match c.state {
            -1 => return self.end_credits(host).map(|done| (!done).then_some(Outcome::Pending)),
            99 => {
                if pressed || now.saturating_sub(c.wait) > 10000 {
                    c.state = 100;
                }
                self.store_credits(c);
                return Ok(Some(next));
            }
            100 => {
                c.black += 0.005;
                if c.black >= 1. {
                    c.black = 1.;
                    c.state = -1;
                }
                let black = c.black;
                self.store_credits(c);
                if !self.credit_alpha(host, BLACK, black)? {
                    return Ok(Some(Outcome::Pending));
                }
                return Ok(Some(next));
            }
            _ => {}
        }
        let current = IMAGE[c.state as usize];
        let current_alpha = self.credit_values(host, current)?.map_or(0., |values| values[2]);
        match c.white {
            1 => {
                if !self.credit_fade(host, WHITE, 0., 60)? {
                    return Ok(Some(Outcome::Pending));
                }
                if current_alpha >= 1. {
                    c.white = 0;
                }
            }
            2 => {
                if !self.credit_fade(host, WHITE, 1., 40)? {
                    return Ok(Some(Outcome::Pending));
                }
                c.white = 0;
            }
            _ => {}
        }
        if c.settle != 0 && now.saturating_sub(c.settle) > 300 {
            if c.white == 0 {
                c.white = 1;
            }
            c.settle = 0;
        }
        let elapsed = now.saturating_sub(c.start) as f64;
        if c.switching {
            if current_alpha >= 1. {
                c.switching = false;
                c.settle = now;
            }
        } else {
            let flash = ((elapsed + 1000.) / f64::from(c.period)) as i32;
            if flash != c.flash {
                c.flash = flash;
                if c.white == 0 && flash < c.frames as i32 {
                    c.white = 2;
                }
            }
            if f64::from(c.period) * 7. / 10. < elapsed - f64::from(c.image) * f64::from(c.period) && c.image != c.variant {
                let ok = self.credit_frame(host, 1, VARIANT, c.image.max(0) as u32)?
                    && self.credit_alpha(host, VARIANT, 0.)?
                    && self.credit_fade(host, VARIANT, 1., 30)?;
                if !ok {
                    self.store_credits(c);
                    return Ok(Some(Outcome::Pending));
                }
                c.variant = c.image;
            }
            let index = ((elapsed / f64::from(c.period)) as i32).min(c.frames as i32);
            if index != c.image {
                let target = if self.credit_values(host, IMAGE[0])?.map_or(0., |values| values[2]) == 1. { 1 } else { 0 };
                c.state = target as i32;
                if index < c.frames as i32 {
                    let object = IMAGE[target];
                    let other = IMAGE[1 - target];
                    let row = PANS[c.heroine as usize].get(index as usize).copied().unwrap_or([0; 5]);
                    let ticks = (c.period / 1000. * 60. + 120.) as u32;
                    let mut ok = self.credit_frame(host, 0, object, index as u32)?
                        && self.credit_alpha(host, VARIANT, 0.)?
                        && self.credit_alpha(host, object, 1.)?
                        && self.credit_alpha(host, other, 0.)?;
                    for animated in [object, VARIANT] {
                        ok = ok
                            && self.credit_ops(host, animated, vec![
                                SequenceOperation::Finish { cancel: true, lanes: Some(vec![0, 4]) },
                                SequenceOperation::Set { channels: vec![0, 1], values: vec![row[0] as f32, row[1] as f32], retained: true },
                                SequenceOperation::Set { channels: vec![3, 4], values: vec![100., 100.], retained: true },
                                if row[4] != 0 {
                                    SequenceOperation::Enqueue { lane: 4, bank: None, track: linear(vec![3, 4], vec![row[2] as f32, row[2] as f32], ticks, vec![3, 4], vec![3, 4], DeltaSource::Retained, false, true) }
                                } else {
                                    SequenceOperation::Enqueue { lane: 0, bank: None, track: linear(vec![0, 1], vec![row[2] as f32, row[3] as f32], ticks, vec![0, 1], vec![0, 1], DeltaSource::Retained, false, true) }
                                },
                            ])?;
                    }
                    if !ok {
                        self.store_credits(c);
                        return Ok(Some(Outcome::Pending));
                    }
                    c.switching = true;
                }
                c.image = index;
            }
        }
        if crate::trace_enabled() {
            eprintln!("DC3 credits state={} image={} white={} switching={} alpha={current_alpha} strip={} e={elapsed}", c.state, c.image, c.white, c.switching, c.strip);
        }
        //Staff strips.
        let y = (now.saturating_sub(c.text_start) as f64 * f64::from(c.speed)) as i64;
        let strip = (y / 576) as i32;
        let offset = (y % 576) as f32;
        if strip != c.strip {
            if strip >= c.strips as i32 - 1 {
                c.state = 99;
                c.wait = now;
            } else {
                //One shared frame decode at a time: object 9, then object 10.
                while c.strip_step < 2 {
                    let index = c.strip_step as usize;
                    if !self.credit_frame(host, 2, STRIP[index], strip as u32 + index as u32)? {
                        self.store_credits(c);
                        return Ok(Some(Outcome::Pending));
                    }
                    c.strip_step += 1;
                }
            }
            c.strip_step = 0;
            c.strip = strip;
        }
        if c.state != 99 && !(self.credit_strip_x(host, STRIP[0], -offset)? && self.credit_strip_x(host, STRIP[1], 576. - offset)?) {
            self.store_credits(c);
            return Ok(Some(Outcome::Pending));
        }
        self.store_credits(c);
        Ok(Some(next))
    }

    ///42E400: release the private objects, textures and packs.
    fn end_credits(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        for slot in [40u8, 41, 42] {
            if self.state.image_names.contains_key(&slot) {
                if !self.publish_texture(host, slot, "-")? {
                    return Ok(false);
                }
            }
        }
        for object in OBJECTS {
            if self.state.sprites.get(&object).is_some_and(|sprite| sprite.actor)
                && !self.credit_ops(host, object, vec![SequenceOperation::Set { channels: vec![2], values: vec![0.], retained: true }])?
            {
                if crate::trace_enabled() {
                    eprintln!("DC3 credits end blocked on object {object}");
                }
                return Ok(false);
            }
            self.state.sprites.remove(&object);
        }
        self.state.credits = None;
        Ok(true)
    }
}
