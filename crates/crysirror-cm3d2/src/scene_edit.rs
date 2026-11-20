// SPDX-License-Identifier: MPL-2.0
//! `SceneEdit` (maid creation/edit): the menu catalogue built from every
//! `.menu` (`InitMenu`, `GetMenuItemSetUP`), category → parts type → item /
//! slider / colour panels of `level4`'s `UI Root`, item and slider
//! callbacks on maid 0, and the OK → name call → confirmation → thumbnail →
//! end-of-scene path.
//!
//! The `SceneEditInfo` tables (parts types, poses, voices, backgrounds,
//! camera targets, wear modes) are evaluated from the installed
//! `Assembly-CSharp.dll` (`crysirror-dotnet`), not copied here.
use crate::adapter::{Game, Opener};
use anyhow::{Context, Result};
use crysirror_dotnet::eval::{Statics, Value};
use std::collections::BTreeMap;

const ROOT: &str = "UI Root";
const PANELS: [&str; 6] = ["ScrollPanel-Category", "ScrollPanel-PartsType", "ScrollPanel-MenuItem", "ScrollPanel-SetItem", "ScrollPanel-Slider", "ScrollPanel-ColorSet"];
const SYSDLG: &str = "__GameMain__/SystemUI Root/SystemDialog";

/// `SceneEditInfo.CCateNameType`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CateNameType {
    pub idx: i64,
    /// `EType`: 0 Item, 1 Set, 2 Slider, 3 Color.
    pub etype: i64,
    pub menu_cate: i64,
    pub parts_type: i64,
    pub btn_name: String,
}

/// `SceneEditInfo.PoseVoiceBgInfo`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Pvb {
    pub file: String,
    pub file2: String,
    pub file3: String,
    pub icon: String,
    pub bone: String,
    pub auto_twist_shoulder: bool,
    pub pos: [f32; 3],
    pub rot: f32,
    pub stage: i64,
    pub is_stage: bool,
}

/// The `SceneEditInfo` statics this controller uses.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Tables {
    pub parts: BTreeMap<i64, CateNameType>,
    pub slider_names: BTreeMap<i64, String>,
    pub wear_mode: BTreeMap<i64, String>,
    pub camera: BTreeMap<i64, (String, f32, [f32; 2])>,
    pub pose: BTreeMap<i64, Vec<Pvb>>,
    pub voice: BTreeMap<i64, Vec<Pvb>>,
    pub bg: Vec<Pvb>,
    pub categories: BTreeMap<i64, String>,
}

fn int(v: Option<&Value>) -> i64 {
    v.and_then(Value::int).unwrap_or(0)
}
fn text(v: Option<&Value>) -> String {
    v.and_then(Value::str).unwrap_or("").to_owned()
}
fn vec_args(st: &Statics, v: Option<&Value>) -> Vec<f32> {
    v.and_then(|v| st.args(v)).map(|a| a.iter().map(|x| x.float().unwrap_or(0.) as f32).collect()).unwrap_or_default()
}

impl Tables {
    pub fn load(asm: &crysirror_dotnet::Assembly) -> Result<Self> {
        let st = asm.eval_cctor("SceneEditInfo")?;
        let field = |n: &str| st.fields.get(n).with_context(|| format!("SceneEditInfo.{n}"));
        let mask = asm.enum_names("TBody/MaskMode")?;
        let mut t = Tables { categories: asm.enum_names("SceneEditInfo/EMenuCategory")?, ..Default::default() };
        for (k, v) in st.dict(field("m_dicPartsTypePair")?).context("m_dicPartsTypePair")? {
            t.parts.insert(
                k.int().unwrap_or(0),
                CateNameType {
                    idx: int(st.field(v, "m_nIdx")),
                    etype: int(st.field(v, "m_eType")),
                    menu_cate: int(st.field(v, "m_eMenuCate")),
                    parts_type: int(st.field(v, "m_ePartsType")),
                    btn_name: text(st.field(v, "m_strBtnPartsTypeName")),
                },
            );
        }
        for (k, v) in st.dict(field("m_dicSliderPartsTypeBtnName")?).context("slider names")? {
            t.slider_names.insert(k.int().unwrap_or(0), v.str().unwrap_or("").to_owned());
        }
        for (k, v) in st.dict(field("m_dicPartsTypeWearMode")?).context("wear modes")? {
            t.wear_mode.insert(k.int().unwrap_or(0), mask.get(&v.int().unwrap_or(0)).cloned().unwrap_or_default());
        }
        for (k, v) in st.dict(field("m_dicPartsTypeCamera")?).context("cameras")? {
            let a = vec_args(&st, st.field(v, "angle"));
            t.camera.insert(k.int().unwrap_or(0), (text(st.field(v, "bone")), st.field(v, "distance").and_then(Value::float).unwrap_or(3.) as f32, [a.first().copied().unwrap_or(0.), a.get(1).copied().unwrap_or(0.)]));
        }
        let pvb = |v: &Value| {
            let p = vec_args(&st, st.field(v, "vPos"));
            Pvb {
                file: text(st.field(v, "strFileName")),
                file2: text(st.field(v, "strFileName2")),
                file3: text(st.field(v, "strFileName3")),
                icon: text(st.field(v, "strIconFileName")),
                bone: text(st.field(v, "bone")),
                auto_twist_shoulder: int(st.field(v, "bAutoTwistShoulder")) != 0,
                pos: [p.first().copied().unwrap_or(0.), p.get(1).copied().unwrap_or(0.), p.get(2).copied().unwrap_or(0.)],
                rot: st.field(v, "fRot").and_then(Value::float).unwrap_or(0.) as f32,
                stage: int(st.field(v, "stage")),
                is_stage: int(st.field(v, "bStage")) != 0,
            }
        };
        for (name, out) in [("m_dicPose", &mut t.pose), ("m_dicVoice", &mut t.voice)] {
            for (k, v) in st.dict(field(name)?).with_context(|| name.to_owned())? {
                out.insert(k.int().unwrap_or(0), st.items(v).unwrap_or(&[]).iter().map(pvb).collect());
            }
        }
        t.bg = st.items(field("m_listBg")?).context("m_listBg")?.iter().map(pvb).collect();
        Ok(t)
    }
}

