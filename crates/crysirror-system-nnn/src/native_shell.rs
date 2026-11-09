// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original outer modes. Configuration, counters and transitions remain native;
//! resource reads, image transforms/composition, media and input are services.
use crate::{config::Names, native_bootstrap::NativeConfiguration, native_resources::ImageLookup};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_tasks::Cancellation;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeMode {
    Nothing = 1,
    Logo = 2,
    Opening = 3,
    Title = 4,
    Movie = 27,
}
impl NativeMode {
    pub(crate) fn name(self) -> &'static [u8] {
        match self {
            Self::Nothing => b"nothing",
            Self::Logo => b"logo",
            Self::Opening => b"opening",
            Self::Title => b"title",
            Self::Movie => b"printmovie",
        }
    }
    pub fn named(name: &[u8]) -> Result<Self> {
        let number = crate::native_mode::number(name)?;
        match number {
            1 => Ok(Self::Nothing),
            2 => Ok(Self::Logo),
            3 => Ok(Self::Opening),
            4 => Ok(Self::Title),
            27 => Ok(Self::Movie),
            _ => anyhow::bail!(
                "NNN unimplemented outer native mode {} ({number})",
                String::from_utf8_lossy(name)
            ),
        }
    }
}
#[derive(Clone, Debug)]
pub struct NativeLogoSpec {
    pub source: ImageLookup,
    pub frames: [u32; 3],
    pub colors: [[u8; 3]; 2],
    pub skip_after_launch: u32,
}
#[derive(Clone, Debug)]
pub struct NativeStartupProfile {
    pub nothing_color: [u8; 3],
    pub nothing_next: NativeMode,
    pub codec_check: bool,
    pub codec_limit: i32,
    pub startup_jump_script: bool,
    pub logos: Vec<NativeLogoSpec>,
    pub logo_corner_colors: Vec<[bool; 2]>,
    pub logo_next: NativeMode,
    pub(crate) logo_return: crate::native_general::ModeReturn,
    pub opening_next: NativeMode,
    pub opening_script: Vec<u8>,
    pub opening_loop: bool,
    pub title_script: Vec<u8>,
    pub title_loop: bool,
    pub title: Names,
    pub movie: Names,
    pub movie_names: Option<Names>,
    pub system_sounds: Option<Names>,
    pub system_sound_disabled: bool,
    pub system_sound_multi: bool,
    pub movie_asset: String,
    pub codec_asset: String,
}
impl NativeStartupProfile {
    /// Invoke on a scoped bootstrap worker. No optional raw OS file fallback.
    pub fn read(
        config: &NativeConfiguration,
        storage: &dyn ClientStorage,
        cancel: &Cancellation,
    ) -> Result<Self> {
        let read = |name: &str| -> Result<Names> {
            cancel.check()?;
            for (folder, ext, cipher) in [("init2", "fxf", true), ("init", "xtx", false)] {
                let path = config.resources.path(&format!("{folder}/{name}.{ext}"))?;
                let reader = match storage.open_asset(&VfsPath::new(Mount::Asset, &path)?) {
                    Ok(r) => r,
                    Err(crysirror_core::Error::NotFound) => continue,
                    Err(e) => return Err(e.into()),
                };
                let bytes = crysirror_resource::read_all(reader.as_ref(), 1 << 20)?;
                return Names::parse(&bytes, cipher);
            }
            anyhow::bail!("NNN original {name} startup configuration absent")
        };
        let mut profile = Self::from_tables(
            config,
            &read("nothing")?,
            &read("logo")?,
            &read("opening")?,
            read("title")?,
            read("printmovie")?,
        )?;
        let path = config.resources.path("spt/movielist.xtx")?;
        profile.movie_names = match storage.open_asset(&VfsPath::new(Mount::Asset, &path)?) {
            Ok(reader) => Some(Names::parse(
                &crysirror_resource::read_all(reader.as_ref(), 1 << 20)?,
                false,
            )?),
            Err(crysirror_core::Error::NotFound) => None,
            Err(e) => return Err(e.into()),
        };
        if !profile.system_sound_disabled {
            let path = config.resources.path("nya/systemse.xtx")?;
            profile.system_sounds = match storage.open_asset(&VfsPath::new(Mount::Asset, &path)?) {
                Ok(reader) => Some(Names::parse(
                    &crysirror_resource::read_all(reader.as_ref(), 1 << 20)?,
                    false,
                )?),
                Err(crysirror_core::Error::NotFound) => None,
                Err(error) => return Err(error.into()),
            };
        }
        cancel.check()?;
        Ok(profile)
    }
    pub fn from_tables(
        config: &NativeConfiguration,
        nothing: &Names,
        logo: &Names,
        opening: &Names,
        title: Names,
        movie: Names,
    ) -> Result<Self> {
        let int = |n: &Names, key: &str, default: i32| -> Result<i32> {
            Ok(n.integer(key.as_bytes())?.unwrap_or(default))
        };
        let mode = |n: &Names, key: &str, default: NativeMode| -> Result<NativeMode> {
            n.parameter(key.as_bytes())
                .map(NativeMode::named)
                .transpose()
                .map(|m| m.unwrap_or(default))
        };
        let color = |n: &Names, prefix: &str, default: [u8; 3]| -> Result<[u8; 3]> {
            let mut c = default;
            for (i, ch) in ["R", "G", "B"].iter().enumerate() {
                c[i] = int(n, &format!("{prefix}{ch}"), i32::from(c[i]))? as u8;
            }
            Ok(c)
        };
        let count = int(logo, "logoNumber", 2)?;
        crate::native_general::ModeReturn::validate_entry(logo)?;
        let logo_return = crate::native_general::ModeReturn::read(
            logo,
            Some(mode(logo, "nextMode", NativeMode::Title)?),
        )?;
        ensure!((1..=64).contains(&count), "NNN original logo count bounds");
        ensure!(
            int(logo, "soundNumber", 0)? == 0,
            "NNN timed logo system sounds require native trigger policy"
        );
        let use_files = int(logo, "useCustomFileFlag", 0)? != 0;
        let use_times = int(logo, "useCustomTimeFlag", 0)? != 0;
        let use_colors = int(logo, "useCustomColorFlag", 0)? != 0;
        let use_skip = int(logo, "useCustomSkipFlag", 0)? != 0;
        let use_custom_bg = int(logo, "useCustomBGColorFlag", 0)? != 0;
        let default_bg = int(logo, "defaultUseBGColor", 0)?;
        let default_times = [
            int(logo, "defaultFadeInTime", 10)?,
            int(logo, "defaultKeepTime", 20)?,
            int(logo, "defaultFadeOutTime", 10)?,
        ];
        let default_colors = [
            color(logo, "defaultFadeInColor", [255; 3])?,
            color(logo, "defaultFadeOutColor", [255; 3])?,
        ];
        let levels = u32::try_from(int(&config.setup, "dwqPackLevel", 0)?)?;
        let mut logos = Vec::new();
        let mut logo_corner_colors = Vec::new();
        for i in 1..=count {
            let mut times = default_times;
            if use_times {
                for (n, key) in ["fadeInTime", "keepTime", "fadeOutTime"].iter().enumerate() {
                    times[n] = int(logo, &format!("{key}{i}"), times[n])?;
                }
            }
            ensure!(
                times.iter().all(|n| (0..=864000).contains(n)),
                "NNN logo frame limits"
            );
            let mut colors = default_colors;
            if use_colors {
                for (n, prefix) in ["fadeInColor", "fadeOutColor"].iter().enumerate() {
                    let mut c = colors[n];
                    for (ch, letter) in ["R", "G", "B"].iter().enumerate() {
                        c[ch] = int(logo, &format!("{prefix}{letter}{i}"), i32::from(c[ch]))? as u8;
                    }
                    colors[n] = c;
                }
            }
            let mut corner_colors = [false; 2];
            for (index, phase) in ["fadeInBGColorFlag", "fadeOutBGColorFlag"]
                .into_iter()
                .enumerate()
            {
                let bg = if use_custom_bg {
                    int(logo, &format!("{phase}{i}"), default_bg)?
                } else {
                    default_bg
                };
                corner_colors[index] = bg != 0;
            }
            logo_corner_colors.push(corner_colors);
            let path = if use_files {
                let file = logo
                    .parameter(format!("filename{i}").as_bytes())
                    .context("NNN custom logo filename absent")?;
                if file.get(2) == Some(&b'\\') || file.get(3) == Some(&b'\\') {
                    file.to_vec()
                } else {
                    [b"sys\\".as_slice(), file].concat()
                }
            } else {
                format!("sys\\logo{i}").into_bytes()
            };
            let skip = if use_skip {
                int(logo, &format!("skipOK{i}"), 1)?
            } else {
                1
            };
            logos.push(NativeLogoSpec {
                source: config.resources.picture(&path, levels)?,
                frames: times.map(|n| n as u32),
                colors,
                skip_after_launch: u32::try_from(skip)?,
            });
        }
        let back = |n: &Names, default_loop: i32| -> Result<(Vec<u8>, bool)> {
            ensure!(
                int(n, "backScriptFlag", 0)? != 0,
                "NNN startup mode requires its original background script profile"
            );
            let name = n
                .parameter(b"backScriptName")
                .context("NNN original background script absent")?;
            config.resources.script(name)?;
            Ok((
                name.to_vec(),
                int(n, "backScriptLoopFlag", default_loop)? != 0,
            ))
        };
        let (opening_script, opening_loop) = back(opening, 0)?;
        let (title_script, title_loop) = back(&title, 1)?;
        let movie_name = movie.parameter(b"movieFileName").unwrap_or(b"default.wmv");
        let codec_name = movie.parameter(b"codecCheckFileName").unwrap_or(movie_name);
        let movie_asset = config
            .resources
            .encoded_path(&[b"movie\\".as_slice(), movie_name].concat())?;
        let codec_asset = config
            .resources
            .encoded_path(&[b"movie\\".as_slice(), codec_name].concat())?;
        Ok(Self {
            movie_asset,
            codec_asset,
            nothing_color: color(nothing, "fillColor", [255; 3])?,
            nothing_next: mode(nothing, "nextMode", NativeMode::Logo)?,
            codec_check: int(nothing, "codecCheckFlag", 0)? != 0,
            codec_limit: int(nothing, "codecLimit", 3)?,
            startup_jump_script: int(nothing, "startupJumpScriptFlag", 0)? != 0,
            logos,
            logo_corner_colors,
            logo_next: mode(logo, "nextMode", NativeMode::Title)?,
            logo_return,
            opening_next: mode(opening, "nextMode", NativeMode::Title)?,
            opening_script,
            opening_loop,
            title_script,
            title_loop,
            title,
            movie,
            movie_names: None,
            system_sounds: None,
            system_sound_disabled: config.setup.integer(b"noSystemSeFlag")?.unwrap_or(0) != 0,
            system_sound_multi: false, /* reviewed legacy0x530350 always stops the previous channel; the later config key does not exist in this ABI */
        })
    }
}

