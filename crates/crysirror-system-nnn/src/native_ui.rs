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
pub(crate) mod special;
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
        ensure!(self.order > i32::MIN, "NNN selection order underflow");
        let last = self
            .layer_base
            .checked_add(386)
            .context("NNN selection layer IDs")?;
        ensure!(
            !(self.layer_base..last).contains(&self.root),
            "NNN selection root/layer collision"
        );
        self.widget_base
            .checked_add(386)
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
    background_input: Option<crysirror_core::image_processing::ImageInput>,
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
        time_limit: _,
        special,
        ..
    } = &call.payload
    else {
        anyhow::bail!("NNN selection callback payload")
    };
    let _ = special; // Active special layouts are checked by the owner before admission.
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
    prepare_records_with_layout(strings, choices, messages, profile, metrics, None)
}
fn prepare_records_with_layout(
    strings: &[Vec<u8>],
    choices: usize,
    messages: usize,
    profile: &SelectionProfile,
    metrics: &dyn TextMeasurement,
    special: Option<&special::Layout>,
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
        let mut origin = if n < messages {
            profile.message_origins[n]
        } else {
            profile.rows[n - messages].text_origin
        };
        let face = special
            .map(|s| s.face(profile, n < messages))
            .unwrap_or(&profile.face);
        if n >= messages {
            if let Some(special) = special {
                special.center(&mut origin, &text, face.size)?;
            }
        }
        let glyphs = positioned(&text, face, origin, metrics)?;
        layers.push(json!({"op":"put","layer":{"id":profile.layer_base+n as u32,"parent":profile.root,"rect":[0,0,profile.canvas[0],profile.canvas[1]],"order":[profile.order,n],"paint":{"kind":"glyph_run","font":face.font,"size":face.size,"color":face.color,"glyphs":glyphs}}}));
        if n >= messages {
            let i = n - messages;
            ui.push(json!({"op":"put","widget":{"id":profile.widget_base+i as u32,"kind":"button","rect":profile.rows[i].hit_rect,"color":[0,0,0,0],"text":"","font":profile.face.font,"interaction_only":true,"keyboard_focusable":true}}));
        }
    }
    Ok(SelectionPlan {
        batch: json!({"layers":layers,"ui":ui}),
        choices,
        messages,
        background_input: None,
    })
}
type BackgroundLoader = Arc<
    dyn Fn(&crysirror_tasks::Cancellation) -> Result<crysirror_core::image_processing::ImageInput>
        + Send
        + Sync,
>;
pub type SelectionProfileFactory =
    Arc<dyn Fn(usize, usize, i32) -> Result<SelectionProfile> + Send + Sync>;
