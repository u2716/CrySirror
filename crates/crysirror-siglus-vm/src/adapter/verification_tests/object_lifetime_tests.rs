//! Actual original opcodes, callback runner, framework restore and engine layers.
use super::*;
use crate::runtime::{constants::{cd,fm,op},forms::codes as c};
fn chain(root:i32,tail:&[i32])->Vec<i32> { [&[c::ELM_GLOBAL_FRONT,c::ELM_STAGE_OBJECT,c::ELM_ARRAY,root],tail].concat() }
fn object(vm:&SiglusVm,index:usize)->&crate::runtime::globals::ObjectState {
 vm.state.vm.as_ref().unwrap().ctx.globals.stage_forms.values().find_map(|s|s.object_lists.get(&1)?.get(index)).unwrap()
}
fn count(vm:&SiglusVm)->i64 {vm.state.vm.as_ref().unwrap().ctx.globals.int_lists.get(&(c::ELM_GLOBAL_G as u32)).and_then(|v|v.first()).copied().unwrap_or(0)}
fn pack_with_counter(mut scene:crate::fixture::SceneBuilder)->Vec<u8> {
 let offset=scene.code.len();
 for (id,form) in [c::FM_FRAMEACTION,c::FM_OBJECT].into_iter().enumerate() {scene.code.push(cd::DEC_PROP);scene.i32(form);scene.i32(id as i32);}
 scene.code.push(cd::ARG);let global=[c::ELM_GLOBAL_G,c::ELM_ARRAY,0];
 scene.element(&global);scene.command(&global,0,&[],fm::INT);scene.int(1);
 scene.code.push(cd::OPERATE_2);scene.i32(fm::INT);scene.i32(fm::INT);scene.code.push(op::PLUS);
 scene.code.push(cd::COMMAND);for v in [1,1,fm::INT,0,fm::VOID] {scene.i32(v);}
 scene.code.push(cd::RETURN);scene.i32(0);
 let mut bytes=scene.finish();let table=bytes.len();bytes.extend((offset as i32).to_le_bytes());
 let index=bytes.len();bytes.extend(0i32.to_le_bytes());bytes.extend(5i32.to_le_bytes());
 let name=bytes.len();for w in "pulse".encode_utf16() {bytes.extend(w.to_le_bytes());}
 for (field,value) in [(19,table),(20,1),(21,index),(22,1),(23,name),(24,1)] {bytes[field*4..field*4+4].copy_from_slice(&(value as i32).to_le_bytes());}
 crate::fixture::pack_scene(&bytes)
}
#[test]
fn free_keeps_original_action_and_child_string_then_rebuild_retires_both_after_restore() {
 let (mut vm,mut host)=machine(false);let mut s=crate::fixture::SceneBuilder::default();
 s.command(&chain(52,&[c::ELM_OBJECT_CREATE_RECT]),2,&[0,0,20,10,255,0,0,255,1,0,0],fm::VOID);
 s.command_string(&chain(52,&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,3,c::ELM_OBJECT_CREATE_STRING]),2,"A",&[1,2,3],fm::VOID);
 s.command(&chain(52,&[c::ELM_OBJECT_INIT_PARAM]),0,&[],fm::VOID);
 s.command(&chain(52,&[c::ELM_OBJECT_DISP]),1,&[1],fm::VOID);
 for op in [c::ELM_OBJECT_GET_TYPE,c::ELM_OBJECT_EXIST_TYPE] {s.command(&chain(52,&[op]),0,&[],fm::INT);}
 s.command(&chain(52,&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,3,c::ELM_OBJECT_GET_ELEMENT_NAME]),0,&[],fm::STR);
 s.element(&chain(52,&[c::ELM_OBJECT_FRAME_ACTION,c::ELM_FRAMEACTION_START]));s.int(1000);s.string("pulse");s.code.push(cd::COMMAND);for v in [0,2,fm::STR,fm::INT,0,fm::VOID] {s.i32(v);}
 s.command(&chain(52,&[c::ELM_OBJECT_FREE]),0,&[],fm::VOID);
 s.command(&[c::ELM_GLOBAL_TIMEWAIT],0,&[10],fm::VOID);
 s.command(&chain(52,&[c::ELM_OBJECT_CREATE_RECT]),2,&[0,0,8,6,0,255,0,255,1,9,8],fm::VOID);
 s.code.push(cd::EOF);host.files.insert("Asset/Scene.pck".into(),pack_with_counter(s));
 vm.step(&mut host,InputFrame::default(),5000).unwrap();
 assert_eq!(object(&vm,52).object_type,0);assert_eq!(object(&vm,52).runtime.child_objects.len(),4);
 assert_eq!(object(&vm,52).frame_action.cmd_name,"pulse");assert_eq!(vm.object_strings.len(),1);
 assert_eq!(vm.diagnostics()["int_stack"],serde_json::json!([1,1]));
 assert_eq!(vm.diagnostics()["str_stack"],serde_json::json!(["stage[1].object[52]"]));
 assert!(host.engine.frame([320,180]).unwrap().iter().any(|p|matches!(p.paint,crysirror_scene::layer::Paint::GlyphRun{..})));
 host.now=5;vm.step(&mut host,InputFrame::default(),5000).unwrap();assert_eq!(count(&vm),1);
 let snapshot=vm.snapshot().unwrap();let engine=host.engine.clone();
 for _ in 0..2 {
  host.now=10;assert_eq!(vm.step(&mut host,InputFrame::default(),5000).unwrap(),VmYield::Halted);
  assert_eq!(count(&vm),2,"reset did not synthesize another finish callback");
  let o=object(&vm,52);assert_eq!(o.object_type,1);assert!(o.runtime.child_objects.is_empty());assert!(o.frame_action.cmd_name.is_empty());
  assert_eq!(vm.object_strings.len(),0,"removed subtree released actual font recipe leases");
  assert!(!host.engine.frame([320,180]).unwrap().iter().any(|p|matches!(p.paint,crysirror_scene::layer::Paint::GlyphRun{..})));
  vm.restore(&snapshot).unwrap();host.engine=engine.clone();host.now=5;
 }
}
#[test]
fn child_receiver_lazily_activates_and_shrink_retires_visible_shared_payloads() {
 let (mut vm,mut host)=machine(false);let mut s=crate::fixture::SceneBuilder::default();
 s.command(&chain(52,&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,3,c::ELM_OBJECT_CREATE_RECT]),2,&[0,0,8,6,0,255,0,255,1,2,3],fm::VOID);
 s.command(&chain(52,&[c::ELM_OBJECT_CHILD,c::ELM_OBJECTLIST_GET_SIZE]),0,&[],fm::INT);
 s.command(&chain(52,&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,-1,c::ELM_OBJECT_GET_ELEMENT_NAME]),0,&[],fm::STR);
 s.command(&[c::ELM_GLOBAL_TIMEWAIT],0,&[10],fm::VOID);
 s.command(&chain(52,&[c::ELM_OBJECT_CHILD,c::ELM_OBJECTLIST_RESIZE]),0,&[1],fm::VOID);
 s.command(&chain(52,&[c::ELM_OBJECT_INIT]),0,&[],fm::VOID);s.code.push(cd::EOF);
 host.files.insert("Asset/Scene.pck".into(),crate::fixture::pack_scene(&s.finish()));
 vm.step(&mut host,InputFrame::default(),5000).unwrap();
 let parent=object(&vm,52);assert!(parent.used);assert_eq!(parent.base.disp,1);assert_eq!(parent.runtime.child_objects.len(),4);
 assert_eq!(vm.diagnostics()["int_stack"],serde_json::json!([4]));
 assert_eq!(vm.diagnostics()["str_stack"],serde_json::json!(["stage[1].object[52]"]));
 assert!(host.engine.frame([320,180]).unwrap().iter().any(|p|matches!(p.paint,crysirror_scene::layer::Paint::Solid{color} if color==[0,255,0,255])));
 host.now=10;vm.step(&mut host,InputFrame::default(),5000).unwrap();
 assert!(object(&vm,52).runtime.child_objects.is_empty());assert!(vm.state.published.is_empty());
}

