//! Authored native transport contracts, separate from original-game behavior.
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{FileSystem, HostServices, InputFrame, MediaInfo, MediaPhase, MediaRequest},
    state::{ClientId, Command},
};
use crysirror_system_nnn::{
    config::Names,
    expression::{VariableProfile, Variables},
    native_bootstrap::NativeConfiguration,
    native_movie::{NativeMoviePlayer, NativeMovieSpec},
    native_resources::ResourceNames,
    native_services_bootstrap::NativePreferences,
    native_shell::{NativeMode, NativeStartupProfile},
    XorProfile,
};
fn names(text: &str) -> Names {
    Names::parse(text.as_bytes(), false).unwrap()
}
fn configuration() -> NativeConfiguration {
    NativeConfiguration {
        main_control: None,
        resources: ResourceNames::new("game").unwrap(),
        setup: names(""),
        variable_names: names("selector"),
        project: names("back"),
        commands: names(""),
        functions: names(""),
        initial: None,
        module_sha256: "authored".into(),
        profile: VariableProfile::Legacy1000,
        scripts: XorProfile::default(),
    }
}
fn profile(config: &NativeConfiguration, movie: &str) -> NativeStartupProfile {
    NativeStartupProfile::from_tables(
        config,
        &names("nextMode logo"),
        &names("logoNumber 1 nextMode opening"),
        &names("backScriptFlag 1 backScriptName back nextMode printmovie"),
        names("backScriptFlag 1 backScriptName back"),
        names(movie),
    )
    .unwrap()
}
#[test]
fn only_the_existing_zero_variable_in_minus_one_profile_skips_movie() {
    let config = configuration();
    let vars = Variables::fresh(VariableProfile::Legacy1000);
    let preferences = NativePreferences::original_defaults();
    let mut profile = profile(
        &config,
        "selectByVarFlag -1 selectMovieVarName selector movieFileName default.mpg",
    );
    assert!(
        NativeMovieSpec::from_profile(&profile, &config, &vars, &preferences)
            .unwrap()
            .asset
            .is_none()
    );
    profile.movie =
        names("selectByVarFlag -1 selectMovieVarName missing movieFileName default.mpg");
    assert_eq!(
        NativeMovieSpec::from_profile(&profile, &config, &vars, &preferences)
            .unwrap()
            .asset
            .as_deref(),
        Some("game/movie/default.mpg")
    );
    profile.movie =
        names("selectByVarFlag 1 selectMovieVarName selector movieFileName default.mpg");
    assert!(
        NativeMovieSpec::from_profile(&profile, &config, &vars, &preferences)
            .unwrap()
            .asset
            .is_some()
    );
}
#[test]
fn positive_selection_uses_original_pair_table_or_default_and_native_volume() {
    let config = configuration();
    let mut vars = Variables::fresh(VariableProfile::Legacy1000);
    vars.write_stored(0, 2);
    let mut preferences = NativePreferences::original_defaults();
    let mut profile = profile(
        &config,
        "selectByVarFlag -1 selectMovieVarName selector movieFileName default.mpg",
    );
    profile.movie_names = Some(names("first.mpg 100 second.mpg 200"));
    let selected = NativeMovieSpec::from_profile(&profile, &config, &vars, &preferences).unwrap();
    assert_eq!(selected.asset.as_deref(), Some("game/movie/second.mpg"));
    assert_eq!(selected.last_wait, 5);
    vars.write_stored(0, 3);
    assert_eq!(
        NativeMovieSpec::from_profile(&profile, &config, &vars, &preferences)
            .unwrap()
            .asset
            .as_deref(),
        Some("game/movie/default.mpg")
    );
    preferences.movie_enabled = false;
    let muted = NativeMovieSpec::from_profile(&profile, &config, &vars, &preferences).unwrap();
    assert!(
        (muted.gain - 0.00001).abs() < 1e-8,
        "native attenuation remains a nonzero DirectShow endpoint"
    );
}
struct Host {
    now: u64,
    phase: MediaPhase,
    frames: u64,
    requests: Vec<MediaRequest>,
    batches: Vec<serde_json::Value>,
}
impl FileSystem for Host {
    fn read(&mut self, _: ClientId, _: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
        Err(crysirror_core::Error::NotFound)
    }
    fn write_atomic(&mut self, _: ClientId, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
        unreachable!()
    }
}
impl HostServices for Host {
    fn now_ms(&self) -> u64 {
        self.now
    }
    fn submit(&mut self, _: ClientId, _: &[Command]) -> crysirror_core::Result<()> {
        unreachable!()
    }
    fn submit_engine(
        &mut self,
        client: ClientId,
        version: u32,
        bytes: &[u8],
    ) -> crysirror_core::Result<()> {
        assert_eq!((client.0, version), (7, 1));
        self.batches.push(serde_json::from_slice(bytes).unwrap());
        Ok(())
    }
    fn media_request(
        &mut self,
        client: ClientId,
        request: MediaRequest,
    ) -> crysirror_core::Result<VfsPath> {
        assert_eq!(client.0, 7);
        self.requests.push(request);
        VfsPath::new(Mount::Asset, "__media/7/19")
    }
    fn media_info(&self, client: ClientId, id: u32) -> crysirror_core::Result<MediaInfo> {
        assert_eq!((client.0, id), (7, 19));
        Ok(MediaInfo {
            phase: self.phase,
            position_ms: 0,
            published_frames: self.frames,
            paused: false,
            frame_ms: 40,
            size: if self.frames > 0 {
                Some([320, 240])
            } else {
                None
            },
            duration_ms: None,
        })
    }
}
fn new_host() -> Host {
    Host {
        now: 100,
        phase: MediaPhase::Loading,
        frames: 0,
        requests: vec![],
        batches: vec![],
    }
}
fn new_spec() -> NativeMovieSpec {
    NativeMovieSpec {
        asset: Some("game/movie/actual.mpg".into()),
        next: NativeMode::Title,
        no_skip: false,
        last_wait: 5,
        exit_fill: Some([4, 5, 6]),
        gain: 0.3,
    }
}
#[test]
fn movie_uses_shared_transport_then_published_surface_and_original_post_stop_wait() {
    let mut host = new_host();
    let mut player =
        NativeMoviePlayer::new(ClientId(7), 19, 20, [800, 600], 20, new_spec()).unwrap();
    assert_eq!(player.poll(&mut host, InputFrame::default()).unwrap(), None);
    assert!(player.dependency_pending());
    assert_eq!(host.requests.len(), 1);
    assert!(matches!(
        &host.requests[0],
        MediaRequest::Play {
            audio: true,
            looping: false,
            ..
        }
    ));
    assert_eq!(host.batches.len(), 0);
    host.phase = MediaPhase::Playing;
    host.frames = 1;
    player.poll(&mut host, InputFrame::default()).unwrap();
    assert_eq!(
        host.batches[0]["layers"][0]["layer"]["paint"]["asset"],
        "__media/7/19"
    );
    player
        .poll(
            &mut host,
            InputFrame {
                buttons_pressed: crysirror_core::service::buttons::MOUSE_LEFT,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(
        host.requests.last(),
        Some(MediaRequest::Remove { id: 19 })
    ));
    assert!(!player.dependency_pending());
    host.now = 199;
    assert_eq!(player.poll(&mut host, Default::default()).unwrap(), None);
    host.now = 200;
    assert_eq!(
        player.poll(&mut host, Default::default()).unwrap(),
        Some(NativeMode::Title)
    );
    assert_eq!(
        host.requests.len(),
        2,
        "completed actor never restarts decoder"
    );
}
#[test]
fn no_skip_ignores_pointer_eof_completes_and_failure_is_visible() {
    let mut host = new_host();
    let mut spec = new_spec();
    spec.no_skip = true;
    spec.last_wait = 0;
    let mut player = NativeMoviePlayer::new(ClientId(7), 19, 20, [800, 600], 20, spec).unwrap();
    player
        .poll(
            &mut host,
            InputFrame {
                buttons_pressed: crysirror_core::service::buttons::MOUSE_LEFT
                    | crysirror_core::service::buttons::MOUSE_RIGHT,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(host.requests.len(), 1);
    host.phase = MediaPhase::Ended;
    assert_eq!(player.poll(&mut host, Default::default()).unwrap(), None);
    host.now = 119;
    assert_eq!(player.poll(&mut host, Default::default()).unwrap(), None);
    host.now = 120;
    assert_eq!(
        player.poll(&mut host, Default::default()).unwrap(),
        Some(NativeMode::Title)
    );
    let mut failed = new_host();
    failed.phase = MediaPhase::Failed;
    let mut player =
        NativeMoviePlayer::new(ClientId(7), 19, 20, [800, 600], 20, new_spec()).unwrap();
    assert!(player
        .poll(&mut failed, Default::default())
        .unwrap_err()
        .to_string()
        .contains("transport failed"));
}

#[test]
fn selector_reads_the_calculated_outer_mode_instead_of_stored_zero() {
    let mut config = configuration();
    config.variable_names = names("mode");
    let mut variables = Variables::fresh(VariableProfile::Legacy1000);
    variables.mark_calculated(0, b"mode".to_vec());
    assert!(variables
        .bind_calculated_value(b"mode", NativeMode::Movie as i32)
        .unwrap());
    assert_eq!(variables.read_stored(0), 0);
    let profile = profile(
        &config,
        "selectByVarFlag -1 selectMovieVarName mode movieFileName actual.mpg",
    );
    let spec = NativeMovieSpec::from_profile(
        &profile,
        &config,
        &variables,
        &NativePreferences::original_defaults(),
    )
    .unwrap();
    assert_eq!(spec.asset.as_deref(), Some("game/movie/actual.mpg"));
}
