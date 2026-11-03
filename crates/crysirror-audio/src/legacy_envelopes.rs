//! Wrapping-tick gain ramps and independent 128-level attenuation banks.
#[derive(Debug, Clone, Copy)]
pub struct GainRamp128 {
    start: u32,
    duration: i32,
    from: i32,
    to: i32,
}

impl GainRamp128 {
    pub fn new(now: u32, duration: i32, from: u8, to: u8) -> Self {
        Self {
            start: now,
            duration,
            from: i32::from(from),
            to: i32::from(to),
        }
    }

    pub fn sample(self, now: u32) -> (u8, bool) {
        let elapsed = now.wrapping_sub(self.start) as i32;
        let t = if self.duration > 0 {
            f64::from(elapsed) / f64::from(self.duration)
        } else {
            1.0
        };
        if t < 1.0 {
            (
                (t * f64::from(self.to - self.from) + f64::from(self.from)) as u8,
                false,
            )
        } else {
            (self.to as u8, true)
        }
    }
}


pub fn compose_128_factors(factors: [u8; 4]) -> f64 {
    let deficits = factors.map(|value| 128_u32.saturating_sub(u32::from(value.min(128))));
    if deficits
        .into_iter()
        .any(|deficit| f64::from(deficit) / 2.666_666_66 >= 48.0)
    {
        return 0.0;
    }
    let deficit = deficits.into_iter().sum::<u32>();
    let attenuation = ((f64::from(deficit) / 2.666_666_66) as i32).clamp(0, 128);
    f64::from(128 - attenuation) / 128.0
}
