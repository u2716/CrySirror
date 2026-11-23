// SPDX-License-Identifier: MPL-2.0
//! Original stage-list lifetime/finish ordering, backed by engine-owned layers.
//! No original backend/window/UI manager is adopted.
use crate::runtime::{
    CommandContext,
    globals::{
        ObjectFrameActionState, ObjectState, PendingFrameActionFinish, StageFormState, WorldState,
    },
};
use anyhow::{Result, ensure};
const EXCALL_LOCAL_NS_XOR: u32 = 0x4000;
const EXCALL_BACKEND_SLOT_BASE: usize = 1_000_000;
fn normal_stage_form_id(ctx: &CommandContext) -> u32 {
    super::stage::stage_storage_form_id(ctx, super::codes::FORM_GLOBAL_STAGE as i32)
}
const INIDEF_OBJECT_CNT: usize = 256;
const INIMAX_OBJECT_CNT: usize = 1024;
const INIDEF_BTN_GROUP_CNT: usize = 4;
const INIMAX_BTN_GROUP_CNT: usize = 256;
const INIDEF_WORLD_CNT: usize = 1;
const INIMAX_WORLD_CNT: usize = 256;
const INIDEF_EFFECT_CNT: usize = 4;
const INIMAX_EFFECT_CNT: usize = 256;
const INIDEF_QUAKE_CNT: usize = 16;
const INIMAX_QUAKE_CNT: usize = 256;

fn parse_i64_local(s: &str) -> Option<i64> {
    let t = s.trim().trim_matches('"');
    t.parse::<i64>().ok()
}

fn cfg_usize_or(ctx: &CommandContext, key: &str, default_value: usize, max_value: usize) -> usize {
    ctx.tables
        .gameexe
        .as_ref()
        .and_then(|cfg| cfg.get_usize(key))
        .unwrap_or(default_value)
        .min(max_value)
}

fn cfg_usize_or_any(
    ctx: &CommandContext,
    keys: &[&str],
    default_value: usize,
    max_value: usize,
) -> usize {
    if let Some(cfg) = ctx.tables.gameexe.as_ref() {
        for key in keys {
            if let Some(v) = cfg.get_usize(key) {
                return v.min(max_value);
            }
        }
    }
    default_value.min(max_value)
}

fn stage_object_use_flags(ctx: &CommandContext, object_cnt: usize) -> Vec<bool> {
    let mut out = vec![true; object_cnt];
    let Some(cfg) = ctx.tables.gameexe.as_ref() else {
        return out;
    };
    for i in 0..object_cnt {
        if let Some(v) = cfg
            .get_indexed_field("OBJECT", i, "USE")
            .and_then(parse_i64_local)
        {
            out[i] = v != 0;
        }
    }
    out
}

fn extend_stage_object_list_with_use_flags(
    st: &mut StageFormState,
    stage_idx: i64,
    object_use: &[bool],
) {
    let entry = st.object_lists.entry(stage_idx).or_default();
    let old_len = entry.len();
    if old_len < object_use.len() {
        entry.reserve(object_use.len() - old_len);
        for _ in &object_use[old_len..] {
            // C++ use_flag belongs to the list slot definition, not the
            // mutable object payload.  A freshly initialized slot has
            // type=NONE regardless of whether the slot itself is enabled.
            entry.push(ObjectState::default());
        }
    }

    let slot_use = st.object_slot_use.entry(stage_idx).or_default();
    if slot_use.len() < object_use.len() {
        slot_use.extend_from_slice(&object_use[slot_use.len()..]);
    }
}

fn extend_list_with_default<T: Default>(list: &mut Vec<T>, cnt: usize) {
    if list.len() < cnt {
        list.extend((0..(cnt - list.len())).map(|_| T::default()));
    }
}

fn extend_world_list_with_indices(list: &mut Vec<WorldState>, cnt: usize) {
    while list.len() < cnt {
        list.push(WorldState::new(list.len() as i32));
    }
}