/// `SceneEdit.SMenuItem`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MenuItem {
    pub file: String,
    pub cate_name: String,
    pub mpn: Option<usize>,
    pub name: String,
    pub sub_name: String,
    pub info: String,
    pub color_set_mpn: Option<usize>,
    pub color_name: Option<String>,
    pub color_list: Vec<usize>,
    pub multicolor: Option<usize>,
    pub priority: f32,
    pub man: bool,
    pub del_only: bool,
    pub icon: Option<String>,
    pub is_color: bool,
    pub parts_type: Option<(usize, usize)>,
}

/// `SceneEdit.InitMenuItemScript` over the menu's commands.
fn menu_item(file: &str, menu: &crate::menu::MenuFile) -> MenuItem {
    let mut mi = MenuItem { file: file.to_owned(), ..Default::default() };
    for c in &menu.commands {
        let arg = |i: usize| c.get(i).map(String::as_str).unwrap_or("");
        match c[0].as_str() {
            "name" => {
                let t = arg(1);
                let cut = t.find(['\u{3000}', ' ']).unwrap_or(t.len());
                mi.name = t[..cut].to_owned();
                mi.sub_name = t[cut..].to_owned();
            }
            "setumei" => mi.info = arg(1).replace("《改行》", "\n"),
            "category" => {
                mi.cate_name = arg(1).to_lowercase();
                mi.mpn = crate::chara::mpn(&mi.cate_name);
            }
            "color_set" => {
                mi.color_set_mpn = crate::chara::mpn(&arg(1).to_lowercase());
                if c.len() >= 3 {
                    mi.color_name = Some(arg(2).to_lowercase());
                }
            }
            "tex" | "テクスチャ変更" if c.len() == 6 => mi.multicolor = crate::parts_color::id(arg(5)),
            "icon" | "icons" => mi.icon = Some(arg(1).to_owned()),
            "delitem" | "unsetitem" => mi.del_only = true,
            "priority" => mi.priority = arg(1).trim().parse().unwrap_or(0.),
            "メニューフォルダ" if arg(1).eq_ignore_ascii_case("man") => mi.man = true,
            _ => {}
        }
    }
    mi
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartsType {
    pub etype: i64,
    pub idx: i64,
    pub name: String,
    pub mpn: usize,
    pub parts_type: i64,
    pub menus: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Category {
    pub ecat: i64,
    pub name: String,
    pub parts: Vec<PartsType>,
}

/// What a button of the panels refers to.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Button {
    Category(usize),
    Parts(usize, usize),
    Item(usize),
    Color(usize),
    Slider(usize),
}

/// The edit scene's state between frames.
#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    tables: Tables,
    items: Vec<MenuItem>,
    sliders: Vec<(usize, String)>,
    cats: Vec<Category>,
    buttons: BTreeMap<usize, Button>,
    /// Item buttons of the open parts type: node → item, with its frame.
    item_frames: Vec<(usize, usize, Option<usize>)>,
    color_frames: Vec<(usize, usize, Option<usize>)>,
    script_label: Option<String>,
    first: bool,
    load_wait: bool,
    /// The OK path: confirmation shown, ThumShot frames, ending fade.
    dialog: Option<&'static str>,
    put_cloth: Option<u64>,
    ending: bool,
    auto_cam: bool,
    eye_to_cam: bool,
    cam_reset: ([f32; 3], f32, [f32; 2]),
}

impl Game {
    /// The installed `Assembly-CSharp.dll`, parsed once.
    pub fn managed(&mut self, opener: &mut Opener) -> Result<std::sync::Arc<crysirror_dotnet::Assembly>> {
        if let Some(a) = &self.managed_asm {
            return Ok(a.clone());
        }
        use crate::unity_scene::UnityFiles;
        let reader = opener.open("Managed/Assembly-CSharp.dll")?;
        let len = reader.len()? as usize;
        anyhow::ensure!(len <= crysirror_dotnet::MAX_ASSEMBLY_BYTES, "Assembly-CSharp.dll too large");
        let mut bytes = vec![0u8; len];
        let mut at = 0;
        while at < len {
            let n = reader.read_at(at as u64, &mut bytes[at..])?;
            anyhow::ensure!(n > 0, "short read of Assembly-CSharp.dll");
            at += n;
        }
        let asm = std::sync::Arc::new(crysirror_dotnet::Assembly::parse(bytes)?);
        self.managed_asm = Some(asm.clone());
        Ok(asm)
    }
}

fn level_node(game: &Game, path: &str) -> Result<usize> {
    game.level.as_ref().and_then(|l| l.scene.find(path)).with_context(|| format!("SceneEdit node {path}"))
}

/// `EventDelegate.Add(button.onClick, handler)` of an `Awake`.
fn set_click(game: &mut Game, path: &str, handler: &str) -> Result<()> {
    let n = level_node(game, path)?;
    game.level.as_mut().unwrap().states.entry(n).or_default().click = Some(handler.into());
    Ok(())
}

