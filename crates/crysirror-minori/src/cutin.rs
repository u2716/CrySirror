// SPDX-License-Identifier: MPL-2.0
use super::{ClientId, HostServices, Sprite};
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
use std::{
    collections::{BTreeMap},
    sync::Arc,
};

pub(super) struct Cutin {
    pub art: Sprite,
    pub sound: Option<String>,
    pub retiring: bool,
    direction: i32,
    hold: i32,
    speed: f64,
    factor: f64,
    phase: Phase,
    previous: Option<u64>,
    fade: bool,
}
#[derive(PartialEq)]
enum Phase {
    Enter,
    Hold,
    Exit,
    Done,
}
pub(super) enum Draw {
    None,
    Move,
    Arrived,
    Finished,
}

impl Cutin {
    pub fn new(mut art: Sprite, canvas: [i32; 2], name: &str, hold: i32, rate: i32) -> Self {
        let direction = if rate >= 0 { 1 } else { -1 };
        art.rect[0] = if direction > 0 {
            -art.rect[2]
        } else {
            canvas[0]
        };
        art.rect[1] = ((canvas[1] - art.rect[3]) / 2).max(0);
        let factor = f64::from(rate).abs() / 100.;
        let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
        Self {
            art,
            sound: Some(format!("{stem}.wav")),
            retiring: false,
            direction,
            hold,
            speed: 2. * f64::from(direction),
            factor: if factor <= 1. { 1.5 } else { factor },
            phase: Phase::Enter,
            previous: None,
            fade: false,
        }
    }
    pub fn fade(&mut self) {
        self.fade = true;
    }
    pub fn step(&mut self, now: u64) -> Draw {
        let tick = now / 10;
        let delta = self.previous.map_or(1, |t| tick.saturating_sub(t));
        if self.previous.is_some() && delta < 4 {
            return Draw::None;
        }
        self.previous = Some(tick);
        match self.phase {
            Phase::Enter | Phase::Exit => {
                let x = self.art.rect[0].saturating_add(self.speed as i32);
                self.speed *= self.factor;
                let limit = if self.phase == Phase::Enter {
                    0
                } else {
                    self.direction * self.art.rect[2]
                };
                if (self.direction > 0 && x >= limit) || (self.direction < 0 && x <= limit) {
                    self.art.rect[0] = limit;
                    if self.phase == Phase::Enter {
                        self.phase = Phase::Hold;
                        return Draw::Arrived;
                    }
                    self.phase = Phase::Done;
                } else {
                    self.art.rect[0] = x;
                }
                Draw::Move
            }
            Phase::Hold => {
                if self.hold >= 0 {
                    self.hold = self
                        .hold
                        .saturating_sub(i32::try_from(delta).unwrap_or(i32::MAX))
                        .max(0);
                }
                if self.fade || self.hold == 0 {
                    self.speed = 2. * f64::from(self.direction);
                    self.phase = Phase::Exit;
                }
                Draw::None
            }
            Phase::Done => Draw::Finished,
        }
    }
}

/// The standalone catalogue retains decoded images between load and present.
#[derive(Default)]
pub(super) struct Catalogue {
    entries: BTreeMap<String, Arc<dyn ImageResource>>,
    displayed: Option<(String, String)>,
    pending: Option<(String, bool, ImageJob)>,
}

impl Catalogue {
    const LAYER: u32 = 502;

    pub fn loading(&self) -> bool {
        self.pending.is_some()
    }

    fn display(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        layers: &mut super::Published,
        name: Option<&str>,
    ) -> anyhow::Result<()> {
        let new = match name {
            Some(name) => {
                let image = self.entries[name].clone();
                let [width, height] = image.size();
                let rect = [0, 0, width.try_into()?, height.try_into()?];
                let art = Sprite {
                    asset: h
                        .publish_image_resource(c, Self::LAYER, image)?
                        .path
                        .relative()
                        .to_owned(),
                    rect,
                    mask_color: None,
                };
                let mut op = art.layer(Self::LAYER, 255);
                op["layer"]["order"] = serde_json::json!([98, 0]);
                if let Err(error) = super::submit(layers, h, c, vec![op]) {
                    h.release_image_resource(c, &art.asset)?;
                    return Err(error);
                }
                Some((name.to_owned(), art.asset))
            }
            None => {
                super::submit(layers, h, c, vec![super::remove(Self::LAYER)])?;
                None
            }
        };
        if let Some((_, old)) = std::mem::replace(&mut self.displayed, new) {
            h.release_image_resource(c, &old)?;
        }
        Ok(())
    }

