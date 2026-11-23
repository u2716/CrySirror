// SPDX-License-Identifier: MPL-2.0
use super::{Presentation, ScreenAbi, Sprite, remove, sprite, submit};
use crate::vm::Vm;
use anyhow::{Result, bail};
use crysirror_core::{
    service::{CivilTimeQuery, CivilTimeState, HostServices, InputFrame, buttons},
    state::ClientId,
};
use crysirror_image::temporal_pattern::PatternRng;
use serde_json::json;

pub(super) const BACK: u32 = 8000;
const OVER: u32 = 8001;
#[derive(Clone, Copy, PartialEq, Debug)]
enum Action {
    NewGame,
    Load,
    System,
    Memories,
    Mosaic,
    Album,
    Music,
    Opening,
    Episodes,
    Episode(usize),
    Route(usize),
    Back,
    Exit,
}
#[derive(Clone, Copy)]
struct Button {
    action: Action,
    bounds: [i32; 4],
    enabled: bool,
}
impl Button {
    fn hit(self, x: i32, y: i32) -> bool {
        let [l, t, r, b] = self.bounds;
        self.enabled && (l..r).contains(&x) && (t..b).contains(&y)
    }
}
struct Menu {
    base: Sprite,
    over: Sprite,
    buttons: Vec<Button>,
    selected: Option<usize>,
    pointer: Option<[i32; 2]>,
    redraw: bool,
}
const EPISODE_LAYERS: std::ops::Range<u32> = 8010..8019;

fn region(art: &Sprite, id: u32, [x, y, r, b]: [i32; 4]) -> serde_json::Value {
    let [_, _, w, h] = art.rect;
    let mut op = art.layer(id, 255);
    op["layer"]["rect"] = json!([x, y, r - x, b - y]);
    op["layer"]["uv"] = json!([
        x as f32 / w as f32,
        y as f32 / h as f32,
        r as f32 / w as f32,
        b as f32 / h as f32
    ]);
    op
}

fn episode_menu(
    h: &mut dyn HostServices,
    c: ClientId,
    vm: &Vm,
) -> Result<(Menu, Vec<serde_json::Value>)> {
    let base = sprite(h, c, &["sys"], "selectep0.png")?;
    let normal = sprite(h, c, &["sys"], "selectep1.png")?;
    let over = sprite(h, c, &["sys"], "selectep1over.png")?;
    let flags = [
        "FD_01", "FD_02", "FD_03", "FD_04", "FD_08", "FD_05", "FD_06", "FD_07", "FD_10",
    ];
    let rects = [
        [40, 40, 340, 135],
        [40, 135, 340, 230],
        [40, 230, 340, 320],
        [40, 325, 340, 420],
        [40, 420, 340, 515],
        [370, 40, 740, 135],
        [370, 135, 740, 230],
        [370, 230, 740, 325],
        [370, 325, 740, 420],
    ];
    let cleared = flags.map(|name| vm.global(name).unwrap_or(0) > 0);
    let mut buttons = Vec::new();
    let mut layers = Vec::new();
    for (i, bounds) in rects.into_iter().enumerate() {
        let enabled = cleared[i] || (i > 0 && cleared[i - 1]);
        buttons.push(Button {
            action: Action::Episode(i),
            bounds,
            enabled,
        });
        if enabled {
            let mut op = region(&normal, EPISODE_LAYERS.start + i as u32, bounds);
            op["layer"]["order"] = json!([BACK, i + 1]);
            layers.push(op);
        }
    }
    buttons.push(Button {
        action: Action::Back,
        bounds: [729, 556, 792, 590],
        enabled: true,
    });
    Ok((
        Menu {
            base,
            over,
            buttons,
            selected: None,
            pointer: None,
            redraw: true,
        },
        layers,
    ))
}

