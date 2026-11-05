// SPDX-License-Identifier: MPL-2.0
use std::{path::PathBuf,fmt::Write};
fn main() {
 let dir=PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("wgsl");
 println!("cargo:rerun-if-changed={}",dir.display());
 let mut files=std::fs::read_dir(&dir).unwrap().map(|e|e.unwrap().path()).filter(|p|p.extension().is_some_and(|e|e=="wgsl")).collect::<Vec<_>>();files.sort();
 let mut source=String::new();let mut catalog=String::from("pub const FILES:&[(&str,&str)]=&[\n");
 for path in files {
  let name=path.file_name().unwrap().to_str().unwrap();let constant=path.file_stem().unwrap().to_str().unwrap().to_ascii_uppercase();
  writeln!(source,"pub const {constant}:&str=include_str!({:?});",path).unwrap();
  writeln!(catalog,"({name:?},{constant}),").unwrap();
 }
 source.push_str(&catalog);source.push_str("];\n");
 std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("wgsl.rs"),source).unwrap();
}
