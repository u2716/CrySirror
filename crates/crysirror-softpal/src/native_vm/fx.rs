//! PAL type-0 raster FX, recovered from the fixture DLL at 0x100B3E30,
//! 0x100B3FE0 and 0x100B4180. Time values use the wrapping PAL millisecond clock.

pub use crate::native_vm::abi::PalRasterSource32 as RasterSource;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RasterUniforms {
    pub offset: f32,
    pub undulate_x: f32,
    pub undulate_y: f32,
}

impl RasterUniforms {
    /// RASTER ps_2_0: the phase is independent of the amplitudes. Evaluate the
    /// original range-reduced cosine polynomial rather than the different
    /// RASTER_SW HLSL expression.
    pub fn sample_uv(self, uv: [f32; 2]) -> [f32; 2] {
        [
            uv[0] + raster_wave(uv[1], self.offset) * self.undulate_x * 0.1,
            uv[1] + raster_wave(uv[0], self.offset) * self.undulate_y * 0.1,
        ]
    }

    pub fn vertex(self, position: [f32; 2]) -> [f32; 2] {
        [
            position[0] + raster_wave(position[1] * 0.5, self.offset) * self.undulate_x * 0.1,
            position[1] + raster_wave(position[0] * 0.5, self.offset) * self.undulate_y * 0.1,
        ]
    }

    /// Active fragment expression in FX.dll RASTER_SW. The apparent vertex
    /// distortion in that resource is commented out and must not be executed.
    pub fn sample_uv_sw(self, uv: [f32; 2]) -> [f32; 2] {
        fn displacement(coordinate: f32, amount: f32, phase: f32) -> f32 {
            if amount == 0.0 {
                return 0.0;
            }
            // At exactly 1 the native expression divides by zero. Keep that
            // undefined input deterministic and finite on all portable hosts.
            let denominator = 180.0 - 180.0 * amount;
            if denominator == 0.0 {
                return 0.0;
            }
            let angle =
                (coordinate * 180.0 + 360.0 * (1.0 - amount) * phase) * 3.141592 / denominator;
            angle.sin() * (0.1 * amount)
        }
        [
            uv[0] + displacement(uv[1], self.undulate_x, self.offset),
            uv[1] + displacement(uv[0], self.undulate_y, self.offset),
        ]
    }
}

fn raster_wave(coordinate: f32, phase: f32) -> f32 {
    let cycles = (coordinate * 512.0 + phase * 512.0) * 0.0019531245343387127 + 0.25;
    let angle = (cycles - cycles.floor()) * 6.2831854820251465 - 3.1415927410125732;
    let square = angle * angle;
    let mut value = square * -2.5239850742764247e-7 + 2.476090048730839e-5;
    value = square * value - 0.001388839678838849;
    value = square * value + 0.04166664183139801;
    value = square * value - 0.5;
    square * value + 1.0
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpriteFxEffect {
    pub effect_id: u32,
    pub state: i32,
    /// Transition duration, retained as signed bits for the public PAL API.
    pub value: i32,
    pub active: bool,
    /// Whether the native source/work allocations have a portable counterpart.
    /// Stop can set state 2 before start has supplied either allocation.
    pub has_source: bool,
    pub source: RasterSource,
    pub start_ms: u32,
    pub uniforms: RasterUniforms,
}

impl SpriteFxEffect {
    pub fn start(source: RasterSource, duration_ms: u32, now_ms: u32) -> Self {
        Self {
            effect_id: 0,
            state: 1,
            value: duration_ms as i32,
            active: true,
            has_source: true,
            source,
            start_ms: now_ms,
            uniforms: RasterUniforms::default(),
        }
    }

    pub fn stop(&mut self, duration_ms: u32, now_ms: u32) {
        self.state = 2;
        self.active = true;
        self.value = duration_ms as i32;
        self.start_ms = now_ms;
    }

    /// Action 0 frees source/work buffers, but keeps the owning FX object.
    pub fn clear(&mut self) {
        self.state = 0;
        self.active = false;
        self.has_source = false;
        self.source = RasterSource::default();
        self.uniforms = RasterUniforms::default();
    }

    pub fn tick(&mut self, now_ms: u32, phase_start: &mut Option<u32>) {
        if !self.active {
            return;
        }
        let duration = self.value as u32;
        let elapsed = now_ms.wrapping_sub(self.start_ms);
        let progress = if elapsed < duration {
            elapsed as f32 / duration as f32
        } else {
            1.0
        };
        // Native stop restarts at full amplitude, even during the entry ramp.
        let amount = if self.state == 2 {
            1.0 - progress
        } else {
            progress
        };
        // The native setter accepts stop without a source. Its later draw
        // dereferences missing buffers; keep that invalid drawing input inert.
        if self.has_source {
            let start = phase_start.get_or_insert(now_ms);
            let phase_elapsed = now_ms.wrapping_sub(*start);
            let offset = if phase_elapsed < self.source.period_ms {
                phase_elapsed as f32 / self.source.period_ms as f32
            } else {
                *start = now_ms;
                1.0
            };
            self.uniforms = RasterUniforms {
                offset,
                undulate_x: self.source.undulate_x * amount,
                undulate_y: self.source.undulate_y * amount,
            };
        }
        if self.state == 2 && elapsed >= duration {
            self.state = 0;
            self.active = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_holds_and_stop_restarts_at_full_amplitude() {
        let source = RasterSource {
            period_ms: 100,
            undulate_x: 0.4,
            undulate_y: 0.2,
        };
        let mut fx = SpriteFxEffect::start(source, 100, 10);
        let mut clock = None;
        fx.tick(10, &mut clock);
        assert_eq!(fx.uniforms, RasterUniforms::default());
        fx.tick(35, &mut clock);
        assert_eq!(fx.uniforms.undulate_x, 0.1);
        fx.stop(50, 35);
        fx.tick(35, &mut clock);
        assert_eq!(fx.state, 2);
        assert_eq!(fx.uniforms.undulate_x, 0.4);
        fx.tick(60, &mut clock);
        assert_eq!(fx.uniforms.undulate_x, 0.2);
        fx.tick(85, &mut clock);
        assert_eq!(fx.state, 0);
        assert!(!fx.active);
        let mut fx = SpriteFxEffect::start(source, 0, 0);
        fx.tick(1000, &mut clock);
        assert_eq!(fx.state, 1);
        assert_eq!(fx.uniforms.undulate_x, 0.4);
    }

    #[test]
    fn phase_clock_is_shared_wraps_and_resets_without_modulo() {
        let source = RasterSource {
            period_ms: 100,
            ..Default::default()
        };
        let mut a = SpriteFxEffect::start(source, 0, u32::MAX - 20);
        let mut b = a;
        let mut clock = None;
        a.tick(u32::MAX - 20, &mut clock);
        b.tick(29, &mut clock);
        assert_eq!(b.uniforms.offset, 0.5);
        a.tick(129, &mut clock);
        assert_eq!(a.uniforms.offset, 1.0);
        b.tick(129, &mut clock);
        assert_eq!(b.uniforms.offset, 0.0);
    }
}
