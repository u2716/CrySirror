// SPDX-License-Identifier: MPL-2.0
//! Shake: trial 4113E0/449A00/453A50/45C950/44D300/495BB0.
//! Native default FPS mode4 has a 60-frame/s envelope and a16ms counter unit.
//! Generic interpolation/alternation belongs to shared scene animation; this
//! module retains only native argument packing, target and wait/skip policy.
use super::*;
use crysirror_scene::{
    animation::SteppedAlternation,
    layer::{Easing, ExternalMotionStamp, MotionStatus},
};
const DRIVER: u32 = 0x4e32_5348;

pub(super) struct Preparing {
    targets: Vec<u32>,
    task: Task<SteppedAlternation>,
    bytes: usize,
    duration: u64,
}
pub(super) struct Shake {
    clock: SteppedAlternation,
    stamp: ExternalMotionStamp,
}
impl N2SystemVm {
    fn shake_easing(args: &Arguments<'_>) -> Result<Easing> {
        let value = args.string(7)?;
        // Native44D8F0 adds the first byte twice. Unknown spellings explicitly
        // select its linear table; quoted spellings have separate equal routes.
        let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(&value);
        ensure!(
            !errors && bytes.len() <= 4096,
            "N2System shake easing encoding/length"
        );
        let hash: u32 = bytes.iter().copied().map(u32::from).sum::<u32>()
            + bytes.first().copied().map_or(0, u32::from);
        let value = match hash {
            407 | 475 => json!("ease_in"),
            408 | 476 => {
                json!({"progress":{"shape":{"kind":"power","exponent":3.,"direction":"in"}}})
            }
            409 | 477 => json!("ease_in_quart"),
            413 | 481 => json!("ease_out"),
            414 | 482 => {
                json!({"progress":{"shape":{"kind":"power","exponent":3.,"direction":"out"}}})
            }
            415 | 483 => json!("ease_out_quart"),
            767 | 835 => json!({"progress":{"shape":{"kind":"sine","direction":"in"}}}),
            773 | 841 => json!({"progress":{"shape":{"kind":"sine","direction":"out"}}}),
            654 | 722 => {
                json!({"progress":{"shape":{"kind":"half_cosine","pi":std::f64::consts::PI}}})
            }
            657 | 725 => json!({"progress":{"shape":{"kind":"inverse_half_cosine"}}}),
            _ => json!("linear"),
        };
        Ok(serde_json::from_value(value)?)
    }
    pub(super) fn native_shake(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
    ) -> Result<bool> {
        let mut duration = state
            .shake_preparation
            .as_ref()
            .map(|p| p.duration)
            .unwrap_or(args.time(1)?);
        if state.shake_preparation.is_none()
            && self.text_switch("SYSTEM_skip", false)?
            && duration < 544
        {
            duration = 0;
        }
        let mut targets = BTreeSet::new();
        for root in self.query(current, &args.string(0)?)? {
            targets.extend(self.entities.animation_targets(root));
        }
        let targets: Vec<_> = targets.into_iter().collect();
        let frequency = args.integer(6)?;
        if frequency > 1000 {
            //449A00 rejects before queuing and the caller ignores that result.
            self.machine.complete(&mut state.thread, Value::Null)?;
            return Ok(true);
        }
        if duration == 0 {
            state.shake_preparation = None;
            for id in targets {
                self.end_shake(host, id)?;
            }
            self.machine.complete(&mut state.thread, Value::Null)?;
            return Ok(true);
        }
        ensure!(
            frequency > 0,
            "N2System zero/negative Shake frequency has undefined native conversion"
        );
        if state.shake_preparation.is_none() {
            if targets.is_empty() {
                self.machine.complete(&mut state.thread, Value::Null)?;
                return Ok(true);
            }
            ensure!(
                targets.len() + self.shakes.len() <= 4096,
                "N2System shake producer capacity"
            );
            let frames = (duration * 60 / 1000).max(1);
            ensure!(frames <= 65535, "N2System Shake16-bit native frame range");
            let remaining: usize = self
                .shakes
                .values()
                .map(|s| s.clock.resident_bytes())
                .sum::<usize>()
                + self
                    .threads
                    .values()
                    .filter_map(|state| state.shake_preparation.as_ref())
                    .map(|p| p.bytes)
                    .sum::<usize>();
            let bytes = (frames as usize + 1)
                .checked_mul(8)
                .and_then(|n| n.checked_mul(targets.len()))
                .context("N2System shake working set overflow")?;
            let limit: usize = if cfg!(target_os = "vita") {
                2 << 20
            } else {
                8 << 20
            };
            ensure!(
                bytes <= limit.saturating_sub(remaining),
                "N2System Shake sample working set"
            );
            let packed_start = (args.integer(2)? as u32).wrapping_shl(16) | args.integer(3)? as u32;
            let packed_end = (args.integer(4)? as u32).wrapping_shl(16) | args.integer(5)? as u32;
            let unpack = |v: u32| [(v >> 16) as u16 as i16 as i32, v as u16 as i32];
            let start = unpack(packed_start);
            let end = unpack(packed_end);
            let interval = (1_000_000. / (f64::from(frequency) * 60.)).trunc() as u32;
            let easing = Self::shake_easing(args)?;
            let task = match self.pool.as_ref().unwrap().spawn(move |cancel| {
                SteppedAlternation::new(start, end, frames as u32, 16, interval, easing, &|| {
                    cancel.check()
                })
            }) {
                Ok(task) => task,
                Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                Err(e) => return Err(e),
            };
            state.shake_preparation = Some(Preparing {
                targets,
                task,
                bytes,
                duration,
            });
            return Ok(false);
        }
        let prepared = state.shake_preparation.as_mut().unwrap();
        let Some(mut clock) = prepared.task.try_take()? else {
            return Ok(false);
        };
        let prepared = state.shake_preparation.take().unwrap();
        let now = host.now_ms();
        clock.restart(now as u32);
        let admitted: Vec<_> = prepared
            .targets
            .into_iter()
            .filter(|id| self.entities.nodes.contains_key(id))
            .collect();
        let mut changes = Vec::new();
        for &id in &admitted {
            if let Some(shake) = self.shakes.get(&id) {
                let mut stamp = shake.stamp;
                stamp.revision = stamp
                    .revision
                    .checked_add(1)
                    .context("N2System shake revision")?;
                changes.push(json!({"op":"commit_external_motion","id":id,"driver":DRIVER,"stamp":stamp,"complete":true,
                    "pose":{"offset":{"translation":[0.,0.,0.],"scale":[0.,0.,0.],"rotation":[0.,0.,0.]}}}));
                changes.push(json!({"op":"remove_external_motion","id":id,"driver":DRIVER,"generation":stamp.generation.to_string()}));
            }
            changes.push(json!({"op":"begin_external_motion","id":id,"driver":DRIVER}));
        }
        // Stop/replacement/admission is one batch: a capacity failure cannot
        // leave only some targets started, or orphan an admitted producer.
        self.submit(host, changes, vec![])?;
        for &id in &admitted {
            let status: MotionStatus = serde_json::from_slice(
                &host.query_engine(self.client, EngineQuery::LayerMotion(id))?,
            )?;
            let stamp = status
                .drivers
                .into_iter()
                .find(|s| s.driver == DRIVER)
                .context("N2System shake producer receipt")?
                .stamp;
            self.shakes.insert(
                id,
                Shake {
                    clock: clock.clone(),
                    stamp,
                },
            );
        }
        let wait = args.value(8);
        let until = match wait {
            Value::Boolean(true) => Some(None),
            Value::String(s) if s.eq_ignore_ascii_case("true") => Some(None),
            Value::Null | Value::Boolean(false) => None,
            Value::String(s)
                if s.eq_ignore_ascii_case("false") || s.eq_ignore_ascii_case("null") =>
            {
                None
            }
            _ => {
                let delay = args.time(8)?;
                (delay > 0).then(|| Some(now.saturating_add(delay)))
            }
        };
        if let Some(until) = until {
            state.wait = Wait::Shakes {
                ids: admitted,
                until,
            };
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn end_shake(&mut self, host: &mut dyn HostServices, id: u32) -> Result<()> {
        let Some(shake) = self.shakes.get(&id) else {
            return Ok(());
        };
        let mut stamp = shake.stamp;
        stamp.revision = stamp
            .revision
            .checked_add(1)
            .context("N2System shake revision")?;
        self.submit(host,vec![
            json!({"op":"commit_external_motion","id":id,"driver":DRIVER,"stamp":stamp,"complete":true,"pose":{"offset":{"translation":[0.,0.,0.],"scale":[0.,0.,0.],"rotation":[0.,0.,0.]}}}),
            json!({"op":"remove_external_motion","id":id,"driver":DRIVER,"generation":stamp.generation.to_string()}),
        ],vec![])?;
        self.shakes.remove(&id);
        Ok(())
    }
    pub(super) fn sample_shakes(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let now = host.now_ms() as u32;
        // Original485890 initializes normal speed1 and skip speed32.
        let speed = if self.text_switch("SYSTEM_skip", false)? {
            32.
        } else {
            1.
        };
        let mut changes = Vec::new();
        let mut done = Vec::new();
        for (&id, shake) in &mut self.shakes {
            let Some([x, y]) = shake.clock.advance(now, speed)? else {
                continue;
            };
            shake.stamp.revision = shake
                .stamp
                .revision
                .checked_add(1)
                .context("N2System shake revision")?;
            changes.push(json!({"op":"commit_external_motion","id":id,"driver":DRIVER,"stamp":shake.stamp,"complete":shake.clock.complete,
                "pose":{"offset":{"translation":[x,y,0],"scale":[0.,0.,0.],"rotation":[0.,0.,0.]}}}));
            if shake.clock.complete {
                changes.push(json!({"op":"remove_external_motion","id":id,"driver":DRIVER,"generation":shake.stamp.generation.to_string()}));
                done.push(id);
            }
        }
        self.submit(host, changes, vec![])?;
        for id in done {
            self.shakes.remove(&id);
        }
        Ok(())
    }
}
