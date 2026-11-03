// SPDX-License-Identifier: MPL-2.0
use crate::runtime::CommandContext;
const EXCALL_LOCAL_NS_XOR: u32 = 0x4000;
pub(crate) fn is_stage_form_id(ctx: &CommandContext, form_id: i32) -> bool {
    let primary = ctx.ids.form_global_stage as i32;
    let canonical = crate::runtime::forms::codes::FORM_GLOBAL_STAGE as i32;
    let primary_local = if primary != 0 { primary ^ 0x4000 } else { 0 };
    let canonical_local = canonical ^ 0x4000;
    (primary != 0 && form_id == primary)
        || form_id == canonical
        || (primary != 0 && form_id == primary_local)
        || form_id == canonical_local
        || form_id == crate::runtime::constants::global_form::STAGE_DEFAULT as i32
        || form_id == crate::runtime::constants::global_form::BACK as i32
        || form_id == crate::runtime::constants::global_form::FRONT as i32
        || form_id == crate::runtime::constants::global_form::NEXT as i32
        || form_id == crate::runtime::constants::global_form::STAGE_ALIAS_37 as i32
        || form_id == crate::runtime::constants::global_form::STAGE_ALIAS_38 as i32
}

fn normal_stage_form_id(ctx: &CommandContext) -> u32 {
    if ctx.ids.form_global_stage != 0 {
        ctx.ids.form_global_stage
    } else {
        crate::runtime::forms::codes::FORM_GLOBAL_STAGE
    }
}

pub(crate) fn stage_storage_form_id(ctx: &CommandContext, raw_form_id: i32) -> u32 {
    let normal = normal_stage_form_id(ctx);
    let local = normal ^ EXCALL_LOCAL_NS_XOR;
    let canonical_local = crate::runtime::forms::codes::FORM_GLOBAL_STAGE ^ EXCALL_LOCAL_NS_XOR;
    if raw_form_id == local as i32 || raw_form_id == canonical_local as i32 {
        local
    } else {
        // BACK/FRONT/NEXT and old aliases all address the ordinary global
        // stage container, just as in C++ C_elm_excall only gets its own
        // container through the EXCALL child forwarding path.
        normal
    }
}

pub(crate) fn current_stage_form_id(ctx: &CommandContext) -> u32 {
    ctx.vm_call
        .as_ref()
        .and_then(|meta| meta.element.first().copied())
        .map(|raw| stage_storage_form_id(ctx, raw))
        .unwrap_or_else(|| normal_stage_form_id(ctx))
}

