// SPDX-License-Identifier: MPL-2.0
use super::*;
fn centered(w: &Widget, button: u64) -> Result<InputFrame> {
    let [x, y, width, height] = w.collider.context("missing authored collider")?;
    Ok(InputFrame {
        pointer_x: (x + width / 2.) as i32,
        pointer_y: (y + height / 2.) as i32,
        buttons_pressed: button,
        ..Default::default()
    })
}
fn press(
    panel: &Panel,
    widgets: &[Widget],
    which: Action,
    scene: Scene,
    layout: &Layout,
) -> Result<Action> {
    let w = widgets
        .iter()
        .find(|w| panel.bindings.get(&w.path) == Some(&which))
        .with_context(|| format!("missing control {which:?}"))?;
    let [x, y, _, _] = w.collider.context("missing authored collider")?;
    let mut input = centered(w, buttons::MOUSE_LEFT)?;
    ensure!(
        (0..1920).contains(&input.pointer_x) && (0..1080).contains(&input.pointer_y),
        "off-canvas {which:?} {x},{y}"
    );
    let mut held = None;
    assert_eq!(
        panel.click(widgets, scene, layout, &input, 0, &mut held),
        None
    );
    input.buttons_pressed = 0;
    let clicked = panel.click(
        widgets,
        scene,
        layout,
        &input,
        buttons::MOUSE_LEFT,
        &mut held,
    );
    assert_eq!(
        clicked,
        Some(which),
        "{which:?} at {x},{y} clip {:?}",
        panel.clip(scene, &w.path, layout)
    );
    Ok(clicked.unwrap())
}
#[test]
#[ignore = "requires COM3D2_ROOT standalone installation"]
fn installed_schedule_and_result_controls() -> Result<()> {
    let (_tmp, mut rt, c, mut op, files, mut s, mut assets) =
        daily::installed_panel(&["sound", "sound2"])?;
    operations::start_day(&mut s)?;
    s.stock.extend([0, 1]);
    let reserve = s.maids.entry(1).or_default();
    reserve.personal = 100;
    reserve.guid_id = "reserve".into();
    reserve.first_name = "澪".into();
    let m = s.maids.entry(0).or_default();
    m.personal = 100;
    m.guid_id = "local-fixture".into();
    m.first_name = "葵".into();
    operations::open_schedule(&mut s)?;
    let day = Scene::Daily(daily::Period::Daytime);
    let mut panel = Panel {
        base: assets.scene(&mut op, &mut rt.host, c, "SchedulePanel")?,
        unit: assets.scene(&mut op, &mut rt.host, c, "TrainingUnit")?,
        plate: assets.scene(&mut op, &mut rt.host, c, "StaffPlate")?,
        category: assets.scene(&mut op, &mut rt.host, c, "TrainingCategory")?,
        ..Default::default()
    };
    // Fill an empty original row, then assign both shifts to both staff.
    for action in [Action::Staff(1), Action::Choose(1)] {
        let widgets = panel.populate(None, &mut s, &mut assets, &op, &mut rt.host, c, &files)?;
        let clicked = press(&panel, &widgets, action, day, &assets.operations_layout)?;
        panel.activate(clicked, &mut s, 0)?;
    }
    assert_eq!(s.schedule_slot(1), Some(1));
    assert!(operations::candidates(&s).is_empty());
    for slot in 0..2 {
        for (shift, work) in [(Shift::Day, 2001), (Shift::Night, 2003)] {
            for action in [
                Action::Shift(slot, shift),
                Action::Training,
                Action::Assign(work),
            ] {
                let widgets =
                    panel.populate(None, &mut s, &mut assets, &op, &mut rt.host, c, &files)?;
                let clicked = press(&panel, &widgets, action, day, &assets.operations_layout)?;
                panel.activate(clicked, &mut s, 0)?;
            }
            assert_eq!(
                panel
                    .bindings
                    .values()
                    .filter(|a| matches!(a, Action::Assign(_)))
                    .count(),
                8
            );
        }
    }
    let rows = panel.populate(None, &mut s, &mut assets, &op, &mut rt.host, c, &files)?;
    let icon = rows
        .iter()
        .find(|w| panel.bindings.get(&w.path) == Some(&Action::Shift(0, Shift::Day)))
        .unwrap();
    let tooltip = rows
        .iter()
        .find(|w| {
            w.path.starts_with("roster/0/") && w.path.ends_with("/DaytimeTask/NamePlate/Name")
        })
        .unwrap();
    assert!(!panel.tooltip_visible(&tooltip.path));
    panel.update_hover(&rows, day, &assets.operations_layout, &centered(icon, 0)?);
    assert!(panel.tooltip_visible(&tooltip.path));
    assert_eq!(tooltip.label.as_ref().unwrap().text, "ガーデニング");
    panel.update_hover(
        &rows,
        day,
        &assets.operations_layout,
        &InputFrame::default(),
    );
    assert!(!panel.tooltip_visible(&tooltip.path));
    for (slot, shift, on) in [
        (0, Shift::Day, true),
        (1, Shift::Day, true),
        (1, Shift::Day, false),
        (0, Shift::Night, true),
    ] {
        let rows = panel.populate(None, &mut s, &mut assets, &op, &mut rt.host, c, &files)?;
        let selected = press(
            &panel,
            &rows,
            Action::Communication(slot, shift),
            day,
            &assets.operations_layout,
        )?;
        panel.activate(selected, &mut s, 0)?;
        assert_eq!(operations::communication(&s, slot, shift), on);
        if slot == 1 {
            assert!(!operations::communication(&s, 0, Shift::Day));
        }
        let rows = panel.populate(None, &mut s, &mut assets, &op, &mut rt.host, c, &files)?;
        let suffix = if shift == Shift::Day {
            "/CommuNoon/SelectCursor"
        } else {
            "/CommuNight/SelectCursor"
        };
        assert_eq!(rows.iter().any(|w|w.path.starts_with(&format!("roster/{slot}/")) && w.path.ends_with(suffix)),on);
    }
    // ScheduleCtrl asks for confirmation on a right click. Cancel preserves
    // the assignment; OK frees the slot without deleting stock or work data.
    s.maids.get_mut(&0).unwrap().leader = true;
    for (name, removed) in [("Cancel", false), ("Ok", true)] {
        let mut rows = panel.populate(None, &mut s, &mut assets, &op, &mut rt.host, c, &files)?;
        let target = rows
            .iter()
            .find(|w| panel.bindings.get(&w.path) == Some(&Action::Staff(1)))
            .unwrap();
        let mut input = centered(target, buttons::MOUSE_RIGHT)?;
        assert_eq!(
            panel.right_click_staff(&rows, day, &assets.operations_layout, &input, 0),
            None
        );
        input.buttons_pressed = 0;
        let slot = panel
            .right_click_staff(
                &rows,
                day,
                &assets.operations_layout,
                &input,
                buttons::MOUSE_RIGHT,
            )
            .unwrap();
        let message = operations::removal_prompt(&s, slot).unwrap();
        assert!(message.contains("澪"));
        let dialog = assets.scene(&mut op, &mut rt.host, c, "SystemDialog")?;
        panel.remove_dialog = Some((
            slot,
            quit_dialog::QuitDialog::show(&mut rows, dialog, 0, &message)?,
        ));
        // A click over the roster while the dialog owns input must leave it open.
        assert!(!panel.resolve_removal(&mut s, &mut rows, input, buttons::MOUSE_LEFT, 400)?);
        assert!(panel.dialog().is_some());
        let choice = rows[panel.dialog().unwrap().first..]
            .iter()
            .find(|w| w.path.ends_with(&format!("/{name}")) && w.collider.is_some())
            .unwrap();
        let mut input = centered(choice, buttons::MOUSE_LEFT)?;
        assert!(!panel.resolve_removal(&mut s, &mut rows, input, 0, 450)?);
        input.buttons_pressed = 0;
        assert_eq!(
            panel.resolve_removal(&mut s, &mut rows, input, buttons::MOUSE_LEFT, 500)?,
            removed
        );
        if !removed {
            assert!(!panel.resolve_removal(&mut s, &mut rows, input, 0, 810)?);
        }
        assert!(panel.dialog().is_none());
        assert_eq!(s.schedule_slot(1), (!removed).then_some(1));
    }
    assert!(operations::removal_prompt(&s, 0).is_none());
    assert_eq!(operations::candidates(&s), vec![1]);
    operations::choose_staff(&mut s, 1, 1)?;
    let rows = panel.populate(None, &mut s, &mut assets, &op, &mut rt.host, c, &files)?;
    let ok = press(&panel, &rows, Action::Ok, day, &assets.operations_layout)?;
    panel.activate(ok, &mut s, 100)?;
    assert_eq!(panel.close, Some(100));
    crate::tjs::retail::close_daily(&mut s, true)?;
    for shift in [Shift::Day, Shift::Night] {
        operations::settle(&mut s, shift)?;
        let kind = ResultPanel::Work(shift);
        let mut panel = Panel {
            base: assets.scene(&mut op, &mut rt.host, c, kind.asset())?,
            unit: assets.scene(&mut op, &mut rt.host, c, "ResultMaidUnit")?,
            stamp: assets
                .scene(&mut op, &mut rt.host, c, "ResultStamp")?
                .into_iter()
                .find(|w| w.path == "ResultWorkIcon"),
            ..Default::default()
        };
        let rows = panel.populate(
            Some(shift),
            &mut s,
            &mut assets,
            &op,
            &mut rt.host,
            c,
            &files,
        )?;
        assert!(rows.iter().any(|w| w.path.starts_with("result/0/")
            && w.path.ends_with("/Task")
            && !w.asset.is_empty()));
        assert!(
            rows.iter()
                .any(|w| w.path.starts_with("result/0/reception/") && w.label.is_some())
        );
        assert!(
            rows.iter()
                .any(|w| w.path == "result/1/ResultWorkIcon" && !w.asset.is_empty())
        );
        press(
            &panel,
            &rows,
            Action::Ok,
            Scene::Result(kind),
            &assets.operations_layout,
        )?;
    }
    for kind in [
        ResultPanel::Work(Shift::Day),
        ResultPanel::Work(Shift::Night),
        ResultPanel::Income,
    ] {
        assert!(
            assets
                .unity
                .resource_of(
                    &mut op,
                    &format!("bg/{}", kind.background()),
                    crysirror_unity::class_id::GAME_OBJECT
                )?
                .is_some()
        );
    }
    for audio in ["SE002.ogg", "BGM015.ogg"] {
        assert!(files.exists(audio), "missing {audio}");
    }
    let mut receipt = assets.scene(&mut op, &mut rt.host, c, "ResultIncomePanel")?;
    // Signed accounting and grouping in the six real receipt labels.
    s.player.schedule_slot[0]
        .backup_status
        .insert("Start_clubMoney".into(), 1_004_999);
    s.player.schedule_slot[0]
        .backup_status
        .insert("Start_clubEvaluation".into(), 32);
    s.player.money = 1_000_000;
    s.player.salon_evaluation = 30;
    setup_income(&mut receipt, &s);
    for (suffix, val) in [
        ("WorkingFunds", "1,004,999"),
        ("ClubIncome", "-4,999"),
        ("TotalAmount", "1,000,000"),
        ("ClientEvaluation", "32"),
        ("AcquisitionEvaluation", "--2"),
        ("Result", "30"),
    ] {
        assert!(
            receipt
                .iter()
                .any(|w| w.path.ends_with(&format!("/{suffix}/Value"))
                    && w.label.as_ref().is_some_and(|label| label.text == val)),
            "receipt {suffix}"
        );
    }
    assert_eq!(
        receipt
            .iter()
            .filter(|w| w.path.ends_with("/Value") && w.label.is_some())
            .count(),
        6
    );
    Ok(())
}

