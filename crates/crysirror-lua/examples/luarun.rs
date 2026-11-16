//! Diagnostic: run Lua 5.3 chunks in order, then optionally call a global:
//! `luarun FILE.lua... [-- FUNCTION ARG...]`. Host functions are absent.
use crysirror_lua::{Host, Lua, Value};
struct NoHost;
impl Host for NoHost {
    fn call(&mut self, _lua: &mut Lua, id: u32, _args: Vec<Value>) -> anyhow::Result<Vec<Value>> {
        anyhow::bail!("no host function {id}")
    }
    fn print(&mut self, text: &[u8]) {
        println!("[print] {}", String::from_utf8_lossy(text));
    }
}
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let split = args.iter().position(|a| a == "--").unwrap_or(args.len());
    let mut lua = Lua::new();
    let mut host = NoHost;
    for path in &args[..split] {
        let proto = crysirror_lua::chunk::load(&std::fs::read(path)?)?;
        let main = lua.load(proto)?;
        match lua.call(&main, vec![], &mut host) {
            Ok(_) => println!("ran {path}"),
            Err(e) => println!("{path}: {e:#}"),
        }
    }
    if let Some(name) = args.get(split + 1) {
        let call_args = args[split + 2..].iter().map(|a| a.parse::<i64>().map(Value::Int).unwrap_or_else(|_| Value::str(a))).collect();
        let f = lua.global(name);
        let results = lua.call(&f, call_args, &mut host)?;
        for r in results {
            println!("=> {}", String::from_utf8_lossy(&r.display()));
        }
    }
    println!("live tables {} funcs {}; freed {}", lua.tables.live(), lua.funcs.live(), lua.collect(&[]));
    Ok(())
}
