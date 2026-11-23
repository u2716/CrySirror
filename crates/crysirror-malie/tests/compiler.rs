// SPDX-License-Identifier: MPL-2.0
//! Malie script compiler (malie.exe 675AE0) over self-authored sources,
//! executed on the EXEC VM.
use anyhow::Result;
use crysirror_malie::compiler::{compile, Source};
use crysirror_malie::program::Program;
use crysirror_malie::vm::{Machine, Step};
use std::collections::HashMap;
use std::sync::Arc;

struct Files(HashMap<String, Vec<u8>>);
impl Source for Files {
    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.0.get(path).cloned())
    }
}
fn build(files: &[(&str, &str)]) -> Result<Program> {
    let mut f = Files(files.iter().map(|(k, v)| (k.to_string(), v.as_bytes().to_vec())).collect());
    Ok(compile(&mut f, "data\\script\\main")?.expect("source exists"))
}
/// Run `main` to completion; native calls answer with their first argument
/// and are recorded.
fn run(p: Program) -> Result<(Vec<(String, Vec<u32>)>, Machine)> {
    let p = Arc::new(p);
    let mut m = Machine::new(p.clone());
    m.function("main")?;
    let mut calls = Vec::new();
    for _ in 0..100_000 {
        match m.step()? {
            Step::Continue => {}
            Step::Stop => return Ok((calls, m)),
            Step::Call(c) => {
                let name = p.functions[c.function as usize].name.clone();
                let r = c.args.first().copied().unwrap_or(0);
                calls.push((name, c.args.clone()));
                m.complete(r)?;
            }
        }
    }
    anyhow::bail!("script did not stop")
}

const HEADER: &str = "extern int __cdecl out(int, int);\n";

#[test]
fn arithmetic_control_flow_and_calls() -> Result<()> {
    let src = format!(
        "{HEADER}
int g;
int table[4];
int __cdecl twice(int v) {{ return v * 2; }}
int __stdcall add3(int a, int b, int c) {{ return a + b + c; }}
int __cdecl main() {{
    int i;
    int sum;
    sum = 0;
    for (i = 0; i < 5; i++) sum += i;
    out(1, sum);
    i = 10;
    while (i > 7) i--;
    out(2, i);
    do {{ i = i + 100; }} while (i < 300);
    out(3, i);
    if (sum == 10) out(4, 1); else out(4, 0);
    g = twice(21);
    out(5, g);
    out(6, add3(1, 2, 3));
    out(7, (1 + 2) * 3 - 4 / 2 % 3 << 1);
    out(8, -5 >> 1);
    out(9, !0 && 3 || 0);
    out(10, ~0 & 0x0f | 0x30);
    out(11, sizeof(table));
    goto skip;
    out(12, 0);
skip:
    out(13, 1);
    return 0;
}}
"
    );
    let (calls, _) = run(build(&[("data/script/main.ms", &src)])?)?;
    let got: Vec<(u32, u32)> = calls.iter().filter(|(n, _)| n == "out").map(|(_, a)| (a[0], a[1])).collect();
    let want: Vec<(u32, u32)> = vec![(1, 10), (2, 7), (3, 307), (4, 1), (5, 42), (6, 6), (7, 14), (8, (-3i32) as u32), (9, 1), (10, 0x3f), (11, 16), (13, 1)];
    assert_eq!(got, want);
    Ok(())
}

#[test]
fn messages_labels_macros_and_includes() -> Result<()> {
    let main = "#include \"defs\"\n\
int __stdcall _ms_message(int n);\n\
#define TWICE(x) ((x) + (x))\n\
#ifdef MISSING\n\
int never;\n\
#else\n\
int __cdecl main() {\n\
    out(TWICE(VALUE), 0);\n\
    $\"こんにちは\\n\" $\"world\";\n\
    $$\n\
    $\"two\";\n\
    $$\n\
    return 0;\n\
}\n\
#endif\n\
int __stdcall _ms_message(int n) { out(100, n); return 0; }\n\
chapter:\n\
@local:\n";
    let defs = "extern int __cdecl out(int, int);\n#define VALUE 21\n";
    let p = build(&[("data/script/main.ms", main), ("data/script/defs.ms", defs)])?;
    assert_eq!(p.message(0)?, "こんにちは\nworld");
    assert_eq!(p.message(1)?, "two");
    assert!(p.labels.contains_key("chapter") && p.labels.contains_key("chapter@local"));
    assert!(!p.symbols.iter().any(|s| s.name == "never"));
    let (calls, _) = run(p)?;
    let got: Vec<(u32, u32)> = calls.iter().map(|(_, a)| (a[0], a[1])).collect();
    assert_eq!(got, vec![(42, 0), (100, 0), (100, 1)]);
    Ok(())
}

#[test]
fn shift_jis_and_compressed_sources() -> Result<()> {
    use std::io::Write;
    let text = "extern int __cdecl out(int, int);\nint __cdecl main() { $\"日本語\"; $$ return 0; }\nint __stdcall _ms_message(int n) { return 0; }\n";
    // Declared after use: _ms_message must be visible at the message.
    let text = text.replace("int __cdecl main", "int __stdcall _ms_message(int n);\nint __cdecl main");
    let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode(&text);
    let p = compile(&mut Files([("data/script/main.txt".to_string(), sjis.to_vec())].into()), "data\\script\\main")?.unwrap();
    assert_eq!(p.message(0)?, "日本語");
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&sjis)?;
    let mut mls = b"MalieScenario".to_vec();
    mls.extend(z.finish()?);
    let p = compile(&mut Files([("data/script/main.mls".to_string(), mls)].into()), "data\\script\\main")?.unwrap();
    assert_eq!(p.message(0)?, "日本語");
    Ok(())
}

#[test]
fn errors_fail_the_compile() {
    let r = build(&[("data/script/main.ms", "int __cdecl main() { undefined_label_target: goto nowhere; return 0; }")]);
    assert!(r.is_err());
    let mut none = Files(HashMap::new());
    assert!(compile(&mut none, "data\\script\\absent").unwrap().is_none());
}

/// Local comparison against a shipped exec.dat: MALIE_COMPILE_SRC names a
/// source file; the compiled sections are written to MALIE_COMPILE_OUT.
#[test]
#[ignore = "needs a locally reconstructed source"]
fn dump_local_compile() -> Result<()> {
    let src = std::env::var("MALIE_COMPILE_SRC")?;
    let out = std::env::var("MALIE_COMPILE_OUT")?;
    let bytes = std::fs::read(&src)?;
    let p = compile(&mut Files([("x.ms".to_string(), bytes)].into()), "x")?.unwrap();
    let mut o = Vec::new();
    let names: Vec<String> = p.functions.iter().map(|f| f.name.clone()).collect();
    o.extend(serde_json::to_vec(&serde_json::json!({
        "code": p.code, "strings": p.strings, "labels": p.labels, "functions": names,
        "messages": p.messages, "ranges": p.message_ranges, "symbols": p.symbols.iter().map(|s| (s.name.clone(), s.kind, s.offset, s.convention)).collect::<Vec<_>>(),
        "global_size": p.global_size,
    }))?);
    std::fs::write(out, o)?;
    Ok(())
}
