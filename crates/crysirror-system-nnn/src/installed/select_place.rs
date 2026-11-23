// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original CCommonSelectPlace (mode 22) as a skinned modal page.
//!
//! MUGEN2 legacy evidence: constructor 509020, Init 508980, Calcu 507d00,
//! Print 5087d0, FinalExitRoutine 507530. Buttons are a CCommonButtonGroup
//! with the "place" prefix (45e180); pixels, hit testing and animation are the
//! shared Title button player. Init computes exist/enable words from
//! GetVarData (533c20); FinalExitRoutine writes `selectedPlaceVarName` =
//! selection + 1 and resumes the script (SetYoyaku).
use super::menu::Page;
use super::*;
use crate::config::Names;

/// 533c20 GetVarData on the raw GetVarNumber result. An absent name yields
/// -1, which reads as unsigned 0xffffffff and takes the literal branch
/// (`& 0x8000` -> `& 0x7fff`), i.e. a nonzero 32767.
fn native_var(variables: &crate::expression::Variables, id: Option<u16>) -> i32 {
    match id {
        Some(id) => variables.read_stored(id),
        None => 0x7fff,
    }
}

fn variable(
    table: &Names,
    config: &crate::native_bootstrap::NativeConfiguration,
    key: &[u8],
) -> Option<u16> {
    table
        .parameter(key)
        .and_then(|name| config.variable_names.index(name))
}

pub(super) fn prepare(
    table: &Names,
    config: &crate::native_bootstrap::NativeConfiguration,
    variables: &crate::expression::Variables,
) -> Result<Page> {
    let n = |key: &[u8], default| -> Result<i32> { Ok(table.integer(key)?.unwrap_or(default)) };
    // Profiles this page does not draw fail visibly instead of hiding UI.
    ensure!(
        n(b"charaFlag", 0)? == 0,
        "NNN SelectPlace character overlay requires its native chara painter"
    );
    ensure!(
        n(b"autoPicLoad", 0)? == 0,
        "NNN SelectPlace autoPicLoad requires its native layer-tag loader"
    );
    ensure!(
        table.parameter(b"onButtonVarName").is_none(),
        "NNN SelectPlace onButtonVarName requires its native hover variable writer"
    );
    let count = usize::try_from(n(b"placeNumber", 1)?)?;
    let mut spec =
        crate::native_title::NativeTitleSpec::from_button_group(table, config, "place", count)?;
    let appear = n(b"placeAppearFlag", 0)? != 0;
    let select = n(b"selectFlag", 0)? != 0;
    for (index, button) in spec.buttons.iter_mut().enumerate() {
        let exists = !appear
            || native_var(
                variables,
                variable(table, config, format!("placeAppearVarName{}", index + 1).as_bytes()),
            ) != 0;
        let selectable = !select
            || native_var(
                variables,
                variable(table, config, format!("placeSelectOkVarName{}", index + 1).as_bytes()),
            ) != 0;
        button.exists = exists;
        button.enabled = exists && selectable;
    }
    Ok(Page {
        background: None,
        slots: vec![],
        pages: 1,
        columns: 1,
        origin: [0; 2],
        step: [0; 2],
        sound: 0,
        error_sound: -1,
        skin: Some(spec),
        image: None,
        table: None,
    })
}

/// FinalExitRoutine target: `selectedPlaceVarName` (default 場所選択番号).
pub(super) fn result_variable(
    table: &Names,
    config: &crate::native_bootstrap::NativeConfiguration,
) -> Option<u16> {
    let default = encoding_rs::SHIFT_JIS.encode("場所選択番号").0;
    let name = table
        .parameter(b"selectedPlaceVarName")
        .unwrap_or(default.as_ref());
    config.variable_names.index(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{VariableProfile, Variables};
    #[test]
    fn select_place_flags_use_native_getvardata_and_group_inheritance() {
        let names = Names { tokens: vec![] };
        let config = crate::native_bootstrap::NativeConfiguration {
            resources: crate::native_resources::ResourceNames::new("").unwrap(),
            setup: names.clone(),
            main_control: None,
            chart_setup: None,
            variable_names: Names::parse(b"appear1 result", false).unwrap(),
            project: names.clone(),
            commands: names.clone(),
            functions: names,
            initial: None,
            module_sha256: "authored".into(),
            profile: VariableProfile::Legacy1000,
            scripts: crate::XorProfile::default(),
        };
        let table = Names::parse(
            b"placeNumber 3 placeAppearFlag 1 placeAppearVarName1 appear1 \
              placeAppearVarName2 unknown selectFlag 1 selectedPlaceVarName result \
              place1FileName a place1SizeX 125 place1SizeY 327 place1PrintX 56 \
              place2PrintX 249 place3PrintX 427",
            false,
        )
        .unwrap();
        let mut variables = Variables::fresh(VariableProfile::Legacy1000);
        let page = prepare(&table, &config, &variables).unwrap();
        let buttons = &page.skin.as_ref().unwrap().buttons;
        // appear1 = 0 hides place 1. An unknown name is GetVarNumber -1,
        // which GetVarData reads as 0x7fff; no placeAppearVarName3 likewise.
        // selectFlag without placeSelectOkVarName stays selectable.
        assert_eq!(
            buttons.iter().map(|b| (b.exists, b.enabled)).collect::<Vec<_>>(),
            vec![(false, false), (true, true), (true, true)]
        );
        // CSuperButtonSetup chains size/source from the previous button.
        assert_eq!(buttons[2].size, [125, 327]);
        assert_eq!(buttons[2].origin, [427, 0]);
        assert_eq!(buttons[1].filename.as_deref(), Some(b"a".as_slice()));
        variables.write_stored(0, 1);
        let page = prepare(&table, &config, &variables).unwrap();
        assert!(page.skin.unwrap().buttons[0].enabled);
        assert_eq!(result_variable(&table, &config), Some(1));
    }
}
