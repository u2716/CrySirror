//! sys methods of 6D9434 beyond the core reader/lock/metadata set.
//!
//! System requests (`c h l m s qs ql rr ee fm e`) set the request byte at
//! engine+148 with a label at +14C; 708048 handles it after the frame's
//! input pass: a request with a default system label (`@@!config`, `@@!log`,
//! `@@!load`, `@@!menu`, `@@!save`, `@@!quicksave`, `@@!quickload`,
//! `@@!reset`, `@@!ext`) always calls a label through 70A1D8, the given one
//! first, and is not subject to the input locks. `e,0` ends the program
//! (707A1C mode 2), `e,1` closes the main form, `fm` minimizes (70A0B0).
//! Thread fields keep their 6D5D38 defaults: run +168 0x10000, out +170
//! 0xFFFFFFF, tz +180/+184 0xFFFF/0, ss +2B5 0.
use super::*;

/// 6D925C: object kinds whose module answers `?mod` with 1.
const MODULES: &[&str] = &[
    "img", "msg", "btn", "bar", "simg", "sbtn", "file", "list", "table", "theora", "timer",
];
/// 6DB1A0..: event names stored at +D4/+DC/+D8/+E0/+E4.
const EVENTS: &[&str] = &["onframe", "onscreen", "onclose", "onminimize", "onunminimize"];

