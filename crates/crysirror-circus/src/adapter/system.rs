//! System calls use client-scoped engine services. Static executable data is
//! parsed on a worker, fingerprinted, and never mapped or executed.
use super::*;
use crate::machine::system_ops::{AccountOperation, SystemReply, SystemRequest};
use crysirror_core::service::{
    CivilDaylightRules, CivilTimeQuery, CivilTimeState, CivilTimeZoneProfile, CivilTimeZoneQuery,
    CivilTimeZoneState, CivilTransition, CivilTransitionDay,
};
#[path = "system_ending.rs"]
mod ending;
#[path = "system_license.rs"]
mod license;
#[path = "system_selection.rs"]
mod selection;
#[path = "system_cloud.rs"]
mod cloud;
#[path = "system_cloud_codec.rs"]
mod cloud_codec;
#[path="system_gestures.rs"]
mod gestures;

#[derive(Clone)]
struct NativeMetadata {
    cloud:cloud_codec::CloudMetadata,
    default_names: [Vec<u8>; 2],
    history_names:Vec<String>,
    save_crops: [[i16; 4]; 3],
    save_titles: Vec<(String, String)>,
    catalogue: Vec<u16>,
    characters: Vec<[i32; 12]>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        now: u64,
        fullscreen: Option<bool>,
        commands: Vec<Value>,
    }
    impl crysirror_core::service::FileSystem for Host {
        fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            Err(crysirror_core::Error::Unsupported("unexpected read"))
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported("unexpected write"))
        }
    }
    impl HostServices for Host {
        fn submit(
            &mut self,
            _: ClientId,
            _: &[crysirror_core::state::Command],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported(
                "unexpected legacy commands",
            ))
        }
        fn now_ms(&self) -> u64 {
            self.now
        }
        fn window_focused(&self, _: ClientId) -> crysirror_core::Result<Option<bool>> {
            Ok(Some(true))
        }
        fn window_fullscreen(&self, client: ClientId) -> crysirror_core::Result<Option<bool>> {
            assert_eq!(client, ClientId(7));
            Ok(self.fullscreen)
        }
        fn submit_engine(
            &mut self,
            client: ClientId,
            _: u32,
            bytes: &[u8],
        ) -> crysirror_core::Result<()> {
            assert_eq!(client, ClientId(7));
            self.commands.push(serde_json::from_slice(bytes).unwrap());
            Ok(())
        }
    }
    fn call(
        services: &mut Services,
        host: &mut Host,
        request: SystemRequest,
        buttons: u64,
    ) -> Result<Option<SystemReply>> {
        Bridge {
            services,
            host,
            client: ClientId(7),
            font: "test",
            input: InputFrame {
                buttons_pressed: buttons,
                ..Default::default()
            },
        }
        .native_system(request)
    }
    #[test]
    fn native_idle_timer_uses_strict_wrapping_pointer_inactivity() {
        let mut services = Services::default();
        let mut host = Host::default();
        assert_eq!(
            call(
                &mut services,
                &mut host,
                SystemRequest::CallbackTimer(60000),
                0
            )
            .unwrap(),
            Some(SystemReply::Done)
        );
        host.now = 60000;
        assert!(matches!(
            call(&mut services, &mut host, SystemRequest::CallbackInput, 0).unwrap(),
            Some(SystemReply::CallbackInput { idle: false, .. })
        ));
        host.now = 60001;
        assert!(matches!(
            call(
                &mut services,
                &mut host,
                SystemRequest::CallbackInput,
                crysirror_core::service::buttons::ENTER
            )
            .unwrap(),
            Some(SystemReply::CallbackInput { idle: true, .. })
        ));
        assert_eq!(
            call(&mut services, &mut host, SystemRequest::CallbackTimer(0), 0).unwrap(),
            Some(SystemReply::Done)
        );
        host.now = 900000;
        assert!(matches!(
            call(&mut services, &mut host, SystemRequest::CallbackInput, 0).unwrap(),
            Some(SystemReply::CallbackInput { idle: false, .. })
        ));
    }
    #[test]
    fn fullscreen_waits_for_actual_presenter_observation_and_submits_once() {
        let mut services = Services::default();
        let mut host = Host {
            fullscreen: Some(false),
            ..Default::default()
        };
        assert_eq!(
            call(&mut services, &mut host, SystemRequest::Fullscreen(true), 0).unwrap(),
            None
        );
        assert_eq!(
            call(&mut services, &mut host, SystemRequest::Fullscreen(true), 0).unwrap(),
            None
        );
        assert_eq!(host.commands.len(), 1);
        assert_eq!(host.commands[0]["ui"][0]["op"], "set_fullscreen");
        host.fullscreen = Some(true);
        assert_eq!(
            call(&mut services, &mut host, SystemRequest::Fullscreen(true), 0).unwrap(),
            Some(SystemReply::Done)
        );
        assert_eq!(
            call(&mut services, &mut host, SystemRequest::WindowState, 0).unwrap(),
            Some(SystemReply::Fullscreen(true))
        );
        host.fullscreen = None;
        assert!(call(&mut services, &mut host, SystemRequest::WindowState, 0).is_err());
    }
    #[test]
    fn native_blank_panel_only_exits_on_space_in_a_later_frame() {
        let mut services = Services::default();
        let mut host = Host::default();
        let request = SystemRequest::Panel {
            mode: 3,
            parameter: 1,
            resume_pc: 12,
            duration_ms: 0,
            skip_unlocked: false,
        };
        assert_eq!(
            call(
                &mut services,
                &mut host,
                request.clone(),
                crysirror_core::service::buttons::SPACE
            )
            .unwrap(),
            None
        );
        host.now = 16;
        assert_eq!(
            call(
                &mut services,
                &mut host,
                request.clone(),
                crysirror_core::service::buttons::ESCAPE
            )
            .unwrap(),
            None
        );
        assert_eq!(
            call(
                &mut services,
                &mut host,
                request,
                crysirror_core::service::buttons::SPACE
            )
            .unwrap(),
            Some(SystemReply::Done)
        );
    }
}
type ThumbnailResult = std::result::Result<Vec<Option<ImageSurface>>, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowRectOwner {
    Restore,
    Enlarge,
    ExitFullscreen,
}
struct NativeWindowGate(i32);
impl Default for NativeWindowGate {
    fn default()->Self {Self(1)} //43A2F0 renderer+24, not zeroed Buf state.
}
#[derive(Default)]
pub(super) struct SystemServices {
    gestures:gestures::NativeGestures,
    clock: Option<Box<dyn CivilTimeQuery>>,
    timezone_query: Option<Box<dyn CivilTimeZoneQuery>>,
    timezone_profile: Option<CivilTimeZoneProfile>,
    metadata: Option<NativeMetadata>,
    metadata_worker: Option<Receiver<std::result::Result<NativeMetadata, String>>>,
    //43A2F0/438AF0 retain windowed308..320 independently of fullscreen.
    window_current: Option<crysirror_core::service::WindowGeometry>,
    //43A460/6E0/840's388..400 rectangle, applied when leaving fullscreen.
    window_restore: Option<crysirror_core::service::WindowGeometry>,
    //410030/439BA0 use the default1280x720 client, not saved WinSizex/y.
    window_default_outer: Option<[u32; 2]>,
    window_padding: Option<[u32; 2]>,
    window_rect_pending: Option<(
        WindowRectOwner,
        crysirror_core::service::WindowGeometry,
        [u32; 2],
    )>,
    fullscreen: bool,
    pending_fullscreen: Option<bool>,
    native_fullscreen_mode: i16,
    pending_native_fullscreen_mode: Option<i16>,
    window_transition_gate:NativeWindowGate,
    forced_window_gate:Option<i32>,
    presentation_aspect:Option<bool>,
    panel_started: Option<(u8, u64)>,
    thumbnail_worker: Option<(i32, Receiver<ThumbnailResult>)>,
    thumbnail_page: Option<i32>,
    license: Option<license::LicenseDialog>,
    account: Option<cloud::AccountJob>,
    music_seek: Option<(u32, i32, u64)>,
    ending: Option<ending::Ending>,
    selection_worker: Option<selection::SelectionWorker>,
    selection_commit: Option<selection::SelectionCommit>,
    pub(super) pointer_motion_ticks:u8,
    native_cursor_requested:bool,
    cursor_input_revision:Option<u64>,
    cursor_activity:Option<u64>,
    cursor_active:Option<bool>,
    cursor_auto_hide:Option<bool>,
    cursor_visible:Option<bool>,
    callback_idle_ms: u32,
    callback_idle_since: u32,
    callback_pointer: Option<[i32; 2]>,
    callback_input_revision: Option<u64>,
    //Core normalizes raw pulses/releases; the native OR-stock below is
    //retained until the admitted native frame, not each host call, finishes.
    callback_input_buffer:Option<crysirror_core::input_buffer::InputBuffer>,
    callback_buttons_down: u64,
    pub(super) callback_buttons_pressed:u64,
    pub(super) callback_input_enabled:bool,
    callback_releases: u64,
}
impl SystemServices {
    pub(in crate::adapter) fn gesture_confirming(&self)->bool {self.gestures.confirming()}
    pub(super) fn window_request_pending(&self) -> bool {
        self.pending_fullscreen.is_some()
            || self.pending_native_fullscreen_mode.is_some()
            || self.window_rect_pending.is_some()
    }
    pub(super) fn account_pending(&self)->bool {self.account.is_some()}
    ///Some(false) is an active modal producer still preparing resources;
    ///it must mask any old menu underneath rather than release its fence.
    pub(super) fn modal_ui_waiting(&self) -> Option<bool> {
        if self.gestures.waiting(){return Some(true)}
        if self.selection_worker.is_some() {
            return Some(false);
        }
        if let Some(ending) = &self.ending {
            return Some(ending.ui_waiting());
        }
        if let Some(license) = &self.license {
            return Some(license.ui_waiting());
        }
        if let Some(account)=&self.account {return Some(account.ui_waiting());}
        if self.panel_started.is_some() {
            return Some(true);
        }
        None
    }
    pub(super) fn observe_window_geometry(
        &mut self,
        geometry: Option<crysirror_core::service::WindowGeometry>,
        fullscreen: Option<bool>,
    ) {
        if fullscreen != Some(false) || self.pending_fullscreen == Some(true) {
            return;
        }
        if let Some(geometry) = geometry {
            self.window_current = Some(geometry);
            if self.window_restore.is_none() {
                self.window_restore = Some(geometry);
            }
            if self.window_padding.is_none() {
                if let (Some(x), Some(y)) = (
                    geometry.outer_size[0].checked_sub(geometry.inner_size[0]),
                    geometry.outer_size[1].checked_sub(geometry.inner_size[1]),
                ) {
                    if let (Some(w), Some(h)) = (1280u32.checked_add(x), 720u32.checked_add(y)) {
                        self.window_padding = Some([x, y]);
                        self.window_default_outer = Some([w, h]);
                    }
                }
            }
        }
    }
}
//4667C6's environment grammar is the original CRT's three-character name,
//signed H[:M[:S]], optional DST name. It does not parse POSIX comma rules.
//Without Win32 TZI,466D03 selects the CRT's pre/post2007 US fallback rules.
fn native_calendar_profile(host: &CivilTimeZoneProfile, year: i32) -> Result<CivilTimeZoneProfile> {
    let Some(tz) = host.environment_tz.as_ref() else {
        return Ok(host.clone());
    };
    let bytes = tz.as_bytes();
    ensure!(
        bytes.len() >= 3 && bytes.is_ascii(),
        "Circus native TZ override is not a three-byte CRT name"
    );
    let mut at = 3;
    let negative = bytes.get(at) == Some(&b'-');
    at += usize::from(negative);
    let atol = |at: usize| -> Result<i32> {
        let mut cursor = at;
        while bytes.get(cursor).is_some_and(|b| b.is_ascii_whitespace()) {
            cursor += 1;
        }
        let sign = match bytes.get(cursor) {
            Some(b'-') => {
                cursor += 1;
                -1i32
            }
            Some(b'+') => {
                cursor += 1;
                1
            }
            _ => 1,
        };
        let mut value = 0i32;
        while let Some(byte @ b'0'..=b'9') = bytes.get(cursor).copied() {
            value = value
                .checked_mul(10)
                .and_then(|value| value.checked_add(i32::from(byte - b'0')))
                .context("Circus CRT TZ offset overflow")?;
            cursor += 1;
        }
        value
            .checked_mul(sign)
            .context("Circus CRT TZ sign overflow")
    };
    let mut bias = atol(at)?
        .checked_mul(3600)
        .context("Circus CRT TZ hour overflow")?;
    while bytes
        .get(at)
        .is_some_and(|b| *b == b'+' || b.is_ascii_digit())
    {
        at += 1;
    }
    if bytes.get(at) == Some(&b':') {
        at += 1;
        bias = bias
            .checked_add(
                atol(at)?
                    .checked_mul(60)
                    .context("Circus CRT TZ minute overflow")?,
            )
            .context("Circus CRT TZ offset overflow")?;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if bytes.get(at) == Some(&b':') {
            at += 1;
            bias = bias
                .checked_add(atol(at)?)
                .context("Circus CRT TZ second overflow")?;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
        }
    }
    if negative {
        bias = bias
            .checked_neg()
            .context("Circus CRT TZ offset overflow")?;
    }
    let day = |month, week| CivilTransition {
        day: CivilTransitionDay::MonthWeekday {
            month,
            week,
            weekday: 0,
        },
        seconds: 7200,
    };
    let daylight = (at < bytes.len()).then(|| CivilDaylightRules {
        start: day(
            if year < 2007 { 4 } else { 3 },
            if year < 2007 { 1 } else { 2 },
        ),
        end: day(
            if year < 2007 { 10 } else { 11 },
            if year < 2007 { 5 } else { 1 },
        ),
    });
    Ok(CivilTimeZoneProfile {
        standard_bias_seconds: bias,
        daylight_bias_seconds: -3600,
        daylight,
        environment_tz: None,
    })
}

