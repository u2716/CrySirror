// SPDX-License-Identifier: MPL-2.0
//! Bounded native object metadata operations, without a renderer/hit tester.
//! Image/rectangle payloads are native state only; CrySirror owns their layers.
//! Original reset/free policy retires declarations; shared services own resources.
use crate::runtime::{
    CommandContext, Value, constants::RuntimeConstants, forms::codes, globals::ObjectState,
};
use anyhow::{Context, Result, bail, ensure};
/// Original wait ownership, separate from shared interpolation mechanics.
pub(super) struct ObjectWait<'a> {
    pub text: &'a crate::native_string::Environment,
    pub state: &'a mut crate::runtime::wait::VmWait,
    pub form: u32,
    pub stage: i64,
    pub slot: usize,
    /// Original root list index, distinct from a child/EXCALL runtime slot.
    pub object_index: usize,
    pub next_slot: &'a mut usize,
    pub chain: &'a [i32],
    pub scene: &'a str,
    pub finishes: &'a mut Vec<crate::runtime::globals::PendingFrameActionFinish>,
}
pub(super) fn count(ctx: &CommandContext) -> Result<usize> {
    let mut todo = Vec::new();
    for stage in ctx.globals.stage_forms.values() {
        todo.extend(stage.object_lists.values().flatten());
        for w in stage.mwnd_lists.values().flatten() {
            todo.extend(
                w.object_list
                    .iter()
                    .chain(&w.face_list)
                    .chain(&w.button_list),
            );
        }
    }
    let mut count = 0;
    while let Some(object) = todo.pop() {
        count += 1;
        ensure!(count + todo.len() <= 4096, "native object tree budget");
        todo.extend(&object.runtime.child_objects);
    }
    Ok(count)
}
/// Only the first access gives an ambient object its default visibility. CREATE
/// stores its display argument directly, so an object already in use keeps it:
/// Rewrite's title hides full-screen `siro`/`kuro` children with display 0.
fn activate_child_receiver(object:&mut ObjectState,ids:&RuntimeConstants) {
    if !object.used && !object.has_int_prop(ids.obj_disp) {object.set_int_prop(ids,ids.obj_disp,1);}
    object.used=true;
}
pub(super) fn dispatch(
    object: &mut ObjectState,
    ids: &RuntimeConstants,
    images: &mut crate::image_manager::ImageManager,
    media_revision: &mut u64,
    tail: &[i32],
    args: &[Value],
    overload: i64,
    available: usize,
    depth: usize,
    wait: &mut ObjectWait<'_>,
) -> Result<Option<Value>> {
    ensure!(depth <= 16, "native object child depth limit");
    let op = *tail.first().context("native object operation")?;
    if ids.obj_f != 0 && op == ids.obj_f {
        return super::object_flags::dispatch(
            &mut object.runtime.prop_lists.f,
            ids,
            &tail[1..],
            args,
            overload,
        );
    }
    if (ids.obj_frame_action != 0 && op == ids.obj_frame_action)
        || (ids.obj_frame_action_ch != 0 && op == ids.obj_frame_action_ch)
    {
        let position = wait
            .chain
            .len()
            .checked_sub(tail.len())
            .context("native frame-action chain extent")?;
        ensure!(
            wait.chain.get(position) == Some(&op),
            "native frame-action structural owner"
        );
        let object_chain = wait.chain[..position].to_vec();
        let mut end = position + 1;
        let (action, action_tail) = if op == ids.obj_frame_action {
            (&mut object.frame_action, &tail[1..])
        } else if tail.len() >= 3 && tail[1] == ids.elm_array {
            let index = usize::try_from(tail[2].max(0))?;
            ensure!(index < 256, "native frame-action channel budget");
            end += 2;
            // Keep the native lazy channel allocation, before dispatch.
            object.frame_action_ch.resize_with(
                object.frame_action_ch.len().max(index + 1),
                Default::default,
            );
            (&mut object.frame_action_ch[index], &tail[3..])
        } else {
            ensure!(
                tail.len() == 2 && overload == 0,
                "native frame-action list chain/overload"
            );
            return match tail[1] {
                codes::ELM_FRAMEACTIONLIST_RESIZE => {
                    ensure!(args.len() == 1, "native frame-action list resize arguments");
                    let size = usize::try_from(
                        args[0]
                            .as_i64()
                            .context("native frame-action list size")?
                            .max(0),
                    )?;
                    ensure!(size <= 256, "native frame-action channel budget");
                    object.frame_action_ch.resize_with(size, Default::default);
                    Ok(None)
                }
                codes::ELM_FRAMEACTIONLIST_GET_SIZE => {
                    ensure!(args.is_empty(), "native frame-action list size arguments");
                    Ok(Some(Value::Int(object.frame_action_ch.len() as i64)))
                }
                _ => bail!("unsupported native frame-action list operation {}", tail[1]),
            };
        };
        let frame_chain = wait.chain[..end].to_vec();
        return super::object_frame_action::dispatch(
            action,
            action_tail,
            args,
            overload,
            &frame_chain,
            &object_chain,
            wait,
        );
    }
    if op == codes::ELM_OBJECT_CHILD {
        if tail.len() == 2 {
            match tail[1] {
                codes::ELM_OBJECTLIST_GET_SIZE => {
                    ensure!(
                        args.is_empty() && overload == 0,
                        "native child-list size arguments"
                    );
                    activate_child_receiver(object,ids);
                    return Ok(Some(Value::Int(object.runtime.child_objects.len() as i64)));
                }
                codes::ELM_OBJECTLIST_RESIZE => {
                    ensure!(
                        args.len() == 1 && overload == 0,
                        "native child-list resize arguments"
                    );
                    let count = args[0].as_i64().context("native child count")?.max(0) as usize;
                    ensure!(
                        count <= 256
                            && count.saturating_sub(object.runtime.child_objects.len())
                                <= available,
                        "native child count/budget"
                    );
                    object
                        .runtime
                        .child_objects
                        .resize_with(count, Default::default);
                    activate_child_receiver(object,ids);
                    return Ok(None);
                }
                _ => {}
            }
        }
        ensure!(
            tail.len() >= 4 && tail[1] == ids.elm_array,
            "unsupported native child chain {tail:?}"
        );
        let index = usize::try_from(tail[2].max(0))?;
        ensure!(index<256,"native child index budget");
        let growth=(index+1).saturating_sub(object.runtime.child_objects.len());
        ensure!(growth<=available,"native child allocation budget");
        let previous_slot=wait.slot;
        let next_before=*wait.next_slot;
        // Stage only new entries until the complete nested operation succeeds.
        // Existing entries keep their stable native identity across reset/free.
        let mut staged=(growth>0).then(ObjectState::default);
        let child=if growth>0 {staged.as_mut().unwrap()} else {&mut object.runtime.child_objects[index]};
        let old_slot=child.nested_runtime_slot;
        let old_used=child.used;
        let old_disp=child.base.disp;
        let old_explicit=child.has_int_prop(ids.obj_disp);
        activate_child_receiver(child,ids);
        wait.slot=child.ensure_runtime_slot(wait.next_slot);
        let result=dispatch(child,ids,images,media_revision,&tail[3..],args,overload,available-growth,depth+1,wait);
        wait.slot=previous_slot;
        if result.is_err() {
            child.nested_runtime_slot=old_slot;
            child.used=old_used;
            if !old_explicit {child.base.disp=old_disp;child.runtime.explicit_int_props.remove(&ids.obj_disp);}
            *wait.next_slot=next_before;
            return result;
        }
        if growth>0 {
            object.runtime.child_objects.resize_with(index+1,Default::default);
            object.runtime.child_objects[index]=staged.unwrap();
        }
        activate_child_receiver(object,ids);
        return result;
    }
    if object.int_event_by_op(ids, op).is_some() {
        return super::object_event::dispatch(object, ids, tail, args, overload);
    }
    if object.int_event_list_by_op(ids, op).is_some() {
        ensure!(
            tail.len() == 4 && tail[1] == ids.elm_array && overload == 0,
            "native offset event list chain/overload"
        );
        let index = usize::try_from(tail[2]).context("negative native offset event index")?;
        ensure!(index < 256, "native offset event list budget");
        let default = if op == ids.obj_tr_rep_eve { 255 } else { 0 };
        let list = object.int_event_list_by_op_mut(ids, op).unwrap();
        list.resize_with(list.len().max(index + 1), || {
            crysirror_scene::animation::IntEvent::new(default)
        });
        return super::object_event::run(&mut list[index], tail[3], args);
    }
    if object.rep_int_event_list_by_rep_op(ids, op).is_some() {
        let default = if op == ids.obj_tr_rep { 255 } else { 0 };
        let list = object.rep_int_event_list_by_rep_op_mut(ids, op).unwrap();
        if tail.len() == 2 && tail[1] == codes::ELM_INTLIST_RESIZE {
            ensure!(
                overload == 0 && args.len() == 1,
                "native offset-list resize arguments"
            );
            let size =
                usize::try_from(args[0].as_i64().context("native offset-list size")?.max(0))?;
            ensure!(size <= 256, "native offset-list budget");
            ensure!(
                list.iter().skip(size).all(|v| !v.check_event()),
                "active native offset-list destruction profile"
            );
            list.resize_with(size, || crysirror_scene::animation::IntEvent::new(default));
            return Ok(None);
        }
        if tail.len() == 2 && tail[1] == codes::ELM_INTLIST_GET_SIZE {
            ensure!(
                overload == 0 && args.is_empty(),
                "native offset-list size arguments"
            );
            return Ok(Some(Value::Int(list.len() as i64)));
        }
        let index = if tail.len() == 3 && tail[1] == ids.elm_array {
            Some(tail[2])
        } else {
            None
        };
        if let Some(index) = index {
            let index = usize::try_from(index).context("negative native offset index")?;
            ensure!(index < 256, "native offset-list index budget");
            list.resize_with(list.len().max(index + 1), || {
                crysirror_scene::animation::IntEvent::new(default)
            });
            if overload == 1 {
                ensure!(args.len() == 1, "native offset setter arguments");
                let value = i32::try_from(args[0].as_i64().context("native offset value")?)?;
                list[index].set_value(value);
                return Ok(None);
            }
            ensure!(
                overload == 0 && args.is_empty(),
                "native offset getter arguments"
            );
            return Ok(Some(Value::Int(i64::from(list[index].get_value()))));
        }
        bail!("unsupported native offset-list operation {tail:?}");
    }
    if op == ids.obj_all_eve {
        ensure!(
            tail.len() == 2 && args.is_empty() && overload == 0,
            "native ALL_EVE chain/arguments"
        );
        return match tail[1] {
            command if command == ids.elm_allevent_end => {
                object.end_all_events();
                Ok(None)
            }
            command if command == ids.elm_allevent_check => {
                Ok(Some(Value::Int(i64::from(object.any_event_active()))))
            }
            command if command == ids.elm_allevent_wait => {
                if object.any_event_active() {
                    wait.state
                        .wait_object_all_events(wait.form, wait.stage, wait.slot, false);
                }
                Ok(None)
            }
            command => bail!("unsupported native ALL_EVE operation {command}"),
        };
    }
    ensure!(tail.len() == 1, "unsupported native object chain {tail:?}");
    if let Some(value)=super::object_metadata::dispatch(object,ids,op,args,overload,wait.stage,wait.object_index)? {return Ok(value);}
    // These five early native setters precede ObjectOpKind dispatch in the
    // pinned source. Their XY overload leaves Z unchanged (including SCALE).
    let vectors=[
        (codes::ELM_OBJECT_SET_POS,ids.obj_set_pos,[ids.obj_x,ids.obj_y,ids.obj_z]),
        (codes::ELM_OBJECT_SET_SCALE,ids.obj_set_scale,[ids.obj_scale_x,ids.obj_scale_y,ids.obj_scale_z]),
        (codes::ELM_OBJECT_SET_ROTATE,ids.obj_set_rotate,[ids.obj_rotate_x,ids.obj_rotate_y,ids.obj_rotate_z]),
        (codes::ELM_OBJECT_SET_CENTER,ids.obj_set_center,[ids.obj_center_x,ids.obj_center_y,ids.obj_center_z]),
        (codes::ELM_OBJECT_SET_CENTER_REP,ids.obj_set_center_rep,[ids.obj_center_rep_x,ids.obj_center_rep_y,ids.obj_center_rep_z]),
    ];
    if let Some((_,_,fields))=vectors.iter().find(|(canonical,mapped,_)|op==*canonical || *mapped!=0 && op==*mapped) {
        ensure!(matches!(overload,0|1) && args.len()==2+usize::from(overload==1),"native vector setter overload/arguments");
        let values=args.iter().map(|v|i32::try_from(v.as_i64().context("native vector setter integer")?).context("native vector setter integer width")).collect::<Result<Vec<_>>>()?;
        ensure!(fields[..args.len()].iter().all(|id|*id!=0),"native vector setter property layout is not mapped");
        for (field,value) in fields.iter().zip(values) {object.set_int_prop(ids,*field,i64::from(value));}
        return Ok(None);
    }
    let source_clip=op==codes::ELM_OBJECT_SET_SRC_CLIP || ids.obj_set_src_clip!=0 && op==ids.obj_set_src_clip;
    if source_clip || op == codes::ELM_OBJECT_SET_CLIP || ids.obj_set_clip != 0 && op == ids.obj_set_clip {
        ensure!(
            overload == 0 && args.len() == 5,
            "native SET_CLIP/SET_SRC_CLIP overload/arguments"
        );
        let values = args
            .iter()
            .map(|value| {
                i32::try_from(value.as_i64().context("native SET_CLIP/SET_SRC_CLIP integer")?)
                    .context("native SET_CLIP/SET_SRC_CLIP integer width")
            })
            .collect::<Result<Vec<_>>>()?;
        let fields = if source_clip {[ids.obj_src_clip_use,ids.obj_src_clip_left,ids.obj_src_clip_top,ids.obj_src_clip_right,ids.obj_src_clip_bottom]} else {[
            ids.obj_clip_use,
            ids.obj_clip_left,
            ids.obj_clip_top,
            ids.obj_clip_right,
            ids.obj_clip_bottom,
        ]};
        ensure!(
            fields.iter().all(|id| *id != 0),
            "native SET_CLIP/SET_SRC_CLIP property layout is not mapped"
        );
        // Original command only assigns these properties. Native clip events
        // remain independent and the normal collector reads their total values.
        for (field, value) in fields.into_iter().zip(values) {
            object.set_int_prop(ids, field, i64::from(value));
        }
        return Ok(None);
    }
    if matches!(
        op,
        codes::ELM_OBJECT_CREATE_STRING
            | codes::ELM_OBJECT_SET_STRING
            | codes::ELM_OBJECT_GET_STRING
            | codes::ELM_OBJECT_SET_STRING_PARAM
    ) {
        return crate::native_string::dispatch(
            object,
            ids,
            media_revision,
            op,
            args,
            overload,
            wait.text,
        );
    }
    if matches!(
        op,
        codes::ELM_OBJECT_CREATE_MOVIE | codes::ELM_OBJECT_CREATE_MOVIE_LOOP
    ) {
        let positional = args
            .iter()
            .filter(|v| !matches!(v, Value::NamedArg { .. }))
            .collect::<Vec<_>>();
        ensure!(
            (0..=2).contains(&overload) && positional.len() == [1, 2, 4][overload as usize],
            "native object movie create overload/arguments"
        );
        let file = positional[0]
            .as_str()
            .context("native object movie filename")?;
        ensure!(
            !file.is_empty() && file.len() <= 4096 && !file.chars().any(char::is_control),
            "native object movie filename bounds"
        );
        let (mut auto_free, mut real, mut ready) = (true, true, false);
        for arg in args {
            if let Value::NamedArg { id, value } = arg {
                let flag = value
                    .as_i64()
                    .context("native object movie named boolean")?
                    != 0;
                match id {
                    0 => auto_free = flag,
                    1 => real = flag,
                    2 => ready = flag,
                    _ => bail!("unknown native object movie named argument {id}"),
                }
            }
        }
        let values=positional[1..].iter().map(|v|match v {
            Value::Int(n)=>i32::try_from(*n).map(i64::from).context("native movie integer width"),
            _=>bail!("native movie positional integer"),
        }).collect::<Result<Vec<_>>>()?;
        let display=values.first().is_some_and(|n|*n!=0);
        if overload>=2 {ensure!(ids.obj_x!=0 && ids.obj_y!=0,"native movie position layout is not mapped");}
        let revision = media_revision
            .checked_add(1)
            .context("native movie declaration revision exhausted")?;
        object.reinit_like();
        object.used = true;
        object.object_type = 9;
        object.file_name = Some(file.into());
        object.movie.start(
            None,
            op == codes::ELM_OBJECT_CREATE_MOVIE_LOOP,
            auto_free,
            real,
            ready,
        );
        *media_revision = revision;
        object.movie.engine_revision = revision;
        object.base.disp=i64::from(display);
        if overload>=2 {
            object.set_int_prop(ids,ids.obj_x,values[1]);
            object.set_int_prop(ids,ids.obj_y,values[2]);
        }
        return Ok(None);
    }
    if matches!(
        op,
        codes::ELM_OBJECT_GET_SIZE_X
            | codes::ELM_OBJECT_GET_SIZE_Y
            | codes::ELM_OBJECT_GET_SIZE_Z
            | codes::ELM_OBJECT_GET_FILE_NAME
            | codes::ELM_OBJECT_CHECK_MOVIE
            | codes::ELM_OBJECT_PAUSE_MOVIE
            | codes::ELM_OBJECT_RESUME_MOVIE
            | codes::ELM_OBJECT_SET_MOVIE_AUTO_FREE
    ) {
        let size_query = matches!(
            op,
            codes::ELM_OBJECT_GET_SIZE_X
                | codes::ELM_OBJECT_GET_SIZE_Y
                | codes::ELM_OBJECT_GET_SIZE_Z
        );
        ensure!(
            tail.len() == 1 && (overload == 0 || size_query && overload == 1),
            "native object query/control chain {tail:?}, overload {overload}"
        );
        if op == codes::ELM_OBJECT_SET_MOVIE_AUTO_FREE {
            ensure!(args.len() == 1, "native movie auto-free arguments");
            object.movie.auto_free_flag =
                args[0].as_i64().context("native movie auto-free value")? != 0;
            return Ok(None);
        }
        let pattern = if size_query {
            ensure!(
                args.len() == usize::from(overload == 1),
                "native size-query arguments"
            );
            if overload == 1 {
                usize::try_from(args[0].as_i64().context("native size pattern")?.max(0))?
            } else {
                0
            }
        } else {
            ensure!(args.is_empty(), "native object query/control arguments");
            0
        };
        if op == codes::ELM_OBJECT_GET_FILE_NAME {
            return Ok(Some(Value::Str(
                object.file_name.clone().unwrap_or_default(),
            )));
        }
        if op == codes::ELM_OBJECT_CHECK_MOVIE {
            return Ok(Some(Value::Int(i64::from(
                object.object_type == 9 && object.movie.playing,
            ))));
        }
        if matches!(
            op,
            codes::ELM_OBJECT_PAUSE_MOVIE | codes::ELM_OBJECT_RESUME_MOVIE
        ) {
            ensure!(object.object_type == 9, "native movie control target");
            object.movie.pause_flag = op == codes::ELM_OBJECT_PAUSE_MOVIE;
            if !object.movie.pause_flag {
                object.movie.playing = true;
            }
            return Ok(None);
        }
        let size = match object.object_type {
            0 => [0, 0],
            1 => [
                object
                    .rect_param
                    .left
                    .abs_diff(object.rect_param.right)
                    .clamp(1, 32767) as u32,
                object
                    .rect_param
                    .top
                    .abs_diff(object.rect_param.bottom)
                    .clamp(1, 32767) as u32,
            ],
            // Pinned SceneVm GET_SIZE only exposes Rect, Movie, mesh and
            // file albums. STRING has no file and returns zero, even though
            // its retained glyph resource has independent rendering bounds.
            3 => [0, 0],
            2 => {
                let frames = images.album_frames(
                    object
                        .file_name
                        .as_deref()
                        .context("native PCT query filename")?,
                )?;
                if let Some(&id) = frames.get(pattern) {
                    let (width, height) = images
                        .originals
                        .get(id)
                        .copied()
                        .context("native PCT original size")?;
                    [width, height]
                } else {
                    [0, 0]
                }
            }
            9 => {
                if pattern != 0 {
                    return Ok(Some(Value::Int(0)));
                }
                ensure!(
                    object.movie.width > 0 && object.movie.height > 0,
                    "native object movie metadata pending (query inside a callback needs an engine resource continuation)"
                );
                [object.movie.width, object.movie.height]
            }
            other => bail!("native size query unsupported object type {other}"),
        };
        return Ok(Some(Value::Int(if op == codes::ELM_OBJECT_GET_SIZE_Z {
            0
        } else {
            i64::from(size[usize::from(op == codes::ELM_OBJECT_GET_SIZE_Y)])
        })));
    }
    if op == codes::ELM_OBJECT_CREATE || op == codes::ELM_OBJECT_CREATE_RECT {
        let integer = |i: usize| {
            let Some(Value::Int(value))=args.get(i) else {bail!("native object create integer");};
            i32::try_from(*value).map(i64::from).context("native object create integer width")
        };
        if op == codes::ELM_OBJECT_CREATE {
            ensure!(
                (0..=3).contains(&overload) && [1, 2, 4, 5][overload as usize] == args.len(),
                "native PCT create overload/arguments"
            );
            let file = args[0].as_str().context("native PCT filename")?;
            ensure!(
                !file.contains('?') && file.len() <= 4096,
                "native PCT tonecurve/name profile"
            );
            let values=(1..args.len()).map(integer).collect::<Result<Vec<_>>>()?;
            let patno=values.get(3).copied().unwrap_or(0);
            let display=values.first().is_some_and(|n|*n!=0);
            if !file.is_empty() {
                ensure!(ids.obj_patno!=0 && (overload<2 || ids.obj_x!=0 && ids.obj_y!=0),"native PCT property layout is not mapped");
            }
            if !file.is_empty() {
                images.load_g00(
                    file,
                    usize::try_from(patno).context("negative native PATNO")?,
                )?;
            }
            object.reinit_like();
            object.used = true;
            if !file.is_empty() {
                object.object_type = 2;
                object.file_name = Some(file.into());
                object.base.disp=i64::from(display);
                if overload >= 2 {
                    object.set_int_prop(ids, ids.obj_x, values[1]);
                    object.set_int_prop(ids, ids.obj_y, values[2]);
                }
                object.set_int_prop(ids, ids.obj_patno, patno);
            }
        } else {
            ensure!(
                (0..=2).contains(&overload) && [8, 9, 11][overload as usize] == args.len(),
                "native RECT create overload/arguments"
            );
            let values=(0..args.len()).map(integer).collect::<Result<Vec<_>>>()?;
            ensure!(overload<2 || ids.obj_x!=0 && ids.obj_y!=0,"native RECT position layout is not mapped");
            let bounds=[values[0],values[1],values[2],values[3]];
            let c=[values[4],values[5],values[6],values[7]].map(|v|v.clamp(0,255));
            object.reinit_like();
            object.used = true;
            object.object_type = 1;
            let p = &mut object.rect_param;
            p.left = bounds[0];
            p.top = bounds[1];
            p.right = bounds[2];
            p.bottom = bounds[3];
            p.color_argb = (c[3] << 24) | (c[0] << 16) | (c[1] << 8) | c[2];
            object.base.disp=i64::from(values.get(8).is_some_and(|n|*n!=0));
            if overload >= 2 {
                object.set_int_prop(ids, ids.obj_x, values[9]);
                object.set_int_prop(ids, ids.obj_y, values[10]);
            }
        }
        return Ok(None);
    }
    if ids.object_op_tables().fixed_int_prop(op).is_some() {
        if overload == 1 {
            ensure!(args.len() == 1, "native object property setter arguments");
            object.set_int_prop(
                ids,
                op,
                args[0].as_i64().context("native object property value")?,
            );
            return Ok(None);
        }
        ensure!(
            overload == 0 && args.is_empty(),
            "native object property getter arguments"
        );
        return Ok(Some(Value::Int(object.get_int_prop(ids, op))));
    }
    bail!("unsupported native embedded object operation {op}")
}

