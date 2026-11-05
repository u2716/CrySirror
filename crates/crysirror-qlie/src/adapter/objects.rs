use super::*;
use crysirror_core::service::{
    AudioAsset, AudioParameters, AudioPreparePhase, AudioRequest, EngineQuery, MediaPhase,
    MediaRequest, PcmBus,
};
fn reindex(o: &mut Object) {
    o.table = o
        .lines
        .iter()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.to_owned()))
        .collect();
}
fn set_name_value(o: &mut Object, name: &str, value: &str) {
    // Native fast-table Add ignores an empty name (572EA4); placeholders
    // from a short split must not become executable callback labels.
    if name.is_empty() {
        return;
    }
    if let Some(line) = o.lines.iter_mut().find(|l| {
        l.split_once('=')
            .is_some_and(|p| p.0.eq_ignore_ascii_case(name))
    }) {
        *line = format!("{name}={value}");
    } else {
        o.lines.push(format!("{name}={value}"));
    }
    o.table.insert(name.to_ascii_lowercase(), value.into());
}
pub(super) fn native_key(code: i32) -> Result<u64> {
    use crysirror_core::service::buttons as B;
    match code & 0xffff {
        0xf0 => Ok(B::WHEEL_UP),
        0xf1 => Ok(B::WHEEL_DOWN),
        value => B::windows_virtual_key(value as u32)
            .with_context(|| format!("unsupported QLIE input key {code:#x}")),
    }
}
impl QlieVm {
    pub(super) fn collect_objects(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
    ) -> Result<()> {
        let mut live = BTreeSet::new();
        for value in m
            .globals
            .iter()
            .filter(|(key, _)| !key.starts_with("resultobj["))
            .map(|(_, v)| v)
            .chain(m.frame.locals.values())
            .chain(m.stack.iter().flat_map(|f| f.locals.values()))
        {
            if let Value::Text(id) = value {
                if m.object_types.contains_key(id) {
                    live.insert(id.clone());
                }
            }
        }
        let mut pending = live.iter().cloned().collect::<Vec<_>>();
        while let Some(id) = pending.pop() {
            if let Some(o) = self.objects.get(&id) {
                for text in o
                    .properties
                    .values()
                    .flatten()
                    .map(Value::text)
                    .chain(o.table.values().cloned())
                {
                    if m.object_types.contains_key(&text) && live.insert(text.clone()) {
                        pending.push(text);
                    }
                }
            }
        }
        let dead = m
            .object_types
            .keys()
            .filter(|id| !live.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        for id in dead {
            if let Some(o) = self.objects.remove(&id) {
                self.free_object(h, o)?;
            }
            m.object_types.remove(&id);
        }
        Ok(())
    }
    pub(super) fn delete(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        args: &[String],
    ) -> Result<()> {
        for name in args {
            let key = m.key(name)?;
            let count = if m.frame.locals.contains_key(&key) {
                None
            } else {
                m.frame
                    .arrays
                    .get(&key)
                    .or_else(|| m.arrays.get(&key))
                    .copied()
            };
            let local = m.frame.locals.contains_key(&key) || m.frame.arrays.contains_key(&key);
            for i in 0..count.unwrap_or(1) {
                let name = if count.is_some() {
                    format!("{key}[{i}]")
                } else {
                    key.clone()
                };
                if local {
                    m.frame.locals.remove(&name);
                } else {
                    m.globals.remove(&name);
                }
            }
            if local {
                m.frame.arrays.remove(&key);
            } else {
                m.arrays.remove(&key);
            }
        }
        // Object variables are references. Deleting a temporary alias must not
        // remove a shared native sound channel or another live object's layer.
        self.collect_objects(h, m)
    }
    fn free_object(&mut self, h: &mut dyn HostServices, o: Object) -> Result<()> {
        if matches!(o.kind.as_str(), "cam" | "camex") && o.properties.contains_key("col") {
            self.submit(
                h,
                json!({"layers":[{"op":"remove","id":o.layer+0x20000000}]}),
            )?;
        }
        if o.ui_registered {
            self.submit(h, json!({"ui":[{"op":"remove","id":o.layer}]}))?;
        }
        if o.created {
            self.submit(h, json!({"layers":[{"op":"remove","id":o.layer}]}))?;
        }
        if o.kind == "snd" && o.channel != 0 {
            h.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel: o.channel,
                    fade_ms: 0.,
                },
            )?;
        }
        if let Some(id) = o.media {
            h.media_request(self.client, MediaRequest::Remove { id })?;
            self.media_ids.remove(&id);
        }
        for (_, media) in o.embedded_media {
            let id = media.id;
            h.media_request(self.client, MediaRequest::Remove { id })?;
            self.media_ids.remove(&id);
        }
        for (ticket, _) in o.audio_tickets {
            h.release_audio(self.client, ticket)?;
        }
        for channel in o.audio_channels {
            h.submit_audio(
                self.client,
                AudioRequest::Stop {
                    channel,
                    fade_ms: 0.,
                },
            )?;
        }
        let mut released = BTreeSet::new();
        for image in o
            .images
            .into_iter()
            .flatten()
            .chain(o.originals.into_values())
        {
            if !released.insert(image.path.relative().to_owned()) {
                continue;
            }
            h.release_image_resource(self.client, image.path.relative())?;
        }
        Ok(())
    }
    fn object(&mut self, m: &Machine) -> Result<String> {
        let id = m.scope.clone();
        let kind = m
            .object_types
            .get(&id)
            .context("QLIE method without object scope")?
            .clone();
        if !self.objects.contains_key(&id) {
            ensure!(self.objects.len() < 4096, "QLIE object budget");
            let layer = self.next_layer;
            self.next_layer += 1;
            self.objects.insert(
                id.clone(),
                Object {
                    kind,
                    layer,
                    gain: 1.,
                    ..Default::default()
                },
            );
        }
        Ok(id)
    }
    pub(super) fn object_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        ins: &Instruction,
    ) -> Result<bool> {
        let id = self.object(m)?;
        let kind = self.objects[&id].kind.clone();
        let a = ins
            .args
            .iter()
            .map(|s| m.eval(s))
            .collect::<Result<Vec<_>>>()?;
        let n = |i: usize| {
            a.get(i)
                .map(Value::integer)
                .transpose()
                .map(|n| n.unwrap_or(0))
        };
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        let op = ins.op.as_str();
        if matches!(kind.as_str(), "table" | "list") {
            let object = self.objects.get_mut(&id).unwrap();
            match op {
                "i" => {
                    object.lines.clear();
                    object.table.clear();
                }
                "l" => {
                    let path = self.resolve(m, &s(0))?;
                    let text = self
                        .text
                        .get(&crate::program::canonical(&path)?)
                        .context("QLIE text resource not prepared")?
                        .clone();
                    let o = self.objects.get_mut(&id).unwrap();
                    o.lines = text
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with(';'))
                        .map(str::to_owned)
                        .collect();
                    o.table = o
                        .lines
                        .iter()
                        .filter_map(|l| l.split_once('='))
                        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
                        .collect();
                }
                "?v" => {
                    let key = s(0).to_ascii_lowercase();
                    let value = object
                        .table
                        .get(&key)
                        .cloned()
                        .map(Value::Text)
                        .or_else(|| a.get(1).cloned())
                        .unwrap_or(Value::Text(String::new()));
                    m.result_text(0, value.text());
                    m.result(0, Value::Int(value.integer().unwrap_or(0)));
                }
                "v" => {
                    set_name_value(object, &s(0), &s(1));
                }
                "?c" => m.result(0, Value::Int(object.lines.len() as i32)),
                "?s" => {
                    let index = usize::try_from(n(0)?)?;
                    m.result_text(
                        0,
                        object.lines.get(index).context("QLIE list index")?.clone(),
                    );
                }
                "?n" => {
                    let index = usize::try_from(n(0)?)?;
                    let line = object.lines.get(index).context("QLIE table name index")?;
                    m.result_text(0, line.split_once('=').map(|p| p.0).unwrap_or(""));
                }
                "s" => {
                    let index = usize::try_from(n(0)?)?;
                    ensure!(index < object.lines.len(), "QLIE line replacement index");
                    object.lines[index] = s(1);
                    reindex(object);
                }
                "ad" => {
                    let separator = s(0);
                    ensure!(!separator.is_empty(), "QLIE list delimiter");
                    let text = s(1);
                    if !text.is_empty() {
                        object.lines.extend(crate::program::split_native(
                            &text,
                            &separator,
                            n(2)? == 1,
                            n(3)? == 1,
                        ));
                        reindex(object);
                    }
                }
                "?i" => m.result(
                    0,
                    Value::Int(
                        object
                            .lines
                            .iter()
                            .position(|v| v == &s(0))
                            .map(|n| n as i32)
                            .unwrap_or(-1),
                    ),
                ),
                "d" => {
                    let index = usize::try_from(n(0)?)?;
                    ensure!(index < object.lines.len(), "QLIE line deletion index");
                    object.lines.remove(index);
                    reindex(object);
                }
                "a" => object.lines.push(s(0)),
                "asn" => {
                    let source = self
                        .objects
                        .get(&s(0))
                        .context("QLIE table/list assignment source")?;
                    ensure!(source.kind == kind, "QLIE table/list assignment type");
                    let (lines, table) = (source.lines.clone(), source.table.clone());
                    let target = self.objects.get_mut(&id).unwrap();
                    target.lines = lines;
                    target.table = table;
                }
                "sort" => object.lines.sort_by_key(|v| v.to_lowercase()),
                "bpos" => {
                    let text = s(0).to_lowercase();
                    let found = object.lines.iter().enumerate().rev().find_map(|(i, line)| {
                        let (key, value) = line.split_once('=')?;
                        (!key.is_empty() && text.contains(&key.to_lowercase()))
                            .then(|| (i, key.to_owned(), value.to_owned()))
                    });
                    m.result_text(0, "");
                    m.result_text(1, "");
                    m.result(0, Value::Int(0));
                    m.result(1, Value::Int(-1));
                    if let Some((i, key, value)) = found {
                        m.result_text(0, key);
                        m.result_text(1, value.clone());
                        m.result(
                            0,
                            Value::Int(crate::expression::parse_integer(&value).unwrap_or(0)),
                        );
                        m.result(1, Value::Int(i as i32));
                    }
                }
                "brep" => {
                    let mut text = s(0);
                    for line in object.lines.iter().rev() {
                        if let Some((key, value)) = line.split_once('=') {
                            if !key.is_empty() {
                                text = text.replace(key, value);
                                ensure!(text.len() <= 1024 * 1024, "QLIE table replacement output");
                            }
                        }
                    }
                    m.result_text(0, text);
                }
                "idx" | "idxn" => {
                    let key = s(0);
                    let index = object
                        .lines
                        .iter()
                        .position(|l| {
                            l.split_once('=')
                                .is_some_and(|(n, _)| n.eq_ignore_ascii_case(&key))
                        })
                        .map(|v| v as i32)
                        .unwrap_or(-1);
                    m.result(0, Value::Int(index));
                }

                "comb" => {
                    let other = self
                        .objects
                        .get(&s(0))
                        .context("QLIE table merge source")?
                        .table
                        .clone();
                    let object = self.objects.get_mut(&id).unwrap();
                    for (key, value) in other {
                        set_name_value(object, &key, &value);
                    }
                }
                _ => bail!("unsupported QLIE {kind} method \\{op}"),
            }
            return Ok(true);
        }
        if kind == "dlgs" {
            return self.dialog_method(h, m, op, &a);
        }
        if kind == "cfg" {
            match op {
                "vtv" => {
                    ensure!(
                        a.len() == 2 && (0..=4).contains(&n(0)?),
                        "QLIE audio volume type"
                    );
                    let gain = n(1)? as f32 / 256.;
                    ensure!((0.0..=4.).contains(&gain), "QLIE audio volume range");
                    h.submit_audio(
                        self.client,
                        AudioRequest::SetMixGroupGain {
                            name: format!("qlie/{}", n(0)?),
                            gain,
                            fade_ms: 0.,
                        },
                    )?;
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(format!("vtv/{}", n(0)?), vec![Value::Int(n(1)?)]);
                }
                "vtm" => {
                    ensure!(
                        a.len() == 2 && (0..=4).contains(&n(0)?),
                        "QLIE audio mute type"
                    );
                    let group = format!("qlie/{}", n(0)?);
                    let gain = self.objects[&id]
                        .properties
                        .get(&format!("vtv/{}", n(0)?))
                        .and_then(|v| v.first())
                        .map(Value::integer)
                        .transpose()?
                        .unwrap_or(256) as f32
                        / 256.;
                    h.submit_audio(
                        self.client,
                        AudioRequest::SetMixGroup {
                            name: group,
                            settings: crysirror_core::service::AudioMixGroupSettings {
                                muted: n(1)? != 0,
                                gain,
                                parent: None,
                            },
                            fade_ms: 0.,
                        },
                    )?;
                }
                "?cv" | "?cvv" | "?cvm" | "cvv" | "cvm" => {
                    let name = s(0).to_lowercase();
                    let source = self
                        .text
                        .get("system/voice/voicedata.txt")
                        .context("QLIE character voice catalog missing")?;
                    let mut aliases = BTreeMap::<usize, Vec<String>>::new();
                    for line in source
                        .lines()
                        .map(str::trim)
                        .filter(|l| !l.starts_with(';') && !l.starts_with("//"))
                    {
                        if let Some((key, value)) = line.split_once('=') {
                            if let Some(index) = key
                                .trim()
                                .to_ascii_lowercase()
                                .strip_prefix("act[")
                                .and_then(|s| s.strip_suffix(']'))
                                .and_then(|s| s.parse::<usize>().ok())
                                .filter(|i| *i < 64)
                            {
                                aliases.insert(
                                    index,
                                    value
                                        .split(',')
                                        .map(|s| s.trim().trim_matches('"').to_lowercase())
                                        .collect(),
                                );
                            }
                        }
                    }
                    // Native 6EA5F8 scans groups in index order and accepts
                    // either an exact alias or the group's wildcard entry.
                    let index = aliases
                        .iter()
                        .find(|(_, v)| v.iter().any(|s| s == &name || s == "*"))
                        .map(|(i, _)| *i);
                    if op == "?cv" {
                        m.result(0, Value::Int(index.map(|i| i as i32).unwrap_or(-1)));
                    } else {
                        let index = index.context("QLIE unknown character voice alias")?;
                        let key = format!("{}/{}", op.trim_start_matches('?'), index);
                        if op.starts_with('?') {
                            m.result(
                                0,
                                self.objects[&id]
                                    .properties
                                    .get(&key)
                                    .and_then(|v| v.first())
                                    .cloned()
                                    .unwrap_or(Value::Int(if op == "?cvv" { 256 } else { 0 })),
                            );
                        } else {
                            ensure!(a.len() == 2, "QLIE character voice setting ABI");
                            let value = n(1)?;
                            ensure!(
                                if op == "cvv" {
                                    (0..=256).contains(&value)
                                } else {
                                    (0..=1).contains(&value)
                                },
                                "QLIE character voice setting range"
                            );
                            self.objects
                                .get_mut(&id)
                                .unwrap()
                                .properties
                                .insert(key, vec![Value::Int(value)]);
                        }
                    }
                }
                "ms" | "mtw" | "mcw" => {
                    ensure!(n(0)? >= 0 && n(0)? <= 60000, "QLIE message timing range");
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(op.into(), a);
                }
                "?ms" | "?mtw" | "?mcw" => {
                    let key = op.trim_start_matches('?');
                    let default = match key {
                        "ms" => 30,
                        "mtw" => 40,
                        _ => 1200,
                    };
                    m.result(
                        0,
                        self.objects[&id]
                            .properties
                            .get(key)
                            .and_then(|v| v.first())
                            .cloned()
                            .unwrap_or(Value::Int(default)),
                    );
                }
                "fn2" => {
                    let fonts = h.font_library(self.client)?;
                    let alias = fonts
                        .resolve_alias(&s(1))?
                        .or(fonts.resolve_alias("default")?)
                        .context("QLIE configured font and fallback are unavailable")?;
                    let o = self.objects.get_mut(&id).unwrap();
                    o.properties
                        .insert(format!("font_alias/{}", n(0)?), vec![Value::Text(alias)]);
                    o.properties
                        .insert(format!("fn2/{}", n(0)?), a[1..].to_vec());
                }
                "fst2" | "faa2" | "fsc2" => {
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(format!("{op}/{}", n(0)?), a[1..].to_vec());
                }
                "?fn" => m.result_text(0, "default"),
                "?faa" => m.result(0, Value::Int(2)),
                "?fst" => m.result(0, Value::Int(1)),
                "?fsc" => m.result(0, Value::Int(0)),
                _ => bail!("unsupported QLIE cfg method \\{op}"),
            }
            return Ok(true);
        }
        if matches!(kind.as_str(), "log" | "llog" | "flog") {
            let o = self.objects.get_mut(&id).unwrap();
            match op {
                "lk" => o.locked = n(0)? != 0,
                "?lk" => m.result(0, Value::Int(i32::from(o.locked))),
                "i" | "cc" => {
                    o.lines.clear();
                    o.table.clear();
                    o.properties.clear();
                }
                "a" => {
                    if !o.locked {
                        if kind == "log" {
                            // TImoObjectDataLog / 5D39A0 uses a 256-entry ring.
                            // Four strings and four integers are independent
                            // result banks, including omitted argument defaults.
                            ensure!(a.len() <= 8, "QLIE message log append ABI");
                            let mut entry = Vec::with_capacity(8);
                            for i in 0..4 {
                                entry.push(Value::Text(
                                    a.get(i).map(Value::text).unwrap_or_default(),
                                ));
                            }
                            for i in 4..8 {
                                entry.push(Value::Int(
                                    a.get(i).map(Value::integer).transpose()?.unwrap_or(0),
                                ));
                            }
                            if o.lines.len() == 256 {
                                o.lines.remove(0);
                            }
                            o.lines.push(serde_json::to_string(&entry)?);
                        } else {
                            ensure!(o.lines.len() < 8192, "QLIE native log limit");
                            o.lines.push(serde_json::to_string(&a)?);
                        }
                    }
                }
                "s" if kind == "log" => {
                    ensure!((1..=9).contains(&a.len()), "QLIE message log replace ABI");
                    // 6DF3A8 / 5D3C30: index zero is the newest entry;
                    // an index outside the ring leaves it unchanged.
                    if let Ok(index) = usize::try_from(n(0)?) {
                        if let Some(index) = o.lines.len().checked_sub(index.saturating_add(1)) {
                            let mut entry = Vec::with_capacity(8);
                            for i in 1..5 {
                                entry.push(Value::Text(
                                    a.get(i).map(Value::text).unwrap_or_default(),
                                ));
                            }
                            for i in 5..9 {
                                entry.push(Value::Int(
                                    a.get(i).map(Value::integer).transpose()?.unwrap_or(0),
                                ));
                            }
                            o.lines[index] = serde_json::to_string(&entry)?;
                        }
                    }
                }
                "?c" => m.result(0, Value::Int(o.lines.len() as i32)),
                "?g" if kind == "log" => {
                    let index = usize::try_from(n(0)?)?;
                    let entry = o
                        .lines
                        .len()
                        .checked_sub(index + 1)
                        .and_then(|i| o.lines.get(i))
                        .map(|s| serde_json::from_str::<Vec<Value>>(s))
                        .transpose()?
                        .unwrap_or_default();
                    for i in 0..4 {
                        m.result_text(i, entry.get(i).map(Value::text).unwrap_or_default());
                        m.result(
                            i,
                            Value::Int(
                                entry
                                    .get(i + 4)
                                    .map(Value::integer)
                                    .transpose()?
                                    .unwrap_or(0),
                            ),
                        );
                    }
                }
                "?g" | "g" if kind != "log" => {
                    let key = s(0).to_ascii_lowercase();
                    let value = o.table.get(&key).cloned().unwrap_or_default();
                    m.result(
                        0,
                        Value::Int(crate::expression::parse_integer(&value).unwrap_or(0)),
                    );
                    m.result_text(0, value);
                }
                "s" if kind != "log" => {
                    o.table.insert(s(0).to_ascii_lowercase(), s(1));
                }
                "?k2" | "k2" | "k" if kind == "llog" => {
                    m.result(0, Value::Int(i32::from(self.last_read)))
                }
                _ => bail!("unsupported QLIE native {kind} method \\{op}"),
            }
            return Ok(true);
        }
        if matches!(kind.as_str(), "tdat" | "mdat") {
            let o = self.objects.get_mut(&id).unwrap();
            match op {
                "lim" => {
                    ensure!(
                        a.len() == 1 && (0..=256).contains(&n(0)?),
                        "QLIE checkpoint count limit"
                    );
                    o.properties.insert("lim".into(), a);
                }
                "?lim" => m.result(
                    0,
                    o.properties
                        .get("lim")
                        .and_then(|v| v.first())
                        .cloned()
                        .unwrap_or(Value::Int(16)),
                ),
                "i" => {
                    o.lines.clear();
                    o.table.clear();
                }
                "g" | "?g" => {
                    ensure!(a.len() == 1, "QLIE checkpoint lookup ABI");
                    m.result(0, Value::Int(0));
                }
                "d" => {
                    ensure!(a.len() == 1, "QLIE checkpoint remove ABI");
                    m.result(0, Value::Int(0));
                }
                "s" => {
                    ensure!((1..=3).contains(&a.len()), "QLIE checkpoint save ABI");
                    // 6FD388 / s acknowledges a failed capture with ResultInt=0.
                    // This bridge cannot yet capture/restore the complete native
                    // reader, resource and audio state. Keep its store empty and
                    // report that failure instead of minting an unusable save.
                    let capability = "native temporary checkpoint capture/restore".to_owned();
                    if self.unavailable.insert(capability.clone()) {
                        let message=format!("QLIE {}:{} checkpoint save failed: {capability} is unavailable; ResultInt[0]=0",m.frame.path,ins.line);
                        h.append_log(self.client, &message)?;
                        eprintln!("CrySirror: {message}");
                    }
                    m.result(0, Value::Int(0));
                }
                "l" => {
                    ensure!(a.len() == 1, "QLIE checkpoint load ABI");
                    m.result(0, Value::Int(0));
                }
                _ => bail!("unsupported QLIE native checkpoint method \\{op}"),
            }
            return Ok(true);
        }
        if kind == "dat" {
            match op {
                "g" | "?g" => {
                    ensure!(a.len() == 1, "QLIE save metadata query ABI");
                    return self.query_save(h, m, &s(0));
                }
                _ => bail!("unsupported QLIE original-save method \\{op}"),
            }
        }
        if kind == "rep" {
            if op == "?r" {
                m.result_text(0, self.rewrite_line(&s(0))?);
                return Ok(true);
            }
            if op == "a" && n(2)? != 0 {
                ensure!(
                    (1..=3).contains(&n(2)?) && self.regex_rules.len() < 4096,
                    "QLIE replacement mode/budget"
                );
                self.regex_rules.insert(
                    s(0),
                    regex::RegexBuilder::new(&s(0))
                        .size_limit(1024 * 1024)
                        .build()?,
                );
            }
            if op == "d" {
                self.regex_rules.remove(&s(0));
            }
            if op == "i" {
                self.regex_rules.clear();
            }
            let o = self.objects.get_mut(&id).unwrap();
            match op {
                "lk" => o.locked = n(0)? != 0,
                "a" => {
                    ensure!((2..=3).contains(&a.len()), "QLIE replacement rule");
                    o.table.insert(s(0), s(1));
                    o.properties
                        .insert(format!("rule:{}", s(0)), vec![Value::Int(n(2)?)]);
                }
                "d" => {
                    o.table.remove(&s(0));
                }
                "i" => {
                    o.table.clear();
                    o.properties.clear();
                    o.locked = false;
                }
                "et" | "nt" => {
                    o.properties.insert(op.into(), a);
                }
                "?c" => m.result(0, Value::Int(o.table.len() as i32)),
                "?lk" => m.result(0, Value::Int(i32::from(o.locked))),
                _ => bail!("unsupported QLIE replacement method \\{op}"),
            }
            return Ok(true);
        }
        if kind == "misc" && op == "prm" {
            ensure!(a.is_empty(), "QLIE command-line parameter ABI");
            // Native 6EF018 / prm returns ParamCount and at most 16 strings.
            // Host renderer/input options are never exposed as game arguments.
            m.result(0, Value::Int(self.config.parameters.len() as i32));
            for i in 0..16 {
                m.result_text(
                    i,
                    self.config.parameters.get(i).cloned().unwrap_or_default(),
                );
            }
            return Ok(true);
        }
        if kind == "skey" {
            let o = self.objects.get_mut(&id).unwrap();
            match op {
                "i" => {
                    o.properties.clear();
                    o.locked = false;
                }
                "lk" => o.locked = n(0)? != 0,
                "?lk" => m.result(0, Value::Int(i32::from(o.locked))),
                "a" => {
                    ensure!(
                        (2..=5).contains(&a.len()) && o.properties.len() < 512,
                        "QLIE key binding"
                    );
                    let code = n(0)?;
                    o.properties.insert(code.to_string(), a[1..].to_vec());
                    o.properties.insert(
                        format!("source/{code}"),
                        vec![Value::Text(m.frame.path.clone())],
                    );
                }
                _ => bail!("unsupported QLIE system key method \\{op}"),
            };
            return Ok(true);
        }
        if kind == "sys" {
            let o = self.objects.get_mut(&id).unwrap();
            match op {
                "dskp" => {
                    m.result(0, Value::Int(i32::from(h.now_ms() < self.skip_until)));
                }
                "?skp" => {
                    let mode = o
                        .properties
                        .get("p")
                        .and_then(|v| v.first())
                        .map(Value::integer)
                        .transpose()?
                        .unwrap_or(0);
                    m.result(
                        0,
                        Value::Int(i32::from(
                            mode >= 3
                                || self.input.buttons_down & crysirror_core::service::buttons::CTRL
                                    != 0,
                        )),
                    );
                }
                "?mrd" => {
                    let frame = std::iter::once(&m.frame)
                        .chain(m.stack.iter().rev())
                        .find(|f| {
                            f.path.starts_with("script/") && !f.path.starts_with("script/title/")
                        })
                        .unwrap_or(&m.frame);
                    let label = m.programs[&frame.path]
                        .labels
                        .iter()
                        .filter(|(_, pc)| **pc <= frame.pc)
                        .max_by_key(|(_, pc)| **pc)
                        .map(|(s, _)| s.clone())
                        .unwrap_or_default();
                    m.result_text(0, frame.path.clone());
                    m.result_text(1, label);
                    m.result(0, Value::Int(i32::try_from(m.frame.pc)?));
                }
                "eve" => {
                    ensure!(a.len() == 2, "QLIE system event ABI");
                    ensure!(
                        s(0).eq_ignore_ascii_case("OnFrame"),
                        "unsupported QLIE system event {}",
                        s(0)
                    );
                    o.properties.insert(
                        format!("eve:{}", s(0).to_ascii_lowercase()),
                        vec![a[1].clone(), Value::Text(m.frame.path.clone())],
                    );
                }
                "ap" => {
                    o.properties.insert("ap".into(), a);
                }
                "?mt" => {
                    m.result(0, Value::Int(i32::try_from(self.tick_ms)?));
                }
                "?ap" => {
                    m.result(
                        0,
                        o.properties
                            .get("ap")
                            .and_then(|v| v.first())
                            .cloned()
                            .unwrap_or(Value::Int(0)),
                    );
                }
                "lk" | "lkc" | "p" | "n2" | "tz" => {
                    o.properties.insert(op.into(), a);
                }
                "?lk" | "?lkc" | "?p" => {
                    m.result(
                        0,
                        o.properties
                            .get(op.trim_start_matches('?'))
                            .and_then(|v| v.first())
                            .cloned()
                            .unwrap_or(Value::Int(0)),
                    );
                }
                _ => bail!("unsupported QLIE system method \\{op}"),
            }
            return Ok(true);
        }
        if matches!(kind.as_str(), "cam" | "camex") {
            return self.camera_method(h, m, &id, op, &a);
        }
        if matches!(kind.as_str(), "msg" | "btn" | "sbtn")
            && self.message_method(h, m, &id, ins, &a)?
        {
            return Ok(true);
        }
        if kind == "eff" {
            return self.effect(h, m, op, &a);
        }
        if kind == "inp" {
            match op {
                "?mp" => {
                    m.result(0, Value::Int(self.input.pointer_x));
                    m.result(1, Value::Int(self.input.pointer_y));
                }
                "gbc" => {
                    m.globals.insert("resultobj[0]".into(), Value::Int(0));
                    let result = h.query_engine(self.client, EngineQuery::UiEvents)?;
                    let events: serde_json::Value = serde_json::from_slice(&result)?;
                    for e in events.as_array().context("QLIE UI event receipt")? {
                        if e["kind"] == "activate" {
                            let button = e["widget"]["id"]
                                .as_u64()
                                .or_else(|| e["widget"].as_u64())
                                .context("QLIE UI event widget")?
                                as u32;
                            if let Some((id, o)) =
                                self.objects.iter().find(|(_, o)| o.layer == button)
                            {
                                m.globals
                                    .insert("resultobj[0]".into(), Value::Text(id.clone()));
                                m.result_text(
                                    0,
                                    o.properties
                                        .get("us")
                                        .and_then(|v| v.first())
                                        .map(Value::text)
                                        .unwrap_or_default(),
                                );
                            }
                        }
                    }
                }
                "mge" | "swe" => {
                    ensure!(n(0)? == 0, "QLIE pointer gesture profile is not installed");
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(op.into(), a);
                }
                "sc" | "?sc" => {
                    use crysirror_core::service::buttons as B;
                    let mut clicked =
                        self.input.buttons_pressed & (B::MOUSE_LEFT | B::ENTER | B::SPACE) != 0;
                    if let Some(keys) = self.objects.get("skey").filter(|o| !o.locked) {
                        for (key, binding) in &keys.properties {
                            let Ok(code) = key.parse::<i32>() else {
                                continue;
                            };
                            if !binding.first().is_some_and(|v| {
                                matches!(
                                    v.text().to_ascii_lowercase().as_str(),
                                    "click" | "keyclick"
                                )
                            }) {
                                continue;
                            }
                            let bits = if code & 0x10000 != 0 {
                                self.input.buttons_pressed
                            } else {
                                self.input.buttons_down
                            };
                            clicked |= bits & native_key(code)? != 0;
                        }
                    }
                    m.result(0, Value::Int(i32::from(clicked)));
                }
                "?k2" | "k2" => {
                    let mut result = 0;
                    for value in &a {
                        let key = value.integer()?;
                        let mask = native_key(key)?;
                        let bits = if key & 0x10000 != 0 {
                            self.input.buttons_pressed
                        } else {
                            self.input.buttons_down
                        };
                        if bits & mask != 0 {
                            result = key;
                            break;
                        }
                    }
                    m.result(0, Value::Int(result));
                }
                _ => bail!("unsupported QLIE input method \\{op}"),
            }
            return Ok(true);
        }
        if matches!(
            kind.as_str(),
            "img" | "btn" | "msg" | "simg" | "sbtn" | "theora" | "snd"
        ) {
            match op {
                "i" => {
                    let old = self.objects.remove(&id).unwrap();
                    let layer = old.layer;
                    let kind = old.kind.clone();
                    self.free_object(h, old)?;
                    self.objects.insert(
                        id,
                        Object {
                            kind,
                            layer,
                            gain: 1.,
                            ..Default::default()
                        },
                    );
                }
                "l" => {
                    if s(0).is_empty() {
                        let mut old = self.objects.remove(&id).unwrap();
                        if kind == "snd" {
                            if old.channel != 0 {
                                h.submit_audio(
                                    self.client,
                                    AudioRequest::Stop {
                                        channel: old.channel,
                                        fade_ms: 0.,
                                    },
                                )?;
                            }
                            old.properties.remove("sk");
                        }
                        let mut message = old.message.take();
                        if let Some(message) = &mut message {
                            message.created = false;
                            message.bound_parent = None;
                        }
                        self.free_visual(h, &old)?;
                        self.objects.insert(
                            id,
                            Object {
                                kind: old.kind,
                                layer: old.layer,
                                position: old.position,
                                properties: old.properties,
                                gain: old.gain,
                                message,
                                ..Default::default()
                            },
                        );
                        return Ok(true);
                    }
                    let path = self.resolve(m, &s(0))?;
                    if kind == "theora" || kind == "snd" {
                        let o = self.objects.get_mut(&id).unwrap();
                        if kind == "snd" && o.asset.as_deref() != Some(&path) {
                            // Native 5C3F8C -> 582A88 -> 582858 closes the old
                            // decoder and starts the new source at zero. The
                            // previous source's seek is not a new-file offset.
                            if o.channel != 0 {
                                h.submit_audio(
                                    self.client,
                                    AudioRequest::Stop {
                                        channel: o.channel,
                                        fade_ms: 0.,
                                    },
                                )?;
                            }
                            o.channel = 0;
                            o.properties.remove("sk");
                        }
                        o.asset = Some(path);
                        return Ok(true);
                    }
                    return self.load_image(h, &id, &path);
                }
                "li" => {
                    let source = s(1);
                    let path = if source.is_empty() {
                        source
                    } else {
                        self.resolve(m, &source)?
                    };
                    return self.replace_image(h, &id, usize::try_from(n(0)?)?, &path);
                }
                "ls" => {
                    let index = usize::try_from(n(0)?)?;
                    ensure!(index < 256, "QLIE animation sound slot limit");
                    let path = self.resolve(m, &s(1))?;
                    if self.load.is_none() {
                        let storage = h.storage(self.client)?;
                        let source = path.clone();
                        let client = self.client;
                        self.load = Some((
                            path.clone(),
                            self.pool.spawn(move |cancel| {
                                resources::load(storage, client, &source, &cancel)
                            })?,
                        ));
                    }
                    let (expected, job) = self.load.as_mut().unwrap();
                    ensure!(
                        expected == &path,
                        "QLIE replacement sound completion identity"
                    );
                    let Some(loaded) = job.try_take()? else {
                        return Ok(false);
                    };
                    self.load = None;
                    let resources::Loaded::Blob(bytes) = loaded else {
                        bail!("QLIE animation sound replacement is not encoded audio")
                    };
                    let bundle = self
                        .objects
                        .get_mut(&id)
                        .unwrap()
                        .bundle
                        .as_mut()
                        .context("QLIE sound replacement needs animation")?;
                    while bundle.sounds.len() <= index {
                        bundle.sounds.push(resources::Member {
                            name: String::new(),
                            bytes: Arc::from([]),
                            metadata: json!({}),
                            slot: usize::MAX,
                            nested: None,
                        });
                    }
                    let sound = &mut bundle.sounds[index];
                    sound.bytes = bytes;
                    sound.name = path;
                }
                "s" if kind == "snd" => {
                    ensure!(a.len() <= 1, "QLIE sound stop ABI");
                    let channel = self.objects[&id].channel;
                    if channel != 0 {
                        h.submit_audio(
                            self.client,
                            AudioRequest::Stop {
                                channel,
                                fade_ms: n(0)?.max(0) as f32,
                            },
                        )?;
                    }
                }
                "p" | "p2" if kind == "snd" => {
                    if self.objects[&id].asset.is_none() {
                        return Ok(true);
                    }
                    let loops = if op == "p" && n(0)? != 0 { -1 } else { n(0)? };
                    if loops == -2 && self.objects[&id].channel != 0 {
                        h.submit_audio(
                            self.client,
                            AudioRequest::Resume {
                                channel: self.objects[&id].channel,
                                delay_ms: 0.,
                                fade_ms: 0.,
                            },
                        )?;
                        return Ok(true);
                    }
                    return self.play_sound(h, &id, loops);
                }
                "fo" if kind == "snd" => {
                    ensure!(s(0).is_empty(), "QLIE sound filter not installed: {}", s(0));
                }
                "sk" if kind == "snd" => {
                    ensure!(a.len() == 1 && n(0)? >= 0, "QLIE sound seek ABI");
                    if let Some((owner, ticket)) = &self.audio_seek {
                        ensure!(owner == &id, "QLIE sound seek continuation owner");
                        let status = h.audio_seek_preparation(self.client, *ticket)?;
                        match status.phase {
                            AudioPreparePhase::Pending => return Ok(false),
                            AudioPreparePhase::Ready => {
                                h.release_audio_seek(self.client, *ticket)?;
                                self.audio_seek = None;
                            }
                            AudioPreparePhase::Failed => {
                                let error = status.error;
                                h.release_audio_seek(self.client, *ticket)?;
                                self.audio_seek = None;
                                bail!("QLIE sound seek failed: {error:?}");
                            }
                        }
                    } else {
                        let channel = self.objects[&id].channel;
                        if channel != 0 {
                            if let Some(info) = h.audio_info(self.client, channel)? {
                                let frame = (n(0)? as u64)
                                    .checked_mul(u64::from(info.source_rate))
                                    .context("QLIE sound seek frame overflow")?
                                    / 1000;
                                let ticket = h.begin_audio_seek(self.client, channel, frame)?;
                                self.audio_seek = Some((id.clone(), ticket));
                                return Ok(false);
                            }
                        }
                    }
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(op.into(), a);
                }
                "pan" if kind == "snd" => {
                    let o = self.objects.get_mut(&id).unwrap();
                    let channel = o.channel;
                    o.properties.insert(op.into(), a.clone());
                    if channel != 0 {
                        h.submit_audio(
                            self.client,
                            AudioRequest::SetPan {
                                channel,
                                pan: (n(0)? as f32 / 512. + 0.5).clamp(0., 1.),
                                fade_ms: n(1)?.max(0) as f32,
                            },
                        )?;
                    }
                }
                "?nt" | "?tt" if kind == "snd" => {
                    let info = h.audio_info(self.client, self.objects[&id].channel)?;
                    let time = info
                        .map(|i| {
                            if op == "?nt" {
                                i.position_frames.saturating_mul(1000)
                                    / u64::from(i.source_rate.max(1))
                            } else {
                                i.source_frames.unwrap_or(0).saturating_mul(1000)
                                    / u64::from(i.source_rate.max(1))
                            }
                        })
                        .unwrap_or(0);
                    m.result(0, Value::Int(i32::try_from(time)?));
                }
                "p" if kind == "theora" => {
                    if self.objects[&id].media.is_none() {
                        let value = (0..1024)
                            .find(|n| !self.media_ids.contains(n))
                            .context("QLIE media IDs exhausted")?;
                        self.media_ids.insert(value);
                        self.objects.get_mut(&id).unwrap().media = Some(value);
                    }
                    let o = &self.objects[&id];
                    let asset = o.asset.clone().context("QLIE movie not loaded")?;
                    let layer = o.media.unwrap();
                    let image = h.media_request(
                        self.client,
                        MediaRequest::Play {
                            id: layer,
                            asset,
                            looping: false,
                            audio: true,
                            gain: o.gain,
                        },
                    )?;
                    self.objects.get_mut(&id).unwrap().properties.insert(
                        "media_image".into(),
                        vec![Value::Text(image.relative().into())],
                    );
                    self.put_parent(h, &id)?;
                }
                "?tt" if kind == "theora" => {
                    let info = h.media_info(
                        self.client,
                        self.objects[&id].media.context("QLIE movie not playing")?,
                    )?;
                    if matches!(info.phase, MediaPhase::Loading) {
                        return Ok(false);
                    }
                    let duration = info
                        .duration_ms
                        .context("QLIE movie decoder duration unavailable")?;
                    m.result(0, Value::Int(i32::try_from(duration)?));
                }
                "p4" => {
                    ensure!((11..=12).contains(&a.len()), "QLIE full image pose ABI");
                    let repeat = animation::repeat(&self.objects[&id], "pose_repeat", n(11)?);
                    self.objects.get_mut(&id).unwrap().position = [n(0)?, n(1)?, n(9)?];
                    self.position_repeated(h, &id, n(10)?.max(0) as u64, repeat)?;
                    let o = &self.objects[&id];
                    let layer = o.layer;
                    let easing = animation::easing(
                        o.properties
                            .get("mm")
                            .and_then(|v| v.first())
                            .and_then(|v| v.integer().ok())
                            .unwrap_or(3),
                    )?;
                    let mut commands = Vec::new();
                    for (i, property) in [
                        "z",
                        "scale_x",
                        "scale_y",
                        "scale_z",
                        "rotation_x",
                        "rotation_y",
                        "rotation_z",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let index = i + 2;
                        let value = if index == 2 {
                            n(index)? as f32
                        } else if index < 6 {
                            n(index)? as f32 / 100.
                        } else {
                            (n(index)? as f32).to_radians()
                        };
                        commands.push(json!({"op":"begin_motion","id":layer,"track":{"property":property,"to":value,"duration":n(10)?.max(0),"repeat":repeat,"easing":easing}}));
                    }
                    self.submit(h, json!({"layers":commands}))?;
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert("pose_repeat".into(), vec![Value::Int(repeat)]);
                }
                "alg" | "alg2" => {
                    let factor = if op == "alg2" { 1 } else { 50 };
                    self.objects.get_mut(&id).unwrap().properties.insert(
                        "alg".into(),
                        vec![Value::Int(n(0)? * factor), Value::Int(n(1)? * factor)],
                    );
                    self.put_parent(h, &id)?;
                    let o = &self.objects[&id];
                    let size = o
                        .bundle
                        .as_ref()
                        .map(|b| b.animation.size)
                        .or_else(|| o.images.first().and_then(|v| v.as_ref()).map(|v| v.size))
                        .unwrap_or(self.config.canvas);
                    let layer: serde_json::Value = serde_json::from_slice(
                        &h.query_engine(self.client, EngineQuery::Layer(o.layer))?,
                    )?;
                    let mut transform = layer["transform"].clone();
                    transform["object_anchor"] = json!(true);
                    transform["texture_center_x"] =
                        json!(size[0] as f32 * n(0)? as f32 * factor as f32 / 100.);
                    transform["texture_center_y"] =
                        json!(size[1] as f32 * n(1)? as f32 * factor as f32 / 100.);
                    self.submit(h,json!({"layers":[{"op":"set_transform","id":o.layer,"transform":transform}]}))?;
                }
                "g2" => {
                    self.put_parent(h, &id)?;
                    self.generator_method(h, &id, op, &a)?;
                }
                "dc" | "sc" | "in" | "out" => {
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(op.into(), a);
                }
                "ps" => {
                    ensure!((0..=3000).contains(&n(0)?), "QLIE animation play speed");
                    let o = self.objects.get_mut(&id).unwrap();
                    let old = o
                        .properties
                        .get("ps")
                        .and_then(|v| v.first())
                        .and_then(|v| v.integer().ok())
                        .unwrap_or(100)
                        .max(0) as u64;
                    if o.visible {
                        o.offset = o.offset.saturating_add(
                            h.now_ms().saturating_sub(o.started).saturating_mul(old) / 100,
                        );
                    }
                    o.started = h.now_ms();
                    o.properties.insert(op.into(), a);
                }
                "?us" | "?ui" => {
                    let values = self.objects[&id]
                        .properties
                        .get(op.trim_start_matches('?'))
                        .cloned()
                        .unwrap_or_default();
                    for i in 0..4 {
                        if op == "?us" {
                            m.result_text(i, values.get(i).map(Value::text).unwrap_or_default());
                        } else {
                            m.result(i, values.get(i).cloned().unwrap_or(Value::Int(0)));
                        }
                    }
                }
                "?m" => {
                    let o = &self.objects[&id];
                    let name = o
                        .bundle
                        .as_ref()
                        .and_then(|b| b.animation.motions.get(o.motion))
                        .map(|v| v.name.clone())
                        .unwrap_or_default();
                    m.result_text(0, name);
                    m.result(0, Value::Int(o.motion as i32));
                }
                "midx" => {
                    let index = self.objects[&id]
                        .bundle
                        .as_ref()
                        .and_then(|b| b.animation.motions.iter().position(|v| v.name == s(0)))
                        .map(|v| v as i32)
                        .unwrap_or(-1);
                    m.result(0, Value::Int(index));
                }
                "?e" => {
                    m.result(0, Value::Int(i32::from(self.objects[&id].visible)));
                }
                "bm" => {
                    let blend = match n(0)? {
                        0 | 1 => "normal",
                        2 => "screen",
                        3 => "multiply",
                        7 => "add",
                        8 => "subtract",
                        v => bail!("unsupported QLIE image blend {v}"),
                    };
                    self.put_parent(h, &id)?;
                    // Store native blend for child paint publication, where it
                    // composes against the actual lower-priority scene.
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert("blend".into(), vec![Value::Text(blend.into())]);
                }
                "p" | "pp" => {
                    ensure!((3..=4).contains(&a.len()), "QLIE image position ABI");
                    let o = self.objects.get_mut(&id).unwrap();
                    o.position = [n(0)?, n(1)?, n(2)?];
                    self.position(
                        h,
                        &id,
                        a.get(3)
                            .map(Value::integer)
                            .transpose()?
                            .unwrap_or(0)
                            .max(0) as u64,
                    )?;
                }
                "?p" if kind == "snd" => {
                    let playing = h
                        .audio_info(self.client, self.objects[&id].channel)?
                        .is_some_and(|s| s.source_playing);
                    m.result(0, Value::Int(i32::from(playing)));
                }
                "?p" => {
                    let o = &self.objects[&id];
                    let mut position = o.position;
                    if o.created {
                        let layer: serde_json::Value = serde_json::from_slice(
                            &h.query_engine(self.client, EngineQuery::Layer(o.layer))?,
                        )?;
                        position[0] =
                            layer["rect"][0].as_f64().context("QLIE position query X")? as i32;
                        position[1] =
                            layer["rect"][1].as_f64().context("QLIE position query Y")? as i32;
                    }
                    for (i, value) in position.into_iter().enumerate() {
                        m.result(i, Value::Int(value));
                    }
                }
                "e" | "d" => {
                    let o = self.objects.get_mut(&id).unwrap();
                    let visible = op == "e";
                    if visible != o.visible {
                        if o.visible {
                            let speed = o
                                .properties
                                .get("ps")
                                .and_then(|v| v.first())
                                .and_then(|v| v.integer().ok())
                                .unwrap_or(100)
                                .max(0) as u64;
                            o.offset = o.offset.saturating_add(
                                h.now_ms().saturating_sub(o.started).saturating_mul(speed) / 100,
                            );
                        }
                        o.started = h.now_ms();
                        o.visible = visible;
                    }
                    self.put_parent(h, &id)?;
                }
                "m" => {
                    let motion = if let Some(Value::Text(name)) =
                        a.first().filter(|v| v.integer().is_err())
                    {
                        match self.objects[&id].bundle.as_ref().and_then(|b| {
                            b.animation
                                .motions
                                .iter()
                                .position(|v| v.name.eq_ignore_ascii_case(name))
                        }) {
                            Some(index) => index,
                            None => return Ok(true),
                        }
                    } else {
                        if n(0)? == -1 {
                            return Ok(true);
                        }
                        usize::try_from(n(0)?)?
                    };
                    let o = self.objects.get_mut(&id).unwrap();
                    if o.motion == motion && n(1)? == 0 {
                        return Ok(true);
                    }
                    o.motion = motion;
                    o.offset = 0;
                    o.started = h.now_ms();
                    o.events.clear();
                    o.animation_clocks.clear();
                }
                "skp" => {
                    let o = self.objects.get_mut(&id).unwrap();
                    o.offset = n(0)?.max(0) as u64;
                    o.started = h.now_ms();
                    for clock in o.animation_clocks.values_mut() {
                        clock.seek(o.offset);
                    }
                }
                "lk" => {
                    self.objects.get_mut(&id).unwrap().locked = n(0)? != 0;
                    self.put_parent(h, &id)?;
                }
                "v" if kind == "snd" => {
                    let o = self.objects.get_mut(&id).unwrap();
                    o.gain = n(0)? as f32 / 256.;
                    o.properties.insert("v".into(), a.clone());
                    if o.channel != 0 {
                        h.submit_audio(
                            self.client,
                            AudioRequest::SetGain {
                                channel: o.channel,
                                gain: o.gain,
                                fade_ms: n(1)?.max(0) as f32,
                            },
                        )?;
                    }
                }
                "?v" if kind == "snd" => {
                    let o = &self.objects[&id];
                    let gain = if o.channel == 0 {
                        o.gain
                    } else {
                        h.audio_info(self.client, o.channel)?
                            .map(|voice| voice.gain)
                            .unwrap_or(o.gain)
                    };
                    m.result(0, Value::Int((gain * 256.).round() as i32));
                }
                "exr" => {
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .retain(|k, _| !k.starts_with("exv/"));
                }
                "exv" | "?exv" => {
                    ensure!(
                        (0..32).contains(&n(0)?),
                        "QLIE animation external variable index"
                    );
                    let key = format!("exv/{}", n(0)?);
                    if op == "exv" {
                        self.objects
                            .get_mut(&id)
                            .unwrap()
                            .properties
                            .insert(key, vec![a.get(1).cloned().unwrap_or(Value::Int(0))]);
                    } else {
                        m.result(
                            0,
                            self.objects[&id]
                                .properties
                                .get(&key)
                                .and_then(|v| v.first())
                                .cloned()
                                .unwrap_or(Value::Int(0)),
                        );
                    }
                }
                "fo" => {
                    ensure!((0..32).contains(&n(0)?), "QLIE filter slot");
                    return self.image_filter(h, &id, n(0)? as usize, &s(1));
                }
                "g" => {
                    self.put_parent(h, &id)?;
                    self.generator_method(h, &id, op, &a)?;
                }
                "col" => {
                    ensure!((4..=5).contains(&a.len()), "QLIE image colour ABI");
                    let repeat = animation::repeat(&self.objects[&id], "colour_repeat", n(4)?);
                    self.put_parent(h, &id)?;
                    let layer = self.objects[&id].layer;
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(op.into(), a.clone());
                    let duration = n(3)?.max(0);
                    let easing = animation::easing(
                        self.objects[&id]
                            .properties
                            .get("mm")
                            .and_then(|v| v.first())
                            .and_then(|v| v.integer().ok())
                            .unwrap_or(3),
                    )?;
                    let mut commands = vec![
                        json!({"op":"begin_motion","id":layer,"track":{"property":"opacity","to":n(0)?.clamp(0,255),"duration":duration,"repeat":repeat,"easing":easing}}),
                    ];
                    if n(1)? != -1 {
                        let color = n(1)? as u32;
                        for (property, shift) in [
                            ("multiply_red", 16),
                            ("multiply_green", 8),
                            ("multiply_blue", 0),
                        ] {
                            commands.push(json!({"op":"begin_motion","id":layer,"track":{"property":property,"to":((color>>shift)&255) as f32/255.,"duration":duration,"repeat":repeat,"easing":easing}}));
                        }
                    }
                    self.submit(h, json!({"layers":commands}))?;
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert("colour_repeat".into(), vec![Value::Int(repeat)]);
                }
                "v" => {
                    self.objects.get_mut(&id).unwrap().gain = n(0)? as f32 / 256.;
                }
                "?dc" => {
                    m.result(
                        0,
                        self.objects[&id]
                            .properties
                            .get("dc")
                            .and_then(|v| v.first())
                            .cloned()
                            .unwrap_or(Value::Int(0)),
                    );
                }
                "ds" | "hs" => {
                    ensure!(
                        a.len() == 2 || a.len() == 4,
                        "QLIE drawing/hit rectangle ABI"
                    );
                    let rect = if a.len() == 2 {
                        [0, 0, n(0)?, n(1)?]
                    } else {
                        [
                            n(0)?,
                            n(1)?,
                            n(2)?.saturating_sub(n(0)?),
                            n(3)?.saturating_sub(n(1)?),
                        ]
                    };
                    ensure!(
                        rect[2] >= 0 && rect[3] >= 0,
                        "QLIE drawing/hit rectangle order"
                    );
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(op.into(), rect.map(Value::Int).to_vec());
                    if op == "ds" {
                        self.put_parent(h, &id)?;
                        let layer = self.objects[&id].layer;
                        if rect[2] == 0 || rect[3] == 0 {
                            self.submit(h,json!({"layers":[{"op":"set_viewport_clip","id":layer,"clip":[0,0,0,0]}]}))?;
                        } else {
                            self.submit(h,json!({"layers":[{"op":"set_clip","id":layer,"clip":rect},{"op":"set_viewport_clip","id":layer,"clip":null}]}))?;
                        }
                    }
                }
                "gsf" => {
                    if a.is_empty() {
                        self.objects.get_mut(&id).unwrap().skip_flags = [false; 32];
                        return Ok(true);
                    }
                    ensure!(
                        a.len() == 2 && (0..32).contains(&n(0)?),
                        "QLIE goto-skip flag ABI"
                    );
                    self.objects.get_mut(&id).unwrap().skip_flags[n(0)? as usize] = n(1)? != 0;
                }
                "gfk" => {
                    // Image dispatcher native0x6B6667 writes TABMP+3564;
                    // an absent/255 entry falls back to the jump key's Value3.
                    if a.is_empty() || (n(0)? == 0 && n(1)? == 0) {
                        self.objects.get_mut(&id).unwrap().jump_targets.clear();
                    } else {
                        ensure!(
                            a.len() == 2 && (1..32).contains(&n(0)?),
                            "QLIE goto-frame-key ABI"
                        );
                        self.objects
                            .get_mut(&id)
                            .unwrap()
                            .jump_targets
                            .insert(n(0)? as u8, n(1)?.clamp(0, 255) as u8);
                    }
                }
                "ui" | "us" => {
                    let o = self.objects.get_mut(&id).unwrap();
                    if op == "us" {
                        o.properties.insert(
                            "callback_file".into(),
                            vec![Value::Text(m.frame.path.clone())],
                        );
                    }
                    o.properties.insert(op.into(), a);
                }
                "sp" | "tf" | "tab" | "mm" | "vtn" => {
                    self.objects
                        .get_mut(&id)
                        .unwrap()
                        .properties
                        .insert(op.into(), a);
                }
                _ => bail!("unsupported QLIE {kind} method \\{op}"),
            }
            return Ok(true);
        }
        bail!("unsupported QLIE object {kind} method \\{op}")
    }
    fn play_sound(&mut self, h: &mut dyn HostServices, id: &str, loops: i32) -> Result<bool> {
        if let Some(ticket) = self.audio_ticket {
            let status = h.audio_preparation(self.client, ticket).with_context(|| {
                let object = &self.objects[id];
                format!(
                    "QLIE sound {id} source {:?}, seek {:?}",
                    object.asset,
                    object.properties.get("sk")
                )
            })?;
            match status.phase {
                AudioPreparePhase::Pending => return Ok(false),
                AudioPreparePhase::Failed => bail!("QLIE audio preparation: {:?}", status.error),
                AudioPreparePhase::Ready => {
                    h.release_audio(self.client, ticket)?;
                    self.audio_ticket = None;
                    return Ok(true);
                }
            }
        }
        let path = self.objects[id]
            .asset
            .clone()
            .context("QLIE audio not loaded")?;
        let o = self.objects.get_mut(id).unwrap();
        o.channel = o.layer;
        let group = o
            .properties
            .get("vtn")
            .and_then(|v| v.first())
            .map(Value::integer)
            .transpose()?
            .unwrap_or(0);
        let ticket = h.begin_audio(
            self.client,
            AudioRequest::PlayAsset {
                channel: o.channel,
                source: AudioAsset {
                    path: VfsPath::new(Mount::Asset, &path)?,
                    frame_limit: None,
                },
                intro: None,
                parameters: AudioParameters {
                    looping: loops != 0,
                    loop_passes: if loops > 0 {
                        Some(loops as u32 + 1)
                    } else {
                        None
                    },
                    pan: o
                        .properties
                        .get("pan")
                        .and_then(|v| v.first())
                        .and_then(|v| v.integer().ok())
                        .map(|v| (v as f32 / 512. + 0.5).clamp(0., 1.))
                        .unwrap_or(0.5),
                    start_ms: o
                        .properties
                        .get("sk")
                        .and_then(|v| v.first())
                        .and_then(|v| v.integer().ok())
                        .unwrap_or(0)
                        .max(0) as u64,
                    gain: o.gain,
                    fade_ms: o
                        .properties
                        .get("v")
                        .and_then(|v| v.get(1))
                        .and_then(|v| v.integer().ok())
                        .unwrap_or(0)
                        .max(0) as f32,
                    mix_group: Some(format!("qlie/{group}")),
                    bus: if group == 0 {
                        PcmBus::Music
                    } else if group == 1 {
                        PcmBus::Voice
                    } else {
                        PcmBus::Sfx
                    },
                    ..Default::default()
                },
            },
        )?;
        self.audio_ticket = Some(ticket);
        Ok(false)
    }
}
