// SPDX-License-Identifier: MPL-2.0
use super::*;
use crysirror_core::image_processing::{ImageJob, ImageProcessState};
use crysirror_core::path::{Mount, VfsPath};
const ROOT: &str = "UI Root/MainScreen/PhotoWindowManager/";
const WINDOWS: &[(&str, &str)] = &[
    ("BG", "BGWindow"),
    ("BGObject", "BGObjectWindow"),
    ("CharaPlacement", "PlacementWindow"),
    ("Transform", "ObjectManagerWindow"),
    ("Motion", "MotionWindow"),
    ("Face", "FaceWindow"),
    ("Light", "LightWindow"),
    ("Effect", "EffectWindow"),
    ("Sound", "SoundWindow"),
    ("PoseEdit", "PoseEditWindow"),
    ("Message", "MessageWindow"),
    ("ScenePreset", "ScenePresetWindow"),
    ("AlignmentPreset", "AlignmentPresetWindow"),
];
pub(in crate::com3d2) fn photo_widget(path: &str) -> bool {
    let Some(sub) = path.strip_prefix(ROOT) else {
        return false;
    };
    sub.starts_with("WindowVisibleBtnsParent/")
        || WINDOWS.iter().any(|(_, w)| {
            sub.strip_prefix(w).is_some_and(|s| {
                s.starts_with("/Parent/BG/TitleBar") || s == "/Parent/BG" || s == "/Parent/BG2"
            })
        })
}
#[derive(Default)]
pub(super) struct Studio {
    hidden: bool,
    opened: BTreeSet<String>,
    backgrounds: Vec<(String, String)>,
    selected: usize,
    scroll: usize,
    shot: Option<(VfsPath, ImageJob)>,
}
impl Game {
    pub(super) fn open_studio(&mut self) -> Result<String> {
        self.adv_camera = ([0., 1.5, 0.], 2., [180., 10.]);
        let mut vm = self
            .script
            .tjs
            .lock()
            .map_err(|_| anyhow::anyhow!("TJS lock"))?;
        let s = &mut vm.runtime.host_mut().state.retail;
        let t = s.table("phot_bg_list.nei")?;
        let mut backgrounds = Vec::new();
        for r in 1..t.rows {
            if t.exists(0, r)
                && !t.exists(4, r)
                && s.enabled("phot_bg_enabled_list", t.int(0, r))?
            {
                backgrounds.push((
                    format!("{}　{}", t.string(1, r), t.string(2, r)),
                    t.string(3, r),
                ));
            }
        }
        drop(vm);
        let bg = backgrounds
            .first()
            .context("empty photo background catalog")?
            .1
            .clone();
        self.leisure.studio.backgrounds = backgrounds;
        self.leisure.studio.opened = ["BGWindow", "FaceWindow", "MotionWindow", "PlacementWindow"]
            .map(str::to_owned)
            .into();
        self.studio_widgets();
        Ok(bg)
    }
    fn studio_widgets(&mut self) {
        let s = &self.leisure.studio;
        self.widgets = self
            .leisure
            .base
            .iter()
            .filter(|w| {
                let path = w.path.strip_prefix(ROOT).unwrap_or("");
                if path.starts_with("WindowVisibleBtnsParent/") {
                    if s.hidden {
                        return path.contains("/UION") || path.contains("/ScreenShot");
                    }
                    return !path.contains("/UION") && !path.contains("/CheckMark/CheckMark")
                        || WINDOWS.iter().any(|(n, window)| {
                            path.contains(&format!("/WindowVisible{n}/BG/CheckMark/CheckMark"))
                                && s.opened.contains(*window)
                        });
                }
                if s.hidden {
                    return false;
                }
                let key = path.split('/').next().unwrap_or("");
                s.opened.contains(key)
            })
            .cloned()
            .collect();
        if !s.hidden && s.opened.contains("BGWindow") {
            if let Some(title) = self
                .widgets
                .iter()
                .find(|w| w.path.ends_with("BGWindow/Parent/BG/TitleBar/Text"))
                .cloned()
            {
                for (i, (name, _)) in s.backgrounds.iter().enumerate().skip(s.scroll).take(8) {
                    let mut w = title.clone();
                    w.path = format!("photo/background/{i}");
                    w.rect = [
                        title.rect[0] + 3.,
                        title.rect[1] + 35. + (i - s.scroll) as f32 * 28.,
                        300.,
                        27.,
                    ];
                    w.collider = Some(w.rect);
                    w.depth += 5;
                    if let Some(l) = &mut w.label {
                        l.set_text(name);
                        l.set_size(18);
                    }
                    if i == s.selected {
                        w.tint = [255, 211, 126, 255];
                    }
                    self.widgets.push(w);
                }
            }
        }
    }
    pub(super) fn studio_scroll(&mut self, input: &InputFrame) {
        if input.buttons_pressed & (buttons::WHEEL_UP | buttons::WHEEL_DOWN) != 0
            && self.widgets.iter().any(|w| {
                w.path.starts_with("photo/background/") && business::contains(w.rect, input)
            })
        {
            let s = &mut self.leisure.studio;
            s.scroll = s
                .scroll
                .saturating_add_signed(if input.buttons_pressed & buttons::WHEEL_DOWN != 0 {
                    1
                } else {
                    -1
                })
                .min(s.backgrounds.len().saturating_sub(8));
            self.studio_widgets();
        }
    }
    pub(super) fn studio_click(
        &mut self,
        path: &str,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        now: u64,
    ) -> Result<()> {
        if path.ends_with("WindowVisibleBtnsLine2/End") {
            return self.leisure_prompt(
                op,
                host,
                c,
                now,
                "セーブしていないデータは失われます。\n終了しますか？",
                "okcancel",
                Response::Exit,
            );
        }
        if path.ends_with("/ScreenShot") {
            if self.leisure.studio.shot.is_none() {
                let (target, distance, angles) = self.adv_camera;
                let view = crate::bg::orbit_camera(target, angles, distance, 35.);
                let vector = |key: &str| serde_json::from_value(view["camera"][key].clone());
                let job = host.begin_world_photo(
                    c,
                    crysirror_core::graphics::WorldPhoto {
                        size: [1920, 1080],
                        eye: vector("eye")?,
                        target: vector("target")?,
                        up: [0., 1., 0.],
                        fov_y_degrees: 35.,
                        near: 0.01,
                        far: 1000.,
                        entities: vec![1],
                    },
                )?;
                let path = VfsPath::new(
                    Mount::Save,
                    &format!(
                        "ScreenShot/img{}.png",
                        chrono::Local::now().format("%Y%m%d%H%M%S")
                    ),
                )?;
                self.leisure.studio.shot = Some((path, job));
                self.sound
                    .play(host, c, &self.files, Kind::Se, "SE022.ogg", 0., false)?;
            }
            return Ok(());
        }
        let s = &mut self.leisure.studio;
        if let Some(index) = path.strip_prefix("photo/background/") {
            let i = index.parse::<usize>()?;
            let name = s.backgrounds[i].1.clone();
            let model = club::prefab(&mut self.assets.unity, op, &name)?;
            self.replace_leisure_model(&name, model, host, c)?;
            self.leisure.studio.selected = i;
        } else if path.ends_with("/UIOFF") {
            s.hidden = true;
        } else if path.ends_with("/UION") {
            s.hidden = false;
        } else if path.ends_with("/AllClose") {
            s.opened.clear();
        } else if let Some((_, w)) = WINDOWS
            .iter()
            .find(|(n, _)| path.ends_with(&format!("/WindowVisible{n}/BG")))
        {
            if !s.opened.remove(*w) {
                s.opened.insert((*w).into());
            }
        } else if let Some(w) = path.strip_prefix(ROOT).and_then(|p| p.split('/').next()) {
            if path.ends_with("/Btn/End") || path.ends_with("/Btn/Min") {
                s.opened.remove(w);
            }
        }
        self.studio_widgets();
        Ok(())
    }
    pub(super) fn publish_studio_shot(&mut self, op: &Opener) -> Result<()> {
        let Some((path, job)) = &mut self.leisure.studio.shot else {
            return Ok(());
        };
        match job.poll()? {
            ImageProcessState::Pending => return Ok(()),
            ImageProcessState::Failed(reason) => bail!("photo mode capture: {reason}"),
            ImageProcessState::Ready(img) => {
                let [width, height] = img.size();
                op.storage.write_atomic(
                    path,
                    &crysirror_image::encode_png(&crysirror_image::ImageSurface {
                        width,
                        height,
                        rgba: img.rgba().to_vec(),
                    })?,
                )?;
            }
        }
        self.leisure.studio.shot = None;
        Ok(())
    }
}
