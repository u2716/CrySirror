//! Host diagnostic checkpoints preserve the live root and worker contexts.
//! They are distinct from menu saves, which resume a parked ADV command.
use super::*;

#[derive(Clone, Debug)]
pub(super) struct HostCheckpoint {
    pub clock: u32,
    pub wait: Option<WaitRequest>,
    thread: ThreadSubsystemState,
    run: RunPipelineState,
}
impl ScriptRuntime {
    pub(super) fn capture_host_state(&self) -> anyhow::Result<HostCheckpoint> {
        anyhow::ensure!(
            !matches!(self.wait_task_kind, Some(WaitRequest::SpriteFx(_)))
                && !matches!(
                    self.thread_state.wait.map(|w| w.request),
                    Some(WaitRequest::SpriteFx(_))
                ),
            "checkpoint must be taken after the sprite effect wait"
        );
        anyhow::ensure!(
            !self.movie_active(),
            "checkpoint must be taken outside a full movie"
        );
        Ok(HostCheckpoint {
            clock: self.pal_time_ms,
            wait: self.wait_task_kind,
            thread: self.thread_state.clone(),
            run: self.run_pipeline,
        })
    }
    pub(super) fn restore_host_state(&mut self, state: &HostCheckpoint) {
        self.pal_time_ms = state.clock;
        self.thread_state = state.thread.clone();
        self.thread_state.ticking = false;
        self.run_pipeline = state.run;
        self.status = match state.wait {
            Some(WaitRequest::Click)
            | Some(WaitRequest::ClickOrTime(_))
            | Some(WaitRequest::AutoClickOrTime(_)) => RuntimeStatus::WaitClick { pc: self.pc },
            Some(_) => RuntimeStatus::WaitFrame { pc: self.pc },
            None => RuntimeStatus::Running { pc: self.pc },
        };
    }
}
pub(super) fn write_wait(out: &mut Vec<u8>, wait: Option<WaitRequest>) -> std::io::Result<()> {
    let (kind, arg) = match wait {
        None => (0, 0),
        Some(WaitRequest::Frame(v)) => (1, v as u32),
        Some(WaitRequest::Time(v)) => (2, v),
        Some(WaitRequest::Click) => (3, 0),
        Some(WaitRequest::ClickOrTime(v)) => (4, v),
        Some(WaitRequest::AutoClickOrTime(v)) => (5, v),
        Some(WaitRequest::TextReveal(v)) => (6, v),
        Some(WaitRequest::SpriteFx(_)) => return Err(invalid()),
    };
    write_u32(out, kind)?;
    write_u32(out, arg)
}
fn read_wait(reader: &mut impl Read) -> std::io::Result<Option<WaitRequest>> {
    let kind = read_u32_from(reader)?;
    let arg = read_u32_from(reader)?;
    Ok(match kind {
        0 => None,
        1 => Some(WaitRequest::Frame(arg as i32)),
        2 => Some(WaitRequest::Time(arg)),
        3 => Some(WaitRequest::Click),
        4 => Some(WaitRequest::ClickOrTime(arg)),
        5 => Some(WaitRequest::AutoClickOrTime(arg)),
        6 => Some(WaitRequest::TextReveal(arg)),
        _ => return Err(invalid()),
    })
}
fn invalid() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid host checkpoint")
}
pub(super) fn write(out: &mut Vec<u8>, saved: Option<&HostCheckpoint>) -> std::io::Result<()> {
    write_u32(out, u32::from(saved.is_some()))?;
    let Some(s) = saved else { return Ok(()) };
    write_u32(out, s.clock)?;
    write_wait(out, s.wait)?;
    let t = &s.thread;
    for value in [
        t.next_id,
        t.last_id,
        t.running,
        t.target_point,
        t.argument_base,
    ] {
        write_i32(out, value)?;
    }
    write_u32(out, t.target_pc.unwrap_or(u32::MAX))?;
    write_i32_vec(out, &t.vars)?;
    write_i32_vec(out, &t.stack)?;
    write_i32_vec(out, &t.argument_stack)?;
    write_u32_vec(out, &t.call_stack)?;
    write_u32(out, t.active.len() as u32)?;
    for (&id, &active) in &t.active {
        write_i32(out, id)?;
        write_u32(out, u32::from(active))?;
    }
    write_wait(out, t.wait.map(|w| w.request))?;
    if let Some(w) = t.wait {
        write_u32(out, w.started_ms)?;
        write_i32(out, w.remaining_frames)?;
    }
    let r = &s.run;
    for word in [
        i32::from(r.run_stack_enabled),
        r.pending_kind,
        r.pending_effect_id,
        r.pending_arg1,
        r.pending_arg2,
        i32::from(r.effect_active),
        i32::from(r.no_wait_latch),
        r.last_run_time_ms as i32,
        r.last_run_arg1,
    ] {
        write_i32(out, word)?;
    }
    Ok(())
}
pub(super) fn read(reader: &mut impl Read) -> std::io::Result<Option<HostCheckpoint>> {
    if read_u32_from(reader)? == 0 {
        return Ok(None);
    }
    let clock = read_u32_from(reader)?;
    let wait = read_wait(reader)?;
    let mut t = ThreadSubsystemState::default();
    t.next_id = read_i32_from(reader)?;
    t.last_id = read_i32_from(reader)?;
    t.running = read_i32_from(reader)?;
    t.target_point = read_i32_from(reader)?;
    t.argument_base = read_i32_from(reader)?;
    let pc = read_u32_from(reader)?;
    t.target_pc = (pc != u32::MAX).then_some(pc);
    t.vars = read_i32_vec_capped(reader, DEFAULT_VAR_COUNT)?;
    t.stack = read_i32_vec_capped(reader, DEFAULT_STACK_LIMIT)?;
    t.argument_stack = read_i32_vec_capped(reader, DEFAULT_STACK_LIMIT)?;
    t.call_stack = read_u32_vec_capped(reader, DEFAULT_STACK_LIMIT)?;
    let count = read_u32_from(reader)?;
    if count > 4096 {
        return Err(invalid());
    }
    for _ in 0..count {
        let id = read_i32_from(reader)?;
        let active = read_u32_from(reader)? != 0;
        t.active.insert(id, active);
    }
    if let Some(request) = read_wait(reader)? {
        t.wait = Some(ThreadWaitState {
            request,
            started_ms: read_u32_from(reader)?,
            remaining_frames: read_i32_from(reader)?,
        });
    }
    let run = RunPipelineState {
        run_stack_enabled: read_i32_from(reader)? != 0,
        pending_kind: read_i32_from(reader)?,
        pending_effect_id: read_i32_from(reader)?,
        pending_arg1: read_i32_from(reader)?,
        pending_arg2: read_i32_from(reader)?,
        effect_active: read_i32_from(reader)? != 0,
        no_wait_latch: read_i32_from(reader)? != 0,
        last_run_time_ms: read_i32_from(reader)? as u32,
        last_run_arg1: read_i32_from(reader)?,
    };
    Ok(Some(HostCheckpoint {
        clock,
        wait,
        thread: t,
        run,
    }))
}
