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
                _ => {}
            }
        }
    }
}
/// A `.dif` difference table: `#dif` digit positions, each `#pattern` key's variable
/// values, the `#chain` transitions between keys and the `#blink` schedule.
#[derive(Clone, Debug, Default)]
pub(crate) struct Dif {
    patterns: BTreeMap<usize, BTreeMap<String, Vec<(String, f32)>>>,
    chains: BTreeMap<usize, Chain>,
    blinks: Vec<Blink>,
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
                "chain" => self.parse_chains(&rows),
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
fn apply_keys(puppet: &mut ElunaPuppet, dif: &Dif, current: &mut BTreeMap<usize, String>, scales: &BTreeMap<String, f32>, keys: &[String]) -> Result<()> {
    for (index, key) in keys.iter().enumerate() {
        let position = index + 1;
        if key.is_empty() || current.get(&position) == Some(key) {
            continue;
        }
        let from = current.insert(position, key.clone()).unwrap_or_default();
        let frames = dif.frames(position, &from, key);
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
pub(crate) struct Model {
    puppet: ElunaPuppet,
    scale: [f32; 2],
    position: [f32; 2],
    textures: Vec<(u32, u32, Vec<u8>)>,
    expression: Dif,
    bodies: Dif,
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
    let textures = (0..puppet.texture_names().len() as u32).filter_map(|index| puppet.texture_rgba(index)).collect();
    Ok(Model { puppet, scale: profile.scale.unwrap_or([1.; 2]), position: profile.position.unwrap_or([0.; 2]), textures, expression, bodies })
}
struct Slot {
    model: Model,
    textures: Vec<String>,
    last_ms: u64,
    expression_keys: BTreeMap<usize, String>,
    body_keys: BTreeMap<usize, String>,
    scales: BTreeMap<String, f32>,
    tone: [u32; 3],
    blink: Option<(usize, f32)>,
    next_blink_ms: u64,
    blink_enabled: bool,
    random: u32,
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
}
#[derive(Default)]
pub(crate) struct Planes {
    slots: BTreeMap<u32, Slot>,
    loading: BTreeMap<u32, Task<Model>>,
    bodies: BTreeMap<u32, String>,
    queued: BTreeMap<u32, Vec<Command>>,
    next_texture: u32,
}
impl Planes {
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
        self.loading.insert(plane, task);
        self.slots.remove(&plane);
        self.bodies.insert(plane, body);
        self.queued.remove(&plane);
        if !expression.is_empty() {
            self.command(plane, Command::Expression(expression));
        }
        Ok(true)
    }
    /// Queue one model command; it runs on the plane's next tick with a loaded model.
    pub fn command(&mut self, plane: u32, command: Command) {
        self.queued.entry(plane).or_default().push(command);
    }
    /// The current key of each body digit, as message 534 reads it.
    pub fn body_digits(&self, plane: u32) -> String {
        self.slots.get(&plane).map_or(String::new(), |slot| slot.body_keys.values().cloned().collect::<Vec<_>>().join(","))
    }
    fn run_command(slot: &mut Slot, command: Command) -> Result<()> {
        match command {
            Command::Expression(words) => {
                let Slot { model, expression_keys, scales, .. } = slot;
                apply_keys(&mut model.puppet, &model.expression, expression_keys, scales, &digit_keys(&words))?;
            }
            Command::Body(words) => {
                let Slot { model, body_keys, scales, .. } = slot;
                apply_keys(&mut model.puppet, &model.bodies, body_keys, scales, &digit_keys(&words))?;
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
        }
        Ok(())
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
    pub fn remove(&mut self, plane: u32) {
        self.loading.remove(&plane);
        self.slots.remove(&plane);
        self.bodies.remove(&plane);
        self.queued.remove(&plane);
    }
    /// One owner frame: publish finished loads and present every visible model.
    pub fn tick(&mut self, host: &mut dyn HostServices, client: ClientId, tasks: &crate::task_policy::NativeTaskPolicy, now_ms: u64) -> Result<()> {
        let done: Vec<u32> = self.loading.keys().copied().collect();
        for plane in done {
            let task = self.loading.get_mut(&plane).unwrap();
            let outcome = match task.try_take() {
                Ok(None) => continue,
                Ok(Some(model)) => Ok(model),
                Err(error) => Err(error),
            };
            self.loading.remove(&plane);
            match outcome {
                Ok(mut model) => {
                    let mut textures = Vec::new();
                    for (width, height, rgba) in core::mem::take(&mut model.textures) {
                        let slot = 0x5100_0000u32.checked_add(self.next_texture).context("CatSystem2 model texture slots")?;
                        self.next_texture += 1;
                        textures.push(host.upload_rgba(client, slot, width, height, &rgba)?.relative().to_owned());
                    }
                    self.slots.insert(plane, Slot { model, textures, last_ms: now_ms, expression_keys: BTreeMap::new(), body_keys: BTreeMap::new(), scales: BTreeMap::new(), tone: [100; 3], blink: None, next_blink_ms: now_ms + 2000, blink_enabled: true, random: plane.wrapping_mul(2_654_435_761) | 1 });
                }
                Err(error) => {
                    host.append_log(client, &format!("CatSystem2 model load failed: {error:#}"))?;
                }
            }
        }
        for (plane, slot) in &mut self.slots {
            let Ok(native) = tasks.plane(*plane) else { continue };
            for command in self.queued.remove(plane).unwrap_or_default() {
                Self::run_command(slot, command)?;
            }
            Self::blink(slot, now_ms)?;
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
            let scale = [slot.model.scale[0] * pose.scale[0], slot.model.scale[1] * pose.scale[1]];
            let origin = slot.model.puppet.origin();
            // 84 already places the model's centre in screen space (640,360 by default).
            let x = pose.base[0] + origin[0] * scale[0];
            let y = pose.base[1] + origin[1] * scale[1];
            let payload = native.layer + 1;
            let rect = [x, y, frame.size[0] * scale[0].abs().max(0.001), frame.size[1] * scale[1].abs().max(0.001)];
            let exists = !serde_json::from_slice::<serde_json::Value>(&host.query_engine(client, crysirror_core::service::EngineQuery::Layer(payload))?)?.is_null();
            let mut commands = Vec::new();
            if exists {
                commands.push(serde_json::json!({"op":"set_rect","id":payload,"rect":rect}));
            } else {
                commands.push(serde_json::json!({"op":"put","layer":{"id":payload,"parent":native.layer,"rect":rect,"paint":{"kind":"solid","color":[0,0,0,0]}}}));
            }
            commands.push(serde_json::json!({"op":"set_depth","id":payload,"depth":pose.position[2]}));
            commands.push(serde_json::json!({"op":"set_opacity","id":native.layer,"opacity":pose.alpha}));
            host.submit_engine(client, 1, &serde_json::to_vec(&serde_json::json!({"layers":commands}))?)?;
            host.present_puppet_frame(client, payload, &serde_json::to_vec(&frame)?)?;
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
