//! Engine-setting record methods of the cfg object (6EA71C).
//!
//! Native keeps these in the config record at 74C754 (constructor 5097E4).
//! 505FE8 fills it from Config.txt with `StrToIntDef(value, default)`, so an
//! absent key takes the reader default below even when an earlier file set
//! it; 50786C writes the same keys back. Most of the values configure the
//! Windows host (task tray, multicore decoding, CRC archive checks, DirectSound
//! focus, letterbox background image, close dialogs); the VM keeps, queries
//! and persists them exactly. KeyCallbackTime, MouseGestureEnable and the
//! Swipe* values feed the input device each frame (70A6E0/70A3CC).
use super::*;

/// (cfg property, Config.txt key, 505FE8 default) of the integer slots that
/// the older cfg loader table does not cover.
pub(super) const INT_SETTINGS: &[(&str, &str, i32)] = &[
    ("crc", "CRCCheck", 1),              // +184
    ("ttm", "TaskTrayMode", 0),          // +188
    ("mam", "ManualAspectMode", 3),      // +190
    ("mas/w", "AspectWidth", 1280),      // +194
    ("mas/h", "AspectHeight", 720),      // +198
    ("nfs", "NoChangeSizeFullScreenMode", 0), // +19C
    ("rfs", "AspectStretchBlt", 0),      // +1A0
    ("blp", "BitBltModeParam", 80),      // +1A8
    ("dsf", "DirectSoundStickyFocus", 1), // +1AC
    ("fdd", "FPSDeactiveDecay", 20),     // +1B0
    ("kct", "KeyCallbackTime", 500),     // +1B4
    ("ssd", "DeactiveSystemStop", 0),    // +1B8
    ("ssm", "MinimizeSystemStop", 0),    // +1BC
    ("mct", "MinimizeClearTitle", 0),    // +1C0
    ("mht", "MouseHideTime", 4000),      // +1C8
    ("vsw", "VSyncMode", 1),             // +1CC
    ("fcd", "FormCloseDialog", 1),       // +1E4
    ("fcs", "FormCloseSave", 1),         // +1E8
    ("mre", "MultiCoreEnable", 1),       // +1F0
    ("mrc", "MultiCoreCount", 2),        // +1F4
    ("mge", "MouseGestureEnable", 7),    // +1FC
    ("swe", "SwipeEnable", 7),           // +200
    ("swp", "SwipePermitFrame", 6),      // +204
    ("swi", "SwipeIntense", 80),         // +208
    ("tdm", "TouchDeviceMode", 0),       // +20C
];

/// Delphi StrToIntDef over a Config.txt value (decimal or `$` hex).
fn int_or(value: Option<&String>, default: i32) -> i32 {
    value
        .and_then(|v| crate::expression::parse_integer(v.trim()))
        .unwrap_or(default)
}

/// 505FE8 for the slots above, BGImageFilename and Font[i].IL/ILRate.
pub(super) fn load(o: &mut Object, settings: &BTreeMap<String, String>) {
    for (property, key, default) in INT_SETTINGS {
        let value = int_or(settings.get(&key.to_ascii_lowercase()), *default);
        o.properties.insert((*property).into(), vec![Value::Int(value)]);
    }
    // +1EC: an empty BGImageFilename becomes "none".
    let background = settings
        .get("bgimagefilename")
        .filter(|v| !v.is_empty())
        .cloned()
        .unwrap_or_else(|| "none".into());
    o.properties.insert("bgi".into(), vec![Value::Text(background)]);
    // +31C[i]: legacy ILRate (default 25) is stored as 100 - rate when
    // positive; a present Font[i].IL (default 75) overrides it. A key with an
    // empty value counts as absent. With neither the slot keeps 0.
    for index in 0..16 {
        let present = |suffix: &str| {
            settings
                .get(&format!("font[{index}].{suffix}"))
                .filter(|v| !v.is_empty())
        };
        let mut value = 0;
        if let Some(rate) = present("ilrate") {
            value = int_or(Some(rate), 25);
            if value > 0 {
                value = 100 - value;
            }
        }
        if let Some(il) = present("il") {
            value = int_or(Some(il), 75);
        }
        o.properties.insert(format!("fil/{index}"), vec![Value::Int(value)]);
    }
}