impl Bridge<'_> {
    ///Source SetWindowPos takes outer dimensions. Shared requests take client
    ///dimensions, so translate using the actual current decoration padding;
    ///a fullscreen transition may change that padding before observation ACK.
    fn apply_window_rectangle(
        &mut self,
        owner: WindowRectOwner,
        target: crysirror_core::service::WindowGeometry,
    ) -> Result<bool> {
        let observed = self
            .host
            .window_geometry(self.client)?
            .context("Circus presenting window geometry unavailable")?;
        let position = target
            .outer_position
            .context("Circus saved outer window position unavailable")?;
        if let Some((expected, rect, _)) = self.services.system.window_rect_pending {
            ensure!(
                owner == expected && rect == target,
                "Circus window rectangle request ownership"
            );
        }
        if observed.outer_size == target.outer_size && observed.outer_position == Some(position) {
            self.services.system.window_rect_pending = None;
            self.services.system.window_current = Some(observed);
            return Ok(true);
        }
        ensure!(
            observed.outer_position.is_some(),
            "Circus host cannot observe outer window position"
        );
        let mut inner = [0; 2];
        for axis in 0..2 {
            let padding = observed.outer_size[axis]
                .checked_sub(observed.inner_size[axis])
                .context("Circus invalid non-client window padding")?;
            inner[axis] = target.outer_size[axis]
                .checked_sub(padding)
                .filter(|v| *v != 0)
                .context("Circus requested outer window rectangle is smaller than its border")?;
        }
        if self
            .services
            .system
            .window_rect_pending
            .as_ref()
            .is_none_or(|(_, _, last)| *last != inner)
        {
            self.submit(
                vec![],
                vec![
                    json!({"op":"set_window_size","size":inner}),
                    json!({"op":"set_window_position","position":position}),
                ],
            )?;
            self.services.system.window_rect_pending = Some((owner, target, inner));
        }
        Ok(false)
    }
    ///442EC0 produces state4 on key-up. Sample only at a native input/frame
    ///boundary; retries of one admitted input revision reuse that edge. Known
    ///blocking native dialogs suspend these global callbacks and clear their
    ///held baseline, so modal-masked input cannot masquerade as a release.
    pub(super) fn sample_callback_releases(&mut self) -> Result<u64> {
        let input = self.host.input_observation(self.client)?;
        ensure!(
            input.client == self.client,
            "Circus foreign callback input observation"
        );
        let ui_owned = self.host.ui_input_owned(self.client)?;
        //433E50 LABEL285 still observes the original key stock after the
        //script's menus/panel have polled it. Only a different modal owner
        //blocks that stock; our full-canvas menu roots are not native dialogs.
        let mut own_menu_eligible=false;
        if ui_owned {
            for scope in 0..4u32 {
                let eligible:Option<bool>=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiInputEligible(0x36ff_fffdu32-scope*0x1_0000))?)?;
                own_menu_eligible|=eligible==Some(true);
            }
        }
        let state = &mut self.services.system;
        let same_revision=state.callback_input_revision==Some(input.revision);
        let blocked = (ui_owned&&!own_menu_eligible)
            || input.focused == Some(false)
            || state.license.is_some()
            || state.account.is_some()
            || state.ending.is_some()
            || state.panel_started.is_some();
        if same_revision&&state.callback_input_enabled==!blocked {return Ok(state.callback_releases)}
        state.callback_input_enabled=!blocked;
        if blocked {
            //442EC0's disabled controller clears key stock. Reset its held
            //baseline without fabricating key-up for the obscured VM owner.
            state.callback_input_buffer=None;
            state.gestures.finish_input_frame();
            state.callback_buttons_pressed=0;state.callback_releases=0;
            state.callback_buttons_down=0;
            state.gestures.observe_stock(0,0,input.time_ms,false);
        }else{
            if state.callback_input_buffer.is_none() {
                state.callback_input_buffer=Some(crysirror_core::input_buffer::InputBuffer::new(self.client.0,1,1)?);
            }
            let buffer=state.callback_input_buffer.as_mut().unwrap();
            buffer.push(self.client.0,input.time_ms,InputFrame{
                buttons_down:input.buttons_down,
                //Re-enabling the controller re-arms genuinely held keys,
                //not an already consumed explicit pulse from this revision.
                buttons_pressed:if same_revision{0}else{input.buttons_pressed},
                pointer_x:input.pointer.map_or(0,|point|point[0]),
                pointer_y:input.pointer.map_or(0,|point|point[1]),
            })?;
            let batch=buffer.read(self.client.0,1)?;
            let mut pressed=0;
            for sample in batch.samples {
                pressed|=sample.frame.buttons_pressed;
                state.callback_buttons_pressed|=sample.frame.buttons_pressed;
                state.callback_releases|=sample.released;
            }
            //The receipt never leaves this client-scoped owner. Normalized
            //edges are now held in442EC0's native stock, so no unbounded input
            //journal grows while a 191 frame-admission gate is closed.
            if let Some(receipt)=batch.receipt {buffer.acknowledge(self.client.0,receipt)?;}
            state.callback_buttons_down=input.buttons_down;
            let press_time=if same_revision{self.host.now_ms()}else{input.time_ms};
            state.gestures.observe_stock(input.buttons_down,pressed,press_time,true);
        }
        state.callback_input_revision = Some(input.revision);
        Ok(state.callback_releases)
    }
    ///Call only after the admitted native frame reaches its presentation
    ///fence. Keep InputBuffer's observed held baseline/revision, so another
    ///host call cannot replay that frame's physical pulse or invent a release.
    pub(in crate::adapter) fn native_buttons_pressed(&self)->u64 {self.services.system.callback_buttons_pressed}
    ///433E50 process6 calls422C50; its manual branch442D60 is exactly
    ///native slots1/7/16 in stock4 (442EC0's release edge). The same442D60
    ///predicate interrupts skippable process21 waits. Space belongs to the
    ///433AA0 visibility policy; function keys belong only to native actions.
    pub(in crate::adapter) fn native_dialogue_advance(&self)->bool {
        use crysirror_core::service::buttons;
        self.services.system.callback_input_enabled
            && self.services.system.callback_releases
                & (buttons::MOUSE_LEFT|buttons::ENTER|buttons::PAGE_DOWN)!=0
    }
    pub(in crate::adapter) fn finish_native_input_frame(&mut self) {
        self.services.system.callback_buttons_pressed=0;
        self.services.system.callback_releases=0;
        self.services.system.gestures.finish_input_frame();
    }
    fn native_thumbnails(&mut self, release: bool, page: i32) -> Result<Option<SystemReply>> {
        if release {
            self.services.system.thumbnail_worker = None;
            for slot in 230..240 {
                if let Some(image) = self.services.images.remove(&slot) {
                    self.host
                        .release_image_resource(self.client, image.path.relative())?;
                }
                self.services.image_extents.remove(&slot);
                self.services.image_names.remove(&slot);
            }
            self.services.system.thumbnail_page = None;
            return Ok(Some(SystemReply::Done));
        }
        ensure!((0..=6553).contains(&page), "Circus thumbnail page range");
        if let Some((expected, receiver)) = &self.services.system.thumbnail_worker {
            ensure!(*expected == page, "Circus thumbnail completion ownership");
            match receiver.try_recv() {
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => bail!("Circus thumbnail worker disconnected"),
                Ok(result) => {
                    self.services.system.thumbnail_worker = None;
                    let decoded = result.map_err(anyhow::Error::msg)?;
                    for (n, image) in decoded.into_iter().enumerate() {
                        let slot = 230 + n as u8;
                        if let Some(old) = self.services.images.remove(&slot) {
                            self.host
                                .release_image_resource(self.client, old.path.relative())?;
                        }
                        self.services.image_extents.remove(&slot);
                        self.services.image_names.remove(&slot);
                        if let Some(image) = image {
                            let name = format!("DC4PHDL{:02}.png", 3 + 10 * page + n as i32);
                            let path = self.host.upload_rgba(
                                self.client,
                                0x3500_0000 + u32::from(slot),
                                image.width,
                                image.height,
                                &image.rgba,
                            )?;
                            self.services
                                .image_extents
                                .insert(slot, [image.width, image.height]);
                            self.services.images.insert(
                                slot,
                                UploadedImage {
                                    path,
                                    size: [image.width, image.height],
                                    opaque: image.rgba.chunks_exact(4).all(|p| p[3] == 255),
                                },
                            );
                            self.services.image_names.insert(slot, name);
                        }
                    }
                    self.services.system.thumbnail_page = Some(page);
                    return Ok(Some(SystemReply::Done));
                }
            }
        }
        let storage = self
            .services
            .storage
            .as_ref()
            .context("Circus thumbnail storage capability")?
            .clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("circus-thumbnails".into())
            .spawn(move || {
                let result = (|| -> Result<Vec<Option<ImageSurface>>> {
                    let mut decoded = Vec::with_capacity(10);
                    for n in 0..10 {
                        let path = VfsPath::new(
                            Mount::Save,
                            &format!("DC4PHDL{:02}.png", 3 + 10 * page + n),
                        )?;
                        if storage.metadata(Mount::Save, path.relative())?.is_none() {
                            decoded.push(None);
                            continue;
                        }
                        let reader = storage.open_read(&path)?;
                        let bytes =
                            crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)?;
                        // Decoding and bounded pixel allocation are shared-image
                        // algorithms; this worker only resolves native slot names.
                        decoded.push(Some(crysirror_image::decode_standard(&bytes)?));
                    }
                    Ok(decoded)
                })();
                let _ = sender.send(result.map_err(|e| format!("{e:#}")));
            })?;
        self.services.system.thumbnail_worker = Some((page, receiver));
        Ok(None)
    }
    fn system_metadata(&mut self) -> Result<Option<NativeMetadata>> {
        if let Some(data) = &self.services.system.metadata {
            return Ok(Some(data.clone()));
        }
        if let Some(receiver) = &self.services.system.metadata_worker {
            match receiver.try_recv() {
                Ok(result) => {
                    self.services.system.metadata_worker = None;
                    let data = result.map_err(anyhow::Error::msg)?;
                    self.services.system.metadata = Some(data.clone());
                    return Ok(Some(data));
                }
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => {
                    bail!("Circus native metadata worker disconnected")
                }
            }
        }
        let storage = self
            .services
            .storage
            .as_ref()
            .context("Circus system storage capability")?
            .clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("circus-system-data".into())
            .spawn(move || {
                let result = (|| -> Result<NativeMetadata> {
                    let reader = storage.open_read(&VfsPath::new(Mount::Asset, "DC4PHDL.exe")?)?;
                    let bytes = crysirror_resource::read_all(
                        reader.as_ref(),
                        crysirror_resource::pe::MAX_PE_BYTES as u64,
                    )?;
                    let pe = crysirror_resource::pe::PeData::parse(&bytes)?;
                    let fingerprint = pe
                        .sha256()
                        .iter()
                        .map(|v| format!("{v:02x}"))
                        .collect::<String>();
                    ensure!(
                        fingerprint
                            == "178ee9a5e34356b11942213e2e32a34afe4933c90d563bf5a3d6402250f54ab7",
                        "Circus unknown native system data profile"
                    );
                    let mut catalogue = Vec::new();
                    for n in 0..=400 {
                        let id = i32::from_le_bytes(pe.va_slice(0x4b4458 + n * 8, 4)?.try_into()?);
                        if id == -1 {
                            break;
                        }
                        ensure!(
                            n < 400 && (1..400).contains(&id),
                            "Circus native music table range"
                        );
                        catalogue.push(id as u16);
                    }
                    ensure!(
                        !catalogue.is_empty() && catalogue.len() < 400,
                        "Circus unterminated native music catalogue"
                    );
                    let mut characters = Vec::new();
                    for n in 0..12 {
                        let bytes = pe.va_slice(0x4b45b8 + n * 48, 48)?;
                        let mut row = [0; 12];
                        for (i, value) in row.iter_mut().enumerate() {
                            *value = i32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into()?);
                        }
                        characters.push(row);
                    }
                    //40A2E0 compares inline constants, not DWORD pointers.
                    //VM order is family/name at265288 then265268.
                    let default_names = [
                        pe.va_slice(0x48b934, 4)?.to_vec(),
                        pe.va_slice(0x48b92c, 4)?.to_vec(),
                    ];
                    let read_string = |address: u64| -> Result<String> {
                        let mut bytes = Vec::new();
                        for n in 0..260 {
                            let byte = pe.va_slice(address + n, 1)?[0];
                            if byte == 0 {
                                let (text, error) =
                                    encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes);
                                ensure!(!error, "Circus selection metadata encoding");
                                return Ok(text.into_owned());
                            }
                            bytes.push(byte);
                        }
                        bail!("Circus selection metadata string terminator")
                    };
                    let mut save_titles = Vec::new();
                    for n in 0..650u64 {
                        let raw = pe.va_slice(0x4b34e8 + n * 8, 8)?;
                        let key = u32::from_le_bytes(raw[..4].try_into()?);
                        if key == 0 {
                            break;
                        }
                        ensure!(n < 649, "Circus title alias terminator");
                        let value = u32::from_le_bytes(raw[4..].try_into()?);
                        save_titles
                            .push((read_string(u64::from(key))?, read_string(u64::from(value))?));
                    }
                    let mut save_crops = [[0; 4]; 3];
                    for (n, crop) in save_crops.iter_mut().enumerate() {
                        for (i, value) in crop.iter_mut().enumerate() {
                            *value = i16::try_from(i32::from_le_bytes(
                                pe.va_slice(0x4ae44c + (n * 16 + i * 4) as u64, 4)?
                                    .try_into()?,
                            ))?;
                        }
                    }
                    let mut history_names=Vec::new();
                    for n in 0..100u64 {
                        let row=pe.va_slice(0x4ae008+32*n,32)?;
                        if row[0]==0 {break}
                        let end=row.iter().position(|byte|*byte==0).context("Circus history name terminator")?;
                        let (name,error)=encoding_rs::SHIFT_JIS.decode_without_bom_handling(&row[..end]);
                        ensure!(!error,"Circus history name encoding");
                        history_names.push(name.into_owned());
                        ensure!(n<99,"Circus history name table terminator");
                    }
                    Ok(NativeMetadata {
                        history_names,
                        cloud:cloud_codec::CloudMetadata::from_pe(&pe)?,
                        default_names,
                        save_crops,
                        save_titles,
                        catalogue,
                        characters,
                    })
                })();
                let _ = sender.send(result.map_err(|e| format!("{e:#}")));
            })?;
        self.services.system.metadata_worker = Some(receiver);
        Ok(None)
    }
    ///442FF0 cursor/idle policy. This observes the owner's input capability,
    ///never desktop pixels. Physical activity is live even while a native
    ///resource continuation retains the admitted input revision. Returned
    ///deadline keeps one pending auto-hide wake.
    pub(super) fn update_native_cursor(&mut self,auto_hide:bool)->Result<Option<u64>> {
        let observation=self.host.input_observation(self.client)?;
        ensure!(observation.client==self.client,"Circus cursor foreign input owner");
        let now=self.host.now_ms();let tick=now as u32;
        let focused=self.host.window_focused(self.client)?.or(observation.focused).unwrap_or(true);
        let activity=self.host.pointer_activity(self.client)?;
        let active=focused&&observation.pointer_inside!=Some(false);
        let moved=activity.is_some()&&activity!=self.services.system.cursor_activity;
        self.services.system.cursor_activity=activity;
        let mut position_changed=false;
        if self.services.system.cursor_input_revision!=Some(observation.revision) {
            self.services.system.cursor_input_revision=Some(observation.revision);
            if focused {
                if let Some(position)=observation.pointer {
                    position_changed=self.services.system.callback_pointer!=Some(position);
                    self.services.system.callback_pointer=Some(position);
                }
            }
        }
        let policy_changed=self.services.system.cursor_auto_hide!=Some(auto_hide);
        let reactivated=active&&self.services.system.cursor_active!=Some(true);
        self.services.system.cursor_auto_hide=Some(auto_hide);
        self.services.system.cursor_active=Some(active);
        // The desktop owns cursor visibility. Restore it when auto-hide is
        // disabled, focus is lost, or the pointer leaves the client, rather
        // than carrying Win32's hidden ShowCursor state into the host.
        if !auto_hide||!active||moved||position_changed||policy_changed||reactivated {
            self.services.system.callback_idle_since=tick;
            self.services.system.native_cursor_requested=true;
        }
        let elapsed=tick.wrapping_sub(self.services.system.callback_idle_since);
        if auto_hide&&active&&elapsed>5000 {
            self.services.system.native_cursor_requested=false;
        }
        let visible=!auto_hide||!active||self.services.system.native_cursor_requested;
        if self.services.system.cursor_visible!=Some(visible) {
            self.submit(vec![],vec![json!({"op":"set_cursor_visible","visible":visible})])?;
            self.services.system.cursor_visible=Some(visible);
        }
        Ok((auto_hide&&active&&visible).then(||now.saturating_add(u64::from(5001u32.saturating_sub(elapsed)))))
    }
    pub(super) fn native_system(&mut self, request: SystemRequest) -> Result<Option<SystemReply>> {
        match request {
            SystemRequest::FramebufferStatus => Ok(Some(SystemReply::FramebufferComplete(
                self.poll_framebuffer_status()?,
            ))),
            SystemRequest::CallbackTimer(duration) => {
                self.services.system.callback_idle_ms = duration;
                self.services.system.callback_idle_since = self.host.now_ms() as u32;
                if self.services.system.cursor_input_revision.is_none(){
                    self.services.system.callback_pointer=Some([self.input.pointer_x,self.input.pointer_y]);
                }
                Ok(Some(SystemReply::Done))
            }
            SystemRequest::CallbackInput => {
                let releases = self.sample_callback_releases()?;
                let now = self.host.now_ms() as u32;
                let position = [self.input.pointer_x, self.input.pointer_y];
                if self.services.system.cursor_input_revision.is_none() && self.services.system.callback_pointer != Some(position) {
                    self.services.system.callback_pointer = Some(position);
                    self.services.system.callback_idle_since = now;
                }
                let timeout = self.services.system.callback_idle_ms;
                let idle = timeout != 0
                    && self
                        .host
                        .window_focused(self.client)?
                        .context("Circus idle callback window focus unavailable")?
                    && now.wrapping_sub(self.services.system.callback_idle_since) > timeout;
                let observed=self.host.window_fullscreen(self.client)?;
                let native_mode=observed.map(|fullscreen| {
                    if self.services.system.pending_native_fullscreen_mode.is_none()
                        &&fullscreen!=(self.services.system.native_fullscreen_mode!=0) {
                        self.services.system.native_fullscreen_mode=i16::from(fullscreen);
                    }
                    self.services.system.native_fullscreen_mode
                });
                Ok(Some(SystemReply::CallbackInput {
                    pressed: if self.host.window_focused(self.client)? == Some(false) {
                        0
                    } else {
                        releases
                    },
                    fullscreen:native_mode,
                    idle,
                }))
            }
            SystemRequest::Gesture(request)=>self.native_gesture(request),
            SystemRequest::VoiceRate(percent)=>Ok(self.native_voice_rate(percent)?.then_some(SystemReply::Done)),
            SystemRequest::WindowTransitionFlag(value)=>{
                if let Some(value)=value {self.services.system.window_transition_gate.0=value;}
                let value=if self.services.movie.active(){0}else{self.services.system.window_transition_gate.0};
                Ok(Some(SystemReply::WindowTransitionFlag(value)))
            }
            SystemRequest::PresentationAspect(preserve)=>{
                if self.services.system.presentation_aspect!=Some(preserve) {
                    self.submit(vec![],vec![json!({"op":"set_presentation_scale","mode":if preserve{"fit"}else{"stretch"}})])?;
                    self.services.system.presentation_aspect=Some(preserve);
                }
                Ok(Some(SystemReply::Done))
            }
            SystemRequest::WindowState | SystemRequest::NativeWindowMode => {
                let observed = self
                    .host
                    .window_fullscreen(self.client)?
                    .context("Circus presenting fullscreen observation unavailable")?;
                if self
                    .services
                    .system
                    .pending_native_fullscreen_mode
                    .is_none()
                    && observed != (self.services.system.native_fullscreen_mode != 0)
                {
                    // An external platform transition has no native signed
                    //request, so its conventional logical mode is0 or1.
                    self.services.system.native_fullscreen_mode = i16::from(observed);
                }
                self.services.system.fullscreen = observed;
                Ok(Some(
                    if matches!(request, SystemRequest::NativeWindowMode) {
                        SystemReply::NativeWindowMode(self.services.system.native_fullscreen_mode)
                    } else {
                        SystemReply::Fullscreen(observed)
                    },
                ))
            }
            SystemRequest::LocalHour => {
                if self.services.system.clock.is_none() {
                    self.services.system.clock = Some(self.host.begin_civil_time(self.client)?);
                }
                let clock = self.services.system.clock.as_mut().unwrap();
                ensure!(
                    clock.client() == self.client,
                    "Circus foreign civil clock query"
                );
                match clock.poll()? {
                    CivilTimeState::Pending => Ok(None),
                    CivilTimeState::Failed(error) => {
                        self.services.system.clock = None;
                        bail!("Circus host local clock: {error}")
                    }
                    CivilTimeState::Ready(time) => {
                        ensure!(time.client == self.client, "Circus foreign civil time");
                        self.services.system.clock = None;
                        Ok(Some(SystemReply::Hour(time.hour)))
                    }
                }
            }
            SystemRequest::Visible(visible) => {
                self.submit(
                    vec![],
                    vec![json!({"op":"set_window_visible","visible":visible})],
                )?;
                Ok(Some(SystemReply::Done))
            }
            SystemRequest::NativeFullscreenMode(mode) => {
                if let Some(expected) = self.services.system.pending_native_fullscreen_mode {
                    ensure!(
                        expected == mode,
                        "Circus native fullscreen mode completion ownership"
                    );
                } else {
                    //18/8 temporarily overrides renderer+24 even when18/7
                    //has disabled ordinary window transitions. Hold this
                    //override through the asynchronous platform ACK.
                    self.services.system.forced_window_gate=Some(self.services.system.window_transition_gate.0);
                    self.services.system.window_transition_gate.0=1;
                    //43A460 snapshots the actual outer rectangle whenever a
                    //nonzero signed mode differs, including1->2 while full.
                    if mode != 0 && mode != self.services.system.native_fullscreen_mode {
                        self.services.system.window_restore = Some(
                            self.host
                                .window_geometry(self.client)?
                                .context("Circus fullscreen entry rectangle unavailable")?,
                        );
                    }
                    self.services.system.pending_native_fullscreen_mode = Some(mode);
                }
                let result = self.native_system(SystemRequest::Fullscreen(mode != 0));
                if matches!(&result,Err(_)|Ok(Some(SystemReply::Done))) {
                    if let Some(previous)=self.services.system.forced_window_gate.take(){self.services.system.window_transition_gate.0=previous;}
                }
                let result=result?;
                if matches!(result, Some(SystemReply::Done)) {
                    //43A460 retains the original signed mode at renderer+8;
                    //only the platform operation itself is a boolean.
                    self.services.system.native_fullscreen_mode = mode;
                    self.services.system.pending_native_fullscreen_mode = None;
                }
                Ok(result)
            }
            SystemRequest::Fullscreen(enabled) => {
                //43A460 returns unchanged when renderer+24 is zero. Movie52
                //temporarily suppresses transitions without overwriting the
                //retained setting, then restores it on return.
                if self.services.system.pending_fullscreen.is_none()&&(self.services.system.window_transition_gate.0==0||self.services.movie.active()) {
                    return Ok(Some(SystemReply::Done));
                }
                let observed = self
                    .host
                    .window_fullscreen(self.client)?
                    .context("Circus presenting fullscreen observation unavailable")?;
                if let Some(expected) = self.services.system.pending_fullscreen {
                    ensure!(
                        expected == enabled,
                        "Circus fullscreen completion ownership"
                    );
                    if observed != enabled {
                        return Ok(None);
                    }
                    if !enabled {
                        let target = self
                            .services
                            .system
                            .window_restore
                            .context("Circus fullscreen restore rectangle unavailable")?;
                        if !self.apply_window_rectangle(WindowRectOwner::ExitFullscreen, target)? {
                            return Ok(None);
                        }
                    }
                    self.services.system.pending_fullscreen = None;
                    self.services.system.fullscreen = enabled;
                    self.services.system.native_fullscreen_mode = i16::from(enabled);
                    return Ok(Some(SystemReply::Done));
                }
                if observed == enabled {
                    if !enabled && self.services.system.fullscreen {
                        let target = self
                            .services
                            .system
                            .window_restore
                            .context("Circus fullscreen restore rectangle unavailable")?;
                        if !self.apply_window_rectangle(WindowRectOwner::ExitFullscreen, target)? {
                            return Ok(None);
                        }
                    }
                    self.services.system.fullscreen = enabled;
                    self.services.system.native_fullscreen_mode = i16::from(enabled);
                    return Ok(Some(SystemReply::Done));
                }
                if enabled
                    && self
                        .services
                        .system
                        .pending_native_fullscreen_mode
                        .is_none()
                {
                    self.services.system.window_restore = Some(
                        self.host
                            .window_geometry(self.client)?
                            .context("Circus fullscreen entry rectangle unavailable")?,
                    );
                }
                self.submit(
                    vec![],
                    vec![json!({"op":"set_fullscreen","enabled":enabled})],
                )?;
                self.services.system.pending_fullscreen = Some(enabled);
                Ok(None)
            }
            SystemRequest::RestoreWindow | SystemRequest::EnlargeWindow => {
                let owner = if matches!(request, SystemRequest::RestoreWindow) {
                    WindowRectOwner::Restore
                } else {
                    WindowRectOwner::Enlarge
                };
                if let Some((expected, target, _)) = self.services.system.window_rect_pending {
                    ensure!(
                        expected == owner,
                        "Circus window operation completion ownership"
                    );
                    return Ok(self
                        .apply_window_rectangle(owner, target)?
                        .then_some(SystemReply::Done));
                }
                let fullscreen = self
                    .host
                    .window_fullscreen(self.client)?
                    .context("Circus presenting fullscreen observation unavailable")?;
                if !fullscreen {
                    self.services.system.observe_window_geometry(
                        self.host.window_geometry(self.client)?,
                        Some(false),
                    );
                }
                let current = self
                    .services
                    .system
                    .window_current
                    .context("Circus retained windowed rectangle unavailable")?;
                if fullscreen {
                    //43A6E0/43A840 copy308..320 to388..400 even though they
                    //do not resize the fullscreen window. This is a retained
                    //windowed rectangle, not the current monitor-sized frame.
                    self.services.system.window_restore = Some(current);
                    return Ok(Some(SystemReply::Done));
                }
                let default = self
                    .services
                    .system
                    .window_default_outer
                    .context("Circus default window rectangle unavailable")?;
                let desired = if owner == WindowRectOwner::Restore {
                    default
                } else {
                    let padding = self
                        .services
                        .system
                        .window_padding
                        .context("Circus initial non-client padding unavailable")?;
                    [1280 + padding[0], 720 + padding[1]]
                };
                //The original28 comparison is against the default rect,
                //rather than the current rect; preserve that native no-op.
                if (owner == WindowRectOwner::Restore && current.outer_size == default)
                    || (owner == WindowRectOwner::Enlarge && desired == default)
                {
                    return Ok(Some(SystemReply::Done));
                }
                let mut target = current;
                target.outer_size = desired;
                self.services.system.window_restore = Some(target);
                Ok(self
                    .apply_window_rectangle(owner, target)?
                    .then_some(SystemReply::Done))
            }
            SystemRequest::RebuildSelection {
                mode,
                page,
                declarations,
            } => self.rebuild_native_selection(mode, page, &declarations),
            SystemRequest::ResumePanel {mode:6} => {
                //408DF0 mode6 observes the existing native dialog handle;
                //setting its active bit again never creates a new dialog.
                Ok(self.services.profile.is_none().then_some(SystemReply::Done))
            }
            SystemRequest::ResumePanel {mode:1} => {
                //408DF0 deletes the ED player on completion. Reactivation
                //without4595F0 leaves its null player active indefinitely;
                //408E80 then draws nothing over43A240's opaque black clear.
                if self.services.system.panel_started.is_none() {
                    self.submit(vec![json!({"op":"put","layer":{"id":0x38e0_0000u32,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[2_000_000_000,0]}})],vec![json!({"op":"put","widget":{"id":0x38e0_0000u32,"rect":[0,0,1280,720],"modal":true,"interaction_only":true}})])?;
                    self.services.system.panel_started=Some((1,self.host.now_ms()));
                }
                Ok(None)
            }
            SystemRequest::ResumePanel {mode} => bail!("Circus retained panel mode{mode}"),
            SystemRequest::RevealCursor => {
                //442650 initializes input13=0;442B70(1) only changes the
                //cursor and idle clock when that requested value changes.
                if !self.services.system.native_cursor_requested {
                    self.submit(vec![],vec![json!({"op":"set_cursor_visible","visible":true})])?;
                    self.services.system.native_cursor_requested=true;
                    self.services.system.cursor_visible=Some(true);
                    self.services.system.callback_idle_since=self.host.now_ms() as u32;
                }
                Ok(Some(SystemReply::Done))
            }
            SystemRequest::DefaultNames => Ok(self
                .system_metadata()?
                .map(|metadata| SystemReply::DefaultNames(metadata.default_names))),
            SystemRequest::HistoryNames => Ok(self.system_metadata()?.map(|metadata|SystemReply::HistoryNames(metadata.history_names))),
            SystemRequest::Catalogue => Ok(self
                .system_metadata()?
                .map(|m| SystemReply::Catalogue(m.catalogue))),
            SystemRequest::CharacterRow(index) => {
                let Some(metadata) = self.system_metadata()? else {
                    return Ok(None);
                };
                Ok(Some(SystemReply::CharacterRow(
                    *metadata
                        .characters
                        .get(index)
                        .context("Circus native character metadata index")?,
                )))
            }
            SystemRequest::RestoreScrollingPair => {
                self.restore_scrolling_pair()?;
                Ok(Some(SystemReply::Done))
            }
            SystemRequest::Account {operation,system,face_names} => self.native_account(operation,system,face_names),
            SystemRequest::CalendarEdge { year, month, day } => {
                //466EDB initializes the process timezone exactly once. Keep a
                //published recurring-rule snapshot, not today's offset or a
                //historical IANA midnight guess.40E760 sets tm_isdst=-1;
                //45BE17 applies standard bias, probes DST, then reapplies bias.
                if self.services.system.timezone_profile.is_none() {
                    if self.services.system.timezone_query.is_none() {
                        self.services.system.timezone_query =
                            Some(self.host.begin_civil_timezone(self.client)?);
                    }
                    let query = self.services.system.timezone_query.as_mut().unwrap();
                    ensure!(
                        query.client() == self.client,
                        "Circus timezone query ownership"
                    );
                    match query.poll()? {
                        CivilTimeZoneState::Pending => return Ok(None),
                        CivilTimeZoneState::Failed(error) => {
                            bail!("Circus native timezone policy: {error}")
                        }
                        CivilTimeZoneState::Ready(profile) => {
                            self.services.system.timezone_profile = Some(profile);
                            self.services.system.timezone_query = None;
                        }
                    }
                }
                let profile = native_calendar_profile(
                    self.services.system.timezone_profile.as_ref().unwrap(),
                    year,
                )?;
                //Pinned45B92D accepts0..0x793406FFF seconds inclusive, including
                //its final eight-hour margin. Failure remains native weekday6.
                let weekday = profile
                    .resolve_crt_midnight(self.client, year, month, day, 0..=0x793406fff)
                    .and_then(|time| time.weekday());
                Ok(Some(SystemReply::Weekday(weekday)))
            }
            SystemRequest::License => self.native_license(),
            SystemRequest::Panel { mode, .. } if mode != 1 && mode != 6 => {
                // Native408E80 has no draw branch for modes3/4 (nor the
                // constructor's unset mode).408DF0 returns on native key10,
                // verified in4B4FB8 as VK_SPACE;442EC0 maps release to state4.
                let now = self.host.now_ms();
                if let Some((owner, started)) = self.services.system.panel_started {
                    ensure!(owner == mode, "Circus blank-panel completion ownership");
                    let events: Vec<crysirror_scene::ui::UiEvent> = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::UiSubtreeEvents(0x38e0_0000))?,
                    )?;
                    if now != started && events.iter().any(|event| event.kind == "blank_advance") {
                        self.submit(
                            vec![json!({"op":"remove","id":0x38e0_0000u32})],
                            vec![json!({"op":"remove","id":0x38e0_0000u32})],
                        )?;
                        self.services.system.panel_started = None;
                        return Ok(Some(SystemReply::Done));
                    }
                } else {
                    //43A240 clears opaque black before408E80's empty draw.
                    // A shared retained black layer has exactly that output;
                    // its modal UI scope prevents the hidden game menu from
                    // accepting pointer input while native process is paused.
                    self.submit(vec![json!({"op":"put","layer":{"id":0x38e0_0000u32,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[2_000_000_000,0]}})],vec![json!({"op":"put","widget":{"id":0x38e0_0000u32,"rect":[0,0,1280,720],"modal":true,"interaction_only":true,"key_events":[{"buttons":crysirror_core::service::buttons::SPACE,"edge":"release","event":"blank_advance"}]}})])?;
                    self.services.system.panel_started = Some((mode, now));
                }
                Ok(None)
            }
            SystemRequest::Panel {
                mode: 1,
                parameter,
                duration_ms,
                skip_unlocked,
                ..
            } => self.native_ending(parameter, duration_ms, skip_unlocked),
            SystemRequest::Panel { mode, .. } => {
                bail!("Circus built-in panel mode{mode} needs its original setup")
            }
            SystemRequest::Thumbnails { release, page } => self.native_thumbnails(release, page),
            SystemRequest::CharacterPreview {
                name,
                position,
                auxiliary,
                scale,
            } => {
                ensure!(self.services.image_names.get(&4).is_some_and(|loaded|loaded.eq_ignore_ascii_case(&name)),"Circus character preview prepared slot4 owner");
                if self.native_sprite_resources(&crate::machine::sprite_ops::SpriteResourceRequest{bindings:vec![crate::machine::sprite_ops::SpriteResourceBinding{slot:4,object:149,crop:None}]})?.is_none(){return Ok(None)}
                let logical=*self.services.image_extents.get(&4).context("Circus character preview source extent")?;
                if !self.native_sprite(&crate::machine::sprite_ops::SpriteRequest{commands:vec![crate::machine::sprite_ops::SpriteCommand{
                    seed:crate::native_live::NativeLiveState::sprite_reset_seed(149)?,
                    operations:vec![crysirror_scene::layer::queued::SequenceOperation::Set{
                        channels:vec![0,1,3,4,12,13,14,15],values:vec![position[0] as f32,position[1] as f32,f32::from(scale),f32::from(scale),auxiliary[0] as f32,auxiliary[1] as f32,logical[0] as f32,logical[1] as f32],retained:false,
                    }],
                }]})? {return Ok(None)}
                Ok(Some(SystemReply::Done))
            }
            SystemRequest::MusicSeek(percent) => self.native_system(SystemRequest::SourceSeek {
                channel: 0,
                percent,
            }),
            SystemRequest::SourcePause { channel, paused } => {
                self.host.submit_audio(
                    self.client,
                    AudioRequest::Pause {
                        channel,
                        paused,
                        fade_ms: 0.,
                    },
                )?;
                Ok(Some(SystemReply::Done))
            }
            SystemRequest::SourceSeek { channel, percent } => {
                if percent == 0 {
                    return Ok(Some(SystemReply::Done));
                }
                let ticket =
                    if let Some((owner, expected, ticket)) = self.services.system.music_seek {
                        ensure!(
                            owner == channel && expected == percent,
                            "Circus music seek completion ownership"
                        );
                        ticket
                    } else {
                        let Some(info) = self.host.audio_info(self.client, channel)? else {
                            return Ok(Some(SystemReply::Done));
                        };
                        let length = info
                            .source_frames
                            .context("Circus music seek needs native decoded frame length")?;
                        ensure!(length > 0, "Circus music seek empty source");
                        let elapsed = info.position_frames.saturating_mul(100) / length;
                        let target_percent = u64::try_from(percent.clamp(0, 100))?.min(elapsed);
                        ensure!(info.source_rate > 0, "Circus source seek sample rate");
                        //43AF90 divides bytes by WAVEFORMATEX.nAvgBytesPerSec,
                        // truncating the full duration to seconds first.
                        //43B820 then truncates percent*seconds/100 before
                        // multiplying back to a byte position. Preserve both
                        // truncations in decoded-source frame units.
                        let seconds = length / u64::from(info.source_rate);
                        let target_seconds =
                            u64::try_from(u128::from(seconds) * u128::from(target_percent) / 100)?;
                        let frame = target_seconds
                            .checked_mul(u64::from(info.source_rate))
                            .context("Circus source seek frame overflow")?;
                        let ticket = self.host.begin_audio_seek(self.client, channel, frame)?;
                        self.services.system.music_seek = Some((channel, percent, ticket));
                        ticket
                    };
                let preparation = self.host.audio_seek_preparation(self.client, ticket)?;
                ensure!(
                    preparation.channel == channel,
                    "Circus foreign music seek channel"
                );
                match preparation.phase {
                    AudioPreparePhase::Pending => Ok(None),
                    AudioPreparePhase::Ready => {
                        self.host.release_audio_seek(self.client, ticket)?;
                        self.services.system.music_seek = None;
                        self.host.submit_audio(
                            self.client,
                            AudioRequest::Resume {
                                channel,
                                delay_ms: 0.,
                                fade_ms: 0.,
                            },
                        )?;
                        Ok(Some(SystemReply::Done))
                    }
                    AudioPreparePhase::Failed => {
                        self.host.release_audio_seek(self.client, ticket)?;
                        self.services.system.music_seek = None;
                        bail!(
                            "Circus music seek failed: {}",
                            preparation.error.unwrap_or_default()
                        )
                    }
                }
            }
        }
    }
}