impl QlieVm {
    fn sys_int(&self, id: &str, key: &str, default: i32) -> i32 {
        self.objects[id]
            .properties
            .get(key)
            .and_then(|v| v.first())
            .and_then(|v| v.integer().ok())
            .unwrap_or(default)
    }
    fn sys_text(&self, id: &str, key: &str) -> String {
        self.objects[id]
            .properties
            .get(key)
            .and_then(|v| v.first())
            .map(Value::text)
            .unwrap_or_default()
    }
    /// Returns false when `op` is not handled here.
    pub(super) fn sys_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let n = |i: usize| -> Result<i32> {
            a.get(i).map(Value::integer).transpose().map(|v| v.unwrap_or(0))
        };
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        let set = |vm: &mut Self, key: &str, values: Vec<Value>| {
            vm.objects
                .get_mut(id)
                .unwrap()
                .properties
                .insert(key.into(), values);
        };
        let request = match op {
            "c" => Some("config"),
            "h" => Some("log"),
            "l" => Some("load"),
            "m" => Some("menu"),
            "s" => Some("save"),
            "qs" => Some("quicksave"),
            "ql" => Some("quickload"),
            "rr" => Some("reset"),
            "ee" => Some("exit"),
            _ => None,
        };
        if let Some(action) = request {
            ensure!(a.len() <= 1, "QLIE system request ABI");
            self.system_request = Some(thread::Callback {
                action: action.into(),
                label: s(0),
                file: String::new(),
                same_reader: false,
            });
            return Ok(true);
        }
        match op {
            // r,label,file: request 8 with "label,file" (707F80); handled
            // by reset_engine after the frame's input pass.
            "msd" | "?msd" => self.frame_wait_method(h, m, op, a)?,
            "r" => {
                ensure!(a.len() <= 2, "QLIE engine reset ABI");
                self.system_request = Some(thread::Callback {
                    action: "engine_reset".into(),
                    label: s(0),
                    file: s(1),
                    same_reader: false,
                });
            }
            "e" => {
                ensure!(a.len() <= 1, "QLIE system end ABI");
                // Mode 2 ends the program; mode 1 closes the main form,
                // which in this host also ends the client.
                self.halt_requested = true;
            }
            "fm" => {
                self.submit(h, json!({"ui":[{"op":"set_window_minimized","minimized":true}]}))?;
            }
            // 70527C empties the two resource caches (4EF768 sets their
            // capacity to 0 and back); engine-owned caches have no VM state.
            "cr" => {}
            // ref (6B9BB4 + 567124) forces a screen redraw; the engine
            // presents every frame.
            "ref" => {}
            "ckc" => set(self, "ckc", vec![Value::Int(n(0)?)]),
            "?ckc" => m.result(0, Value::Int(self.sys_int(id, "ckc", 0))),
            "?mod" => {
                ensure!(a.len() == 1, "QLIE module query ABI");
                m.result(0, Value::Int(i32::from(MODULES.contains(&s(0).as_str()))));
            }
            // 549334/549328: presentation rectangle size of the screen.
            "?ods" => {
                m.result(0, Value::Int(i32::try_from(self.config.canvas[0])?));
                m.result(1, Value::Int(i32::try_from(self.config.canvas[1])?));
            }
            // StrToIntDef("564", 0): the engine revision.
            "?rev" => m.result(0, Value::Int(564)),
            // Presenter blit mode 0..4 (cfg BitBltMode in this host).
            "?blt" => {
                let blt = self
                    .objects
                    .get("cfg")
                    .and_then(|o| o.properties.get("blt"))
                    .and_then(|v| v.first())
                    .and_then(|v| v.integer().ok())
                    .unwrap_or_else(|| {
                        self.config_settings_map()
                            .get("bitbltmode")
                            .and_then(|v| crate::expression::parse_integer(v))
                            .unwrap_or(0)
                    });
                m.result(0, Value::Int(blt));
            }
            "run" => {
                ensure!(a.len() == 1, "QLIE run speed ABI");
                let v = n(0)?;
                set(self, "run", vec![Value::Int(if v < 1 { 0x10000 } else { v })]);
            }
            "?run" => m.result(0, Value::Int(self.sys_int(id, "run", 0x10000))),
            "out" => {
                let v = if a.is_empty() { 0xFFFFFF } else { n(0)? };
                set(self, "out", vec![Value::Int(v)]);
            }
            "?out" => m.result(0, Value::Int(self.sys_int(id, "out", 0xFFFFFFF))),
            "?tz" => {
                let tz = self.objects[id].properties.get("tz").cloned().unwrap_or_default();
                for (i, default) in [0xFFFF, 0].into_iter().enumerate() {
                    m.result(i, tz.get(i).cloned().unwrap_or(Value::Int(default)));
                }
            }
            "ss" => set(self, "ss", vec![Value::Int(i32::from(n(0)? != 0))]),
            "?ss" => {
                if self.sys_int(id, "ss", 0) != 0 {
                    m.result(0, Value::Int(1));
                }
            }
            "v" => set(self, "v", vec![Value::Text(s(0))]),
            "?v" => m.result_text(0, self.sys_text(id, "v")),
            // fc stores a process-wide string (+741998 global).
            "fc" => {
                self.global_strings.insert("fc".into(), s(0));
            }
            "?fc" => {
                m.result_text(0, self.global_strings.get("fc").cloned().unwrap_or_default());
            }
            // tc/?tc: the main form caption (4E13E0/4E1300).
            "tc" => {
                let title = s(0);
                self.submit(h, json!({"ui":[{"op":"set_title","text":title}]}))?;
                self.global_strings.insert("tc".into(), title);
            }
            "?tc" => {
                let title = self
                    .global_strings
                    .get("tc")
                    .cloned()
                    .or_else(|| self.game_setting_text("FormCaption"))
                    .unwrap_or_default();
                m.result_text(0, title);
            }
            "eve" => {
                ensure!(a.len() == 2, "QLIE system event ABI");
                let name = s(0).to_ascii_lowercase();
                // Unknown names are ignored natively.
                if EVENTS.contains(&name.as_str()) {
                    let reader = m.reader_path().to_owned();
                    set(self, &format!("eve:{name}"), vec![a[1].clone(), Value::Text(reader)]);
                }
            }
            "?eve" => {
                ensure!(a.len() == 1, "QLIE system event query ABI");
                let name = s(0).to_ascii_lowercase();
                if EVENTS.contains(&name.as_str()) {
                    let label = self.objects[id]
                        .properties
                        .get(&format!("eve:{name}"))
                        .and_then(|v| v.first())
                        .map(Value::text)
                        .unwrap_or_default();
                    m.result_text(0, label);
                }
            }
            // srr executes "\ret,<sr label>" (6E6704) on this thread.
            "srr" => {
                let label = self.sys_text(id, "sr");
                ensure!(
                    m.return_named((!label.is_empty()).then_some(label.as_str()))?,
                    "QLIE srr outside subroutine"
                );
            }
            // hdr/?hdr: the master output level limiter (5BFBE8 on the
            // mixer's +28 stage): min/max level, attack and release rates and
            // the current gain. 5C3670 initialises it to 7FFF/17FFD/500/44100
            // with gain = min. The values are kept; the mix is not limited.
            "hdr" => {
                ensure!(a.len() <= 5, "QLIE output limiter ABI");
                let mut low = n(0)?;
                let mut high = n(1)?;
                if low < 256 {
                    low = 0x7FFF;
                }
                if high < 256 {
                    high = 98301;
                }
                let attack = if n(2)? == 0 { 500 } else { n(2)? }.max(1);
                let release = if n(3)? == 0 { 44100 } else { n(3)? }.max(1);
                if low > high {
                    high = low;
                }
                let mut gain = self.limiter.map_or(0x7FFF, |l| l[2]);
                if n(4)? != 0 {
                    gain = n(4)?.max(low);
                }
                self.limiter = Some([low, high, gain, attack, release]);
            }
            "?hdr" => {
                let [low, high, gain, attack, release] =
                    self.limiter.unwrap_or([0x7FFF, 98301, 0x7FFF, 500, 44100]);
                for (i, v) in [gain, low, high, attack, release].into_iter().enumerate() {
                    m.result(i, Value::Int(v));
                }
            }
            // ?mpd (703688): reader position as file, 1-based line and label.
            "?mpd" => {
                ensure!(a.len() <= 1, "QLIE reader position query ABI");
                let frame = &m.frame;
                let label = m.programs[&frame.path]
                    .labels
                    .iter()
                    .filter(|(_, pc)| **pc <= frame.pc)
                    .max_by_key(|(_, pc)| **pc)
                    .map(|(s, _)| s.clone())
                    .unwrap_or_default();
                let (path, line) = (frame.path.clone(), i32::try_from(frame.pc)? + 1);
                m.result_text(0, path);
                m.result(0, Value::Int(line));
                m.result_text(1, label);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    pub(super) fn game_setting_text(&self, key: &str) -> Option<String> {
        self.text
            .get("system/etc/game.txt")
            .into_iter()
            .flat_map(|s| s.lines())
            .filter_map(|line| line.split_once('='))
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
            .map(|(_, v)| v.trim().to_owned())
    }
}