impl QlieVm {
    /// Game.txt boolean (4E7010): "true"/"false" ignoring case, else default.
    fn game_bool(&self, key: &str, default: bool) -> bool {
        match self
            .text
            .get("system/etc/game.txt")
            .into_iter()
            .flat_map(|s| s.lines())
            .filter_map(|line| line.split_once('='))
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
            .map(|(_, v)| v.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("true") => true,
            Some("false") => false,
            _ => default,
        }
    }
    fn game_text(&self, key: &str) -> Option<String> {
        self.text
            .get("system/etc/game.txt")
            .into_iter()
            .flat_map(|s| s.lines())
            .filter_map(|line| line.split_once('='))
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
            .map(|(_, v)| v.trim().to_owned())
    }
    fn cfg_int(&self, id: &str, property: &str) -> Result<i32> {
        let default = INT_SETTINGS
            .iter()
            .find(|(p, _, _)| *p == property)
            .map(|(_, _, d)| *d)
            .unwrap_or(0);
        self.objects[id]
            .properties
            .get(property)
            .and_then(|v| v.first())
            .map(Value::integer)
            .transpose()
            .map(|v| v.unwrap_or(default))
    }
    /// Engine setting read by the input device and other consumers. Before a
    /// script touches cfg the value comes from the same config text 505FE8
    /// reads at startup.
    pub(super) fn engine_setting(&mut self, property: &str) -> i32 {
        let (key, default) = INT_SETTINGS
            .iter()
            .find(|(p, _, _)| *p == property)
            .map(|(_, k, d)| (*k, *d))
            .expect("QLIE engine setting table");
        if let Some(value) = self
            .objects
            .get("cfg")
            .filter(|o| o.properties.contains_key("native/config_loaded"))
            .and_then(|o| o.properties.get(property))
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
        {
            return value;
        }
        if self.boot_settings.is_none() {
            let settings = self.config_settings_map();
            if !self.system_data.loaded {
                return int_or(settings.get(&key.to_ascii_lowercase()), default);
            }
            self.boot_settings = Some(settings);
        }
        int_or(
            self.boot_settings.as_ref().unwrap().get(&key.to_ascii_lowercase()),
            default,
        )
    }
    /// Returns false when `op` is not one of these engine-setting methods.
    pub(super) fn cfg_setting_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let n = |i: usize| -> Result<i32> {
            a.get(i).context("QLIE cfg setting argument")?.integer()
        };
        let set = |vm: &mut Self, property: &str, value: Value| {
            vm.objects
                .get_mut(id)
                .unwrap()
                .properties
                .insert(property.into(), vec![value]);
        };
        // ?nct reads the slot that mct writes (+1C0).
        let query = match op {
            "?nct" => Some("mct"),
            "?crc" | "?ttm" | "?mam" | "?nfs" | "?rfs" | "?dsf" | "?fdd" | "?kct" | "?ssd"
            | "?ssm" | "?mht" | "?vsw" | "?fcd" | "?fcs" | "?mre" | "?mrc" | "?mge" | "?swe"
            | "?swp" | "?swi" | "?tdm" => Some(&op[1..]),
            _ => None,
        };
        if let Some(property) = query {
            ensure!(a.is_empty(), "QLIE engine setting query ABI");
            let value = self.cfg_int(id, property)?;
            m.result(0, Value::Int(value));
            return Ok(true);
        }
        match op {
            "crc" | "rfs" | "nfs" => {
                ensure!(a.len() == 1, "QLIE engine flag setting ABI");
                set(self, op, Value::Int(i32::from(n(0)? != 0)));
            }
            "ttm" => {
                ensure!(a.len() == 1, "QLIE task tray mode ABI");
                set(self, op, Value::Int(n(0)?.clamp(0, 3)));
            }
            "fdd" => {
                ensure!(a.len() == 1, "QLIE inactive frame decay ABI");
                set(self, op, Value::Int(n(0)?.clamp(10, 100)));
            }
            "mrc" => {
                ensure!(a.len() == 1, "QLIE multicore count ABI");
                set(self, op, Value::Int(n(0)?.clamp(1, 8)));
            }
            // Presentation/buffer setters of the older loader table.
            "blt" | "gim" => {
                ensure!(a.len() == 1, "QLIE presentation setting ABI");
                set(self, op, Value::Int(n(0)?));
            }
            "fdm" => {
                ensure!(a.len() == 1, "QLIE fullscreen display mode ABI");
                set(self, op, Value::Int(i32::from(n(0)? != 0)));
            }
            "fps" => {
                ensure!(a.len() == 1, "QLIE frame limit ABI");
                set(self, op, Value::Int(n(0)?.max(10)));
            }
            "sb" => {
                ensure!(a.len() == 1, "QLIE stream buffer ABI");
                set(self, op, Value::Int(n(0)?.max(0x2000)));
            }
            "cs" => {
                ensure!(a.len() == 1, "QLIE cache size ABI");
                set(self, op, Value::Int(n(0)?.clamp(0, 0x20000000)));
            }
            // sfs is accepted and ignored by 6EA71C.
            "sfs" => {}
            "dsf" | "kct" | "ssd" | "ssm" | "mct" | "mht" | "vsw" | "fcd" | "fcs" | "mre"
            | "mge" | "swe" | "swp" | "swi" | "tdm" | "mam" => {
                ensure!(a.len() == 1, "QLIE engine setting ABI");
                set(self, op, Value::Int(n(0)?));
            }
            "mas" => {
                ensure!(a.len() == 2, "QLIE manual aspect size ABI");
                let w = if n(0)? < 320 { 800 } else { n(0)? };
                let h = if n(1)? < 240 { 600 } else { n(1)? };
                set(self, "mas/w", Value::Int(w));
                set(self, "mas/h", Value::Int(h));
            }
            "?mas" => {
                ensure!(a.is_empty(), "QLIE manual aspect query ABI");
                m.result(0, Value::Int(self.cfg_int(id, "mas/w")?));
                m.result(1, Value::Int(self.cfg_int(id, "mas/h")?));
            }
            // Only parameter 0 exists; other indices are ignored.
            "blp" => {
                ensure!(a.len() == 2, "QLIE blit parameter ABI");
                if n(0)? == 0 {
                    set(self, op, Value::Int(n(1)?.clamp(0, 100)));
                }
            }
            "?blp" => {
                ensure!(a.len() == 1, "QLIE blit parameter query ABI");
                if n(0)? == 0 {
                    m.result(0, Value::Int(self.cfg_int(id, "blp")?));
                }
            }
            "bgi" => {
                ensure!(a.len() == 1, "QLIE background image setting ABI");
                set(self, op, Value::Text(a[0].text()));
            }
            "?bgi" => {
                ensure!(a.is_empty(), "QLIE background image query ABI");
                let value = self.objects[id]
                    .properties
                    .get("bgi")
                    .and_then(|v| v.first())
                    .map(Value::text)
                    .unwrap_or_else(|| "none".into());
                m.result_text(0, value);
            }
            // +168 Game.txt DefaultFont (empty -> MS Gothic), resolved by
            // 4E9B30 against the installed faces with Tahoma as fallback.
            "?dfn" => {
                ensure!(a.is_empty(), "QLIE default font query ABI");
                let name = self
                    .game_text("DefaultFont")
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| "ＭＳ ゴシック".into());
                let fonts = h.font_library(self.client)?;
                let name = if fonts.resolve_alias(&name)?.is_some() {
                    name
                } else {
                    "Tahoma".into()
                };
                m.result_text(0, name);
            }
            // +31C[i]; Game.txt OldFontILRate (default true) exchanges the
            // stored interline value with the legacy 100 - rate form.
            "fil" => {
                ensure!(a.len() == 2, "QLIE font interline setting ABI");
                let index = n(0)?;
                if (0..16).contains(&index) {
                    let mut value = n(1)?;
                    if self.game_bool("OldFontILRate", true) && value > 0 {
                        value = 100 - value;
                    }
                    set(self, &format!("fil/{index}"), Value::Int(value.clamp(-1, 100)));
                }
            }
            "?fil" => {
                ensure!(a.len() <= 1, "QLIE font interline query ABI");
                if let Some(index) = a.first().map(Value::integer).transpose()? {
                    // Native answers only the first eight slots.
                    if (0..8).contains(&index) {
                        let mut value = self.cfg_int(id, &format!("fil/{index}"))?;
                        if self.game_bool("OldFontILRate", true) && value > 0 {
                            value = 100 - value;
                        }
                        m.result(0, Value::Int(value));
                    }
                } else {
                    for index in 0..16 {
                        let value = self.cfg_int(id, &format!("fil/{index}"))?;
                        m.result(index, Value::Int(value));
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl QlieVm {
    /// Lower-cased Name=Value pairs of the config 505FE8 reads: the user copy
    /// in the save mount, else the shipped System/config.txt.
    pub(super) fn config_settings_map(&self) -> BTreeMap<String, String> {
        self.system_data
            .config
            .as_ref()
            .or_else(|| {
                self.text
                    .iter()
                    .find(|(name, _)| {
                        name.as_str() == "system/config.txt" || name.ends_with("/system/config.txt")
                    })
                    .map(|(_, text)| text)
            })
            .map(|text| {
                text.lines()
                    .filter_map(|line| line.split_once('='))
                    .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim().to_owned()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
