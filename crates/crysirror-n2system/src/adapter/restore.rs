// SPDX-License-Identifier: MPL-2.0
//! ADR 0104 slot checkpoint, stage 2: restore. All fallible work (file decode,
//! validation, module preparation, layer/wait parsing) happens before the live
//! world is touched; the commit at the owner boundary then has no fallible
//! resource loading left. Images/sounds are re-acquired lazily from their
//! catalog paths like any new scene; sounds that were playing restart at the
//! saved position (the original engine's restart policy, not device replay).
use super::checkpoint::SlotCheckpoint;
use super::*;
use crysirror_scene::layer::LayerSpec;

pub(super) struct Restore {
    pub checkpoint: SlotCheckpoint,
    /// Modules named by the checkpoint that the live VM had not loaded.
    pub modules: Vec<(String, Module)>,
}
/// Worker half of `Load(n)`: read and validate the checkpoint, then prepare the
/// modules it names that the live VM has not loaded. No VM state is touched.
pub(super) fn load_slot(
    storage: Arc<dyn ClientStorage>,
    catalog: Arc<Catalog>,
    config: Config,
    existing: BTreeSet<String>,
    slot: u32,
    cancel: &crysirror_tasks::Cancellation,
) -> Result<Restore> {
    cancel.check()?;
    let path = VfsPath::new(Mount::Save, &format!("n2system-v1/slots/{slot:04}.json"))?;
    ensure!(
        storage.metadata(Mount::Save, path.relative())?.is_some(),
        "N2System save slot {slot} does not exist"
    );
    let checkpoint: SlotCheckpoint = serde_json::from_slice(&storage::read(storage.as_ref(), &path)?)
        .context("N2System slot checkpoint")?;
    checkpoint.validate()?;
    ensure!(checkpoint.slot == slot, "N2System slot checkpoint identity");
    let mut loaded = existing;
    let mut modules = Vec::new();
    for name in &checkpoint.modules {
        if loaded.contains(name) {
            continue;
        }
        for (n, m) in crate::resource::modules(
            storage.clone(),
            catalog.clone(),
            &config.script_root,
            name,
            &config.encoding,
            loaded.clone(),
            cancel,
        )? {
            loaded.insert(n.clone());
            modules.push((n, m));
        }
    }
    Ok(Restore { checkpoint, modules })
}
impl N2SystemVm {
    fn preflight_restore(&self, restore: &Restore, now: u64) -> Result<()> {
        let cp = &restore.checkpoint;
        cp.validate()?;
        ensure!(
            cp.entry == self.config.entry && cp.canvas == self.config.canvas,
            "N2System slot checkpoint belongs to a different entry/canvas"
        );
        let has = |module: &str, routine: &str| {
            restore
                .modules
                .iter()
                .find(|(n, _)| n == module)
                .map(|(_, m)| m)
                .or_else(|| self.machine.modules.get(module))
                .is_some_and(|m| m.routines.contains_key(routine))
        };
        for name in &cp.modules {
            ensure!(
                self.machine.modules.contains_key(name)
                    || restore.modules.iter().any(|(n, _)| n == name),
                "N2System slot checkpoint module {name} unavailable"
            );
        }
        for thread in &cp.threads {
            thread.wait.restore(now, self.frame)?;
            for frame in thread.thread.frames.iter().chain([&thread.entry]) {
                ensure!(
                    has(&frame.module, &frame.routine),
                    "N2System slot checkpoint routine {}::{} unavailable",
                    frame.module,
                    frame.routine
                );
            }
        }
        for e in &cp.entities {
            serde_json::from_value::<LayerSpec>(e.layer.clone())
                .with_context(|| format!("N2System checkpoint layer of {:?}", e.entity.name))?;
        }
        Ok(())
    }
    /// Owner-boundary commit of a prepared slot restore.
    pub(super) fn apply_restore(
        &mut self,
        host: &mut dyn HostServices,
        restore: Restore,
    ) -> Result<()> {
        let now = host.now_ms();
        self.preflight_restore(&restore, now)?;
        host.append_log(
            self.client,
            &format!("N2System: restoring slot {} checkpoint", restore.checkpoint.slot),
        )?;
        let Restore {
            checkpoint: cp,
            modules,
        } = restore;
        self.clear_native_world(host)?;
        for (_, state) in std::mem::take(&mut self.threads) {
            self.release_execution(host, state)?;
        }
        for (name, module) in modules {
            self.machine.insert(&name, module)?;
        }
        // Variables are assigned after module insertion, which seeds routine
        // parameter defaults.
        self.machine.variables = cp.variables;
        self.machine.flags = cp.flags;
        self.machine.arrays = cp.arrays;
        let nodes = cp.entities.iter().map(|e| e.entity.clone()).collect();
        self.entities.import(nodes, cp.aliases, cp.next_entity)?;
        // Parents before children, so every put names an existing ancestor.
        let mut order = Vec::with_capacity(cp.entities.len());
        let mut todo = cp
            .entities
            .iter()
            .filter(|e| e.entity.parent.is_none())
            .map(|e| e.entity.id)
            .collect::<Vec<_>>();
        let layers = cp
            .entities
            .iter()
            .map(|e| (e.entity.id, e))
            .collect::<BTreeMap<_, _>>();
        while let Some(id) = todo.pop() {
            order.push(id);
            todo.extend(
                cp.entities
                    .iter()
                    .filter(|e| e.entity.parent == Some(id))
                    .map(|e| e.entity.id),
            );
        }
        ensure!(
            order.len() == cp.entities.len(),
            "N2System slot checkpoint entity cycle"
        );
        let puts = order
            .iter()
            .map(|id| json!({"op":"put","layer":layers[id].layer}))
            .collect::<Vec<_>>();
        self.submit(host, puts, vec![])?;
        self.main = cp.main;
        for t in cp.threads {
            let mut execution = Execution::new(t.thread);
            execution.entry = t.entry;
            execution.wait = t.wait.restore(now, self.frame)?;
            execution.enabled = t.enabled;
            execution.page = t.page;
            execution.dialog = t.dialog;
            self.threads.insert(t.id, execution);
        }
        self.style = cp.style;
        self.text_dirty = cp.pages.keys().copied().collect();
        self.text_published.clear();
        self.text_jobs.clear();
        self.pages = cp.pages;
        self.history = cp.history;
        self.history_newline = false;
        self.history_revision = self
            .history_revision
            .checked_add(1)
            .context("backlog revision")?;
        self.read_marks = cp.read_marks;
        self.no_log = cp.no_log;
        self.disused = cp.disused;
        self.voices = cp.voices;
        self.choices = cp
            .choices
            .into_iter()
            .map(|(id, c)| {
                (
                    id,
                    choice::Choice {
                        // The shared widget is registered again by refresh_choices.
                        widget: false,
                        state: c.state,
                        activation: 0,
                        next: c.next,
                        disabled: c.disabled,
                    },
                )
            })
            .collect();
        for (id, s) in cp.sounds {
            // Handles and metadata are acquired again on first use.
            self.sound_resources.insert(
                id,
                media::SoundResource {
                    pan: s.pan,
                    rate: s.rate,
                    position_ms: s.position_ms,
                    loop_region: s.loop_region,
                    echo: s.echo,
                    stereo_effect: s.stereo_effect,
                    ..Default::default()
                },
            );
        }
        self.restore_sounds = self
            .entities
            .nodes
            .values()
            .filter(|e| matches!(e.kind, Kind::Sound { playing: true, .. }))
            .map(|e| e.id)
            .collect();
        Ok(())
    }
    /// Restart sounds that were playing when the slot was written, once their
    /// decoder metadata is ready.
    pub(super) fn update_restored_sounds(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let mut pending = Vec::new();
        for (ticket, paused) in std::mem::take(&mut self.restored_audio) {
            let info = host.audio_preparation(self.client,ticket)?;
            match info.phase {
                AudioPreparePhase::Pending => pending.push((ticket,paused)),
                AudioPreparePhase::Ready => {
                    self.connect_sound_effects(host,info.channel,paused)?;
                    host.release_audio(self.client,ticket)?;
                }
                AudioPreparePhase::Failed => {
                    host.release_audio(self.client,ticket)?;
                    bail!("N2System restored audio preparation: {}", info.error.unwrap_or_default());
                }
            }
        }
        self.restored_audio = pending;
        let mut scratch = None;
        for id in std::mem::take(&mut self.restore_sounds) {
            if !self.entities.nodes.contains_key(&id) {
                continue;
            }
            if !self.sound_ready(host, id)? {
                self.restore_sounds.push(id);
                continue;
            }
            let state = scratch.get_or_insert_with(|| {
                Execution::new(Thread {
                    frames: vec![vm::Frame {
                        module: String::new(),
                        routine: String::new(),
                        pc: 0,
                        base: 0,
                        result: false,
                    }],
                    ..Default::default()
                })
            });
            self.play_sound(host, state, id)?;
        }
        if let Some(mut state) = scratch {
            // Transfer preparation ownership to the VM before dropping the
            // temporary execution. Early release would cancel the new voices.
            self.restored_audio.extend(state.audio.drain(..).map(|ticket| (ticket,state.dsp_paused.remove(&ticket))));
            self.release_execution(host, state)?;
        }
        Ok(())
    }
}
