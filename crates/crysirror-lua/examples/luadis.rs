//! Diagnostic listing of Lua 5.3 binary chunks: `luadis FILE.lua...`.
fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let proto = crysirror_lua::chunk::load(&std::fs::read(&path)?)?;
        let mut out = String::new();
        crysirror_lua::chunk::disassemble(&proto, "main", &mut out);
        println!("== {path}\n{out}");
    }
    Ok(())
}
