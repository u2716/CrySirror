//! Native lifetime and allocation preflight, independently of resource devices.
use super::{codes as c, passive_object::{dispatch,ObjectWait}};
use crate::runtime::{CommandContext,Value,globals::ObjectState};
struct Harness {ctx:CommandContext,object:ObjectState,next:usize,finishes:Vec<crate::runtime::globals::PendingFrameActionFinish>}
impl Harness {
 fn new()->Self {let mut object=ObjectState::default();object.object_type=1;object.base.x=42;object.nested_runtime_slot=Some(900);object.frame_action.cmd_name="old".into();object.runtime.child_objects.push(ObjectState::default());Self{ctx:CommandContext::new("asset".into()),object,next:1000,finishes:vec![]}}
 fn run(&mut self,tail:&[i32],args:&[Value],overload:i64,available:usize)->anyhow::Result<Option<Value>> {
  let env=crate::native_string::Environment::new(&self.ctx);
  dispatch(&mut self.object,&self.ctx.ids,&mut self.ctx.images,&mut self.ctx.native_media_revision,tail,args,overload,available,0,&mut ObjectWait{text:&env,state:&mut self.ctx.wait,form:0,stage:1,slot:52,object_index:52,next_slot:&mut self.next,chain:&[],scene:"fixture",finishes:&mut self.finishes})
 }
}
#[test]
fn all_native_resource_creators_validate_every_argument_before_retiring_old_children_or_actions() {
 let mut h=Harness::new();let old=bincode::serialize(&h.object).unwrap();
 let mut rect=vec![Value::Int(0);11];rect[10]=Value::Str("bad Y".into());
 let mut pct=vec![Value::Str(String::new()),Value::Int(1),Value::Int(2),Value::Str("bad Y".into())];
 for (op,args,overload) in [
  (c::ELM_OBJECT_CREATE_RECT,rect,2),
  (c::ELM_OBJECT_CREATE,pct.clone(),2),
  (c::ELM_OBJECT_CREATE_MOVIE,vec![Value::Str("film".into()),Value::Int(1),Value::Int(2),Value::Str("bad Y".into())],2),
  (c::ELM_OBJECT_CREATE_STRING,vec![Value::Str("A".into()),Value::Int(1),Value::Int(2),Value::Int(i64::MAX)],2),
 ] {assert!(h.run(&[op],&args,overload,4096).is_err());assert_eq!(bincode::serialize(&h.object).unwrap(),old);assert_eq!(h.ctx.native_media_revision,0);assert!(h.finishes.is_empty());}
 pct[0]=Value::Str("picture".into());pct[3]=Value::Int(3);h.ctx.ids.obj_x=0;
 let before=bincode::serialize(&h.object).unwrap();
 let error=h.run(&[c::ELM_OBJECT_CREATE],&pct,2,4096).unwrap_err();
 assert!(format!("{error:#}").contains("property layout"));
 assert_eq!(bincode::serialize(&h.object).unwrap(),before);
 let mut movie=vec![Value::Str("film".into()),Value::Int(1),Value::Int(2),Value::Int(3)];
 let before=bincode::serialize(&h.object).unwrap();
 assert!(h.run(&[c::ELM_OBJECT_CREATE_MOVIE],&movie,2,4096).is_err());
 assert_eq!(bincode::serialize(&h.object).unwrap(),before);movie.clear();
}
#[test]
fn child_lazy_allocation_uses_remaining_global_budget_and_rejected_new_calls_leave_no_slots() {
 let mut h=Harness::new();h.object.runtime.child_objects.clear();h.object.base.disp=0;
 let before=bincode::serialize(&h.object).unwrap();
 assert!(h.run(&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,3,c::ELM_OBJECT_GET_TYPE],&[],0,3).is_err());
 assert_eq!(bincode::serialize(&h.object).unwrap(),before);assert_eq!(h.next,1000);
 assert!(h.run(&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,3,999_999],&[],0,4).is_err());
 assert_eq!(bincode::serialize(&h.object).unwrap(),before);assert_eq!(h.next,1000);
 assert!(matches!(h.run(&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,3,c::ELM_OBJECT_GET_ELEMENT_NAME],&[],0,4).unwrap(),Some(Value::Str(s)) if s=="stage[1].object[52]"));
 assert_eq!(h.object.runtime.child_objects.len(),4);assert!(h.object.used);assert_eq!(h.object.base.disp,1);assert_eq!(h.next,1001);
 assert_eq!(h.object.runtime.child_objects[3].nested_runtime_slot,Some(1000));
 // Known original negative child indices select zero, not a large allocation.
 assert!(matches!(h.run(&[c::ELM_OBJECT_CHILD,c::ELM_ARRAY,-9,c::ELM_OBJECT_GET_TYPE],&[],0,0).unwrap(),Some(Value::Int(0))));
 h.object.runtime.child_objects[3].frame_action.cmd_name="retired-child".into();
 h.run(&[c::ELM_OBJECT_CHILD,c::ELM_OBJECTLIST_RESIZE],&[Value::Int(1)],0,0).unwrap();
 assert_eq!(h.object.runtime.child_objects.len(),1);assert!(h.finishes.is_empty(),"pinned CHILD truncate releases resources without stage-finish callbacks");
}
#[test]
fn successful_rebuild_clears_children_and_action_state_without_losing_receiver_identity() {
 let mut h=Harness::new();
 h.run(&[c::ELM_OBJECT_CREATE_RECT],&[0,0,8,6,1,2,3,255,1,4,5].map(Value::Int),2,4096).unwrap();
 assert!(h.object.runtime.child_objects.is_empty());assert!(h.object.frame_action.cmd_name.is_empty());
 assert_eq!(h.object.nested_runtime_slot,Some(900));assert_eq!(h.object.object_type,1);
 assert_eq!((h.object.base.x,h.object.base.y,h.object.base.disp),(4,5,1));
 assert!(h.finishes.is_empty());
}