#[test]
fn original_named_hint_arguments_and_clear_update_native_policy_without_changing_payload() {
 let (mut vm,mut host)=machine(false);let mut s=crate::fixture::SceneBuilder::default();
 s.command(&chain(52,&[c::ELM_OBJECT_CREATE_RECT]),2,&[0,0,8,6,1,2,3,255,1,2,3],fm::VOID);
 s.element(&chain(52,&[c::ELM_OBJECT_ADD_HINTS]));s.int(1);s.code.push(cd::COMMAND);
 for value in [0,1,fm::INT,1,0,fm::VOID] {s.i32(value);}
 s.command(&[c::ELM_GLOBAL_TIMEWAIT],0,&[10],fm::VOID);
 s.command(&chain(52,&[c::ELM_OBJECT_CLEAR_HINTS]),0,&[],fm::VOID);s.code.push(cd::EOF);
 host.files.insert("Asset/Scene.pck".into(),crate::fixture::pack_scene(&s.finish()));
 vm.step(&mut host,InputFrame::default(),5000).unwrap();assert!(object(&vm,52).base.no_event_hint);
 let before=host.engine.frame([320,180]).unwrap();let saved=vm.snapshot().unwrap();
 for _ in 0..2 {
  host.now=10;vm.step(&mut host,InputFrame::default(),5000).unwrap();assert!(!object(&vm,52).base.no_event_hint);
  let after=host.engine.frame([320,180]).unwrap();
  let fingerprint=|frame:&[crysirror_scene::Primitive]|serde_json::to_value(frame.iter().map(|p|(p.owner,p.id,p.quad,&p.paint,p.clip,p.opacity,p.uv)).collect::<Vec<_>>()).unwrap();
  assert_eq!(fingerprint(&after),fingerprint(&before));
  vm.restore(&saved).unwrap();host.now=0;
 }
}
