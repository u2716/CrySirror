//! Engine-owned source geometry, texture coordinates and retained state.
use crysirror_core::state::ClientId;
use crysirror_scene::layer::*;
use serde_json::json;
const A:ClientId=ClientId(1);
fn node(id:u32,parent:Option<u32>)->LayerSpec {serde_json::from_value(json!({"id":id,"parent":parent,"rect":[20,30,100,80],"paint":{"kind":"solid","color":[255,0,0,255]}})).unwrap()}
#[test]
fn crop_remaps_atlas_uvs_before_rotation_and_preserves_children_and_content_size() {
 let mut layers=LayerSystem::default();let mut parent=node(1,None);
 parent.source_crop=Some([25.,10.,50.,40.]);parent.uv=Some([0.,0.,1.,1.]);parent.transform.rotate=std::f32::consts::FRAC_PI_2;
 let child=node(2,Some(1));layers.install(A,&[parent,child]).unwrap();
 let frame=layers.frame([400,400]).unwrap();let p=frame.iter().find(|p|p.id==1).unwrap();
 assert_eq!(p.size,[100,80]);assert_eq!(p.extent,[50.,40.]);assert_eq!(p.uv,[0.25,0.125,0.75,0.625]);
 for (actual,expected) in p.quad.iter().zip([[10.,55.],[10.,105.],[-30.,105.],[-30.,55.]]) {assert!((actual[0]-expected[0]).abs()<1e-4 && (actual[1]-expected[1]).abs()<1e-4,"{actual:?}");}
 assert_eq!(frame.iter().find(|p|p.id==2).unwrap().extent,[100.,80.]);
 layers.apply_batch(A,&[LayerCommand::SetSourceCrop{id:1,crop:Some([0.;4])}]).unwrap();
 let frame=layers.frame([400,400]).unwrap();assert_eq!(frame.len(),1);assert_eq!(frame[0].id,2);
}
#[test]
fn own_crop_is_transactional_scoped_and_intersects_per_glyph_without_reflow() {
 let mut layers=LayerSystem::default();let spec:LayerSpec=serde_json::from_value(json!({"id":1,"rect":[0,0,20,20],"source_crop":[2,3,8,9],"paint":{"kind":"glyph_run","font":"fixture","size":12,"color":[255,255,255,255],"glyphs":[{"character":"A","baseline":[0,12],"crop":[0,0,5,5]},{"character":"B","baseline":[12,12]}]}})).unwrap();
 layers.install(A,&[spec]).unwrap();let frame=layers.frame([100,100]).unwrap();
 assert_eq!(frame[0].extent,[20.,20.]);let Paint::GlyphRun{glyphs,..}=&frame[0].paint else {panic!()};
 assert_eq!(glyphs[0].crop,Some([2.,3.,3.,2.]));assert_eq!(glyphs[1].crop,Some([2.,3.,8.,9.]));assert_eq!(glyphs[1].baseline,[12.,12.]);
 let before=serde_json::to_vec(&layers).unwrap();
 for crop in [Some([0.,0.,-1.,2.]),Some([f32::NAN,0.,1.,2.])] {assert!(layers.apply_batch(A,&[LayerCommand::SetSourceCrop{id:1,crop}]).is_err());assert_eq!(serde_json::to_vec(&layers).unwrap(),before);}
 assert!(layers.apply_batch(ClientId(2),&[LayerCommand::SetSourceCrop{id:1,crop:None}]).is_err());
 let mut restored:LayerSystem=serde_json::from_slice(&before).unwrap();restored.validate().unwrap();assert_eq!(restored.get(A,1).unwrap().source_crop,Some([2.,3.,8.,9.]));
}
#[test]
fn crop_math_preserves_repeat_and_perspective_and_rejects_bad_rectangles() {
 use crop::*;
 assert_eq!(crop_rect([0.,0.,100.,80.],Some([25.,-10.,50.,50.]),[2.,1.,-2.,-1.]).unwrap().unwrap(),CropRect{rect:[25.,0.,50.,40.],uv:[1.,1.,-1.,0.]});
 assert!(crop_rect([0.,0.,2.,2.],Some([2.,0.,1.,1.]),[0.,0.,1.,1.]).unwrap().is_none());
 assert!(crop_rect([0.,0.,2.,2.],Some([0.,0.,-1.,1.]),[0.,0.,1.,1.]).is_err());
 let q=subrect_quad([[10.,20.,0.,2.],[30.,20.,0.,1.],[30.,60.,0.,1.],[10.,60.,0.,2.]],[100.,100.],[50.,50.,25.,25.]).unwrap();
 assert!((q[0][0]-25./1.5).abs()<1e-5);assert_eq!(q[0][1],40.);assert_eq!(q[0][3],1.5);
}
