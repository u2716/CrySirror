// SPDX-License-Identifier: MPL-2.0
//! Standalone ConfigMgr opens the persistent system UI over the title.
use super::*;

#[derive(Clone, Copy, PartialEq)]
enum Control {
    Volume(Option<Kind>),
    MessageTransparency,
}
const SLIDERS: [(&str, Control); 7] = [
    ("Sound/MasterVolume", Control::Volume(None)),
    ("Sound/SystemVolume", Control::Volume(Some(Kind::System))),
    ("Sound/MusicVolume", Control::Volume(Some(Kind::Bgm))),
    ("Sound/VoiceVolume", Control::Volume(Some(Kind::Voice))),
    ("Sound/SoundEffectVolume", Control::Volume(Some(Kind::Se))),
    (
        "Sound/SoundEnvironmentVolume",
        Control::Volume(Some(Kind::Env)),
    ),
    (
        "Message/MessageWindowTransparency",
        Control::MessageTransparency,
    ),
];
const ROOT: &str = "SystemUI Root/ConfigPanel/";
type Rect = [f32; 4];
fn inside(rect: Rect, input: InputFrame) -> bool {
    let (x, y) = (input.pointer_x as f32, input.pointer_y as f32);
    x >= rect[0] && x < rect[0] + rect[2] && y >= rect[1] && y < rect[1] + rect[3]
}
struct Slider {
    control: Control,
    track: Rect,
    hit: Rect,
    thumb: usize,
}
pub(super) struct Panel {
    first: usize,
    opened: u64,
    closing: Option<u64>,
    ok: Rect,
    ok_pressed: bool,
    drag: Option<usize>,
    sliders: Vec<Slider>,
}
impl Panel {
    pub fn opacity(&self, index: usize, now: u64) -> Option<u8> {
        if index < self.first {
            return None;
        }
        let fade = |start: u64| (now.saturating_sub(start) as f32 / 300.).min(1.);
        Some(
            (255.
                * self
                    .closing
                    .map_or_else(|| fade(self.opened), |start| 1. - fade(start)))
            .round() as u8,
        )
    }
}
impl Game {
    pub(super) fn open_config(
        &mut self,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        now: u64,
    ) -> Result<()> {
        let mut rows = self.assets.scene(op, host, client, "StandaloneConfig")?;
        let first = self.widgets.len();
        let floor = rows
            .iter()
            .map(|w| w.depth)
            .min()
            .context("empty config panel")?;
        let top = self
            .widgets
            .iter()
            .map(|w| w.depth)
            .max()
            .unwrap_or_default()
            + 1;
        for w in &mut rows {
            w.depth = top + (w.depth - floor);
            let p = w.path.strip_prefix(ROOT).unwrap_or("");
            if p.contains('/')
                && !SLIDERS.iter().any(|(path, _)| {
                    p.strip_prefix(path)
                        .is_some_and(|tail| tail.is_empty() || tail.starts_with('/'))
                        && !p.ends_with("/3D")
                })
            {
                w.tint[3] = w.tint[3].min(120);
            }
        }
        let ok = rows
            .iter()
            .find(|w| w.path.ends_with("/ConfigPanel/OK"))
            .and_then(|w| w.collider)
            .context("missing system config OK")?;
        let mut sliders = vec![];
        for (path, control) in SLIDERS {
            let path = format!("{ROOT}{path}/Slider");
            let bar = rows
                .iter()
                .find(|w| w.path == path)
                .with_context(|| format!("missing {path}"))?;
            let hit = bar.collider.context("config slider has no collider")?;
            let track = rows
                .iter()
                .find(|w| w.path == format!("{path}/Foreground"))
                .map_or(bar.rect, |w| w.rect);
            ensure!(track[2] > 0., "empty config track {path}");
            let thumb = rows
                .iter()
                .position(|w| w.path == format!("{path}/Thumb"))
                .context("missing config slider thumb")?;
            let value = match control {
                Control::MessageTransparency => self.message_transparency,
                Control::Volume(None) => self.sound.all_volume,
                Control::Volume(Some(k)) => self.sound.volumes[k as usize],
            };
            let w = &mut rows[thumb];
            business::move_by(
                w,
                track[0] + track[2] * value / 100. - (w.rect[0] + w.rect[2] / 2.),
                0.,
            );
            sliders.push(Slider {
                control,
                track,
                hit,
                thumb: first + thumb,
            });
        }
        self.widgets.extend(rows);
        self.config_panel = Some(Panel {
            first,
            opened: now,
            closing: None,
            ok,
            ok_pressed: false,
            drag: None,
            sliders,
        });
        self.pressed = None;
        Ok(())
    }