#[derive(Default)]
pub(crate) struct Title {
    clock: Option<Box<dyn CivilTimeQuery>>,
    menu: Option<Menu>,
    route: bool,
    episodes: bool,
    system: Option<super::settings::Settings>,
}
pub(crate) enum Outcome {
    Pending,
    Waiting,
    Start,
    Story { script: String, episode: bool },
    SettingsClosed,
    Close,
}
impl Title {
    pub(crate) fn name(&self) -> &'static str {
        if self.episodes {
            "episodes"
        } else if self.route {
            "routes"
        } else if self.system.is_some() {
            "settings"
        } else {
            "title"
        }
    }

    fn open(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        view: &mut Presentation,
        vm: &Vm,
    ) -> Result<bool> {
        let flag = |name| vm.global(name).is_some_and(|v| v != 0);
        use Action::*;
        let mut buttons = Vec::new();
        let mut add = |action, bounds, enabled| {
            buttons.push(Button {
                action,
                bounds,
                enabled,
            })
        };
        let storage = h.storage(c)?;
        let has = |name| -> Result<bool> { Ok(storage.exists(&super::asset("sys", name)?)) };
        let (base, over) = match view.screen_abi {
            _ if has("topmenutxt.png")? => {
                for (action, y, bottom) in [
                    (NewGame, 348, 380),
                    (Load, 380, 413),
                    (Album, 413, 449),
                    (Memories, 449, 482),
                    (Music, 482, 517),
                    (Opening, 517, 551),
                ] {
                    add(action, [50, y, 350, bottom], true);
                }
                ("topmenu.png".into(), "topmenutxt.png".into())
            }
            _ if has("title0over.png")? => {
                let progress = ["pre001", "ef_321_01", "ef_415_01", "ef_609_01", "ef_800_01"];
                let variant = progress
                    .iter()
                    .rposition(|name| flag(name))
                    .map_or(0, |i| i + 1);
                for (action, y, bottom) in [
                    (NewGame, 43, 71),
                    (Load, 71, 102),
                    (System, 102, 132),
                    (Memories, 132, 157),
                    (Exit, 157, 186),
                ] {
                    add(
                        action,
                        [668, y, 800, bottom],
                        action != Memories || variant == 5,
                    );
                }
                (
                    format!("title{variant}.png"),
                    format!("title{variant}over.png"),
                )
            }
            _ if has("selectep0.png")? => {
                let variant = if vm.global("FD_10").unwrap_or(0) > 0 {
                    2
                } else if vm.global("FD_01").unwrap_or(0) > 0 {
                    1
                } else {
                    0
                };
                for (action, top, bottom) in [
                    (NewGame, 43, 71),
                    (Load, 71, 99),
                    (Episodes, 102, 129),
                    (System, 132, 157),
                    (Memories, 157, 185),
                    (Exit, 185, 213),
                ] {
                    let enabled = match action {
                        Episodes => variant >= 1,
                        Memories => variant == 2,
                        _ => true,
                    };
                    add(action, [658, top, 800, bottom], enabled);
                }
                (
                    format!("topmenu{variant}.png"),
                    format!("topmenu{variant}over.png"),
                )
            }
            _ if has("topmenu3over.png")? || has("topmenu1noload.png")? => {
                let (suffix, memories, load) = if has("topmenu1noload.png")? {
                    if flag("TOHKA_CLEAR") {
                        ("2", true, true)
                    } else if ["AYAME_CLEAR", "SUI_CLEAR", "REN_CLEAR"]
                        .into_iter()
                        .all(flag)
                    {
                        if flag("D06") {
                            ("1", false, true)
                        } else {
                            ("1noload", false, false)
                        }
                    } else {
                        ("0", false, true)
                    }
                } else if flag("CLEAR_HOTARU") {
                    ("3", true, true)
                } else if flag("CLEAR_SAKURA") {
                    ("2", false, true)
                } else {
                    ("1", false, true)
                };
                for (action, y) in [
                    (NewGame, 32),
                    (Load, 80),
                    (System, 128),
                    (Memories, 176),
                    (Exit, 224),
                ] {
                    add(
                        action,
                        [1056, y, 1280, y + 32],
                        (action != Memories || memories) && (action != Load || load),
                    );
                }
                (
                    format!("topmenu{suffix}.png"),
                    format!("topmenu{suffix}over.png"),
                )
            }
            ScreenAbi::Early => {
                let unlocked = flag("ef_000");
                let stem = if unlocked { "topmenu2" } else { "topmenu" };
                for (action, top, bottom) in [
                    (NewGame, 43, 71),
                    (Load, 71, 102),
                    (System, 102, 132),
                    (Memories, 132, 157),
                    (Exit, 157, 186),
                ] {
                    add(
                        action,
                        [668, top, 800, bottom],
                        action != Memories || unlocked,
                    );
                }
                (format!("{stem}.png"), format!("{stem}Over.png"))
            }
            ScreenAbi::SignedDuration => {
                let cleared = flag("eden_D001");
                let variant = if cleared {
                    2
                } else if flag("eden_B006") {
                    1
                } else {
                    0
                };
                for (action, top, bottom) in [
                    (NewGame, 100, 128),
                    (Load, 128, 157),
                    (System, 157, 186),
                    (Memories, 186, 214),
                    (Mosaic, 214, 242),
                    (Exit, 242, 268),
                ] {
                    add(
                        action,
                        [850, top, 1024, bottom],
                        match action {
                            Memories => cleared,
                            Mosaic => false,
                            _ => true,
                        },
                    );
                }
                (
                    format!("topmenu{variant}.png"),
                    format!("topmenu{variant}over.png"),
                )
            }
            ScreenAbi::PersistentZoom => {
                if self.clock.is_none() {
                    self.clock = Some(h.begin_civil_time(c)?);
                }
                let time = match self.clock.as_mut().unwrap().poll()? {
                    CivilTimeState::Pending => return Ok(false),
                    CivilTimeState::Failed(message) => bail!("minori title clock: {message}"),
                    CivilTimeState::Ready(value) => value,
                };
                self.clock = None;
                let five_periods = has("topmenu/topmenue.png")?;
                let family = match (time.hour, time.minute) {
                    (4..=10, _) if five_periods => "A",
                    (19, 0..=4) if five_periods => "D",
                    (0..=3 | 19..=23, _) if five_periods => "E",
                    (5..=10, _) => "A",
                    (11..=15, _) => "B",
                    (16..=19, _) => "C",
                    _ => "D",
                };
                if has("topmenu/topmenuover.png")? {
                    for (action, l, r) in [
                        (NewGame, 272, 380),
                        (Load, 420, 508),
                        (System, 548, 680),
                        (Memories, 716, 884),
                        (Exit, 924, 1008),
                    ] {
                        add(action, [l, 232, r, 720], true);
                    }
                    (
                        format!("topmenu/topmenu{family}.png"),
                        "topmenu/topmenuover.png".to_owned(),
                    )
                } else {
                    let rng = view.random.get_or_insert_with(|| PatternRng::msvc(1));
                    let variant = if five_periods {
                        if flag("Groute3") {
                            1
                        } else {
                            vm.global("Groute2")
                                .or_else(|| vm.global("Groute1"))
                                .filter(|v| (0..4).contains(v))
                                .unwrap_or(0) as u32
                        }
                    } else if flag("A18_CLEAR") {
                        rng.next_draw()? % 3 + 1
                    } else if flag("A00_CLEAR") {
                        1
                    } else {
                        match (flag("B19_CLEAR"), flag("C23_CLEAR")) {
                            (true, true) => rng.next_draw()? % 2 + 2,
                            (true, false) => 2,
                            (false, true) => 3,
                            _ => 0,
                        }
                    };
                    for (action, l, r) in [
                        (NewGame, 296, 358),
                        (Load, 402, 484),
                        (System, 528, 654),
                        (Memories, 696, 860),
                        (Exit, 904, 984),
                    ] {
                        add(action, [l, 236, r, 720], true);
                    }
                    (
                        format!("topmenu/topmenu{family}.png"),
                        format!("topmenu/topmenu{family}{variant}.png"),
                    )
                }
            }
        };
        self.menu = Some(Menu {
            base: sprite(h, c, &["sys"], &base)?,
            over: sprite(h, c, &["sys"], &over)?,
            buttons,
            selected: None,
            pointer: None,
            redraw: true,
        });
        Ok(true)
    }

    pub(crate) fn tick(
        &mut self,
        h: &mut dyn HostServices,
        c: ClientId,
        view: &mut Presentation,
        vm: &Vm,
        input: InputFrame,
        routes: &[(String, [i32; 4])],
        episodes: &[String],
    ) -> Result<Outcome> {
        if self.menu.is_none() && !self.open(h, c, view, vm)? {
            return Ok(Outcome::Pending);
        }
        if (self.route || self.episodes)
            && input.buttons_pressed & (buttons::ESCAPE | buttons::MOUSE_RIGHT) != 0
        {
            self.route = false;
            self.episodes = false;
            self.menu = None;
            let mut ops = vec![remove(OVER)];
            ops.extend(EPISODE_LAYERS.map(remove));
            submit(&mut view.published, h, c, ops)?;
            return Ok(Outcome::Pending);
        }
        let menu = self.menu.as_mut().unwrap();
        let mut ops = Vec::new();
        let closed = self.system.is_some();
        if let Some(settings) = &mut self.system {
            if input.buttons_pressed & (buttons::ESCAPE | buttons::MOUSE_RIGHT) == 0
                && !settings.input(input, &mut view.speeds, &mut view.sound_settings, &mut ops) {
                if !ops.is_empty() {
                    submit(&mut view.published, h, c, ops)?;
                }
                return Ok(Outcome::Waiting);
            }
            settings.remove(&mut ops);
            self.system = None;
            menu.redraw = true;
        }
        if menu.redraw {
            ops.push(menu.base.layer(BACK, 255));
        }
        let pointer = [input.pointer_x, input.pointer_y];
        let before = menu.selected;
        let clicked = input.buttons_pressed & buttons::MOUSE_LEFT != 0;
        if menu.pointer != Some(pointer) || clicked {
            menu.selected = menu
                .buttons
                .iter()
                .position(|b| b.hit(pointer[0], pointer[1]));
            menu.pointer = Some(pointer);
        }
        let pressed = input.buttons_pressed;
        let backward = pressed & (buttons::UP | buttons::LEFT) != 0;
        if backward || pressed & (buttons::DOWN | buttons::RIGHT) != 0 {
            let count = menu.buttons.len();
            let first = menu
                .selected
                .unwrap_or(if backward { 0 } else { count - 1 });
            menu.selected = (1..=count)
                .map(|step| {
                    if backward {
                        (first + count - step) % count
                    } else {
                        (first + step) % count
                    }
                })
                .find(|i| menu.buttons[*i].enabled);
        }
        if before != menu.selected || menu.redraw {
            menu.redraw = false;
            if let Some(index) = menu.selected {
                ops.push(region(&menu.over, OVER, menu.buttons[index].bounds));
            } else {
                ops.push(remove(OVER));
            }
        }
        let selection = if clicked {
            menu.selected
        } else if pressed & (buttons::ENTER | buttons::SPACE) != 0 {
            menu.selected.or(Some(0))
        } else {
            None
        };
        let mut result = if closed {
            Outcome::SettingsClosed
        } else {
            Outcome::Waiting
        };
        if let Some(index) = selection.filter(|_| !closed) {
            match menu.buttons[index].action {
                Action::NewGame if !routes.is_empty() => {
                    let base = sprite(h, c, &["sys"], "topmenu/selectroute.png")?;
                    let over = sprite(h, c, &["sys"], "topmenu/selectrouteover.png")?;
                    let mut buttons: Vec<_> = routes
                        .iter()
                        .enumerate()
                        .map(|(i, (_, bounds))| Button {
                            action: Action::Route(i),
                            bounds: *bounds,
                            enabled: true,
                        })
                        .collect();
                    buttons.push(Button {
                        action: Action::Back,
                        bounds: [272, 232, 380, 720],
                        enabled: true,
                    });
                    self.menu = Some(Menu {
                        base,
                        over,
                        buttons,
                        selected: None,
                        pointer: None,
                        redraw: true,
                    });
                    self.route = true;
                    ops.push(remove(OVER));
                    result = Outcome::Pending;
                }
                Action::Episodes => {
                    anyhow::ensure!(episodes.len() == 9, "missing native minori episode table");
                    let (menu, unlocked) = episode_menu(h, c, vm)?;
                    ops.extend(unlocked);
                    ops.push(remove(OVER));
                    self.menu = Some(menu);
                    self.episodes = true;
                    result = Outcome::Pending;
                }
                Action::Episode(i) => {
                    ops.extend(EPISODE_LAYERS.map(remove));
                    ops.extend([remove(OVER), remove(BACK)]);
                    result = Outcome::Story {
                        script: episodes[i].clone(),
                        episode: true,
                    };
                }
                Action::Back => {
                    if self.episodes {
                        ops.extend(EPISODE_LAYERS.map(remove));
                        self.episodes = false;
                    }
                    self.menu = None;
                    self.route = false;
                    result = Outcome::Pending;
                }
                Action::Route(i) => {
                    ops.push(remove(OVER));
                    ops.push(remove(BACK));
                    result = Outcome::Story {
                        script: routes[i].0.clone(),
                        episode: false,
                    };
                }
                Action::NewGame | Action::Exit => {
                    ops.push(remove(OVER));
                    ops.push(remove(BACK));
                    result = if menu.buttons[index].action == Action::NewGame {
                        Outcome::Start
                    } else {
                        Outcome::Close
                    };
                }
                Action::System => {
                    self.system = Some(super::settings::Settings::open(
                        h,
                        c,
                        view.screen_abi,
                        view.speeds,
                        &view.voice_actors,
                        &view.voice_marks,
                        &view.sound_settings,
                        &mut ops,
                    )?);
                    ops.push(remove(OVER));
                }
                action => bail!("minori title scene {action:?} requires its native menu"),
            }
        }
        submit(&mut view.published, h, c, ops)?;
        Ok(result)
    }
}