fn set_active(game: &mut Game, path: &str, on: bool) -> Result<()> {
    let n = level_node(game, path)?;
    game.level.as_mut().unwrap().states.entry(n).or_default().active = Some(on);
    Ok(())
}

/// `SceneEdit.Start`.
pub fn start(game: &mut Game, opener: &mut Opener) -> Result<Edit> {
    let asm = game.managed(opener)?;
    let tables = Tables::load(&asm)?;
    // CharacterMgr.DeactivateCharaAll; maid 0 visible.
    for c in &mut game.characters {
        c.visible = false;
    }
    let tag = game.adv.tag_backup.clone();
    let (script_label, first) = if tag.get("name").map(String::as_str) == Some("SceneEdit") {
        (tag.get("label").cloned(), tag.contains_key("first"))
    } else {
        (None, false)
    };
    game.adv_ui(crate::kag::AdvUi::Window(false), opener)?;
    for p in ["Window-ItemInfo", "OkCancel", "ViewReset"] {
        set_active(game, &format!("{ROOT}/{p}"), true)?;
    }
    // EditOkCancel/EditViewReset.Awake delegates, SceneEdit.SetOnClickCallBack.
    set_click(game, &format!("{ROOT}/OkCancel/Ok"), "EditOkCancel.OnClickOk")?;
    for b in ["ViewReset", "AutoCam", "EyeToCam"] {
        set_click(game, &format!("{ROOT}/ViewReset/{b}"), &format!("EditViewReset.OnClick{b}"))?;
    }
    // EditOkCancel.Show(OK): no Cancel button.
    set_active(game, &format!("{ROOT}/OkCancel/Cancel"), false)?;
    set_active(game, &format!("{ROOT}/Window-ItemInfo"), false)?;
    for p in PANELS {
        set_active(game, &format!("{ROOT}/{p}"), p == "ScrollPanel-Category")?;
        set_active(game, &format!("{ROOT}/{p}/Arrow"), false)?;
    }
    set_active(game, &format!("{ROOT}/ViewCancel"), false)?;
    set_active(game, &format!("{ROOT}/FFNameDialog"), false)?;
    let personal = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?.runtime.host().state.maid(0).map_or(0, |m| m.personal) as i64;
    let mut edit = Edit {
        tables,
        items: Vec::new(),
        sliders: Vec::new(),
        cats: Vec::new(),
        buttons: BTreeMap::new(),
        item_frames: Vec::new(),
        color_frames: Vec::new(),
        script_label,
        first,
        load_wait: true,
        dialog: None,
        put_cloth: None,
        ending: false,
        auto_cam: true,
        eye_to_cam: true,
        cam_reset: ([-0.05539433, 0.95894, 0.1269088], 3., [-180., 11.5528]),
    };
    if let Some(m) = game.maid(0) {
        m.visible = true;
        m.eye = (crate::kag::EyeLook::Camera("目と顔を向ける".into()), 0.);
    }
    // Bg(m_listBg[0]) (the first entry is not a yotogi stage).
    let bg = edit.tables.bg.first().cloned().context("SceneEditInfo.m_listBg is empty")?;
    game.adv_ui(crate::kag::AdvUi::SetBg(bg.file.clone()), opener)?;
    game.light = ([40., 180., 18.], 0.95);
    game.camera_offset = 0.;
    game.camera = edit.cam_reset;
    if let Some(p) = edit.tables.pose.get(&personal).and_then(|v| v.first()).cloned() {
        pose(game, opener, &mut edit, &p)?;
    }
    init_menu(game, opener, &mut edit)?;
    update_panel_category(game, opener, &mut edit)?;
    game.sound_ops.push(crate::sound::SoundOp::Play { kind: crate::sound::Kind::Bgm, file: "BGM002.ogg".into(), fade_s: 5., looping: true });
    game.fade_to(1., 0.);
    Ok(edit)
}

/// `SceneEdit.InitMenu`.
fn init_menu(game: &mut Game, opener: &mut Opener, e: &mut Edit) -> Result<()> {
    let storage = opener.storage.clone();
    e.cats = [0i64, 1, 2, 3, 4, 5, 6, 7, 8]
        .iter()
        .map(|c| Category { ecat: *c, name: e.tables.categories.get(c).cloned().unwrap_or_default(), parts: Vec::new() })
        .collect();
    let mut colors: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let have = |name: &str| -> bool {
        game.adv.tjs.lock().ok().is_none_or(|t| t.runtime.host().state.player.have_item.get(&name.to_lowercase()).copied().unwrap_or(true))
    };
    let mut listed = Vec::new();
    for path in game.files.menu_files.clone() {
        let file = path.rsplit('/').next().unwrap_or(&path).to_owned();
        if path.to_lowercase().contains("/man") || !have(&file) || file.contains("_zurashi") || file.contains("_mekure") {
            continue;
        }
        let Ok(menu) = crate::menu::MenuFile::parse(&game.files.read(storage.as_ref(), &file)?) else { continue };
        let mi = menu_item(&file, &menu);
        if mi.icon.is_none() || mi.man {
            continue;
        }
        let i = e.items.len();
        e.items.push(mi);
        add_menu_item(e, i, &mut colors);
        listed.push(i);
    }
    // Slider parts types (m_nSliderValue 500, name = the MPN).
    for (mpn, c) in e.tables.parts.clone() {
        if c.etype == 2 {
            let i = e.items.len();
            e.items.push(MenuItem { cate_name: crate::chara::MPN[mpn as usize].to_owned(), mpn: Some(mpn as usize), name: c.btn_name.clone(), ..Default::default() });
            add_menu_item(e, i, &mut colors);
        }
    }
    for i in listed {
        let Some(cm) = e.items[i].color_set_mpn else { continue };
        let all = colors.get(&cm).cloned().unwrap_or_default();
        e.items[i].color_list = match &e.items[i].color_name {
            Some(pat) => {
                let pat = pat.replace('*', ".*");
                all.into_iter().filter(|j| glob_match(&pat, &e.items[*j].file)).collect()
            }
            None => all,
        };
    }
    // SortPartsType (m_nIdx) and SortItem (priority, then name; colours by priority).
    let items = e.items.clone();
    for c in &mut e.cats {
        c.parts.sort_by_key(|p| p.idx);
        for p in &mut c.parts {
            p.menus.sort_by(|a, b| items[*a].priority.total_cmp(&items[*b].priority).then_with(|| items[*a].name.cmp(&items[*b].name)));
        }
    }
    for i in 0..e.items.len() {
        let mut l = e.items[i].color_list.clone();
        l.sort_by(|a, b| items[*a].priority.total_cmp(&items[*b].priority));
        e.items[i].color_list = l;
    }
    Ok(())
}