    pub(super) fn update_config(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        let panel = self.config_panel.as_mut().context("missing config panel")?;
        if let Some(start) = panel.closing {
            if now.saturating_sub(start) >= 300 {
                self.widgets.truncate(panel.first);
                self.config_panel = None;
            }
            return Ok(());
        }
        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            panel.ok_pressed = inside(panel.ok, input);
            panel.drag = panel
                .sliders
                .iter()
                .position(|s| inside(s.hit, input) || inside(self.widgets[s.thumb].rect, input));
        }
        if let Some(i) = panel.drag
            && (input.buttons_down | released) & buttons::MOUSE_LEFT != 0
        {
            let s = &panel.sliders[i];
            let value =
                (((input.pointer_x as f32 - s.track[0]) / s.track[2]).clamp(0., 1.) * 100.).ceil();
            match s.control {
                Control::Volume(k) => {
                    let prev = k.map_or(self.sound.all_volume, |k| self.sound.volumes[k as usize]);
                    if prev != value {
                        self.sound.set_volume(host, client, k, value)?;
                    }
                }
                Control::MessageTransparency => self.message_transparency = value,
            }
            let w = &mut self.widgets[s.thumb];
            business::move_by(
                w,
                s.track[0] + s.track[2] * value / 100. - (w.rect[0] + w.rect[2] / 2.),
                0.,
            );
        }
        if released & buttons::MOUSE_LEFT != 0 {
            if panel.ok_pressed && inside(panel.ok, input) {
                panel.closing = Some(now);
            }
            panel.ok_pressed = false;
            panel.drag = None;
        }
        Ok(())
    }
}

#[test]
#[ignore = "requires installed standalone system UI and title"]
fn installed_title_config_modal_sound_and_return() -> Result<()> {
    let mut m = installed_menu::Menu::new()?;
    m.title()?;
    let config = m
        .game
        .widgets
        .iter()
        .find(|w| w.path.ends_with("/ConfigGroup/Config"))
        .context("title config button")?
        .rect;
    m.click(config)?;
    m.tick(300)?;
    let panel = m
        .game
        .config_panel
        .as_ref()
        .context("title did not open config")?;
    let slider = panel
        .sliders
        .iter()
        .find(|s| s.control == Control::Volume(None))
        .unwrap();
    let [x, y, width, height] = slider.track;
    let quarter = [x + width * 0.25, y, 1., height];
    m.click(quarter)?;
    assert!((24.0..=26.).contains(&m.game.sound.all_volume));
    let changed = m.game.sound.all_volume;
    let start = m
        .game
        .widgets
        .iter()
        .find(|w| w.path.ends_with("/StartGroup/Start"))
        .context("title start button")?
        .rect;
    m.click(start)?;
    assert!(m.game.leaving.is_none());
    let ok = m.game.config_panel.as_ref().unwrap().ok;
    m.click(ok)?;
    m.tick(300)?;
    assert!(m.game.config_panel.is_none());
    assert_eq!(m.game.scene, Scene::Title);
    m.click(config)?;
    let slider = m
        .game
        .config_panel
        .as_ref()
        .unwrap()
        .sliders
        .iter()
        .find(|s| s.control == Control::Volume(None))
        .unwrap();
    let thumb = m.game.widgets[slider.thumb].rect;
    assert!((thumb[0] + thumb[2] / 2. - (x + width * changed / 100.)).abs() < 0.1);
    Ok(())
}
