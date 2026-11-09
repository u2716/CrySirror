// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original General mode-return policy. Rendering stays in shared image/layer
//! services; this module selects the original ordinary/special exit profile.
use crate::{config::Names, native_shell::NativeMode};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::HostServices,
    state::ClientId,
};
use crysirror_image::{
    effects::ImageEffect,
    fixed_mix::{FixedMixAlpha, FixedMixRounding, FixedMixSpec},
    transition::ImageTransition,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModeReturn {
    pub capture_exit: bool,
    pub fade: Option<ExitFade>,
}

/// Legacy4f29a0 PrintSimpleWipe(count, time-1, 0), not the -1 Overrap
/// branch (which reuses a prior enter-screen buffer). Pixels are engine-owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExitFade {
    pub frames: u32,
    pub color: [u8; 4],
    pub fill_exit: Option<[u8; 4]>,
}

fn color(table: &Names, prefix: &str) -> Result<[u8; 4]> {
    //4f0e20 packs signed native words before writing BGR, rather than
    // independently clamping each component. Preserve overlapping high bits.
    let r = table.integer(format!("{prefix}R").as_bytes())?.unwrap_or(0);
    let g = table.integer(format!("{prefix}G").as_bytes())?.unwrap_or(0);
    let b = table.integer(format!("{prefix}B").as_bytes())?.unwrap_or(0);
    let packed = r.wrapping_shl(16) | g.wrapping_shl(8) | b;
    Ok([(packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255])
}

impl ModeReturn {
    pub(crate) fn validate_entry(table: &Names) -> Result<()> {
        ensure!(
            table.integer(b"enterFadeInType")?.unwrap_or(0) == 0,
            "NNN General entry fade requires its original countdown/shared exact effect"
        );
        ensure!(
            table.integer(b"enterExitVoiceFlag")?.unwrap_or(0) == 0,
            "NNN General delayed entry/exit voice requires its original trigger policy"
        );
        Ok(())
    }
    ///4f0cd0 captures before admitting the countdown.4f2640 reads special
    /// create only with nonzero special type, but special fill unconditionally.
    pub fn read(table: &Names, target: Option<NativeMode>) -> Result<Self> {
        // NewGame uses -1, which cannot match a positive system mode. Other
        // configured mode names don't need an implemented consumer merely to
        // select the ordinary branch. The original lookup includes atoi.
        let special_number = table
            .parameter(b"exitFadeOutSpecialMode")
            .map(crate::native_mode::number)
            .transpose()?
            .unwrap_or(-2);
        let special =
            special_number >= 0 && target.is_some_and(|mode| mode as i32 == special_number);
        let (fade_key, time_key, color_prefix, fill_key) = if special {
            (
                b"exitFadeOutTypeSpecial".as_slice(),
                b"exitFadeOutTimeSpecial".as_slice(),
                "exitFadeOutColorSpecial",
                b"fillExitScreenFlagSpecial".as_slice(),
            )
        } else {
            (
                b"exitFadeOutType".as_slice(),
                b"exitFadeOutTime".as_slice(),
                "exitFadeOutColor",
                b"fillExitScreenFlag".as_slice(),
            )
        };
        let kind = table.integer(fade_key)?.unwrap_or(0);
        ensure!(
            matches!(kind, 0 | 1),
            "NNN General exit fade type {kind} requires its exact native wipe/retained enter-screen profile"
        );
        let fade = if kind == 0 {
            None
        } else {
            let frames = table.integer(time_key)?.unwrap_or(10);
            ensure!(
                (1..=864000).contains(&frames),
                "NNN General exit fade time bounds"
            );
            Some(ExitFade {
                frames: frames as u32,
                color: color(table, color_prefix)?,
                // Legacy FillExitScreen4f0e90 always uses ordinary RGB even
                // on the special branch. Do not substitute the fade colour.
                fill_exit: (table.integer(fill_key)?.unwrap_or(i32::from(!special)) != 0)
                    .then(|| {
                        //4f2640 reads ordinary RGB only for a nonzero
                        // ordinary type. In a special-only configuration the
                        // constructor's black survives any ordinary RGB token.
                        if table.integer(b"exitFadeOutType")?.unwrap_or(0) == 0 {
                            Ok([0, 0, 0, 255])
                        } else {
                            color(table, "exitFadeOutColor")
                        }
                    })
                    .transpose()?,
            })
        };
        Ok(Self {
            // The special create flag is read only for nonzero special fades.
            // With type zero the constructor's false survives a config token.
            capture_exit: kind != 0
                || (!special && table.integer(b"createExitScreenFlag")?.unwrap_or(1) != 0),
            fade,
        })
    }
}

/// Only policy and immutable engine handles live here. A dependency pump drains
/// an admitted job; only an owner tick can advance the native countdown.
pub(crate) struct ExitFadePlayer {
    client: ClientId,
    source: Arc<dyn ImageResource>,
    target: Option<Arc<dyn ImageResource>>,
    spec: ExitFade,
    frame_ms: u64,
    next_ms: Option<u64>,
    count: u32,
    job: Option<ImageJob>,
    finished: bool,
    skip: bool,
}
impl ExitFadePlayer {
    pub fn new(
        host: &mut dyn HostServices,
        client: ClientId,
        source: Arc<dyn ImageResource>,
        spec: ExitFade,
        frame_ms: u64,
    ) -> Result<Self> {
        ensure!(
            source.client() == client
                && (1..=60000).contains(&frame_ms)
                && (1..=864000).contains(&spec.frames),
            "NNN General exit fade scope/clock"
        );
        let job = host.image_metadata(client)?.begin_process(
            ImageInput::Solid {
                size: source.size(),
                color: spec.color,
            },
            b"[]".as_slice().into(),
        )?;
        Ok(Self {
            client,
            source,
            target: None,
            spec,
            frame_ms,
            next_ms: None,
            count: 0,
            job: Some(job),
            finished: false,
            skip: false,
        })
    }
    pub fn dependency_pending(&self) -> bool {
        self.job.is_some()
    }
    pub fn finished(&self) -> bool {
        self.finished
    }
    pub fn note_skip(&mut self, skip: bool) {
        self.skip |= skip;
    }
    pub fn poll_dependency(&mut self) -> Result<Option<Arc<dyn ImageResource>>> {
        let Some(job) = self.job.as_mut() else {
            return Ok(None);
        };
        match job.poll()? {
            ImageProcessState::Pending => Ok(None),
            ImageProcessState::Failed(error) => {
                anyhow::bail!("NNN General shared exit fade: {error}")
            }
            ImageProcessState::Ready(image) => {
                ensure!(
                    image.client() == self.client && image.size() == self.source.size(),
                    "NNN General exit fade result scope"
                );
                self.job = None;
                if self.target.is_none() {
                    self.target = Some(image);
                    return Ok(None);
                }
                self.finished = self.count == self.spec.frames;
                Ok(Some(image))
            }
        }
    }
    pub fn poll(&mut self, host: &mut dyn HostServices) -> Result<Option<Arc<dyn ImageResource>>> {
        if self.job.is_some() {
            return self.poll_dependency();
        }
        if self.finished {
            return Ok(None);
        }
        let now = host.now_ms();
        if self.next_ms.is_some_and(|next| now < next) {
            return Ok(None);
        }
        let count = if self.skip {
            self.spec.frames
        } else {
            self.count + 1
        };
        let terminal = count == self.spec.frames;
        if terminal && self.spec.fill_exit.is_none() {
            self.count = count;
            self.finished = true;
            return Ok(Some(self.source.clone()));
        }
        let (input, effects) = if terminal {
            // GeneralCalcu returns before GeneralPrint on the terminal step.
            // The next mode receives the capture or FillExitScreen, never a
            // last blended display substituted for the original exit buffer.
            (
                ImageInput::Solid {
                    size: self.source.size(),
                    color: self.spec.fill_exit.unwrap(),
                },
                vec![],
            )
        } else {
            let percent = (u64::from(count) * 100
                / u64::from(self.spec.frames.saturating_sub(1).max(1)))
            .min(100);
            (
                ImageInput::Transition {
                    from: self.source.clone(),
                    to: self
                        .target
                        .clone()
                        .context("NNN General exit target absent")?,
                    mask: None,
                },
                vec![ImageEffect::Transition {
                    spec: ImageTransition {
                        progress: percent as f32 / 100.,
                        softness: 0.,
                        blur: 0.,
                        rule_table: None,
                        fixed_weighted: Some(FixedMixSpec {
                            left_weight: ((100 - percent) * 256 / 100) as u32,
                            right_weight: (percent * 256 / 100) as u32,
                            fractional_bits: 8,
                            rounding: FixedMixRounding::SumThenShift,
                            alpha: FixedMixAlpha::Opaque,
                        }),
                    },
                }],
            )
        };
        self.job = Some(
            host.image_metadata(self.client)?
                .begin_process(input, serde_json::to_vec(&effects)?.into())?,
        );
        self.count = count;
        self.skip = false;
        self.next_ms = Some(
            now.checked_add(self.frame_ms)
                .context("NNN General exit clock overflow")?,
        );
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn names(text: &str) -> Names {
        Names::parse(text.as_bytes(), false).unwrap()
    }
    #[test]
    fn ordinary_special_and_newgame_returns_select_original_profile_before_mutation() {
        assert!(ModeReturn::read(&names(""), None).unwrap().capture_exit);
        let table = names(
            "exitFadeOutType 2 exitFadeOutSpecialMode nothing exitFadeOutTypeSpecial 0 createExitScreenFlagSpecial 1",
        );
        assert!(
            !ModeReturn::read(&table, Some(NativeMode::Nothing))
                .unwrap()
                .capture_exit
        );
        assert!(ModeReturn::read(&table, Some(NativeMode::Opening)).is_err());
        assert!(ModeReturn::read(&table, None).is_err());
        let numeric =
            names("exitFadeOutType 2 exitFadeOutSpecialMode +1suffix exitFadeOutTypeSpecial 0");
        assert!(
            !ModeReturn::read(&numeric, Some(NativeMode::Nothing))
                .unwrap()
                .capture_exit
        );
        assert!(ModeReturn::read(&numeric, Some(NativeMode::Title)).is_err());
        assert!(
            !ModeReturn::read(&names("createExitScreenFlag 0"), None)
                .unwrap()
                .capture_exit
        );
        assert!(
            ModeReturn::read(&names("exitFadeOutSpecialMode unknown"), None)
                .unwrap()
                .capture_exit
        );
    }
    #[test]
    fn exit_fade_selects_native_defaults_and_special_fill_uses_ordinary_rgb() {
        let ordinary = ModeReturn::read(&names("exitFadeOutType 1 createExitScreenFlag 0 exitFadeOutColorR 4 exitFadeOutColorG 5 exitFadeOutColorB 6"), None).unwrap();
        assert!(
            ordinary.capture_exit,
            "nonzero type forces capture even when create is disabled"
        );
        assert_eq!(
            ordinary.fade,
            Some(ExitFade {
                frames: 10,
                color: [4, 5, 6, 255],
                fill_exit: Some([4, 5, 6, 255])
            })
        );
        let special = names("exitFadeOutType 1 exitFadeOutSpecialMode nothing exitFadeOutTypeSpecial 1 exitFadeOutTimeSpecial 3 exitFadeOutColorSpecialR 70 exitFadeOutColorR 4 exitFadeOutColorG 5 exitFadeOutColorB 6 fillExitScreenFlagSpecial 1");
        let spec = ModeReturn::read(&special, Some(NativeMode::Nothing))
            .unwrap()
            .fade
            .unwrap();
        assert_eq!(spec.color, [70, 0, 0, 255]);
        assert_eq!(spec.frames, 3);
        assert_eq!(spec.fill_exit, Some([4, 5, 6, 255]));
        let special_only = ModeReturn::read(&names("exitFadeOutSpecialMode nothing exitFadeOutTypeSpecial 1 exitFadeOutColorR 90 fillExitScreenFlagSpecial 1"), Some(NativeMode::Nothing)).unwrap();
        assert_eq!(special_only.fade.unwrap().fill_exit, Some([0, 0, 0, 255]));
        assert_eq!(color(&names("xG 256 xB -1"), "x").unwrap(), [255; 4]);
        assert!(ModeReturn::read(&names("exitFadeOutType 1 exitFadeOutTime 0"), None).is_err());
    }
    #[test]
    fn shared_exit_pixels_clock_pumps_skip_and_handoff_buffers_remain_distinct() {
        use std::time::{Duration, Instant};
        fn drain(fade: &mut ExitFadePlayer) -> Option<Arc<dyn ImageResource>> {
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut result = None;
            while fade.dependency_pending() {
                if let Some(image) = fade.poll_dependency().unwrap() {
                    result = Some(image);
                }
                assert!(Instant::now() < deadline, "shared image worker timed out");
                std::thread::yield_now();
            }
            result
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project = serde_json::from_value(serde_json::json!({"schema":2,"name":"native fade","content_revision":"1","assets":"assets","canvas":[1,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        let client = ClientId(1);
        let mut source_job = runtime
            .host
            .image_metadata(client)
            .unwrap()
            .begin_process(
                ImageInput::Solid {
                    size: [1, 1],
                    color: [200, 100, 50, 255],
                },
                b"[]".as_slice().into(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let source = loop {
            match source_job.poll().unwrap() {
                ImageProcessState::Ready(image) => break image,
                ImageProcessState::Failed(error) => panic!("{error}"),
                ImageProcessState::Pending => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
            }
        };
        for fill_exit in [None, Some([4, 5, 6, 255])] {
            let mut fade = ExitFadePlayer::new(
                &mut runtime.host,
                client,
                source.clone(),
                ExitFade {
                    frames: 3,
                    color: [100, 20, 40, 255],
                    fill_exit,
                },
                50,
            )
            .unwrap();
            assert!(drain(&mut fade).is_none());
            fade.poll(&mut runtime.host).unwrap();
            let first = drain(&mut fade).unwrap();
            assert_eq!(&*first.rgba(), &[150, 60, 45, 255]);
            for _ in 0..100 {
                assert!(fade.poll_dependency().unwrap().is_none());
            }
            assert_eq!(fade.count, 1, "dependency pumps cannot advance native time");
            assert!(fade.poll(&mut runtime.host).unwrap().is_none());
            assert_eq!(fade.count, 1);
            runtime.tick(50, Default::default()).unwrap();
            fade.poll(&mut runtime.host).unwrap();
            assert_eq!(&*drain(&mut fade).unwrap().rgba(), &[100, 20, 40, 255]);
            assert!(!fade.finished());
            // A fresh skip edge is retained until the next native Calcu;
            // neither a timer wait nor an admitted worker may be bypassed.
            fade.note_skip(true);
            assert!(fade.poll(&mut runtime.host).unwrap().is_none());
            runtime.tick(50, Default::default()).unwrap();
            let mut result = fade.poll(&mut runtime.host).unwrap();
            if result.is_none() {
                result = drain(&mut fade);
            }
            assert!(fade.finished());
            assert_eq!(
                &*result.unwrap().rgba(),
                &fill_exit.unwrap_or([200, 100, 50, 255])
            );
            assert_eq!(&*source.rgba(), &[200, 100, 50, 255]);
        }
    }
}
