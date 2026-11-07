//! Native cooperative readers (70A1D8/70A488). Reader-owned host state is
//! suspended along with the VM stack; shared resources remain engine owned.
use super::*;

pub(super) struct HostContext {
    wait: Option<Wait>,
    stop_poll: Option<(usize, StopWait)>,
    frame_event: Option<FrameEvent>,
    last_frame_event: Option<u64>,
    selection: Vec<String>,
    scope_next: usize,
    button_callbacks: std::collections::VecDeque<String>,
    skip_until: u64,
    capture: bool,
    second_capture: bool,
    capture_waiting: bool,
    effect_priority: i32,
    effect_clock: bool,
    rule_fade: Option<(String, f32, bool)>,
    animated_fade: Option<u64>,
    paused_at: u64,
}
impl HostContext {
    /// Reader waits only; capture/fade state names device canvases and is
    /// not part of a reader save.
    pub(super) fn to_saved(&self) -> super::save_state::SavedHost {
        super::save_state::SavedHost::new(
            self.wait.clone(),
            self.stop_poll.clone(),
            self.frame_event.clone(),
            self.selection.clone(),
            self.scope_next,
            self.button_callbacks.iter().cloned().collect(),
            self.skip_until,
            self.paused_at,
        )
    }
    pub(super) fn from_saved(saved: super::save_state::SavedHost, paused_at: u64) -> Self {
        let mut context = Self::empty(paused_at);
        let (wait, stop_poll, frame_event, selection, scope_next, buttons, skip_until) =
            saved.rebased(paused_at);
        context.wait = wait;
        context.stop_poll = stop_poll;
        context.frame_event = frame_event;
        context.selection = selection;
        context.scope_next = scope_next;
        context.button_callbacks = buttons.into();
        context.skip_until = skip_until;
        context
    }
    pub(super) fn empty(paused_at: u64) -> Self {
        Self {
            wait: None,
            stop_poll: None,
            frame_event: None,
            last_frame_event: None,
            selection: Vec::new(),
            scope_next: 0,
            button_callbacks: Default::default(),
            skip_until: 0,
            capture: false,
            second_capture: false,
            capture_waiting: false,
            effect_priority: 65535,
            effect_clock: false,
            rule_fade: None,
            animated_fade: None,
            paused_at,
        }
    }
    pub(super) fn exchange_graphic(&mut self, vm: &mut QlieVm) {
        std::mem::swap(&mut self.capture, &mut vm.capture);
        std::mem::swap(&mut self.second_capture, &mut vm.second_capture);
        std::mem::swap(&mut self.capture_waiting, &mut vm.capture_waiting);
        std::mem::swap(&mut self.effect_priority, &mut vm.effect_priority);
        std::mem::swap(&mut self.effect_clock, &mut vm.effect_clock);
        std::mem::swap(&mut self.rule_fade, &mut vm.rule_fade);
        std::mem::swap(&mut self.animated_fade, &mut vm.animated_fade);
    }
}
/// 6E3AB0 defaults callback routing to the system reader; only an explicit
/// `self` target executes inside the current reader's include scope.
pub(super) struct Callback {
    pub action: String,
    pub label: String,
    pub file: String,
    pub same_reader: bool,
}
impl Callback {
    pub(super) fn default_label(&self) -> Option<&'static str> {
        match self.action.as_str() {
            "config" => Some("@@!Config"),
            "configvolume" => Some("@@!ConfigVolume"),
            "configvoice" => Some("@@!ConfigVoice"),
            "save" => Some("@@!Save"),
            "load" => Some("@@!Load"),
            "log" | "backlog" => Some("@@!Log"),
            "menu" => Some("@@!Menu"),
            "reset" => Some("@@!Reset"),
            "exit" => Some("@@!ext"),
            "quicksave" => Some("@@!QuickSave"),
            "quickload" => Some("@@!QuickLoad"),
            "selectload" => Some("@@!SelectLoad"),
            _ => None,
        }
    }
    pub(super) fn lock_mask(&self) -> i32 {
        match self.action.as_str() {
            "save" | "quicksave" => 4,
            "load" | "quickload" | "selectload" => 8,
            "config" | "configvolume" | "configvoice" => 16,
            "log" | "backlog" => 32,
            "view" => 64,
            "auto" => 128,
            "skip" | "logskip" => 256,
            "jump" | "scenereturn" => 512,
            "menu" => 1024,
            "reset" => 2048,
            _ => 0,
        }
    }
    pub(super) fn has_entry(&self) -> bool {
        !self.label.is_empty()
            || self.default_label().is_some()
            || matches!(
                self.action.as_str(),
                "custom"
                    | "screen"
                    | "formclose"
                    | "view"
                    | "auto"
                    | "skip"
                    | "logskip"
                    | "scenereturn"
                    | "minimize"
            )
    }
}
impl QlieVm {
    /// Stable widget owner root. Its zero-coverage plane makes the panel
    /// pointer-transparent while retaining a subtree/clip authority for saves.
    pub(super) fn put_thread_ui_root(
        &mut self,
        h: &mut dyn HostServices,
        thread: ThreadKind,
    ) -> Result<()> {
        let root = self.root_layer_for(thread);
        self.submit(
            h,
            json!({"ui":[{"op":"put","widget":{
                "id":root,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],
                "kind":"panel","interaction_only":true,"keyboard_focusable":false,
                "hit_mask":{"size":[1,1],"coverage":[0],"threshold":1,
                    "source_from_local":[0,0,0,0,0,0,0,0,1]}
            }}]}),
        )
    }
    fn start_system_reader(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        label: &str,
        advance: bool,
    ) -> Result<()> {
        let first = if advance {
            m.enter_system(label)?
        } else {
            m.enter_system_event(label)?
        };
        self.system_input_active = true;
        if first {
            self.game_pause_ms = Some(h.now_ms());
            self.paused_random = self.random.clone();
            self.suspended_host = Some(self.take_host_context(h.now_ms()));
            self.thread_ui(h, ThreadKind::Game, false)?;
            self.submit(h,json!({"layers":[{"op":"set_clock_paused","id":1,"paused":true},{"op":"set_visible","id":1,"visible":false}]}))?;
        } else {
            self.discard_system_objects(h)?;
        }
        m.reset_system_globals();
        self.host_thread = ThreadKind::System;
        let root = self.root_layer_for(ThreadKind::System);
        if !self.system_root {
            self.submit(h,json!({"layers":[{"op":"put","layer":{"id":root,"rect":[0,0,self.config.canvas[0],self.config.canvas[1]],"transform":{"z":1000,"canvas_projection":{"plane":1000,"near":50,"far":5000,"near_clamp":true,"affine_uv":true}},"order":[i32::MIN,0],"paint":{"kind":"solid","color":[0,0,0,255]}}}]}))?;
            self.put_thread_ui_root(h, ThreadKind::System)?;
            self.system_root = true;
        }
        self.submit(
            h,
            json!({"layers":[{"op":"set_visible","id":root,"visible":true}]}),
        )?;
        self.selection.clear();
        Ok(())
    }
    /// 708048/707F0C dispatch system actions through 70A1D8, not through a
    /// subroutine call in whichever game file registered the input widget.
    /// 708048: system requests are not subject to the input locks.
    pub(super) fn dispatch_system_request(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        callback: Callback,
    ) -> Result<()> {
        self.dispatch_callback_checked(h, m, callback, false)
    }
    pub(super) fn dispatch_callback(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        callback: Callback,
    ) -> Result<()> {
        self.dispatch_callback_checked(h, m, callback, true)
    }
    fn dispatch_callback_checked(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        callback: Callback,
        check_locks: bool,
    ) -> Result<()> {
        let locks = self
            .objects
            .get(&Self::builtin("sys", m.thread_kind()))
            .and_then(|o| o.properties.get("lk"))
            .and_then(|v| v.first())
            .map(Value::integer)
            .transpose()?
            .unwrap_or(0);
        if check_locks && locks & callback.lock_mask() != 0 {
            return Ok(());
        }
        let label = if callback.label.is_empty() {
            callback.default_label().with_context(|| {
                format!(
                    "unsupported QLIE native input action {} without a script entry",
                    callback.action
                )
            })?
        } else {
            &callback.label
        };
        if !callback.same_reader && (!callback.action.is_empty()) {
            ensure!(
                callback.default_label().is_some()
                    || matches!(
                        callback.action.as_str(),
                        "custom"
                            | "screen"
                            | "formclose"
                            | "view"
                            | "auto"
                            | "skip"
                            | "logskip"
                            | "scenereturn"
                            | "minimize"
                    ),
                "unsupported QLIE native input action {}",
                callback.action
            );
            self.start_system_reader(h, m, label, false)?;
            // The native dispatcher clears delivered key/button edges after a
            // system action. The opening Escape cannot also cancel its new UI.
            self.input.buttons_pressed = 0;
            self.consume_key_edges();
            self.last_frame_event = Some(h.now_ms());
            return Ok(());
        }
        self.last_frame_event = Some(h.now_ms());
        let context = FrameEvent {
            depth: m.stack.len(),
            wait: self.wait.take(),
            poll: self.stop_poll.take(),
            scope: m.scope.clone(),
            selection: self.selection.clone(),
            scope_next: self.scope_next,
        };
        let resume = m.frame.pc;
        // Timer/OnFrame events interrupt between arbitrary instructions and
        // keep the reader's call receipt: Result banks return restored
        // (routine +8/+104, 6EA164). Button/key actions deliver ResultBtn*.
        let results = callback.action.is_empty().then(|| m.capture_call_results());
        m.jump(label, Some(&callback.file), Some(vec![]), false)?;
        if results.is_some() {
            m.frame.call_results = results;
        }
        m.stack.last_mut().unwrap().pc = resume;
        self.frame_event = Some(context);
        Ok(())
    }
    /// 6A1B64: a button reacts only when its +3053 group intersects the
    /// owning reader's System input mask (+364).
    pub(super) fn input_mask_allows(&self, o: &Object) -> bool {
        let group = o
            .properties
            .get("in")
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(255);
        let mask = self
            .objects
            .get(&Self::builtin("sys", o.thread))
            .and_then(|s| s.properties.get("in"))
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(0xFFFFFFF);
        group & mask != 0
    }
    pub(super) fn input_thread(&self) -> ThreadKind {
        if self.system_input_active {
            ThreadKind::System
        } else {
            ThreadKind::Game
        }
    }
    pub(super) fn visual_time(&self, thread: ThreadKind, started: u64, now: u64) -> u64 {
        if thread == ThreadKind::Game && self.system_input_active {
            self.game_pause_ms.map_or(now, |paused| paused.max(started))
        } else {
            now
        }
    }
    pub(super) fn host_object_roots(&self) -> Vec<String> {
        fn roots(
            out: &mut Vec<String>,
            selection: &[String],
            event: &Option<FrameEvent>,
            buttons: &std::collections::VecDeque<String>,
        ) {
            out.extend(selection.iter().cloned());
            out.extend(buttons.iter().cloned());
            if let Some(event) = event {
                out.push(event.scope.clone());
                out.extend(event.selection.iter().cloned());
            }
        }
        let mut out = vec![];
        roots(
            &mut out,
            &self.selection,
            &self.frame_event,
            &self.button_callbacks,
        );
        for context in self.suspended_host.iter().chain(self.cross_host.iter()) {
            roots(
                &mut out,
                &context.selection,
                &context.frame_event,
                &context.button_callbacks,
            );
        }
        out
    }
    pub(super) fn builtin(name: &str, thread: ThreadKind) -> String {
        Machine::context_builtin(name, thread).unwrap_or_else(|| name.into())
    }
    pub(super) fn root_layer_for(&self, thread: ThreadKind) -> u32 {
        match thread {
            ThreadKind::Game => 1,
            ThreadKind::System => 0x70000000,
        }
    }
    pub(super) fn capture_id(&self, offset: u32) -> u32 {
        (match self.host_thread {
            ThreadKind::Game => 0x10000000,
            ThreadKind::System => 0x11000000,
        }) + offset
    }
    fn take_host_context(&mut self, now: u64) -> HostContext {
        HostContext {
            wait: self.wait.take(),
            stop_poll: self.stop_poll.take(),
            frame_event: self.frame_event.take(),
            last_frame_event: self.last_frame_event.take(),
            selection: std::mem::take(&mut self.selection),
            scope_next: std::mem::take(&mut self.scope_next),
            button_callbacks: std::mem::take(&mut self.button_callbacks),
            skip_until: std::mem::take(&mut self.skip_until),
            capture: std::mem::take(&mut self.capture),
            second_capture: std::mem::take(&mut self.second_capture),
            capture_waiting: std::mem::take(&mut self.capture_waiting),
            effect_priority: std::mem::replace(&mut self.effect_priority, 65535),
            effect_clock: std::mem::take(&mut self.effect_clock),
            rule_fade: self.rule_fade.take(),
            animated_fade: self.animated_fade.take(),
            paused_at: now,
        }
    }
    pub(super) fn restore_host_context(&mut self, mut saved: HostContext, now: u64) {
        let delta = now.saturating_sub(saved.paused_at);
        fn shift_wait(wait: &mut Option<Wait>, delta: u64) {
            match wait {
                Some(Wait::Clock(t) | Wait::FixedClock(t)) => *t = t.saturating_add(delta),
                Some(Wait::Stop(s)) => s.deadline = s.deadline.map(|t| t.saturating_add(delta)),
                Some(Wait::Time { deadline, .. }) => *deadline = deadline.saturating_add(delta),
                Some(Wait::Button { deadline }) => *deadline = deadline.map(|t| t.saturating_add(delta)),
                Some(Wait::Sound { .. } | Wait::Message | Wait::Wave { .. }) | None => {}
            }
        }
        shift_wait(&mut saved.wait, delta);
        if let Some((_, stop)) = &mut saved.stop_poll {
            stop.deadline = stop.deadline.map(|t| t.saturating_add(delta));
        }
        if let Some(event) = &mut saved.frame_event {
            shift_wait(&mut event.wait, delta);
            if let Some((_, stop)) = &mut event.poll {
                stop.deadline = stop.deadline.map(|t| t.saturating_add(delta));
            }
        }
        self.wait = saved.wait;
        self.stop_poll = saved.stop_poll;
        self.frame_event = saved.frame_event;
        self.last_frame_event = saved.last_frame_event;
        self.selection = saved.selection;
        self.scope_next = saved.scope_next;
        self.button_callbacks = saved.button_callbacks;
        self.skip_until = saved.skip_until.saturating_add(delta);
        self.capture = saved.capture;
        self.second_capture = saved.second_capture;
        self.capture_waiting = saved.capture_waiting;
        self.effect_priority = saved.effect_priority;
        self.effect_clock = saved.effect_clock;
        self.rule_fade = saved.rule_fade;
        self.animated_fade = saved.animated_fade;
        for o in self
            .objects
            .values_mut()
            .filter(|o| o.thread == self.host_thread)
        {
            let object_delta = now.saturating_sub(saved.paused_at.max(o.started));
            o.started = o.started.saturating_add(object_delta);
            if let Some(group) = &mut o.generator {
                group.resume_clock(saved.paused_at, now);
            }
            if let Some(position) = &mut o.native_position {
                position.resume_clock(saved.paused_at, now);
            }
        }
    }
    fn thread_ui(
        &mut self,
        h: &mut dyn HostServices,
        thread: ThreadKind,
        active: bool,
    ) -> Result<()> {
        let commands = self
            .objects
            .values()
            .filter(|o| o.thread == thread && o.ui_registered)
            .map(|o| json!({"op":"set_enabled","id":o.layer,"enabled":active&&!o.locked&&self.input_mask_allows(o)}))
            .collect::<Vec<_>>();
        if !commands.is_empty() {
            self.submit(h, json!({"ui":commands}))?;
        }
        Ok(())
    }
    pub(super) fn discard_system_objects(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let ids = self
            .objects
            .iter()
            .filter(|(_, o)| o.thread == ThreadKind::System)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            let object = self.objects.remove(&id).unwrap();
            self.free_object(h, object)?;
        }
        let captures = [
            self.capture.then_some(self.capture_id(0)),
            self.second_capture.then_some(self.capture_id(2)),
        ]
        .into_iter()
        .flatten()
        .flat_map(|id| {
            [
                json!({"op":"hide","layer":id}),
                json!({"op":"destroy","id":id}),
            ]
        })
        .collect::<Vec<_>>();
        let mut layers = vec![];
        if self.effect_clock {
            layers.push(json!({"op":"remove","id":self.capture_id(1)}));
        }
        if !captures.is_empty() || !layers.is_empty() {
            self.submit(h, json!({"canvases":captures,"layers":layers}))?;
        }
        self.take_host_context(h.now_ms());
        Ok(())
    }
    pub(super) fn thread_instruction(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        ins: &Instruction,
    ) -> Result<bool> {
        match ins.op.as_str() {
            "srct" => {
                ensure!(ins.args.len() == 1, "QLIE source-thread ABI");
                m.set_source_thread(m.eval(&ins.args[0])?.integer()?)?;
                // Builtin selection uses a native ID in the selected thread's
                // manager. Switching the source manager changes that target
                // even when another scp has not been issued yet.
                let remap = |id: &str| {
                    let name = id.strip_prefix("system/").unwrap_or(id);
                    Machine::context_builtin(name, m.source_thread_kind())
                        .unwrap_or_else(|| id.into())
                };
                let scope = remap(&m.scope);
                self.selection = self.selection.iter().map(|id| remap(id)).collect();
                m.frame.selection = self.selection.clone();
                m.scope = scope;
            }
            "thrd" => {
                ensure!((1..=2).contains(&ins.args.len()), "QLIE thread-switch ABI");
                let selector = m.eval(&ins.args[0])?.integer()?;
                match selector {
                    1 => {
                        let label = ins
                            .args
                            .get(1)
                            .map(|s| m.eval(s).map(|v| v.text()))
                            .transpose()?
                            .unwrap_or_default();
                        self.start_system_reader(h, m, &label, true)?;
                    }
                    0 => {
                        ensure!(
                            self.suspended_host.is_some(),
                            "QLIE no paused GameThread host"
                        );
                        m.resume_game()?;
                        self.discard_system_objects(h)?;
                        self.host_thread = ThreadKind::Game;
                        self.system_input_active = false;
                        self.game_pause_ms = None;
                        self.paused_random = None;
                        let saved = self.suspended_host.take().unwrap();
                        self.restore_host_context(saved, h.now_ms());
                        self.submit(h,json!({"layers":[{"op":"set_clock_paused","id":1,"paused":false},{"op":"set_visible","id":1,"visible":true},{"op":"set_visible","id":self.root_layer_for(ThreadKind::System),"visible":false}]}))?;
                        self.thread_ui(h, ThreadKind::Game, true)?;
                    }
                    _ => bail!("unsupported QLIE thread selector {selector}"),
                }
            }
            "tsub" => {
                ensure!(!ins.args.is_empty(), "QLIE thread-subroutine ABI");
                let label = m.eval(&ins.args[0])?.text();
                let file = ins
                    .args
                    .get(1)
                    .filter(|s| !s.is_empty())
                    .map(|s| m.eval(s).map(|v| v.text()))
                    .transpose()?;
                let parameters = ins
                    .args
                    .iter()
                    .skip(2)
                    .map(|s| m.call_parameter(s))
                    .collect::<Result<Vec<_>>>()?;
                let cross = m.thread_kind() == ThreadKind::System;
                m.call_game(&label, file.as_deref(), parameters)?;
                if cross {
                    ensure!(
                        self.cross_host.is_none(),
                        "QLIE recursive cross-thread host call"
                    );
                    self.cross_host = Some(self.take_host_context(h.now_ms()));
                    self.host_thread = ThreadKind::Game;
                    // The callee operates on game graphics without consuming the
                    // paused reader's wait or event continuation.
                    let game = self
                        .suspended_host
                        .as_ref()
                        .context("QLIE paused game host absent")?;
                    self.capture = game.capture;
                    self.second_capture = game.second_capture;
                    self.capture_waiting = game.capture_waiting;
                    self.effect_priority = game.effect_priority;
                    self.effect_clock = game.effect_clock;
                    self.rule_fade = game.rule_fade.clone();
                    self.animated_fade = game.animated_fade;
                    self.selection = m.frame.selection.clone();
                }
            }
            _ => unreachable!(),
        }
        Ok(true)
    }
    pub(super) fn finish_thread_call(
        &mut self,
        h: &mut dyn HostServices,
        m: &Machine,
    ) -> Result<()> {
        if m.thread_kind() == ThreadKind::System {
            if let Some(saved) = self.cross_host.take() {
                // Keep native changes to game graphic state, but leave the
                // suspended reader's original input wait untouched.
                let game = self
                    .suspended_host
                    .as_mut()
                    .context("QLIE paused game host absent")?;
                game.capture = self.capture;
                game.second_capture = self.second_capture;
                game.capture_waiting = self.capture_waiting;
                game.effect_priority = self.effect_priority;
                game.effect_clock = self.effect_clock;
                game.rule_fade = self.rule_fade.take();
                game.animated_fade = self.animated_fade.take();
                self.take_host_context(h.now_ms());
                self.host_thread = ThreadKind::System;
                self.restore_host_context(saved, h.now_ms());
            }
        }
        Ok(())
    }
}