use crysirror_core::{
    image_processing::{
        ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState,
        ImageResource,
    },
    service::{HostServices, InputFrame},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LogoFrame {
    index: usize,
    phase: usize,
    percent: u16,
}
/// A native outer Logo mode with immutable images, public worker operations
/// and ordinary Layers. No pixels, decoder, window or device live in this type.
pub struct NativeLogoPlayer {
    client: ClientId,
    layer: u32,
    canvas: [u32; 2],
    frame_ms: u64,
    launch_count: u32,
    specs: Vec<NativeLogoSpec>,
    corner_colors: Vec<[bool; 2]>,
    resources: crate::resource_bridge::NativeResources,
    pool: TaskPool,
    source_task: Option<Task<crate::resource_bridge::NativeImageSource>>,
    source_job: Option<ImageJob>,
    source: Option<Arc<dyn ImageResource>>,
    frame_job: Option<(LogoFrame, ImageJob)>,
    presented: Option<LogoFrame>,
    published: Option<String>,
    final_frame: Option<Arc<dyn ImageResource>>,
    finish_job: Option<ImageJob>,
    index: usize,
    phase: usize,
    count: u32,
    clock: Option<u64>,
    remainder: u64,
    first_ready: bool,
    finished: bool,
    pending_buttons: u64,
    pub api_calls: u64,
}
impl NativeLogoPlayer {
    pub fn new(
        client: ClientId,
        layer: u32,
        canvas: [u32; 2],
        frame_ms: u64,
        launch_count: u32,
        specs: Vec<NativeLogoSpec>,
        resources: crate::resource_bridge::NativeResources,
    ) -> Result<Self> {
        ensure!(
            client.0 != 0
                && layer != 0
                && canvas.iter().all(|n| (1..=8192).contains(n))
                && (1..=10000).contains(&frame_ms)
                && !specs.is_empty()
                && specs.len() <= 64,
            "NNN native logo scope/bounds"
        );
        Ok(Self {
            client,
            layer,
            canvas,
            frame_ms,
            launch_count,
            corner_colors: vec![[false; 2]; specs.len()],
            specs,
            resources,
            pool: TaskPool::new("nnn-native-logo-source", 1, 1)?,
            source_task: None,
            source_job: None,
            source: None,
            frame_job: None,
            presented: None,
            published: None,
            final_frame: None,
            finish_job: None,
            index: 0,
            phase: 0,
            count: 0,
            clock: None,
            remainder: 0,
            first_ready: false,
            finished: false,
            pending_buttons: 0,
            api_calls: 0,
        })
    }
    pub fn preparation_pending(&self) -> bool {
        !self.first_ready && !self.finished
    }
    fn configure_corner_colors(&mut self, flags: Vec<[bool; 2]>) -> Result<()> {
        ensure!(
            flags.len() == self.specs.len()
                && self.source_task.is_none()
                && self.source_job.is_none(),
            "NNN logo corner-color policy shape/lifecycle"
        );
        self.corner_colors = flags;
        Ok(())
    }
    pub fn dependency_pending(&self) -> bool {
        self.source_task.is_some()
            || self.source_job.is_some()
            || self.frame_job.is_some()
            || self.finish_job.is_some()
    }
    pub fn native_location(&self) -> (usize, usize, u32) {
        (self.index, self.phase, self.count)
    }
    fn next(&mut self) {
        self.index += 1;
        self.phase = 0;
        self.count = 0;
        self.source = None;
        self.presented = None;
        self.clock = None;
        self.remainder = 0;
    }
    fn elapse(&mut self, now: u64, input: InputFrame) -> Result<()> {
        if self.source.is_none() {
            return Ok(());
        }
        let spec = &self.specs[self.index];
        if spec.skip_after_launch != 0
            && spec.skip_after_launch <= self.launch_count.saturating_add(1)
            && input.buttons_pressed
                & (crysirror_core::service::buttons::MOUSE_LEFT
                    | crysirror_core::service::buttons::MOUSE_RIGHT)
                != 0
        {
            self.next();
            return Ok(());
        }
        let previous = self.clock.replace(now).unwrap_or(now);
        ensure!(now >= previous, "NNN logo clock moved backwards");
        self.remainder = self
            .remainder
            .checked_add(now - previous)
            .context("NNN logo clock overflow")?;
        // Keep remaining debt, rather than unboundedly iterating a long gap on
        // the owner. There is no I/O wait or sleep in this native clock step.
        let steps = (self.remainder / self.frame_ms).min(4096);
        self.remainder -= steps * self.frame_ms;
        for _ in 0..steps {
            self.count += 1;
            if self.count >= self.specs[self.index].frames[self.phase] {
                self.count = 0;
                self.phase += 1;
                if self.phase == 3 {
                    self.next();
                    break;
                }
            }
        }
        Ok(())
    }
    fn target(&self) -> LogoFrame {
        let spec = &self.specs[self.index];
        let percent = match self.phase {
            0 => u64::from(self.count) * 100 / u64::from(spec.frames[0].max(1)),
            1 => 100,
            _ => {
                u64::from(spec.frames[2].max(1).saturating_sub(self.count)) * 100
                    / u64::from(spec.frames[2].max(1))
            }
        }
        .min(100) as u16;
        LogoFrame {
            index: self.index,
            phase: self.phase,
            percent,
        }
    }
    /// Returns true only after the actual native page sequence ends. A completed
    /// read/process receipt is owner-published before its predecessor is released.
    pub fn poll(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<bool> {
        if self.finished {
            return Ok(true);
        }
        self.pending_buttons |= input.buttons_pressed
            & (crysirror_core::service::buttons::MOUSE_LEFT
                | crysirror_core::service::buttons::MOUSE_RIGHT);
        if let Some((key, job)) = &mut self.frame_job {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN logo common composition: {error}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(
                        image.client() == self.client && image.size() == self.canvas,
                        "NNN logo frame resource scope"
                    );
                    let published =
                        host.publish_image_resource(self.client, self.layer, image.clone())?;
                    self.api_calls += 1;
                    let path = published.path.relative().to_owned();
                    let [w, h] = self.canvas;
                    let batch = serde_json::to_vec(
                        &serde_json::json!({"layers":[{"op":"put","layer":{"id":self.layer,"rect":[0,0,w,h],"order":[2000,0],"paint":{"kind":"image","asset":path}}}]}),
                    )?;
                    if let Err(e) = host.submit_engine(self.client, 1, &batch) {
                        let _ = host.release_image_resource(self.client, &path);
                        return Err(e.into());
                    }
                    self.api_calls += 1;
                    if let Some(old) = self.published.replace(path) {
                        host.release_image_resource(self.client, &old)?;
                    }
                    self.presented = Some(*key);
                    self.final_frame = Some(image);
                    self.frame_job = None;
                    self.first_ready = true;
                }
            }
        }
        if let Some(task) = &mut self.source_task {
            let Some(source) = task.try_take()? else {
                return Ok(false);
            };
            self.source_job = Some(
                host.image_metadata(self.client)?.begin_process(
                    source.input,
                    serde_json::to_vec(&[crysirror_image::effects::ImageEffect::OverrideAlpha {
                        alpha: 255,
                    }])?
                    .into(),
                )?,
            );
            self.api_calls += 1;
            self.source_task = None;
        }
        if let Some(job) = &mut self.source_job {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN logo shared image decode: {error}")
                }
                ImageProcessState::Ready(image) => {
                    ensure!(image.client() == self.client, "NNN logo image client");
                    if self.corner_colors[self.index].iter().any(|flag| *flag) {
                        let [r, g, b, _] = image
                            .pixel_at([0, 0])?
                            .context("NNN decoded logo first pixel absent")?;
                        for phase in 0..2 {
                            if self.corner_colors[self.index][phase] {
                                self.specs[self.index].colors[phase] = [r, g, b];
                            }
                        }
                    }
                    self.source = Some(image);
                    self.source_job = None;
                    self.clock = Some(host.now_ms());
                }
            }
        }
        if self.source.is_some() {
            self.elapse(
                host.now_ms(),
                InputFrame {
                    buttons_pressed: self.pending_buttons,
                    ..input
                },
            )?;
            self.pending_buttons = 0;
        }
        if self.index == self.specs.len() {
            // Logo.Print clamps its completed page index to the last page,
            // phase2/count=fadeOutTime: the final frame is exactly its fill
            // colour, including skip. Capture that engine-owned generation,
            // rather than retaining the previous partially faded frame.
            if self.finish_job.is_none() {
                let color = self.specs.last().unwrap().colors[1];
                self.finish_job = Some(host.image_metadata(self.client)?.begin_process(
                    ImageInput::Solid {
                        size: self.canvas,
                        color: [color[0], color[1], color[2], 255],
                    },
                    b"[]".as_slice().into(),
                )?);
                self.api_calls += 1;
            }
            match self.finish_job.as_mut().unwrap().poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN Logo terminal shared fill: {error}")
                }
                ImageProcessState::Ready(image) => {
                    self.final_frame = Some(image);
                    self.finish_job = None;
                }
            }
            if self.published.is_some() {
                host.submit_engine(
                    self.client,
                    1,
                    &serde_json::to_vec(
                        &serde_json::json!({"layers":[{"op":"remove","id":self.layer}]}),
                    )?,
                )?;
                self.api_calls += 1;
            }
            if let Some(path) = self.published.take() {
                host.release_image_resource(self.client, &path)?;
            }
            self.finished = true;
            return Ok(true);
        }
        let Some(source) = &self.source else {
            if self.source_task.is_none() {
                let spec = self.specs[self.index].source.clone();
                let resources = self.resources.clone();
                self.source_task = Some(
                    self.pool
                        .spawn(move |cancel| resources.image_source(&spec, &cancel))?,
                );
            }
            return Ok(false);
        };
        let key = self.target();
        if self.presented == Some(key) {
            return Ok(false);
        }
        let spec = &self.specs[self.index];
        let color = spec.colors[if self.phase == 2 { 1 } else { 0 }];
        let size = source.size();
        let input = ImageInput::Composite {
            base: Box::new(ImageInput::Solid {
                size: self.canvas,
                color: [color[0], color[1], color[2], 255],
            }),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(source.clone())),
                source: [
                    0,
                    0,
                    size[0].min(self.canvas[0]) as i32,
                    size[1].min(self.canvas[1]) as i32,
                ],
                destination: [0, 0],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::MaskPiecewise256 {
                    weight: key.percent * 256 / 100,
                },
            }],
        };
        self.frame_job = Some((
            key,
            host.image_metadata(self.client)?
                .begin_process(input, b"[]".as_slice().into())?,
        ));
        self.api_calls += 1;
        Ok(false)
    }
}