/// Starts at AllMain's return from the daytime menu. Only the scenario
/// interpreter and business callbacks run, without a game VM, renderer or audio.
#[test]
#[ignore = "requires COM3D2_ROOT standalone installation"]
fn installed_management_script_callbacks() -> Result<()> {
    management_script(false,false)
}

#[test]
#[ignore = "requires COM3D2_ROOT standalone installation"]
fn installed_default_night_work_to_second_day() -> Result<()> {
    management_script(true,false)
}

#[test]
#[ignore = "requires COM3D2_ROOT standalone installation"]
fn installed_upgrade_script_to_second_day() -> Result<()> {
    management_script(false,true)
}

fn management_script(default_assignment: bool, manual_upgrade:bool) -> Result<()> {
    use crate::kag::AdvUi;
    let (_tmp, mut rt, client, mut op, files, mut s, mut assets) =
        daily::installed_panel(&["parts", "parts2"])?;
    operations::start_day(&mut s)?;
    let mut script = AdvKag::for_profile(
        &files,
        op.storage.as_ref(),
        crate::tjs::ScriptProfile::Com3d2,
        s,
    )?;
    let assignments;
    {
        let mut vm = script.tjs.lock().unwrap();
        vm.eval("CreateNewMaid('Muku')")?;
        let s = &mut vm.runtime.host_mut().state;
        crate::tjs::retail::finish_profile(s, true)?;
        s.active_maids.fill(None);
        s.player.generic_flag.insert("時間帯".into(), 2);
        s.player.generic_flag.insert("オープニング終了".into(), 1);
        operations::open_schedule(s)?;
        if !default_assignment {
            if manual_upgrade {
                operations::upgrade::assign(s,0,Shift::Day)?;
                // Buy from the installed grade-one catalog before scheduling the upgrade.
                s.player.current_salon_grade = 1;
                s.player.money = 30_000;
                let shop = crate::tjs::retail::shop::Catalog::open(s, 0)?;
                shop.buy(s, 10000)?;
                assert_eq!(s.player.money, 0);
                assert!(s.retail.materials.contains(&10));
            } else {
                operations::assign(s, 0, Shift::Day, 2001)?;
            }
            operations::assign(s, 0, Shift::Night, 2003)?;
        }
        let m = &s.maids[&s.schedule_slot(0).unwrap()];
        assignments = [m.noon_work_id, m.night_work_id];
        if manual_upgrade { assert_eq!(assignments[0],3000); } else { assert!((2001..=2003).contains(&assignments[0])); }
        assert_eq!(assignments[1], if default_assignment { 10000 } else { 2003 });
        crate::tjs::retail::close_daily(s, true)?;
    }
    script.load(&files, op.storage.as_ref(), "AllMain", "*RET_DaytimeMain")?;
    script.exec();
    let mut scenes = Vec::new();
    let mut ended = false;
    let mut second_day = false;
    for step in 1..=400 {
        let state = script
            .update(step * 5000, true, false, &files, op.storage.as_ref())
            .with_context(|| script.position())?;
        for effect in std::mem::take(&mut script.ui) {
            let mut vm = script.tjs.lock().unwrap();
            let s = &mut vm.runtime.host_mut().state;
            match effect {
                AdvUi::StartDay => {
                    let mut expected = vec![];
                    if manual_upgrade {expected.extend(["Facility","SceneFacilityPowerUp"]);}
                    expected.extend(["ResultWorkDaytime", "Night"]);
                    if default_assignment { expected.push("Yotogi"); }
                    expected.extend(["ResultWorkNight", "ResultIncome", "SceneStartDaily"]);
                    ensure!(ended && scenes == expected, "unexpected schedule traversal {scenes:?}");
                    operations::start_day(s)?;
                    assert_eq!(s.player.days, 2);
                    let id = s.schedule_slot(0).context("missing restored roster")?;
                    let m = &s.maids[&id];
                    assert_eq!([m.noon_work_id, m.night_work_id], assignments);
                    assert_eq!(m.work_data.get(&assignments[0]).unwrap().play_count, 1);
                    if default_assignment {
                        assert!(m.work_data.get(&10000).is_none());
                        assert_eq!(m.yotogi_play_count, 0);
                        assert_eq!(m.generic_flag.get("_PlayedNightWorkId10000"), Some(&1));
                        assert_eq!(m.generic_flag.get("_YotogiPlayed"), Some(&0));
                    } else {
                        assert_eq!(m.work_data.get(&assignments[1]).unwrap().play_count, 1);
                    }
                    second_day = true;
                }
                AdvUi::EndDay => {
                    operations::end_day(s)?;
                    ended = true;
                }
                AdvUi::CharaDeactivate { man: false, slot } | AdvUi::RetailDeactivate(slot) => {
                    if let Some(x) = s.active_maids.get_mut(slot) {
                        *x = None;
                    }
                }
                AdvUi::RetailChoices(_) => bail!("unexpected choice at {}", script.position()),
                _ => {}
            }
        }
        if let KagStep::SceneCall(scene_name) = state {
            if scene_name == "SceneADV" {
                script.enabled = true;
                script.exec();
                continue;
            }
            let mut return_key = "label";
            let tag = script
                .tag_backup
                .get("type")
                .cloned()
                .unwrap_or(scene_name.clone());
            {
                let mut vm = script.tjs.lock().unwrap();
                let s = &mut vm.runtime.host_mut().state;
                match scene_name.as_str() {
                    "SceneDaily" => match scene(Some(&tag))? {
                        Scene::Daily(daily::Period::Daytime) if second_day => {
                            assert_eq!(s.player.flag("時間帯"), 2);
                            assert_eq!(s.player.days, 2);
                            let casino = crate::tjs::retail::daily_status(s, false)?;
                            let mut widgets =
                                assets.scene(&mut op, &mut rt.host, client, "SceneDaily")?;
                            let karaoke = (1..=100).any(|i| {
                                files.exists(&format!("karaoke_music_enable_list_{i:03}.nei"))
                            });
                            daily::prepare(
                                &mut widgets,
                                &assets.daily_buttons,
                                daily::Period::Daytime,
                                karaoke,
                                casino,
                                s,
                            )?;
                            assert!(widgets.iter().any(|w| w.path.ends_with("/Days/Value")
                                && w.label.as_ref().is_some_and(|l| l.text == "2")));
                            return Ok(());
                        }
                        Scene::Result(ResultPanel::Work(shift)) => {
                            let funds = s.player.money;
                            operations::settle(s, shift)?;
                            if shift==Shift::Day && manual_upgrade {
                                let mut result = Panel { base: assets.scene(&mut op,&mut rt.host,client,"ResultWorkPanel")?,
                                    unit:assets.scene(&mut op,&mut rt.host,client,"ResultMaidUnit")?,
                                    stamp:assets.scene(&mut op,&mut rt.host,client,"ResultStamp")?.into_iter().find(|w|w.path=="ResultWorkIcon"),..Default::default() };
                                let receipt=result.populate(Some(shift),s,&mut assets,&op,&mut rt.host,client,&files)?;
                                for title in ["RECIPE 0010","ワックスがけ","トレーニングルーム","施設EXPアップ","+ 1"] {
                                    assert!(receipt.iter().any(|w|w.label.as_ref().is_some_and(|l|l.text==title)),"missing upgrade receipt {title}");
                                }
                                assert!(receipt.iter().any(|w|w.path.ends_with("/Thumbnail") && !w.asset.is_empty()));
                            }
                            if shift == Shift::Night && default_assignment {
                                assert_eq!(s.player.money, funds);
                                assert_eq!(operations::receipt(s, 0, shift)?, (0, ("NoonBonus", "NightBonus")));
                            }
                        }
                        Scene::Result(ResultPanel::Income) => {
                            assert_eq!(s.player.flag("_Schedule_Night_Resulted"), 1);
                        }
                        Scene::Daily(daily::Period::Night) => {
                            crate::tjs::retail::daily_status(s, true)?;
                            crate::tjs::retail::close_daily(s, false)?;
                        }
                        x => bail!("unexpected management entry {x:?}"),
                    },
                    "SceneCharacterSelect" if manual_upgrade && tag=="Facility" => {
                        let mode=PickMode::from_tag(Some(&tag))?;
                        let mut panel=Panel{base:assets.scene(&mut op,&mut rt.host,client,"SceneCharacterSelect")?,
                            plate:assets.scene(&mut op,&mut rt.host,client,"PickerPlate")?,..Default::default()};
                        panel.init_picker(mode,true,s);
                        let mut widgets=panel.picker_widgets(s,&mut assets,&mut rt.host,client)?;
                        let button=widgets.iter().find(|w|w.path.ends_with("/ButtonParent/OK")).context("picker OK")?;
                        let mut input=centered(button,buttons::MOUSE_LEFT)?;
                        let mut pressed=None;
                        assert!(panel.picker_input(mode,&mut widgets,s,&mut assets,&mut op,&mut rt.host,client,input,0,0,&mut pressed)?.is_none());
                        input.buttons_pressed=0;
                        return_key=panel.picker_input(mode,&mut widgets,s,&mut assets,&mut op,&mut rt.host,client,input,buttons::MOUSE_LEFT,20,&mut pressed)?.context("facility employee not selected")?.finish(s)?;
                    }
                    "SceneFacilityPowerUp" if manual_upgrade => {
                        assert!(s.maid(0).is_some());
                        return_key=power::fixture_confirm_upgrade(s,&mut assets,&mut op,&mut rt.host,client)?;
                    }
                    "SceneCharacterSelect" if default_assignment => {
                        assert_eq!(tag, "Yotogi");
                        assert_eq!(script.tag_backup["cancel_label"], "*End");
                        let id = s.schedule_slot(0).unwrap();
                        assert_eq!(s.chara_guid_list, [s.maids[&id].guid_id.clone()]);
                        let mode = PickMode::from_tag(Some(&tag))?;
                        let mut panel = Panel {
                            base: assets.scene(&mut op, &mut rt.host, client, "SceneCharacterSelect")?,
                            plate: assets.scene(&mut op, &mut rt.host, client, "PickerPlate")?,
                            ..Default::default()
                        };
                        panel.init_picker(mode, true, s);
                        assert_eq!(panel.picker.selected, Some(id));
                        let mut widgets = panel.picker_widgets(s, &mut assets, &mut rt.host, client)?;
                        assert!(widgets.iter().any(|w| w.path.starts_with(&format!("staff/{id}/"))));
                        let mut held = None;
                        let mut clock = 0;
                        // First dismiss the system dialog. Reopen it, then accept.
                        for name in ["/ButtonParent/Cancel", "/Cancel", "/ButtonParent/Cancel", "/Ok"] {
                            let first = panel.dialog().map_or(0, |d| d.first);
                            let w = widgets[first..].iter().find(|w| w.path.ends_with(name) && w.collider.is_some())
                                .with_context(|| format!("missing picker control {name}"))?;
                            let mut input = centered(w, buttons::MOUSE_LEFT)?;
                            assert!((0..1920).contains(&input.pointer_x) && (0..1080).contains(&input.pointer_y), "{name} outside viewport");
                            let mut poll = |input, released| {
                                clock += 310;
                                panel.picker_input(mode, &mut widgets, s, &mut assets, &mut op, &mut rt.host, client, input, released, clock, &mut held)
                            };
                            assert!(poll(input, 0)?.is_none());
                            input.buttons_pressed = 0;
                            let exit = poll(input, buttons::MOUSE_LEFT)?;
                            if name == "/Cancel" {
                                assert!(poll(input, 0)?.is_none());
                                assert!(panel.dialog().is_none());
                                assert_eq!(s.chara_guid_list.len(), 1);
                            } else if name == "/Ok" {
                                let exit = exit.context("confirmation did not return from picker")?;
                                return_key = exit.finish(s)?;
                                assert_eq!(return_key, "cancel_label");
                                assert!(s.chara_guid_list.is_empty());
                            } else {
                                assert!(panel.dialog().is_some());
                            }
                        }
                    }
                    "SceneStartDaily" => {}
                    _ => bail!(
                        "unexpected fixture scene {scene_name} at {}",
                        script.position()
                    ),
                }
                if scene_name!="SceneCharacterSelect" {s.active_maids.fill(None);}
            }
            scenes.push(tag);
            let label = script.tag_backup[return_key].clone();
            script.jump_label(&files, op.storage.as_ref(), &label)?;
            script.enabled = true;
            script.exec();
        }
    }
    bail!(
        "business script did not finish: {}, {}, {scenes:?}",
        script.position(),
        script.wait_state()
    )
}

