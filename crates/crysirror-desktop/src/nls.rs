// SPDX-License-Identifier: MPL-2.0
//! Launch policy only. Native adapters retain decoding, byte traversal and save semantics.
use anyhow::{bail, ensure, Result};
use serde_json::{json, Value};

pub fn options(adapter: &str) -> &'static [&'static str] {
    match adapter {
        "reallive" => &["sjis", "gbk", "big5", "utf8", "western", "korean"],
        "avg32" => &["sjis", "gbk", "big5", "utf8"],
        "rfvp" | "softpal" => &["sjis", "gbk", "utf8"],
        "nscripter" => &["sjis", "utf8"],
        "bgi" => &["sjis", "gbk"],
        _ => &[],
    }
}
/// Validate every client before changing anything. None/auto preserves adapter
/// defaults, including RealLive's per-scenario RLdev metadata precedence.
pub fn apply(project: &mut Value, selected: Option<&str>) -> Result<()> {
    let Some(selected) = selected.filter(|s| !s.is_empty() && *s != "auto") else {
        return Ok(());
    };
    let clients = project["clients"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("project clients"))?;
    ensure!(!clients.is_empty(), "NLS requires a native client");
    for client in clients.iter() {
        let adapter = client["adapter"].as_str().unwrap_or("");
        ensure!(
            options(adapter).contains(&selected),
            "NLS {selected:?} is not supported by {adapter}; supported: {:?}",
            options(adapter)
        );
        ensure!(client["config"].is_object(), "native client configuration");
    }
    for client in clients {
        let adapter = client["adapter"].as_str().unwrap().to_owned();
        let config = &mut client["config"];
        match adapter.as_str() {
            "reallive" | "avg32" | "nscripter" => config["encoding"] = json!(selected),
            "rfvp" | "softpal" => config["nls"] = json!(selected),
            "bgi" => config["gbk"] = json!(selected == "gbk"),
            _ => bail!("native NLS configuration unavailable"),
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_settings_and_auto_preserve_semantics() {
        for (adapter, key, value) in [
            ("reallive", "encoding", json!("gbk")),
            ("avg32", "encoding", json!("gbk")),
            ("rfvp", "nls", json!("gbk")),
            ("softpal", "nls", json!("gbk")),
            ("bgi", "gbk", json!(true)),
        ] {
            let mut p = json!({"clients":[{"adapter":adapter,"config":{}}]});
            let original = p.clone();
            apply(&mut p, Some("auto")).unwrap();
            assert_eq!(p, original);
            apply(&mut p, Some("gbk")).unwrap();
            assert_eq!(p["clients"][0]["config"][key], value);
        }
    }
    #[test]
    fn unsupported_choice_is_atomic() {
        let mut p = json!({"clients":[{"adapter":"reallive","config":{}},{"adapter":"siglus","config":{}}]});
        let original = p.clone();
        assert!(apply(&mut p, Some("gbk")).is_err());
        assert_eq!(p, original);
        assert!(apply(
            &mut json!({"clients":[{"adapter":"rfvp","config":{}}]}),
            Some("big5")
        )
        .is_err());
    }
}
