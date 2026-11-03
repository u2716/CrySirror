// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native selection wait/result policy over retained engine UI interactions.
//! No VM hit tester, event interpreter, font or widget renderer is embedded.
use crate::{
    Program,
    machine::{Machine, NativeCall, Payload},
    native_text::{TextFace, plain_cells, positioned},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    service::{EngineQuery, HostServices, TextMeasurement},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SelectionRow {
    pub text_origin: [i32; 2],
    pub hit_rect: [i32; 4],
}
/// Original setup/layout has to be resolved before admission. These are native
/// row/cursor positions, not a generic invented menu centering algorithm.
#[derive(Clone, Debug)]
pub struct SelectionProfile {
    pub client: ClientId,
    pub root: u32,
    pub widget_base: u32,
    pub layer_base: u32,
    pub order: i32,
    pub canvas: [u32; 2],
    pub face: TextFace,
    pub rows: Vec<SelectionRow>,
    pub message_origins: Vec<[i32; 2]>,
    pub single_byte_map: Option<Arc<[char]>>,
}
impl SelectionProfile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.client.0 != 0 && self.root != 0 && self.widget_base != 0 && self.layer_base != 0,
            "NNN selection identities"
        );
        ensure!(
            self.rows.len() <= 250
                && self.message_origins.len() <= 250
                && self.rows.len() + self.message_origins.len() <= 250,
            "NNN selection row budget"
        );
        ensure!(
            self.canvas.iter().all(|n| (1..=8192).contains(n)),
            "NNN selection canvas bounds"
        );
        let last = self
            .layer_base
            .checked_add(250)
            .context("NNN selection layer IDs")?;
        ensure!(
            !(self.layer_base..last).contains(&self.root),
            "NNN selection root/layer collision"
        );
        self.widget_base
            .checked_add(250)
            .context("NNN selection widget IDs")?;
        for row in &self.rows {
            ensure!(
                row.hit_rect[2] > 0
                    && row.hit_rect[3] > 0
                    && row.hit_rect.iter().all(|n| (-32767..=32767).contains(n)),
                "NNN selection hit rectangle"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
struct SelectionPlan {
    batch: Value,
    choices: usize,
    messages: usize,
}
fn prepare(
    call: &NativeCall,
    profile: &SelectionProfile,
    metrics: &dyn TextMeasurement,
) -> Result<SelectionPlan> {
    profile.validate()?;
    ensure!(
        call.identify == 0x66660006 && call.code == 0x22220006,
        "NNN selection callback identity"
    );
    let Payload::Selection {
        choices,
        messages,
        strings,
        time_limit,
        special,
        ..
    } = &call.payload
    else {
        anyhow::bail!("NNN selection callback payload")
    };
    ensure!(
        *special == 0,
        "NNN special selection layout needs native profile"
    );
    ensure!(
        *time_limit == 0,
        "NNN timed selection needs native timeout/variable/timer profile"
    );
    prepare_records(
        strings,
        *choices as usize,
        *messages as usize,
        profile,
        metrics,
    )
}
fn prepare_records(
    strings: &[Vec<u8>],
    choices: usize,
    messages: usize,
    profile: &SelectionProfile,
    metrics: &dyn TextMeasurement,
) -> Result<SelectionPlan> {
    profile.validate()?;
    ensure!(
        choices > 0
            && choices == profile.rows.len()
            && messages == profile.message_origins.len()
            && strings.len() == choices + messages,
        "NNN native selection/layout row mismatch"
    );
    let mut layers = Vec::new();
    let mut ui = Vec::new();
    for (n, raw) in strings.iter().enumerate() {
        // Original supplemental messages precede choices and skip one byte.
        let raw = if n < messages {
            raw.get(1..)
                .context("NNN supplemental message prefix absent")?
        } else {
            raw.as_slice()
        };
        let raw = &raw[..raw.len().min(254)];
        let text = plain_cells(raw, profile.single_byte_map.as_deref())?;
        let origin = if n < messages {
            profile.message_origins[n]
        } else {
            profile.rows[n - messages].text_origin
        };
        let glyphs = positioned(&text, &profile.face, origin, metrics)?;
        layers.push(json!({"op":"put","layer":{"id":profile.layer_base+n as u32,"parent":profile.root,"rect":[0,0,profile.canvas[0],profile.canvas[1]],"order":[profile.order,n],"paint":{"kind":"glyph_run","font":profile.face.font,"size":profile.face.size,"color":profile.face.color,"glyphs":glyphs}}}));
        if n >= messages {
            let i = n - messages;
            ui.push(json!({"op":"put","widget":{"id":profile.widget_base+i as u32,"kind":"button","rect":profile.rows[i].hit_rect,"color":[0,0,0,0],"text":"","font":profile.face.font,"interaction_only":true,"keyboard_focusable":true}}));
        }
    }
    Ok(SelectionPlan {
        batch: json!({"layers":layers,"ui":ui}),
        choices,
        messages,
    })
}
pub type SelectionProfileFactory =
    Arc<dyn Fn(usize, usize) -> Result<SelectionProfile> + Send + Sync>;
pub struct NativeSelectionBridge {
    pub profile: SelectionProfile,
    pub api_calls: u64,
    profile_factory: Option<SelectionProfileFactory>,
    pool: TaskPool,
    worker: Option<Task<SelectionPlan>>,
    ready: Option<SelectionPlan>,
    pending: Option<NativeCall>,
    live: Option<(usize, usize)>,
    selected: Option<usize>,
    restored_wait_armed: bool,
}
impl NativeSelectionBridge {
    pub fn dependency_pending(&self) -> bool {
        self.worker.is_some()
    }
    pub fn new(profile: SelectionProfile) -> Result<Self> {
        Self::from_pool(profile, TaskPool::new("nnn-native-selection", 1, 1)?)
    }
    fn from_pool(profile: SelectionProfile, pool: TaskPool) -> Result<Self> {
        profile.validate()?;
        Ok(Self {
            profile,
            api_calls: 0,
            profile_factory: None,
            pool,
            worker: None,
            ready: None,
            pending: None,
            live: None,
            selected: None,
            restored_wait_armed: false,
        })
    }
    /// Original scripts determine choice counts at the pending opcode. Keep
    /// immutable native setup until that boundary, rather than guessing a menu
    /// size while constructing an installed-game factory.
    pub fn new_dynamic(
        profile: SelectionProfile,
        factory: SelectionProfileFactory,
    ) -> Result<Self> {
        let mut bridge = Self::new(profile)?;
        bridge.profile_factory = Some(factory);
        Ok(bridge)
    }
    pub fn try_commit(
        &mut self,
        machine: &mut Machine,
        call: &NativeCall,
        program: &Program,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        machine.validate_receipt(call)?;
        if self.pending.is_none() {
            if let Some(factory) = &self.profile_factory {
                let Payload::Selection {
                    choices, messages, ..
                } = &call.payload
                else {
                    anyhow::bail!("NNN dynamic selection payload")
                };
                let profile = factory(*choices as usize, *messages as usize)?;
                profile.validate()?;
                ensure!(
                    profile.client == self.profile.client
                        && profile.root == self.profile.root
                        && profile.widget_base == self.profile.widget_base
                        && profile.layer_base == self.profile.layer_base,
                    "NNN dynamic selection changed capability bindings"
                );
                self.profile = profile;
            }
            ensure!(
                matches!(call.payload, Payload::Selection { .. }),
                "NNN selection requires native payload"
            );
            let metrics = host.text_measurement(self.profile.client)?;
            let permit = self.pool.reserve()?;
            let record = call.clone();
            let profile = self.profile.clone();
            self.worker = Some(permit.spawn(move |cancel| {
                cancel.check()?;
                let plan = prepare(&record, &profile, metrics.as_ref())?;
                cancel.check()?;
                Ok(plan)
            })?);
            self.pending = Some(call.clone());
            return Ok(false);
        }
        ensure!(
            self.pending.as_ref() == Some(call),
            "NNN stale selection receipt"
        );
        if let Some(worker) = &mut self.worker {
            let Some(plan) = worker.try_take()? else {
                return Ok(false);
            };
            self.worker = None;
            self.ready = Some(plan);
        }
        if let Some(plan) = &self.ready {
            // The native selection is the exclusive UI consumer for this
            // client until it returns. Clear earlier native-mode events before
            // creating this new widget generation, never after an activation.
            let _ = host.query_engine(self.profile.client, EngineQuery::UiEvents)?;
            host.submit_engine(self.profile.client, 1, &serde_json::to_vec(&plan.batch)?)?;
            self.api_calls += 1;
            self.live = Some((plan.choices, plan.messages));
            self.ready = None;
            self.selected = None;
            return Ok(false);
        }
        let Some(chosen) = self.poll_choice(host)? else {
            return Ok(false);
        };
        // Validate CASE destination before removing shared resources. A bad
        // original table cannot consume the UI then fail with its PC pending.
        let mut candidate = machine.clone();
        candidate.acknowledge_case(call, program, u16::try_from(chosen)?)?;
        self.retire_selection(host)?;
        *machine = candidate;
        self.live = None;
        self.pending = None;
        self.selected = None;
        Ok(true)
    }
    fn poll_choice(&mut self, host: &mut dyn HostServices) -> Result<Option<usize>> {
        let (choices, _) = self.live.context("NNN selection UI not published")?;
        if self.selected.is_none() {
            let events: Vec<Value> = serde_json::from_slice(
                &host.query_engine(self.profile.client, EngineQuery::UiEvents)?,
            )?;
            for event in events {
                if event["kind"] != "activate" {
                    continue;
                }
                let Some(id) = event["widget"]["id"].as_u64() else {
                    continue;
                };
                let start = u64::from(self.profile.widget_base);
                if (start..start + choices as u64).contains(&id) {
                    self.selected = Some((id - start) as usize + 1);
                    break;
                }
            }
        }
        Ok(self.selected)
    }
    fn retire_selection(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let (choices, messages) = self.live.context("NNN selection UI not published")?;
        let layers = (0..choices + messages)
            .map(|n| json!({"op":"remove","id":self.profile.layer_base+n as u32}))
            .collect::<Vec<_>>();
        let ui = (0..choices)
            .map(|n| json!({"op":"remove","id":self.profile.widget_base+n as u32}))
            .collect::<Vec<_>>();
        host.submit_engine(
            self.profile.client,
            1,
            &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?,
        )?;
        self.api_calls += 1;
        Ok(())
    }
    pub(crate) fn restored_node_ids(&self) -> (Vec<u32>, Vec<u32>) {
        let Some((choices, messages)) = self.live else {
            return (vec![], vec![]);
        };
        (
            (0..choices + messages)
                .map(|n| self.profile.layer_base + n as u32)
                .collect(),
            (0..choices)
                .map(|n| self.profile.widget_base + n as u32)
                .collect(),
        )
    }
    pub(crate) fn restore_seed(&self) -> Result<SelectionRestoreSeed> {
        ensure!(
            self.worker.is_none() && self.ready.is_none(),
            "NNN selection restore requires drained dependencies"
        );
        Ok(SelectionRestoreSeed {
            profile: self.profile.clone(),
            factory: self.profile_factory.clone(),
            pool: self.pool.clone(),
        })
    }
    /// A loaded SELECT has already consumed its opcode; only the saved CASE
    /// cursor is resumed after a real shared UI activation.
    pub(crate) fn poll_restored_input(
        &mut self,
        machine: &mut Machine,
        program: &Program,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        ensure!(
            self.pending.is_none() && self.worker.is_none() && self.ready.is_none(),
            "NNN saved selection dependencies are incomplete"
        );
        if !self.restored_wait_armed {
            let _ = host.query_engine(self.profile.client, EngineQuery::UiEvents)?;
            self.restored_wait_armed = true;
            return Ok(false);
        }
        let Some(chosen) = self.poll_choice(host)? else {
            return Ok(false);
        };
        let mut candidate = machine.clone();
        candidate.resume_saved_case(program, u16::try_from(chosen)?)?;
        self.retire_selection(host)?;
        *machine = candidate;
        self.live = None;
        self.selected = None;
        self.restored_wait_armed = false;
        Ok(true)
    }
}

#[derive(Clone)]
pub(crate) struct SelectionRestoreSeed {
    profile: SelectionProfile,
    factory: Option<SelectionProfileFactory>,
    pool: TaskPool,
}
pub(crate) struct SavedSelectionPreparation {
    bridge: NativeSelectionBridge,
}
pub(crate) struct PreparedSavedSelection {
    pub bridge: NativeSelectionBridge,
    pub batch: Value,
}
impl SelectionRestoreSeed {
    pub(crate) fn dormant(&self) -> Result<NativeSelectionBridge> {
        let mut bridge = NativeSelectionBridge::from_pool(self.profile.clone(), self.pool.clone())?;
        bridge.profile_factory = self.factory.clone();
        Ok(bridge)
    }

    pub(crate) fn begin(
        &self,
        strings: Vec<Vec<u8>>,
        total: usize,
        supplemental: usize,
        host: &mut dyn HostServices,
    ) -> Result<SavedSelectionPreparation> {
        // Original native word33 is total, word34 the supplemental prefix;
        // SetMessageKosuu(total - supplemental, supplemental) follows directly.
        ensure!(
            total <= 64
                && supplemental < total
                && strings.len() == total
                && strings
                    .iter()
                    .all(|raw| raw.len() <= 254 && !raw.contains(&0)),
            "NNN saved selection fixed rows"
        );
        let choices = total - supplemental;
        let profile = if let Some(factory) = &self.factory {
            factory(choices, supplemental)?
        } else {
            self.profile.clone()
        };
        profile.validate()?;
        ensure!(
            profile.client == self.profile.client
                && profile.root == self.profile.root
                && profile.widget_base == self.profile.widget_base
                && profile.layer_base == self.profile.layer_base,
            "NNN saved selection changed capability bindings"
        );
        let metrics = host.text_measurement(profile.client)?;
        let worker_profile = profile.clone();
        let permit = self.pool.reserve()?;
        let worker = permit.spawn(move |cancel| {
            cancel.check()?;
            let plan = prepare_records(
                &strings,
                choices,
                supplemental,
                &worker_profile,
                metrics.as_ref(),
            )?;
            cancel.check()?;
            Ok(plan)
        })?;
        let mut bridge = NativeSelectionBridge::from_pool(profile, self.pool.clone())?;
        bridge.profile_factory = self.factory.clone();
        bridge.worker = Some(worker);
        Ok(SavedSelectionPreparation { bridge })
    }
}
impl SavedSelectionPreparation {
    pub(crate) fn poll(&mut self) -> Result<bool> {
        if let Some(worker) = &mut self.bridge.worker {
            let Some(plan) = worker.try_take()? else {
                return Ok(false);
            };
            self.bridge.worker = None;
            self.bridge.ready = Some(plan);
        }
        ensure!(
            self.bridge.ready.is_some(),
            "NNN saved selection result already consumed"
        );
        Ok(true)
    }
    pub(crate) fn finish(mut self) -> Result<PreparedSavedSelection> {
        ensure!(
            self.bridge.worker.is_none(),
            "NNN saved selection is not ready"
        );
        let plan = self
            .bridge
            .ready
            .take()
            .context("NNN saved selection plan absent")?;
        self.bridge.live = Some((plan.choices, plan.messages));
        self.bridge.restored_wait_armed = false;
        Ok(PreparedSavedSelection {
            bridge: self.bridge,
            batch: plan.batch,
        })
    }
}
