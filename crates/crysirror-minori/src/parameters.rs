// SPDX-License-Identifier: MPL-2.0
//! The native CProperties text file stores tab-separated byte strings in the save directory.
use anyhow::Result;
use crysirror_core::{
    Error,
    path::{Mount, VfsPath},
    service::ClientStorage,
};

const KEYS: [&[u8]; 3] = [
    b"messageSpeedTBR",
    b"messageSpeedRead",
    b"messageSpeedAutoPlay",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Speeds(pub [u8; 3]);

impl Default for Speeds {
    fn default() -> Self {
        Self([50; 3])
    }
}

fn bytes(storage: &dyn ClientStorage, path: &VfsPath) -> Result<Vec<u8>> {
    match storage.open_read(path) {
        Ok(file) => Ok(crysirror_resource::read_all(file.as_ref(), 2 << 20)?),
        Err(Error::NotFound) => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

fn setting(line: &[u8]) -> Option<(usize, &[u8])> {
    let tab = line.iter().position(|&c| c == b'\t')?;
    let index = KEYS.iter().position(|&key| key == &line[..tab])?;
    Some((index, &line[tab + 1..]))
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct SoundSettings {
    values: std::collections::BTreeMap<String, i32>,
}

#[derive(Clone, Copy)]
pub(crate) struct EffectSettings {
    pub snow_interval: u32,
    pub transition_interval: u32,
    pub screen_effect: bool,
}

impl Default for EffectSettings {
    fn default() -> Self { Self { snow_interval: 10, transition_interval: 5, screen_effect: true } }
}

impl EffectSettings {
    fn decode(data: &[u8]) -> Self {
        let mut settings = Self::default();
        for line in data.split(|&b| b == b'\n') {
            let Some(tab) = line.iter().position(|&b| b == b'\t') else { continue };
            let value = std::str::from_utf8(&line[tab + 1..]).ok().and_then(crate::vm::decimal_prefix).unwrap_or(0);
            match &line[..tab] {
                b"screenEffect" => settings.screen_effect = value != 0,
                b"effectDrawInterval" => settings.snow_interval = value.max(0) as u32,
                b"transitionDrawInterval" => settings.transition_interval = value as u32,
                _ => {}
            }
        }
        settings
    }
}

impl SoundSettings {
    fn decode(data: &[u8]) -> Self {
        let mut values = std::collections::BTreeMap::new();
        for line in data.split(|&b| b == b'\n') {
            let Some(tab) = line.iter().position(|&b| b == b'\t') else {
                continue;
            };
            let Ok(key) = std::str::from_utf8(&line[..tab]) else {
                continue;
            };
            if !matches!(
                key,
                "effectDrawInterval"
                    | "volumeBGM"
                    | "volumeSE"
                    | "volumeVoice"
                    | "muteBGM"
                    | "muteSE"
                    | "muteVoice"
            ) && !(key.len() <= 3 && key.bytes().all(|b| b.is_ascii_lowercase()))
            {
                continue;
            }
            if let Some(value) = std::str::from_utf8(&line[tab + 1..])
                .ok()
                .and_then(|s| s.trim().parse().ok())
            {
                values.insert(key.to_owned(), value);
            }
        }
        Self { values }
    }
    pub fn get(&self, key: &str, default: i32) -> i32 {
        self.values.get(key).copied().unwrap_or(default)
    }
    pub fn toggle_actor(&mut self, key: &str) {
        let enabled = self.get(key, 1) == 0;
        self.values.insert(key.to_owned(), i32::from(enabled));
    }
    fn update(&self, original: Vec<u8>, old: &Self) -> Vec<u8> {
        let mut changes: std::collections::BTreeMap<&str, i32> = self
            .values
            .iter()
            .filter(|(key, v)| old.values.get(*key) != Some(v))
            .map(|(key, &v)| (key.as_str(), v))
            .collect();
        if changes.is_empty() {
            return original;
        }
        let mut out = Vec::with_capacity(original.len());
        let mut written = std::collections::BTreeSet::new();
        for line in original.split_inclusive(|c| *c == b'\n') {
            let body = line.strip_suffix(b"\n").unwrap_or(line);
            let body = body.strip_suffix(b"\r").unwrap_or(body);
            if let Some(tab) = body.iter().position(|&b| b == b'\t')
                && let Ok(key) = std::str::from_utf8(&body[..tab])
                && let Some(value) = changes.get(key)
            {
                out.extend_from_slice(&body[..=tab]);
                out.extend_from_slice(value.to_string().as_bytes());
                out.extend_from_slice(&line[body.len()..]);
                written.insert(key);
            } else {
                out.extend_from_slice(line);
            }
        }
        changes.retain(|key, _| !written.contains(key));
        for (key, v) in changes {
            if !out.is_empty() && !out.ends_with(b"\n") {
                out.extend_from_slice(b"\r\n");
            }
            out.extend_from_slice(format!("{key}\t{v}\r\n").as_bytes());
        }
        out
    }
    pub fn voice_enabled(&self, path: &str, actors: &[String]) -> bool {
        if self.get("muteVoice", 0) != 0 {
            return false;
        }
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path).as_bytes();
        let length = if name.get(3) == Some(&b'-') { 3 } else { 2 };
        if name.len() < length {
            return true;
        }
        let key = std::str::from_utf8(&name[..length]).unwrap_or("");
        let key = if actors.iter().any(|id| id == key) {
            key
        } else {
            "etc"
        };
        self.get(key, 1) != 0
    }
}

impl Speeds {
    fn decode(bytes: &[u8]) -> Self {
        let mut speeds = Self::default();
        for line in bytes.split(|&c| c == b'\n') {
            if let Some((index, value)) = setting(line) {
                if let Ok(value) = std::str::from_utf8(value)
                    .unwrap_or("")
                    .trim()
                    .parse::<i32>()
                {
                    speeds.0[index] = value.clamp(0, 100) as u8;
                }
            }
        }
        speeds
    }

    pub fn read(storage: &dyn ClientStorage) -> Result<(Self, u32, bool, SoundSettings, EffectSettings)> {
        let data = bytes(storage, &VfsPath::new(Mount::Save, "sysparams.txt")?)?;
        let color = data
            .split(|b| *b == b'\n')
            .filter_map(|line| {
                let value = line.strip_prefix(b"panelColor\t")?;
                u32::from_str_radix(std::str::from_utf8(value).ok()?.trim(), 16).ok()
            })
            .last()
            .unwrap_or(0x805f5f6e);
        // CProperties reads cls as an integer and returns its nonzero test.
        let ctrl_skip = data
            .split(|b| *b == b'\n')
            .filter_map(|line| line.strip_prefix(b"cls\t"))
            .last()
            .and_then(|raw| std::str::from_utf8(raw).ok()?.trim().parse::<i32>().ok())
            .unwrap_or(0)
            != 0;
        Ok((
            Self::decode(&data),
            color,
            ctrl_skip,
            SoundSettings::decode(&data),
            EffectSettings::decode(&data),
        ))
    }

    fn update(self, original: &[u8], previous: Self) -> Vec<u8> {
        let changed = [0, 1, 2].map(|i| self.0[i] != previous.0[i]);
        let mut found = [false; 3];
        let mut out = Vec::with_capacity(original.len());
        for line in original.split_inclusive(|&c| c == b'\n') {
            let body = line.strip_suffix(b"\n").unwrap_or(line);
            let body = body.strip_suffix(b"\r").unwrap_or(body);
            if let Some((i, _)) = setting(body) {
                if changed[i] {
                    out.extend_from_slice(KEYS[i]);
                    out.push(b'\t');
                    out.extend_from_slice(self.0[i].to_string().as_bytes());
                    out.extend_from_slice(&line[body.len()..]);
                    found[i] = true;
                    continue;
                }
            }
            out.extend_from_slice(line);
        }
        for i in 0..3 {
            if changed[i] && !found[i] {
                if !out.is_empty() && !out.ends_with(b"\n") {
                    out.extend_from_slice(b"\r\n");
                }
                out.extend_from_slice(KEYS[i]);
                out.push(b'\t');
                out.extend_from_slice(self.0[i].to_string().as_bytes());
                out.extend_from_slice(b"\r\n");
            }
        }
        out
    }

    pub fn write_all(
        self,
        storage: &dyn ClientStorage,
        previous: Self,
        sound: &SoundSettings,
        previous_sound: &SoundSettings,
    ) -> Result<()> {
        let path = VfsPath::new(Mount::Save, "sysparams.txt")?;
        let original = bytes(storage, &path)?;
        storage.write_atomic(
            &path,
            &sound.update(self.update(&original, previous), previous_sound),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_changed_native_fields_are_replaced() {
        let original = b"fontname\t\x83\x65\r\nmessageSpeedTBR\t42\r\nother\t1";
        let old = Speeds::decode(original);
        assert_eq!(old, Speeds([42, 50, 50]));
        let edited = Speeds([4, 50, 76]);
        let out = edited.update(original, old);
        assert_eq!(
            out,
            b"fontname\t\x83\x65\r\nmessageSpeedTBR\t4\r\nother\t1\r\nmessageSpeedAutoPlay\t76\r\n"
        );
        assert_eq!(Speeds::decode(&out), edited);
    }
}
