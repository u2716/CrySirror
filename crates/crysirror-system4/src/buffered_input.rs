// Copyright (C) 2026 CrySirror contributors. GPL-2.0-or-later.
//! Native event-dispatch policy over the engine-owned logical InputBuffer.
//! It retains framed input during a native peek lock; no OS event loop, private
//! event queue implementation or fabricated pointer/focus data is introduced.
use anyhow::{ensure, Result};
use crysirror_core::{
    input_buffer::{InputBuffer, MAX_INPUT_SAMPLES},
    service::InputFrame,
    state::ClientId,
};
pub(crate) struct Events {
    owner: ClientId,
    buffer: InputBuffer,
    revision: Option<(u64, u64, InputFrame)>,
    fallback: Option<(u64, InputFrame)>,
}
impl Events {
    pub(crate) fn new(owner: ClientId) -> Result<Self> {
        // Receipts never escape this instance and no restore path exists.
        Ok(Self {
            owner,
            buffer: InputBuffer::new(owner.0, 1, MAX_INPUT_SAMPLES)?,
            revision: None,
            fallback: None,
        })
    }
    pub(crate) fn offer(
        &mut self,
        owner: ClientId,
        time: u64,
        frame: InputFrame,
        revision: Option<u64>,
    ) -> Result<()> {
        ensure!(owner == self.owner, "foreign System4 input inbox");
        if let Some(revision) = revision {
            if self.revision == Some((revision, time, frame)) {
                return Ok(());
            }
            self.buffer.push(owner.0, time, frame)?;
            self.revision = Some((revision, time, frame));
        } else {
            if self.fallback == Some((time, frame)) {
                return Ok(());
            }
            self.buffer.push(owner.0, time, frame)?;
            self.fallback = Some((time, frame));
        }
        Ok(())
    }
    pub(crate) fn pump(
        &mut self,
        owner: ClientId,
        locked: bool,
        force: bool,
        input: &mut crate::input::Input,
    ) -> Result<()> {
        ensure!(owner == self.owner, "foreign System4 input pump");
        if locked && !force {
            return Ok(());
        }
        let batch = self.buffer.read(owner.0, MAX_INPUT_SAMPLES)?;
        for sample in batch.samples {
            input.observe_sample(self.buffer.epoch(), sample);
        }
        if let Some(receipt) = batch.receipt {
            self.buffer.acknowledge(owner.0, receipt)?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crysirror_core::service::buttons;
    #[test]
    fn native_lock_defers_shared_receipts_and_forced_peek_retains_clear_until_new_press() {
        let owner = ClientId(3);
        let mut events = Events::new(owner).unwrap();
        let mut input = crate::input::Input::default();
        let pressed = InputFrame {
            buttons_down: buttons::ENTER,
            buttons_pressed: buttons::ENTER,
            ..Default::default()
        };
        events.offer(owner, 0, pressed, Some(1)).unwrap();
        events.offer(owner, 0, pressed, Some(1)).unwrap();
        events.pump(owner, true, false, &mut input).unwrap();
        assert_eq!(input.key(13).unwrap(), 0);
        assert_eq!(events.buffer.len(), 1);
        events.pump(owner, true, true, &mut input).unwrap();
        assert_eq!(input.key(13).unwrap(), 1);
        assert!(events.buffer.is_empty());
        input.clear(false);
        events.pump(owner, false, false, &mut input).unwrap();
        assert_eq!(input.key(13).unwrap(), 0);
        events
            .offer(
                owner,
                1,
                InputFrame {
                    buttons_down: buttons::ENTER,
                    ..Default::default()
                },
                Some(2),
            )
            .unwrap();
        events.pump(owner, false, false, &mut input).unwrap();
        assert_eq!(input.key(13).unwrap(), 0);
        events
            .offer(owner, 2, InputFrame::default(), Some(3))
            .unwrap();
        events.offer(owner, 3, pressed, Some(4)).unwrap();
        events.pump(owner, false, false, &mut input).unwrap();
        assert_eq!(input.key(13).unwrap(), 1);
        assert!(events.pump(ClientId(4), false, true, &mut input).is_err());
    }
    #[test]
    fn native_locked_frame_history_preserves_a_wheel_pulse_after_later_idle_frames() {
        let owner = ClientId(3);
        let mut events = Events::new(owner).unwrap();
        let mut input = crate::input::Input::default();
        events
            .offer(
                owner,
                0,
                InputFrame {
                    buttons_pressed: buttons::WHEEL_UP,
                    ..Default::default()
                },
                Some(1),
            )
            .unwrap();
        events
            .offer(owner, 100, InputFrame::default(), Some(2))
            .unwrap();
        events.pump(owner, true, false, &mut input).unwrap();
        assert_eq!(input.wheel(), [0, 0]);
        events.pump(owner, false, false, &mut input).unwrap();
        assert_eq!(input.wheel(), [1, 0]);
    }
}
