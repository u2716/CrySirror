// SPDX-License-Identifier: MPL-2.0
//! Standalone DailyCtrl button bindings and DailyMgr menu policy.
use super::*;
use crysirror_core::service::{TextInputSessionHandle, TextInputSessionSpec};
use crysirror_unity::reader::Reader;
use std::collections::{BTreeMap, BTreeSet};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Period {
    Daytime,
    Night,
}

impl Period {
    pub fn from_tag(tag: Option<&String>) -> Result<Self> {
        match tag.map(String::as_str) {
            Some("Daytime") => Ok(Self::Daytime),
            Some("Night") => Ok(Self::Night),
            other => bail!("unsupported COM3D2 SceneDaily entry {other:?}"),
        }
    }
    pub fn tutorial(self) -> &'static str {
        match self {
            Self::Daytime => "SceneDailyDayTime",
            Self::Night => "SceneDailyNight",
        }
    }
    pub fn background(self) -> &'static str {
        match self {
            Self::Daytime => "ShinShitsumu_ChairRot",
            Self::Night => "ShinShitsumu_ChairRot_Night",
        }
    }
    pub fn bgm(self) -> &'static str {
        match self {
            Self::Daytime => "BGM020.ogg",
            Self::Night => "BGM009.ogg",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub(super) enum Button {
    Dance,
    Schedule,
    MaidManagement,
    EventScenario,
    Shop,
    FacilityManagement,
    CompetitiveShow,
    Casino,
    Save,
    Load,
    Trophy,
    FreeMode,
    PhotoMode,
    DeskCustomize,
    ManEdit,
    LegacyMode,
    Karaoke,
    Credit,
    Next,
    MyRoomCustom,
}
use Button::*;

const BUTTONS: [Button; 20] = [
    Dance,
    Schedule,
    MaidManagement,
    EventScenario,
    Shop,
    FacilityManagement,
    CompetitiveShow,
    Casino,
    Save,
    Load,
    Trophy,
    FreeMode,
    PhotoMode,
    DeskCustomize,
    ManEdit,
    LegacyMode,
    Karaoke,
    Credit,
    Next,
    MyRoomCustom,
];

impl Button {
    fn visible(self, period: Period, karaoke: bool) -> bool {
        match self {
            LegacyMode => false, // Standalone without the linked Legacy plugin.
            Karaoke => karaoke,
            Dance | Casino => period == Period::Night,
            Schedule | CompetitiveShow => period == Period::Daytime,
            _ => true,
        }
    }
    pub fn label(self) -> Option<&'static str> {
        Some(match self {
            Schedule | Save | Load => return None,
            Next => "label",
            Dance => "label_dance",
            MaidManagement => "label_maidmanagement",
            EventScenario => "label_eventscenario",
            Shop => "label_shop",
            FacilityManagement => "label_facilitymanagement",
            CompetitiveShow => "label_competitiveshow",
            Casino => "label_casino",
            Trophy => "label_trophy",
            FreeMode => "label_free",
            PhotoMode => "label_photomode",
            DeskCustomize => "label_deskcustomize",
            MyRoomCustom => "label_myroom",
            ManEdit => "label_useredit",
            LegacyMode => "label_legacymode",
            Karaoke => "label_karaoke",
            Credit => "label_credit",
        })
    }
    pub fn stops_bgm(self) -> bool {
        self.label().is_some()
            && !matches!(
                self,
                FreeMode | Trophy | PhotoMode | DeskCustomize | MaidManagement | FacilityManagement
            )
    }
}

pub(super) type Bindings = BTreeMap<i64, Button>;
pub(super) fn bindings(data: &[u8]) -> Result<Bindings> {
    let mut r = Reader::new(data);
    r.seek(32)?;
    ensure!(
        r.count(16)? == BUTTONS.len(),
        "unexpected DailyCtrl button count"
    );
    let mut entries = Bindings::new();
    let mut seen = BTreeSet::new();
    for _ in BUTTONS {
        let kind = *BUTTONS
            .get(usize::try_from(r.i32()?)?)
            .context("DailyCtrl ButtonType")?;
        let ptr = r.pptr64()?;
        ensure!(
            ptr.file_id == 0
                && ptr.path_id != 0
                && seen.insert(kind)
                && entries.insert(ptr.path_id, kind).is_none(),
            "invalid DailyCtrl binding {kind:?}"
        );
    }
    Ok(entries)
}

