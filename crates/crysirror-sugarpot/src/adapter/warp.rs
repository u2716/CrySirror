// SPDX-License-Identifier: MPL-2.0
//!448530 EF29 /4383B0 case63. Only the native counters and draw recipe live
//!here; immutable common Composite/RectMap workers own image processing.
use super::*;
const OVERLAY: u32 = 65003;
pub(super) struct Warp {
    target: i32,
    amplitude: [i32; 2],
    frames: i32,
    timeout: i32,
    start: u64,
    plan: Option<stage::StagePlan>,
    image: Option<UploadedImage>,
}
impl Warp {
    pub(super) fn preparing(&self) -> bool {
        self.plan.is_some()
    }
    pub(super) fn overlay_path(&self) -> Option<String> {
        self.image
            .as_ref()
            .map(|image| image.path.relative().to_owned())
    }
}
fn displacement(amplitude: [i32; 2], index: i32, frames: i32) -> [i32; 2] {
    //4344C0 converts each integer input to float, performs x87 double
    //arithmetic, then stores the decayed amplitude back to float before ftol.
    //A negative retained scratch index has an undefined jump-table default;
    //portable policy starts that uninitialized branch at its first sample.
    let index = index.max(0);
    let values = amplitude.map(|value| {
        let value = f64::from(value as f32);
        (value - value / f64::from(frames as f32) * f64::from(index as f32)) as f32 as i32
    });
    let [x, y] = values;
    let alternate = amplitude[1] > amplitude[0];
    match (index % 4, alternate) {
        (0, _) => [x.wrapping_neg(), y.wrapping_neg()],
        (1, false) | (2, true) => [x, y.wrapping_neg()],
        (2, false) | (1, true) => [x.wrapping_neg(), y],
        _ => [x, y],
    }
}
impl SugarPotVm {
    pub(super) fn begin_warp(&mut self, h: &mut dyn HostServices, args: &[String]) -> Result<bool> {
        //43F580 dispatches the active raster/fall/filter after its last
        //non-SYS sprite, before the remaining native sprite sequence. This
        //recipe cannot substitute a finished scene capture at that boundary.
        ensure!(
            !self.raster.as_ref().is_some_and(|effect| effect.state != 0)
                && !self.fall.as_ref().is_some_and(|effect| effect.state != 0)
                && !self.filter.as_ref().is_some_and(|effect| effect.state != 0),
            "SugarPot EF29 active postprocessing draw boundary not connected"
        );
        let target = self.number(args, 0, 0)?;
        ensure!(
            target == -1 || (target >= 0 && (target as usize) < self.sprites.len()),
            "SugarPot EF29 sprite ID"
        );
        let amplitude = [self.number(args, 2, 0)?, self.number(args, 3, 0)?];
        let frames = self.number(args, 4, 0)? / 33;
        let timeout = self.number(args, 5, 0)?;
        self.native_effect_index = self.native_effect_index.max(0);
        if let Some(window) = self.windows.get_mut(self.current_window) {
            window.visibility = 1;
        }
        self.publish(h, None)?;
        self.warp = Some(Warp {
            target,
            amplitude,
            frames,
            timeout,
            start: h.now_ms(),
            plan: None,
            image: None,
        });
        self.wait = Some(Wait::Warp);
        Ok(true)
    }
    pub(super) fn warp_frame(&mut self, h: &mut dyn HostServices, skip: bool) -> Result<bool> {
        let mut warp = self
            .warp
            .take()
            .context("SugarPot EF29 wait without controller")?;
        let result = self.poll_warp(h, &mut warp, skip);
        if !matches!(result, Ok(true)) {
            self.warp = Some(warp);
        }
        result
    }
    fn poll_warp(&mut self, h: &mut dyn HostServices, warp: &mut Warp, skip: bool) -> Result<bool> {
        //Worker completion belongs to the admitted native sample. Input/time
        //changes cannot turn an unfinished dependency into a successful frame.
        if let Some(plan) = &mut warp.plan {
            if !plan.poll(h, self.client)? {
                return Ok(false);
            }
            let resource = plan
                .ready_resource()
                .context("SugarPot EF29 ready resource missing")?;
            let image = h.publish_image_resource(self.client, self.next_image, resource)?;
            self.next_image = self
                .next_image
                .checked_add(1)
                .context("SugarPot EF29 image ID")?;
            self.submit(h, vec![json!({"op":"put","layer":{"id":OVERLAY,"rect":[0,0,1024,600],"order":[2999,0],"sampling":{"linear":false},"paint":{"kind":"image","asset":image.path.relative()}}})], vec![])?;
            if !self.published_layers.contains(&OVERLAY) {
                self.published_layers.push(OVERLAY);
            }
            if let Some(old) = warp.image.replace(image) {
                h.release_image_resource(self.client, old.path.relative())?;
            }
            warp.plan = None;
            self.native_effect_index = self.native_effect_index.max(0).saturating_add(1);
            //43B3F0 redraws the retained62CC message after each EF frame.
            //Its existing GlyphRun layers remain above the temporary stage.
            if !self.native_text.committed.is_empty() {
                let text = self.native_text.committed.clone();
                ensure!(
                    self.text_inner(h, &text, false)?,
                    "SugarPot EF29 retained redraw dependency not prepared"
                );
                self.wait = Some(Wait::Warp);
            }
            return Ok(false);
        }
        if h.now_ms() < self.native_effect_next {
            return Ok(false);
        }
        self.native_effect_next = h.now_ms().saturating_add(33);
        let elapsed = h.now_ms().wrapping_sub(warp.start) as u32 as i32;
        if skip
            || self.settings.header[4] != 0
            || elapsed > warp.timeout
            || self.native_effect_index >= warp.frames
        {
            self.native_effect_index = 0;
            self.submit(h, vec![json!({"op":"remove","id":OVERLAY})], vec![])?;
            self.published_layers.retain(|id| *id != OVERLAY);
            self.publish(h, None)?;
            if let Some(image) = warp.image.take() {
                h.release_image_resource(self.client, image.path.relative())?;
            }
            return Ok(true);
        }
        //The completion predicate excludes zero/negative frames before division.
        let offset = displacement(warp.amplitude, self.native_effect_index, warp.frames);
        warp.plan = Some(self.plan_stage_warp(h, warp.target, offset)?);
        Ok(false)
    }
}
