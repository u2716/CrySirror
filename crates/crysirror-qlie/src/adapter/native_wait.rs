//! Native reader waits (6D648C statements, 6D286C reader loop, 6D474C click
//! handling) and sys `msd`/`?msd` over the per-frame wait record.
//!
//! The native frame keeps mode +4C with values +50/+54, a jump label +58, a
//! sound channel +5C and the stop label +60. Modes: 0 run, 1 resume (play),
//! 2 stop, 3 wait, 4 clk, 5 time, 6 sndt, 7/8 msg (message revealing /
//! held), 9 btn, 10 wav. Time values count down by the frame time while
//! positive; this adapter keeps absolute host deadlines instead.
//!
//! - `time` (no argument) restarts the reader clock (+344); `time,ms[,label]`
//!   waits until the clock reaches `ms`. A click answers ResultInt0 = clock
//!   and, with a label, jumps there and ends the wait.
//! - `sndt,channel,ms[,label]` waits until the sound channel stops or reaches
//!   `ms`; a click answers ResultInt0 = position and jumps to the label.
//! - `wav,channel` waits until the channel stops; a click ends the wait.
//! - `msg` waits while a message is revealing; a click completes the reveal.
//! - `btn[,ms]` waits for a button click (ResultObj0 the button, ResultInt0..3
//!   and ResultStr0..3 its `ui`/`us` values); 0 or omitted waits forever; a
//!   timeout answers ResultObj0 null, ResultInt0 0 and ResultStr0 "".
use super::*;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Record {
    pub mode: u8,
    pub p1: i32,
    pub p2: i32,
    pub text: String,
    pub label: String,
    pub channel: i32,
}

fn remaining(deadline: u64, now: u64) -> i32 {
    deadline.saturating_sub(now).min(i32::MAX as u64) as i32
}

