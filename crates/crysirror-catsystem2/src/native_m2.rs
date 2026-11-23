// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Character planes (task class 54, `m2plane`). The original loads an E-mote body through
//! its m2config profile; here `.pfl`/`.dif` policy stays in the adapter while the model,
//! its clock and its mesh come from the shared eluna puppet.
use anyhow::{ensure, Context, Result};
use crysirror_core::{path::{Mount, VfsPath}, service::{ClientStorage, HostServices}, state::ClientId};
use crysirror_puppet::{PlayOptions, PuppetSource};
use crysirror_puppet_eluna::ElunaPuppet;
use crysirror_tasks::{Task, TaskPool};
use std::collections::BTreeMap;

/// `.pfl` rows and the `.dif` fields a character profile applies.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Profile {
    pub bodies: BTreeMap<u32, String>,
    pub waits: BTreeMap<(u32, i32), (String, String)>,
    pub scale: Option<[f32; 2]>,
    pub position: Option<[f32; 2]>,
    pub scale_offset: Option<[f32; 2]>,
    pub body_difs: BTreeMap<u32, String>,
}
fn sections(text: &str) -> Vec<(String, Vec<Vec<String>>)> {
    let mut out: Vec<(String, Vec<Vec<String>>)> = Vec::new();
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('#') {
            out.push((name.trim().to_owned(), Vec::new()));
        } else if let Some((_, rows)) = out.last_mut() {
            rows.push(line.split_whitespace().map(str::to_owned).collect());
        }
    }
    out
}
impl Profile {
    pub fn parse_pfl(text: &str) -> Self {
        let mut out = Self::default();
        for (name, rows) in sections(text) {
            for row in rows {
                match (name.as_str(), row.as_slice()) {
                    ("charaname", [id, _, file, ..]) => {
                        if let Ok(id) = id.parse() {
                            out.bodies.insert(id, file.clone());
                        }
                    }
                    ("motname", [body, wait, motion, dif, rest @ ..]) => {
                        if let (Ok(body), Ok(wait)) = (body.parse(), wait.parse()) {
                            out.waits.insert((body, wait), (motion.clone(), dif.clone()));
                            if let Some(second) = rest.first() {
                                out.body_difs.insert(body, second.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }
    pub fn apply_dif(&mut self, text: &str) {
        let pair = |row: &[String]| -> Option<[f32; 2]> { Some([row.first()?.parse().ok()?, row.get(1)?.parse().ok()?]) };
        for (name, rows) in sections(text) {
            match name.as_str() {
                "scale" => self.scale = rows.first().and_then(|row| pair(row)).or(self.scale),
                "position" => self.position = rows.first().and_then(|row| pair(row)).or(self.position),
                "scaleoffset" => {
                    if let Some(row) = rows.first() {
                        let mut offset = self.scale_offset.unwrap_or([0.; 2]);
                        for (index, value) in row.iter().take(2).enumerate() {
                            if let Ok(value) = value.parse() { offset[index] = value; }
                        }
                        self.scale_offset = Some(offset);
                    }
                }
                _ => {}
            }
        }
    }
}
/// A `.dif` difference table: `#dif` digit positions, each `#pattern` key's variable
/// values, the `#chain` transitions between keys and the `#blink` schedule.
/// The E-mote configuration (B9BD98), reached by FES `m2_getconf`/`m2_setconf`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct M2Config {
    motion: u32,
    eff_dif: u32,
    eff_iner: u32,
    eff_blink: u32,
    eff_lip: u32,
    fade: u32,
    interleave: u32,
    /// `iner_lv` per `#inertia_lv` label, in percent of the base material.
    levels: BTreeMap<String, u32>,
    /// `iner_enable` per `#inertia_enable` label.
    enables: BTreeMap<String, u32>,
}
impl Default for M2Config {
    /// 5816B0.
    fn default() -> Self {
        Self { motion: 1, eff_dif: 1, eff_iner: 1, eff_blink: 1, eff_lip: 1, fade: 0, interleave: 1, levels: BTreeMap::new(), enables: BTreeMap::new() }
    }
}
impl M2Config {
    /// 786360; an unknown key answers nothing.
    pub fn get(&self, key: &[u8], label: &str) -> Option<u32> {
        Some(match key {
            b"motion" => self.motion,
            b"eff_dif" => self.eff_dif,
            b"eff_iner" => self.eff_iner,
            b"eff_blink" => self.eff_blink,
            b"eff_lip" => self.eff_lip,
            b"fade" => self.fade,
            b"interleave" => self.interleave,
            b"iner_lv" => self.levels.get(label).copied().unwrap_or(100),
            b"iner_enable" => self.enables.get(label).copied().unwrap_or(1),
            _ => return None,
        })
    }
    /// 786840: an absent value is the key's default.
    pub fn set(&mut self, key: &[u8], value: Option<u32>, label: &str) {
        let one = value.unwrap_or(1);
        match key {
            b"motion" => self.motion = one,
            b"eff_dif" => self.eff_dif = one,
            b"eff_iner" => self.eff_iner = one,
            b"eff_blink" => self.eff_blink = one,
            b"eff_lip" => self.eff_lip = one,
            b"fade" => self.fade = one,
            b"interleave" => self.interleave = value.unwrap_or(0),
            b"iner_lv" => {
                self.levels.insert(label.to_owned(), value.unwrap_or(100));
            }
            b"iner_enable" => {
                self.enables.insert(label.to_owned(), one);
            }
            b"reset" => *self = Self::default(),
            _ => {}
        }
    }
}
const M2_CONFIG_FILE: &str = "m2config.json";
/// Loads the options saved by `save_m2_config`; this file is this adapter's own form.
pub(crate) fn load_m2_config(storage: &dyn ClientStorage) -> Result<M2Config> {
    let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save, M2_CONFIG_FILE)?;
    match storage.read(&path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).context("CatSystem2 E-mote options file")?),
        Err(crysirror_core::Error::NotFound) => Ok(M2Config::default()),
        Err(error) => Err(error.into()),
    }
}
pub(crate) fn save_m2_config(storage: &dyn ClientStorage, config: &M2Config) -> Result<()> {
    let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save, M2_CONFIG_FILE)?;
    storage.write_atomic(&path, &serde_json::to_vec(config)?)?;
    Ok(())
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Dif {
    patterns: BTreeMap<usize, BTreeMap<String, Vec<(String, f32)>>>,
    shifts: BTreeMap<usize, BTreeMap<String, Vec<Shift>>>,
    chains: BTreeMap<usize, Chain>,
    blinks: Vec<Blink>,
    /// `#lip_table`, `#lip_variable`, `#lip_flow` by (digit, key).
    lips: BTreeMap<(usize, String), Lip>,
    /// `#attribute`: number → {x, y, extra} (587790).
    attributes: BTreeMap<u32, [i32; 3]>,
    /// `#touch`: colour → table names (587A70), and each table's {action, voice} rows.
    touch: BTreeMap<u32, Vec<String>>,
    touch_tables: BTreeMap<String, Vec<(String, String)>>,
    /// `#material`, `#material_min`, `#material_max`, `#inertia_*` (5A75E0).
    materials: [BTreeMap<String, crysirror_puppet_eluna::inertia::SpringMaterial>; 3],
    inertia_horizontal: Vec<String>,
    inertia_vertical: Vec<String>,
    inertia_links: Vec<crysirror_puppet_eluna::inertia::SpringLink>,
    /// `#inertia_lv` and `#inertia_enable`: `%label` then its variables.
    level_labels: BTreeMap<String, Vec<String>>,
    enable_labels: BTreeMap<String, Vec<String>>,
    /// Waiting-wave definition files selected by native 965.
    waiting_files: Vec<String>,
}
#[derive(Clone, Debug)]
struct Shift { variable: String, value: f32, curve: String, frames: u32 }
struct PatternShiftRun {
    transitions: BTreeMap<((bool, usize), String), crysirror_puppet_eluna::modulation::ScalarTransition>,
    values: BTreeMap<((bool, usize), String), f32>,
    applied: BTreeMap<String, f32>,
}
impl Default for PatternShiftRun {
    fn default() -> Self { Self { transitions: BTreeMap::new(), values: BTreeMap::new(), applied: BTreeMap::new() } }
}
#[derive(Clone, Debug, Default)]
struct Lip {
    table: Vec<f32>,
    variable: String,
    /// {pattern, frames, may stop here once the voice ended}.
    flow: Vec<(usize, f32, bool)>,
}
/// One step of an `action.def` macro; a macro's frames and waits run in order.
#[derive(Clone, Debug, PartialEq)]
enum ActionStep {
    Frame { frames: f32, vars: Vec<(String, f32)>, restore: bool },
    Wait(f32),
}
/// `#action`: `$name`, then `frame n` with its `var name value` lines, `wait n` and
/// `restore`, which returns what the macro changed. `easing` has no shared equivalent:
/// E-mote parameter transitions here are linear.
fn parse_actions(text: &str) -> BTreeMap<String, Vec<ActionStep>> {
    let mut out: BTreeMap<String, Vec<ActionStep>> = BTreeMap::new();
    let mut current = None::<String>;
    let mut comment = false;
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if comment {
            comment = !line.contains("*/");
            continue;
        }
        if line.starts_with("/*") {
            comment = !line.contains("*/");
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(first) = words.first() else { continue };
        if let Some(name) = first.strip_prefix('$') {
            current = Some(name.to_owned());
            out.entry(name.to_owned()).or_default();
            continue;
        }
        let Some(steps) = current.as_ref().and_then(|name| out.get_mut(name)) else { continue };
        let number = words.get(1).and_then(|w| w.parse::<f32>().ok());
        match *first {
            "frame" => steps.push(ActionStep::Frame { frames: number.unwrap_or(0.), vars: Vec::new(), restore: false }),
            "wait" => steps.push(ActionStep::Wait(number.unwrap_or(0.))),
            "restore" => {
                if let Some(ActionStep::Frame { restore, .. }) = steps.last_mut() {
                    *restore = true;
                }
            }
            "var" => {
                if let (Some(ActionStep::Frame { vars, .. }), Some(name), Some(value)) = (steps.last_mut(), words.get(1), words.get(2).and_then(|w| w.parse::<f32>().ok())) {
                    vars.push(((*name).to_owned(), value));
                }
            }
            _ => {}
        }
    }
    out
}
#[derive(Clone, Debug, Default)]
struct Chain {
    default_frames: f32,
    between: BTreeMap<(String, String), f32>,
}
#[derive(Clone, Debug)]
struct Blink {
    position: usize,
    key: String,
    from: f32,
    to: f32,
    speed: f32,
    wait: [u32; 2],
}
impl Dif {
    pub fn parse(&mut self, text: &str) {
        for (name, rows) in sections(text) {
            match name.as_str() {
                "pattern" => {
                    let mut current: Option<(usize, String)> = None;
                    for row in rows {
                        if let Some(position) = row[0].strip_prefix('%') {
                            current = position.parse().ok().zip(row.get(1).cloned());
                            if let Some((position, key)) = &current {
                                self.patterns.entry(*position).or_default().insert(key.clone(), Vec::new());
                            }
                        } else if let (Some((position, key)), Some(value)) = (&current, row.get(1).and_then(|v| v.parse().ok())) {
                            self.patterns.get_mut(position).and_then(|keys| keys.get_mut(key)).unwrap().push((row[0].clone(), value));
                        }
                    }
                }
                "pattern_shift" => {
                    let mut current: Option<(usize, String)> = None;
                    for row in rows {
                        if let Some(position) = row[0].strip_prefix('%') {
                            current = position.parse().ok().zip(row.get(1).cloned());
                        } else if let (Some((position, key)), Some(value)) = (&current, row.get(1).and_then(|value| value.parse().ok())) {
                            self.shifts.entry(*position).or_default().entry(key.clone()).or_default().push(Shift { variable: row[0].clone(), value, curve: row.get(2).cloned().unwrap_or_default(), frames: row.get(3).and_then(|value| value.parse().ok()).unwrap_or(0) });
                        }
                    }
                }
                "chain" => self.parse_chains(&rows),
                "waiting" => self.waiting_files.extend(rows.into_iter().filter_map(|row| row.into_iter().next())),
                "material" | "material_min" | "material_max" => {
                    let table = match name.as_str() { "material" => 0, "material_min" => 1, _ => 2 };
                    // 5A75E0: rows of nine columns; shorter ones are skipped.
                    for row in rows.iter().filter(|row| row.len() >= 9) {
                        let number = |at: usize| row[at].parse::<f32>().unwrap_or(0.);
                        self.materials[table].insert(row[0].clone(), crysirror_puppet_eluna::inertia::SpringMaterial {
                            mass: number(1),
                            spring: number(2),
                            resistance: number(3),
                            time_step: number(4),
                            max_travel: number(5),
                            invert: row[6].parse::<i32>().unwrap_or(0) != 0,
                            scale: number(7),
                            shift: number(8),
                        });
                    }
                }
                "inertia_lr" => self.inertia_horizontal.extend(rows.iter().map(|row| row[0].clone())),
                "inertia_ud" => self.inertia_vertical.extend(rows.iter().map(|row| row[0].clone())),
                "inertia_lv" | "inertia_enable" => {
                    let table = if name == "inertia_lv" { &mut self.level_labels } else { &mut self.enable_labels };
                    let mut label = String::new();
                    for row in &rows {
                        match row[0].strip_prefix('%') {
                            Some(found) => label = found.to_owned(),
                            None => table.entry(label.clone()).or_default().push(row[0].clone()),
                        }
                    }
                }
                "inertia_dif" => {
                    for row in rows.iter().filter(|row| row.len() >= 4) {
                        self.inertia_links.push(crysirror_puppet_eluna::inertia::SpringLink {
                            source: row[0].clone(),
                            target: row[1].clone(),
                            gain: row[2].parse().unwrap_or(0.),
                            divisor: row[3].parse().unwrap_or(1.),
                        });
                    }
                }
                "lip_table" => {
                    for row in rows {
                        if let (Some(position), Some(key), true) = (row[0].parse::<usize>().ok(), row.get(1), row.len() > 5) {
                            self.lips.entry((position, key.clone())).or_default().table = row[5..].iter().filter_map(|v| v.parse().ok()).collect();
                        }
                    }
                }
                "lip_variable" => {
                    for row in rows {
                        if let (Some(position), Some(key), Some(variable)) = (row[0].parse::<usize>().ok(), row.get(1), row.get(2)) {
                            self.lips.entry((position, key.clone())).or_default().variable = variable.clone();
                        }
                    }
                }
                "lip_flow" => {
                    for row in rows {
                        if let (Some(position), Some(key), true) = (row[0].parse::<usize>().ok(), row.get(1), row.len() > 2) {
                            self.lips.entry((position, key.clone())).or_default().flow = row[2..].iter().filter_map(|step| {
                                let mut parts = step.split(',');
                                Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.trim() == "1"))
                            }).collect();
                        }
                    }
                }
                "attribute" => {
                    for row in rows {
                        if let [number, x, y, extra, ..] = row.as_slice() {
                            if let (Ok(number), Ok(x), Ok(y), Ok(extra)) = (number.parse(), x.parse(), y.parse(), extra.parse()) {
                                self.attributes.insert(number, [x, y, extra]);
                            }
                        }
                    }
                }
                "touch" => {
                    let mut table = None::<String>;
                    for row in rows {
                        if let Some(colour) = row[0].strip_prefix("0x").and_then(|hex| u32::from_str_radix(hex, 16).ok()) {
                            self.touch.insert(colour, row[1..].iter().map(|t| t.trim_start_matches('$').to_owned()).collect());
                        } else if let Some(name) = row[0].strip_prefix('$') {
                            self.touch_tables.entry(name.to_owned()).or_default();
                            table = Some(name.to_owned());
                        } else if let Some(table) = &table {
                            self.touch_tables.get_mut(table).unwrap().push((row[0].clone(), row.get(1).cloned().unwrap_or_default()));
                        }
                    }
                }
                "blink" => {
                    for row in rows {
                        if let [position, key, from, to, speed, low, high, ..] = row.as_slice() {
                            if let (Ok(position), Ok(from), Ok(to), Ok(speed), Ok(low), Ok(high)) = (position.parse(), from.parse(), to.parse(), speed.parse(), low.parse(), high.parse()) {
                                self.blinks.push(Blink { position, key: key.clone(), from, to, speed, wait: [low, high] });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    fn parse_chains(&mut self, rows: &[Vec<String>]) {
        let mut macros: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut current: Vec<usize> = Vec::new();
        for row in rows {
            match row.as_slice() {
                [name] if name.starts_with('$') => current = macros.get(name).cloned().unwrap_or_default(),
                [position, name] if name.starts_with('$') => {
                    if let Ok(position) = position.parse() {
                        macros.entry(name.clone()).or_default().push(position);
                    }
                }
                [position, _, frames] if position.parse::<usize>().is_ok() => {
                    let chain = self.chains.entry(position.parse().unwrap()).or_default();
                    chain.default_frames = frames.parse().unwrap_or(0.);
                }
                [word, value] if word == "frame" => {
                    for position in &current {
                        self.chains.entry(*position).or_default().default_frames = value.parse().unwrap_or(0.);
                    }
                }
                [from, to, _, frames] => {
                    for position in &current {
                        self.chains.entry(*position).or_default().between.insert((from.clone(), to.clone()), frames.parse().unwrap_or(0.));
                    }
                }
                _ => {}
            }
        }
    }
    /// Frames of the transition at `position` from one key to another; `fix` of one frame is immediate.
    fn frames(&self, position: usize, from: &str, to: &str) -> f32 {
        let Some(chain) = self.chains.get(&position) else { return 0. };
        chain.between.get(&(from.to_owned(), to.to_owned())).copied().unwrap_or(chain.default_frames)
    }
}
/// `4,3,0,5,0:0`: one key per digit position, in order.
pub(crate) fn digit_keys(words: &str) -> Vec<String> {
    words.split([',', ':']).map(|word| word.trim().to_owned()).collect()
}
fn apply_keys(puppet: &mut ElunaPuppet, dif: &Dif, current: &mut BTreeMap<usize, String>, scales: &BTreeMap<String, f32>, shifts: &mut PatternShiftRun, namespace: bool, keys: &[String]) -> Result<()> {
    for (index, key) in keys.iter().enumerate() {
        let position = index + 1;
        if key.is_empty() || current.get(&position) == Some(key) {
            continue;
        }
        let from = current.insert(position, key.clone()).unwrap_or_default();
        let frames = dif.frames(position, &from, key);
        let selected = dif.shifts.get(&position).and_then(|keys| keys.get(key));
        let mut next = BTreeMap::new();
        for shift in selected.into_iter().flatten() {
            let curve = match shift.curve.as_str() {
                "" | "linear" => crysirror_puppet_eluna::modulation::Curve::Linear,
                "sin_fs" => crysirror_puppet_eluna::modulation::Curve::SineOut,
                "sin_sf" => crysirror_puppet_eluna::modulation::Curve::SineIn,
                "sin_ss" => crysirror_puppet_eluna::modulation::Curve::SineInOut,
                "nomove" => crysirror_puppet_eluna::modulation::Curve::Hold,
                other => anyhow::bail!("CatSystem2 pattern_shift curve {other} is not implemented"),
            };
            // 5879D0 copies row+36 to the last argument of 56DE30;
            // it is the per-variable duration at +68, not a condition index.
            next.insert(shift.variable.clone(), (shift.value, curve, shift.frames as f32));
        }
        let previous: Vec<_> = shifts.values.keys().filter(|(digit, _)| *digit == (namespace, position)).map(|(_, name)| name.clone()).collect();
        for name in previous { next.entry(name).or_insert((0., crysirror_puppet_eluna::modulation::Curve::Linear, 0.)); }
        for (name, (value, curve, duration)) in next {
            let id = ((namespace, position), name);
            let from = shifts.values.get(&id).copied().unwrap_or(0.);
            shifts.transitions.insert(id.clone(), crysirror_puppet_eluna::modulation::ScalarTransition::new(from, value, duration, curve));
            shifts.values.entry(id).or_default();
        }
        let Some(values) = dif.patterns.get(&position).and_then(|keys| keys.get(key)) else { continue };
        for (variable, value) in values {
            // 99 marks a digit's "無し" row: it names no value of the variable.
            if *value != 99. && puppet.parameter_value(variable).is_some() {
                let scale = scales.get(variable).copied().unwrap_or(1.);
                puppet.set_parameter(variable, value * scale, frames * 1000. / 60.)?;
            }
        }
    }
    Ok(())
}
fn text_asset(storage: &dyn ClientStorage, name: &str) -> Result<String> {
    let path = VfsPath::new(Mount::Asset, &format!("m2config.int/{name}"))?;
    let path = crate::native_resource_paths::resolve(storage, &path)?.with_context(|| format!("CatSystem2 m2config {name} missing"))?;
    let bytes = crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 16 * 1024 * 1024)?;
    Ok(encoding_rs::SHIFT_JIS.decode(&bytes).0.into_owned())
}
#[derive(Clone, Debug)]
struct WaveSpec {
    order: usize,
    waveform: u32,
    negative: Vec<[f32; 2]>,
    positive: Vec<[f32; 2]>,
    reverse: bool,
    duty: f32,
    period: Vec<[f32; 2]>,
}
fn wave_ranges(text: &str) -> Result<Vec<[f32; 2]>> {
    text.split(',').map(|value| {
        let (start, end) = value.split_once('-').unwrap_or((value, value));
        let start: f32 = start.parse().context("CatSystem2 waiting range start")?;
        let end: f32 = end.parse().context("CatSystem2 waiting range end")?;
        ensure!(start.is_finite() && end.is_finite(), "CatSystem2 non-finite waiting range");
        Ok([start, end])
    }).collect()
}
fn parse_waiting(text: &str) -> Result<BTreeMap<String, BTreeMap<String, WaveSpec>>> {
    let mut out = BTreeMap::new();
    for (label, rows) in sections(text) {
        let mut period = Vec::new();
        let mut mode = "";
        let mut variables = BTreeMap::new();
        for (order, row) in rows.into_iter().enumerate() {
            match row[0].as_str() {
                "%time" => { mode = "time"; continue; }
                "%set" => { mode = "set"; continue; }
                _ => {}
            }
            if mode == "time" { period = wave_ranges(&row[0])?; mode = ""; }
            else if mode == "set" && row.len() >= 3 {
                let negative = wave_ranges(&row[2])?;
                let positive = row.get(3).map(|value| wave_ranges(value)).transpose()?.unwrap_or_else(|| negative.clone());
                let waveform = row[1].parse().context("CatSystem2 waiting waveform")?;
                ensure!(waveform <= 2, "CatSystem2 unsupported waiting waveform {waveform}");
                let reverse = row.get(4).map(|value| value.parse::<i32>()).transpose()?.unwrap_or(0) != 0;
                let duty = row.get(5).map(|value| value.parse::<f32>()).transpose()?.unwrap_or(50.) / 100.;
                let period = row.get(6).map(|value| wave_ranges(value)).transpose()?.unwrap_or_else(|| period.clone());
                ensure!(duty.is_finite() && period.iter().flatten().all(|value| *value >= 0. && *value < u32::MAX as f32), "CatSystem2 invalid waiting period/duty");
                variables.insert(row[0].clone(), WaveSpec { order, waveform, negative, positive, reverse, duty, period });
            }
        }
        out.insert(label, variables);
    }
    Ok(out)
}
fn wave_random(random: &mut crate::motion::RandomStreams, client:ClientId, draw:crysirror_puppet_eluna::modulation::RandomDraw)->Result<f64>{
    use crysirror_puppet_eluna::modulation::RandomDraw;
    match draw {RandomDraw::Index=>Ok(f64::from(random.integer_word(client)?)),RandomDraw::Unit=>Ok(f64::from(random.float_value(client)?))}
}
fn sample_wave(spec: &WaveSpec, sampler: &mut crysirror_puppet_eluna::modulation::CorrelatedRanges, random: &mut crate::motion::RandomStreams, client:ClientId) -> Result<crysirror_puppet_eluna::modulation::Oscillator> {
    let negative = sampler.sample(0, &spec.negative, |draw| wave_random(random,client,draw))?;
    let positive = sampler.sample(1, &spec.positive, |draw| wave_random(random,client,draw))?;
    let period = sampler.sample(2, &spec.period, |draw| wave_random(random,client,draw))? as u32;
    // 5992F0's cosine modes divide elapsed frames by period. A zero period
    // feeds NaN/Inf to cos; it cannot be acknowledged as a zero-valued motion.
    ensure!(period != 0 || spec.waveform == 0, "CatSystem2 zero-period cosine waiting waveform is non-finite");
    Ok(crysirror_puppet_eluna::modulation::Oscillator {
        waveform: spec.waveform, negative, positive,
        phase: if spec.reverse { 0. } else { std::f32::consts::PI },
        duty: spec.duty, period,
    })
}
struct WaitingRun {
    bank: crysirror_puppet_eluna::modulation::ModulationBank,
    variables: BTreeMap<String, WaveSpec>,
    applied: BTreeMap<String, f32>,
}
pub(crate) struct Model {
    puppet: ElunaPuppet,
    scale: [f32; 2],
    position: [f32; 2],
    scale_offset: [f32; 2],
    textures: Vec<(u32, u32, Vec<u8>)>,
    expression: Dif,
    bodies: Dif,
    actions: BTreeMap<String, Vec<ActionStep>>,
    initial: crysirror_puppet_eluna::PlaybackCheckpoint,
    waiting: BTreeMap<String, BTreeMap<String, WaveSpec>>,
}
impl Model {
    // 587BA0 reads the DIF position/scaleoffset. 574410 supplies the script
    // coordinate, and 574070 sends their sum plus offset*(1-scriptScale) to
    // IEmotePlayer::SetCoord. The DIF position is not scaled model geometry.
    fn anchor(&self, pose: &crate::task_policy::PlanePose) -> [f32; 2] {
        [
            pose.base[0] + self.position[0] + self.scale_offset[0] * (1. - pose.scale[0]),
            pose.base[1] + self.position[1] + self.scale_offset[1] * (1. - pose.scale[0]),
        ]
    }
}
/// A `.dif` with its `#include` files and comments folded in.
fn dif_text(storage: &dyn ClientStorage, name: &str) -> Result<String> {
    fn strip(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find("/*") {
            out.push_str(&rest[..start]);
            rest = rest[start + 2..].find("*/").map_or("", |end| &rest[start + 2 + end + 2..]);
        }
        out.push_str(rest);
        out
    }
    let text = strip(&text_asset(storage, name)?);
    let mut out = String::new();
    for (section, rows) in sections(&text) {
        if section == "include" {
            for row in rows {
                out.push_str(&strip(&text_asset(storage, &row[0])?));
                out.push('\n');
            }
        }
    }
    out.push_str(&text);
    Ok(out)
}
fn load(storage: &dyn ClientStorage, spec: &str, check: &dyn Fn() -> Result<()>) -> Result<Model> {
    let mut words = spec.split(',');
    let name = words.next().unwrap_or("");
    let body: u32 = words.next().unwrap_or("1").parse().unwrap_or(1);
    let wait: i32 = words.next().unwrap_or("0").parse().unwrap_or(0);
    let mut profile = Profile::parse_pfl(&text_asset(storage, &format!("{name}.pfl"))?);
    let file = profile.bodies.get(&body).with_context(|| format!("CatSystem2 {name} has no body {body}"))?.clone();
    let motion = profile.waits.get(&(body, wait)).cloned();
    let mut expression = Dif::default();
    let mut bodies = Dif::default();
    if let Some((_, dif)) = &motion {
        let dif = dif_text(storage, dif)?;
        profile.apply_dif(&dif);
        expression.parse(&dif);
    }
    if let Some(dif) = profile.body_difs.get(&body) {
        bodies.parse(&dif_text(storage, dif)?);
    }
    check()?;
    let path = VfsPath::new(Mount::Asset, &format!("psz.int/{file}.psz"))?;
    let path = crate::native_resource_paths::resolve(storage, &path)?.with_context(|| format!("CatSystem2 model {file} missing"))?;
    let packed = crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 256 * 1024 * 1024)?;
    ensure!(packed.len() > 16 && &packed[..4] == b"PSZ\0", "CatSystem2 model {file} is not PSZ");
    let mut plain = Vec::new();
    std::io::Read::read_to_end(&mut flate2::read::ZlibDecoder::new(&packed[16..]), &mut plain)?;
    check()?;
    let key = crysirror_puppet_eluna::emote_key_from_header(&plain);
    let mut puppet = ElunaPuppet::from_psb(&[&plain], key).with_context(|| format!("CatSystem2 model {file}"))?;
    if let Some((motion, _)) = motion.filter(|(motion, _)| puppet.motions().iter().any(|m| m.id == *motion)) {
        puppet.play(&motion, PlayOptions { looping: Some(true), weight: 1. })?;
    }
    // A title without `action.def` simply has no macros.
    let actions = text_asset(storage, "action.def").map_or_else(|_| BTreeMap::new(), |text| parse_actions(&text));
    let textures = (0..puppet.texture_names().len() as u32).filter_map(|index| puppet.texture_rgba(index)).collect();
    let mut waiting = BTreeMap::new();
    for file in expression.waiting_files.iter().chain(&bodies.waiting_files) {
        waiting.extend(parse_waiting(&text_asset(storage, file)?)?);
    }
    let initial = puppet.checkpoint();
    Ok(Model { puppet, initial, waiting, scale: profile.scale.unwrap_or([1.; 2]), position: profile.position.unwrap_or([0.; 2]), scale_offset: profile.scale_offset.unwrap_or([0.; 2]), textures, expression, bodies, actions })
}
struct Slot {
    model: Model,
    textures: Vec<String>,
    presentation: Option<u32>,
    last_ms: u64,
    expression_keys: BTreeMap<usize, String>,
    body_keys: BTreeMap<usize, String>,
    scales: BTreeMap<String, f32>,
    tone: [u32; 3],
    blink: Option<(usize, f32)>,
    next_blink_ms: u64,
    blink_enabled: bool,
    random: u32,
    action: Option<ActionRun>,
    lip: Option<LipRun>,
    inertia: Option<InertiaRun>,
    inertia_checked: bool,
    reset_on_motion: bool,
    waiting: Option<WaitingRun>,
    waiting_samplers: BTreeMap<String, crysirror_puppet_eluna::modulation::CorrelatedRanges>,
    waiting_label: String,
    pattern_shifts: PatternShiftRun,
    // A VM may yield and resume repeatedly at the same engine time. Geometry
    // and previous-frame nested motions advance only once unless a command,
    // pose or configuration actually changed in between those resumptions.
    last_update: Option<(u64, crate::task_policy::PlanePose, M2Config)>,
}
/// The model's inertia (+0xF8): off until something moves the model's base (574410 sets
/// +0x160); +0x10C is the on/off of native 907.
struct InertiaRun {
    solver: crysirror_puppet_eluna::inertia::InertiaSolver,
    enabled: bool,
    start: [f32; 2],
    moved: bool,
    tick_ms: f32,
    /// The offsets last added, taken back before adding the next.
    applied: BTreeMap<String, f32>,
    levels: BTreeMap<String, (Vec<String>, u32)>,
    enables: BTreeMap<String, (Vec<String>, u32)>,
}
/// 591620's queue position over a macro.
struct ActionRun {
    steps: Vec<ActionStep>,
    next: usize,
    wait_ms: f32,
    saved: BTreeMap<String, f32>,
}
/// 584FC0's walk over `#lip_flow`, restarted per voice.
struct LipRun {
    channel: u32,
    step: usize,
    frame: f32,
}
/// Commands that may arrive while the plane's model is still loading.
#[derive(Clone)]
pub(crate) enum Command {
    Expression(String),
    Body(String),
    ParameterScale(Option<String>, f32),
    ClearParameterScale,
    Tone([u32; 3]),
    Parameter(String, f32),
    Blink(bool),
    /// 591620: start a macro.
    Action(String),
    /// 591820: move on to the next timed step.
    ActionAdvance,
    /// 575860: drop the queue.
    ActionClear,
    /// 540: flap on voice channel, or stop.
    Lip(Option<u32>),
    /// 526 → 5744D0.
    Inertia(bool),
    /// 543/544: a named timeline of the model, or none.
    Timeline(Option<String>),
    /// 545: reset interpolation/player state on subsequent motion setup.
    ResetOnMotion(bool),
    Waiting(String),
}
#[derive(Default)]
pub(crate) struct Planes {
    slots: BTreeMap<u32, Slot>,
    loading: BTreeMap<u32, Task<Model>>,
    load_order: std::collections::VecDeque<u32>,
    bodies: BTreeMap<u32, String>,
    queued: BTreeMap<u32, Vec<Command>>,
    next_texture: u32,
    retired_textures: Vec<String>,
}
impl Planes {
    fn flush_retired(&mut self, host:&mut dyn HostServices, client:ClientId)->Result<()> {
        while let Some(path)=self.retired_textures.last() {
            // The shared host retains any frozen Canvas/mesh reader until
            // its own dependency fence ends. Ending VM ownership is separate.
            host.release_image_resource(client,path)?;
            self.retired_textures.pop();
        }
        Ok(())
    }
    fn retire_slot(&mut self, host:&mut dyn HostServices, client:ClientId, plane:u32)->Result<()> {
        if let Some(layer)=self.slots.get(&plane).and_then(|slot|slot.presentation) {
            let present=host.query_engine(client,crysirror_core::service::EngineQuery::Layer(layer))?;
            if !serde_json::from_slice::<serde_json::Value>(&present)?.is_null() {
                // A loading/deleted model must not leave a live mesh referencing
                // its old textures. Frozen captures remain independently owned.
                let empty=crysirror_puppet::PuppetFrame {size:[1.;2],..Default::default()};
                host.present_puppet_frame(client,layer,&serde_json::to_vec(&empty)?)?;
            }
        }
        if let Some(slot)=self.slots.remove(&plane) {
            self.retired_textures.extend(slot.textures);
        }
        self.flush_retired(host,client)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!(self.slots.iter().map(|(plane, slot)| {
            serde_json::json!({
                "plane": plane,
                "body": self.bodies.get(plane),
                "model_origin": slot.model.puppet.origin(),
                "model_size": slot.model.puppet.size(),
                "profile_scale": slot.model.scale,
                "profile_position": slot.model.position,
                "profile_scale_offset": slot.model.scale_offset,
                "pose": slot.last_update.as_ref().map(|(_, pose, _)| serde_json::json!({
                    "position": pose.position, "base": pose.base,
                    "scale": pose.scale, "rotation": pose.rotation,
                })),
            })
        }).collect::<Vec<_>>())
    }
    /// Loading definitions and active waiting tracks consume the session MT streams.
    pub fn needs_random(&self) -> bool {
        !self.loading.is_empty() || self.slots.values().any(|slot| slot.waiting.is_some())
    }
    /// 637F30 message 524: select the plane's model. The first three words name the body;
    /// the rest are expression digits on the same body, so they never reload it.
    /// Answers false when no worker is free yet and the caller must retry.
    pub fn begin(&mut self, host: &mut dyn HostServices, client: ClientId, pool: &TaskPool, plane: u32, spec: String) -> Result<bool> {
        let body = spec.split(',').take(3).collect::<Vec<_>>().join(",");
        let expression = spec.splitn(4, ',').nth(3).unwrap_or("").to_owned();
        if self.bodies.get(&plane) == Some(&body) {
            if !expression.is_empty() {
                self.command(plane, Command::Expression(expression));
            }
            return Ok(true);
        }
        let permit = match pool.reserve() {
            Ok(permit) => permit,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
            Err(e) => return Err(e),
        };
        let storage = host.storage(client)?;
        let task = permit.spawn(move |cancel| load(storage.as_ref(), &spec, &|| cancel.check()))?;
        self.retire_slot(host,client,plane)?;
        self.loading.insert(plane, task);
        self.load_order.retain(|id| *id != plane);
        self.load_order.push_back(plane);
        self.bodies.insert(plane, body);
        self.queued.remove(&plane);
        if !expression.is_empty() {
            self.command(plane, Command::Expression(expression));
        }
        Ok(true)
    }
    /// Queue one model command; it runs on the plane's next tick with a loaded model.
    pub fn command(&mut self, plane: u32, command: Command) {
        // 57E3C0: a plane with no model answers nothing and keeps nothing.
        if !self.slots.contains_key(&plane) && !self.loading.contains_key(&plane) {
            return;
        }
        self.queued.entry(plane).or_default().push(command);
    }
    /// The current key of each body digit, as message 534 reads it.
    pub fn body_digits(&self, plane: u32) -> String {
        self.slots.get(&plane).map_or(String::new(), |slot| slot.body_keys.values().cloned().collect::<Vec<_>>().join(","))
    }
    fn reset_motion(slot: &mut Slot) {
        if !slot.reset_on_motion { return; }
        slot.model.puppet.restore_checkpoint(&slot.model.initial);
        slot.expression_keys.clear();
        slot.body_keys.clear();
        slot.action = None;
        slot.blink = None;
        slot.inertia = None;
        slot.inertia_checked = false;
        slot.pattern_shifts = PatternShiftRun::default();
        slot.waiting = None;
    }
    fn run_command(slot: &mut Slot, command: Command, random:Option<&mut crate::motion::RandomStreams>, client:ClientId) -> Result<()> {
        match command {
            Command::Expression(words) => {
                Self::reset_motion(slot);
                let Slot { model, expression_keys, scales, pattern_shifts, .. } = slot;
                apply_keys(&mut model.puppet, &model.expression, expression_keys, scales, pattern_shifts, false, &digit_keys(&words))?;
            }
            Command::Body(words) => {
                Self::reset_motion(slot);
                let Slot { model, body_keys, scales, pattern_shifts, .. } = slot;
                apply_keys(&mut model.puppet, &model.bodies, body_keys, scales, pattern_shifts, true, &digit_keys(&words))?;
            }
            Command::ParameterScale(Some(name), scale) => { slot.scales.insert(name, scale); }
            Command::ParameterScale(None, scale) => {
                let names = slot.model.puppet.parameters().into_iter().map(|p| p.id).collect::<Vec<_>>();
                for name in names { slot.scales.insert(name, scale); }
            }
            Command::ClearParameterScale => slot.scales.clear(),
            Command::Tone(tone) => slot.tone = tone,
            Command::Parameter(name, value) => {
                if slot.model.puppet.parameter_value(&name).is_some() {
                    slot.model.puppet.set_parameter(&name, value, 0.)?;
                }
            }
            Command::Blink(on) => slot.blink_enabled = on,
            Command::Action(name) => {
                slot.action = slot.model.actions.get(&name).map(|steps| ActionRun { steps: steps.clone(), next: 0, wait_ms: 0., saved: BTreeMap::new() });
            }
            Command::ActionAdvance => {
                if let Some(run) = &mut slot.action {
                    run.wait_ms = 0.;
                }
            }
            Command::ActionClear => slot.action = None,
            Command::Inertia(enabled) => {
                if let Some(run) = &mut slot.inertia {
                    run.enabled = enabled;
                }
            }
            Command::Lip(Some(channel)) => slot.lip = Some(LipRun { channel, step: 0, frame: 0. }),
            Command::Lip(None) => {
                slot.lip = None;
                Self::close_mouth(slot)?;
            }
            Command::Timeline(Some(name)) => {
                if slot.model.puppet.motions().iter().any(|m| m.id == name) {
                    slot.model.puppet.play(&name, PlayOptions { looping: Some(false), weight: 1. })?;
                }
            }
            Command::Timeline(None) => {}
            Command::ResetOnMotion(enabled) => slot.reset_on_motion = enabled,
            Command::Waiting(label) => {
                // 637F30 message547's empty string calls 575280(false): no
                // waveform contribution is published until another selection.
                if label.is_empty() { slot.waiting = None; return Ok(()); }
                if let Some(variables) = slot.model.waiting.get(&label).cloned() {
                    let random=random.context("CatSystem2 waiting has no admitted session random stream")?;
                    let mut ordered = variables.iter().collect::<Vec<_>>();
                    ordered.sort_by_key(|(_, spec)| spec.order);
                    // Groups for every label were initialized when the model was
                    // admitted; selecting a label must not create new MT draws.
                    let sampler=slot.waiting_samplers.get_mut(&label).context("CatSystem2 waiting range groups missing")?;
                    sampler.reset_channel(0);
                    let oscillators=ordered.into_iter().map(|(name,spec)|Ok((name.clone(),sample_wave(spec,sampler,random,client)?))).collect::<Result<_>>()?;
                    slot.waiting_label = label;
                    let run = slot.waiting.get_or_insert_with(|| WaitingRun { bank: Default::default(), variables: BTreeMap::new(), applied: BTreeMap::new() });
                    run.bank.select(oscillators, 60.);
                    run.variables = variables;
                }
            },
        }
        Ok(())
    }
    /// 584D00: the lip variable returns to pattern table entry 0.
    fn close_mouth(slot: &mut Slot) -> Result<()> {
        let Slot { model, expression_keys, .. } = slot;
        for ((position, key), lip) in &model.expression.lips {
            if expression_keys.get(position) == Some(key) {
                if let (Some(value), true) = (lip.table.first(), model.puppet.parameter_value(&lip.variable).is_some()) {
                    model.puppet.set_parameter(&lip.variable, *value, 0.)?;
                }
            }
        }
        Ok(())
    }
    /// The builder around 59209A: one solver over both difference files' tables.
    fn inertia_of(model: &Model, base: [f32; 2]) -> Option<InertiaRun> {
        let mut solver = crysirror_puppet_eluna::inertia::InertiaSolver::default();
        for dif in [&model.expression, &model.bodies] {
            for (name, material) in &dif.materials[0] {
                solver.add_spring(name, *material);
            }
        }
        for dif in [&model.expression, &model.bodies] {
            for (name, _) in &dif.materials[0] {
                solver.set_range(name, dif.materials[1].get(name).copied(), dif.materials[2].get(name).copied());
            }
            dif.inertia_horizontal.iter().for_each(|name| solver.add_horizontal(name));
            dif.inertia_vertical.iter().for_each(|name| solver.add_vertical(name));
            for link in &dif.inertia_links {
                solver.add_link(link.clone(), model.puppet.parameter_value(&link.source).unwrap_or(0.));
            }
        }
        let labels = |table: fn(&Dif) -> &BTreeMap<String, Vec<String>>, initial: u32| {
            let mut out: BTreeMap<String, (Vec<String>, u32)> = BTreeMap::new();
            for dif in [&model.expression, &model.bodies] {
                for (label, names) in table(dif) {
                    out.entry(label.clone()).or_insert((Vec::new(), initial)).0.extend(names.iter().cloned());
                }
            }
            out
        };
        let levels = labels(|dif| &dif.level_labels, 100);
        let enables = labels(|dif| &dif.enable_labels, 1);
        (!solver.is_empty()).then(|| InertiaRun { solver, enabled: true, start: base, moved: false, tick_ms: 0., applied: BTreeMap::new(), levels, enables })
    }
    /// 573D60 → 5930C0, 594530, 594760, 5948F0. The steps run per 60 Hz tick; the spring
    /// offsets add onto whatever else set the variable.
    fn inertia(slot: &mut Slot, base: [f32; 2], dt_ms: f32, config: &M2Config) -> Result<()> {
        let Slot { model, inertia, .. } = slot;
        let Some(run) = inertia else { return Ok(()) };
        // 573D60: the options reach the model when they change; 593C60 and 5941C0 restart
        // the springs they touch. A level of 100 percent is the base material.
        let InertiaRun { solver, levels, enables, .. } = &mut *run;
        for (label, (names, applied)) in levels.iter_mut() {
            let level = config.get(b"iner_lv", label).unwrap_or(100);
            if level != *applied {
                *applied = level;
                solver.set_level(names.iter().map(String::as_str), level as f32 / 100.);
            }
        }
        for (label, (names, applied)) in enables.iter_mut() {
            let enable = config.get(b"iner_enable", label).unwrap_or(1);
            if enable != *applied {
                *applied = enable;
                solver.set_enabled(names.iter().map(String::as_str), enable != 0);
            }
        }
        run.moved |= base != run.start;
        if !(run.moved && run.enabled && config.eff_iner != 0) {
            // 593430(0) around 5930C0: position follows without pushing, and nothing steps.
            run.solver.reset_motion();
            return Ok(());
        }
        run.tick_ms += dt_ms;
        let ticks = (run.tick_ms * 60. / 1000.) as u32;
        run.tick_ms -= ticks as f32 * 1000. / 60.;
        let puppet = &model.puppet;
        run.solver.advance(base, ticks, |name| puppet.parameter_value(name));
        let outputs: Vec<(String, f32)> = run.solver.outputs().map(|(name, value)| (name.to_owned(), value)).collect();
        for (name, value) in outputs {
            let Some(current) = model.puppet.parameter_value(&name) else { continue };
            let earlier = run.applied.insert(name.clone(), value).unwrap_or(0.);
            if value != earlier {
                model.puppet.set_parameter(&name, current - earlier + value, 0.)?;
            }
        }
        Ok(())
    }
    /// 584FC0: `#lip_flow` for the current mouth key. Each step eases from its pattern to
    /// the next over its frames; a marked step may end the flap once the voice stopped.
    /// With a `.lv` table (message 221 → 575490) 5A1F30 instead hands the variable that table's
    /// value directly; past the table it answers 0. Its index is an argument this reading did
    /// not trace to its source: the tables hold 60 entries per second of voice, so the playback
    /// position indexes them here. The `.lpb` form (message 220) appears in neither test title.
    fn lip(host: &mut dyn HostServices, client: ClientId, slot: &mut Slot, dt_ms: f32, sound: &crate::native_kcs_sound::KcsSound) -> Result<()> {
        let Some(mut run) = slot.lip.take() else { return Ok(()) };
        let playing = host.audio_playing(client, run.channel)?;
        let found = {
            let Slot { model, expression_keys, .. } = &*slot;
            model.expression.lips.iter().find(|((position, key), _)| expression_keys.get(position) == Some(key)).map(|(_, lip)| lip.clone())
        };
        let Some(lip) = found.filter(|lip| !lip.flow.is_empty() && !lip.table.is_empty()) else {
            slot.lip = playing.then_some(run);
            return Ok(());
        };
        if let Some(levels) = sound.levels(run.channel) {
            let value = host.audio_info(client, run.channel)?.filter(|info| info.source_rate != 0).and_then(|info| {
                levels.get((info.position_frames.saturating_mul(60) / u64::from(info.source_rate)) as usize).copied()
            });
            if slot.model.puppet.parameter_value(&lip.variable).is_some() {
                slot.model.puppet.set_parameter(&lip.variable, value.unwrap_or(0.), 0.)?;
            }
            slot.lip = Some(run);
            return Ok(());
        }
        run.frame += dt_ms * 60. / 1000.;
        loop {
            let step = lip.flow[run.step % lip.flow.len()];
            if run.frame < step.1.max(1.) {
                break;
            }
            if step.2 && !playing {
                return Self::close_mouth(slot);
            }
            run.frame -= step.1.max(1.);
            run.step += 1;
        }
        let at = |index: usize| {
            let pattern = lip.flow[index % lip.flow.len()].0;
            lip.table.get(pattern).copied().unwrap_or(lip.table[0])
        };
        let span = lip.flow[run.step % lip.flow.len()].1.max(1.);
        let value = at(run.step) + (at(run.step + 1) - at(run.step)) * (run.frame / span);
        if slot.model.puppet.parameter_value(&lip.variable).is_some() {
            slot.model.puppet.set_parameter(&lip.variable, value, 0.)?;
        }
        slot.lip = Some(run);
        Ok(())
    }
    /// 591820 over time: `wait` holds; `frame` starts its transitions and holds as long.
    fn action(slot: &mut Slot, dt_ms: f32) -> Result<()> {
        let Slot { model, action, .. } = slot;
        let Some(run) = action else { return Ok(()) };
        run.wait_ms -= dt_ms;
        while run.wait_ms <= 0. {
            let Some(step) = run.steps.get(run.next).cloned() else {
                *action = None;
                return Ok(());
            };
            run.next += 1;
            match step {
                ActionStep::Wait(frames) => run.wait_ms += frames * 1000. / 60.,
                ActionStep::Frame { frames, vars, restore } => {
                    let duration = frames * 1000. / 60.;
                    for (name, value) in &vars {
                        if let Some(current) = model.puppet.parameter_value(name) {
                            run.saved.entry(name.clone()).or_insert(current);
                            model.puppet.set_parameter(name, *value, duration)?;
                        }
                    }
                    if restore {
                        for (name, value) in &run.saved {
                            model.puppet.set_parameter(name, *value, duration)?;
                        }
                    }
                    run.wait_ms += duration;
                }
            }
        }
        Ok(())
    }
    /// 587790: `#attribute` of the current model.
    pub fn attribute(&self, plane: u32, number: u32) -> Option<[i32; 3]> {
        self.slots.get(&plane)?.model.expression.attributes.get(&number).copied()
    }
    /// 587A70: the `#touch` table for a colour.
    fn touch_table(&self, plane: u32, colour: u32, table: u32) -> Option<&Vec<(String, String)>> {
        let dif = &self.slots.get(&plane)?.model.expression;
        dif.touch_tables.get(dif.touch.get(&colour)?.get(table as usize)?)
    }
    /// 580020: rows of that table.
    pub fn touch_rows(&self, plane: u32, colour: u32, table: u32) -> u32 {
        self.touch_table(plane, colour, table).map_or(0, |rows| rows.len() as u32)
    }
    /// 57FFC0: one {action, voice} row.
    pub fn touch_row(&self, plane: u32, colour: u32, table: u32, row: u32) -> Option<(String, String)> {
        self.touch_table(plane, colour, table)?.get(row as usize).cloned()
    }
    /// 5758F0: a macro is queued or running.
    pub fn action_active(&self, plane: u32) -> bool {
        self.slots.get(&plane).is_some_and(|slot| slot.action.is_some())
            || self.queued.get(&plane).is_some_and(|queue| queue.iter().any(|c| matches!(c, Command::Action(_))))
    }
    /// `#blink`: the eye digit's variable closes and reopens on a random schedule.
    fn blink(slot: &mut Slot, now_ms: u64) -> Result<()> {
        if !slot.blink_enabled {
            return Ok(());
        }
        if let Some((_position, end)) = slot.blink {
            if now_ms as f32 >= end {
                slot.blink = None;
                let Slot { model, expression_keys, .. } = slot;
                for blink in model.expression.blinks.clone() {
                    if expression_keys.get(&blink.position) == Some(&blink.key) {
                        if let Some(values) = model.expression.patterns.get(&blink.position).and_then(|keys| keys.get(&blink.key)) {
                            for (variable, value) in values.clone() {
                                model.puppet.set_parameter(&variable, value + blink.from, blink.speed * 1000. / 60.)?;
                            }
                        }
                    }
                }
            }
            return Ok(());
        }
        if now_ms < slot.next_blink_ms {
            return Ok(());
        }
        let Slot { model, expression_keys, random, next_blink_ms, blink, .. } = slot;
        let found = model.expression.blinks.iter().find(|b| expression_keys.get(&b.position) == Some(&b.key)).cloned();
        let Some(found) = found else {
            *next_blink_ms = now_ms + 1000;
            return Ok(());
        };
        *random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let span = found.wait[1].saturating_sub(found.wait[0]).max(1);
        *next_blink_ms = now_ms + u64::from(found.wait[0] + (*random >> 8) % span) * 1000 / 60;
        if let Some(values) = model.expression.patterns.get(&found.position).and_then(|keys| keys.get(&found.key)) {
            for (variable, value) in values.clone() {
                if model.puppet.parameter_value(&variable).is_some() {
                    model.puppet.set_parameter(&variable, value + found.to, found.speed * 1000. / 60.)?;
                }
            }
        }
        *blink = Some((found.position, now_ms as f32 + found.speed * 1000. / 60.));
        Ok(())
    }
    /// 57BFD0 for the plane's entry: busy while its model still loads or the
    /// E-mote player animates (vtable +240) or its macro queue (this[96]) runs.
    pub fn busy(&self, plane: u32) -> bool {
        self.loading.contains_key(&plane) || self.action_active(plane) || self.slots.get(&plane).is_some_and(|slot| slot.model.puppet.is_animating())
    }
    pub fn remove(&mut self, host:&mut dyn HostServices, client:ClientId, plane: u32)->Result<()> {
        self.retire_slot(host,client,plane)?;
        self.loading.remove(&plane);
        self.load_order.retain(|id| *id != plane);
        self.bodies.remove(&plane);
        self.queued.remove(&plane);
        Ok(())
    }
    /// One owner frame: publish finished loads and present every visible model.
    pub fn tick(&mut self, host: &mut dyn HostServices, client: ClientId, tasks: &crate::task_policy::NativeTaskPolicy, sound: &crate::native_kcs_sound::KcsSound, config: &M2Config, canvas:[u32;2], now_ms: u64, mut random:Option<&mut crate::motion::RandomStreams>) -> Result<()> {
        self.flush_retired(host,client)?;
        // Do not consume completed worker results until the owning session has
        // admitted its seed. Publication follows VM load admission order, rather
        // than whichever worker finishes first, so MT draw order stays stable.
        if !self.loading.is_empty() && random.is_none() { return Ok(()); }
        while let Some(plane) = self.load_order.front().copied() {
            let task = self.loading.get_mut(&plane).unwrap();
            let outcome = match task.try_take() {
                Ok(None) => break,
                Ok(Some(model)) => Ok(model),
                Err(error) => Err(error),
            };
            self.loading.remove(&plane);
            self.load_order.pop_front();
            match outcome {
                Ok(mut model) => {
                    let mut waiting_samplers = BTreeMap::new();
                    if !model.waiting.is_empty() {
                        let stream = random.as_deref_mut().context("CatSystem2 model waiting has no admitted session random stream")?;
                        for (label, variables) in &model.waiting {
                            let mut ordered = variables.values().collect::<Vec<_>>();
                            ordered.sort_by_key(|spec| spec.order);
                            let entries = ordered.into_iter().flat_map(|spec| [(0, spec.negative.clone()), (1, spec.positive.clone()), (2, spec.period.clone())]);
                            waiting_samplers.insert(label.clone(), crysirror_puppet_eluna::modulation::CorrelatedRanges::new(entries, |draw| wave_random(stream, client, draw))?);
                        }
                    }
                    let mut textures = Vec::new();
                    for (width, height, rgba) in core::mem::take(&mut model.textures) {
                        let slot = 0x5100_0000u32.checked_add(self.next_texture).context("CatSystem2 model texture slots")?;
                        self.next_texture += 1;
                        match host.upload_rgba(client, slot, width, height, &rgba) {
                            Ok(path)=>textures.push(path.relative().to_owned()),
                            Err(error)=>{
                                self.retired_textures.extend(textures);
                                self.flush_retired(host,client)?;
                                return Err(error.into());
                            }
                        }
                    }
                    self.slots.insert(plane, Slot { model, textures, presentation: None, last_ms: now_ms, expression_keys: BTreeMap::new(), body_keys: BTreeMap::new(), scales: BTreeMap::new(), tone: [100; 3], blink: None, next_blink_ms: now_ms + 2000, blink_enabled: true, random: plane.wrapping_mul(2_654_435_761) | 1, action: None, lip: None, inertia: None, inertia_checked: false, reset_on_motion: false, waiting: None, waiting_samplers, waiting_label: String::new(), pattern_shifts: PatternShiftRun::default(), last_update: None });
                }
                Err(error) => {
                    host.append_log(client, &format!("CatSystem2 model load failed: {error:#}"))?;
                }
            }
        }
        for (plane, slot) in &mut self.slots {
            let Ok(native) = tasks.plane(*plane) else { continue };
            let commands = self.queued.remove(plane).unwrap_or_default();
            let stamp = (now_ms, native.pose, config.clone());
            if commands.is_empty() && slot.last_update.as_ref() == Some(&stamp) {
                continue;
            }
            let dt_ms = now_ms.saturating_sub(slot.last_ms) as f32;
            slot.model.puppet.begin_parameter_batch()?;
            let parameter_updates = (|| -> Result<()> {
                for command in commands {
                    Self::run_command(slot, command, random.as_deref_mut(), client)?;
                }
                // 5744D0: the options gate blinking and lip movement as well.
                if config.eff_blink != 0 {
                    Self::blink(slot, now_ms)?;
                }
                Self::action(slot, dt_ms)?;
                if config.eff_lip != 0 {
                    Self::lip(host, client, slot, dt_ms, sound)?;
                } else if slot.lip.take().is_some() {
                    Self::close_mouth(slot)?;
                }
                if slot.inertia.is_none() && !slot.inertia_checked {
                    slot.inertia_checked = true;
                    slot.inertia = Self::inertia_of(&slot.model, slot.model.anchor(&native.pose));
                }
                let anchor = slot.model.anchor(&native.pose);
                Self::inertia(slot, anchor, dt_ms, config)?;
                Ok(())
            })();
            // Always close the batch, including when a semantic command fails;
            // subsequent sampling can never observe a half-open batch.
            let committed = slot.model.puppet.end_parameter_batch();
            parameter_updates?;
            committed?;
            let mut offsets = BTreeMap::<String, f32>::new();
            {
                let run = &mut slot.pattern_shifts;
                run.applied.clear();
                for (id, transition) in &mut run.transitions {
                    let value = transition.advance(dt_ms * 60. / 1000.);
                    run.values.insert(id.clone(), value);
                    *offsets.entry(id.1.clone()).or_default() += value;
                }
            }
            if let Some(run) = &mut slot.waiting {
                let sampler = slot.waiting_samplers.get_mut(&slot.waiting_label).context("CatSystem2 waiting sampler missing")?;
                let random=random.as_deref_mut().context("CatSystem2 waiting has no session random stream")?;
                let mut failure=None;
                let values=run.bank.advance_with(f64::from(dt_ms)*60./1000.,|name|{
                    let spec=run.variables.get(name)?;
                    match sample_wave(spec,sampler,random,client){Ok(wave)=>Some(wave),Err(error)=>{failure=Some(error);None}}
                });
                if let Some(error)=failure{return Err(error);}

                run.applied = values;
                for (name, offset) in &run.applied { *offsets.entry(name.clone()).or_default() += offset; }
            }
            slot.model.puppet.set_parameter_offsets(offsets)?;
            slot.model.puppet.advance(now_ms.saturating_sub(slot.last_ms) as f32)?;
            slot.last_ms = now_ms;
            let mut frame = slot.model.puppet.frame()?;
            // `_fade_z` layers are depth-fade helpers painted in flat colours; the E-mote
            // driver never shows them as ordinary sprites.
            frame.parts.retain(|part| !part.id.contains("_fade_z"));
            frame.textures = slot.textures.clone();
            if slot.tone != [100; 3] {
                // 6390B0: the plane's colour percentages.
                for part in &mut frame.parts {
                    for color in &mut part.colors {
                        for channel in 0..3 {
                            color[channel] *= slot.tone[channel] as f32 / 100.;
                        }
                    }
                }
            }
            let pose = &native.pose;
            // 574450 retains both supplied scale words, but 574070 passes
            // only DIF.scaleX * script.scaleX to the E-mote SDK's uniform
            // SetScale. A spare Y word must not squash or hide the actor.
            let scale = [slot.model.scale[0] * pose.scale[0]; 2];
            let origin = slot.model.puppet.origin();
            // Frame vertices have had origin subtracted. Restore that offset
            // once, and rotate/reflect the resulting quad about the script's
            // world base, rather than its changing bounding-box corner.
            let extent_scale = [scale[0].abs(), scale[1].abs()];
            let offset = [origin[0] * extent_scale[0], origin[1] * extent_scale[1]];
            let anchor = slot.model.anchor(pose);
            // 59C080 projects E-mote about the native render target's centre;
            // its camera sits at -height/2 with width/height aspect scaling.
            // Shared Layers use a top-left origin. DIF.position remains SDK
            // model policy and must not double as this viewport conversion.
            let x = canvas[0] as f32 * 0.5 + anchor[0] + offset[0];
            let y = canvas[1] as f32 * 0.5 + anchor[1] + offset[1];
            let payload = native.layer + 1;
            let rect = [x, y, (frame.size[0] * extent_scale[0]).max(0.001), (frame.size[1] * extent_scale[1]).max(0.001)];
            let exists = !serde_json::from_slice::<serde_json::Value>(&host.query_engine(client, crysirror_core::service::EngineQuery::Layer(payload))?)?.is_null();
            let mut commands = Vec::new();
            if exists {
                commands.push(serde_json::json!({"op":"set_rect","id":payload,"rect":rect}));
            } else {
                commands.push(serde_json::json!({"op":"put","layer":{"id":payload,"parent":native.layer,"rect":rect,"paint":{"kind":"solid","color":[0,0,0,0]}}}));
            }
            commands.push(serde_json::json!({"op":"set_transform","id":payload,"transform":{"pivot_x":-offset[0],"pivot_y":-offset[1],"scale_x":if scale[0].is_sign_negative(){-1.}else{1.},"scale_y":if scale[1].is_sign_negative(){-1.}else{1.},"rotate":pose.rotation}}));
            commands.push(serde_json::json!({"op":"set_visible","id":payload,"visible":scale[0]!=0.&&scale[1]!=0.}));
            commands.push(serde_json::json!({"op":"set_depth","id":payload,"depth":pose.position[2]}));
            commands.push(serde_json::json!({"op":"set_opacity","id":native.layer,"opacity":pose.alpha}));
            host.submit_engine(client, 1, &serde_json::to_vec(&serde_json::json!({"layers":commands}))?)?;
            host.present_puppet_frame(client, payload, &serde_json::to_vec(&frame)?)?;
            slot.presentation = Some(payload);
            slot.last_update = Some(stamp);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_read_body_wait_scale_and_position() {
        let mut profile = Profile::parse_pfl("// x\n#charaname\n1\tbody\tca01_l_body_1\n#motname\n1\t0\twaiting_loop\t\tca_m.dif\tbody.dif\n");
        assert_eq!(profile.bodies[&1], "ca01_l_body_1");
        assert_eq!(profile.waits[&(1, 0)].1, "ca_m.dif");
        profile.apply_dif("#scale\n0.6 0.6\t// X Y\n#position\n-640 -204\n");
        assert_eq!(profile.scale, Some([0.6, 0.6]));
        assert_eq!(profile.position, Some([-640., -204.]));
    }
}