/// Reviewed portable host policy: only the actual shared decoder/header
/// capability is known here. This never manufactures the original DirectShow
/// numeric codec status and does not claim that every audio packet can decode.
pub struct NativeStartupShell {
    client: ClientId,
    root: u32,
    pub profile: NativeStartupProfile,
    mode: NativeMode,
    probe: Option<crysirror_core::service::MediaProbeJob>,
    probe_info: Option<crysirror_core::service::MediaProbeInfo>,
    logo: NativeLogoPlayer,
    logo_return_started: bool,
    logo_fade: Option<crate::native_general::ExitFadePlayer>,
    logo_exit: Option<Arc<dyn ImageResource>>,
    opening_entry_pending: bool,
    nothing_job: Option<ImageJob>,
    nothing_frame: Option<Arc<dyn ImageResource>>,
    cover_frame: Option<Arc<dyn ImageResource>>,
    cover_path: Option<String>,
    nothing_published: bool,
    pub api_calls: u64,
}
impl NativeStartupShell {
    pub fn new(
        client: ClientId,
        root: u32,
        canvas: [u32; 2],
        frame_ms: u64,
        launch_count: u32,
        profile: NativeStartupProfile,
        resources: crate::resource_bridge::NativeResources,
    ) -> Result<Self> {
        ensure!(
            !profile.startup_jump_script,
            "NNN native configured startupJumpScript requires InitNewGame transition policy"
        );
        ensure!(
            profile.nothing_next == NativeMode::Logo && profile.logo_next == NativeMode::Opening,
            "NNN unreviewed startup mode chain"
        );
        let mut logo = NativeLogoPlayer::new(
            client,
            9000,
            canvas,
            frame_ms,
            launch_count,
            profile.logos.clone(),
            resources,
        )?;
        logo.configure_corner_colors(profile.logo_corner_colors.clone())?;
        Ok(Self {
            client,
            root,
            profile,
            mode: NativeMode::Nothing,
            probe: None,
            probe_info: None,
            logo,
            logo_return_started: false,
            logo_fade: None,
            logo_exit: None,
            opening_entry_pending: false,
            nothing_job: None,
            nothing_frame: None,
            cover_frame: None,
            cover_path: None,
            nothing_published: false,
            api_calls: 0,
        })
    }
    pub fn preparation_pending(&self) -> bool {
        self.mode == NativeMode::Nothing
            || (self.mode == NativeMode::Logo && self.logo.preparation_pending())
    }
    pub fn dependency_pending(&self) -> bool {
        self.probe.is_some()
            || self.nothing_job.is_some()
            || self.logo.dependency_pending()
            || self
                .logo_fade
                .as_ref()
                .is_some_and(|fade| fade.dependency_pending())
    }
    pub fn mode(&self) -> NativeMode {
        self.mode
    }
    pub(crate) fn update_launch_count(&mut self, count: u32) {
        self.logo.launch_count = count;
    }
    /// Re-enter the existing native objects without VarInit/SystemFile reload.
    /// Nothing's decoder check is constructor-scoped and is intentionally kept;
    /// Logo.Init starts its sequence again with the same launch-count threshold.
    pub fn restart(
        &mut self,
        host: &mut dyn HostServices,
        mode: NativeMode,
        exit: Arc<dyn ImageResource>,
    ) -> Result<()> {
        ensure!(
            matches!(
                mode,
                NativeMode::Nothing | NativeMode::Logo | NativeMode::Opening
            ),
            "NNN native startup re-entry mode"
        );
        let mut logo = NativeLogoPlayer::new(
            self.client,
            self.logo.layer,
            self.logo.canvas,
            self.logo.frame_ms,
            self.logo.launch_count,
            self.profile.logos.clone(),
            self.logo.resources.clone(),
        )?;
        logo.configure_corner_colors(self.profile.logo_corner_colors.clone())?;
        let cover = if mode == NativeMode::Nothing {
            self.nothing_frame
                .clone()
                .context("NNN Nothing original prepared fill absent")?
        } else {
            exit
        };
        self.publish_cover(host, cover)?;
        self.logo = logo;
        self.logo_return_started = false;
        self.logo_fade = None;
        self.logo_exit = None;
        self.opening_entry_pending = mode == NativeMode::Opening;
        self.nothing_published = mode == NativeMode::Nothing;
        self.mode = mode;
        Ok(())
    }
    pub(crate) fn exit_frame(&self) -> Option<Arc<dyn ImageResource>> {
        self.cover_frame.clone()
    }
    /// Resource completion may finish Logo outside an ordinary input tick.
    /// Retain the handoff until the opening VM owner actually consumes it.
    pub(crate) fn take_opening_entry_frame(&mut self) -> Result<Option<Arc<dyn ImageResource>>> {
        if !self.opening_entry_pending {
            return Ok(None);
        }
        ensure!(
            self.mode == NativeMode::Opening,
            "NNN pending Opening entry mode changed"
        );
        let frame = self
            .exit_frame()
            .context("NNN Opening inherited exit buffer absent")?;
        self.opening_entry_pending = false;
        Ok(Some(frame))
    }
    fn publish_cover(
        &mut self,
        host: &mut dyn HostServices,
        image: Arc<dyn ImageResource>,
    ) -> Result<()> {
        ensure!(
            image.client() == self.client && image.size() == self.logo.canvas,
            "NNN outer cover scope"
        );
        let published = host.publish_image_resource(self.client, 9001, image.clone())?;
        let path = published.path.relative().to_owned();
        let [w, h] = self.logo.canvas;
        let batch = serde_json::to_vec(
            &serde_json::json!({"layers":[{"op":"put","layer":{"id":9001,"rect":[0,0,w,h],"order":[1999,0],"paint":{"kind":"image","asset":path}}}]}),
        )?;
        if let Err(error) = host.submit_engine(self.client, 1, &batch) {
            let _ = host.release_image_resource(self.client, &path);
            return Err(error.into());
        }
        if let Some(old) = self.cover_path.replace(path) {
            host.release_image_resource(self.client, &old)?;
        }
        self.cover_frame = Some(image);
        self.api_calls += 2;
        Ok(())
    }
    pub(crate) fn release_cover(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(path) = self.cover_path.as_ref() {
            host.submit_engine(
                self.client,
                1,
                &serde_json::to_vec(&serde_json::json!({"layers":[{"op":"remove","id":9001}]}))?,
            )?;
            host.release_image_resource(self.client, path)?;
            self.cover_path = None;
            // The native exit-screen buffer outlives its visible presentation.
            // createExitScreenFlag=0 on a later return retains this generation.
            self.api_calls += 2;
        }
        Ok(())
    }
    pub(crate) fn enter_title(&mut self) -> Result<()> {
        ensure!(
            matches!(
                self.mode,
                NativeMode::Movie | NativeMode::Opening | NativeMode::Title
            ),
            "NNN unreviewed native Title transition"
        );
        self.mode = NativeMode::Title;
        self.opening_entry_pending = false;
        Ok(())
    }
    pub(crate) fn enter_movie(&mut self) -> Result<()> {
        ensure!(
            self.mode == NativeMode::Opening && self.profile.opening_next == NativeMode::Movie,
            "NNN unreviewed native Movie transition"
        );
        self.mode = NativeMode::Movie;
        self.opening_entry_pending = false;
        Ok(())
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"mode":format!("{:?}",self.mode),"logo":self.logo.native_location(),"dependency_pending":self.dependency_pending(),"legacy_codec_limit":self.profile.codec_limit,"portable_codec_policy":"shared_video_header_then_live_media_transport","video_header_verified":self.probe_info.as_ref().map(|i|i.header_validated),"video_decoder":self.probe_info.as_ref().map(|i|i.video_decoder.as_str()),"audio_decoder":self.probe_info.as_ref().and_then(|i|i.audio_decoder.as_deref()),"audio_codec_verified":false,"engine_api_calls":self.api_calls+self.logo.api_calls})
    }
    /// True hands control to the original configured background SPT, not sys.
    pub fn poll(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<bool> {
        if self.mode == NativeMode::Nothing {
            if !self.nothing_published {
                let c = self.profile.nothing_color;
                if self.nothing_job.is_none() {
                    self.nothing_job = Some(host.image_metadata(self.client)?.begin_process(
                        ImageInput::Solid {
                            size: self.logo.canvas,
                            color: [c[0], c[1], c[2], 255],
                        },
                        b"[]".as_slice().into(),
                    )?);
                    self.api_calls += 1;
                }
                let image = match self.nothing_job.as_mut().unwrap().poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(error) => {
                        anyhow::bail!("NNN Nothing shared fill: {error}")
                    }
                    ImageProcessState::Ready(image) => image,
                };
                host.submit_engine(self.client,1,&serde_json::to_vec(&serde_json::json!({"layers":[{"op":"put","layer":{"id":self.root,"rect":[0,0,self.logo.canvas[0],self.logo.canvas[1]],"paint":{"kind":"solid","color":[c[0],c[1],c[2],255]}}}]}))?)?;
                self.api_calls += 1;
                self.publish_cover(host, image.clone())?;
                self.nothing_frame = Some(image);
                self.nothing_job = None;
                self.nothing_published = true;
            }
            if self.profile.codec_check && self.probe_info.is_none() {
                if self.probe.is_none() {
                    self.probe = Some(host.begin_media_probe(
                        self.client,
                        &VfsPath::new(Mount::Asset, &self.profile.codec_asset)?,
                    )?);
                    self.api_calls += 1;
                }
                match self.probe.as_mut().unwrap().poll()? {
                    crysirror_core::service::MediaProbeState::Pending => return Ok(false),
                    crysirror_core::service::MediaProbeState::Failed(error) => anyhow::bail!(
                        "NNN portable shared video decoder/header check failed (legacy DirectShow status not substituted): {error}"
                    ),
                    crysirror_core::service::MediaProbeState::Ready(info) => {
                        ensure!(
                            info.client == self.client
                                && info.header_validated
                                && !info.video_decoder.is_empty(),
                            "NNN shared video header capability not established"
                        );
                        self.probe_info = Some(info);
                        self.probe = None;
                    }
                }
            }
            self.mode = NativeMode::Logo;
        }
        if self.mode == NativeMode::Logo {
            if !self.logo.poll(host, input)? {
                return Ok(false);
            }
            if !self.logo_return_started {
                let frame = self
                    .logo
                    .final_frame
                    .clone()
                    .context("NNN Logo completed return frame absent")?;
                let policy = self.profile.logo_return;
                // Original ReturnFadeOut4f0cd0 captures Logo.Print's terminal
                // page when requested, otherwise preserves the old exit buffer.
                let exit = if policy.capture_exit {
                    frame.clone()
                } else {
                    self.cover_frame
                        .clone()
                        .context("NNN Logo retained exit buffer absent")?
                };
                let fade = policy
                    .fade
                    .map(|spec| {
                        crate::native_general::ExitFadePlayer::new(
                            host,
                            self.client,
                            frame.clone(),
                            spec,
                            self.logo.frame_ms,
                        )
                    })
                    .transpose()?;
                // Keep the completed Logo on screen while the shared colour
                // target is prepared. The old Nothing cover must not flash.
                self.publish_cover(host, frame)?;
                self.logo_exit = Some(exit);
                self.logo_fade = fade;
                self.logo_return_started = true;
                if self.logo_fade.is_some() {
                    // The mouse edge that ended Logo was already consumed by
                    //503070. It must not also skip the new General countdown.
                    return Ok(false);
                }
            }
            if let Some(fade) = self.logo_fade.as_mut() {
                use crysirror_core::service::buttons as B;
                fade.note_skip(input.buttons_pressed & (B::MOUSE_LEFT | B::MOUSE_RIGHT) != 0);
                let image = fade.poll(host)?;
                let finished = fade.finished();
                if let Some(image) = image {
                    if finished {
                        self.logo_exit = Some(image);
                    } else {
                        self.publish_cover(host, image)?;
                    }
                }
                if !finished {
                    return Ok(false);
                }
                self.logo_fade = None;
            }
            // GeneralCalcu4f2ce0 returns before the terminal GeneralPrint.
            // Preserve the last visible blend, but pass capture/FillExitScreen
            // as the next mode's native exit buffer, not that visible blend.
            self.cover_frame = Some(
                self.logo_exit
                    .take()
                    .context("NNN Logo native exit buffer absent")?,
            );
            self.mode = self.profile.logo_next;
            self.opening_entry_pending = self.mode == NativeMode::Opening;
        }
        Ok(self.mode == NativeMode::Opening)
    }
}

