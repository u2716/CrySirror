// GPL-2.0-or-later. Original GLEffectCopyB600 raw FPU/integer flow and
// SACT B3F0/E540/E570/E5B0 plus secondary interface getters read statically.
//! Native shake parameters/sample policy; the shared engine copies pixels.
use super::*;
use crysirror_vfx::falling_atlas::SequenceRng;
#[derive(Clone, Copy)]
pub(super) struct Quake {
    mode: i32,
    rotation: i32,
    x: i32,
    y: i32,
    cycles: i32,
}
impl Default for Quake {
    fn default() -> Self {
        Self {
            mode: 0,
            rotation: 128,
            x: 128,
            y: 128,
            cycles: 3,
        }
    }
}
impl Quake {
    pub(super) fn cross(x: i32, y: i32) -> Self {
        Self {
            mode: 0,
            rotation: 0,
            x,
            y,
            cycles: 0,
        }
    }
    pub(super) fn set_cross(&mut self, x: i32, y: i32) {
        self.mode = 0;
        self.x = x;
        self.y = y;
    }
    pub(super) fn set_rotation(&mut self, amplitude: i32, cycles: i32) {
        self.mode = 1;
        self.rotation = amplitude;
        self.cycles = cycles;
    }
    pub(super) fn validate(self) -> Result<()> {
        ensure!(
            [self.x, self.y, self.rotation]
                .iter()
                .all(|v| (-16384..=16384).contains(v)),
            "native Quake amplitude exceeds bounded logical coordinate profile"
        );
        Ok(())
    }
    pub(super) fn sample(
        self,
        elapsed: u32,
        duration: u32,
        old: Arc<dyn ImageResource>,
        new: Arc<dyn ImageResource>,
        rng: &mut SequenceRng,
    ) -> Result<ImageInput> {
        self.validate()?;
        ensure!(
            duration > 0 && elapsed < duration,
            "native active Quake clock"
        );
        //The native FPU stores duration and then (1-t/duration) as binary32
        //before evaluating either branch. No alternate simulation clock.
        let duration32 = duration as f32;
        let rate = (1.0 - f64::from(elapsed) / f64::from(duration32)) as f32;
        let [x, y] = if self.mode == 0 {
            let mut axis = |amplitude: i32| -> Result<i32> {
                if amplitude == 0 {
                    return Ok(0);
                }
                let value = rng.next_u32()? as i32 % amplitude - amplitude / 2;
                Ok((f64::from(value) * f64::from(rate)).trunc() as i32)
            };
            [axis(self.x)?, axis(self.y)?]
        } else if self.mode == 1 {
            //Native IMUL keeps the low32 bits, followed by unsigned-to-FPU
            //conversion. Preserve that wrapping phase; do not reinterpret it
            //as a signed angular velocity when cycles is negative.
            let phase = elapsed.wrapping_mul(self.cycles as u32).wrapping_mul(360);
            let angle = -(f64::from(phase) / f64::from(duration32)) * 0.0174532925199433;
            let amplitude = f64::from(self.rotation) * f64::from(rate);
            //Portable binary64 trig is a declared outline-motion profile;
            //original x87 transcendental bit identity is not asserted.
            [
                (angle.cos() * amplitude).trunc() as i32,
                (angle.sin() * amplitude).trunc() as i32,
            ]
        } else {
            [0, 0]
        };
        let size = old.size();
        ensure!(new.size() == size, "native Quake frame extent");
        //B600 first restores old, then copies the clipped offset new image.
        //QuakeScreen supplies a black old image; Effect27 supplies oldWindow.
        let sx = (-x).max(0);
        let sy = (-y).max(0);
        let dx = x.max(0);
        let dy = y.max(0);
        let w = (size[0] as i32 - sx).min(size[0] as i32 - dx);
        let h = (size[1] as i32 - sy).min(size[1] as i32 - dy);
        let base = ImageInput::Resource(old);
        Ok(if w > 0 && h > 0 {
            layer(
                base,
                ImageInput::Resource(new),
                [sx, sy, w, h],
                [dx, dy],
                ImageCompositeMath::CopyRgb,
            )
        } else {
            base
        })
    }
}