#[test]
#[ignore = "requires COM3D2_ROOT standalone installation"]
fn installed_theater_selection_and_shift_receipts() -> Result<()> {
    let (_tmp,mut rt,c,mut op,files,mut s,mut assets)=daily::installed_panel(&[])?;
    operations::start_day(&mut s)?;
    for (id,name) in [(0,"甲"),(1,"乙")] {
        s.stock.push(id);
        let m=s.maids.entry(id).or_default();
        m.personal=100;
        m.guid_id=format!("theater-{id}");
        m.first_name=name.into();
    }
    operations::open_schedule(&mut s)?;
    operations::choose_staff(&mut s,1,1)?;
    let mut p=Panel {
        base: assets.scene(&mut op,&mut rt.host,c,"SchedulePanel")?,
        category:assets.scene(&mut op,&mut rt.host,c,"TrainingCategory")?,
        unit:assets.scene(&mut op,&mut rt.host,c,"TrainingUnit")?,
        paid_unit:assets.scene(&mut op,&mut rt.host,c,"WorkUnit")?,
        ..Default::default()
    };
    let day=Scene::Daily(daily::Period::Daytime);
    for (slot,shift) in [(0,Shift::Day),(0,Shift::Night),(1,Shift::Night)] {
        for action in [Action::Shift(slot,shift),Action::WorkCategory,Action::OpenRooms(3004)] {
            let widgets=p.populate(None,&mut s,&mut assets,&op,&mut rt.host,c,&files)?;
            let a=press(&p,&widgets,action,day,&assets.operations_layout)?;
            p.activate(a,&mut s,0)?;
        }
        let jobs=operations::paid::rooms(&mut s,slot,shift,3004)?;
        let theater=jobs.iter().find(|r|r.name=="劇場").unwrap().index;
        let previous=shift.work(&s.maids[&slot]);
        let widgets=p.room_widgets(&mut s,slot,shift,3004,&mut assets,&mut op,&mut rt.host,c)?;
        assert_eq!(widgets.iter().filter(|w|w.path.starts_with("room/")).count(),12);
        assert_eq!(widgets.iter().filter(|w|w.path.starts_with("room/") && w.collider.is_some()).count(),1);
        assert!(widgets.iter().any(|w|w.path==format!("room/{theater}") && !w.asset.is_empty()));
        assert!(widgets.iter().any(|w|w.label.as_ref().is_some_and(|l|l.text=="劇場")));
        assert_eq!(shift.work(&s.maids[&slot]),previous);
        let action=press(&p,&widgets,Action::Room(theater),day,&assets.operations_layout)?;
        assert!(operations::paid::assign_room(&mut s,slot,shift,3004,0).is_err());
        p.activate(action,&mut s,0)?;
        assert_eq!(shift.work(&s.maids[&slot]),3004);
        let rooms=operations::paid::rooms(&mut s,slot,shift,3004)?;
        assert_eq!(rooms[theater].count, if slot==1 {2} else {1});
        let widgets=p.room_widgets(&mut s,slot,shift,3004,&mut assets,&mut op,&mut rt.host,c)?;
        let detail="/Window Facility Details/Facility Info/";
        let label=|suffix:&str| widgets.iter().find(|w|w.path.contains(detail) && w.path.ends_with(suffix)).and_then(|w|w.label.as_ref()).map(|l|l.text.as_str());
        assert_eq!(label("/Text Facility Name"),Some("劇場"));
        assert_eq!(label("/Text Number of people"),Some(if slot==1 {"2"} else {"1"}));
        assert_eq!(label("/Text Status"),Some(if slot==1 {"営業可能"} else {"未稼働"}));
        assert!(widgets.iter().any(|w|w.path.contains(detail) && w.path.ends_with("/Image Thumbnail") && !w.asset.is_empty()));
        for (name, expected) in [("parent stars Rank/RawImage",rooms[theater].rank),("parent stars Staff/RawImage",rooms[theater].staff_rank)] {
            assert!(expected>0);
            assert_eq!(widgets.iter().filter(|w|w.path.contains(detail) && w.path.ends_with(name) && !w.asset.is_empty()).count(),expected);
        }
        assert_eq!(widgets.iter().filter(|w|w.path.ends_with("/frame maid icon/maid icon")).count(),1);
        let a=press(&p,&widgets,Action::CloseRooms,day,&assets.operations_layout)?;
        p.activate(a,&mut s,0)?;
    }
    crate::tjs::retail::close_daily(&mut s,true)?;
    let cash=s.player.money;
    operations::settle(&mut s,Shift::Day)?;
    assert_eq!(s.player.schedule_slot[0].noon_success_level,3);
    assert_eq!(s.player.money,cash);
    assert_eq!(s.club_gauge,3);
    assert!(s.maids[&0].work_data.get(&3004).is_none());
    operations::settle(&mut s,Shift::Night)?;
    for id in 0..2 {
        assert_eq!(s.maids[&id].work_data.get(&3004).unwrap().play_count,1);
        assert_eq!(s.maids[&id].others_play_count,2);
        assert!(s.maids[&id].sales>0);
        let b=&s.player.schedule_slot[id].backup_status;
        assert_eq!(b.get("NightBonus_others_play_count").unwrap()-b.get("NoonBonus_others_play_count").unwrap(),2);
    }
    let expected_sales=s.maids[&0].sales+s.maids[&1].sales;
    assert_eq!(s.player.money-cash,expected_sales);
    assert_eq!(expected_sales%1000,0);
    assert_eq!(s.club_gauge,9);
    assert_eq!(i64::from(s.player.salon_evaluation),s.maids[&0].evaluation+s.maids[&1].evaluation);
    let after=s.player.money;
    operations::settle(&mut s,Shift::Night)?;
    assert_eq!(s.player.money,after);
    Ok(())
}

