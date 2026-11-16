// SPDX-License-Identifier: MPL-2.0
//! Compiles style/ui.css (a custom-property subset) into const UI themes.
use std::{collections::BTreeMap, env, fs, path::Path};

const COLORS: &[&str] = &[
    "backdrop", "shadow", "panel", "panel-border", "title", "text", "disabled",
    "control", "control-text", "accent", "accent-text", "menu", "separator",
];
const LENGTHS: &[&str] = &["radius", "control-radius", "menu-radius"];

fn strip_comments(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = &rest[i + 2..];
        let j = rest.find("*/").expect("unterminated comment");
        rest = &rest[j + 2..];
    }
    out + rest
}

fn declarations(block: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for decl in block.split(';').map(str::trim).filter(|d| !d.is_empty()) {
        let (k, v) = decl.split_once(':').unwrap_or_else(|| panic!("bad declaration {decl:?}"));
        let k = k.trim().strip_prefix("--").unwrap_or_else(|| panic!("only custom properties: {k}"));
        assert!(
            COLORS.contains(&k) || LENGTHS.contains(&k),
            "unknown UI style property --{k}"
        );
        assert!(map.insert(k.to_owned(), v.trim().to_owned()).is_none(), "duplicate --{k}");
    }
    map
}

/// The body of the first `:root { … }` at or after `from`.
fn root_block(css: &str) -> (&str, usize) {
    let start = css.find(":root").expect(":root block");
    let open = start + css[start..].find('{').expect("{");
    let close = open + css[open..].find('}').expect("}");
    (&css[open + 1..close], close + 1)
}

fn color(v: &str) -> [u8; 4] {
    if let Some(h) = v.strip_prefix('#') {
        let n = |s: &str| u8::from_str_radix(s, 16).unwrap_or_else(|_| panic!("bad colour {v}"));
        return match h.len() {
            3 | 4 => {
                let c: Vec<u8> = h.chars().map(|c| n(&c.to_string()) * 17).collect();
                [c[0], c[1], c[2], *c.get(3).unwrap_or(&255)]
            }
            6 | 8 => {
                let a = if h.len() == 8 { n(&h[6..8]) } else { 255 };
                [n(&h[0..2]), n(&h[2..4]), n(&h[4..6]), a]
            }
            _ => panic!("bad colour {v}"),
        };
    }
    let inner = v
        .strip_prefix("rgba(")
        .or_else(|| v.strip_prefix("rgb("))
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or_else(|| panic!("bad colour {v}"));
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    assert!(parts.len() == 3 || parts.len() == 4, "bad colour {v}");
    let c = |s: &str| s.parse::<u8>().unwrap_or_else(|_| panic!("bad colour {v}"));
    let a = parts.get(3).map_or(255, |s| {
        let f: f32 = s.parse().unwrap_or_else(|_| panic!("bad alpha {v}"));
        assert!((0.0..=1.0).contains(&f), "bad alpha {v}");
        (f * 255.).round() as u8
    });
    [c(parts[0]), c(parts[1]), c(parts[2]), a]
}

fn length(v: &str) -> f32 {
    let n: f32 = v
        .strip_suffix("px")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or_else(|| panic!("bad length {v}"));
    assert!((0.0..=64.0).contains(&n), "length out of range {v}");
    n
}

fn theme(name: &str, d: &BTreeMap<String, String>) -> String {
    let mut s = format!("pub const {name}: Theme = Theme {{\n");
    for k in COLORS {
        let v = d.get(*k).unwrap_or_else(|| panic!("{name}: missing --{k}"));
        s += &format!("    {}: {:?},\n", k.replace('-', "_"), color(v));
    }
    for k in LENGTHS {
        let v = d.get(*k).unwrap_or_else(|| panic!("{name}: missing --{k}"));
        s += &format!("    {}: {:?},\n", k.replace('-', "_"), length(v));
    }
    s + "};\n"
}

fn main() {
    let path = "style/ui.css";
    println!("cargo:rerun-if-changed={path}");
    let css = strip_comments(&fs::read_to_string(path).expect("style/ui.css"));
    let (light, end) = root_block(&css);
    let media = end + css[end..].find("@media").expect("dark @media block");
    let rest = &css[media..];
    assert!(
        rest.starts_with("@media (prefers-color-scheme: dark)"),
        "only the prefers-color-scheme: dark media query is supported"
    );
    let (dark, _) = root_block(rest);
    let out = theme("LIGHT", &declarations(light)) + &theme("DARK", &declarations(dark));
    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("ui_theme.rs");
    fs::write(dest, out).unwrap();
}