#[derive(Default)]
pub(super) struct Menu {
    rename: Option<Rename>,
    pub panel_close: Option<u64>,
    pub pending_label: Option<&'static str>,
    pub pages: Vec<(String, String)>,
    pub page: usize,
    pub tutorial_first: usize,
    pub closing_help: Option<u64>,
}

const RENAME: &str = "/clubNameChangePanel/";
struct Rename {
    opened: u64,
    closing: Option<u64>,
    value: String,
    session: Option<TextInputSessionHandle>,
    selected: bool,
}

pub(super) fn money(amount: i64) -> String {
    let digits = amount.unsigned_abs().to_string();
    let mut out = String::new();
    if amount < 0 {
        out.push('-');
    }
    for (i, digit) in digits.bytes().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(char::from(digit));
    }
    out
}

pub(super) fn prepare(
    widgets: &mut Vec<Widget>,
    bindings: &Bindings,
    period: Period,
    karaoke: bool,
    casino: bool,
    state: &crate::tjs::TjsState,
) -> Result<()> {
    // UITable reflows the active children. Take its authored sibling positions
    // from the root UIWidgets, and move the entire button including its collider.
    let mut roots = BTreeMap::new();
    for w in widgets.iter() {
        if let Some(kind) = w.button.and_then(|button| bindings.get(&button)) {
            ensure!(
                w.collider.is_some(),
                "missing DailyCtrl collider for {kind:?}"
            );
            roots.insert(w.path.clone(), (*kind, w.rect, [0.; 2]));
        }
    }
    ensure!(
        roots.len() == BUTTONS.len(),
        "resolved {} DailyCtrl roots",
        roots.len()
    );
    for (table, axis) in [("/VerticalLine1/", 1), ("/Horizon1/", 0)] {
        let mut children: Vec<_> = roots
            .iter()
            .filter(|(p, _)| p.contains(table))
            .map(|(p, (kind, rect, _))| (p.clone(), *kind, rect[axis]))
            .collect();
        children.sort_by(|a, b| a.2.total_cmp(&b.2));
        let active: Vec<_> = children
            .iter()
            .filter(|(_, k, _)| k.visible(period, karaoke))
            .collect();
        let start = if axis == 0 {
            children.len() - active.len()
        } else {
            0
        };
        for (i, (path, _, pos)) in active.into_iter().enumerate() {
            roots.get_mut(path).unwrap().2[axis] = children[start + i].2 - pos;
        }
    }
    widgets.retain_mut(|w| {
        let root = std::iter::successors(Some(w.path.as_str()), |p| {
            p.rsplit_once('/').map(|(parent, _)| parent)
        })
        .find_map(|p| roots.get(p));
        if let Some((kind, _, offset)) = root {
            if !kind.visible(period, karaoke) {
                return false;
            }
            for axis in 0..2 {
                w.rect[axis] += offset[axis];
                if let Some(c) = &mut w.collider {
                    c[axis] += offset[axis];
                }
            }
            if *kind == Casino && !casino && w.button.is_some() {
                w.collider = None;
                w.tint = [128, 128, 128, 255];
            }
        }
        let Some(path) = w.path.split_once("/StatusViewer/").map(|(_, s)| s) else {
            return true;
        };
        if let Some(i) = path
            .strip_prefix("ClubGrade/symbol/Image/")
            .and_then(|s| s.parse::<i32>().ok())
        {
            return i <= state.player.current_salon_grade;
        }
        if let Some(i) = path
            .strip_prefix("ClubＧauge/symbo/Image/")
            .and_then(|s| s.parse::<i32>().ok())
        {
            let fill = ((state.club_gauge - (i - 1) * 20) as f32 / 20.).clamp(0., 1.);
            if fill == 0. {
                return false;
            }
            w.rect[2] *= fill;
            w.uv[2] = w.uv[0] + (w.uv[2] - w.uv[0]) * fill;
        }
        if let Some(l) = &mut w.label {
            let value = match path {
                "ClubName/Value" => state
                    .retail
                    .club_name
                    .as_deref()
                    .unwrap_or("エンパイアクラブ")
                    .to_owned(),
                "Evaluation/Value" => state.player.salon_evaluation.to_string(),
                "NumberOfMaids/Value" => state.stock.len().to_string(),
                "Days/Value" => state.player.days.to_string(),
                "Money/Value" => money(state.player.money),
                "MainBusiness/Value" => state
                    .retail
                    .business_name
                    .as_deref()
                    .unwrap_or("-")
                    .to_owned(),
                _ => return true,
            };
            l.set_text(&value);
        }
        true
    });
    Ok(())
}