/// The `Regex.IsMatch` of `m_strMenuNameInColorSet` (`*` → `.*`, unanchored).
fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split(".*").collect();
    let text = text.to_lowercase();
    let mut at = 0;
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() {
            continue;
        }
        let _ = i;
        match text[at..].find(p) {
            Some(k) => at += k + p.len(),
            None => return false,
        }
    }
    true
}

/// `SceneEdit.AddMenuItemToList`.
fn add_menu_item(e: &mut Edit, i: usize, colors: &mut BTreeMap<usize, Vec<usize>>) {
    let Some(mpn) = e.items[i].mpn.filter(|m| *m != 0) else { return };
    let Some(ct) = e.tables.parts.get(&(mpn as i64)).cloned() else { return };
    match ct.etype {
        0..=2 => {
            let Some(ci) = e.cats.iter().position(|c| c.ecat == ct.menu_cate) else { return };
            let pi = match e.cats[ci].parts.iter().position(|p| p.parts_type == ct.parts_type) {
                Some(pi) => pi,
                None => {
                    let name = if ct.etype == 2 { e.tables.slider_names.get(&ct.parts_type).cloned().unwrap_or_default() } else { ct.btn_name.clone() };
                    let idx = e.tables.parts.get(&(mpn as i64)).map_or(0, |c| c.idx);
                    e.cats[ci].parts.push(PartsType { etype: ct.etype, idx, name, mpn, parts_type: ct.parts_type, menus: Vec::new() });
                    e.cats[ci].parts.len() - 1
                }
            };
            e.items[i].parts_type = Some((ci, pi));
            e.cats[ci].parts[pi].menus.push(i);
        }
        3 => {
            e.items[i].is_color = true;
            colors.entry(mpn).or_default().push(i);
        }
        _ => {}
    }
}

/// `NGUITools.AddChild(grid, Resources.Load("SceneEdit/MainMenu/Prefab/" + name))`.
fn add_child(game: &mut Game, opener: &mut Opener, grid: usize, prefab: &str) -> Result<usize> {
    let go = game
        .unity
        .resource_of(opener, &format!("SceneEdit/MainMenu/Prefab/{prefab}"), crysirror_unity::class_id::GAME_OBJECT)?
        .with_context(|| format!("Resources SceneEdit/MainMenu/Prefab/{prefab}"))?;
    let p = game.unity.prefab(opener, &go)?;
    let level = game.level.as_mut().context("SceneEdit level")?;
    let node = level.scene.instantiate(&p, p.roots[0], Some(grid));
    level.scene.nodes[node].position = [0.; 3];
    level.scene.nodes[node].scale = [1.; 3];
    Ok(node)
}

/// The first descendant (or self) with a component of `class`.
fn with_script(game: &Game, node: usize, class: &str) -> Option<usize> {
    let scene = &game.level.as_ref()?.scene;
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if scene.script(n, class).is_some() {
            return Some(n);
        }
        stack.extend(scene.nodes[n].children.iter().rev());
    }
    None
}

fn clear_grid(game: &mut Game, e: &mut Edit, panel: &str) -> Result<()> {
    let grid = level_node(game, &format!("{ROOT}/{panel}/Scroll View/UIGrid"))?;
    let level = game.level.as_mut().unwrap();
    for c in std::mem::take(&mut level.scene.nodes[grid].children) {
        level.states.remove(&c);
        e.buttons.retain(|k, _| *k != c);
    }
    Ok(())
}

fn reposition(game: &mut Game, panel: &str) -> Result<()> {
    let grid = level_node(game, &format!("{ROOT}/{panel}/Scroll View/UIGrid"))?;
    let level = game.level.as_mut().unwrap();
    for (n, p) in crate::ngui::grid_positions(&level.scene, &level.states, grid) {
        level.states.entry(n).or_default().position = Some(p);
    }
    Ok(())
}

/// A `ButtonCate` with `label`, its UIButton wired to `ClickCallback`.
fn cate_button(game: &mut Game, opener: &mut Opener, e: &mut Edit, panel: &str, label: &str, b: Button) -> Result<()> {
    let grid = level_node(game, &format!("{ROOT}/{panel}/Scroll View/UIGrid"))?;
    let node = add_child(game, opener, grid, "ButtonCate")?;
    let lbl = with_script(game, node, "UILabel");
    let btn = with_script(game, node, "UIButton").context("ButtonCate has no UIButton")?;
    let level = game.level.as_mut().unwrap();
    if let Some(l) = lbl {
        level.states.entry(l).or_default().text = Some(label.to_owned());
    }
    level.states.entry(btn).or_default().click = Some("SceneEdit.ClickCallback".into());
    if let Some(frame) = level.scene.child(node, "Frame") {
        level.states.entry(frame).or_default().alpha = Some(0.);
    }
    e.buttons.insert(btn, b);
    Ok(())
}

