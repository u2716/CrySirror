// SPDX-License-Identifier: MPL-2.0
//! Native interval/fade/auto policy from DialogueTextAnimation.cs and
//! DialoguePage.cs, NitroSharp 77a93db4. Shared clocks apply retained opacity.
use super::*;
use crysirror_scene::{animation::StaggeredFade, layer::Paint};

pub(super) struct TextFade {
    pub clock: StaggeredFade,
    pub characters: usize,
    pub auto_since: Option<u64>,
    paint: Paint,
    initial: usize,
    last_ms: u64,
    published_complete: bool,
}
impl N2SystemVm {
    pub(super) fn text_number(&self, name: &str, flag: bool, fallback: f32) -> Result<f32> {
        let map = if flag {
            &self.machine.flags
        } else {
            &self.machine.variables
        };
        let number = map
            .get(name)
            .map(Value::number)
            .transpose()?
            .unwrap_or(fallback);
        ensure!(
            number.is_finite() && number.abs() <= 86_400_000.,
            "N2System text setting {name}"
        );
        Ok(number.max(0.))
    }
    pub(super) fn text_switch(&self, name: &str, fallback: bool) -> Result<bool> {
        Ok(self
            .machine
            .variables
            .get(name)
            .map(Value::truth)
            .transpose()?
            .unwrap_or(fallback))
    }
    pub(super) fn begin_text_fade(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        paint: Paint,
        positions: Vec<f32>,
        initial: usize,
        instant: bool,
    ) -> Result<()> {
        let interval = self.text_number("SYSTEM_text_interval", false, 48.)?;
        let speed = self.text_number("SYSTEM_text_speed", true, 200.)?;
        ensure!(initial <= positions.len(), "N2System reveal prefix");
        let start = positions[initial..]
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        let positions = positions[initial..]
            .iter()
            .map(|p| p - start)
            .collect::<Vec<_>>();
        let characters = positions.iter().copied().fold(0., f32::max).ceil() as usize
            + usize::from(!positions.is_empty());
        let mut clock = StaggeredFade::new(positions, interval, speed)?;
        if instant || speed <= 0. || self.text_switch("SYSTEM_skip", false)? {
            clock.finish();
        }
        self.text_fades.insert(
            id,
            TextFade {
                clock,
                characters,
                auto_since: None,
                paint,
                initial,
                last_ms: host.now_ms(),
                published_complete: false,
            },
        );
        self.sample_text_fades(host)
    }
    pub(super) fn finish_text_fade(&mut self, host: &mut dyn HostServices, id: u32) -> Result<()> {
        if let Some(fade) = self.text_fades.get_mut(&id) {
            fade.clock.finish();
            fade.published_complete = false;
        }
        self.sample_text_fades(host)
    }
    pub(super) fn sample_text_fades(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms();
        let mut changes = Vec::new();
        for (&id, fade) in &mut self.text_fades {
            if fade.published_complete {
                continue;
            }
            fade.clock
                .advance(now.saturating_sub(fade.last_ms) as f32)?;
            fade.last_ms = now;
            let mut paint = fade.paint.clone();
            let Paint::GlyphRun { glyphs, color, .. } = &mut paint else {
                bail!("N2System fade requires glyph paint");
            };
            fade.clock.apply(glyphs, fade.initial, *color)?;
            changes.push(json!({"op":"set_paint","id":id,"paint":paint}));
            fade.published_complete = fade.clock.complete();
        }
        self.submit(host, changes, vec![])
    }
}