#[test]
#[ignore = "requires COM3D2_ROOT standalone installation"]
fn installed_roster_mouse_drag() -> Result<()> {
    let (_tmp,mut rt,c,mut op,files,mut s,mut assets)=daily::installed_panel(&[])?;
    operations::start_day(&mut s)?;
    for id in 0..2 {
        let m=s.maids.entry(id).or_default();
        m.personal=100;m.guid_id=format!("drag-{id}");s.stock.push(id);
    }
    operations::open_schedule(&mut s)?;
    operations::choose_staff(&mut s,1,1)?;
    operations::assign(&mut s,0,Shift::Day,2003)?;
    let mut p=Panel{base:assets.scene(&mut op,&mut rt.host,c,"SchedulePanel")?,..Default::default()};
    let scene=Scene::Daily(daily::Period::Daytime);
    for (from,to,want) in [(0,1,[Some(1),Some(0),None]),(1,2,[Some(1),None,Some(0)])] {
        p.selection=Some((from,Shift::Day));
        let widgets=p.populate(None,&mut s,&mut assets,&op,&mut rt.host,c,&files)?;
        let pos=|i|->Result<InputFrame>{centered(widgets.iter().find(|w|p.bindings.get(&w.path)==Some(&Action::Staff(i))).unwrap(),0)};
        let mut start=pos(from)?;let end=pos(to)?;
        let mut pressed=None;
        start.buttons_pressed=buttons::MOUSE_LEFT;
        start.buttons_down=buttons::MOUSE_LEFT;
        assert!(!p.roster_drag(&mut s,&widgets,scene,&assets.operations_layout,&start,0,&mut pressed)?);
        let mut move_to=end;
        move_to.buttons_down=buttons::MOUSE_LEFT;
        assert!(p.roster_drag(&mut s,&widgets,scene,&assets.operations_layout,&move_to,0,&mut pressed)?);
        assert!(p.drag_offset(&format!("roster/{from}/MaidStatus")).is_some_and(|d|d[1]>0.));
        assert!(p.roster_drag(&mut s,&widgets,scene,&assets.operations_layout,&end,buttons::MOUSE_LEFT,&mut pressed)?);
        assert_eq!([s.schedule_slot(0),s.schedule_slot(1),s.schedule_slot(2)],want);
        assert_eq!(p.selection,Some((to,Shift::Day)));
        assert_eq!(s.maids[&0].noon_work_id,2003);
        assert!(p.drag.is_none());
    }
    Ok(())
}

#[test]
fn result_panel_fades_independently_of_camera() {
    for kind in [ResultPanel::Work(Shift::Day),ResultPanel::Work(Shift::Night),ResultPanel::Income] {
        let mut panel=Panel::default();
        for (age,alpha) in [(0,0),(125,128),(250,255),(900,255)]{
            assert_eq!(panel.alpha(Scene::Result(kind),1000+age,1000),Some(alpha));
        }
        panel.close=Some(3000);
        for (age,alpha) in [(0,255),(125,128),(250,0),(900,0)] {
            assert_eq!(panel.alpha(Scene::Result(kind),3000+age,1000),Some(alpha));
        }
    }
}
