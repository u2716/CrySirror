//! DC4PH native system command policy (40A2E0/4284B0). Shared engine services
//! own windows, civil clock sampling, UI, resources, and optional remote I/O.
use super::*;
use crate::settings::{SETTINGS_BYTES, SystemData};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemRequest {
    LocalHour,
    WindowState,
    NativeWindowMode,
    ///Renderer+24, initialized to1 by43A2F0; gates43A460 transitions.
    WindowTransitionFlag(Option<i32>),
    PresentationAspect(bool),
    Gesture(super::gesture_ops::GestureRequest),
    VoiceRate(i32),
    FramebufferStatus,
    CallbackInput,
    CallbackTimer(u32),
    Visible(bool),
    Fullscreen(bool),
    NativeFullscreenMode(i16),
    RestoreWindow,
    EnlargeWindow,
    ///4219A0(1), requested by18/18 only when its rollback mirror held66341.
    RestoreScrollingPair,
    /// No endpoint or credentials are embedded by the adapter.
    Account { operation:AccountOperation, system:Vec<u8>, face_names:Vec<u8> },
    License,
    Panel {
        mode: u8,
        parameter: i16,
        resume_pc: u32,
        duration_ms: u64,
        skip_unlocked: bool,
    },
    Thumbnails {
        release: bool,
        page: i32,
    },
    Catalogue,
    DefaultNames,
    HistoryNames,
    ResumePanel {mode:u8},
    RevealCursor,
    RebuildSelection {mode:u8,page:i32,declarations:text_ops::panel::PanelDeclarations},
    CharacterRow(usize),
    MusicSeek(i32),
    SourceSeek {
        channel: u32,
        percent: i32,
    },
    SourcePause {
        channel: u32,
        paused: bool,
    },
    CharacterPreview {
        name: String,
        position: [i32; 2],
        auxiliary:[i32;2],
        scale: u16,
    },
    /// Resolve native midnight using its cached host annual rule and CRT policy.
    CalendarEdge {
        year: i32,
        month: u8,
        day: u8,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountOperation {
    Upload,
    Download,
    Credentials,
    ClearReport,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemReply {
    NativeWindowMode(i16),
    WindowTransitionFlag(i32),
    Gesture(super::gesture_ops::GestureReply),
    Done,
    FramebufferComplete(bool),
    CallbackInput {
        pressed: u64,
        fullscreen: Option<i16>,
        idle: bool,
    },
    Hour(u8),
    Fullscreen(bool),
    Account {
        authenticated: bool,
        available: bool,
        status: bool,
        system: Option<Vec<u8>>,
    },
    Cancelled,
    Catalogue(Vec<u16>),
    DefaultNames([Vec<u8>;2]),
    HistoryNames(Vec<String>),
    Selection(text_ops::panel::PanelDeclarations),
    CharacterRow([i32; 12]),
    Weekday(Option<u8>),
    EndingComplete(u8),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct SystemState {
    #[serde(skip)]
    pub(super) gestures:super::gesture_ops::GestureState,
    #[serde(skip)]
    pub(super) input_prefix:super::gesture_ops::PrefixState,
    year: i32,
    edit_backup: Option<Vec<u8>>,
    edit_active: bool,
    reset_backup: Option<Vec<u8>>,
    #[serde(default)] reset_preview_active:bool,
    #[serde(default)] reset_cursor_pending:bool,
    #[serde(default)] close_requested:bool,
    #[serde(default)] close_complete:bool,
    panel_mode: u8,
    #[serde(skip)]
    character_preview_owner:Option<(String,u32)>,
    random_attempts: u16,
    music_order: Vec<u16>,
    music_cursor: usize,
    order_job: Option<OrderJob>,
    music_stopped: bool,
    incremented_seek: Option<(u32, i32)>,
    pub(super) frame_audio: i16,
    #[serde(skip)]
    reset_job: Option<PreferenceReset>,
    #[serde(skip)]
    account_reply: Option<(u32,SystemReply)>,
    #[serde(default)]
    callback_fullscreen: Option<i16>,
    #[serde(default)]
    callback_frame: Option<u64>,
    #[serde(skip)]
    pending_music_callback:bool,
    #[serde(skip)]
    pub(super) callback_panel_scroll:bool,
}
impl SystemState {
    pub(super) fn finish_callback_frame(&mut self) {self.callback_frame=None;}
}
impl Default for SystemState {
    fn default() -> Self {
        Self {
            gestures:Default::default(),
            input_prefix:Default::default(),
            year: 0,
            edit_backup: Some(vec![0;SETTINGS_BYTES]),
            edit_active: false,
            reset_backup: Some(vec![0;SETTINGS_BYTES]),
            reset_preview_active:false,
            reset_cursor_pending:false,
            close_requested:false,
            close_complete:false,
            panel_mode: 0,
            character_preview_owner:None,
            random_attempts: 0,
            music_order: Vec::new(),
            music_cursor: 0,
            order_job: None,
            music_stopped: false,
            incremented_seek: None,
            frame_audio: 0,
            reset_job: None,
            account_reply:None,
            callback_fullscreen: None,
            callback_frame: None,
            pending_music_callback:false,
            callback_panel_scroll:false,
        }
    }
}
#[derive(Clone, Debug)]
struct PreferenceReset {
    scene: String,
    pc: u32,
    selector: i16,
    parameter: i16,
    replacement: SystemData,
    names: Option<[String; 2]>,
    music_name: Option<String>,
    music: Option<MusicAsset>,
    committed: bool,
    controls_ready: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct OrderJob {
    order: Vec<u16>,
    cursor: usize,
    fill: usize,
    attempts: u16,
}

fn patch_prefix(data: &SystemData, prefix: &[u8]) -> Result<SystemData> {
    ensure!(
        prefix.len() == SETTINGS_BYTES,
        "Circus native preference snapshot extent"
    );
    let mut bytes = data.bytes().to_vec();
    bytes[..SETTINGS_BYTES].copy_from_slice(prefix);
    SystemData::try_from(bytes)
}
fn reset_data(data: &SystemData, group: Option<i16>) -> Result<SystemData> {
    let mut bytes = data.bytes().to_vec();
    let defaults = SystemData::default();
    let indices: &[usize] = match group {
        None => {
            bytes[..184].copy_from_slice(&defaults.bytes()[..184]);
            &[]
        }
        Some(0) => &[20, 9, 11, 24, 44, 35, 33, 25, 26, 45],
        Some(1) => &[1, 6, 3, 5, 28, 8, 39, 27],
        Some(2) => &[32, 31, 15, 13, 17, 16, 19, 18, 22, 37, 29, 30, 23],
        Some(5) => &[36, 38],
        _ => &[],
    };
    for index in indices {
        let at = index * 4;
        bytes[at..at + 4].copy_from_slice(&defaults.bytes()[at..at + 4]);
    }
    if group.is_none() || group == Some(5) {
        bytes[368..384].copy_from_slice(&defaults.bytes()[368..384]);
    }
    if group.is_none() || group == Some(4) {
        bytes[17640..17720].fill(0);
    }
    if group.is_none() {
        bytes[17720..19324].fill(0);
    }
    SystemData::try_from(bytes)
}

impl Machine {
    ///40D240 selector237 changes the renderer's presentation transform, not
    ///just its saved DWORD20. Re-read after load/reset/restore as well as setters.
    pub(crate) fn poll_native_window_values(&self,host:&mut dyn NativeHost)->Result<bool> {
        Ok(matches!(host.system(SystemRequest::PresentationAspect(self.state.value(99,237)?!=0))?,Some(SystemReply::Done)))
    }
    ///40E9C0/40E7D0 commit preferences before43CE40. Preserve that ordering
    ///across metadata/decode retries: neither re-read the old enable flag nor
    ///let the host discard the restart under its previous disabled controls.
    pub(super) fn poll_preference_reset(
        &mut self, host: &mut dyn NativeHost, i: &Instruction, selector: i16,
        parameter: i16, replacement: Option<SystemData>, replay: bool,
        names: Option<[String; 2]>,
    ) -> Result<bool> {
        if self.state.system_ops.reset_job.is_none() {
            self.state.system_ops.reset_job = Some(PreferenceReset {
                scene: self.state.scene.clone(), pc: i.offset, selector, parameter,
                replacement: replacement.context("Circus preference reset declaration missing")?, names,
                music_name: if replay {let name=self.state.native_live.retained_music_name()?;(!name.is_empty()).then_some(name)}else{None},
                music: None, committed: false, controls_ready: false,
            });
        }
        let job = self.state.system_ops.reset_job.as_ref().unwrap();
        ensure!(job.scene == self.state.scene && job.pc == i.offset && job.selector == selector && job.parameter == parameter,
            "Circus preference reset continuation owner changed");
        if job.music_name.is_some() && job.music.is_none() {
            let name = job.music_name.as_ref().unwrap().clone();
            let Some(asset) = host.music_by_name(&name, true)? else { return Ok(false); };
            self.state.system_ops.reset_job.as_mut().unwrap().music = Some(asset);
        }
        if !self.state.system_ops.reset_job.as_ref().unwrap().committed {
            let old_gain=self.state.audio_controls()?.gain;
            let job = self.state.system_ops.reset_job.as_ref().unwrap();
            self.state.system_data = job.replacement.clone();
            //40E7D0/40E9C0→40DAD0 reapplies enables, not43B4C0.
            //Keep active-buffer gain until a native gain setter/new PCM.
            self.state.media.effective_gains=Some(old_gain);
            if let Some(names) = &job.names { self.state.configured_names = names.clone(); }
            if selector == 29 && parameter == 1 {
                self.state.assign(100, 49, 99 - self.state.system_data.dword(6))?;
            }
            self.state.system_ops.reset_job.as_mut().unwrap().committed = true;
        }
        if !self.state.system_ops.reset_job.as_ref().unwrap().controls_ready {
            let controls = self.state.audio_controls()?;
            if !host.emit(&Event::NativeMixerControls { enabled: controls.enabled, gain: controls.gain,playback_gain:controls.playback_gain,force_gain:[false;3],force_enable:[true;3] })? { return Ok(false); }
            self.state.system_ops.reset_job.as_mut().unwrap().controls_ready = true;
        }
        if let Some(asset) = self.state.system_ops.reset_job.as_ref().unwrap().music.clone() {
            if !host.emit(&Event::PlayMusic { asset: asset.clone(), looping: true })? { return Ok(false); }
            self.state.last_music = Some(asset);
        }
        self.state.system_ops.reset_job = None;
        Ok(true)
    }
    ///40C110 is called before41C180 persists preferences. An abandoned
    ///98 preview is restored once, including OS-close and native halt paths.
    pub(crate) fn native_cursor_auto_hide(&self)->bool {self.state.system_data.dword(25)!=0}
    pub(crate) fn request_native_close(&mut self)->bool {
        self.state.system_ops.close_requested=true;true
    }
    pub(crate) fn native_close_requested(&self)->bool {self.state.system_ops.close_requested}
    pub(crate) fn poll_native_close(&mut self,host:&mut dyn NativeHost)->Result<bool> {
        if self.state.system_ops.close_complete {return Ok(true)}
        ensure!(self.state.system_ops.close_requested,"Circus close poll without owner");
        if self.state.system_ops.reset_preview_active {
            let backup=self.state.system_ops.reset_backup.as_deref().context("Circus native close reset backup missing")?;
            self.state.system_data=patch_prefix(&self.state.system_data,backup)?;
            self.state.system_ops.reset_preview_active=false;
            self.state.native_flags.insert(9668,0);
        }
        if !host.write_system(self.state.system_data.bytes())? {return Ok(false)}
        self.state.system_ops.close_complete=true;
        self.state.halted=true;
        Ok(true)
    }
    fn system_catalogue(&mut self, host: &mut dyn NativeHost) -> Result<Option<Vec<u16>>> {
        let Some(reply) = host.system(SystemRequest::Catalogue)? else {
            return Ok(None);
        };
        let SystemReply::Catalogue(ids) = reply else {
            bail!("Circus music catalogue completion type")
        };
        ensure!(
            !ids.is_empty() && ids.len() <= 400 && ids.iter().all(|id| *id > 0 && *id < 400),
            "Circus native music catalogue range"
        );
        ensure!(
            ids.iter().collect::<BTreeSet<_>>().len() == ids.len(),
            "Circus duplicate native music catalogue"
        );
        Ok(Some(ids))
    }
    fn rebuild_music_order(&mut self, host: &mut dyn NativeHost, ids: &[u16]) -> Result<bool> {
        if self.state.value(100, 26)? <= 1 {
            let old = self
                .state
                .system_ops
                .music_order
                .get(self.state.value(7, 98)? as usize)
                .copied();
            self.state.system_ops.music_cursor = old
                .and_then(|id| ids.iter().position(|v| *v == id))
                .unwrap_or(0);
            self.state.system_ops.music_order = ids.to_vec();
            return Ok(true);
        }
        if self.state.system_ops.order_job.is_none() {
            let cursor = self.state.value(7, 98)? as usize;
            ensure!(cursor < 400, "Circus native music shuffle cursor");
            let old = self
                .state
                .system_ops
                .music_order
                .get(cursor)
                .copied()
                .unwrap_or(0);
            let mut order = vec![0; 400];
            order[cursor] = old;
            self.state.system_ops.order_job = Some(OrderJob {
                order,
                cursor,
                fill: 0,
                attempts: 0,
            });
        }
        let job = self.state.system_ops.order_job.as_mut().unwrap();
        while job.fill < ids.len() && job.attempts < 10000 {
            if job.fill == job.cursor && job.order[job.cursor] != 0 {
                job.fill += 1;
                job.attempts += 1;
                continue;
            }
            let Some(draw) = host.random(ids.len() as u16)? else {
                return Ok(false);
            };
            ensure!(
                draw >= 0 && (draw as usize) < ids.len(),
                "Circus music shuffle random range"
            );
            let id = ids[draw as usize];
            if !job.order.contains(&id) {
                job.order[job.fill] = id;
                job.fill += 1;
            }
            job.attempts += 1;
        }
        for id in ids {
            if job.fill >= ids.len() {
                break;
            }
            if !job.order.contains(id) {
                job.order[job.fill] = *id;
                job.fill += 1;
            }
        }
        ensure!(
            job.fill >= ids.len(),
            "Circus native music shuffle completion"
        );
        self.state.system_ops.music_cursor = job.cursor;
        self.state.system_ops.music_order = job.order[..ids.len()].to_vec();
        self.state.system_ops.order_job = None;
        Ok(true)
    }
    fn music_unlocked(&self, id: u16) -> bool {
        self.state.system_data.dword(4430 + usize::from(id)) != 0
    }
    fn set_music_unlocked(&mut self, id: u16, value: bool) -> Result<()> {
        ensure!(id < 400, "Circus unlocked music index");
        let mut bytes = self.state.system_data.bytes().to_vec();
        let at = 17720 + usize::from(id) * 4;
        bytes[at..at + 4].copy_from_slice(&i32::from(value).to_le_bytes());
        self.state.system_data = SystemData::try_from(bytes)?;
        Ok(())
    }
    fn music_gallery(&mut self, parameter: i16, host: &mut dyn NativeHost) -> Result<bool> {
        let count = (0..400).filter(|id| self.music_unlocked(*id)).count();
        if parameter == 6 {
            self.state.assign(3, 0, count as i32)?;
            return Ok(true);
        }
        if count == 0 {
            self.state.assign(100, 21, 0)?;
        }
        if !matches!(parameter, 0 | 1 | 2 | 3 | 5 | 99) {
            return Ok(true);
        }
        let Some(ids) = self.system_catalogue(host)? else {
            return Ok(false);
        };
        let filter = self.state.value(100, 21)? != 0 && count != 0;
        match parameter {
            0 | 2 | 3 => {
                let mut cursor = self.state.system_ops.music_cursor as i32;
                if parameter == 0 {
                    if !self.rebuild_music_order(host, &ids)? {
                        return Ok(false);
                    }
                    cursor = self.state.system_ops.music_cursor as i32;
                    if self.state.value(100, 26)? <= 1 {
                        self.state.assign(7, 98, cursor)?;
                    }
                    // Native0 only publishes the selected slot when filtering.
                    if !filter {
                        return Ok(true);
                    }
                } else {
                    cursor += if parameter == 2 { 1 } else { -1 };
                }
                for _ in 0..=ids.len() {
                    if cursor < 0 || cursor >= self.state.system_ops.music_order.len() as i32 {
                        if !self.rebuild_music_order(host, &ids)? {
                            return Ok(false);
                        }
                        cursor = if parameter == 3 {
                            ids.len() as i32 - 1
                        } else {
                            0
                        };
                    }
                    let id = *self
                        .state
                        .system_ops
                        .music_order
                        .get(cursor as usize)
                        .context("Circus music gallery order before initialization")?;
                    if !filter || self.music_unlocked(id) {
                        self.state.system_ops.music_cursor = cursor as usize;
                        self.state.assign(7, 98, cursor)?;
                        if parameter == 0 {
                            self.state.assign(99, 254, cursor)?;
                        }
                        return Ok(true);
                    }
                    cursor += if parameter == 3 { -1 } else { 1 };
                }
                bail!("Circus unlocked gallery has no reachable catalogue entry")
            }
            1 | 5 | 99 => {
                let cursor = self.state.value(7, 98)? as usize;
                let cursor = if cursor >= self.state.system_ops.music_order.len() { 0 } else { cursor };
                let id = if parameter == 5 {
                    ids[cursor]
                } else {
                    //The constructor's global400-element order is zeroed;
                    //before408EB0 a direct play therefore selects native ID0.
                    self.state.system_ops.music_order.get(cursor).copied().unwrap_or(0)
                };
                if parameter == 99 {
                    let seek = self.state.value(99, 187)?;
                    let Some(reply) = host.system(SystemRequest::MusicSeek(seek))? else {
                        return Ok(false);
                    };
                    ensure!(
                        matches!(reply, SystemReply::Done),
                        "Circus music seek completion type"
                    );
                } else {
                    let Some(asset) = host.music(id, false)? else {
                        return Ok(false);
                    };
                    if !self.state.system_ops.music_stopped {
                        if !host.emit(&Event::ReleaseMusic {forced:true,steps:0})? {
                            return Ok(false);
                        }
                        self.state.system_ops.music_stopped = true;
                    }
                    let event=if parameter==5 {Event::PlayResidentMusic{asset:asset.clone(),looping:false}}
                        else {Event::PlayMusic{asset:asset.clone(),looping:false}};
                    if !host.emit(&event)? {return Ok(false)}
                    //40C950 marks the native music-unlocked byte even in
                    //the gallery path; this is independent of gallery4430.
                    self.state.assign(96,i16::try_from(id)?,1)?;
                    self.state.system_ops.music_stopped = false;
                }
                self.state.assign(7, 98, cursor as i32)?;
                self.state.assign(3, 11, 0)?;
                self.state.system_ops.music_cursor = cursor;
                if parameter == 5 {
                    if filter {
                        self.set_music_unlocked(id, true)?;
                    }
                    if self.state.value(100, 26)? == 2 {
                        if let Some(position) = self
                            .state
                            .system_ops
                            .music_order
                            .iter()
                            .position(|v| *v == id)
                        {
                            self.state.assign(99, 254, cursor as i32)?;
                            self.state.assign(7, 98, position as i32)?;
                            self.state.system_ops.music_cursor = position;
                        }
                    }
                } else {
                    if let Some(index)=ids.iter().position(|value|*value==id) {
                        self.state.assign(99,254,index as i32)?;
                    }
                }
                Ok(true)
            }
            _ => Ok(true),
        }
    }
    ///Static waits must still run433E50's channel0/500ms callback policy at
    ///native60Hz. The11416 map entry shares normal callback restore/clear.
    pub(crate) fn native_callback_deadline(&self,now:u64)->Option<u64> {
        self.state.callbacks.get(&11416).is_some_and(|name|!name.is_empty()).then(||now.saturating_add(16))
    }
    ///433E50 global callbacks are sampled once after a native input frame.
    ///A pending physical service never consumes that frame's callback sample.
    pub(super) fn poll_system_callbacks(
        &mut self,
        host: &mut dyn NativeHost,
        now: u64,
        _pressed: bool,
    ) -> Result<bool> {
        self.state.system_ops.callback_panel_scroll|=host.take_panel_scroll_changed();
        let music_enabled=self.state.callbacks.get(&11416).is_some_and(|name|!name.is_empty());
        if !music_enabled {self.state.system_ops.pending_music_callback=false;}
        let pending_music=self.state.system_ops.pending_music_callback;
        if self.state.system_ops.callback_frame == Some(now)&&!pending_music {return Ok(true)}
        if !music_enabled&&!pending_music&&!self.state.callbacks.iter().any(|(_,name)|!name.is_empty()&&name!="-") {
            self.state.system_ops.callback_panel_scroll=false;
            return Ok(true);
        }
        let Some(reply) = host.system(SystemRequest::CallbackInput)? else {
            return Ok(false);
        };
        let SystemReply::CallbackInput {
            pressed,
            fullscreen,
            idle,
        } = reply
        else {
            bail!("Circus callback input receipt");
        };
        self.state.system_ops.callback_frame = Some(now);
        let panel_scroll=std::mem::take(&mut self.state.system_ops.callback_panel_scroll);
        use crysirror_core::service::buttons as b;
        let mut trigger = pending_music.then_some((11416,false));
        //442D60 and442D90 preserve this exact keyboard/mouse priority.
        for (offset, mask) in [
            (10456, b::MOUSE_LEFT | b::ENTER | b::PAGE_DOWN),
            (10520, b::MOUSE_RIGHT | b::ESCAPE),
            (10712, b::LEFT),
            (10776, b::RIGHT),
            (10840, b::UP),
            (10904, b::DOWN),
        ] {
            if trigger.is_none()&&pressed & mask != 0
                && self
                    .state
                    .callbacks
                    .get(&offset)
                    .is_some_and(|name| !name.is_empty() && name != "-")
            {
                trigger = Some((offset, true));
                break;
            }
        }
        //433E50 LABEL285: panel -2 follows directional callbacks and
        //precedes the remaining common callbacks, for process11 and12 alike.
        if trigger.is_none()&&panel_scroll&&self.state.callbacks.get(&11288).is_some_and(|name|!name.is_empty()) {
            trigger=Some((11288,false));
        }
        if trigger.is_none()&&music_enabled {
            //433E50 LABEL285:11416 has its own opcode61 producer. Merely
            //being nonempty suppresses fullscreen/idle, even while music is
            //playing or its500ms throttle has not expired (including "-").
            let playing=host.audio_playing(0)?;
            let last=self.state.native_flags.get(&23868).copied().unwrap_or(0) as u32;
            if !playing&&(now as u32).wrapping_sub(last)>500 {
                self.state.system_ops.pending_music_callback=true;
                trigger=Some((11416,false));
            }
        }
        if trigger.is_none()&&!music_enabled
            && self
                .state
                .callbacks
                .get(&10968)
                .is_some_and(|name| !name.is_empty() && name != "-")
        {
            let observed =
                fullscreen.context("Circus callback fullscreen observation unavailable")?;
            if self.state.system_ops.callback_fullscreen != Some(observed) {
                self.state.system_ops.callback_fullscreen = Some(observed);
                trigger = Some((10968, false));
            }
        }
        if trigger.is_none()&&!music_enabled
            && idle
            && self
                .state
                .callbacks
                .get(&11352)
                .is_some_and(|name| !name.is_empty() && name != "-")
        {
            let Some(reply) = host.system(SystemRequest::CallbackTimer(0))? else {
                self.state.system_ops.callback_frame = None;
                return Ok(false);
            };
            ensure!(
                matches!(reply, SystemReply::Done),
                "Circus callback timer completion receipt"
            );
            trigger = Some((11352, false));
        }
        if let Some((offset, interrupt)) = trigger {
            self.state.dialogue.panel_wait=false;
            if self.program.is_none() {
                self.program = host.scene(&self.state.scene)?;
                if self.program.is_none() {
                    self.state.system_ops.callback_frame = None;
                    return Ok(false);
                }
            }
            if offset==11416 {
                //Commit23868 after the program owner is available. A lazy
                //scene worker retry must not consume the observed callback.
                self.state.native_flags.insert(23868,now as u32 as i32);
                self.state.native_flags.insert(15624,1);
                self.state.system_ops.pending_music_callback=false;
            }
            let name = &self.state.callbacks[&offset];
            let program = self
                .program
                .as_ref()
                .context("Circus callback program missing")?;
            //41FB00 leaves the decoder PC untouched when4381F0 finds no label.
            if let Some(target) = program.instructions.iter().find(|i| {
                i.opcode == 0x4b && matches!(&i.arguments,Arguments::String(label)if label==name)
            }) {
                self.state.pc = target.offset;
                self.state
                    .native_live
                    .advance_pc(&self.state.scene, self.state.pc)?;
            }
            if interrupt {
                self.state.wait_process = 0;
                self.state.wait_click = false;
                self.state.wait_until = None;
                self.state.wait_animations = false;
                self.state.wait_audio_mask = 0;
                self.state.wait_menu_motion = false;
                self.state.menu_wait = false;
                self.state.wait_text_idle = None;
            } else {
                self.state.wait_process = 0;
                self.state.wait_click = false;
                self.state.wait_until = None;
                self.state.menu_wait = false;
            }
        }
        Ok(true)
    }
    pub(super) fn execute_system(
        &mut self,
        program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>> {
        let complete = |next| {
            Some(CommandResult::Complete {
                next,
                outcome: Outcome::Advanced,
            })
        };
        let mut request = None;
        match i.opcode {
            0x43 => {
                let offset = match a[0] {
                    0 => 10584,
                    1 => 10648,
                    2 => 10456,
                    3 => 10520,
                    4 => 10712,
                    5 => 10776,
                    6 => 10968,
                    10 => 10840,
                    11 => 10904,
                    12 => 11032,
                    13 => 11096,
                    14 => 11160,
                    15 => 11288,
                    16 => 11352,
                    _ => return Ok(complete(i.next)), // Original switch default.
                };
                let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(text);
                ensure!(
                    !error && encoded.len() < 64,
                    "Circus native callback label extent"
                );
                if a[0] == 6 {
                    let Some(SystemReply::NativeWindowMode(value)) =
                        host.system(SystemRequest::NativeWindowMode)?
                    else {
                        return Ok(Some(CommandResult::Retry));
                    };
                    self.state.system_ops.callback_fullscreen = Some(value);
                } else if a[0] == 16 {
                    let Some(reply) = host.system(SystemRequest::CallbackTimer(60000))? else {
                        return Ok(Some(CommandResult::Retry));
                    };
                    ensure!(
                        matches!(reply, SystemReply::Done),
                        "Circus idle timer declaration receipt"
                    );
                }
                if a[0] == 0 {
                    self.state.callbacks.insert(10456, String::new());
                }
                if a[0] == 1 {
                    self.state.callbacks.insert(10520, String::new());
                }
                self.state.callbacks.insert(offset, text.into());
            }
            0xa9 => match a[3] {
                0 | 1 | 2 => {
                    let percent = if a[3] == 0 {
                        if let Some((pc, value)) = self.state.system_ops.incremented_seek {
                            ensure!(pc == i.offset, "Circus source seek instruction owner");
                            value
                        } else {
                            let value = self.state.value(99, 187)?;
                            let value = if value != 99 {
                                value.wrapping_add(1)
                            } else {
                                value
                            };
                            self.state.system_ops.incremented_seek = Some((i.offset, value));
                            value
                        }
                    } else {
                        self.state.value(99, 187)?
                    };
                    request = Some(SystemRequest::SourceSeek {
                        channel: match a[3] {
                            0 => 0,
                            1 => 2,
                            _ => 8,
                        },
                        percent,
                    });
                }
                10 | 11 => {
                    request = Some(SystemRequest::SourcePause {
                        channel: 8,
                        paused: a[3] == 11,
                    })
                }
                99 => self.state.system_ops.frame_audio = a[0],
                _ => {}
            },
            0x84 => {
                let mut pc = i.next;
                let mut sum = 0i32;
                let mut count = 0;
                loop {
                    let row = program.instruction(pc)?;
                    if row.opcode == 2 {
                        break;
                    }
                    let Arguments::Words(words) = row.arguments else {
                        bail!("Circus sum requires native nine-byte value rows")
                    };
                    ensure!(
                        row.next == row.offset + 9 && row.continuation.is_none(),
                        "Circus sum row width"
                    );
                    sum = sum.wrapping_add(self.value(host, words[0], words[1])?);
                    pc = row.next;
                    count += 1;
                    ensure!(
                        count <= program.instructions.len(),
                        "Circus unterminated sum"
                    );
                }
                self.state.assign(a[0], a[1], sum)?;
                return Ok(complete(pc));
            }
            0x9e => {
                self.state.native_flags.insert(66508, a[0].into());
                self.state.native_live.scene_marker(a[0]);
            }
            0xb5 => {
                if a[3] == 99 {
                    self.state.system_ops.year = a[0].into();
                } else {
                    let year = self.state.system_ops.year;
                    let month = i32::from(a[1]) / 100;
                    let day = i32::from(a[1]) % 100;
                    //The pinned statically linked45BE17 checks the input
                    //year1969..3001 both before and after month normalization,
                    //before allowing the day count to cross a year boundary.
                    let month_year=i64::from(year)+(i64::from(month)-1).div_euclid(12);
                    let date = ((1969..=3001).contains(&year)&&(1969..=3001).contains(&month_year))
                        .then(||crysirror_core::service::normalize_civil_date(year, month, day)).flatten();
                    let weekday = match date {
                        Some((y, m, d)) => {
                            let Some(reply) = host.system(SystemRequest::CalendarEdge {
                                year: y,
                                month: m,
                                day: d,
                            })?
                            else {
                                return Ok(Some(CommandResult::Retry));
                            };
                            let SystemReply::Weekday(value) = reply else {
                                bail!("Circus calendar completion type")
                            };
                            value
                        }
                        _ => None,
                    };
                    self.state.assign(3, 90, a[0].into())?;
                    self.state.assign(3, 91, month)?;
                    self.state.assign(3, 92, day)?;
                    self.state.assign(
                        3,
                        93,
                        weekday.filter(|v| *v != 0).map_or(6, |v| i32::from(v) - 1),
                    )?;
                }
            }
            0x18 => match a[0] {
                0 | 4 | 5 | 7 | 15 | 24 | 26 => return Ok(None),
                1 | 9 | 11 | 12 | 13 | 14 | 16 | 17 | 19 | 21 | 99 => {}
                2 | 3 => request = Some(SystemRequest::Visible(a[0] == 3)),
                6 => {
                    if let Some((scene, start, end)) = &self.state.read_run {
                        let Some(id) = host.scene_index(scene)? else {
                            return Ok(Some(CommandResult::Retry));
                        };
                        if id >= 0 && *start != 0 {
                            self.state
                                .system_data
                                .mark_read(id as usize, *start, *end)?;
                        }
                    }
                    if a[1] == 1 {
                        let Some(scene) = host.scene_index(&self.state.scene)? else {
                            return Ok(Some(CommandResult::Retry));
                        };
                        self.state.native_live.discard_scene_read_history(scene)?;
                        self.state.read_run = None;
                    }
                }
                8 => request = Some(SystemRequest::NativeFullscreenMode(a[1])),
                10 => request = Some(SystemRequest::License),
                18 => {
                    //244503 is in the731976 rollback mirror, at the position
                    //of live66341; it is not the current resource27 handle.
                    let scrolling = self
                        .state
                        .saves
                        .rollback
                        .as_ref()
                        .map(|snapshot| snapshot.observed_body().native_word(66341 * 4))
                        .transpose()?
                        .unwrap_or(0)
                        != 0;
                    if scrolling {
                        let Some(reply) = host.system(SystemRequest::RestoreScrollingPair)? else {
                            return Ok(Some(CommandResult::Retry));
                        };
                        ensure!(
                            matches!(reply, SystemReply::Done),
                            "Circus scrolling restoration receipt"
                        );
                        let dimensions = host.image_dimensions(27)?;
                        for (object, crop) in [(64, [0, 0, 3072, 576]), (65, [0, 600, 3072, 1176])]
                        {
                            self.state.native_live.reset_object(object)?;
                            self.state.native_live.bind_resource(
                                27,
                                object,
                                Some(crop),
                                dimensions,
                            )?;
                        }
                        self.sync_sprite_states(host)?;
                        self.sync_effect_frame(&host.effect_states()?)?;
                        self.state.native_live.falling_effect(16, 1)?;
                        self.state.native_flags.insert(66341, 1);
                    }
                    self.state.native_live.falling_effect(7, 1)?;
                    self.state.native_flags.insert(66336, 1);
                }
                20 | 29 => {
                    let replacement = if self.state.system_ops.reset_job.is_none() {
                        Some(reset_data(&self.state.system_data, if a[0] == 29 { Some(a[1]) } else { None })?)
                    } else { None };
                    let replay = self.state.system_data.dword(13) == 0;
                    if !self.poll_preference_reset(host, i, a[0], a[1], replacement, replay, None)? {
                        return Ok(Some(CommandResult::Retry));
                    }
                }
                22 | 23 | 25 => {
                    request = Some(SystemRequest::Account {operation:match a[0] {
                        22 => AccountOperation::Upload,
                        23 => AccountOperation::Download,
                        _ => AccountOperation::Credentials,
                    },system:self.state.system_data.bytes().to_vec(),face_names:self.state.native_live.cloud_face_names()})
                }
                27 => request = Some(SystemRequest::RestoreWindow),
                28 => request = Some(SystemRequest::EnlargeWindow),
                _ => {} // Original18 switch default advances without changing state.
            },
            0x1e => match a[0] {
                1 => request = Some(SystemRequest::LocalHour),
                2 => {
                    let mut best = 0;
                    let mut value = self.state.value(5, 0)?;
                    for n in 1..4 {
                        let candidate = self.state.value(5, n)?;
                        if candidate > value {
                            best = n;
                            value = candidate
                        }
                    }
                    self.state.assign(3, 0, best.into())?;
                }
                6 => {
                    self.state.system_ops.panel_mode = if a[1] != 0 { 4 } else { 3 };
                    self.state.native_flags.insert(272075, 1);
                }
                7 if a[1] == 0 => {}
                7 if matches!(self.state.system_ops.panel_mode,1|6) => {
                    request=Some(SystemRequest::ResumePanel{mode:self.state.system_ops.panel_mode});
                }
                7 => {
                    request = Some(SystemRequest::Panel {
                        mode: self.state.system_ops.panel_mode,
                        parameter: a[1],
                        resume_pc: i.next,
                        duration_ms: 0,
                        skip_unlocked: false,
                    })
                }
                9 => {
                    self.state.system_ops.panel_mode=1;
                    request = Some(SystemRequest::Panel {
                        mode: 1,
                        parameter: a[1],
                        resume_pc: i.next,
                        duration_ms: u64::from(self.state.value(3,0)?.wrapping_mul(1000) as u32),
                        skip_unlocked: self.state.value(100, if a[1] == 1 { 110 } else { 111 })?
                            != 0,
                    })
                }
                18 => {
                    match a[1] {
                        0 => {
                            self.state.system_ops.edit_backup =
                                Some(self.state.system_data.bytes()[..SETTINGS_BYTES].to_vec());
                            self.state.system_ops.edit_active = true;
                        }
                        1 => {
                            let backup = self
                                .state
                                .system_ops
                                .edit_backup
                                .as_deref()
                                .context("Circus preference compare before capture")?;
                            ensure!(backup.len() == SETTINGS_BYTES, "Circus edit backup extent");
                            let current = self.state.system_data.bytes();
                            let different = current[..270] != backup[..270]
                                || current[271..SETTINGS_BYTES] != backup[271..];
                            self.state.assign(3, 0, i32::from(different))?;
                            self.state.system_ops.edit_active = false;
                        }
                        _ => {
                            if self.state.system_ops.edit_active {
                                let backup = self
                                    .state
                                    .system_ops
                                    .edit_backup
                                    .as_deref()
                                    .context("Circus edit backup missing")?;
                                self.state.system_data =
                                    patch_prefix(&self.state.system_data, backup)?;
                            }
                            self.state.system_ops.edit_active = false;
                        }
                    }
                    self.state
                        .native_flags
                        .insert(272085, i32::from(self.state.system_ops.edit_active));
                }
                51 if a[1]==1 => {
                    let Some(reply)=host.system(SystemRequest::DefaultNames)? else{return Ok(Some(CommandResult::Retry))};
                    let SystemReply::DefaultNames(defaults)=reply else{bail!("Circus default-name metadata reply")};
                    let mut unchanged=true;
                    for (name,default) in self.state.protagonist_names.iter().zip(defaults.iter()) {
                        let (encoded,_,error)=encoding_rs::SHIFT_JIS.encode(name);
                        ensure!(!error,"Circus protagonist name encoding");
                        unchanged &= encoded.len()==4 && encoded.as_ref()==default.as_slice();
                    }
                    self.state.native_flags.insert(66516,i32::from(!unchanged));
                    self.state.native_live.protagonist_difference(!unchanged);
                }
                51 => {self.state.system_ops.panel_mode=6;return Ok(None)},
                19 => return Ok(None),
                20 => {
                    request = Some(SystemRequest::Thumbnails {
                        release: a[1] != 0,
                        page: self.state.value(99, 0)?,
                    })
                }
                98 => {
                    if !self.state.system_ops.reset_cursor_pending {
                    let replacement=if self.state.system_ops.reset_job.is_none() {
                        let backup=self.state.system_data.bytes()[..SETTINGS_BYTES].to_vec();
                        let mut replacement=reset_data(&self.state.system_data,None)?.bytes().to_vec();
                        replacement[60..64].copy_from_slice(&backup[60..64]);
                        self.state.system_ops.reset_backup=Some(backup);
                        Some(SystemData::try_from(replacement)?)
                    }else{None};
                    //40E9C0 commits reset controls and conditionally replays
                    //music before442B70(1). That helper is cursor visibility,
                    //not a keyboard-stock reset (complete442B70 source).
                    let replay=self.state.system_data.dword(13)==0;
                    if !self.poll_preference_reset(host,i,98,a[1],replacement,replay,None)? {return Ok(Some(CommandResult::Retry))}
                    self.state.system_ops.reset_cursor_pending=true;
                    }
                    if self.state.system_data.dword(25)!=0 {
                        if host.system(SystemRequest::RevealCursor)?.is_none(){return Ok(Some(CommandResult::Retry))}
                    }
                    let mut bytes=self.state.system_data.bytes().to_vec();bytes[100..104].fill(0);
                    self.state.system_data=SystemData::try_from(bytes)?;
                    self.state.system_ops.reset_cursor_pending=false;
                    self.state.system_ops.reset_preview_active=true;
                    self.state.native_flags.insert(9668,1);
                }
                99 => {
                    let backup=self.state.system_ops.reset_backup.as_deref().context("Circus reset restore before capture")?;
                    self.state.system_data=patch_prefix(&self.state.system_data,backup)?;
                    self.state.system_ops.reset_preview_active=false;
                    self.state.native_flags.insert(9668,0);
                }
                105 => request = Some(SystemRequest::Account { operation:AccountOperation::ClearReport,
                    system:self.state.system_data.bytes().to_vec(),face_names:self.state.native_live.cloud_face_names() }),
                200 => {
                    if !self.music_gallery(a[1], host)? {
                        return Ok(Some(CommandResult::Retry));
                    }
                }
                201 | 202 => {
                    if a[0] == 201 && (a[1] == 254 || a[1] == 255) {
                        let mut bytes = self.state.system_data.bytes().to_vec();
                        for n in 0..400 {
                            bytes[17720 + n * 4..17724 + n * 4]
                                .copy_from_slice(&i32::from(a[1] == 255).to_le_bytes());
                        }
                        self.state.system_data = SystemData::try_from(bytes)?;
                    } else {
                        let Some(ids) = self.system_catalogue(host)? else {
                            return Ok(Some(CommandResult::Retry));
                        };
                        let index = i32::from(a[1]) + self.state.value(99, 246)?;
                        let id = *ids
                            .get(usize::try_from(index)?)
                            .context("Circus native gallery music index")?;
                        if a[0] == 201 {
                            self.set_music_unlocked(id, !self.music_unlocked(id))?;
                        } else {
                            self.state
                                .assign(3, 0, i32::from(self.music_unlocked(id)))?;
                        }
                    }
                }
                203 => {
                    let base_character = self.state.value(3, 5)?;
                    let subcharacter = self.state.value(3, 84)?;
                    let character = if base_character >= 5 {
                        base_character + subcharacter
                    } else {
                        base_character
                    };
                    let Some(reply) =
                        host.system(SystemRequest::CharacterRow(usize::try_from(character)?))?
                    else {
                        return Ok(Some(CommandResult::Retry));
                    };
                    let SystemReply::CharacterRow(row) = reply else {
                        bail!("Circus character preview completion type")
                    };
                    let prefix = match base_character {
                        0 => "hm_",
                        1 => "hd_",
                        2 => "io_",
                        3 => "nm_",
                        4 => "ar_",
                        5 => match subcharacter {
                            0 => "ym_",
                            1 => "mm_",
                            2 => "kn_",
                            3 => "ai_",
                            4 => "ci_",
                            5 => "en_",
                            _ => "tr_",
                        },
                        _ => "",
                    };
                    let selection = usize::try_from(self.state.value(3, 80)?)?;
                    ensure!(selection < 12, "Circus character suffix byte extent");
                    let suffix = row[9 + selection / 4].to_le_bytes()[selection % 4];
                    ensure!(
                        suffix.is_ascii()
                            && !suffix.is_ascii_control()
                            && !b"/\\,:".contains(&suffix),
                        "Circus character resource suffix"
                    );
                    let prefix = format!("{prefix}{}", suffix as char);
                    let mut variant = self.state.value(3, 81)?;
                    if variant == 0 && row[5] == 0 {
                        variant = 1;
                    }
                    if variant == 1 && row[6] == 0 {
                        variant = 2;
                    }
                    let pose = self.state.value(3, 82)?;
                    // Native409320 leaves the old sprite unchanged for invalid
                    // variant/pose selections; it does not generate an image.
                    if !(0..3).contains(&variant) || !(0..2).contains(&pose) {
                        return Ok(complete(i.next));
                    }
                    let expression = self.state.value(3, 83)?;
                    let expression = if character == 9 {
                        *[1, 2, 3, 5, 6, 7, 9, 11, 12, 13, 14, 15, 0, 0, 0]
                            .get(usize::try_from(expression)?)
                            .context("Circus native character expression map")?
                    } else {
                        expression + 1
                    };
                    let n = self.state.value(3, 85)? != 0 || prefix.contains("hm_y");
                    let name = format!(
                        "bst\\{prefix}{}_{}{}{expression:02},bst\\{prefix}",
                        variant + 1,
                        if pose == 0 { 'a' } else { 'b' },
                        if n { "n" } else { "" }
                    );
                    let mut x = 400;
                    if name.contains("io_") {
                        if name.contains("1_a") || name.contains("1_b") {
                            x = 428;
                        }
                        if name.contains("io_e") {
                            x -= 238;
                        }
                    }
                    let auxiliary=self.state.native_live.character_auxiliary(&name)?;
                    let owner=(self.state.scene.clone(),i.offset);
                    if self.state.system_ops.character_preview_owner.as_ref()!=Some(&owner) {
                        let reset=crate::native_live::NativeLiveState::sprite_reset_seed(149)?;
                        if !host.sprite(&sprite_ops::SpriteRequest{commands:vec![sprite_ops::SpriteCommand{
                            seed:self.state.native_live.sprite_seed(149)?,
                            operations:vec![crysirror_scene::layer::queued::SequenceOperation::Reset{values:reset.values,retained:reset.retained}],
                        }]})? {return Ok(Some(CommandResult::Retry))}
                        self.state.native_live.reset_object(149)?;
                        self.sync_sprite_states(host)?;
                        self.state.system_ops.character_preview_owner=Some(owner);
                    }
                    let name=self.resource_variant(4,&name);
                    if !self.resource_load(host,4,&name,false)? {return Ok(Some(CommandResult::Retry))}
                    request = Some(SystemRequest::CharacterPreview {
                        name,
                        position: [x, -171],
                        auxiliary,
                        scale: 51,
                    });
                }
                204 => {
                    let mut character = self.state.value(3, 5)?;
                    if character == 5 {
                        character += self.state.value(3, 84)?;
                    }
                    let Some(reply) =
                        host.system(SystemRequest::CharacterRow(usize::try_from(character)?))?
                    else {
                        return Ok(Some(CommandResult::Retry));
                    };
                    let SystemReply::CharacterRow(row) = reply else {
                        bail!("Circus character bounds completion type")
                    };
                    let variant = self.state.value(3, 81)?;
                    let limit = match a[1] {
                        0 => Some(row[0]),
                        1 => Some(row[1]),
                        2 | 3 => {
                            let base = if a[1] == 2 { 5 } else { 2 };
                            let selected = if variant == 1 {
                                1
                            } else if variant == 2 {
                                2
                            } else {
                                0
                            };
                            let mut value = row[base + selected];
                            if value == 0 && variant == 0 {
                                value = row[base + 1]
                            } else if value == 0 && variant == 1 {
                                value = row[base + 2]
                            }
                            Some(value)
                        }
                        _ => None,
                    };
                    let value = self.state.value(3, 0)?;
                    if limit.is_some_and(|limit| value >= limit)
                        || (a[1] == 4 && row[8] == 0)
                        || a[1] == 5
                    {
                        self.state.assign(3, 0, 99)?;
                    }
                }
                206 => {
                    let mut enabled = [false; 5];
                    for (n, v) in enabled.iter_mut().enumerate() {
                        *v = self.state.value(100, 102 + n as i16)? != 0;
                    }
                    let choice = if let Some(first) = enabled.iter().position(|v| *v) {
                        loop {
                            if self.state.system_ops.random_attempts >= 10000 {
                                break first as i32;
                            }
                            let Some(draw) = host.random(5)? else {
                                return Ok(Some(CommandResult::Retry));
                            };
                            ensure!((0..5).contains(&draw), "Circus CRT random choice range");
                            self.state.system_ops.random_attempts += 1;
                            if enabled[draw as usize] {
                                break draw;
                            }
                        }
                    } else {
                        5
                    };
                    self.state.assign(5, 0, choice)?;
                    self.state.system_ops.random_attempts = 0;
                }
                211 => {
                    for n in (50..=57).chain(60..=69) {
                        self.state.assign(5, n, 0)?;
                    }
                }
                // 40A2E0's default explicitly returns without a state change.
                _ => {}
            },
            _ => return Ok(None),
        }
        if let Some(request) = request {
            let account=matches!(request,SystemRequest::Account {..});
            let reply=if account {
                if let Some((pc,reply))=&self.state.system_ops.account_reply {
                    ensure!(*pc==i.offset,"Circus account commit ownership");reply.clone()
                } else {
                    let Some(reply)=host.system(request)? else{return Ok(Some(CommandResult::Retry));};
                    self.state.system_ops.account_reply=Some((i.offset,reply.clone()));reply
                }
            } else {let Some(reply)=host.system(request)? else{return Ok(Some(CommandResult::Retry));};reply};
            match reply {
                SystemReply::Done => {
                    if i.opcode == 0xa9 && a[3] == 0 {
                        let (pc, value) = self
                            .state
                            .system_ops
                            .incremented_seek
                            .take()
                            .context("Circus source seek increment missing")?;
                        ensure!(pc == i.offset, "Circus source seek increment owner");
                        self.state.assign(99, 187, value)?;
                    }
                }
                SystemReply::Hour(hour) => {
                    ensure!(hour < 24, "Circus local clock hour");
                    self.state.assign(3, 0, hour.into())?;
                }
                SystemReply::Account { status, system, .. } => {
                    if let Some(bytes)=system {
                        //426300 merges the live preference prefix; it writes
                        //the imported110..114 slots but does not force an
                        //extra system.dat publication here.
                        self.state.system_data=SystemData::try_from(bytes)?;
                    }
                    if i.opcode == 0x18 {
                        self.state
                            .assign(99, 204, i32::from(status))?;
                        self.state
                            .native_flags
                            .insert(272084, i32::from(status));
                    }
                    self.state.system_ops.account_reply=None;
                }
                SystemReply::Cancelled => {
                    self.state.halted = true;
                    return Ok(Some(CommandResult::Complete {
                        next: i.next,
                        outcome: Outcome::Halted,
                    }));
                }
                SystemReply::EndingComplete(index) => {
                    ensure!(
                        matches!(index, 110 | 111),
                        "Circus ending completion preference"
                    );
                    self.state.assign(100, i16::from(index), 1)?;
                }
                _ => bail!("Circus system completion type"),
            }
        }
        if i.opcode==0x1e && a[0]==203 {
            self.sync_sprite_states(host)?;
            let dimensions=host.image_dimensions(4)?;
            let crop=dimensions.map(|(_,logical)|[0,0,logical[0] as i32,logical[1] as i32]);
            self.state.native_live.bind_resource(4,149,crop,dimensions)?;
            self.state.system_ops.character_preview_owner=None;
            self.state.native_live.character_preview_completed();
        }
        if i.opcode==0x1e && self.state.native_flags.get(&272075).copied().unwrap_or(0)!=0 {
            let declarations=self.state.dialogue.panel.finalized()?;
            let Some(reply)=host.system(SystemRequest::RebuildSelection{mode:self.state.system_ops.panel_mode,page:self.state.value(99,0)?,declarations})? else{return Ok(Some(CommandResult::Retry))};
            let SystemReply::Selection(declarations)=reply else{bail!("Circus selection rebuild reply")};
            self.state.dialogue.panel=declarations;
            self.state.system_ops.panel_mode=0;
            self.state.native_flags.insert(272075,0);
        }
        if i.opcode==0xa9 {self.state.native_live.audio_transport_completed();}
        Ok(complete(i.next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        draws: std::collections::VecDeque<Option<i32>>,
        calls: usize,
        replies: std::collections::VecDeque<Option<SystemReply>>,
        requests: Vec<SystemRequest>,
    }
    impl NativeHost for Host {
        fn scene(&mut self, _: &str) -> Result<Option<Arc<Program>>> {
            bail!("unexpected scene read")
        }
        fn emit(&mut self, _: &Event) -> Result<bool> {
            bail!("unexpected event")
        }
        fn save_exists(&mut self, _: u16) -> Result<bool> {
            bail!("unexpected save probe")
        }
        fn random(&mut self, modulus: u16) -> Result<Option<i32>> {
            assert_eq!(modulus, 5);
            self.calls += 1;
            self.draws.pop_front().context("unexpected random draw")
        }
        fn system(&mut self, request: SystemRequest) -> Result<Option<SystemReply>> {
            self.requests.push(request);
            self.replies
                .pop_front()
                .context("unexpected system request")
        }
    }
    #[test]
    fn callback_priority_is_native_key_order_and_one_sample_per_frame() {
        use crysirror_core::service::buttons as b;
        let mut machine = Machine::new("test");
        let mut p = program(0x4b, [0; 4]);
        p.instructions[0].arguments = Arguments::String("left".into());
        p.instructions.push(Instruction {
            offset: 12,
            next: 21,
            opcode: 0x4b,
            arguments: Arguments::String("up".into()),
            continuation: None,
        });
        machine.program = Some(Arc::new(p));
        machine.state.callbacks.insert(10712, "left".into());
        machine.state.callbacks.insert(10840, "up".into());
        machine.state.wait_click = true;
        let mut host = Host::default();
        host.replies.push_back(Some(SystemReply::CallbackInput {
            pressed: b::UP | b::LEFT,
            fullscreen: None,
            idle: false,
        }));
        assert!(machine.poll_system_callbacks(&mut host, 4, false).unwrap());
        assert_eq!(machine.state.pc, 3);
        assert!(!machine.state.wait_click);
        assert!(machine.poll_system_callbacks(&mut host, 4, false).unwrap());
        assert_eq!(host.requests.len(), 1);
    }
    #[test]
    fn callback_declarations_keep_unknown_native_noop_and_idle_timer() {
        let mut machine = Machine::new("test");
        let p = program(0x43, [16, 0, 0, 0]);
        let mut host = Host::default();
        host.replies.push_back(Some(SystemReply::Done));
        machine
            .execute_system(
                &p,
                &p.instructions[0],
                [16, 0, 0, 0, 0, 0, 0, 0],
                "idle",
                &mut host,
                0,
            )
            .unwrap();
        assert_eq!(
            machine.state.callbacks.get(&11352).map(String::as_str),
            Some("idle")
        );
        assert_eq!(host.requests, [SystemRequest::CallbackTimer(60000)]);
        machine
            .execute_system(
                &p,
                &p.instructions[0],
                [99, 0, 0, 0, 0, 0, 0, 0],
                "unused",
                &mut host,
                0,
            )
            .unwrap();
        assert_eq!(machine.state.callbacks.len(), 1);
    }
    fn program(opcode: u8, words: [i16; 4]) -> Program {
        Program {
            profile: Profile::Dc4PlusHarmony,
            code_file_offset: 0,
            labels: vec![],
            label_metadata: vec![],
            raw_strings: Default::default(),
            instructions: vec![Instruction {
                offset: 3,
                next: 12,
                opcode,
                arguments: Arguments::Words(words),
                continuation: None,
            }],
        }
    }
    fn run(
        machine: &mut Machine,
        host: &mut Host,
        opcode: u8,
        words: [i16; 4],
    ) -> Result<Option<CommandResult>> {
        let p = program(opcode, words);
        let mut args = [0; 8];
        args[..4].copy_from_slice(&words);
        machine.execute_system(&p, &p.instructions[0], args, "", host, 0)
    }
    #[test]
    fn random_choice_resumes_after_pending_without_recording_script_random_history() {
        let mut machine = Machine::new("test");
        machine.state.assign(100, 104, 1).unwrap();
        let mut host = Host {
            draws: [Some(0), None, Some(2)].into(),
            calls: 0,
            ..Default::default()
        };
        assert!(matches!(
            run(&mut machine, &mut host, 0x1e, [206, 0, 0, 0]).unwrap(),
            Some(CommandResult::Retry)
        ));
        assert_eq!(machine.state.value(5, 0).unwrap(), 0);
        assert!(matches!(
            run(&mut machine, &mut host, 0x1e, [206, 0, 0, 0]).unwrap(),
            Some(CommandResult::Complete { .. })
        ));
        assert_eq!(machine.state.value(5, 0).unwrap(), 2);
        assert_eq!(host.calls, 3);
        assert!(machine.state.random_history.is_empty());
    }
    #[test]
    fn repeated_seek_wait_does_not_apply_native_percent_increment_twice() {
        let mut machine = Machine::new("test");
        machine.state.assign(99, 187, 40).unwrap();
        let mut host = Host {
            replies: [None, Some(SystemReply::Done)].into(),
            ..Default::default()
        };
        assert!(matches!(
            run(&mut machine, &mut host, 0xa9, [0; 4]).unwrap(),
            Some(CommandResult::Retry)
        ));
        assert_eq!(machine.state.value(99, 187).unwrap(), 40);
        assert!(matches!(
            run(&mut machine, &mut host, 0xa9, [0; 4]).unwrap(),
            Some(CommandResult::Complete { .. })
        ));
        assert_eq!(machine.state.value(99, 187).unwrap(), 41);
        assert_eq!(
            host.requests,
            vec![
                SystemRequest::SourceSeek {
                    channel: 0,
                    percent: 41
                };
                2
            ]
        );
    }
    #[test]
    fn calendar_uses_native_year_failure_and_monday_based_success() {
        let mut machine = Machine::new("test");
        let mut host = Host::default();
        run(&mut machine, &mut host, 0xb5, [7, 229, 0, 0]).unwrap();
        assert_eq!(machine.state.value(3, 93).unwrap(), 6);
        run(&mut machine, &mut host, 0xb5, [2024, 0, 0, 99]).unwrap();
        run(&mut machine, &mut host, 0xb5, [7, 229, 0, 0]).unwrap();
        assert_eq!(machine.state.value(3, 93).unwrap(), 3);
        run(&mut machine, &mut host, 0xb5, [7, 230, 0, 0]).unwrap();
        assert_eq!(machine.state.value(3, 93).unwrap(), 4);
    }
    #[test]
    fn preference_comparison_ignores_only_native_auto_mode_byte() {
        let mut machine = Machine::new("test");
        let mut host = Host::default();
        run(&mut machine, &mut host, 0x1e, [18, 0, 0, 0]).unwrap();
        machine.state.assign(100, 54, 1).unwrap();
        run(&mut machine, &mut host, 0x1e, [18, 1, 0, 0]).unwrap();
        assert_eq!(machine.state.value(3, 0).unwrap(), 0);
        machine.state.assign(100, 55, 1).unwrap();
        run(&mut machine, &mut host, 0x1e, [18, 1, 0, 0]).unwrap();
        assert_eq!(machine.state.value(3, 0).unwrap(), 1);
        assert_eq!(machine.state.value(100, 54).unwrap(), 1);
    }
    #[test]
    fn variadic_sum_leaves_the_pc_on_native_marker_and_wraps_destination_byte() {
        let mut machine = Machine::new("test");
        let mut host = Host::default();
        machine.state.assign(5, 0, 250).unwrap();
        machine.state.assign(5, 1, 10).unwrap();
        let mut p = program(0x84, [3, 0, 0, 0]);
        for (offset, opcode, args) in [
            (12, 0x20, [5, 0, 0, 0]),
            (21, 0x20, [5, 1, 0, 0]),
            (30, 2, [0; 4]),
        ] {
            p.instructions.push(Instruction {
                offset,
                next: offset + 9,
                opcode,
                arguments: Arguments::Words(args),
                continuation: None,
            });
        }
        let result = machine
            .execute_system(
                &p,
                &p.instructions[0],
                [3, 0, 0, 0, 0, 0, 0, 0],
                "",
                &mut host,
                0,
            )
            .unwrap();
        assert!(matches!(
            result,
            Some(CommandResult::Complete { next: 30, .. })
        ));
        assert_eq!(machine.state.value(3, 0).unwrap(), 4);
    }
    #[test]
    fn scoped_preference_reset_preserves_names_history_and_audio_source_tail() {
        let mut bytes = SystemData::default().bytes().to_vec();
        bytes[184] = 77;
        bytes[3000] = 88;
        bytes[17640] = 99;
        bytes[17720] = 99;
        bytes[SETTINGS_BYTES + 2004] = 42;
        let old = SystemData::try_from(bytes).unwrap();
        let reset = reset_data(&old, Some(4)).unwrap();
        assert_eq!(reset.bytes()[184], 77);
        assert_eq!(reset.bytes()[3000], 88);
        assert_eq!(reset.bytes()[17640], 0);
        assert_eq!(reset.bytes()[17720], 99);
        assert_eq!(reset.bytes()[SETTINGS_BYTES + 2004], 42);
        let reset = reset_data(&old, None).unwrap();
        assert_eq!(reset.bytes()[184], 77);
        assert_eq!(reset.bytes()[3000], 88);
        assert_eq!(reset.bytes()[17720], 0);
    }
    #[test]
    fn restore_patches_only_native_first_preference_block() {
        let mut bytes = SystemData::default().bytes().to_vec();
        bytes[SETTINGS_BYTES] = 71;
        let old = SystemData::try_from(bytes).unwrap();
        let new = patch_prefix(&old, &vec![23; SETTINGS_BYTES]).unwrap();
        assert_eq!(new.bytes()[200], 23);
        assert_eq!(new.bytes()[SETTINGS_BYTES], 71);
        assert!(patch_prefix(&old, &[0; 8]).is_err());
    }
}
