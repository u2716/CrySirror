//! ext (6E1EDC), ppngpool (6E15D8) and twitter (6D3A0C) objects.
use super::*;

/// twitter settings (6D3A0C s/?g): integer and string slots by name.
const TWITTER_INT: &[&str] = &["consumer_code", "connecttimeout", "readtimeout", "savebutton"];
const TWITTER_TEXT: &[&str] = &[
    "consumer_key", "consumer_secret", "consumer_codestr", "access_token_key",
    "access_token_secret", "tag", "textbackup",
];
const TWITTER_URL: &[&str] = &["request_token", "authorize", "access_token", "post_text", "post_media"];

impl QlieVm {
    pub(super) fn minor_kind_method(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        kind: &str,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<Option<bool>> {
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        match (kind, op) {
            // ext s,text / i call the embedding application's OnExtCommand /
            // OnExtInit hooks (engine+58/+60); no embedding host installs them,
            // so only the stored command text and i's tick floor remain.
            ("ext", "s") => {
                self.global_strings.insert("ext".into(), s(0));
            }
            ("ext", "i") => {}
            // ppngpool d,file decodes a PNG into the shared pool (553930) and
            // reports script_fileloaderror when that fails; a,file releases it
            // (553D48); cc,flag clears the pool (55360C). Decoded images are
            // cached by the engine; the pool membership is kept for the errors.
            ("ppngpool", "d") => {
                ensure!(a.len() == 1, "QLIE PNG pool ABI");
                let name = s(0);
                if name.to_ascii_lowercase().ends_with(".png") {
                    let candidates = self.resource_candidates(m, &name)?;
                    let found = candidates.iter().any(|c| self.config.assets.contains_key(c));
                    ensure!(found, "QLIE script_fileloaderror: {name}");
                    self.png_pool.insert(name.to_ascii_lowercase());
                } else {
                    bail!("QLIE script_fileloaderror: {name}");
                }
            }
            ("ppngpool", "a") => {
                self.png_pool.remove(&s(0).to_ascii_lowercase());
            }
            ("ppngpool", "cc") => {
                if a.first().map(Value::integer).transpose()?.unwrap_or(0) == 1 {
                    self.png_pool.clear();
                }
            }
            ("twitter", "s" | "url") => {
                ensure!(a.len() == 2, "QLIE twitter setting ABI");
                let name = s(0).to_ascii_lowercase();
                let known = if op == "url" { TWITTER_URL } else { &[TWITTER_INT, TWITTER_TEXT].concat()[..] };
                ensure!(known.contains(&name.as_str()), "QLIE script_undefinedtypename: {name}");
                let o = self.objects.get_mut(id).unwrap();
                o.properties.insert(format!("{op}/{name}"), vec![a[1].clone()]);
            }
            ("twitter", "?g") => {
                ensure!(a.len() == 1, "QLIE twitter query ABI");
                let name = s(0).to_ascii_lowercase();
                let value = self.objects[id].properties.get(&format!("s/{name}")).and_then(|v| v.first()).cloned();
                if TWITTER_INT.contains(&name.as_str()) {
                    m.result(0, Value::Int(value.map(|v| v.integer()).transpose()?.unwrap_or(0)));
                } else if TWITTER_TEXT.contains(&name.as_str()) {
                    m.result_text(0, value.map(|v| v.text()).unwrap_or_default());
                } else {
                    bail!("QLIE script_undefinedtypename: {name}");
                }
            }
            ("twitter", "i") => {
                self.objects.get_mut(id).unwrap().properties.retain(|k, _| !k.starts_with("s/") && !k.starts_with("url/"));
            }
            // OAuth 1.0a posting to the retired Twitter v1.1 API is not
            // performed: the call is logged and answers nothing.
            ("twitter", "auth" | "post") => {
                let message = format!("QLIE twitter\\{op} ignored: Twitter v1.1 posting is not supported");
                h.append_log(self.client, &message)?;
                eprintln!("CrySirror: {message}");
            }
            _ => return Ok(None),
        }
        Ok(Some(true))
    }
}