/// `SceneEdit.UpdatePanel_Category`.
fn update_panel_category(game: &mut Game, opener: &mut Opener, e: &mut Edit) -> Result<()> {
    clear_grid(game, e, "ScrollPanel-Category")?;
    for (i, c) in e.cats.clone().iter().enumerate() {
        cate_button(game, opener, e, "ScrollPanel-Category", &c.name, Button::Category(i))?;
    }
    reposition(game, "ScrollPanel-Category")?;
    for p in &PANELS[1..] {
        set_active(game, &format!("{ROOT}/{p}"), false)?;
    }
    Ok(())
}

/// `SceneEdit.UpdatePanel_PartsType`.
fn update_panel_parts(game: &mut Game, opener: &mut Opener, e: &mut Edit, ci: usize) -> Result<()> {
    game.camera_offset = 0.;
    set_active(game, &format!("{ROOT}/Window-ItemInfo"), false)?;
    set_active(game, &format!("{ROOT}/ScrollPanel-PartsType"), true)?;
    set_active(game, &format!("{ROOT}/ScrollPanel-PartsType/Arrow"), false)?;
    for p in &PANELS[1..] {
        clear_grid(game, e, p)?;
    }
    for (pi, p) in e.cats[ci].parts.clone().iter().enumerate() {
        cate_button(game, opener, e, "ScrollPanel-PartsType", &p.name, Button::Parts(ci, pi))?;
    }
    reposition(game, "ScrollPanel-PartsType")?;
    Ok(())
}

/// `SceneEdit.UpdatePanel_MenuItem`.
fn update_panel_menu(game: &mut Game, opener: &mut Opener, e: &mut Edit, ci: usize, pi: usize) -> Result<()> {
    // SetCameraOffset(RIGHT): Screen.width / 3 · 2 − Screen.width / 2.
    game.camera_offset = crate::ngui::WIDTH / 6.;
    set_active(game, &format!("{ROOT}/Window-ItemInfo"), false)?;
    for p in &PANELS[2..] {
        clear_grid(game, e, p)?;
    }
    e.item_frames.clear();
    e.color_frames.clear();
    let pt = e.cats[ci].parts[pi].clone();
    match pt.etype {
        0 | 1 => {
            let (panel, prefab, frame) = if pt.etype == 0 { ("ScrollPanel-MenuItem", "ButtonItem", true) } else { ("ScrollPanel-SetItem", "ButtonSet", false) };
            for p in &PANELS[2..] {
                set_active(game, &format!("{ROOT}/{p}"), *p == panel)?;
            }
            let grid = level_node(game, &format!("{ROOT}/{panel}/Scroll View/UIGrid"))?;
            for i in pt.menus {
                let node = add_child(game, opener, grid, prefab)?;
                let btn = with_script(game, node, "UIButton").context("item button")?;
                let level = game.level.as_mut().unwrap();
                let fr = level.scene.child(node, "Frame");
                if let Some(f) = fr {
                    level.states.entry(f).or_default().active = Some(false);
                }
                level.states.entry(btn).or_default().click = Some("SceneEdit.ClickCallback".into());
                if let Some(icon) = &e.items[i].icon {
                    // gcBtn.normalSprite2D = Sprite.Create(icon).
                    let sprite = with_script(game, node, "UI2DSprite").unwrap_or(btn);
                    game.pending_textures.push((1, sprite, icon.clone()));
                }
                e.buttons.insert(btn, Button::Item(i));
                e.item_frames.push((btn, i, if frame { fr } else { None }));
            }
            reposition(game, panel)?;
            update_selected_menu_item(game, opener, e, pt.mpn)?;
        }
        2 => {
            for p in &PANELS[2..] {
                set_active(game, &format!("{ROOT}/{p}"), *p == "ScrollPanel-Slider")?;
            }
            let grid = level_node(game, &format!("{ROOT}/ScrollPanel-Slider/Scroll View/UIGrid"))?;
            e.sliders.clear();
            for i in pt.menus {
                let node = add_child(game, opener, grid, "Slider")?;
                let slider = with_script(game, node, "UISlider").context("Slider prefab")?;
                let level = game.level.as_mut().unwrap();
                if let Some(n) = level.scene.child(node, "CategoryTitle/Name") {
                    level.states.entry(n).or_default().text = Some(e.items[i].name.clone());
                }
                level.states.entry(slider).or_default().click = Some("SceneEdit.SlideCallback".into());
                e.buttons.insert(slider, Button::Slider(i));
                e.sliders.push((node, e.items[i].cate_name.clone()));
            }
            reposition(game, "ScrollPanel-Slider")?;
            update_sliders(game, e)?;
        }
        _ => {}
    }
    Ok(())
}

