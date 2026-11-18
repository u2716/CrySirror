// SPDX-License-Identifier: MPL-2.0
//! Malie configuration. The native config pages (game/text/sound/voice SVG)
//! bind controls by id: `<group>_<value>_opt` radio options, `<name>_chk`
//! check boxes and `<name>_vol` 0..100 scroll bars. Values are stored under
//! those names; defaults come from `malie.ini` ([SOUNDDEFAULTVOLUME],
//! [SYSTEM] WbOpacity, CHARAnn). Persisted as JSON in the save mount.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    pub options: BTreeMap<String, String>,
    pub checks: BTreeMap<String, bool>,
    pub values: BTreeMap<String, i32>,
    pub window_opacity: f32,
}
pub const FILE: &str = "malie/config.json";

#[derive(Clone, Copy, Debug)]
pub enum Volume {
    Bgm,
    Voice,
    Se,
    SystemSe,
    Movie,
    Env,
}
impl Volume {
    fn key(self) -> &'static str {
        match self {
            Self::Bgm => "bgm",
            Self::Voice => "voice",
            Self::Se => "se",
            Self::SystemSe => "systemse",
            Self::Movie => "movie",
            Self::Env => "env",
        }
    }
}
impl Settings {
    pub fn defaults(ini: &BTreeMap<String, BTreeMap<String, String>>) -> Self {
        let mut s = Self::default();
        let get = |sec: &str, k: &str| ini.get(sec).and_then(|m| m.get(k)).and_then(|v| v.trim().parse::<i32>().ok());
        for (k, ini_key) in [("bgm", "bgm"), ("voice", "voice"), ("se", "se"), ("systemse", "systemse"), ("movie", "movie"), ("env", "env")] {
            s.values.insert(k.into(), get("sounddefaultvolume", ini_key).unwrap_or(100).clamp(0, 100));
            s.checks.insert(format!("play{k}"), true);
        }
        for n in 1..=19 {
            let k = format!("chara{n:02}");
            s.values.insert(k.clone(), get("sounddefaultvolume", &k).unwrap_or(100).clamp(0, 100));
            s.checks.insert(k, true);
        }
        let opacity = get("system", "wbopacity").unwrap_or(70).clamp(0, 100);
        s.values.insert("window".into(), opacity);
        s.window_opacity = opacity as f32 / 100.;
        s.values.insert("textspeed".into(), 50);
        s.values.insert("autospeed".into(), 50);
        for (g, v) in [
            ("autosave", "yes"),
            ("cgeffect", "normal"),
            ("cheffect", "normal"),
            ("fadebgm", "yes"),
            ("mweffect", "normal"),
            ("savecomment", "yes"),
            ("screen", "window"),
            ("voicecut", "yes"),
            ("autovoice", "yes"),
            ("openface", "yes"),
            ("readcolor", "yes"),
            ("skip", "readonly"),
            ("skspeed", "normal"),
        ] {
            s.options.insert(g.into(), v.into());
        }
        s
    }
    pub fn volume(&self, v: Volume) -> f32 {
        let enabled = *self.checks.get(&format!("play{}", v.key())).unwrap_or(&true);
        if !enabled {
            return 0.;
        }
        *self.values.get(v.key()).unwrap_or(&100) as f32 / 100.
    }
    pub fn option(&self, group: &str) -> &str {
        self.options.get(group).map_or("", String::as_str)
    }
    /// Per-glyph reveal interval: textspeed 100 is instant, 0 is 80 ms.
    pub fn text_interval_ms(&self) -> u64 {
        let v = *self.values.get("textspeed").unwrap_or(&50);
        ((100 - v.clamp(0, 100)) as u64 * 80) / 100
    }
    /// Auto-mode delay after a page completes (plus voice length).
    pub fn auto_delay_ms(&self) -> u64 {
        let v = *self.values.get("autospeed").unwrap_or(&50);
        500 + ((100 - v.clamp(0, 100)) as u64 * 40)
    }
    pub fn sync_derived(&mut self) {
        self.window_opacity = *self.values.get("window").unwrap_or(&70) as f32 / 100.;
    }
}
