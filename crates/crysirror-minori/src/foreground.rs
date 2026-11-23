// SPDX-License-Identifier: MPL-2.0
use super::{ClientId, HostServices, Result, Sprite, Value, asset, json, remove, sprite};
use std::collections::BTreeMap;

const FIRST: u32 = 540;
#[derive(Default)]
pub(super) struct Foreground {
    pub main: Option<Sprite>,
    aux: BTreeMap<u32, Sprite>,
}
pub(super) fn blend(op: &mut Value, path: &str) {
    for (suffix, mode) in [
        ("_sc.png", json!("screen")),
        ("_ov.png", json!({"stored_byte":{"mode":"overlay"}})),
        ("_ml.png", json!("multiply")),
        ("_ad.png", json!("add")),
    ] {
        if path.ends_with(suffix) {
            op["layer"]["blend"] = mode;
            break;
        }
    }
}

impl Foreground {
    pub fn requires_frame_publication(&self) -> bool {
        self.main.iter().chain(self.aux.values()).any(Sprite::requires_frame_publication)
    }

    pub fn load(
        h: &mut dyn HostServices,
        c: ClientId,
        source: &str,
        at: [i32; 2],
        banks: &[&str],
    ) -> Result<Self> {
        let (fixed, remaining) = source.split_once(':').unwrap_or(("", source));
        let (group, upper) = remaining.split_once('+').unwrap_or((remaining, ""));
        let mut parts = group.splitn(3, ',');
        let main = parts.next().unwrap_or_default();
        let first = parts.next().unwrap_or_default();
        let second = parts.next().unwrap_or_default();
        let mut result = Self::default();
        let storage = h.storage(c)?;
        for (i, name) in [main, first, second, upper, fixed].into_iter().enumerate() {
            if name.is_empty() || name == "*" {
                continue;
            }
            let mut chosen = None;
            for bank in banks {
                if storage.exists(&asset(bank, super::tone::split(name).1)?) {
                    chosen = Some(*bank);
                    break;
                }
            }
            let Some(bank) = chosen else { continue };
            // The standing foreground loader looks for a same-stem SQZ for its
            // animated auxiliaries. Keep their stage layer IDs across camera changes.
            let companion = name.rsplit_once('.').map(|(stem, _)| format!("{stem}.sqz"));
            let source = match companion.as_deref() {
                Some(path) if matches!(i, 1 | 2) && storage.exists(&asset(bank, path)?) => path,
                _ => name,
            };
            let mut art = sprite(h, c, &[bank], source)?;
            if i <= 2 {
                art.rect[0] = at[0];
                art.rect[1] = at[1];
            }
            if i == 0 {
                result.main = Some(art)
            } else {
                result.aux.insert(FIRST + i as u32 - 1, art);
            }
        }
        Ok(result)
    }
    pub fn walking_frame(&self, published: &super::Published) -> Option<u32> {
        published.ordinal(FIRST + 1).or_else(|| published.ordinal(FIRST))
    }
    pub fn auxiliary(&self, main_id: u32, camera: [i32; 2], ops: &mut Vec<Value>) {
        self.walking_auxiliary(main_id, camera, |_| None, ops);
    }
    pub fn walking_auxiliary(&self, main_id: u32, camera: [i32; 2], secondary: impl Fn(&Sprite) -> Option<[i32; 2]>, ops: &mut Vec<Value>) {
        for id in FIRST..FIRST + 4 {
            if let Some(art) = self.aux.get(&id) {
                let mut art = art.clone();
                if id < FIRST + 2 {
                    let camera = secondary(&art).unwrap_or(camera);
                    art.rect[0] = camera[0].saturating_neg();
                    art.rect[1] = camera[1].saturating_neg();
                }
                let mut op = art.layer(id, 255);
                op["layer"]["order"] = json!([main_id, (id - FIRST + 1) as i32]);
                blend(&mut op, &art.asset);
                ops.push(op);
            } else {
                ops.push(remove(id));
            }
        }
    }
    pub fn draw_main(art: &Sprite, id: u32, path: &str) -> Value {
        let mut op = art.layer(id, 255);
        blend(&mut op, path);
        op
    }
    pub fn is_aux(id: u32) -> bool {
        (FIRST..FIRST + 4).contains(&id)
    }
}