/// `SceneEdit.UpdateSelectedMenuItem`.
fn update_selected_menu_item(game: &mut Game, opener: &mut Opener, e: &mut Edit, mpn: usize) -> Result<()> {
    let Some(maid) = game.maid(0).cloned() else { return Ok(()) };
    let file = maid.props.get(mpn).map(|p| p.file.to_lowercase()).unwrap_or_default();
    let mut selected = None;
    for (btn, i, fr) in e.item_frames.clone() {
        let on = !file.is_empty() && e.items[i].file.to_lowercase() == file;
        if let Some(f) = fr {
            game.level.as_mut().unwrap().states.entry(f).or_default().active = Some(on);
        }
        if on {
            selected = Some(i);
        }
        let _ = btn;
    }
    if file.is_empty() {
        for (_, i, fr) in e.item_frames.clone() {
            let hit = e.items[i].color_list.iter().any(|c| e.items[*c].mpn.and_then(|m| maid.props.get(m)).is_some_and(|p| p.file.eq_ignore_ascii_case(&e.items[*c].file)));
            if hit {
                if let Some(f) = fr {
                    game.level.as_mut().unwrap().states.entry(f).or_default().active = Some(true);
                }
                selected = Some(i);
            }
        }
    }
    update_panel_color(game, opener, e, selected)
}

/// `SceneEdit.UpdatePanel_ColorSet`.
fn update_panel_color(game: &mut Game, opener: &mut Opener, e: &mut Edit, selected: Option<usize>) -> Result<()> {
    let Some(sel) = selected.filter(|s| e.items[*s].color_set_mpn.is_some()) else {
        return set_active(game, &format!("{ROOT}/ScrollPanel-ColorSet"), false);
    };
    set_active(game, &format!("{ROOT}/ScrollPanel-ColorSet"), true)?;
    clear_grid(game, e, "ScrollPanel-ColorSet")?;
    e.color_frames.clear();
    let grid = level_node(game, &format!("{ROOT}/ScrollPanel-ColorSet/Scroll View/UIGrid"))?;
    for c in e.items[sel].color_list.clone() {
        let node = add_child(game, opener, grid, "ButtonColor")?;
        let btn = with_script(game, node, "UIButton").context("colour button")?;
        let level = game.level.as_mut().unwrap();
        let fr = level.scene.child(node, "Frame");
        level.states.entry(btn).or_default().click = Some("SceneEdit.ClickCallback".into());
        if let Some(icon) = &e.items[c].icon {
            let sprite = with_script(game, node, "UI2DSprite").unwrap_or(btn);
            game.pending_textures.push((1, sprite, icon.clone()));
        }
        e.buttons.insert(btn, Button::Color(c));
        e.color_frames.push((btn, c, fr));
    }
    reposition(game, "ScrollPanel-ColorSet")?;
    let cm = e.items[sel].color_set_mpn.unwrap();
    update_selected_color(game, e, cm);
    Ok(())
}

/// `SceneEdit.UpdateSelected_ColorSet`.
fn update_selected_color(game: &mut Game, e: &Edit, mpn: usize) {
    let file = game.maid(0).and_then(|m| m.props.get(mpn)).map(|p| p.file.to_lowercase()).unwrap_or_default();
    if file.is_empty() {
        return;
    }
    for (_, c, fr) in &e.color_frames {
        if let Some(f) = fr {
            let on = e.items[*c].file.to_lowercase() == file;
            game.level.as_mut().unwrap().states.entry(*f).or_default().active = Some(on);
        }
    }
    // SATD-cm3d2-game-logic: ColorPaletteMgr.OpenColorPalette for 無限色 items.
}

/// `SceneEdit.UpdateSliders`: value (prop − min)/(max − min), label = value.
fn update_sliders(game: &mut Game, e: &Edit) -> Result<()> {
    let Some(maid) = game.maid(0).cloned() else { return Ok(()) };
    for (node, cate) in &e.sliders {
        let Some(p) = crate::chara::mpn(cate).and_then(|i| maid.props.get(i)) else { continue };
        let v = (p.value - p.min) as f32 / (p.max - p.min).max(1) as f32;
        let level = game.level.as_mut().unwrap();
        if let Some(s) = with_script_scene(&level.scene, *node, "UISlider") {
            level.states.entry(s).or_default().value = Some(v);
        }
        if let Some(l) = level.scene.child(*node, "CategoryTitle/Number/Value") {
            level.states.entry(l).or_default().text = Some(p.value.to_string());
        }
    }
    Ok(())
}

fn with_script_scene(scene: &crate::unity_scene::Scene, node: usize, class: &str) -> Option<usize> {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        if scene.script(n, class).is_some() {
            return Some(n);
        }
        stack.extend(scene.nodes[n].children.iter().rev());
    }
    None
}

/// `SceneEdit.Pose`.
fn pose(game: &mut Game, opener: &mut Opener, e: &mut Edit, p: &Pvb) -> Result<()> {
    let file = format!("{}.anm", p.file);
    let duration = game.anm_duration(opener, &file)?;
    let now = game.now;
    if let Some(m) = game.maid(0) {
        m.cross_fade(&file, true, false, 0., now, duration);
        for (k, on) in [("ShoulderL", p.auto_twist_shoulder), ("ShoulderR", p.auto_twist_shoulder), ("WristL", true), ("WristR", true), ("ThighL", true), ("ThighR", true)] {
            m.auto_twist.insert(k, on);
        }
        let sets = m.blend_sets.clone();
        m.face.anime(&p.file2, &sets);
        m.face.blend(if p.file3.is_empty() { "無し" } else { &p.file3 });
    }
    if e.auto_cam && p.bone.is_empty() {
        game.camera = e.cam_reset;
    }
    Ok(())
}