    pub fn apply(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        layers: &mut super::Published,
        command: crate::vm::CutinCommand,
    ) -> anyhow::Result<()> {
        use crate::vm::CutinCommand;
        match command {
            CutinCommand::Present(name) if name.is_empty() => self.display(h, c, layers, None)?,
            CutinCommand::Present(name) if self.entries.contains_key(&name) => {
                self.display(h, c, layers, Some(&name))?;
            }
            CutinCommand::ReleaseAll => {
                self.display(h, c, layers, None)?;
                self.entries.clear();
            }
            CutinCommand::Release(name) => {
                if self
                    .displayed
                    .as_ref()
                    .is_some_and(|(shown, _)| shown == &name)
                {
                    self.display(h, c, layers, None)?;
                }
                self.entries.remove(&name);
            }
            action => {
                let show = matches!(action, CutinCommand::Present(_));
                let name = match action {
                    CutinCommand::Load(name) | CutinCommand::Present(name) => name,
                    _ => unreachable!(),
                };
                if name.is_empty() {
                    return Ok(());
                }
                let path = super::asset("bg", &name)?;
                if !h.storage(c)?.exists(&path) {
                    return Ok(());
                }
                let size = h.image_size(c, path.relative())?;
                let job = h.image_metadata(c)?.begin_process(
                    ImageInput::ScopedAsset {
                        path,
                        size,
                        fingerprint: None,
                    },
                    Arc::from(b"[]".as_slice()),
                )?;
                self.pending = Some((name, show, job));
            }
        }
        Ok(())
    }

    pub fn poll(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        layers: &mut super::Published,
    ) -> anyhow::Result<bool> {
        let Some((_, _, job)) = &mut self.pending else {
            return Ok(true);
        };
        let image = match job.poll()? {
            ImageProcessState::Pending => return Ok(false),
            ImageProcessState::Failed(_) => {
                // Native load failure keeps the previous entry and selection.
                self.pending = None;
                return Ok(true);
            }
            ImageProcessState::Ready(image) => image,
        };
        let (name, show, _) = self.pending.take().unwrap();
        if self
            .displayed
            .as_ref()
            .is_some_and(|(shown, _)| shown == &name)
        {
            self.display(h, c, layers, None)?;
        }
        self.entries.insert(name.clone(), image);
        if show {
            self.display(h, c, layers, Some(&name))?;
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_rate_enters_from_the_right_holds_and_fades_off_the_left_edge() {
        let art = Sprite {
            asset: "bg/sample.png".into(),
            mask_color: None,
            rect: [0, 0, 8, 4],
        };
        let mut c = Cutin::new(art, [8, 8], "sample.png", -1, -10);
        assert_eq!(c.art.rect, [8, 2, 8, 4]);
        assert_eq!(c.sound.as_deref(), Some("sample.wav"));
        assert!(matches!(c.step(0), Draw::Move));
        assert_eq!(c.art.rect[0], 6);
        assert!(matches!(c.step(39), Draw::None));
        assert!(matches!(c.step(40), Draw::Move));
        assert!(matches!(c.step(80), Draw::Arrived));
        assert_eq!(c.art.rect[0], 0);
        assert!(matches!(c.step(2000), Draw::None));
        c.fade();
        assert!(matches!(c.step(2040), Draw::None));
        for now in [2080, 2120, 2160] {
            assert!(matches!(c.step(now), Draw::Move));
        }
        assert_eq!(c.art.rect[0], -8);
        assert!(matches!(c.step(2200), Draw::Finished));
    }
}
