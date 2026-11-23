// SPDX-License-Identifier: MPL-2.0
//! RLD v1 enterClip on the active CG surface, using TmAsyncMagnify's integer edges.
use anyhow::{Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::json;

pub(super) struct Clip {
    from: [i32; 4],
    to: [i32; 4],
    shown: [i32; 4],
    source: [u32; 2],
    asset: String,
    started: Option<u64>,
    duration: u32,
    mode: i32,
}

fn edges([x, y, w, h]: [i32; 4], [sw, sh]: [u32; 2]) -> Result<[i32; 4]> {
    ensure!(
        x >= 0
            && y >= 0
            && w > 0
            && h > 0
            && i64::from(x) + i64::from(w) <= i64::from(sw)
            && i64::from(y) + i64::from(h) <= i64::from(sh)
            && sw <= i32::MAX as u32
            && sh <= i32::MAX as u32,
        "unsupported ExHibit clip outside source"
    );
    Ok([x, y, x + w, y + h])
}

fn offset(delta: i32, elapsed: u32, duration: u32, mode: i32) -> i32 {
    if elapsed >= duration {
        return delta;
    }
    if elapsed == 0 {
        return 0;
    }
    let n = i64::from(delta).abs();
    let step = match mode {
        // FCAsyncEffect::calcInt squares the two unsigned clock words before widening.
        1 => {
            n * i64::from(elapsed.wrapping_mul(elapsed))
                / i64::from(duration.wrapping_mul(duration))
        }
        2 => ((elapsed as f64 * n as f64 * n as f64 / duration as f64).sqrt()) as i64,
        3 => (n as f64 * (elapsed as f64).ln() / (duration as f64).ln()) as i64,
        _ => n * i64::from(elapsed) / i64::from(duration),
    };
    delta.signum() * step.clamp(0, n) as i32
}

impl Clip {
    pub fn new(
        asset: String,
        source: [u32; 2],
        from: [i32; 4],
        to: [i32; 4],
        duration: u32,
        mode: i32,
    ) -> Result<Self> {
        ensure!(
            mode != 1 || duration == 0 || duration.wrapping_mul(duration) != 0,
            "invalid ExHibit quadratic clip duration"
        );
        let from = edges(from, source)?;
        let to = edges(to, source)?;
        Ok(Self {
            from,
            to,
            shown: from,
            asset,
            source,
            started: None,
            duration,
            mode,
        })
    }
    fn sample(&self, elapsed: u32) -> [i32; 4] {
        std::array::from_fn(|i| {
            self.from[i] + offset(self.to[i] - self.from[i], elapsed, self.duration, self.mode)
        })
    }
    pub fn crop(&self) -> [i32; 4] {
        let [l, t, r, b] = self.shown;
        [l, t, r - l, b - t]
    }
    pub fn present(
        &mut self,
        h: &mut dyn HostServices,
        client: ClientId,
        canvas: [u32; 2],
    ) -> Result<bool> {
        let now = h.now_ms();
        let elapsed = now
            .saturating_sub(*self.started.get_or_insert(now))
            .min(u64::from(self.duration)) as u32;
        let next = self.sample(elapsed);
        if next != self.shown {
            let [sw, sh] = self.source;
            let uv = std::array::from_fn::<_, 4, _>(|i| next[i] as f32 / [sw, sh][i % 2] as f32);
            let [w, height] = canvas;
            let payload = json!({"layers":[{"op":"put","layer":{"id":1,"order":[0,0],"rect":[0,0,w,height],"uv":uv,"paint":{"kind":"image","asset":self.asset}}}]});
            h.submit_engine(client, 1, &serde_json::to_vec(&payload)?)?;
            self.shown = next;
        }
        Ok(elapsed == self.duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_title_pan_samples_integer_source_edges() -> Result<()> {
        let clip = Clip::new(
            "cg".into(),
            [800, 1100],
            [0, 500, 800, 600],
            [0, 0, 800, 600],
            2500,
            1,
        )?;
        assert_eq!(clip.sample(0), [0, 500, 800, 1100]);
        assert_eq!(clip.sample(1250), [0, 375, 800, 975]);
        assert_eq!(clip.sample(2499), [0, 1, 800, 601]);
        assert_eq!(clip.sample(2500), [0, 0, 800, 600]);
        assert_eq!(offset(300, 25, 100, 2), 150);
        assert_eq!(offset(-300, 10, 100, 3), -150);
        Ok(())
    }
}
