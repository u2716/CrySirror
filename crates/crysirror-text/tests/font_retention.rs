//! Scoped canonical font retention without font files, native VMs or a device.
use crysirror_core::state::ClientId;
use crysirror_text::{TextSystem,dynamic::PreparedFontFace,glyph::GlyphRaster};
use std::sync::Arc;
const A:ClientId=ClientId(1);
fn face(size:u16)->Arc<PreparedFontFace> {
 let mut b=b"TVP pre-rendered font\x1a".to_vec();b.extend([1,2]);b.extend(1u32.to_le_bytes());b.extend(36u32.to_le_bytes());b.extend(38u32.to_le_bytes());b.extend((b'A' as u16).to_le_bytes());b.extend(58u32.to_le_bytes());
 for v in [2i16,2,-1,2,3,0,4,0] {b.extend(v.to_le_bytes());}b.extend([32,0x43]);
 Arc::new(PreparedFontFace::prepare_precomputed(b.into(),size,&||Ok(())).unwrap())
}
#[test]
fn real_frozen_registration_keeps_retired_canonical_rendering_until_last_snapshot_drops() {
 let text=TextSystem::default();let alias="__font/1/one";
 text.publish_font(A,alias,face(12)).unwrap();let view=text.for_client(A);
 let held=view.frozen_font(alias).unwrap();text.unpublish_font(A,alias).unwrap();
 let glyph_id=view.glyph_id(alias,'A').unwrap();let cell=view.rasterize_glyph_cell(alias,GlyphRaster{glyph_id,size:12.,baseline:[0.,2.]}).unwrap();
 assert_eq!((cell.width,cell.height),(2,2));assert_eq!(cell.coverage.iter().filter(|v|**v>0.).count(),4);
 assert!(text.for_client(ClientId(2)).glyph_id(alias,'A').is_err());
 assert!(text.unpublish_font(A,alias).is_err(),"load registration is already gone");
 let in_flight=view.frozen().unwrap();drop(held);
 assert!(view.glyph_id(alias,'A').is_ok(),"admitted whole-view snapshot is a real holder");
 drop(in_flight);assert!(view.glyph_id(alias,'A').is_err());assert!(view.frozen_font(alias).is_err());
}
#[test]
fn sharing_face_bytes_or_another_clients_alias_does_not_keep_a_retired_registration_alive() {
 let text=TextSystem::default();let shared=face(12);let alias="__font/1/one";
 text.publish_font(A,alias,shared.clone()).unwrap();text.publish_font(ClientId(2),"__font/2/other",shared.clone()).unwrap();
 text.unpublish_font(A,alias).unwrap();
 assert!(text.for_client(A).glyph_id(alias,'A').is_err(),"raw face bytes are not a held canonical registration");
 assert!(text.for_client(A).frozen_font(alias).is_err());
 assert!(text.for_client(ClientId(2)).glyph_id("__font/2/other",'A').is_ok());
 assert_eq!(Arc::strong_count(&shared),2,"retired weak slot owns no face bytes");
}
#[test]
fn held_alias_collision_rejects_mutation_but_expired_registration_can_be_collected() {
 let text=TextSystem::default();let alias="__font/1/one";let first=face(12);let digest=first.digest.clone();
 text.publish_font(A,alias,first.clone()).unwrap();let view=text.for_client(A);let held=view.frozen_font(alias).unwrap();text.unpublish_font(A,alias).unwrap();
 assert!(text.publish_font(A,alias,face(13)).is_err());assert_eq!(view.font_digest(alias).unwrap(),digest);
 // Identical publication reactivates the same registration, not a replacement
 // which would invalidate the already admitted immutable view.
 text.publish_font(A,alias,first).unwrap();text.unpublish_font(A,alias).unwrap();drop(held);
 assert!(view.font_digest(alias).is_err());let second=face(13);let digest2=second.digest.clone();
 text.publish_font(A,alias,second).unwrap();assert_eq!(view.font_digest(alias).unwrap(),digest2);
}
#[test]
fn live_retired_registrations_count_against_the_catalog_limit_and_dead_weak_slots_are_reclaimed() {
 let text=TextSystem::default();let shared=face(12);let view=text.for_client(A);let mut held=Vec::new();
 for n in 0..128 {let alias=format!("__font/1/{n}");text.publish_font(A,&alias,shared.clone()).unwrap();held.push(view.frozen_font(&alias).unwrap());text.unpublish_font(A,&alias).unwrap();}
 assert!(text.publish_font(A,"__font/1/next",shared.clone()).is_err());
 drop(held);text.publish_font(A,"__font/1/next",shared.clone()).unwrap();
 for n in 128..512 {let alias=format!("__font/1/{n}");text.publish_font(A,&alias,shared.clone()).unwrap();text.unpublish_font(A,&alias).unwrap();assert!(view.glyph_id(&alias,'A').is_err());}
 assert!(view.glyph_id("__font/1/next",'A').is_ok());
}
