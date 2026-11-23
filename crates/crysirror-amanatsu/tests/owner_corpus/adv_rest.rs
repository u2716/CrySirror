// SPDX-License-Identifier: MPL-2.0
use crysirror_amanatsu::{
    adv::{Effect, Var, Wait},
    level::Level,
    talk::{self, Step, Talk},
};

/// Drive the actual 200-ms DitherOut command from the shared morning
/// scenario against the authored ADVCore canvas and normal Talk tick.
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn morning_dither_out_publishes_the_player_material_tween() {
    let mut unity = super::unity();
    let key = unity.bundle_files().into_iter().find(|key| {
        key.starts_with(&format!("bundle:{}#", talk::ADV_BUNDLE)) && !key.ends_with(".resS")
    }).unwrap();
    let mut level = Level::root_of(&mut unity, &key, "ADVCore").unwrap();
    let mut talk = Talk::start(&mut unity, &mut level, 0, "Test", 1, vec!["morning".into()]).unwrap();
    let index = talk::scenario_index(&mut unity, 0).unwrap();
    let rows = talk::load_scenario(&mut unity, &index, "morning").unwrap();
    let row = rows.into_iter().find(|row| row.name() == "CharaDitherOut").unwrap();
    assert_eq!(row.args, ["-1", "0.2"]);
    talk.program.open("morning", vec![row]);
    talk.advance(&mut unity, &mut level, 0);
    let mut values = Vec::new();
    for time in [50, 100, 150, 200] {
        for step in talk.tick(&mut unity, &mut level, time) {
            match step {
                Step::Chara(Effect::Dither { actor: -1, value }) => values.push(value),
                Step::Fault(error) => panic!("{error}"),
                _ => {}
            }
        }
    }
    assert_eq!(values, [0.75, 0.5, 0.25, 0., 1.]);
}

/// Exercise the installed shared scenario, both its native fade canvas and
/// the timed commands that keep running across Open(e1-approach).
#[test]
#[ignore = "needs AMANATSU_ROOT"]
fn rest_event_runs_background_commands_while_showing_the_approach() {
    let mut unity = super::unity();
    let key = unity.bundle_files().into_iter().find(|key| {
        key.starts_with(&format!("bundle:{}#", talk::ADV_BUNDLE)) && !key.ends_with(".resS")
    }).unwrap();
    let mut level = Level::root_of(&mut unity, &key, "ADVCore").unwrap();
    let mut talk = Talk::start(&mut unity, &mut level, 0, "Test", 1, vec!["rest".into()]).unwrap();
    let card_path = "DefaultData/0/chara/female/AL_F_00.png";
    talk.heroine_card = Some(card_path.into());
    let card = crysirror_amanatsu::card::Card::parse(&unity.read_all(card_path).unwrap()).unwrap();
    let (personality, first_name, last_name) = card.parameter().unwrap();
    talk.program.vars.insert("Judge".into(), Var::Int(1));
    talk.program.vars.insert("G_MapID".into(), Var::Int(0));
    let back = level.find("ADVCore/Canvas_Main/FadeBack").unwrap();
    let mut steps = talk.advance(&mut unity, &mut level, 0);
    assert!(matches!(talk.wait, Wait::Text { .. }));
    steps.extend(talk.click(&mut unity, &mut level, None, 0));
    assert!(matches!(talk.wait, Wait::Choice(_)));
    // Choose the first cloned uGUI control through the real hit ancestry.
    let template = level.find("ADVCore/Canvas_Main/background_Choices/Choices/Choice").unwrap();
    let parent = level.scene.nodes[template].parent.unwrap();
    let first = *level.scene.nodes[parent].children.iter().rev().nth(1).unwrap();
    steps.extend(talk.click(&mut unity, &mut level, Some(first), 0));
    assert_eq!(talk.selected, Some(0));
    let mut prefab = None;
    let mut move_samples = Vec::new();
    let mut opacity = Vec::new();
    let mut approached = false;
    let mut gaze = None;
    let mut cleared_cloth = false;
    let mut shown = None;
    for frame in 1..=1100u64 {
        let now = frame * 16;
        steps.extend(talk.tick(&mut unity, &mut level, now));
        if matches!(&talk.wait, Wait::Text { .. }) {
            if talk.program.name == "e1-approach" {
                if !approached {
                    approached = true;
                    shown = Some(talk.wait.clone());
                }
                if let Some(ref text) = shown {
                    assert_eq!(&talk.wait, text);
                }
                let pc = talk.program.pc;
                steps.extend(talk.click(&mut unity, &mut level, None, now));
                assert_eq!(talk.program.pc, pc, "ClickNext must block manual advance");
                assert_eq!(Some(&talk.wait), shown.as_ref());
            } else if talk.program.name == "e1" {
                break;
            } else {
                steps.extend(talk.click(&mut unity, &mut level, None, now));
            }
        }
        for step in steps.drain(..) {
            match step {
                Step::Fault(error) => panic!("{error}"),
                Step::Chara(Effect::MoveOffset { offset, .. }) => move_samples.push(offset[2]),
                Step::Chara(Effect::ClothHideClear(0)) => cleared_cloth = true,
                Step::Chara(Effect::EyePattern { actor, pattern }) => {
                    let states = crysirror_amanatsu::face::eye_type_states(&mut unity).unwrap();
                    let state = &states[usize::try_from(pattern).unwrap()];
                    assert_eq!(state.look_type, 1);
                    let offset = crysirror_amanatsu::face::eye_offset(state, 0, glam::Vec3::ZERO, glam::Vec3::Z, glam::Vec3::new(0.2, 0., 1.));
                    assert!(offset[0] > 0. && offset[1].abs() < 1e-6);
                    gaze = Some((actor, pattern));
                }
                Step::Chara(Effect::EventCg { bundle, asset }) => {
                    assert!(unity.bundle_asset(&format!("lib/{bundle}"), &asset, crysirror_unity::class_id::GAME_OBJECT).unwrap().is_some());
                    prefab = Some(asset);
                }
                _ => {}
            }
        }
        if !move_samples.is_empty() {
            opacity.push(level.states[&back].group_alpha.unwrap());
        }
    }
    assert_eq!(prefab.as_deref(), Some("EventCG_ChairTalk_000_00"));
    assert!(approached);
    assert_eq!(talk.program.name, "e1", "TaskWait must resume the parent");
    assert_eq!(talk.program.vars["NowID"], Var::Int(0));
    assert_eq!(gaze, Some((0, 1)));
    assert!(cleared_cloth);
    assert_eq!(talk.program.vars["H_Personality"], Var::Int(personality));
    assert_eq!(talk.program.vars["H_ChaVoice"], Var::Str(format!("c{personality:02}")));
    assert_eq!(talk.program.substitute("[H姓] [H名]"), format!("{last_name} {first_name}"));
    assert_eq!(move_samples.first(), Some(&-16.));
    assert!(move_samples.iter().any(|&z| z > -9. && z < -7.));
    assert_eq!(move_samples.last(), Some(&0.));
    assert_eq!(opacity.first(), Some(&1.));
    assert!(opacity.iter().any(|&alpha| alpha > 0.4 && alpha < 0.6));
    assert_eq!(opacity.last(), Some(&0.));
    assert!(!talk.log.iter().any(|row| ["Format ", "Vector ", "Regulate ", "Task ", "TaskWait ", "FadeDelay ", "CharaLerpVectorMove "].iter().any(|op| row.contains(&format!("ADV {op}")))));
}