fn ensure_stage_form_initialized_from_gameexe(
    ctx: &CommandContext,
    form_id: u32,
    st: &mut StageFormState,
) {
    let normal_form_id = normal_stage_form_id(ctx);
    st.backend_slot_base = if form_id == (normal_form_id ^ EXCALL_LOCAL_NS_XOR) {
        EXCALL_BACKEND_SLOT_BASE
    } else {
        0
    };
    if st.initialized_from_gameexe {
        return;
    }

    // Original C++ initializes both stage-list variants from the same Gp_ini
    // list counts. The ordinary list uses use_ini=true; C_elm_excall uses
    // use_ini=false, which only changes per-object USE/template flags.
    // C++ constructs BACK/FRONT/NEXT eagerly before any script can touch a stage.
    // This Rust port creates the state lazily, so BACK may already contain objects
    // by the time this initialization runs.  Preserve the original invariant by
    // sizing every stage object list to at least the largest list that already
    // exists, then use the C++ object.use defaults for newly materialized peers.
    let cfg_object_cnt = cfg_usize_or(ctx, "OBJECT.CNT", INIDEF_OBJECT_CNT, INIMAX_OBJECT_CNT);
    let existing_object_cnt = st
        .object_lists
        .values()
        .map(|list| list.len())
        .max()
        .unwrap_or(0)
        .min(INIMAX_OBJECT_CNT);
    let object_cnt = cfg_object_cnt
        .max(existing_object_cnt)
        .min(INIMAX_OBJECT_CNT);
    let use_ini = st.backend_slot_base == 0;
    let object_use = if use_ini {
        stage_object_use_flags(ctx, object_cnt)
    } else {
        // C_elm_excall constructs m_stage_list with use_ini=false.
        // C_elm_object_list::_init() therefore enables every fixed object slot
        // instead of inheriting #OBJECT.*.USE from the gameplay stage.
        vec![true; object_cnt]
    };
    let group_cnt = cfg_usize_or_any(
        ctx,
        &["OBJBTNGROUP.CNT", "BUTTON.GROUP.CNT"],
        INIDEF_BTN_GROUP_CNT,
        INIMAX_BTN_GROUP_CNT,
    );
    let mwnd_cnt = cfg_usize_or(ctx, "MWND.CNT", 2, 256);
    let world_cnt = cfg_usize_or(ctx, "WORLD.CNT", INIDEF_WORLD_CNT, INIMAX_WORLD_CNT);
    let effect_cnt = cfg_usize_or(ctx, "EFFECT.CNT", INIDEF_EFFECT_CNT, INIMAX_EFFECT_CNT);
    let quake_cnt = cfg_usize_or(ctx, "QUAKE.CNT", INIDEF_QUAKE_CNT, INIMAX_QUAKE_CNT);

    let backend_slot_base = st.backend_slot_base;
    for stage_idx in 0..3 {
        extend_stage_object_list_with_use_flags(st, stage_idx, &object_use);
        if backend_slot_base != 0
            && let Some(objects) = st.object_lists.get_mut(&stage_idx)
        {
            for (idx, obj) in objects.iter_mut().enumerate() {
                obj.backend_runtime_slot = Some(backend_slot_base + idx);
            }
        }

        let groups = st.group_lists.entry(stage_idx).or_default();
        extend_list_with_default(groups, group_cnt);

        let mwnds = st.mwnd_lists.entry(stage_idx).or_default();
        extend_list_with_default(mwnds, mwnd_cnt);

        let worlds = st.world_lists.entry(stage_idx).or_default();
        extend_world_list_with_indices(worlds, world_cnt);

        let effects = st.effect_lists.entry(stage_idx).or_default();
        extend_list_with_default(effects, effect_cnt);

        let quakes = st.quake_lists.entry(stage_idx).or_default();
        extend_list_with_default(quakes, quake_cnt);
    }

    st.initialized_from_gameexe = true;
}

