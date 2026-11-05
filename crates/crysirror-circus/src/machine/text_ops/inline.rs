//!431F20's native event selection. Retained movement, masks and sampling are
//! supplied by shared Layer actors; this module only spells the native sequence.
use super::*;
use crate::machine::sprite_ops::{SpriteCommand, SpriteRequest};
use crysirror_scene::layer::queued::{DeltaSource, SequenceOperation, SequenceTrack};
pub(super) fn deferred_in_mode_two(kind: u8) -> bool {
    matches!(kind,14..=17|19..=22|34|35)
}
impl Machine {
    pub(super) fn execute_inline_effect(
        &mut self,
        event: &InlineEvent,
        host: &mut dyn NativeHost,
    ) -> Result<bool> {
        let kind = event.kind;
        // These four values are genuine holes in431F20's bounded branches,
        // despite unreachable fifth switch arms in the original source.
        if matches!(kind, 18 | 23 | 28 | 33) {
            return Ok(true);
        }
        if matches!(kind, 29..=32) {
            let slot = kind - 25;
            self.state.image_names.insert(slot, String::new());
            self.state.native_live.image_name(slot, "")?;
            if self.state.native_skipping() {
                for slot in [21, slot] {
                    if host
                        .resource(&crate::machine::resource_ops::ResourceRequest::Unload { slot })?
                        .is_none()
                    {
                        return Ok(false);
                    }
                    self.state.native_live.unload_image(slot)?;
                    self.state.image_names.remove(&slot);
                }
                return Ok(true);
            }
        }
        if self.state.native_skipping() {
            return Ok(true);
        }
        let objects: Vec<u16> = match kind {
            14..=17 => vec![10 * u16::from(kind - 13)],
            19..=22 => vec![10 * u16::from(kind - 18)],
            24..=27 => vec![10 * u16::from(kind - 23)],
            29..=32 => vec![10 * u16::from(kind - 28)],
            34 | 35 => (0..64).collect(),
            _ => bail!("Circus inline effect kind{kind}"),
        };
        let mask = matches!(kind,24..=27|29..=32);
        let commands = objects
            .iter()
            .map(|object| {
                Ok(SpriteCommand {
                    seed: self.state.native_live.sprite_seed(*object)?,
                    operations: if mask {
                        vec![SequenceOperation::Finish {
                            cancel: false,
                            lanes: None,
                        }]
                    } else {
                        //44A510 force-completes position before445390 clears
                        //other lanes; it does not discard an unfinished position.
                        vec![
                            SequenceOperation::Finish {
                                cancel: false,
                                lanes: Some(vec![0]),
                            },
                            SequenceOperation::Finish {
                                cancel: true,
                                lanes: None,
                            },
                        ]
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if !host.sprite(&SpriteRequest { commands })? {
            return Ok(false);
        }
        self.sync_sprite_states(host)?;
        if mask {
            for object in objects {
                if !host.effect_control(&crate::machine::effect_ops::EffectRequest::SpriteMask {
                    seed: self.state.native_live.sprite_seed(object)?,
                    mode: kind < 29,
                    ticks: 60,
                })? {
                    return Ok(false);
                }
            }
        } else {
            let vertical = matches!(kind, 14..=17 | 35);
            let mut commands = Vec::new();
            for object in objects {
                let seed = self.state.native_live.sprite_seed(object)?;
                let position = [seed.values[0], seed.values[1]];
                let mut operations = Vec::new();
                for offset in [8, 0, -8, 0] {
                    let mut target = position;
                    target[usize::from(vertical)] += offset as f32;
                    operations.push(SequenceOperation::Enqueue {
                        lane: 0,
                        bank: None,
                        track: SequenceTrack::Linear {
                            channels: vec![0, 1],
                            target: target.iter().map(|v| (*v as i32) as f32).collect(),
                            ticks: 10,
                            velocity_divisor: None, ieee_inputs: false,
                            references: vec![0, 1],
                            loop_origins: vec![0, 1],
                            delta: DeltaSource::Retained,
                            looping: false,
                            start: None,
                            complete_first: false,
                            complete_zero: true,
                            fast: true,
                            fast_channels: None,
                            skip_zero: true,
                            capture_zero: true,
                        },
                    });
                }
                commands.push(SpriteCommand { seed, operations });
            }
            if !host.sprite(&SpriteRequest { commands })? {
                return Ok(false);
            }
        }
        self.sync_sprite_states(host)?;
        Ok(true)
    }
}