impl QlieVm {
    fn clock_slot(&self) -> usize {
        usize::from(self.host_thread == ThreadKind::System)
    }
    pub(super) fn script_clock(&self, now: u64) -> i64 {
        now.saturating_sub(self.script_clock[self.clock_slot()]) as i64
    }
    fn sound_state(&self, h: &mut dyn HostServices, channel: i32) -> Result<(bool, i64)> {
        let Some(o) = self.objects.get(&format!("snd[{channel}]")) else {
            return Ok((false, 0));
        };
        if o.channel == 0 {
            return Ok((false, 0));
        }
        Ok(match h.audio_info(self.client, o.channel)? {
            Some(i) => (
                i.source_playing,
                (i.position_frames as f64 * 1000. / f64::from(i.source_rate.max(1))).round() as i64,
            ),
            None => (false, 0),
        })
    }
    /// Statements; false when `op` is not one of them.
    pub(super) fn wait_statement(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[String],
    ) -> Result<bool> {
        let int = |m: &mut Machine, i: usize| -> Result<i32> {
            a.get(i).map(|v| m.eval(v)?.integer()).transpose().map(|v| v.unwrap_or(0))
        };
        let text = |m: &mut Machine, i: usize| -> Result<String> {
            a.get(i).map(|v| m.eval(v).map(|v| v.text())).transpose().map(|v| v.unwrap_or_default())
        };
        let now = h.now_ms();
        match op {
            "time" => {
                m.result(0, Value::Int(0));
                if a.is_empty() {
                    let slot = self.clock_slot();
                    self.script_clock[slot] = now;
                } else {
                    let target = int(m, 0)?;
                    let label = text(m, 1)?;
                    let origin = self.script_clock[self.clock_slot()];
                    self.wait = Some(Wait::Time {
                        deadline: origin.saturating_add(target.max(0) as u64),
                        label,
                    });
                }
            }
            "sndt" => {
                m.result(0, Value::Int(0));
                let channel = int(m, 0)?.clamp(0, 63);
                let until = int(m, 1)?;
                let label = text(m, 2)?;
                self.wait = Some(Wait::Sound { channel, until, label });
            }
            "wav" => {
                let channel = int(m, 0)?.clamp(0, 63);
                self.wait = Some(Wait::Wave { channel });
            }
            "msg" => self.wait = Some(Wait::Message),
            "btn" => {
                let ms = int(m, 0)?;
                self.wait = Some(Wait::Button {
                    deadline: (ms > 0).then(|| now.saturating_add(ms as u64)),
                });
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    fn message_revealing(&self, h: &mut dyn HostServices) -> Result<Vec<u32>> {
        let mut layers = Vec::new();
        for o in self.objects.values().filter(|o| o.kind == "msg" && o.thread == self.host_thread) {
            let Some(message) = o.message.as_ref().filter(|m| m.created) else { continue };
            let layer: crysirror_scene::layer::LayerSpec = serde_json::from_slice(
                &h.query_engine(self.client, crysirror_core::service::EngineQuery::Layer(message.layer))?,
            )?;
            if layer.paint.glyph_reveal_status().is_some_and(|s| !s.complete) {
                layers.push(message.layer);
            }
        }
        Ok(layers)
    }
    /// Polls the native waits; Some(wait) keeps waiting.
    pub(super) fn poll_native_wait(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        wait: Wait,
        pressed: bool,
    ) -> Result<Option<Wait>> {
        let now = h.now_ms();
        Ok(match wait {
            Wait::Time { deadline, label } => {
                if now >= deadline {
                    None
                } else if pressed {
                    m.result(0, Value::Int(self.script_clock(now).min(i32::MAX as i64) as i32));
                    if label.is_empty() {
                        Some(Wait::Time { deadline, label })
                    } else {
                        m.jump(&label, None, None, false)?;
                        None
                    }
                } else {
                    Some(Wait::Time { deadline, label })
                }
            }
            Wait::Sound { channel, until, label } => {
                let (playing, position) = self.sound_state(h, channel)?;
                if !playing || position >= i64::from(until) {
                    None
                } else if pressed {
                    m.result(0, Value::Int(position.min(i32::MAX as i64) as i32));
                    if label.is_empty() {
                        Some(Wait::Sound { channel, until, label })
                    } else {
                        m.jump(&label, None, None, false)?;
                        None
                    }
                } else {
                    Some(Wait::Sound { channel, until, label })
                }
            }
            Wait::Wave { channel } => {
                let (playing, _) = self.sound_state(h, channel)?;
                (playing && !pressed).then_some(Wait::Wave { channel })
            }
            Wait::Message => {
                let revealing = self.message_revealing(h)?;
                if revealing.is_empty() {
                    None
                } else {
                    if pressed {
                        let commands: Vec<_> = revealing
                            .into_iter()
                            .map(|id| json!({"op":"complete_reveal","id":id}))
                            .collect();
                        self.submit(h, json!({"layers":commands}))?;
                    }
                    Some(Wait::Message)
                }
            }
            Wait::Button { deadline } => {
                if let Some(id) = self.button_clicked.take() {
                    let o = &self.objects[&id];
                    let ints = o.properties.get("ui").cloned().unwrap_or_default();
                    let strings = o.properties.get("us").cloned().unwrap_or_default();
                    m.thread_globals_mut().insert("resultobj[0]".into(), Value::Text(id.clone()));
                    for i in 0..4 {
                        m.result(i, ints.get(i).cloned().unwrap_or(Value::Int(0)));
                        m.result_text(i, strings.get(i).map(Value::text).unwrap_or_default());
                    }
                    None
                } else if deadline.is_some_and(|t| now >= t) {
                    m.thread_globals_mut().insert("resultobj[0]".into(), Value::Int(0));
                    m.result(0, Value::Int(0));
                    m.result_text(0, "");
                    None
                } else {
                    Some(Wait::Button { deadline })
                }
            }
            other => Some(other),
        })
    }
    fn record_of(&self, wait: Option<&Wait>, now: u64) -> Record {
        let origin = self.script_clock[self.clock_slot()];
        match wait {
            None => Record::default(),
            Some(Wait::FixedClock(t)) => Record { mode: 3, p1: remaining(*t, now), p2: remaining(*t, now), ..Default::default() },
            Some(Wait::Clock(t)) => {
                let r = if *t >= u64::MAX / 2 { -1 } else { remaining(*t, now) };
                Record { mode: 4, p1: r, p2: r, ..Default::default() }
            }
            Some(Wait::Stop(s)) => {
                let r = s.deadline.map_or(-1, |t| remaining(t, now));
                Record { mode: 2, p1: r, p2: r, text: s.label.clone(), ..Default::default() }
            }
            Some(Wait::Time { deadline, label }) => Record {
                mode: 5,
                p1: deadline.saturating_sub(origin).min(i32::MAX as u64) as i32,
                label: label.clone(),
                ..Default::default()
            },
            Some(Wait::Sound { channel, until, label }) => Record {
                mode: 6,
                p1: *until,
                label: label.clone(),
                channel: *channel,
                ..Default::default()
            },
            Some(Wait::Message) => Record { mode: 7, ..Default::default() },
            Some(Wait::Button { deadline }) => Record {
                mode: 9,
                p1: deadline.map_or(-1, |t| remaining(t, now)),
                ..Default::default()
            },
            Some(Wait::Wave { channel }) => Record { mode: 10, channel: *channel, ..Default::default() },
        }
    }
    /// The wait for a record, keeping the label/channel fields of `base`.
    fn wait_of(&self, record: &Record, now: u64, m: &mut Machine) -> Result<Option<Wait>> {
        let after = |ms: i32| now.saturating_add(ms.max(0) as u64);
        Ok(match record.mode {
            0 => None,
            1 => {
                m.result(0, Value::Int(record.p1));
                None
            }
            2 => {
                ensure!(!record.text.is_empty(), "QLIE msd stop mode without a stop label");
                Some(Wait::Stop(StopWait {
                    label: record.text.clone(),
                    deadline: (record.p1 > 0).then(|| after(record.p1)),
                }))
            }
            3 => (record.p1 > 0).then(|| Wait::FixedClock(after(record.p1))),
            4 => Some(Wait::Clock(if record.p1 < 0 { u64::MAX / 2 } else { after(record.p1) })),
            5 => Some(Wait::Time {
                deadline: self.script_clock[self.clock_slot()].saturating_add(record.p1.max(0) as u64),
                label: record.label.clone(),
            }),
            6 => Some(Wait::Sound { channel: record.channel, until: record.p1, label: record.label.clone() }),
            7 | 8 => Some(Wait::Message),
            9 => Some(Wait::Button { deadline: (record.p1 > 0).then(|| after(record.p1)) }),
            10 => Some(Wait::Wave { channel: record.channel }),
            mode => bail!("QLIE msd wait mode {mode} is not a native mode"),
        })
    }
    fn frame_record(&self, m: &Machine, index: usize, now: u64) -> Record {
        if index == m.stack.len() {
            return self.record_of(self.wait.as_ref(), now);
        }
        if let Some(record) = self.frame_waits.get(&index) {
            return record.clone();
        }
        if let Some((depth, stop)) = &self.stop_poll {
            if *depth == index {
                return self.record_of(Some(&Wait::Stop(stop.clone())), now);
            }
        }
        if let Some(event) = self.frame_event.as_ref().filter(|e| e.depth == index) {
            return self.record_of(event.wait.as_ref(), now);
        }
        Record::default()
    }
    /// sys `?msd,depth` (6D9AF6) and `msd,depth,mode,p1,p2` (6DAB78); depth
    /// 0 is the running frame. Depths outside the stack are ignored.
    pub(super) fn frame_wait_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<()> {
        let n = |i: usize| -> Result<i32> {
            a.get(i).map(Value::integer).transpose().map(|v| v.unwrap_or(0))
        };
        let depth = n(0)?;
        let frames = m.stack.len() as i32 + 1;
        if depth < 0 || depth >= frames {
            return Ok(());
        }
        let index = (frames - 1 - depth) as usize;
        let now = h.now_ms();
        let mut record = self.frame_record(m, index, now);
        if op == "?msd" {
            m.result(0, Value::Int(i32::from(record.mode)));
            m.result(1, Value::Int(record.p1));
            m.result(2, Value::Int(record.p2));
            m.result_text(0, record.text);
            return Ok(());
        }
        ensure!(a.len() == 4, "QLIE msd ABI");
        record.mode = n(1)? as u8;
        record.p1 = n(2)?;
        record.p2 = n(3)?;
        if index == m.stack.len() {
            self.wait = self.wait_of(&record, now, m)?;
            self.stop_poll = None;
            return Ok(());
        }
        // Only a valid record is stored; it applies when the frame runs.
        ensure!(record.mode <= 10, "QLIE msd wait mode {} is not a native mode", record.mode);
        ensure!(record.mode != 2 || !record.text.is_empty(), "QLIE msd stop mode without a stop label");
        if self.stop_poll.as_ref().is_some_and(|(d, _)| *d == index) {
            self.stop_poll = None;
        }
        if let Some(event) = self.frame_event.as_mut().filter(|e| e.depth == index) {
            event.wait = None;
            event.poll = None;
        }
        ensure!(self.frame_waits.len() < 256, "QLIE frame wait budget");
        self.frame_waits.insert(index, record);
        Ok(())
    }
    /// Applies a stored record when its frame becomes the running frame.
    pub(super) fn apply_frame_wait(&mut self, h: &mut dyn HostServices, m: &mut Machine) -> Result<()> {
        if self.frame_waits.is_empty() {
            return Ok(());
        }
        let current = m.stack.len();
        self.frame_waits.retain(|k, _| *k <= current);
        if let Some(record) = self.frame_waits.remove(&current) {
            self.wait = self.wait_of(&record, h.now_ms(), m)?;
        }
        Ok(())
    }
}
