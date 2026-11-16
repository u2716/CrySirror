use crysirror_lua::{Host, Lua, Value};
struct NoHost;
impl Host for NoHost {
    fn call(&mut self, _lua: &mut Lua, id: u32, _args: Vec<Value>) -> anyhow::Result<Vec<Value>> {
        anyhow::bail!("no host function {id}")
    }
}
fn call(lua: &mut Lua, path: &str, args: Vec<Value>) -> Vec<String> {
    let (lib, name) = path.split_once('.').unwrap_or(("", path));
    let f = if lib.is_empty() {
        lua.global(name)
    } else {
        let Value::Table(t) = lua.global(lib) else { panic!("{lib}") };
        lua.get_field(t, name).unwrap()
    };
    lua.call(&f, args, &mut NoHost).unwrap().iter().map(|v| String::from_utf8_lossy(&v.display()).into_owned()).collect()
}
fn s(text: &str) -> Value {
    Value::str(text)
}
#[test]
fn patterns() {
    let mut lua = Lua::new();
    assert_eq!(call(&mut lua, "string.find", vec![s("a,b,c"), s("(.-),()")]), ["1", "2", "a", "3"]);
    assert_eq!(call(&mut lua, "string.find", vec![s("hello world"), s("o w")]), ["5", "7"]);
    assert_eq!(call(&mut lua, "string.match", vec![s("key = value"), s("(%w+)%s*=%s*(%w+)")]), ["key", "value"]);
    assert_eq!(call(&mut lua, "string.gsub", vec![s("hello world"), s("o"), s("0")]), ["hell0 w0rld", "2"]);
    assert_eq!(call(&mut lua, "string.gsub", vec![s("abc"), s("%w"), s("%0%0")]), ["aabbcc", "3"]);
    assert_eq!(call(&mut lua, "string.match", vec![s("[x]"), s("%[(.*)%]")]), ["x"]);
    assert_eq!(call(&mut lua, "string.find", vec![s("abc"), s("^b")]), ["nil"]);
    assert_eq!(call(&mut lua, "string.match", vec![s("f(a(b)c)"), s("%b()")]), ["(a(b)c)"]);
    assert_eq!(call(&mut lua, "string.match", vec![s("2024-01-02"), s("(%d+)-(%d+)-(%d+)$")]), ["2024", "01", "02"]);
}
#[test]
fn formatting() {
    let mut lua = Lua::new();
    assert_eq!(call(&mut lua, "string.format", vec![s("%02d:%3d|%-3d|%x"), Value::Int(5), Value::Int(7), Value::Int(1), Value::Int(255)]), ["05:  7|1  |ff"]);
    assert_eq!(call(&mut lua, "string.format", vec![s("%.2f %g %g %s"), Value::Num(3.14159), Value::Num(0.1), Value::Num(1e20), Value::Int(3)]), ["3.14 0.1 1e+20 3"]);
    assert_eq!(call(&mut lua, "tostring", vec![Value::Num(2.0)]), ["2.0"]);
    assert_eq!(call(&mut lua, "tostring", vec![Value::Num(0.1)]), ["0.1"]);
    assert_eq!(call(&mut lua, "string.sub", vec![s("hello"), Value::Int(-3)]), ["llo"]);
    assert_eq!(call(&mut lua, "math.floor", vec![Value::Num(-1.5)]), ["-2"]);
    assert_eq!(call(&mut lua, "math.modf", vec![Value::Num(3.25)]), ["3.0", "0.25"]);
}
