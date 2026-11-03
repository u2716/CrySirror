// Temporary inventory probe (not part of the suite).
use crysirror_krkr::{host::KrkrHost,natives};
use crysirror_tjs2::runtime::{Runtime,Variant};
#[test]
#[ignore]
fn dump_members() {
    let mut rt=Runtime::with_host(KrkrHost::new([32,32],"default".into()));
    natives::install(&mut rt);
    let classes=std::env::var("PROBE_CLASSES").unwrap();
    let mut out=serde_json::Map::new();
    for c in classes.split(',') {
        let v=rt.global_member(c);
        let names:Vec<String>=match v.object_handle() {Some(h)=>rt.object_members(h).into_iter().map(|(n,_)|n).collect(),None=>vec!["<absent>".into()]};
        out.insert(c.into(),serde_json::json!(names));
    }
    std::fs::write(std::env::var("PROBE_OUT").unwrap(),serde_json::to_string_pretty(&out).unwrap()).unwrap();
}
#[test]
#[ignore]
fn probe_instances() {
    let mut rt=Runtime::with_host(KrkrHost::new([32,32],"default".into()));
    natives::install(&mut rt);
    let src=std::fs::read_to_string(std::env::var("PROBE_TJS").unwrap()).unwrap();
    let v=natives::run_source(&mut rt,"probe.tjs",&src,None).unwrap();
    let h=v.object_handle().unwrap();
    let n=rt.object_member(h,"count").to_integer().unwrap();
    let mut out=String::new();
    for i in 0..n { out+=&format!("{}\n",rt.object_member(h,&i.to_string()).to_tjs_string().unwrap()); }
    std::fs::write(std::env::var("PROBE_OUT").unwrap(),out).unwrap();
    let _=Variant::Void;
}
