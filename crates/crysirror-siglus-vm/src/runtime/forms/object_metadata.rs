// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Pinned OBJECT metadata, hints and reset/free policy. Resources are retained
//! engine declarations; the adapter's normal publication retires their leases.
//! No image, font, composition, hit-testing or audio implementation lives here.
use super::codes;
use crate::runtime::{Value, constants::RuntimeConstants, globals::ObjectState};
use anyhow::{Context, Result, ensure};

/// `None` means another native operation owns this code. `Some(None)` is a
/// completed void operation; queries return the original native value.
pub(super) fn dispatch(
    object: &mut ObjectState,
    ids: &RuntimeConstants,
    op: i32,
    args: &[Value],
    overload: i64,
    stage: i64,
    object_index: usize,
) -> Result<Option<Option<Value>>> {
    let is = |canonical, mapped| op == canonical || mapped != 0 && op == mapped;
    let init = is(codes::ELM_OBJECT_INIT, ids.obj_init);
    let free = is(codes::ELM_OBJECT_FREE, ids.obj_free);
    let param = is(codes::ELM_OBJECT_INIT_PARAM, ids.obj_init_param);
    let exists = is(codes::ELM_OBJECT_EXIST_TYPE, ids.obj_exist_type);
    let hints = op == codes::ELM_OBJECT_ADD_HINTS;
    let clear = op == codes::ELM_OBJECT_CLEAR_HINTS;
    if !(init || free || param || exists || hints || clear
        || op == codes::ELM_OBJECT_GET_TYPE || op == codes::ELM_OBJECT_GET_ELEMENT_NAME)
    { return Ok(None); }
    ensure!(overload == 0, "native object metadata overload");
    if hints {
        ensure!(args.len() <= 32, "native object hint argument budget");
        let mut value = None;
        for arg in args {
            let Value::NamedArg { id: 0, value: v } = arg else {
                anyhow::bail!("native object ADD_HINTS requires named no_event integer");
            };
            value = Some(i32::try_from(v.as_i64().context("native object hint integer")?)
                .context("native object hint integer width")? != 0);
        }
        if let Some(value) = value { object.base.no_event_hint = value; }
        return Ok(Some(None));
    }
    ensure!(args.is_empty(), "native object metadata arguments");
    let result = if init {
        // Original id-mapped early INIT runs reinit before the later dead
        // ObjectOpKind arm. Its mutable `used` marker becomes true; immutable
        // per-list USE is held elsewhere. Preserve receiver runtime identities.
        object.reinit_like();
        object.used = true;
        None
    } else if free {
        // free_type(false) only clears this payload. CHILD, frame actions,
        // ordinary parameters and slot identities deliberately survive.
        object.init_type_like();
        object.used = false;
        None
    } else if param {
        // Original init_param_like resets actions/events/F/buttons, but does
        // not reconstruct the type resource, its captured font or CHILD.
        object.init_param_like();
        None
    } else if clear {
        object.base.no_event_hint = false;
        None
    } else if exists {
        Some(Value::Int(i64::from(object.object_type != 0)))
    } else if op == codes::ELM_OBJECT_GET_TYPE {
        Some(Value::Int(object.object_type))
    } else {
        // Child recursion in the pinned dispatcher retains original obj_u;
        // it is not the backend/nested runtime slot and not the last child id.
        Some(Value::Str(format!("stage[{stage}].object[{object_index}]")))
    };
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::globals::ObjectFrameActionState;
    fn invoke(o:&mut ObjectState,op:i32,args:&[Value])->Result<Option<Value>> {
        Ok(dispatch(o,&RuntimeConstants::default(),op,args,0,1,52)?.expect("known native op"))
    }
    fn object()->ObjectState {
        let mut o=ObjectState::default();o.used=true;o.object_type=2;
        o.file_name=Some("picture".into());o.base.x=31;o.base.disp=1;
        o.base.no_event_hint=true;o.base.wipe_erase=1;
        o.backend_runtime_slot=Some(1_000_052);o.nested_runtime_slot=Some(100_002);
        o.frame_action.cmd_name="callback".into();o.frame_action_ch.push(ObjectFrameActionState::default());
        o.runtime.prop_lists.f=std::sync::Arc::new(vec![11,22]);
        let mut child=ObjectState::default();child.object_type=1;child.used=true;
        o.runtime.child_objects.push(child);o
    }
    #[test]
    fn free_preserves_child_actions_flags_and_identity_but_releases_only_type() {
        let mut o=object();let old_f=o.runtime.prop_lists.f.clone();
        invoke(&mut o,codes::ELM_OBJECT_FREE,&[]).unwrap();
        assert_eq!(o.object_type,0);assert!(!o.used);assert_eq!(o.thumb_save_no,-1);
        assert!(o.file_name.is_none());assert_eq!(o.runtime.child_objects.len(),1);
        assert_eq!(o.runtime.child_objects[0].object_type,1);
        assert_eq!(o.frame_action.cmd_name,"callback");assert_eq!(o.frame_action_ch.len(),1);
        assert_eq!((o.base.x,o.base.disp,o.base.wipe_erase),(31,1,1));
        assert!(std::sync::Arc::ptr_eq(&old_f,&o.runtime.prop_lists.f));
        assert_eq!((o.backend_runtime_slot,o.nested_runtime_slot),(Some(1_000_052),Some(100_002)));
    }
    #[test]
    fn init_param_keeps_type_and_children_while_init_retires_subtree_and_keeps_receiver_identity() {
        let mut o=object();invoke(&mut o,codes::ELM_OBJECT_INIT_PARAM,&[]).unwrap();
        assert_eq!(o.object_type,2);assert_eq!(o.file_name.as_deref(),Some("picture"));
        assert_eq!(o.runtime.child_objects.len(),1);assert!(o.used);
        assert_eq!(o.base.x,0);assert_eq!(o.base.disp,0);assert!(!o.base.no_event_hint);
        assert!(o.frame_action.cmd_name.is_empty());assert!(o.frame_action_ch.is_empty());
        assert!(o.runtime.prop_lists.f.is_empty());
        invoke(&mut o,codes::ELM_OBJECT_INIT,&[]).unwrap();
        assert_eq!(o.object_type,0);assert!(o.runtime.child_objects.is_empty());
        assert!(o.used);assert_eq!(o.thumb_save_no,-1);
        assert_eq!((o.backend_runtime_slot,o.nested_runtime_slot),(Some(1_000_052),Some(100_002)));
    }
    #[test]
    fn queries_use_raw_type_and_original_root_index_even_for_nested_runtime_identity() {
        let mut o=object();
        assert!(matches!(invoke(&mut o,codes::ELM_OBJECT_GET_TYPE,&[]).unwrap(),Some(Value::Int(2))));
        assert!(matches!(invoke(&mut o,codes::ELM_OBJECT_EXIST_TYPE,&[]).unwrap(),Some(Value::Int(1))));
        assert!(matches!(invoke(&mut o,codes::ELM_OBJECT_GET_ELEMENT_NAME,&[]).unwrap(),Some(Value::Str(s)) if s=="stage[1].object[52]"));
        invoke(&mut o,codes::ELM_OBJECT_FREE,&[]).unwrap();
        assert!(matches!(invoke(&mut o,codes::ELM_OBJECT_EXIST_TYPE,&[]).unwrap(),Some(Value::Int(0))));
    }
    #[test]
    fn hints_validate_entire_native_signature_before_mutating_and_last_named_value_wins() {
        let mut o=object();let named=|id,n|Value::NamedArg{id,value:Box::new(Value::Int(n))};
        invoke(&mut o,codes::ELM_OBJECT_ADD_HINTS,&[named(0,0),named(0,1)]).unwrap();
        assert!(o.base.no_event_hint);
        let before=bincode::serialize(&o).unwrap();
        for args in [vec![named(0,0),named(1,1)],vec![Value::Int(0)],vec![named(0,i64::MAX)]] {
            assert!(invoke(&mut o,codes::ELM_OBJECT_ADD_HINTS,&args).is_err());
            assert_eq!(bincode::serialize(&o).unwrap(),before);
        }
        assert!(invoke(&mut o,codes::ELM_OBJECT_INIT_PARAM,&[Value::Int(1)]).is_err());
        assert_eq!(bincode::serialize(&o).unwrap(),before);
        invoke(&mut o,codes::ELM_OBJECT_CLEAR_HINTS,&[]).unwrap();assert!(!o.base.no_event_hint);
    }
}
