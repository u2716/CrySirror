// SPDX-License-Identifier: MPL-2.0
use super::super::installed_menu::Menu;
use super::*;
fn click(m: &mut Menu, end: &str) -> Result<()> {
    let r = m
        .game
        .widgets
        .iter()
        .rev()
        .find(|w| w.path.ends_with(end) && w.collider.is_some())
        .and_then(|w| w.collider)
        .with_context(|| format!("missing hit target {end} in {:?}", m.game.scene))?;
    m.click(r)
}
fn open_daily_button(
    m: &mut Menu,
    night: bool,
    button: daily::Button,
    destination: Destination,
) -> Result<()> {
    let period = if night {
        daily::Period::Night
    } else {
        daily::Period::Daytime
    };
    m.open(
        night,
        if night {
            "*夜メニュー"
        } else {
            "*昼メニュー"
        },
        Scene::Daily(period),
    )?;
    if !m.game.daily.pages.is_empty() {
        click(m, "/Button OK")?;
        m.tick(501)?;
    }
    let rect = m
        .game
        .widgets
        .iter()
        .find(|w| w.button.and_then(|id| m.game.assets.daily_buttons.get(&id)) == Some(&button))
        .and_then(|w| w.collider)
        .context("daily button not interactable")?;
    m.click(rect)?;
    m.wait(Scene::Leisure(destination))?;
    m.tick(500)
}
#[test]
#[ignore = "installed standalone 1.0.4 scene and archives"]
fn installed_photo_menu() -> Result<()> {
    let mut m = Menu::new()?;
    open_daily_button(&mut m, false, daily::Button::PhotoMode, Photography)?;
    assert!(m.game.club_meshes > 0);
    click(&mut m, "/UIOFF")?;
    assert!(
        m.game
            .widgets
            .iter()
            .all(|w| w.path.contains("/UION") || w.path.contains("/ScreenShot"))
    );
    click(&mut m, "/UION")?;
    click(&mut m, "photo/background/1")?;
    assert_eq!(
        m.game
            .script
            .tjs
            .lock()
            .unwrap()
            .runtime
            .host()
            .state
            .bg_name,
        "EmpireClub_Rotary_Night"
    );
    click(&mut m, "WindowVisibleBtnsLine2/End")?;
    click(&mut m, "/Base/Cancel")?;
    m.tick(301)?;
    assert_eq!(m.game.scene, Scene::Leisure(Photography));
    click(&mut m, "WindowVisibleBtnsLine2/End")?;
    click(&mut m, "/Base/Ok")?;
    m.wait(Scene::Daily(daily::Period::Daytime))
}
#[test]
#[ignore = "installed standalone 1.0.4 scene and archives"]
fn installed_creative_room() -> Result<()> {
    let mut m = Menu::new()?;
    open_daily_button(&mut m, true, daily::Button::MyRoomCustom, CreativeRoom)?;
    let initial = m.game.club_meshes;
    assert_eq!(initial, 10);
    click(&mut m, "/Slider SizeX/HitArea")?;
    assert!(m.game.club_meshes > initial);
    let edited = m.game.club_meshes;
    click(&mut m, "/Button Save")?;
    click(&mut m, "/Element/Button Element")?;
    click(&mut m, "/Base/Ok")?;
    click(&mut m, "/Button Save")?;
    click(&mut m, "/Slider SizeZ/HitArea")?;
    assert_ne!(edited, m.game.club_meshes);
    click(&mut m, "/Button Load")?;
    click(&mut m, "/Element/Button Element")?;
    click(&mut m, "/Base/Ok")?;
    assert_eq!(edited, m.game.club_meshes);
    click(&mut m, "/Button Quit")?;
    click(&mut m, "/Base/Ok")?;
    m.wait(Scene::Daily(daily::Period::Night))
}
#[test]
#[ignore = "installed standalone 1.0.4 scene and archives"]
fn installed_examination() -> Result<()> {
    let mut m = Menu::with_motion(true)?;
    open_daily_button(&mut m, false, daily::Button::CompetitiveShow, Examination)?;
    m.tick(6400)?;
    assert!(m.game.widgets.is_empty());
    m.tick(3500)?;
    let key = if m.game.leisure.dialog.is_some() {
        "/Base/Ok"
    } else {
        "/Viewer/Ok"
    };
    click(&mut m, key)?;
    assert_eq!(m.game.scene, Scene::Script);
    assert!(
        m.game
            .script
            .position()
            .to_lowercase()
            .contains("daytimemain")
    );
    assert_eq!(
        m.game
            .script
            .tjs
            .lock()
            .unwrap()
            .runtime
            .host()
            .state
            .tmp_flags
            .get("品評会_演出省略"),
        Some(&0)
    );
    // The original caller plays the entrant's result dialogue before reopening the menu.
    for _ in 0..160 {
        if m.game.scene == Scene::Daily(daily::Period::Daytime) {
            return Ok(());
        }
        if m.game.script.hitret_since().is_some() {
            m.click([900., 800., 8., 8.])?;
        }
        m.tick(250)?;
    }
    bail!(
        "examination return stalled at {} ({})",
        m.game.script.position(),
        m.game.script.wait_state()
    )
}
#[test]
#[ignore = "installed standalone 1.0.4 scene and archives"]
fn installed_casino() -> Result<()> {
    let mut m = Menu::with_motion(true)?;
    {
        let mut vm = m.game.script.tjs.lock().unwrap();
        let s = &mut vm.runtime.host_mut().state;
        let cat = crate::tjs::retail::construction::Catalog::load(&mut s.retail)?;
        s.player.money = cat.get(170)?.cost + 310_000;
        cat.reserve(s, 8, Some(170))?;
        crate::tjs::retail::operations::start_day(s)?;
        assert_eq!(
            crate::tjs::retail::construction::Catalog::live(&s.retail, 8),
            Some(170)
        );
    }
    open_daily_button(&mut m, true, daily::Button::Casino, Casino)?;
    click(&mut m, "/CasinoMenu/SelectDealer")?;
    let selected = m
        .game
        .widgets
        .iter()
        .find(|w| w.path.starts_with("dealer/"))
        .and_then(|w| w.collider)
        .context("dealer roster")?;
    m.click(selected)?;
    click(&mut m, "/Ok_Button")?;
    click(&mut m, "/CasinoMenu/BlackJack")?;
    assert!(m.game.leisure.dialog.is_some());
    click(&mut m, "/Base/Ok")?;
    click(&mut m, "/ButtonKankin")?;
    click(&mut m, "/ExChangeUI/Max")?;
    click(&mut m, "/ExChangeUI/Cancel")?;
    {
        let vm = m.game.script.tjs.lock().unwrap();
        assert_eq!(vm.runtime.host().state.retail.casino_coins, 0);
    }
    click(&mut m, "/ButtonKankin")?;
    click(&mut m, "/ExChangeUI/Max")?;
    click(&mut m, "/ExChangeUI/OK")?;
    {
        let vm = m.game.script.tjs.lock().unwrap();
        let s = &vm.runtime.host().state;
        assert_eq!((s.player.money, s.retail.casino_coins), (0, 310));
    }
    click(&mut m, "/CasinoMenu/Cancel_Button")?;
    m.wait(Scene::Daily(daily::Period::Night))
}
