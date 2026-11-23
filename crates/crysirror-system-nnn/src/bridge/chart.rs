// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Chart callbacks retain one native receipt while scoped checkpoint/system
//! writes complete. The VM context is immutable CPU data; no host enters a job.
use super::*;
use crate::{
    expression::Variables,
    native_bootstrap::NativeConfiguration,
    native_chart::{ChartCheckpoint, ChartRuntime, ChartSnapshotRequest, ChartState, ChartWrite},
    native_save::{LegacySaveProfile, LiveExecution, NativeSaveFile},
    native_system_file::{NativeSystemSave, SystemStamp},
};
use crysirror_core::service::{CivilTimeQuery, CivilTimeState, StorageWrite, StorageWriteState};
use crysirror_tasks::{Task, TaskPool};
use std::collections::VecDeque;
#[derive(Clone)]
pub(crate) struct ChartContext {
    pub call: Option<NativeCall>,
    pub configuration: Arc<NativeConfiguration>,
    pub execution: Arc<LiveExecution>,
    pub pool: TaskPool,
    pub loader: crate::adapter::ScriptLoader,
}
pub(super) struct ChartWait {
    pub call: Option<NativeCall>,
    pub context: ChartContext,
    pub requested: bool,
    returning: Option<ChartReturnWork>,
    new_flags: u32,
}
struct ChartReturnWork {
    slot: i32,
    variables: Variables,
    task: Option<Task<Option<(ChartCheckpoint, crate::native_control::ScriptSession)>>>,
    loaded: Option<(ChartCheckpoint, crate::native_control::ScriptSession)>,
}
pub(crate) enum ChartReturn {
    Pending,
    Continue,
    Restored(crate::native_control::ScriptSession),
}
pub(super) struct ChartOperation {
    call: NativeCall,
    runtime: ChartRuntime,
    state: ChartState,
    variables: Variables,
    context: Option<ChartContext>,
    snapshots: VecDeque<ChartSnapshotRequest>,
    civil: Option<Box<dyn CivilTimeQuery>>,
    encode: Option<Task<(String, Arc<[u8]>)>>,
    write: Option<Box<dyn StorageWrite>>,
    system: Option<NativeSystemSave>,
    system_required: bool,
    command: bool,
}
impl ChartOperation {
    fn dependency_pending(&self) -> bool {
        self.civil.is_some()
            || self.encode.is_some()
            || self.write.is_some()
            || self.system.is_some()
            || !self.snapshots.is_empty()
    }
}
impl Bridge {
    pub(crate) fn restore_chart_mode(&mut self, context: ChartContext) -> Result<()> {
        ensure!(
            self.chart_wait.is_none()
                && context.call.is_none()
                && self.chart_runtime.is_some()
                && self.auxiliary.chart.is_some(),
            "NNN saved chart mode boundary"
        );
        self.chart_wait = Some(ChartWait {
            call: None,
            context,
            requested: true,
            returning: None,
            new_flags: 0,
        });
        Ok(())
    }
    pub(crate) fn configure_chart_runtime(&mut self, runtime: Option<ChartRuntime>) -> Result<()> {
        ensure!(
            self.chart_runtime.is_none(),
            "NNN chart runtime already configured"
        );
        self.chart_runtime = runtime;
        Ok(())
    }
    fn chart_command(&self, call: &NativeCall) -> bool {
        let Some(runtime) = self.chart_runtime.as_ref() else {
            return false;
        };
        matches!(call.payload, Payload::UserCall { command: true, .. })
            && runtime.definition.default_command
            && runtime
                .definition
                .commands
                .tokens
                .get(call.code as usize * 2)
                .is_some_and(|n| n.eq_ignore_ascii_case(b"selectchart"))
    }
    pub(crate) fn chart_context_needed(&self, call: &NativeCall) -> Result<bool> {
        if self
            .chart_wait
            .as_ref()
            .is_some_and(|wait| wait.call.as_ref() == Some(call))
            || self.pending_chart.is_some()
            || self
                .chart_context
                .as_ref()
                .is_some_and(|ctx| ctx.call.as_ref() == Some(call))
        {
            return Ok(false);
        }
        if self.chart_command(call) {
            return Ok(true);
        }
        Ok(
            matches!(call.payload, Payload::UserCall { command: false, .. })
                && self.chart_runtime.as_ref().is_some_and(|runtime| {
                    runtime
                        .definition
                        .function(self.user_function_name(call).unwrap_or(&[]))
                        == Some(5)
                }),
        )
    }
    pub(crate) fn bind_chart_context(&mut self, context: ChartContext) -> Result<()> {
        ensure!(
            self.chart_context.is_none(),
            "NNN ChartMenu has another execution context"
        );
        self.chart_context = Some(context);
        Ok(())
    }
    pub(super) fn chart_dependency_pending(&self) -> bool {
        self.pending_chart
            .as_ref()
            .is_some_and(|pending| pending.dependency_pending())
            || self.pending_chart_progress.is_some()
    }
    pub(super) fn try_chart_operation(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<Option<bool>> {
        if self
            .chart_wait
            .as_ref()
            .is_some_and(|wait| wait.call.as_ref() == Some(call))
        {
            return Ok(Some(false));
        }
        if self.pending_chart.is_none() {
            let Some(runtime) = self.chart_runtime.as_ref() else {
                return Ok(None);
            };
            let command = self.chart_command(call);
            if !command && !matches!(call.payload, Payload::UserCall { command: false, .. }) {
                return Ok(None);
            }
            let name = if command {
                vec![]
            } else {
                self.user_function_name(call)?.to_vec()
            };
            if !command && runtime.definition.function(&name).is_none() {
                return Ok(None);
            }
            let Payload::UserCall { parameters, .. } = &call.payload else {
                unreachable!()
            };
            let mut runtime = runtime.clone();
            let mut state = self
                .auxiliary
                .chart
                .clone()
                .context("NNN ChartMenu selection/road actor absent")?;
            let mut variables = machine.native_variables_candidate(call)?;
            ensure!(
                variables.native_writes().is_empty(),
                "NNN ChartMenu native argument resource writes"
            );
            let (snapshots, save_system) = if command {
                (runtime.start(&mut state, &mut variables)?, false)
            } else {
                let effect = runtime
                    .apply(&mut state, &name, parameters, &mut variables)?
                    .unwrap();
                (effect.checkpoint.into_iter().collect(), effect.save_system)
            };
            let context = self.chart_context.take();
            if !snapshots.is_empty() {
                ensure!(
                    context
                        .as_ref()
                        .is_some_and(|ctx| ctx.call.as_ref() == Some(call)),
                    "NNN ChartMenu checkpoint has no original script context"
                );
            }
            self.pending_chart = Some(ChartOperation {
                call: call.clone(),
                runtime,
                state,
                variables,
                context,
                snapshots: snapshots.into(),
                civil: None,
                encode: None,
                write: None,
                system: None,
                system_required: save_system,
                command,
            });
        }
        let mut pending = self.pending_chart.take().unwrap();
        let result = self.poll_chart_operation(&mut pending, machine, call, host);
        match result {
            Ok(true) => Ok(Some(true)),
            Ok(false) => {
                if !pending.command
                    || !self
                        .chart_wait
                        .as_ref()
                        .is_some_and(|wait| wait.call.as_ref() == Some(call))
                {
                    self.pending_chart = Some(pending);
                }
                Ok(Some(false))
            }
            Err(error) => {
                self.pending_chart = Some(pending);
                Err(error)
            }
        }
    }
    fn poll_chart_operation(
        &mut self,
        pending: &mut ChartOperation,
        machine: &mut Machine,
        call: &NativeCall,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        ensure!(
            pending.call == *call,
            "NNN ChartMenu callback receipt changed"
        );
        machine.validate_receipt(call)?;
        if pending.system_required {
            if pending.system.is_none() {
                pending.system = Some(NativeSystemSave::begin(
                    self.client,
                    call,
                    pending.variables.clone(),
                    host,
                )?);
                self.api_calls += 1;
            }
            let work = pending.system.as_mut().unwrap();
            let before = work.api_calls;
            let result = work.poll_storage(
                self.system_file
                    .as_mut()
                    .context("NNN ChartMenu SystemFile absent")?,
                call,
                host,
            );
            self.api_calls += work.api_calls - before;
            if !result? || !self.poll_result_save(host)? {
                return Ok(false);
            }
            pending.system_required = false;
            pending.system = None;
        }
        if let Some(write) = pending.write.as_mut() {
            match write.poll()? {
                StorageWriteState::Pending => return Ok(false),
                StorageWriteState::Failed(e) => anyhow::bail!("NNN chart checkpoint write: {e}"),
                StorageWriteState::Ready => {
                    pending.write = None;
                }
            }
        }
        if let Some(encode) = pending.encode.as_mut() {
            let Some((path, bytes)) = encode.try_take()? else {
                return Ok(false);
            };
            pending.encode = None;
            pending.write = Some(host.storage(self.client)?.begin_write_atomic(
                &crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save, &path)?,
                bytes,
            )?);
            self.api_calls += 1;
            return Ok(false);
        }
        if let Some(request) = pending.snapshots.front().cloned() {
            if pending.civil.is_none() {
                pending.civil = Some(host.begin_civil_time(self.client)?);
                self.api_calls += 1;
            }
            let sample = match pending.civil.as_mut().unwrap().poll()? {
                CivilTimeState::Pending => return Ok(false),
                CivilTimeState::Failed(e) => anyhow::bail!("NNN chart checkpoint civil time: {e}"),
                CivilTimeState::Ready(sample) => sample,
            };
            pending.civil = None;
            let context = pending.context.as_ref().unwrap();
            let profile = LegacySaveProfile::for_configuration(&context.configuration)?;
            let mut file = NativeSaveFile::for_live(
                &profile,
                SystemStamp::from_civil(sample, self.client)?,
                crate::native_thumbnail::NativeThumbnail {
                    size: [1, 1],
                    bgrx: vec![0; 4],
                },
            )?;
            file.replace_live_execution(
                &context.execution.with_variables(&request.variables)?,
                34,
            )?;
            self.write_original_common_status(&mut file)?;
            let snapshot = ChartCheckpoint {
                status: file.blocks[2].clone(),
                variables: (0..1000)
                    .map(|i| request.variables.read_stored(i))
                    .collect(),
                state: request.state,
            };
            let path = snapshot.path(request.slot, pending.runtime.definition.block_size)?;
            pending.encode = Some(context.pool.spawn(move |cancel| {
                let bytes = snapshot.encode(&|| cancel.check().map_err(Into::into))?;
                Ok((path, Arc::from(bytes)))
            })?);
            pending.snapshots.pop_front();
            return Ok(false);
        }
        if pending.command {
            // The native command owns the General-mode wait until its actual
            // UI actor returns. Never ACK merely because its checkpoint exists.
            machine.update_native_variables(call, pending.variables.clone())?;
            self.chart_runtime = Some(pending.runtime.clone());
            self.auxiliary.chart = Some(pending.state.clone());
            let context = pending
                .context
                .take()
                .context("NNN ChartMenu mode execution context absent")?;
            let runtime = &pending.runtime;
            let mut new_flags = 0u32;
            if runtime
                .definition
                .setup
                .integer(b"newMarkPrintFlag")?
                .unwrap_or(0)
                != 0
            {
                let old = runtime
                    .definition
                    .setup
                    .parameter(b"newMarkVarName")
                    .or(Some(b"newchartflag".as_slice()))
                    .and_then(|name| context.configuration.variable_names.index(name))
                    .map(|id| pending.variables.read_stored(id) as u32)
                    .unwrap_or(u32::MAX);
                for group in 0..runtime.definition.maps.len().min(32) {
                    if old & (1 << group) != 0 || !runtime.open(group, &mut pending.variables)? {
                        continue;
                    }
                    for slot in &runtime.definition.maps[group].nodes {
                        if runtime.selectable(&pending.state, *slot, &mut pending.variables)?
                            && (runtime.seen(*slot)?
                                || runtime
                                    .definition
                                    .node(runtime.slot(&mut pending.variables)?)?
                                    .next
                                    .contains(slot))
                        {
                            new_flags |= 1 << group;
                            break;
                        }
                    }
                }
            }
            self.chart_wait = Some(ChartWait {
                call: Some(call.clone()),
                context,
                requested: true,
                returning: None,
                new_flags,
            });
            return Ok(false);
        }
        machine.acknowledge_with_variables(call, pending.variables.clone())?;
        self.chart_runtime = Some(pending.runtime.clone());
        self.auxiliary.chart = Some(pending.state.clone());
        Ok(true)
    }
    pub(crate) fn poll_chart_save(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let Some(runtime) = self.chart_runtime.as_mut() else {
            return Ok(true);
        };
        if self.pending_chart_progress.is_none() {
            if !runtime.dirty() {
                return Ok(true);
            }
            self.pending_chart_progress =
                Some(runtime.begin_save(host.storage(self.client)?.as_ref())?);
            self.api_calls += 2;
        }
        if self
            .pending_chart_progress
            .as_mut()
            .unwrap()
            .poll(runtime)?
        {
            self.pending_chart_progress = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub(crate) fn take_chart_mode_request(&mut self) -> bool {
        if let Some(wait) = self.chart_wait.as_mut() {
            let requested = wait.requested;
            wait.requested = false;
            requested
        } else {
            false
        }
    }
    pub(crate) fn chart_mode_active(&self) -> bool {
        self.chart_wait.is_some()
    }
    pub(crate) fn chart_view(&self) -> Result<(&ChartRuntime, &ChartState)> {
        Ok((
            self.chart_runtime
                .as_ref()
                .context("NNN ChartMenu runtime absent")?,
            self.auxiliary
                .chart
                .as_ref()
                .context("NNN ChartMenu selection/road state absent")?,
        ))
    }
    /// FinalExitRoutine: adjacent choices ACK the retained command. Revisiting
    /// a read node reconstructs the original saved reader instead; no synthetic
    /// call is acknowledged and persistent variables 200..299 remain current.
    pub(crate) fn poll_chart_return(
        &mut self,
        machine: &mut Machine,
        call: Option<&NativeCall>,
        slot: i32,
        host: &mut dyn HostServices,
    ) -> Result<ChartReturn> {
        let mut wait = self
            .chart_wait
            .take()
            .context("NNN ChartMenu return without its command")?;
        let result = (|| {
            ensure!(
                wait.call.as_ref() == call,
                "NNN ChartMenu return receipt changed"
            );
            if let Some(call) = call {
                machine.validate_receipt(call)?;
            }
            if wait.returning.is_none() {
                let runtime = self.chart_runtime.as_ref().unwrap();
                let state = self.auxiliary.chart.as_ref().unwrap();
                let mut variables = if let Some(call) = call {
                    machine.native_variables_candidate(call)?
                } else {
                    machine
                        .variables()
                        .context("NNN chart restored variable bank absent")?
                        .clone()
                };
                let from = runtime.slot(&mut variables)?;
                let direct = runtime.definition.node(from)?.next.contains(&slot);
                ensure!(
                    runtime.selectable(state, slot, &mut variables)?
                        && (direct || runtime.seen(slot)?),
                    "NNN ChartMenu selected unavailable node"
                );
                runtime.set_from(&mut variables, from)?;
                runtime.set_slot(&mut variables, slot)?;
                let task = if direct {
                    None
                } else {
                    let storage = host.storage(self.client)?;
                    let config = wait.context.configuration.clone();
                    let loader = wait.context.loader.clone();
                    let layout = state.layout;
                    let block = runtime.definition.block_size;
                    let current = variables.clone();
                    Some(wait.context.pool.spawn(move |cancel| {
                        cancel.check()?;
                        let path = crysirror_core::path::VfsPath::new(
                            crysirror_core::path::Mount::Save,
                            &format!("chart{}/slot{slot}.dat", slot as usize / block + 1),
                        )?;
                        let reader = match storage.open_read(&path) {
                            Ok(reader) => reader,
                            Err(crysirror_core::Error::NotFound) => return Ok(None),
                            Err(error) => return Err(error.into()),
                        };
                        let bytes = crysirror_resource::read_all(reader.as_ref(), 64 << 20)?;
                        let checkpoint = ChartCheckpoint::parse(&bytes, layout, &|| {
                            cancel.check().map_err(Into::into)
                        })?;
                        let session =
                            checkpoint.prepare_session(&config, &current, &loader, &cancel)?;
                        Ok(Some((checkpoint, session)))
                    })?)
                };
                wait.returning = Some(ChartReturnWork {
                    slot,
                    variables,
                    task,
                    loaded: None,
                });
            }
            let work = wait.returning.as_mut().unwrap();
            ensure!(
                work.slot == slot,
                "NNN ChartMenu changed choice during checkpoint I/O"
            );
            if let Some(task) = work.task.as_mut() {
                let Some(loaded) = task.try_take()? else {
                    return Ok(ChartReturn::Pending);
                };
                work.loaded = loaded;
                work.task = None;
            }
            if let Some((checkpoint, _)) = work.loaded.as_ref() {
                self.validate_native_status_block(&checkpoint.status)?;
                let audio = NativeSaveFile::status_audio_work(&checkpoint.status)?;
                audio.prepare_policy(
                    &self
                        .audio
                        .as_ref()
                        .context("NNN ChartMenu saved audio absent")?
                        .policy,
                )?;
                if !self.poll_outer_music(
                    host,
                    [
                        if audio.music_number < 0 {
                            -1
                        } else {
                            audio
                                .music_number
                                .checked_add(1)
                                .context("NNN chart saved music overflow")?
                        },
                        audio.music_repeat_count,
                        -9999,
                        -1,
                    ],
                    true,
                )? {
                    return Ok(ChartReturn::Pending);
                }
            }
            if let Some((checkpoint, mut session)) = work.loaded.take() {
                self.restore_native_status_block(&checkpoint.status)?;
                self.auxiliary.chart = Some(checkpoint.state);
                let audio = NativeSaveFile::status_audio_work(&checkpoint.status)?;
                let bridge = self
                    .audio
                    .as_mut()
                    .context("NNN ChartMenu saved audio absent")?;
                bridge.restore_native_work_records(audio)?;
                let mut variables = session
                    .machine()
                    .variables()
                    .context("NNN chart restored variables absent")?
                    .clone();
                self.chart_final_variables(
                    &mut variables,
                    slot,
                    &wait.context.configuration,
                    wait.new_flags,
                )?;
                session.machine_mut().replace_variables(variables)?;
                Ok(ChartReturn::Restored(session))
            } else {
                self.chart_final_variables(
                    &mut work.variables,
                    slot,
                    &wait.context.configuration,
                    wait.new_flags,
                )?;
                if let Some(call) = call {
                    machine.acknowledge_with_variables(call, work.variables.clone())?;
                } else {
                    machine.replace_variables(work.variables.clone())?;
                }
                Ok(ChartReturn::Continue)
            }
        })();
        if matches!(result, Ok(ChartReturn::Pending) | Err(_)) {
            self.chart_wait = Some(wait);
        }
        result
    }
    fn chart_final_variables(
        &self,
        variables: &mut Variables,
        slot: i32,
        config: &NativeConfiguration,
        new_flags: u32,
    ) -> Result<()> {
        let runtime = self.chart_runtime.as_ref().unwrap();
        let setup = &runtime.definition.setup;
        if setup.integer(b"newMarkPrintFlag")?.unwrap_or(0) != 0 {
            if let Some(id) = config.variable_names.index(
                setup
                    .parameter(b"newMarkVarName")
                    .unwrap_or(b"newchartflag"),
            ) {
                let flags = variables.read_stored(id) as u32;
                variables.write_native(
                    id,
                    (flags | (new_flags & (1u32 << (runtime.definition.chart(slot)? & 31)))) as i32,
                )?;
            }
        }
        Ok(())
    }
}
