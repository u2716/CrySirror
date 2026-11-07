//! Original mm6 procedural position policy. Shared scalar-jitter sampling
//! computes values; Layer external producers own admission and completion.
use super::*;
use crysirror_core::service::EngineQuery;
use crysirror_image::progress_curve::{sample_scalar_jitter, ProgressDirection, ProgressShape};
use crysirror_scene::layer::{ExternalMotionStamp, MotionStatus};
const DRIVER: u32 = 1;
#[derive(Clone)]
pub(super) struct Position {
    from: [f64; 2],
    to: [f64; 2],
    started: u64,
    advanced: u64,
    duration: u64,
    repeat: i32,
    strength: f64,
    stamp: ExternalMotionStamp,
    last_tick: Option<u64>,
}
impl Position {
    pub(super) fn resume_clock(&mut self, since: u64, now: u64) {
        self.started = self
            .started
            .saturating_add(now.saturating_sub(since.max(self.started)));
        self.last_tick = None;
    }
    pub(super) fn advance_clock(&mut self, delta: u64) {
        self.advanced = self.advanced.saturating_add(delta);
        self.last_tick = None;
    }
}
impl QlieVm {
    pub(super) fn cancel_native_position(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
    ) -> Result<()> {
        if let Some(position) = self.objects.get_mut(id).unwrap().native_position.take() {
            let layer = self.objects[id].layer;
            self.submit(h,json!({"layers":[{"op":"remove_external_motion","id":layer,"driver":DRIVER,"generation":position.stamp.generation.to_string()}]}))?;
        }
        Ok(())
    }
    pub(super) fn begin_jitter_position(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        duration: u64,
        repeat: i32,
    ) -> Result<()> {
        self.cancel_native_position(h, id)?;
        let object = &self.objects[id];
        let layer = object.layer;
        let view: serde_json::Value =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::Layer(layer))?)?;
        let from = [
            view["rect"][0].as_f64().context("QLIE jitter position X")?,
            view["rect"][1].as_f64().context("QLIE jitter position Y")?,
        ];
        let to = [f64::from(object.position[0]), f64::from(object.position[1])];
        let strength = object
            .properties
            .get("mm")
            .and_then(|v| v.get(1))
            .map(Value::integer)
            .transpose()?
            .unwrap_or(100)
            .clamp(0, 200);
        self.submit(h,json!({"layers":[{"op":"end_motion","id":layer,"property":"x"},{"op":"end_motion","id":layer,"property":"y"},{"op":"begin_external_motion","id":layer,"driver":DRIVER}]}))?;
        let status: MotionStatus =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::LayerMotion(layer))?)?;
        let stamp = status
            .drivers
            .into_iter()
            .find(|v| v.driver == DRIVER)
            .context("QLIE jitter producer admission")?
            .stamp;
        self.objects.get_mut(id).unwrap().native_position = Some(Position {
            from,
            to,
            started: h.now_ms(),
            advanced: 0,
            duration,
            repeat,
            strength: f64::from(strength) * 0.01,
            stamp,
            last_tick: None,
        });
        self.update_native_positions(h)
    }
    pub(super) fn update_native_positions(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.host_thread == ThreadKind::Game && self.input_thread() == ThreadKind::System {
            return Ok(());
        }
        let now = h.now_ms();
        let mut commands = vec![];
        let random = self
            .random
            .as_mut()
            .context("QLIE jitter session seed pending")?;
        for object in self
            .objects
            .values_mut()
            .filter(|o| o.thread == self.host_thread && o.created)
        {
            let Some(position) = &mut object.native_position else {
                continue;
            };
            if position.last_tick == Some(now) {
                continue;
            }
            let status: MotionStatus = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::LayerMotion(object.layer))?,
            )?;
            let producer = status
                .drivers
                .iter()
                .find(|v| v.driver == DRIVER)
                .context("QLIE jitter producer absent")?;
            ensure!(
                producer.stamp == position.stamp,
                "QLIE jitter producer ownership"
            );
            let elapsed = now
                .saturating_sub(position.started)
                .saturating_add(position.advanced);
            let limit = if position.repeat < 0 {
                u64::MAX
            } else {
                position.duration.saturating_mul(position.repeat as u64 + 1)
            };
            let complete = producer.skip_requested || elapsed >= limit || position.duration == 0;
            let phase = if complete {
                1.
            } else {
                (elapsed % position.duration) as f64 / position.duration as f64
            };
            let mut point = position.to;
            if !complete {
                for axis in 0..2 {
                    let noise = random.sample(0xffffff) as f64 / 16777215.;
                    point[axis] = sample_scalar_jitter(
                        ProgressShape::Power {
                            exponent: 1. + 1.4 * position.strength,
                            direction: ProgressDirection::Out,
                        },
                        position.from[axis],
                        position.to[axis],
                        phase,
                        20. * position.strength,
                        noise,
                    )?;
                }
            }
            position.last_tick = Some(now);
            position.stamp.revision = position
                .stamp
                .revision
                .checked_add(1)
                .context("QLIE jitter revision")?;
            commands.push(json!({"op":"commit_external_motion","id":object.layer,"driver":DRIVER,"stamp":position.stamp,"complete":complete,"pose":{"rect":[point[0],point[1],self.config.canvas[0],self.config.canvas[1]]}}));
            if complete {
                commands.push(json!({"op":"remove_external_motion","id":object.layer,"driver":DRIVER,"generation":position.stamp.generation.to_string()}));
                object.native_position = None;
            }
        }
        if !commands.is_empty() {
            self.submit(h, json!({"layers":commands}))?;
        }
        Ok(())
    }
}
