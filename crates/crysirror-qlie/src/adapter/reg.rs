//! reg methods (704790): TRegistry access, kept per client.
//!
//! Native reads and writes the host registry. Here the registry is a store in
//! the save capability (`Registry.json`), so a game sees exactly the keys it
//! created itself; keys of other software are absent. Paths compare
//! case-insensitively like registry keys and keep their original spelling.
//! k/dk/?k create, delete and test keys; vs/vi write string/integer values;
//! dv deletes a value; ?v answers ReadInteger and ReadString (a value of the
//! other type reads as 0 or empty); ?kl/?vl,list,root,key fill a list with
//! subkey or value names. Writers and testers push ResultInt 1 on success.
use super::*;

const FILE: &str = "Registry.json";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct RegKey {
    name: String,
    values: BTreeMap<String, (String, RegValue)>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
enum RegValue {
    Text(String),
    Int(i32),
}
#[derive(Default)]
pub(super) struct Registry {
    keys: Option<BTreeMap<String, RegKey>>,
    load: Option<Task<Option<Vec<u8>>>>,
    write: Option<Task<()>>,
}
fn key_path(root: &str, key: &str) -> String {
    let key = key.trim_matches('\\');
    format!("{}\\{}", root.trim().to_ascii_uppercase(), key)
}

impl QlieVm {
    pub(super) fn reg_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        if self.registry.keys.is_none() {
            if self.registry.load.is_none() {
                let storage = h.storage(self.client)?;
                self.registry.load = Some(self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    let path = VfsPath::new(Mount::Save, FILE)?;
                    Ok(if storage.metadata(Mount::Save, FILE)?.is_some() {
                        Some(storage.read(&path)?)
                    } else {
                        None
                    })
                })?);
            }
            let Some(bytes) = self.registry.load.as_mut().unwrap().try_take()? else {
                return Ok(false);
            };
            self.registry.load = None;
            self.registry.keys = Some(match bytes {
                Some(bytes) => serde_json::from_slice(&bytes).context("QLIE registry store")?,
                None => BTreeMap::new(),
            });
        }
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        let (offset, list) = if matches!(op, "?kl" | "?vl") { (1, Some(s(0))) } else { (0, None) };
        let path = key_path(&s(offset), &s(offset + 1));
        let lower = path.to_lowercase();
        let name = s(offset + 2);
        let keys = self.registry.keys.as_mut().unwrap();
        let mut changed = false;
        let ok = match op {
            "k" => {
                if !keys.contains_key(&lower) {
                    keys.insert(lower.clone(), RegKey { name: path.clone(), ..Default::default() });
                    changed = true;
                }
                true
            }
            "?k" => keys.contains_key(&lower),
            "dk" => {
                let prefix = format!("{lower}\\");
                let before = keys.len();
                keys.retain(|k, _| *k != lower && !k.starts_with(&prefix));
                changed = keys.len() != before;
                changed
            }
            "vs" | "vi" => {
                let value = if op == "vs" {
                    RegValue::Text(s(offset + 3))
                } else {
                    RegValue::Int(a.get(3).map(Value::integer).transpose()?.unwrap_or(0))
                };
                let key = keys
                    .entry(lower.clone())
                    .or_insert_with(|| RegKey { name: path.clone(), ..Default::default() });
                key.values.insert(name.to_lowercase(), (name.clone(), value));
                changed = true;
                true
            }
            "dv" => {
                changed = keys
                    .get_mut(&lower)
                    .is_some_and(|k| k.values.remove(&name.to_lowercase()).is_some());
                changed
            }
            "?v" => {
                let value = keys.get(&lower).and_then(|k| k.values.get(&name.to_lowercase()));
                let (int, text) = match value {
                    Some((_, RegValue::Int(v))) => (*v, String::new()),
                    Some((_, RegValue::Text(t))) => (0, t.clone()),
                    None => (0, String::new()),
                };
                m.result(0, Value::Int(int));
                m.result_text(0, text);
                return Ok(true);
            }
            "?kl" | "?vl" => {
                let names: Option<Vec<String>> = keys.get(&lower).map(|k| {
                    if op == "?vl" {
                        k.values.values().map(|(n, _)| n.clone()).collect()
                    } else {
                        let prefix = format!("{lower}\\");
                        keys.iter()
                            .filter(|(p, _)| p.starts_with(&prefix) && !p[prefix.len()..].contains('\\'))
                            .map(|(_, k)| k.name.rsplit('\\').next().unwrap_or("").to_owned())
                            .collect()
                    }
                });
                let target = list.unwrap_or_default();
                match names {
                    Some(names) => {
                        if let Some(o) = self.objects.get_mut(&target).filter(|o| o.kind == "list") {
                            super::objects::strings_install(o, &names.join("\r\n"))?;
                        }
                        true
                    }
                    None => false,
                }
            }
            _ => bail!("unsupported QLIE reg method \\{op}"),
        };
        if changed {
            let bytes = serde_json::to_vec(self.registry.keys.as_ref().unwrap())?;
            let storage = h.storage(self.client)?;
            // A newer write supersedes an unfinished older one.
            self.registry.write = Some(self.pool.spawn(move |cancel| {
                cancel.check()?;
                storage.write_atomic(&VfsPath::new(Mount::Save, FILE)?, &bytes)?;
                Ok(())
            })?);
        }
        if ok {
            m.result(0, Value::Int(1));
        }
        Ok(true)
    }
}