impl Menu {
    pub fn hit(&self, widgets: &[Widget], input: &InputFrame) -> Option<usize> {
        let modal = !self.pages.is_empty();
        widgets.iter().enumerate().rev().find_map(|(i, w)| {
            if modal != (i >= self.tutorial_first) {
                return None;
            }
            if !modal
                && (w.path.contains(RENAME) != self.rename.is_some()
                    || (w.button.is_none() && !w.path.ends_with("/InputField")))
            {
                return None;
            }
            let [x, y, width, height] = w.collider?;
            let (px, py) = (input.pointer_x as f32, input.pointer_y as f32);
            (px >= x && px < x + width && py >= y && py < y + height).then_some(i)
        })
    }
}

impl Menu {
    pub fn rename_alpha(&self, path: &str, now: u64) -> Option<u8> {
        if !path.contains(RENAME) {
            return None;
        }
        Some(self.rename.as_ref().map_or(0, |r| {
            let alpha = r
                .closing
                .map_or(now.saturating_sub(r.opened) as f32 / 300., |t| {
                    1. - now.saturating_sub(t) as f32 / 300.
                });
            (255. * alpha.clamp(0., 1.)).round() as u8
        }))
    }
}

impl Game {
    pub(super) fn update_daily(
        &mut self,
        period: Period,
        op: &mut Opener,
        host: &mut dyn HostServices,
        client: ClientId,
        input: InputFrame,
        released: u64,
        now: u64,
    ) -> Result<()> {
        if self.slot_viewer.is_some() { return self.update_slots(op,host,client,input,released,now); }
        if self.business.in_schedule() { return self.update_schedule(op,host,client,input,released,now); }
        if let Some(start) = self.daily.panel_close
            && self.leaving.is_none()
        {
            if now.saturating_sub(start) >= 250 {
                self.sound.stop(host, client, Kind::Bgm, 1.)?;
                self.leaving = Some(now);
            }
            return Ok(());
        }
        if let Some(start) = self.leaving {
            if now.saturating_sub(start) >= 500 {
                let key = self
                    .daily
                    .pending_label
                    .take()
                    .context("missing DailyAPI jump")?;
                self.resume_at(key, op, host, client, now)?;
            }
            return Ok(());
        }
        if let Some(start) = self.daily.closing_help {
            if now.saturating_sub(start) >= 500 {
                self.widgets.truncate(self.daily.tutorial_first);
                self.daily.pages.clear();
                self.daily.closing_help = None;
            }
            return Ok(());
        }
        if now.saturating_sub(self.entered) < 500 {
            return Ok(());
        }
        if let Some(rename) = self.daily.rename.as_mut() {
            if let Some(start) = rename.closing {
                if now.saturating_sub(start) >= 300 {
                    self.daily.rename = None;
                }
                return Ok(());
            }
            if now.saturating_sub(rename.opened) >= 300 && rename.session.is_none() {
                let field = self
                    .widgets
                    .iter()
                    .find(|w| w.path.contains(RENAME) && w.path.ends_with("/InputField"))
                    .context("missing club name input")?;
                rename.session = Some(host.open_text_input_session(
                    client,
                    TextInputSessionSpec {
                        capture: true,
                        cursor_area: Some(field.rect.map(|v| v as i32)),
                        ..Default::default()
                    },
                )?);
            }
            if let Some(h) = rename.session {
                let text = host.take_text_input_committed(client, h)?;
                let back = input.buttons_pressed & buttons::BACKSPACE != 0;
                if back || !text.is_empty() {
                    if rename.selected {
                        rename.value.clear();
                    } else if back && let Some((i, _)) = rename.value.grapheme_indices(true).last()
                    {
                        rename.value.truncate(i);
                    }
                    rename.selected = false;
                    rename.value.push_str(&text);
                    let filtered: String = rename
                        .value
                        .chars()
                        .filter(|c| !matches!(c, '\r' | '\n' | '\t' | '\\'))
                        .collect();
                    rename.value = if filtered.is_empty() {
                        " ".into()
                    } else {
                        filtered.graphemes(true).take(15).collect()
                    };
                }
            }
            let label = self
                .widgets
                .iter_mut()
                .find(|w| w.path.ends_with("/InputField/Label") && w.path.contains(RENAME))
                .and_then(|w| w.label.as_mut())
                .context("missing club name input label")?;
            label.set_text(&rename.value);
        }
        let hit = self.daily.hit(&self.widgets, &input);
        if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
            self.pressed = hit;
        }
        if released & buttons::MOUSE_LEFT == 0 || self.pressed.take() != hit {
            return Ok(());
        }
        let Some(i) = hit else {
            return Ok(());
        };
        let w = &self.widgets[i];
        if !self.daily.pages.is_empty() {
            if w.path.ends_with("/Button OK") {
                self.daily.closing_help = Some(now);
            } else if let Some((_, tail)) = w.path.split_once("/page-") {
                let page = tail.split('/').next().unwrap_or("").parse::<usize>()?;
                if page != self.daily.page {
                    self.widgets.truncate(self.daily.tutorial_first);
                    self.assets.tutorial(
                        op,
                        host,
                        client,
                        &self.daily.pages,
                        page,
                        &self.files,
                        &mut self.widgets,
                    )?;
                    self.daily.page = page;
                }
            }
            return Ok(());
        }
        if let Some(rename) = self.daily.rename.as_mut() {
            let ok = w.path.ends_with("/Base/Ok");
            if ok || w.path.ends_with("/Base/Cancel") {
                if let Some(h) = rename.session.take() {
                    host.release_text_input_session(client, h)?;
                }
                if ok {
                    self.script
                        .tjs
                        .lock()
                        .map_err(|_| anyhow::anyhow!("TJS lock"))?
                        .runtime
                        .host_mut()
                        .state
                        .retail
                        .club_name = Some(rename.value.clone());
                    let label = self
                        .widgets
                        .iter_mut()
                        .find(|w| w.path.ends_with("/ClubName/Value"))
                        .and_then(|w| w.label.as_mut())
                        .context("missing club name label")?;
                    label.set_text(&rename.value);
                }
                rename.closing = Some(now);
            }
            return Ok(());
        }
        if w.path.ends_with("/ClubName/ChangeBtn") {
            let vm = self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?;
            let value = vm
                .runtime
                .host()
                .state
                .retail
                .club_name
                .as_deref()
                .unwrap_or("エンパイアクラブ")
                .to_owned();
            self.daily.rename = Some(Rename {
                opened: now,
                closing: None,
                value,
                session: None,
                selected: true,
            });
            return Ok(());
        }
        let kind = *w
            .button
            .and_then(|b| self.assets.daily_buttons.get(&b))
            .context("unbound DailyCtrl button")?;
        if matches!(kind, Save | Load) {
            let mode=if kind==Save {slots::Mode::DailySave} else {slots::Mode::DailyLoad};
            return self.open_slots(mode,op,host,client,now);
        }
        if kind == Schedule { return self.begin_schedule(op,host,client,now); }
        let label = kind.label().with_context(|| {
            format!("COM3D2 SceneDaily {kind:?} local panel is not implemented")
        })?;
        ensure!(
            self.script.tag_backup.contains_key(label),
            "SceneDaily SceneCall lacks {label}"
        );
        if kind == Next {
            let mut vm = self
                .script
                .tjs
                .lock()
                .map_err(|_| anyhow::anyhow!("TJS lock"))?;
            crate::tjs::retail::close_daily(
                &mut vm.runtime.host_mut().state,
                period == Period::Daytime,
            )?;
            self.daily.pending_label = Some(label);
            self.daily.panel_close = Some(now);
            return Ok(());
        }
        if kind.stops_bgm() {
            self.sound.stop(host, client, Kind::Bgm, 1.)?;
        }
        self.daily.pending_label = Some(label);
        self.leaving = Some(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn daily_native_binding_order_and_invalid_records() -> Result<()> {
        let mut bytes = vec![0; 32];
        bytes.extend(20_i32.to_le_bytes());
        for i in (0..20_i32).rev() {
            bytes.extend(i.to_le_bytes());
            bytes.extend(0_i32.to_le_bytes());
            bytes.extend((3100_i64 + i64::from(i)).to_le_bytes());
        }
        let table = bindings(&bytes)?;
        assert_eq!(table[&3119], MyRoomCustom);
        assert_eq!(table[&3101].label(), None);
        assert_eq!(table[&3118].label(), Some("label"));
        bytes[36..40].copy_from_slice(&18_i32.to_le_bytes());
        assert!(bindings(&bytes).is_err());
        Ok(())
    }
    #[test]
    fn daily_visibility_and_scene_returns() {
        assert_eq!(BUTTONS.iter().filter(|k| k.label().is_some()).count(), 17);
        assert!(!Casino.visible(Period::Daytime, true) && Casino.visible(Period::Night, true));
        assert!(!Schedule.visible(Period::Night, true) && Schedule.visible(Period::Daytime, true));
        assert!(
            !Karaoke.visible(Period::Daytime, false) && !LegacyMode.visible(Period::Daytime, true)
        );
        assert_eq!(MyRoomCustom.label(), Some("label_myroom"));
        assert!(!MaidManagement.stops_bgm() && Shop.stops_bgm() && !FreeMode.stops_bgm());
        assert_eq!(money(-123456789), "-123,456,789");
    }
}

#[cfg(test)]
pub(super) fn installed_panel(extra: &[&str]) -> Result<(tempfile::TempDir,crysirror_runtime::Runtime,ClientId,Opener,std::sync::Arc<GameFiles>,crate::tjs::TjsState,Assets)> {
    use crysirror_core::path::{Mount, VfsPath};
    use crysirror_resource::{ArchiveMount, DecodeContext, FormatRegistry};
    use std::sync::Arc;

    let root = std::path::PathBuf::from(std::env::var_os("COM3D2_ROOT").context("COM3D2_ROOT")?);
    let tmp = tempfile::tempdir()?;
    let client = ClientId(1);
    // Use the ordinary host resource API without creating the COM3D2 VM or stepping a frame.
    let project = serde_json::from_value(
        json!({"schema":2,"name":"daily assets","content_revision":"1",
        "canvas":[1920,1080],"assets":"assets","clients":[{"name":"test","adapter":"crysirror-script-v1","game":"fixture"}]}),
    )?;
    let mut rt = crysirror_runtime::Runtime::with_asset_root(
        project,
        &root,
        tmp.path(),
        Default::default(),
    )?;
    let mut reg = FormatRegistry::default();
    crysirror_formats_arc::register(&mut reg)?;
    let archives: Vec<_> = ["csv", "system", "system2", "script"].into_iter().chain(extra.iter().copied()).map(str::to_owned).collect();
    for name in &archives {
        let path = VfsPath::new(Mount::Asset, &format!("GameData/{name}.arc"))?;
        let source = rt.host.storage(client)?.open_asset(&path)?;
        let cx = DecodeContext::new(client);
        let (_, archive) = reg.open(source, Some("kiss/warc"), &cx)?;
        rt.mount_provider(client, name, Arc::new(ArchiveMount::new(archive, cx)?))?;
    }
    let mut op = Opener {
        storage: rt.host.storage(client)?,
        dir: "COM3D2x64_Data".into(),
    };
    let files = Arc::new(GameFiles::index(op.storage.as_ref(), &archives)?);
    let mut state = crate::tjs::TjsState::default();
    state.player.schedule_slot.resize_with(40, Default::default);
    state
        .retail
        .attach(files.clone(), op.storage.clone(), &op.dir)?;
    let assets = Assets::load(&mut op)?;
    Ok((tmp,rt,client,op,files,state,assets))
}

#[test]
#[ignore = "requires COM3D2_ROOT standalone installation; reads assets only"]
fn daily_installed_widgets_scripts_and_tutorial() -> Result<()> {
    let (_tmp,mut rt,client,mut op,files,mut state,mut assets)=installed_panel(&[])?;
    let source = assets.scene(&mut op, &mut rt.host, client, "SceneDaily")?;
    let center = |w: &Widget| {
        let [x, y, width, height] = w.collider.unwrap();
        InputFrame {
            pointer_x: (x + width / 2.) as i32,
            pointer_y: (y + height / 2.) as i32,
            ..Default::default()
        }
    };
    crate::tjs::retail::start_before_hiring(&mut state)?;
    assert_eq!(state.retail.business_name.as_deref(), Some("アイドル系"));
    state.club_gauge = 30;
    for (period, file, page_count) in [
        (Period::Daytime, "daytimemain.ks", 7),
        (Period::Night, "nightmain.ks", 3),
    ] {
        let casino = crate::tjs::retail::daily_status(&mut state, period == Period::Night)?;
        assert!(!casino);
        let mut widgets = source.clone();
        let karaoke =
            (1..=100).any(|i| files.exists(&format!("karaoke_music_enable_list_{i:03}.nei")));
        prepare(
            &mut widgets,
            &assets.daily_buttons,
            period,
            karaoke,
            casino,
            &state,
        )?;
        let gauge = |w: &Widget| w.path.contains("ClubＧauge/symbo/Image/");
        assert_eq!(widgets.iter().filter(|w| gauge(w)).count(), 2);
        let second = |w: &Widget| gauge(w) && w.path.ends_with("/2");
        assert_eq!(
            widgets.iter().find(|w| second(w)).unwrap().rect[2],
            source.iter().find(|w| second(w)).unwrap().rect[2] * 0.5
        );
        let ks = files.read(op.storage.as_ref(), file)?;
        let scenario = encoding_rs::SHIFT_JIS.decode(&ks).0;
        let call = scenario
            .lines()
            .find(|l| l.starts_with("@SceneCall name=SceneDaily "))
            .context("missing Daily script call")?;
        let mut menu = Menu {
            tutorial_first: widgets.len(),
            ..Default::default()
        };
        for button in BUTTONS {
            let w = widgets
                .iter()
                .find(|w| w.button.and_then(|b| assets.daily_buttons.get(&b)) == Some(&button));
            assert_eq!(w.is_some(), button.visible(period, karaoke), "{button:?}");
            let Some(w) = w else { continue };
            if button == Casino {
                assert!(w.collider.is_none());
            } else {
                let i = menu
                    .hit(&widgets, &center(w))
                    .with_context(|| format!("{period:?} {button:?}"))?;
                assert_eq!(widgets[i].button, w.button);
            }
            if let Some(key) = button.label() {
                let target = call
                    .split_whitespace()
                    .find_map(|a| a.split_once('=').filter(|(k, _)| *k == key).map(|(_, v)| v))
                    .with_context(|| format!("{file}: {key}"))?;
                assert!(
                    scenario
                        .lines()
                        .any(|l| l.split_whitespace().next() == Some(target)),
                    "{file}: {target}"
                );
            }
        }
        menu.rename = Some(Rename {
            opened: 0,
            closing: None,
            value: String::new(),
            session: None,
            selected: true,
        });
        for suffix in ["/Base/Ok", "/Base/Cancel", "/InputField"] {
            let field = widgets
                .iter()
                .find(|w| w.path.contains(RENAME) && w.path.ends_with(suffix))
                .unwrap();
            let hit = menu
                .hit(&widgets, &center(field))
                .context("club name modal collider")?;
            assert_eq!(widgets[hit].path, field.path);
        }
        let next = center(widgets.iter().find(|w| w.path.ends_with("/Next")).unwrap());
        assert!(menu.hit(&widgets, &next).is_none());
        menu.rename = None;
        let table = state.retail.table("tutorial_list.nei")?;
        let row = (1..table.rows)
            .find(|&r| table.string(0, r) == period.tutorial())
            .unwrap();
        menu.pages = (2..table.columns)
            .step_by(2)
            .filter(|&c| table.exists(c, row))
            .map(|c| (table.string(c, row), table.string(c + 1, row)))
            .collect();
        assert_eq!(menu.pages.len(), page_count);
        let next = center(widgets.iter().find(|w| w.path.ends_with("/Next")).unwrap());
        for page in 0..page_count {
            widgets.truncate(menu.tutorial_first);
            assets.tutorial(
                &mut op,
                &mut rt.host,
                client,
                &menu.pages,
                page,
                &files,
                &mut widgets,
            )?;
            let controls = &widgets[menu.tutorial_first..];
            assert_eq!(
                controls.iter().filter(|w| w.collider.is_some()).count(),
                page_count + 1
            );
            for w in controls.iter().filter(|w| w.collider.is_some()) {
                let hit = menu.hit(&widgets, &center(w)).unwrap();
                assert_eq!(widgets[hit].path, w.path);
            }
            assert!(
                menu.hit(&widgets, &next)
                    .is_none_or(|hit| hit >= menu.tutorial_first)
            );
            assert!(controls.iter().any(|w| {
                w.path
                    .ends_with(&format!("/page-{page}/Background/Checkmark"))
            }));
            assert!(
                controls
                    .iter()
                    .any(|w| w.path.ends_with("/Content Image") && !w.asset.is_empty())
            );
        }
    }
    Ok(())
}