pub(crate) fn validate_stage_form_allocation(ctx: &CommandContext, form: u32) -> Result<()> {
    let current = super::passive_object::count(ctx)?;
    let existing = ctx.globals.stage_forms.get(&form);
    let count = cfg_usize_or(ctx, "OBJECT.CNT", INIDEF_OBJECT_CNT, INIMAX_OBJECT_CNT).max(
        existing.map_or(0, |s| {
            s.object_lists.values().map(|v| v.len()).max().unwrap_or(0)
        }),
    );
    let growth = (0..3)
        .map(|i| {
            count.saturating_sub(
                existing
                    .and_then(|s| s.object_lists.get(&i))
                    .map_or(0, |v| v.len()),
            )
        })
        .sum::<usize>();
    ensure!(
        current.saturating_add(growth) <= 4096,
        "native EXCALL stage allocation exceeds object budget"
    );
    Ok(())
}
pub(crate) fn ensure_stage_form_allocated(ctx: &mut CommandContext, form: u32) -> Result<()> {
    validate_stage_form_allocation(ctx, form)?;
    let mut stage = ctx.globals.stage_forms.remove(&form).unwrap_or_default();
    ensure_stage_form_initialized_from_gameexe(ctx, form, &mut stage);
    ctx.globals.stage_forms.insert(form, stage);
    Ok(())
}
pub(crate) fn queue_stage_form_finishes(ctx: &mut CommandContext, form_id: u32) {
    fn collect_object_finishes(
        obj: &ObjectState,
        object_chain: &[i32],
        out: &mut Vec<PendingFrameActionFinish>,
    ) {
        let push = |fa: &ObjectFrameActionState,
                    frame_action_chain: Vec<i32>,
                    out: &mut Vec<PendingFrameActionFinish>| {
            if fa.cmd_name.is_empty() {
                return;
            }
            out.push(PendingFrameActionFinish {
                frame_action_chain,
                object_chain: Some(object_chain.to_vec()),
                snapshot: fa.clone(),
                reinit_after_finish: false,
                scn_name: fa.scn_name.clone(),
                cmd_name: fa.cmd_name.clone(),
                end_time: fa.end_time,
                args: fa.args.clone(),
            });
        };

        let mut root_fa = object_chain.to_vec();
        root_fa.push(crate::runtime::forms::codes::elm_value::OBJECT_FRAME_ACTION);
        push(&obj.frame_action, root_fa, out);

        for (ch_idx, fa) in obj.frame_action_ch.iter().enumerate() {
            let mut ch = object_chain.to_vec();
            ch.push(crate::runtime::forms::codes::elm_value::OBJECT_FRAME_ACTION_CH);
            ch.push(crate::runtime::forms::codes::ELM_ARRAY);
            ch.push(ch_idx as i32);
            push(fa, ch, out);
        }

        for (child_idx, child) in obj.runtime.child_objects.iter().enumerate() {
            let mut child_chain = object_chain.to_vec();
            child_chain.push(crate::runtime::forms::codes::elm_value::OBJECT_CHILD);
            child_chain.push(crate::runtime::forms::codes::ELM_ARRAY);
            child_chain.push(child_idx as i32);
            collect_object_finishes(child, &child_chain, out);
        }
    }

    let Some(st) = ctx.globals.stage_forms.get(&form_id) else {
        return;
    };
    let mut pending = Vec::new();

    for stage_idx in 0..3 {
        if let Some(objects) = st.object_lists.get(&stage_idx) {
            for (obj_idx, obj) in objects.iter().enumerate() {
                let chain = vec![
                    form_id as i32,
                    crate::runtime::forms::codes::ELM_ARRAY,
                    stage_idx as i32,
                    crate::runtime::forms::codes::elm_value::STAGE_OBJECT,
                    crate::runtime::forms::codes::ELM_ARRAY,
                    obj_idx as i32,
                ];
                collect_object_finishes(obj, &chain, &mut pending);
            }
        }

        if let Some(mwnds) = st.mwnd_lists.get(&stage_idx) {
            for (mwnd_idx, mwnd) in mwnds.iter().enumerate() {
                for (selector, list) in [
                    (
                        crate::runtime::forms::codes::elm_value::MWND_BUTTON,
                        &mwnd.button_list,
                    ),
                    (
                        crate::runtime::forms::codes::elm_value::MWND_FACE,
                        &mwnd.face_list,
                    ),
                    (
                        crate::runtime::forms::codes::elm_value::MWND_OBJECT,
                        &mwnd.object_list,
                    ),
                ] {
                    for (obj_idx, obj) in list.iter().enumerate() {
                        let chain = vec![
                            form_id as i32,
                            crate::runtime::forms::codes::ELM_ARRAY,
                            stage_idx as i32,
                            crate::runtime::forms::codes::elm_value::STAGE_MWND,
                            crate::runtime::forms::codes::ELM_ARRAY,
                            mwnd_idx as i32,
                            selector,
                            crate::runtime::forms::codes::ELM_ARRAY,
                            obj_idx as i32,
                        ];
                        collect_object_finishes(obj, &chain, &mut pending);
                    }
                }
            }
        }

        if let Some(items) = st.btnselitem_lists.get(&stage_idx) {
            for (item_idx, item) in items.iter().enumerate() {
                for (obj_idx, obj) in item.object_list.iter().enumerate() {
                    let chain = vec![
                        form_id as i32,
                        crate::runtime::forms::codes::ELM_ARRAY,
                        stage_idx as i32,
                        crate::runtime::forms::codes::elm_value::STAGE_BTNSELITEM,
                        crate::runtime::forms::codes::ELM_ARRAY,
                        item_idx as i32,
                        crate::runtime::forms::codes::elm_value::BTNSELITEM_OBJECT,
                        crate::runtime::forms::codes::ELM_ARRAY,
                        obj_idx as i32,
                    ];
                    collect_object_finishes(obj, &chain, &mut pending);
                }
            }
        }
    }

    ctx.globals.pending_frame_action_finishes.extend(pending);
}

pub(crate) fn free_stage_form(ctx: &mut CommandContext, form: u32) {
    // Finish callbacks were drained by the original SceneVm runner. Deleting
    // declarations retires shared layers and Media on the next owner publish;
    // the adapter never owns a second sprite/UI backend to clear here.
    ctx.globals.stage_forms.remove(&form);
    if ctx
        .globals
        .focused_stage_mwnd
        .is_some_and(|(id, _, _)| id == form)
    {
        ctx.globals.focused_stage_mwnd = None;
    }
    if ctx
        .globals
        .focused_stage_group
        .is_some_and(|(id, _, _)| id == form)
    {
        ctx.globals.focused_stage_group = None;
    }
}