pub fn cd_text_current_mwnd(
    ctx: &mut CommandContext,
    text: &str,
    read_flag: i64,
) -> anyhow::Result<bool> {
    let accepted = crate::message::append(ctx, text, read_flag)?;
    if accepted && !text.is_empty() {
        ctx.set_current_read_flag_for_skip(ctx.current_scene_no.unwrap_or(-1), read_flag);
    }
    Ok(accepted)
}
pub fn cd_name_current_mwnd(ctx: &mut CommandContext, name: &str) -> anyhow::Result<bool> {
    crate::message::name(ctx, name)
}
pub fn restore_stage_form_backends_after_load(
    _: &mut CommandContext,
    st: &mut crate::runtime::globals::StageFormState,
) {
    st.rect_layers.clear();
}
/// Native selector parsing plus CPU object state. The original interpreter still
/// owns decoded values/argument overloads; shared layers are extracted afterward.
pub fn dispatch(ctx: &mut CommandContext, args: &[crate::runtime::Value]) -> anyhow::Result<bool> {
    use crate::runtime::{Value, forms::codes};
    use anyhow::{Context, ensure};
    let call = ctx
        .vm_call
        .clone()
        .context("native stage missing call metadata")?;
    let chain = &call.element;
    let root = *chain.first().context("empty stage chain")?;
    let (stage_idx, mut p) = if root == codes::ELM_GLOBAL_BACK {
        (0, 1)
    } else if root == codes::ELM_GLOBAL_FRONT {
        (1, 1)
    } else if root == codes::ELM_GLOBAL_NEXT {
        (2, 1)
    } else {
        ensure!(
            chain.get(1) == Some(&ctx.ids.elm_array),
            "native stage requires array selector"
        );
        (i64::from(*chain.get(2).context("native stage index")?), 3)
    };
    ensure!(
        (0..=2).contains(&stage_idx),
        "native stage index unsupported"
    );
    let family = stage_storage_form_id(ctx, root);
    let text_environment=crate::native_string::Environment::new(ctx);
    let op = *chain.get(p).context("native stage operation missing")?;
    if op == codes::ELM_STAGE_MWND {
        ensure!(family==normal_stage_form_id(ctx),"native EXCALL message-window identity requires an independent shared UI mapping");
        if chain.len() == p + 4 && chain[p + 1] == ctx.ids.elm_array
            && crate::message::voice::global_operation(chain[p + 3]).is_some() {
            return crate::message::voice::dispatch(ctx, chain[p + 3], args);
        }
        ensure!(stage_idx == 1, "native MWND bridge supports FRONT only");
        if chain.get(p + 3) == Some(&codes::ELM_MWND_BUTTON) {
            ensure!(
                chain.len() >= p + 7
                    && chain[p + 1] == ctx.ids.elm_array
                    && chain[p + 4] == ctx.ids.elm_array,
                "native MWND button chain"
            );
            let window = usize::try_from(chain[p + 2]).context("native MWND button window")?;
            let index = usize::try_from(chain[p + 5]).context("native MWND button index")?;
            ensure!(
                window == 0 && index < 256,
                "native WAKU button profile/budget"
            );
            let op = chain[p + 6];
            if chain.len() == p + 7
                && matches!(
                    op,
                    codes::ELM_OBJECT_GET_BUTTON_HIT_STATE
                        | codes::ELM_OBJECT_GET_BUTTON_STATE
                        | codes::ELM_OBJECT_GET_BUTTON_REAL_STATE
                        | codes::ELM_OBJECT_GET_BUTTON_NO
                        | codes::ELM_OBJECT_GET_BUTTON_PUSHKEEP
                        | codes::ELM_OBJECT_GET_BUTTON_ALPHA_TEST
                )
            {
                ensure!(
                    call.al_id == 0 && args.is_empty(),
                    "native button getter arguments"
                );
                let w = crate::message::buttons::native(ctx);
                let object = w.and_then(|w| w.button_list.get(index));
                let value = match op {
                    codes::ELM_OBJECT_GET_BUTTON_HIT_STATE => {
                        i64::from(w.is_some_and(|w| w.open) && object.is_some_and(|o| o.button.hit))
                    }
                    codes::ELM_OBJECT_GET_BUTTON_STATE => object.map_or(0, |o| o.button.state),
                    codes::ELM_OBJECT_GET_BUTTON_REAL_STATE => {
                        object.map_or(0, |o| crate::message::buttons::real_state(ctx, index, o))
                    }
                    codes::ELM_OBJECT_GET_BUTTON_NO => object.map_or(0, |o| o.button.button_no),
                    codes::ELM_OBJECT_GET_BUTTON_PUSHKEEP => {
                        i64::from(object.is_some_and(|o| o.button.push_keep))
                    }
                    _ => i64::from(object.is_some_and(|o| o.button.alpha_test)),
                };
                ctx.push(Value::Int(value));
                return Ok(true);
            }
            crate::message::initialize_profile(ctx)?;
            let globals = &mut *ctx.globals;
            let state = globals
                .stage_forms
                .get_mut(&family)
                .context("native WAKU stage missing")?;

            let state = &mut **state;
            let next_slot = state.next_nested_object_slot.entry(stage_idx)
                .or_insert(state.backend_slot_base + 100_000);
            let o = state.mwnd_lists.get_mut(&stage_idx)
                .and_then(|list| list.get_mut(window))
                .and_then(|w| w.button_list.get_mut(index))
                .context("native WAKU button missing")?;
            let slot = o.ensure_runtime_slot(next_slot);
            let value = super::passive_object::dispatch(
                o,
                &ctx.ids,
                &mut ctx.images,
                &mut ctx.native_media_revision,
                &chain[p + 6..],
                args,
                call.al_id,
                0,
                0,
                &mut super::passive_object::ObjectWait {
                    object_index:index,
                    text:&text_environment,
                    state: &mut ctx.wait, form: family, stage: stage_idx, slot, next_slot,
                    chain, scene: ctx.current_scene_name.as_deref().unwrap_or_default(),
                    finishes: &mut globals.pending_frame_action_finishes,
                },
            )?;
            if let Some(value) = value {
                ctx.push(value);
            } else if call.ret_form != 0 {
                ctx.push(Value::Int(0));
            }
            return Ok(true);
        }
        if chain.get(p + 3) == Some(&codes::ELM_MWND_OBJECT) {
            ensure!(
                chain.len() >= p + 7
                    && chain.get(p + 1) == Some(&ctx.ids.elm_array)
                    && chain.get(p + 4) == Some(&ctx.ids.elm_array),
                "native MWND embedded object chain {chain:?}"
            );
            let window = usize::try_from(chain[p + 2]).context("native MWND index")?;
            let child = usize::try_from(chain[p + 5]).context("native MWND object index")?;
            ensure!(window < 16 && child < 256, "native MWND object count limit");
            let total = super::passive_object::count(ctx)?;
            let old = ctx
                .globals
                .stage_forms
                .get(&family)
                .and_then(|s| s.mwnd_lists.get(&stage_idx))
                .and_then(|w| w.get(window))
                .map_or(0, |w| w.object_list.len());
            let growth = (child + 1).saturating_sub(old);
            ensure!(total + growth <= 4096, "native MWND object tree budget");
            let globals = &mut *ctx.globals;
            let state = globals
                .stage_forms
                .entry(family)
                .or_default();

            let state = &mut **state;
            let next_slot = state.next_nested_object_slot.entry(stage_idx)
                .or_insert(state.backend_slot_base + 100_000);
            let windows = state.mwnd_lists
                .entry(stage_idx)
                .or_default();
            windows.resize_with(windows.len().max(window + 1), Default::default);
            let objects = &mut windows[window].object_list;
            objects.resize_with(objects.len().max(child + 1), Default::default);
            let slot = objects[child].ensure_runtime_slot(next_slot);
            let value = super::passive_object::dispatch(
                &mut objects[child],
                &ctx.ids,
                &mut ctx.images,
                &mut ctx.native_media_revision,
                &chain[p + 6..],
                args,
                call.al_id,
                4096 - total - growth,
                0,
                &mut super::passive_object::ObjectWait {
                    object_index:child,
                    text:&text_environment,
                    state: &mut ctx.wait, form: family, stage: stage_idx, slot, next_slot,
                    chain, scene: ctx.current_scene_name.as_deref().unwrap_or_default(),
                    finishes: &mut globals.pending_frame_action_finishes,
                },
            )?;
            if let Some(value) = value {
                ctx.push(value);
            } else if call.ret_form != 0 {
                ctx.push(Value::Int(0));
            }
            return Ok(true);
        }
        if chain.len() == p + 4 && chain[p + 1] == ctx.ids.elm_array && call.al_id == 0 {
            let window = usize::try_from(chain[p + 2]).context("native MWND query window")?;
            ensure!(window < 16, "native MWND query window budget");
            if crate::message::query_metadata(ctx, window, chain[p + 3], args)? {
                return Ok(true);
            }
        }
        ensure!(
            chain.get(p + 1) == Some(&ctx.ids.elm_array)
                && chain.get(p + 2) == Some(&0)
                && chain.len() == p + 4,
            "native MWND bridge supports MWND[0] only"
        );
        return crate::message::dispatch(ctx, chain[p + 3], args);
    }
    if op == codes::ELM_STAGE_CREATE_OBJECT {
        ensure!(chain.len()==p+1 && call.al_id==0 && args.len()==1,"native CREATE_OBJECT chain/arguments");
        let count = args
            .first()
            .and_then(Value::as_i64)
            .context("CREATE_OBJECT count")?;
        ensure!((0..=4096).contains(&count), "native object count");
        let total=super::passive_object::count(ctx)?;
        let old=ctx.globals.stage_forms.get(&family).and_then(|s|s.object_lists.get(&stage_idx)).map_or(0,|v|v.len());
        ensure!(total.saturating_add((count as usize).saturating_sub(old))<=4096,"native CREATE_OBJECT tree budget");
        ctx.globals
            .stage_forms
            .entry(family)
            .or_default()
            .ensure_object_list(stage_idx, count as usize);
        if let Some(state)=ctx.globals.stage_forms.get_mut(&family) {
            let state=&mut **state;
            if state.backend_slot_base!=0 {
                if let Some(objects)=state.object_lists.get_mut(&stage_idx) {
                    for (index,object) in objects.iter_mut().enumerate() { object.backend_runtime_slot=Some(state.backend_slot_base+index); }
                }
            }
        }
        if call.ret_form != 0 {
            ctx.push(Value::Int(0));
        }
        return Ok(true);
    }
    ensure!(
        op == codes::ELM_STAGE_OBJECT,
        "unsupported native stage member {op}"
    );
    p += 1;
    ensure!(
        chain.get(p) == Some(&ctx.ids.elm_array),
        "native object selector"
    );
    let index = *chain.get(p + 1).context("native object index")?;
    ensure!(index >= 0, "negative native object index");
    p += 2;
    let index = index as usize;
    ensure!(index < 4096, "native object slot budget");
    let total = super::passive_object::count(ctx)?;
    let old = ctx
        .globals
        .stage_forms
        .get(&family)
        .and_then(|s| s.object_lists.get(&stage_idx))
        .map_or(0, |v| v.len());
    let growth = (index + 1).saturating_sub(old);
    ensure!(total + growth <= 4096, "native object tree budget");
    let globals = &mut *ctx.globals;
    let state = globals
        .stage_forms
        .entry(family)
        .or_default();

    let state = &mut **state;
    let next_slot = state.next_nested_object_slot.entry(stage_idx)
        .or_insert(state.backend_slot_base + 100_000);
    let list = &mut **state.object_lists
        .entry(stage_idx)
        .or_default();
    list.resize_with(list.len().max(index + 1), Default::default);
    if state.backend_slot_base!=0 { list[index].backend_runtime_slot=Some(state.backend_slot_base+index); }
    let slot = list[index].runtime_slot_or(index);
    let value = super::passive_object::dispatch(
        &mut list[index],
        &ctx.ids,
        &mut ctx.images,
        &mut ctx.native_media_revision,
        &chain[p..],
        args,
        call.al_id,
        4096 - total - growth,
        0,
        &mut super::passive_object::ObjectWait {
            object_index:index,
                    text:&text_environment,
            state: &mut ctx.wait, form: family, stage: stage_idx, slot, next_slot,
                    chain, scene: ctx.current_scene_name.as_deref().unwrap_or_default(),
                    finishes: &mut globals.pending_frame_action_finishes,
        },
    )?;
    if let Some(value) = value {
        ctx.push(value);
    } else if call.ret_form != 0 {
        ctx.push(Value::Int(0));
    }
    Ok(true)
}

pub(crate) use super::stage_lifecycle::{ensure_stage_form_allocated, validate_stage_form_allocation, queue_stage_form_finishes, free_stage_form};