/// Per-frame update and NGUI events of the edit scene.
pub fn update(game: &mut Game, opener: &mut Opener, e: &mut Edit) -> Result<()> {
    if e.load_wait {
        // CoLoadWait: ResetCharaPosAll, then FadeIn(3).
        e.load_wait = false;
        game.adv_ui(crate::kag::AdvUi::AllResetPos, opener)?;
        game.fade_to(0., 3.);
    }
    if let Some(t) = e.put_cloth {
        // CoWaitPutCloth: 60 frames, SE022, ThumShot, FadeOut(1) → OnEndScene.
        if game.now >= t + 1000 && !e.ending {
            e.ending = true;
            game.sound_ops.push(crate::sound::SoundOp::Play { kind: crate::sound::Kind::Se, file: "SE022.ogg".into(), fade_s: 0., looping: false });
            game.fade_to(1., 1.);
            e.put_cloth = Some(game.now);
        } else if e.ending && game.now >= t + 1000 {
            end_scene(game, opener, e)?;
            e.put_cloth = None;
        }
    }
    for (si, node, handler) in std::mem::take(&mut game.events) {
        match handler.as_str() {
            "SceneEdit.ClickCallback" if si == 1 => click(game, opener, e, node)?,
            "SceneEdit.SlideCallback" if si == 1 => slide(game, opener, e, node)?,
            "EditOkCancel.OnClickOk" | "SceneEdit.OnEditOk" => on_edit_ok(game, e)?,
            "FFNameDialog.OnClickA" => {
                set_active(game, &format!("{ROOT}/FFNameDialog"), false)?;
                first_name_call(game, false)?;
                confirm(game)?;
                e.dialog = Some("confirm");
            }
            "FFNameDialog.OnClickB" => {
                set_active(game, &format!("{ROOT}/FFNameDialog"), false)?;
                first_name_call(game, true)?;
                confirm(game)?;
                e.dialog = Some("confirm");
            }
            "SystemDialog.OnClickOk" if e.dialog == Some("confirm") => {
                game.set_main_active(SYSDLG, false)?;
                e.dialog = None;
                // OnEndDlgOk: ProfileMgr.UpdateProfileData, ThumShot camera,
                // then CoWaitPutCloth.
                game.update_profile_data(0)?;
                e.put_cloth = Some(game.now);
            }
            "SystemDialog.OnClickCancel" => {
                game.set_main_active(SYSDLG, false)?;
                e.dialog = None;
            }
            "EditViewReset.OnClickViewReset" => game.camera = e.cam_reset,
            "EditViewReset.OnClickAutoCam" => e.auto_cam = !e.auto_cam,
            "EditViewReset.OnClickEyeToCam" => {
                e.eye_to_cam = !e.eye_to_cam;
                let look = if e.eye_to_cam { "目と顔を向ける" } else { "無視する" };
                if let Some(m) = game.maid(0) {
                    m.eye = (crate::kag::EyeLook::Camera(look.into()), 0.8);
                }
            }
            other => game.log.push(format!("SceneEdit event {other} not ported")),
        }
    }
    Ok(())
}

/// `SceneEdit.ClickCallback`.
fn click(game: &mut Game, opener: &mut Opener, e: &mut Edit, node: usize) -> Result<()> {
    let Some(b) = e.buttons.get(&node).copied() else { return Ok(()) };
    match b {
        Button::Category(ci) => {
            if let Some(m) = game.maid(0) {
                m.face.mouth_open = false;
            }
            for p in &PANELS[1..] {
                set_active(game, &format!("{ROOT}/{p}"), false)?;
            }
            match e.cats[ci].ecat {
                // ランダム / プリセット / プロフィール panels.
                5..=8 if e.cats[ci].ecat != 5 => {
                    if e.auto_cam {
                        game.camera = e.cam_reset;
                    }
                    if let Some(m) = game.maid(0) {
                        m.set_mask_mode("None")?;
                    }
                    game.camera_offset = crate::ngui::WIDTH / 6.;
                    game.log.push(format!("SceneEdit panel {} not ported", e.cats[ci].name));
                }
                _ => update_panel_parts(game, opener, e, ci)?,
            }
            set_active(game, &format!("{ROOT}/ScrollPanel-Category/Arrow"), true)?;
        }
        Button::Parts(ci, pi) => {
            let pt = e.cats[ci].parts[pi].clone();
            // PartsTypeWearMode / PartsTypeCamera.
            if let Some(mode) = e.tables.wear_mode.get(&(pt.mpn as i64)).cloned() {
                if let Some(m) = game.maid(0) {
                    m.set_mask_mode(&mode)?;
                }
            }
            if e.auto_cam {
                if let Some((bone, distance, angle)) = e.tables.camera.get(&(pt.mpn as i64)).cloned() {
                    // SATD-cm3d2-rendering: the bone's world position needs
                    // the CPU skeleton; the reset target is used for both.
                    let _ = bone;
                    game.camera = (e.cam_reset.0, distance, angle);
                }
            }
            if let Some(m) = game.maid(0) {
                m.face.mouth_open = crate::chara::MPN[pt.mpn] == "accha";
            }
            update_panel_menu(game, opener, e, ci, pi)?;
            set_active(game, &format!("{ROOT}/ScrollPanel-PartsType/Arrow"), true)?;
        }
        Button::Item(i) | Button::Color(i) => {
            let mi = e.items[i].clone();
            let storage = opener.storage.clone();
            let files = game.files.clone();
            let items = e.items.clone();
            if let Some(m) = game.maid(0) {
                m.set_prop_file(&mi.cate_name, &mi.file, false)?;
                if let Some(cm) = mi.color_set_mpn.filter(|c| *c != 0) {
                    let list = &mi.color_list;
                    let pick = if crate::chara::MPN[cm] == "haircolor" {
                        if m.props.get(cm).is_some_and(|p| p.file.is_empty()) { list.iter().find(|c| items[**c].multicolor.is_some()).or(list.first()) } else { None }
                    } else {
                        list.iter().find(|c| items[**c].multicolor.is_none()).or(list.first())
                    };
                    if let Some(c) = pick {
                        m.set_prop_file(&items[*c].cate_name, &items[*c].file, false)?;
                    }
                }
                m.all_proc_prop(&files, storage.as_ref())?;
            }
            if mi.is_color {
                if let Some(mpn) = mi.mpn {
                    update_selected_color(game, e, mpn);
                }
            } else if let Some((ci, pi)) = mi.parts_type {
                update_selected_menu_item(game, opener, e, e.cats[ci].parts[pi].mpn)?;
            }
        }
        Button::Slider(_) => {}
    }
    Ok(())
}