#[derive(Clone)]
enum NativeBackground {
    Solid([u8; 3]),
    Image { asset: String, size: [u32; 2] },
}
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
    timer: Option<(i32, i32)>,
    timer_at: Option<u64>,
    timer_steps: u32,
    controls: (i32, i32, i32),
    special_layouts: bool,
    special_policy: Option<Arc<special::Policy>>,
    special_layout: Option<special::Layout>,
    special_cursor: special::Presenter,
    special_last: Option<Value>,
    back_print_mode: i32,
    native_rows: Vec<Vec<u8>>,
    background: Option<NativeBackground>,
    background_name: Vec<u8>,
    background_loader: Option<BackgroundLoader>,
    background_job: Option<crysirror_core::image_processing::ImageJob>,
}
impl NativeSelectionBridge {
    pub(crate) fn resume_after_menu(&mut self, now: u64) {
        self.timer_at = Some(now);
    }
    pub(crate) fn set_frame_steps(&mut self, steps: u32) {
        self.timer_steps = steps;
    }
    fn set_timer(&mut self, limit: i32, automatic: i32) -> Result<()> {
        ensure!(
            (0..=86_400_000).contains(&limit) && (0..=u16::MAX as i32).contains(&automatic),
            "NNN selection timer/CASE bounds"
        );
        self.controls.0 = limit;
        self.controls.1 = automatic;
        self.timer = (limit > 0).then_some((limit, automatic));
        self.timer_at = None;
        Ok(())
    }
    pub(crate) fn original_controls(&self) -> (i32, i32, i32) {
        self.controls
    }
    pub fn dependency_pending(&self) -> bool {
        self.worker.is_some() || self.background_job.is_some() || self.special_cursor.pending()
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
            timer: None,
            timer_at: None,
            timer_steps: 1,
            controls: (0, 0, 0),
            special_layouts: false,
            special_policy: None,
            special_layout: None,
            special_cursor: Default::default(),
            special_last: None,
            back_print_mode: 0,
            native_rows: Vec::new(),
            background: None,
            background_name: Vec::new(),
            background_loader: None,
            background_job: None,
        })
    }
    pub(crate) fn configure_native_background(
        &mut self,
        setup: &crate::config::Names,
    ) -> Result<()> {
        ensure!(
            self.pending.is_none() && self.live.is_none(),
            "NNN selection background change during native actor"
        );
        self.special_layouts = setup.integer(b"useSpecialNumber")?.unwrap_or(0) > 0;
        self.back_print_mode = setup.integer(b"backPrintMode")?.unwrap_or(0);
        ensure!(
            (0..=2).contains(&self.back_print_mode),
            "NNN unknown selection native background mode"
        );
        self.background_name = setup
            .parameter(b"backFileName")
            .unwrap_or(b"sel_bg")
            .to_vec();
        self.background = if self.back_print_mode == 0 {
            Some(NativeBackground::Solid([
                setup.integer(b"backColorR")?.unwrap_or(0) as u8,
                setup.integer(b"backColorG")?.unwrap_or(0) as u8,
                setup.integer(b"backColorB")?.unwrap_or(0) as u8,
            ]))
        } else {
            None
        };
        Ok(())
    }
    pub(crate) fn configure_special(&mut self, policy: Arc<special::Policy>) -> Result<()> {
        ensure!(
            self.pending.is_none() && self.live.is_none(),
            "NNN special selection policy change during an active actor"
        );
        self.special_policy = Some(policy);
        Ok(())
    }
    pub(crate) fn refresh_special(
        &mut self,
        variables: &crate::expression::Variables,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        if self.live.is_none() {
            return Ok(true);
        }
        let Some(layout) = self.special_layout.as_mut() else {
            return Ok(true);
        };
        let layers = layout.windows(&self.profile, variables)?;
        let batch = json!({"layers":layers});
        if self.special_last.as_ref() != Some(&batch) {
            host.submit_engine(self.profile.client, 1, &serde_json::to_vec(&batch)?)?;
            self.api_calls += 1;
            self.special_last = Some(batch);
        }
        self.special_cursor.poll(layout, &self.profile, host)
    }
    pub(crate) fn configure_background_resources(
        &mut self,
        resources: crate::resource_bridge::NativeResources,
        names: crate::native_resources::ResourceNames,
        levels: u32,
    ) -> Result<()> {
        if self.back_print_mode != 1 {
            return Ok(());
        }
        ensure!(
            resources.client() == self.profile.client,
            "NNN selection background scope"
        );
        let mut name = b"sys/".to_vec();
        name.extend(&self.background_name);
        let lookup = names.picture(&name, levels)?;
        self.background_loader = Some(Arc::new(move |cancel| {
            cancel.check()?;
            Ok(resources.image_source(&lookup, cancel)?.input)
        }));
        Ok(())
    }
    fn poll_background(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        use crysirror_core::image_processing::ImageProcessState;
        if self.background_job.is_none() {
            let input = self
                .ready
                .as_mut()
                .and_then(|plan| plan.background_input.take());
            if let Some(input) = input {
                self.background_job = Some(
                    host.image_metadata(self.profile.client)?.begin_process(
                        input,
                        serde_json::to_vec(&[
                            crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                        ])?
                        .into(),
                    )?,
                );
                self.api_calls += 1;
            }
        }
        let Some(job) = self.background_job.as_mut() else {
            return Ok(true);
        };
        match job.poll()? {
            ImageProcessState::Pending => Ok(false),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("NNN original selection background: {error}")
            }
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.profile.client,
                    "NNN native selection background resource scope"
                );
                let size = image.size();
                let published = host.publish_image_resource(
                    self.profile.client,
                    self.profile.layer_base + 250,
                    image,
                )?;
                self.api_calls += 1;
                if let Some(NativeBackground::Image { asset, .. }) =
                    self.background.replace(NativeBackground::Image {
                        asset: published.path.relative().to_owned(),
                        size,
                    })
                {
                    host.release_image_resource(self.profile.client, &asset)?;
                    self.api_calls += 1;
                }
                self.background_job = None;
                Ok(true)
            }
        }
    }
    fn append_background(&self, batch: &mut Value) -> Result<()> {
        let paint = match &self.background {
            Some(NativeBackground::Solid([r, g, b])) => json!({"kind":"solid","color":[r,g,b,255]}),
            Some(NativeBackground::Image { asset, .. }) => json!({"kind":"image","asset":asset}),
            None => {
                ensure!(
                    self.back_print_mode != 1,
                    "NNN original selection background resource not prepared"
                );
                return Ok(());
            }
        };
        let size = match &self.background {
            Some(NativeBackground::Image { size, .. }) => *size,
            _ => self.profile.canvas,
        };
        let id = self
            .profile
            .layer_base
            .checked_add(386)
            .context("NNN selection background layer ID")?;
        ensure!(
            id != self.profile.root,
            "NNN selection background/root identity collision"
        );
        let order = self
            .profile
            .order
            .checked_sub(1)
            .context("NNN selection background ordering")?;
        batch["layers"].as_array_mut().context("NNN selection layer batch")?.insert(0,json!({"op":"put","layer":{"id":id,"parent":self.profile.root,"rect":[0,0,size[0],size[1]],"order":[order,0],"paint":paint}}));
        Ok(())
    }
    pub(crate) fn advances_effects(&self) -> bool {
        self.back_print_mode == 2
    }
    pub(crate) fn pending_selection(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn published_selection(&self) -> bool {
        self.live.is_some()
    }
    pub(crate) fn original_rows(&self) -> Result<(Vec<Vec<u8>>, usize, usize)> {
        let (choices, messages) = self.live.context("NNN save selection is not published")?;
        ensure!(
            self.native_rows.len() == choices + messages,
            "NNN original selection row ownership"
        );
        Ok((self.native_rows.clone(), choices, messages))
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
                    choices,
                    messages,
                    special,
                    ..
                } = &call.payload
                else {
                    anyhow::bail!("NNN dynamic selection payload")
                };
                let profile = factory(*choices as usize, *messages as usize, *special)?;
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
            if let Payload::Selection {
                time_limit,
                auto_select,
                special,
                choices,
                ..
            } = &call.payload
            {
                // CheckSpecial is false when the game has no special rows,
                // even if the script retains a nonzero special flag.
                ensure!(
                    *special == 0 || !self.special_layouts || self.special_policy.is_some(),
                    "NNN active special selection layout needs its native resources"
                );
                self.controls.2 = *special;
                self.special_layout = if *special != 0 && self.special_layouts {
                    Some(
                        self.special_policy
                            .as_ref()
                            .unwrap()
                            .layout(&mut self.profile, *choices as usize)?,
                    )
                } else {
                    None
                };
                self.special_last = None;
                let limit = if *time_limit < 0 {
                    machine
                        .variables()
                        .context("NNN timed choice variable bank absent")?
                        .read_calculated(u16::try_from(
                            time_limit
                                .checked_neg()
                                .context("NNN timer variable overflow")?,
                        )?)?
                        .max(0)
                } else {
                    *time_limit
                };
                self.set_timer(limit, *auto_select)?;
            }
            let metrics = host.text_measurement(self.profile.client)?;
            let permit = self.pool.reserve()?;
            if let Payload::Selection { strings, .. } = &call.payload {
                self.native_rows = strings
                    .iter()
                    .map(|row| row[..row.len().min(254)].to_vec())
                    .collect();
            }
            let background_loader = if self.back_print_mode == 1 && self.background.is_none() {
                Some(
                    self.background_loader
                        .clone()
                        .context("NNN selection background reader absent")?,
                )
            } else {
                None
            };
            let record = call.clone();
            let profile = self.profile.clone();
            let layout = self.special_layout.clone();
            let variables = machine.native_variables_candidate(call)?;
            self.worker = Some(permit.spawn(move |cancel| {
                cancel.check()?;
                let Payload::Selection {
                    choices,
                    messages,
                    strings,
                    ..
                } = &record.payload
                else {
                    anyhow::bail!("NNN selection payload");
                };
                let mut plan = prepare_records_with_layout(
                    strings,
                    *choices as usize,
                    *messages as usize,
                    &profile,
                    metrics.as_ref(),
                    layout.as_ref(),
                )?;
                if let Some(layout) = &layout {
                    plan.batch["layers"]
                        .as_array_mut()
                        .unwrap()
                        .extend(layout.windows(&profile, &variables)?);
                }
                if let Some(loader) = background_loader {
                    plan.background_input = Some(loader(&cancel)?);
                }
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
        if self.ready.is_some() && !self.poll_background(host)? {
            return Ok(false);
        }
        if let Some(plan) = &self.ready {
            // The native selection is the exclusive UI consumer for this
            // client until it returns. Clear earlier native-mode events before
            // creating this new widget generation, never after an activation.
            let _ = host.query_engine(self.profile.client, EngineQuery::UiEvents)?;
            let mut batch = plan.batch.clone();
            self.append_background(&mut batch)?;
            host.submit_engine(self.profile.client, 1, &serde_json::to_vec(&batch)?)?;
            self.api_calls += 1;
            self.live = Some((plan.choices, plan.messages));
            self.timer_at = Some(host.now_ms());
            self.ready = None;
            self.selected = None;
            return Ok(false);
        }
        if !self.refresh_special(&machine.native_variables_candidate(call)?, host)? {
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
        if self.selected.is_none() && self.timer_at != Some(host.now_ms()) {
            self.timer_at = Some(host.now_ms());
            if let Some((remaining, automatic)) = self.timer.as_mut() {
                *remaining = remaining.saturating_sub(i32::try_from(self.timer_steps)?);
                if *remaining <= 0 {
                    self.selected = Some(*automatic as usize);
                }
            }
        }
        Ok(self.selected)
    }
    fn retire_selection(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let (choices, messages) = self.live.context("NNN selection UI not published")?;
        let mut layers = (0..choices + messages)
            .map(|n| json!({"op":"remove","id":self.profile.layer_base+n as u32}))
            .collect::<Vec<_>>();
        if self.background.is_some() {
            layers.push(json!({"op":"remove","id":self.profile.layer_base+250}));
        }
        if let Some(layout) = &self.special_layout {
            layers.extend(
                layout
                    .node_ids(&self.profile)
                    .into_iter()
                    .map(|id| json!({"op":"remove","id":id})),
            );
        }
        let ui = (0..choices)
            .map(|n| json!({"op":"remove","id":self.profile.widget_base+n as u32}))
            .collect::<Vec<_>>();
        host.submit_engine(
            self.profile.client,
            1,
            &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?,
        )?;
        self.api_calls += 1;
        self.special_cursor.release(&self.profile, host)?;
        Ok(())
    }
    pub(crate) fn restored_node_ids(&self) -> (Vec<u32>, Vec<u32>) {
        let Some((choices, messages)) = self.live else {
            return (vec![], vec![]);
        };
        (
            (0..choices + messages)
                .map(|n| self.profile.layer_base + n as u32)
                .chain(
                    self.background
                        .as_ref()
                        .map(|_| self.profile.layer_base + 250),
                )
                .chain(
                    self.special_layout
                        .as_ref()
                        .into_iter()
                        .flat_map(|l| l.node_ids(&self.profile)),
                )
                .collect(),
            (0..choices)
                .map(|n| self.profile.widget_base + n as u32)
                .collect(),
        )
    }
    pub(crate) fn restore_seed(&self) -> Result<SelectionRestoreSeed> {
        ensure!(
            !self.dependency_pending() && self.ready.is_none(),
            "NNN selection restore requires drained dependencies"
        );
        Ok(SelectionRestoreSeed {
            profile: self.profile.clone(),
            factory: self.profile_factory.clone(),
            pool: self.pool.clone(),
            back_print_mode: self.back_print_mode,
            special_layouts: self.special_layouts,
            special_policy: self.special_policy.clone(),
            background: self.background.clone(),
            background_name: self.background_name.clone(),
            background_loader: self.background_loader.clone(),
        })
    }
    pub(crate) fn owned_special_image(&self) -> Option<&str> {
        self.special_cursor.path()
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
            self.timer_at = Some(host.now_ms());
            return Ok(false);
        }
        if !self.refresh_special(
            machine
                .variables()
                .context("NNN saved SELECT variables absent")?,
            host,
        )? {
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
    back_print_mode: i32,
    special_layouts: bool,
    special_policy: Option<Arc<special::Policy>>,
    background: Option<NativeBackground>,
    background_name: Vec<u8>,
    background_loader: Option<BackgroundLoader>,
}
pub(crate) struct SavedSelectionPreparation {
    bridge: NativeSelectionBridge,
}
pub(crate) struct PreparedSavedSelection {
    pub bridge: NativeSelectionBridge,
    pub batch: Value,
}
impl SelectionRestoreSeed {
    pub(crate) fn has_frozen_background(&self) -> bool {
        self.back_print_mode == 0 && self.background.is_some()
            || self.back_print_mode == 1
                && (self.background.is_some() || self.background_loader.is_some())
    }
    pub(crate) fn advances_effects(&self) -> bool {
        self.back_print_mode == 2
    }

    pub(crate) fn dormant(&self) -> Result<NativeSelectionBridge> {
        let mut bridge = NativeSelectionBridge::from_pool(self.profile.clone(), self.pool.clone())?;
        bridge.profile_factory = self.factory.clone();
        bridge.back_print_mode = self.back_print_mode;
        bridge.special_layouts = self.special_layouts;
        bridge.special_policy = self.special_policy.clone();
        bridge.background = self.background.clone();
        bridge.background_name = self.background_name.clone();
        bridge.background_loader = self.background_loader.clone();
        Ok(bridge)
    }

    pub(crate) fn begin(
        &self,
        strings: Vec<Vec<u8>>,
        total: usize,
        supplemental: usize,
        host: &mut dyn HostServices,
    ) -> Result<SavedSelectionPreparation> {
        self.begin_with_controls(
            strings,
            total,
            supplemental,
            (0, 0, 0),
            &crate::expression::Variables::fresh(crate::expression::VariableProfile::Legacy1000),
            host,
        )
    }
    pub(crate) fn begin_with_controls(
        &self,
        strings: Vec<Vec<u8>>,
        total: usize,
        supplemental: usize,
        controls: (i32, i32, i32),
        variables: &crate::expression::Variables,
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
        let mut profile = if let Some(factory) = &self.factory {
            factory(choices, supplemental, controls.2)?
        } else {
            self.profile.clone()
        };
        let layout = if controls.2 != 0 && self.special_layouts {
            Some(
                self.special_policy
                    .as_ref()
                    .context("NNN saved special selection resource policy absent")?
                    .layout(&mut profile, choices)?,
            )
        } else {
            None
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
        let background_loader = if self.back_print_mode == 1 && self.background.is_none() {
            Some(
                self.background_loader
                    .clone()
                    .context("NNN saved selection background reader absent")?,
            )
        } else {
            None
        };
        let original_rows = strings.clone();
        let worker_profile = profile.clone();
        let worker_layout = layout.clone();
        let variables = variables.clone();
        let permit = self.pool.reserve()?;
        let worker = permit.spawn(move |cancel| {
            cancel.check()?;
            let mut plan = prepare_records_with_layout(
                &strings,
                choices,
                supplemental,
                &worker_profile,
                metrics.as_ref(),
                worker_layout.as_ref(),
            )?;
            if let Some(layout) = &worker_layout {
                plan.batch["layers"]
                    .as_array_mut()
                    .unwrap()
                    .extend(layout.windows(&worker_profile, &variables)?);
            }
            if let Some(loader) = background_loader {
                plan.background_input = Some(loader(&cancel)?);
            }
            cancel.check()?;
            Ok(plan)
        })?;
        let mut bridge = NativeSelectionBridge::from_pool(profile, self.pool.clone())?;
        bridge.profile_factory = self.factory.clone();
        bridge.back_print_mode = self.back_print_mode;
        bridge.special_layouts = self.special_layouts;
        bridge.special_policy = self.special_policy.clone();
        bridge.background = self.background.clone();
        bridge.background_name = self.background_name.clone();
        bridge.background_loader = self.background_loader.clone();
        bridge.native_rows = original_rows;
        bridge.special_layout = layout;
        bridge.controls.2 = controls.2;
        bridge.set_timer(controls.0, controls.1)?;
        bridge.worker = Some(worker);
        Ok(SavedSelectionPreparation { bridge })
    }
}
impl SavedSelectionPreparation {
    pub(crate) fn set_controls(&mut self, limit: i32, automatic: i32, special: i32) -> Result<()> {
        ensure!(
            special == 0 || !self.bridge.special_layouts || self.bridge.special_layout.is_some(),
            "NNN saved active special selection requires native layout resources"
        );
        self.bridge.controls.2 = special;
        self.bridge.set_timer(limit, automatic)
    }
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
    pub(crate) fn poll_resources(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if !self.poll()? {
            return Ok(false);
        }
        self.bridge.poll_background(host)
    }
    pub(crate) fn finish(mut self) -> Result<PreparedSavedSelection> {
        ensure!(
            self.bridge.worker.is_none(),
            "NNN saved selection is not ready"
        );
        let mut plan = self
            .bridge
            .ready
            .take()
            .context("NNN saved selection plan absent")?;
        self.bridge.append_background(&mut plan.batch)?;
        self.bridge.live = Some((plan.choices, plan.messages));
        self.bridge.restored_wait_armed = false;
        Ok(PreparedSavedSelection {
            bridge: self.bridge,
            batch: plan.batch,
        })
    }
}