#[cfg(test)]
mod string_tests {
    use super::*;
    #[test]
    fn complete_vector_setter_family_preserves_omitted_z_and_rejects_before_any_mutation() {
        let mut ctx=CommandContext::new("asset".into());let env=crate::native_string::Environment::new(&ctx);
        let mut object=ObjectState::default();let mut next_slot=1000;let mut finishes=Vec::new();
        let mut owner=ObjectWait{object_index:0,text:&env,state:&mut ctx.wait,form:0,stage:1,slot:0,next_slot:&mut next_slot,chain:&[],scene:"fixture",finishes:&mut finishes};
        for (op,fields) in [
            (codes::ELM_OBJECT_SET_POS,[ctx.ids.obj_x,ctx.ids.obj_y,ctx.ids.obj_z]),
            (codes::ELM_OBJECT_SET_SCALE,[ctx.ids.obj_scale_x,ctx.ids.obj_scale_y,ctx.ids.obj_scale_z]),
            (codes::ELM_OBJECT_SET_ROTATE,[ctx.ids.obj_rotate_x,ctx.ids.obj_rotate_y,ctx.ids.obj_rotate_z]),
            (codes::ELM_OBJECT_SET_CENTER,[ctx.ids.obj_center_x,ctx.ids.obj_center_y,ctx.ids.obj_center_z]),
            (codes::ELM_OBJECT_SET_CENTER_REP,[ctx.ids.obj_center_rep_x,ctx.ids.obj_center_rep_y,ctx.ids.obj_center_rep_z]),
        ] {
            let mut invoke=|object:&mut ObjectState,args:&[Value],overload|dispatch(object,&ctx.ids,&mut ctx.images,&mut ctx.native_media_revision,&[op],args,overload,4096,0,&mut owner);
            invoke(&mut object,&[Value::Int(1),Value::Int(2),Value::Int(77)],1).unwrap();
            invoke(&mut object,&[Value::Int(-3),Value::Int(-4)],0).unwrap();
            assert_eq!(fields.map(|field|object.get_int_prop(&ctx.ids,field)),[-3,-4,77]);
            let old=bincode::serialize(&object).unwrap();
            assert!(invoke(&mut object,&[Value::Int(4),Value::Int(i64::MAX)],0).is_err());
            assert_eq!(bincode::serialize(&object).unwrap(),old);
            assert!(invoke(&mut object,&[Value::Int(4),Value::Int(5)],1).is_err());
            assert_eq!(bincode::serialize(&object).unwrap(),old);
        }
    }
    #[test]
    fn clip_validation_and_unknown_layout_leave_the_original_object_unchanged() {
        let mut ctx = CommandContext::new("asset".into());
        let env = crate::native_string::Environment::new(&ctx);
        let mut object = ObjectState::default();
        let before = bincode::serialize(&object).unwrap();
        let mut next_slot = 1000;
        let mut finishes = Vec::new();
        let mut owner = ObjectWait {
            object_index:0,            text: &env,
            state: &mut ctx.wait,
            form: 0,
            stage: 1,
            slot: 0,
            next_slot: &mut next_slot,
            chain: &[],
            scene: "test",
            finishes: &mut finishes,
        };
        let invalid = [
            Value::Int(1),
            Value::Int(0),
            Value::Int(0),
            Value::Int(i64::MAX),
            Value::Int(10),
        ];
        assert!(
            dispatch(
                &mut object,
                &ctx.ids,
                &mut ctx.images,
                &mut ctx.native_media_revision,
                &[codes::ELM_OBJECT_SET_CLIP],
                &invalid,
                0,
                4096,
                0,
                &mut owner
            )
            .is_err()
        );
        assert_eq!(bincode::serialize(&object).unwrap(), before);
        ctx.ids.obj_clip_use = 0;
        let valid = [
            Value::Int(1),
            Value::Int(0),
            Value::Int(0),
            Value::Int(10),
            Value::Int(10),
        ];
        let error = dispatch(
            &mut object,
            &ctx.ids,
            &mut ctx.images,
            &mut ctx.native_media_revision,
            &[codes::ELM_OBJECT_SET_CLIP],
            &valid,
            0,
            4096,
            0,
            &mut owner,
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("property layout"));
        assert_eq!(bincode::serialize(&object).unwrap(), before);
    }
    #[test]
    fn source_clip_validation_and_unknown_layout_leave_the_original_object_unchanged() {
        let mut ctx = CommandContext::new("asset".into());
        let env = crate::native_string::Environment::new(&ctx);
        let mut object = ObjectState::default();
        let before = bincode::serialize(&object).unwrap();
        let mut next_slot = 1000;
        let mut finishes = Vec::new();
        let mut owner = ObjectWait {
            object_index:0,            text: &env,
            state: &mut ctx.wait,
            form: 0,
            stage: 1,
            slot: 0,
            next_slot: &mut next_slot,
            chain: &[],
            scene: "test",
            finishes: &mut finishes,
        };
        let invalid = [
            Value::Int(1),
            Value::Int(0),
            Value::Int(0),
            Value::Int(i64::MAX),
            Value::Int(10),
        ];
        assert!(
            dispatch(
                &mut object,
                &ctx.ids,
                &mut ctx.images,
                &mut ctx.native_media_revision,
                &[codes::ELM_OBJECT_SET_SRC_CLIP],
                &invalid,
                0,
                4096,
                0,
                &mut owner
            )
            .is_err()
        );
        assert_eq!(bincode::serialize(&object).unwrap(), before);
        ctx.ids.obj_src_clip_use = 0;
        let valid = [
            Value::Int(1),
            Value::Int(0),
            Value::Int(0),
            Value::Int(10),
            Value::Int(10),
        ];
        let error = dispatch(
            &mut object,
            &ctx.ids,
            &mut ctx.images,
            &mut ctx.native_media_revision,
            &[codes::ELM_OBJECT_SET_SRC_CLIP],
            &valid,
            0,
            4096,
            0,
            &mut owner,
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("property layout"));
        assert_eq!(bincode::serialize(&object).unwrap(), before);
    }
    #[test]
    fn invalid_string_replacement_preserves_callback_bearing_subtree() {
        let mut ctx = CommandContext::new("asset".into());
        let env = crate::native_string::Environment::new(&ctx);
        let mut object = ObjectState::default();
        object.runtime.child_objects.push(ObjectState::default());
        object.runtime.child_objects[0].frame_action.cmd_name = "pending-finish".into();
        let old = bincode::serialize(&object).unwrap();
        let mut next_slot = 1000;
        let mut finishes = Vec::new();
        let mut ownership = ObjectWait {
            object_index:0,            text: &env,
            state: &mut ctx.wait,
            form: 0,
            stage: 1,
            slot: 52,
            next_slot: &mut next_slot,
            chain: &[],
            scene: "test",
            finishes: &mut finishes,
        };
        let result = dispatch(
            &mut object,
            &ctx.ids,
            &mut ctx.images,
            &mut ctx.native_media_revision,
            &[codes::ELM_OBJECT_CREATE_STRING],
            &[],
            0,
            4096,
            0,
            &mut ownership,
        );
        assert!(format!("{:#}", result.unwrap_err()).contains("string arguments"));
        assert_eq!(bincode::serialize(&object).unwrap(), old);
    }
}

#[cfg(test)]
mod child_receiver_tests {
    use super::*;
    #[test]
    fn child_access_keeps_created_visibility_and_defaults_ambient_objects() {
        let ids=RuntimeConstants::default();
        let mut hidden=ObjectState::default();
        hidden.used=true;
        hidden.base.disp=0;
        activate_child_receiver(&mut hidden,&ids);
        assert_eq!(hidden.base.disp,0);
        let mut ambient=ObjectState::default();
        activate_child_receiver(&mut ambient,&ids);
        assert!(ambient.used);
        assert_eq!(ambient.base.disp,1);
    }
}