#[cfg(test)]
mod mode_reentry_tests {
    use super::*;
    use crate::{expression::VariableProfile, native_resources::ResourceNames, XorProfile};
    use std::time::{Duration, Instant};
    fn names(text: &str) -> Names {
        Names::parse(text.as_bytes(), false).unwrap()
    }
    #[test]
    fn nothing_logo_reentry_uses_shared_immutable_cover_and_preserves_old_generation() {
        check_reentry("", [255, 255, 255, 255], false);
    }
    #[test]
    fn logo_general_return_fade_preserves_native_capture_fill_and_visible_frame_separately() {
        check_reentry("exitFadeOutType 1 exitFadeOutTime 4 exitFadeOutColorR 20 exitFadeOutColorG 40 exitFadeOutColorB 60", [20, 40, 60, 255], true);
        check_reentry("exitFadeOutType 1 exitFadeOutTime 4 exitFadeOutColorR 20 exitFadeOutColorG 40 exitFadeOutColorB 60 fillExitScreenFlag 0", [255, 255, 255, 255], true);
        check_reentry("createExitScreenFlag 0", [7, 8, 9, 255], false);
    }
    fn check_reentry(logo_options: &str, expected_exit: [u8; 4], expect_fade: bool) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("game")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"native re-entry","content_revision":"1","assets":"game","canvas":[16,16],"clients":[{"name":"one","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        crysirror_formats_blackcyc::register_images(&mut runtime.assets.decoders).unwrap();
        runtime.tick(0, Default::default()).unwrap();
        let config = NativeConfiguration {
            main_control: None,
            resources: ResourceNames::new("").unwrap(),
            setup: names("dwqPackLevel 0"),
            variable_names: names(""),
            project: names("back"),
            commands: names(""),
            functions: names(""),
            initial: None,
            module_sha256: "authored".into(),
            profile: VariableProfile::Legacy1000,
            scripts: XorProfile::default(),
        };
        let profile=NativeStartupProfile::from_tables(&config,&names("fillColorR 7 fillColorG 8 fillColorB 9 nextMode logo"),&names(&format!("logoNumber 1 nextMode opening useCustomFileFlag 1 filename1 logo defaultFadeInTime 1 defaultKeepTime 1 defaultFadeOutTime 1 {logo_options}")),&names("backScriptFlag 1 backScriptName back"),names("backScriptFlag 1 backScriptName back"),names("movieFileName a.mpg")).unwrap();
        let image = crysirror_image::ImageSurface::filled(2, 1, [80, 90, 100, 255]).unwrap();
        let mut bytes = vec![b' '; 64];
        bytes[..4].copy_from_slice(b"PNG ");
        bytes[36..40].copy_from_slice(&2u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&1u32.to_le_bytes());
        bytes[48..64].copy_from_slice(b"PACKTYPE=8      ");
        bytes.extend(crysirror_image::encode_png(&image).unwrap());
        let bytes: Arc<[u8]> = bytes.into();
        let resources = crate::resource_bridge::NativeResources::blackcyc(
            ClientId(1),
            Arc::new(move |path, cancel| {
                cancel.check()?;
                ensure!(path == "dwq/sys/logo.dwq", "authored logo path");
                Ok(Some(Arc::new(crysirror_core::service::MemoryAssetReader(
                    bytes.clone(),
                ))))
            }),
        )
        .unwrap();
        let mut shell =
            NativeStartupShell::new(ClientId(1), 1, [16, 16], 20, 0, profile, resources).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut saw_intermediate = false;
        while shell.mode() != NativeMode::Opening {
            shell.poll(&mut runtime.host, Default::default()).unwrap();
            if shell.logo_fade.is_some() {
                if let Some(frame) = &shell.cover_frame {
                    let pixel = frame.pixel_at([0, 0]).unwrap().unwrap();
                    saw_intermediate |= pixel != [255, 255, 255, 255] && pixel != [20, 40, 60, 255];
                }
            }
            if !shell.dependency_pending() {
                runtime.tick(20, Default::default()).unwrap();
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            saw_intermediate, expect_fade,
            "nonzero return must produce an actual shared blend"
        );
        let old = shell.exit_frame().unwrap();
        let completed_at = runtime.host.now_ms();
        for _ in 0..5 {
            assert!(shell
                .poll(&mut runtime.host, InputFrame::default())
                .unwrap());
        }
        assert_eq!(runtime.host.now_ms(), completed_at);
        assert!(Arc::ptr_eq(
            &shell.take_opening_entry_frame().unwrap().unwrap(),
            &old
        ));
        assert!(
            shell.take_opening_entry_frame().unwrap().is_none(),
            "the native entry is consumed only once even after completion-only polls"
        );
        assert!(
            old.rgba()
                .chunks_exact(4)
                .all(|pixel| pixel == expected_exit),
            "native capture/fill is distinct from the last displayed blend"
        );
        let saved = old.size();
        let fill = shell.nothing_frame.clone().unwrap();
        shell.release_cover(&mut runtime.host).unwrap();
        assert!(runtime.host.engine.layers.get(ClientId(1), 9001).is_none());
        assert!(
            Arc::ptr_eq(&shell.exit_frame().unwrap(), &old),
            "original exit buffer outlives visible layer"
        );
        let clock = runtime.host.now_ms();
        shell
            .restart(&mut runtime.host, NativeMode::Nothing, old.clone())
            .unwrap();
        assert_eq!(shell.mode(), NativeMode::Nothing);
        assert!(Arc::ptr_eq(&shell.exit_frame().unwrap(), &fill));
        let layer = runtime.host.engine.layers.get(ClientId(1), 9001).unwrap();
        let crysirror_scene::layer::Paint::Image { asset, .. } = &layer.paint else {
            panic!("shared image cover")
        };
        assert_eq!(
            &runtime.assets.surface(ClientId(1), asset).unwrap().rgba[..4],
            &[7, 8, 9, 255]
        );
        assert_eq!(
            runtime.host.now_ms(),
            clock,
            "re-entry never advances clock"
        );
        assert_eq!(old.size(), saved, "old immutable frame still lives");
        shell
            .restart(&mut runtime.host, NativeMode::Opening, old.clone())
            .unwrap();
        assert!(Arc::ptr_eq(&shell.exit_frame().unwrap(), &old));
        assert_eq!(shell.mode(), NativeMode::Opening);
        assert!(Arc::ptr_eq(
            &shell.take_opening_entry_frame().unwrap().unwrap(),
            &old
        ));
    }
}
