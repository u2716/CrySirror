// Copyright (c) 2026 CrySirror contributors. MIT.
//! YUKIIRO410170/4108B0/4108C0/411630 and416430 clock policy.
//! The host clock still owns async work/device transport; this native clock
//! owns script deadlines, interpolation, input event intervals and published
//! GetTime. Generic scaling/wrapping/conversion lives in crysirror-core.
use super::*;

pub(super) struct NativeClock {
    origin: Option<u32>,
    offset: u32,
    rate: f64,
    published: u32,
    pub frozen: bool,
}
impl Default for NativeClock {
    fn default() -> Self {
        Self {
            origin: None,
            offset: 0,
            rate: 1.0,
            published: 0,
            frozen: false,
        }
    }
}
impl NativeClock {
    fn sample(&self, host: u32) -> Result<u32> {
        Ok(crysirror_core::time_mapping::scale_wrapping_u32_time(
            host,
            self.origin.unwrap_or(0),
            if self.origin.is_some() {
                self.rate
            } else {
                1.0
            },
            self.offset,
        )?)
    }
    fn frozen_sample(&mut self, host: u32) -> Result<u32> {
        if self.frozen {
            //411630 holds the published word for this pass even when the
            //500ms escape clears the flag; the next owner pass samples anew.
            if (host.wrapping_sub(self.published) as i32).wrapping_abs() > 500 {
                self.frozen = false;
            }
            Ok(self.published)
        } else {
            self.sample(host)
        }
    }
    fn publish(&mut self, host: u32) -> Result<()> {
        self.published = self.frozen_sample(host)?;
        Ok(())
    }
    fn set_rate(&mut self, host: u32, rate: f32) -> Result<()> {
        ensure!(rate.is_finite(), "Majiro nonfinite native clock rate");
        if rate == 1.0 {
            self.offset = self.published.wrapping_sub(host);
            self.origin = None;
        } else {
            //A nonzero word is the original enabled sentinel. Preserve the
            //timeGetTime()==0 ->1 branch rather than inventing continuity.
            self.origin = Some(if host == 0 { 1 } else { host });
        }
        self.rate = f64::from(rate);
        Ok(())
    }
    fn compensate(&mut self, host: u32) -> Result<()> {
        let sample = self.frozen_sample(host)?;
        let advance = (sample.wrapping_sub(self.published) as i32).clamp(0, 1000);
        self.offset = self.offset.wrapping_sub(advance as u32);
        Ok(())
    }
    pub(super) fn diagnostics(&self) -> serde_json::Value {
        json!({"published":self.published,"origin":self.origin,"offset":self.offset,
            "rate":self.rate,"frozen":self.frozen})
    }
}
impl MajiroVm {
    pub(super) fn native_now(&self) -> u32 {
        self.clock.published
    }
    pub(super) fn publish_native_clock(&mut self, host: u64) -> Result<()> {
        self.clock.publish(host as u32)
    }
    pub(super) fn clock_native(
        &mut self,
        hash: u32,
        arguments: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        let result = match hash {
            //416430's inline case reads the first payload as a float word;
            //the dispatcher consumes any further operands without reading them.
            0x806261ce => {
                let rate = match arguments
                    .first()
                    .context("Majiro clock rate operand missing")?
                {
                    Value::Float(rate) => *rate,
                    Value::Integer(word) => f32::from_bits(*word as u32),
                    _ => bail!("Majiro clock rate requires a scalar float word"),
                };
                self.clock.set_rate(host.now_ms() as u32, rate)?;
                0
            }
            0x42f6fddf => {
                self.clock.compensate(host.now_ms() as u32)?;
                0
            }
            //401CB0/4108B0 returns the owner-pass cached word, not4108C0's
            //fresh real-time mapping. Merely querying does not advance time.
            0x6584f13e => self.native_now() as i32,
            _ => return Ok(None),
        };
        Ok(Some(if returns {
            vec![Value::Integer(result)]
        } else {
            vec![]
        }))
    }
}