/// `SceneEdit.SlideCallback`.
fn slide(game: &mut Game, opener: &mut Opener, e: &mut Edit, node: usize) -> Result<()> {
    let Some(Button::Slider(i)) = e.buttons.get(&node).copied() else { return Ok(()) };
    let v = game.level.as_ref().and_then(|l| l.states.get(&node)).and_then(|s| s.value).unwrap_or(0.);
    let cate = e.items[i].cate_name.clone();
    let storage = opener.storage.clone();
    let files = game.files.clone();
    if let Some(m) = game.maid(0) {
        let p = crate::chara::mpn(&cate).and_then(|k| m.props.get(k)).cloned().context("slider prop")?;
        m.set_value(&cate, (p.min as f32 + (p.max - p.min) as f32 * v + 0.5) as i32)?;
        if cate == "MuneL" {
            let l = m.value("MuneL");
            let k = crate::chara::mpn("MuneTare").unwrap();
            m.props[k].max = l.max(1);
            if l < 1 {
                m.props[k].value = 0;
            }
            let t = m.props[k].value;
            m.set_value("MuneTare", t)?;
        }
        m.all_proc_prop(&files, storage.as_ref())?;
    }
    update_sliders(game, e)
}

/// `SceneEdit.OnEditOk`.
fn on_edit_ok(game: &mut Game, e: &mut Edit) -> Result<()> {
    if e.dialog.is_some() {
        return Ok(());
    }
    if let Some(m) = game.maid(0) {
        m.set_mask_mode("None")?;
    }
    let employed = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?.runtime.host().state.maid(0).is_some_and(|m| m.employment);
    if employed {
        game.update_profile_data(0)?;
        e.put_cloth = Some(game.now);
        return Ok(());
    }
    // FFNameDialog.Show(msg, 苗字で呼ぶ → family, 名前で呼ぶ → first).
    set_active(game, &format!("{ROOT}/FFNameDialog"), true)?;
    let level = game.level.as_mut().unwrap();
    for (path, t) in [("Base/Message", "名前と苗字のどちらで呼びますか？"), ("Base/SelectA/label", "苗字で呼ぶ"), ("Base/SelectB/label", "名前で呼ぶ")] {
        if let Some(n) = level.scene.find(&format!("{ROOT}/FFNameDialog/{path}")) {
            level.states.entry(n).or_default().text = Some(t.into());
        }
    }
    for (path, h) in [("Base/SelectA", "FFNameDialog.OnClickA"), ("Base/SelectB", "FFNameDialog.OnClickB")] {
        if let Some(n) = level.scene.find(&format!("{ROOT}/FFNameDialog/{path}")) {
            level.states.entry(n).or_default().click = Some(h.into());
        }
    }
    e.dialog = Some("ffname");
    Ok(())
}

fn first_name_call(game: &mut Game, first: bool) -> Result<()> {
    let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
    if let Some(m) = tjs.runtime.host_mut().state.maid_mut(0) {
        m.is_first_name_call = first;
    }
    Ok(())
}

/// `SceneEdit.Conform`: SysDlg.Show("このメイドにしますか？", OK_CANCEL).
fn confirm(game: &mut Game) -> Result<()> {
    game.set_main_active(SYSDLG, true)?;
    for (b, on) in [("Ok", true), ("Cancel", true)] {
        game.set_main_active(&format!("{SYSDLG}/Base/{b}"), on)?;
    }
    game.set_main_text(&format!("{SYSDLG}/Base/Message"), "このメイドにしますか？")?;
    for (b, h) in [("Ok", "SystemDialog.OnClickOk"), ("Cancel", "SystemDialog.OnClickCancel")] {
        if let Some(n) = game.main.scene.find(&format!("{SYSDLG}/Base/{b}")) {
            game.main.states.entry(n).or_default().click = Some(h.into());
        }
    }
    Ok(())
}

/// `SceneEdit.OnEndScene` (+ `OnDestroy`'s mask/visibility reset).
fn end_scene(game: &mut Game, opener: &mut Opener, e: &mut Edit) -> Result<()> {
    {
        let mut tjs = game.adv.tjs.lock().map_err(|_| anyhow::anyhow!("TJS lock"))?;
        if let Some(m) = tjs.runtime.host_mut().state.maid_mut(0) {
            m.employment = true;
            m.maid_point = 0;
            if e.first {
                m.leader = true;
            }
        }
    }
    if let Some(m) = game.maid(0) {
        m.set_mask_mode("None")?;
        m.face.mouth_open = false;
        m.visible = false;
    }
    game.camera_offset = 0.;
    game.adv_ui(crate::kag::AdvUi::SetBg(String::new()), opener)?;
    if let Some(label) = e.script_label.clone() {
        let storage = opener.storage.clone();
        game.adv.jump_label(&game.files, storage.as_ref(), &label)?;
        game.adv.exec();
    }
    Ok(())
}
