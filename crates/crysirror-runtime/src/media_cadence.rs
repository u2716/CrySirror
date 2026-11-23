//! Demand-driven presentation of independently addressable movie frames.
use crysirror_core::service::IndexedMoviePacing;
use std::sync::{
    Arc, OnceLock,
    mpsc::{self, Receiver, SyncSender},
};

pub(super) struct DecoderControl {
    pub targets: Receiver<usize>,
    pub index: Arc<OnceLock<(u32, usize)>>,
}
pub(super) struct Delivery {
    mode: IndexedMoviePacing,
    targets: SyncSender<usize>,
    index: Arc<OnceLock<(u32, usize)>>,
    outstanding: Option<usize>,
    previous: usize,
    late: u32,
    pub audio_seen: bool,
}

impl Delivery {
    pub fn new(mode: IndexedMoviePacing) -> (Self, DecoderControl) {
        let (targets, rx) = mpsc::sync_channel(1);
        let index = Arc::new(OnceLock::new());
        let decoder = DecoderControl {
            targets: rx,
            index: index.clone(),
        };
        (
            Self {
                mode,
                targets,
                index,
                outstanding: Some(0),
                previous: 0,
                late: 0,
                audio_seen: false,
            },
            decoder,
        )
    }

    pub fn presented(&mut self, decode_ms: u64) {
        let Some(frame) = self.outstanding.take() else {
            return;
        };
        if frame == 0 {
            return;
        }
        if frame > self.previous.saturating_add(1) {
            self.late = self.late.saturating_add(1);
        }
        self.previous = frame;
        if matches!(self.mode, IndexedMoviePacing::Adaptive30)
            && self.index.get().is_some_and(|i| i.0 == 30)
            && self.late < 30
            && decode_ms > 30
        {
            self.previous += 1;
            self.late += 1;
        }
    }

    /// Return true once the selected frame has crossed the end of the index.
    pub fn advance(&mut self, time: u32) -> anyhow::Result<bool> {
        if self.outstanding.is_some() {
            return Ok(false);
        }
        let Some(&(rate, frames)) = self.index.get() else {
            return Ok(false);
        };
        let mut target = rate.wrapping_mul(time) as usize / 1000;
        if rate == 30
            && (self.mode == IndexedMoviePacing::HalfRate30
                || self.mode == IndexedMoviePacing::Adaptive30 && self.late >= 30)
        {
            target &= !1;
        }
        if target <= self.previous {
            return Ok(false);
        }
        if target >= frames {
            return Ok(true);
        }
        self.targets.send(target)?;
        self.outstanding = Some(target);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn half_rate_and_adaptive_switch_use_capture_time_and_exact_frame_indices() -> anyhow::Result<()>
    {
        let (mut half, decoder) = Delivery::new(IndexedMoviePacing::HalfRate30);
        decoder.index.set((30, 5)).unwrap();
        half.presented(0);
        half.advance(34)?;
        assert!(decoder.targets.try_recv().is_err());
        half.advance(67)?;
        assert_eq!(decoder.targets.try_recv()?, 2);
        half.presented(40);
        half.advance(167)?;
        assert_eq!(decoder.targets.try_recv()?, 4);
        half.presented(0);
        assert!(half.advance(200)?);
        let (mut adaptive, decoder) = Delivery::new(IndexedMoviePacing::Adaptive30);
        decoder.index.set((30, 120)).unwrap();
        adaptive.presented(0);
        for i in 0..30 {
            let frame = 2 * i + 1;
            adaptive.advance((frame * 1000u32).div_ceil(30))?;
            assert_eq!(decoder.targets.try_recv()?, frame as usize);
            adaptive.presented(31);
        }
        adaptive.advance(2034)?; // frame 61 rounds back to 60
        assert!(decoder.targets.try_recv().is_err());
        adaptive.advance(2067)?;
        assert_eq!(decoder.targets.try_recv()?, 62);
        Ok(())
    }
}
