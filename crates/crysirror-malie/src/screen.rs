// SPDX-License-Identifier: MPL-2.0
//! Native SVG DOM and Tweener binding policy. Squirrel runs unchanged source;
//! transforms and interpolated values are sent to shared retained layers.
//! Ragnarok 563EB0 loads title/title.svg; 563FD0 invokes showLogo once, then
//! showTitle on later visits, with restore for the native return modes.
use anyhow::{bail, ensure, Context, Result};
use crysirror_scene::{animation::ScalarTween, layer::Easing};
use crysirror_squirrel::Interpreter;
use quick_xml::{events::Event, Reader};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};
#[derive(Clone)]
pub struct Node {
    pub kind: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<usize>,
    values: BTreeMap<String, f32>,
}
impl Node {
    fn number(&self, key: &str, default: f32) -> Result<f32> {
        self.attrs.get(key).map_or(Ok(default), |v| {
            v.trim_end_matches("px")
                .parse()
                .with_context(|| format!("Malie SVG {key}={v:?}"))
        })
    }
    fn value(&self, key: &str, default: f32) -> f32 {
        self.values.get(key).copied().unwrap_or(default)
    }
}
struct Tween {
    node: usize,
    values: Vec<(String, ScalarTween)>,
    at: u64,
    duration: u64,
    callback: Option<u32>,
    begun: bool,
}
pub enum Action {
    Tag(String),
    Label(String),
    Dialog(String),
    /// gotoLabel: jump without returning to the caller.
    Goto(String),
    SetFlag(String, u32),
    EndModal(u32),
    EndDialog,
    SystemSe(String),
    SetConfig(String, String),
}
pub struct Document {
    pub nodes: Vec<Node>,
    pub ids: BTreeMap<String, usize>,
    templates: BTreeSet<usize>,
    pub script: String,
    pub now: u64,
    pub started: u64,
    tweens: Vec<Tween>,
    pub actions: VecDeque<Action>,
    pub focus: Option<String>,
    hover: Option<String>,
    pending_focus: bool,
    hits: Vec<(String, [f32; 4])>,
    pub root: String,
    pub flags: BTreeMap<String, u32>,
    /// Retained layer/widget id bases and draw-order major for this document.
    pub layer_base: u32,
    pub widget_base: u32,
    pub order_major: i32,
    pub config: BTreeMap<String, bool>,
    /// Native dialog state: checked buttons, scroll values, list cursors,
    /// rich-text contents, replaced image sources and hidden elements.
    pub checked: BTreeMap<String, bool>,
    pub scroll: BTreeMap<String, f32>,
    pub list_cursor: BTreeMap<String, usize>,
    pub list_hover: Option<(String, usize)>,
    pub texts: BTreeMap<String, String>,
    pub images: BTreeMap<String, String>,
    pub hidden: BTreeSet<String>,
    pub disabled: BTreeSet<String>,
    pub list_hits: Vec<(String, usize, [f32; 4])>,
    pub scroll_hits: Vec<(String, [f32; 4], [f32; 4])>,
    pub text_requests: Vec<TextRequest>,
    pub dragging: Option<String>,
    /// Asset paths known to exist (MalieSystem.isExistFile).
    pub files: BTreeSet<String>,
    listeners: Vec<(usize, String, u32)>,
}
/// A `ui:richtext` element to draw with the owner's font: canvas rect,
/// text, pixel size, colour and edge colour.
#[derive(Clone, Debug)]
pub struct TextRequest {
    pub id: String,
    pub rect: [f32; 4],
    pub text: String,
    pub size: f32,
    pub color: [u8; 4],
    pub edge: Option<[u8; 4]>,
    pub parent: u32,
    pub order: i32,
}
/// Pointer result for dialog policies.
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    Button(String),
    List(String, usize),
    Scroll(String, f32),
}
impl Document {
    pub fn parse(text: &str, root: &str) -> Result<Self> {
        ensure!(text.len() <= 1024 * 1024, "Malie SVG source budget");
        let mut reader = Reader::from_str(text);
        let mut nodes: Vec<Node> = Vec::new();
        let mut ids = BTreeMap::new();
        let mut parents: Vec<usize> = Vec::new();
        let mut script = String::new();
        loop {
            let event = reader.read_event()?;
            let empty = matches!(&event, Event::Empty(_));
            match event {
                Event::Start(e) | Event::Empty(e) => {
                    let kind = String::from_utf8(e.name().as_ref().to_vec())?;
                    let mut attrs = BTreeMap::new();
                    for attr in e.attributes() {
                        let attr = attr?;
                        attrs.insert(
                            String::from_utf8(attr.key.as_ref().to_vec())?,
                            attr.decode_and_unescape_value(reader.decoder())?
                                .into_owned(),
                        );
                    }
                    ensure!(
                        nodes.len() < 4096 && parents.len() < 128,
                        "Malie SVG node/nesting budget"
                    );
                    let index = nodes.len();
                    // Shipped dialogs repeat ids (an image and an empty
                    // template group); lookups resolve the first element.
                    if let Some(id) = attrs.get("id") {
                        ids.entry(id.clone()).or_insert(index);
                    }
                    if kind == "m:script" {
                        script = attrs
                            .get("xlink:href")
                            .cloned()
                            .context("Malie SVG script href")?;
                    }
                    if let Some(parent) = parents.last() {
                        nodes[*parent].children.push(index);
                    }
                    nodes.push(Node {
                        kind,
                        attrs,
                        children: Vec::new(),
                        values: BTreeMap::new(),
                    });
                    if !empty {
                        parents.push(index);
                    }
                }
                Event::End(_) => {
                    parents.pop().context("Malie SVG close without parent")?;
                }
                Event::Eof => break,
                Event::DocType(_) => bail!("Malie SVG document types are unsupported"),
                _ => {}
            }
        }
        ensure!(
            !nodes.is_empty() && nodes[0].kind == "svg" && parents.is_empty(),
            "Malie SVG root framing"
        );
        let mut templates = BTreeSet::new();
        // Elements referenced as button/list/scroll states or by `use` are
        // symbols: they are drawn only through their references.
        for node in &nodes {
            for (key, value) in &node.attrs {
                let template = match node.kind.as_str() {
                    "ui:button" => key == "on" || key.starts_with("on_") || key.starts_with("off"),
                    "ui:listbox" => matches!(key.as_str(), "cursor" | "over" | "focus"),
                    "ui:scrollbar" => key.starts_with("slider-"),
                    "use" => key == "xlink:href",
                    _ => false,
                };
                if template {
                    if let Some(index) = value.strip_prefix('#').and_then(|id| ids.get(id)) {
                        templates.insert(*index);
                    }
                }
            }
        }
        Ok(Self {
            nodes,
            ids,
            templates,
            script,
            now: 0,
            started: 0,
            tweens: Vec::new(),
            actions: VecDeque::new(),
            focus: None,
            hover: None,
            pending_focus: false,
            hits: Vec::new(),
            root: root.into(),
            flags: BTreeMap::new(),
            layer_base: 700_000,
            widget_base: 800_000,
            order_major: 50,
            config: BTreeMap::new(),
            checked: BTreeMap::new(),
            scroll: BTreeMap::new(),
            list_cursor: BTreeMap::new(),
            list_hover: None,
            texts: BTreeMap::new(),
            images: BTreeMap::new(),
            hidden: BTreeSet::new(),
            disabled: BTreeSet::new(),
            list_hits: Vec::new(),
            scroll_hits: Vec::new(),
            text_requests: Vec::new(),
            dragging: None,
            files: BTreeSet::new(),
            listeners: Vec::new(),
        })
    }
    fn node(&self, id: &str) -> Result<usize> {
        self.ids
            .get(id)
            .copied()
            .with_context(|| format!("Malie DOM element {id:?} not found"))
    }
    fn request(&mut self, args: Vec<Value>) -> Result<Value> {
        ensure!(!args.is_empty(), "Malie Squirrel host command missing");
        let op = args[0].as_str().context("Malie host command name")?;
        let text = |i: usize| -> Result<&str> {
            args.get(i)
                .and_then(Value::as_str)
                .context("Malie host string operand")
        };
        match op {
            "print" => Ok(Value::Null),
            "exists" => Ok(json!(self.ids.contains_key(text(1)?))),
            "get" => {
                let node = &self.nodes[self.node(text(1)?)?];
                Ok(json!(node.attrs.get(text(2)?).cloned().unwrap_or_default()))
            }
            "set" => {
                let id = self.node(text(1)?)?;
                let key = text(2)?.to_owned();
                let v = args.get(3).context("Malie DOM attribute value")?;
                self.nodes[id].attrs.insert(
                    key,
                    v.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| v.to_string()),
                );
                Ok(Value::Null)
            }
            "focus" => {
                self.node(text(1)?)?;
                self.focus = Some(text(1)?.into());
                self.pending_focus = true;
                Ok(Value::Null)
            }
            "play" => Ok(Value::Null),
            "flag" => Ok(json!(self.flags.get(text(1)?).copied().unwrap_or(0))),
            "config" => {
                let key = text(1)?;
                let value = self.config.get(key).copied();
                ensure!(value.is_some(), "Malie config query {key} not connected");
                Ok(json!(value.unwrap()))
            }
            "tag" | "label" | "dialog" | "goto" | "se" => {
                ensure!(self.actions.len() < 1024, "Malie UI action queue budget");
                self.actions.push_back(match op {
                    "tag" => Action::Tag(text(1)?.into()),
                    "label" => Action::Label(text(1)?.into()),
                    "goto" => Action::Goto(text(1)?.into()),
                    "se" => Action::SystemSe(text(1)?.into()),
                    _ => Action::Dialog(text(1)?.into()),
                });
                Ok(Value::Null)
            }
            "setflag" => {
                let v = args.get(2).and_then(Value::as_i64).unwrap_or(0) as u32;
                self.flags.insert(text(1)?.into(), v);
                self.actions.push_back(Action::SetFlag(text(1)?.into(), v));
                Ok(Value::Null)
            }
            "endmodal" => {
                self.actions.push_back(Action::EndModal(args.get(1).and_then(Value::as_i64).unwrap_or(0) as u32));
                Ok(Value::Null)
            }
            "enddialog" => {
                self.actions.push_back(Action::EndDialog);
                Ok(Value::Null)
            }
            "setconfig" => {
                let v = args.get(2).map(|v| v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string())).unwrap_or_default();
                self.actions.push_back(Action::SetConfig(text(1)?.into(), v));
                Ok(Value::Null)
            }
            "isskip" => Ok(json!(self.config.get("skip").copied().unwrap_or(false))),
            "isdebug" => Ok(json!(false)),
            "exists_file" => Ok(json!(self.files.contains(&text(1)?.replace('\\', "/").trim_start_matches("./").to_ascii_lowercase()))),
            "time" => Ok(json!(self.now.saturating_sub(self.started) as f64 / 1000.)),
            "settime" => {
                let t = args.get(1).and_then(Value::as_f64).unwrap_or(0.);
                ensure!(t.is_finite() && (0. ..=86_400.).contains(&t), "Malie setCurrentTime");
                self.started = self.now.saturating_sub((t * 1000.) as u64);
                Ok(Value::Null)
            }
            "untween" => {
                let node = self.node(text(1)?)?;
                self.tweens.retain(|t| t.node != node);
                Ok(Value::Null)
            }
            "listen" => {
                let node = self.node(text(1)?)?;
                let event = text(2)?.to_owned();
                let f = args.get(3).and_then(|v| v["$function"].as_u64()).context("Malie event listener closure")? as u32;
                ensure!(self.listeners.len() < 1024, "Malie listener budget");
                self.listeners.push((node, event, f));
                Ok(Value::Null)
            }
            "create" => {
                ensure!(self.nodes.len() < 8192, "Malie DOM budget");
                let kind = text(1)?.to_owned();
                let id = format!("__created{}", self.nodes.len());
                let mut attrs = BTreeMap::new();
                attrs.insert("id".into(), id.clone());
                self.ids.insert(id.clone(), self.nodes.len());
                self.nodes.push(Node { kind, attrs, children: Vec::new(), values: BTreeMap::new() });
                Ok(json!(id))
            }
            "insert" => {
                // insertBefore(new, ref): attach under ref's parent before ref
                // (or at the end of the document root when ref is null).
                let new = self.node(text(2)?)?;
                let reference = args.get(3).and_then(Value::as_str).map(|r| self.node(r)).transpose()?;
                for n in &mut self.nodes {
                    n.children.retain(|c| *c != new);
                }
                let (parent, at) = match reference {
                    Some(r) => {
                        let p = self.nodes.iter().position(|n| n.children.contains(&r)).unwrap_or(0);
                        (p, self.nodes[p].children.iter().position(|c| *c == r).unwrap_or(0))
                    }
                    None => (self.node(text(1)?)?, usize::MAX),
                };
                let children = &mut self.nodes[parent].children;
                let at = at.min(children.len());
                children.insert(at, new);
                Ok(Value::Null)
            }
            "tween" => {
                let node = self.node(text(1)?)?;
                let p = args
                    .get(2)
                    .and_then(Value::as_object)
                    .context("Malie tween table")?;
                ensure!(self.tweens.len() < 4096, "Malie active tween budget");
                let seconds = |key: &str| -> Result<u64> {
                    let v = p
                        .get(key)
                        .map_or(Ok(0.), |v| v.as_f64().context("Malie tween time"))?;
                    ensure!(
                        v.is_finite() && (0. ..=600.).contains(&v),
                        "Malie tween time budget"
                    );
                    Ok((v * 1000.) as u64)
                };
                let delay = seconds("delay")?;
                let duration = seconds("time")?;
                let at = self
                    .now
                    .checked_add(delay)
                    .context("Malie tween clock overflow")?;
                let easing = match p
                    .get("transition")
                    .and_then(Value::as_str)
                    .unwrap_or("linear")
                {
                    "linear" => Easing::Linear,
                    "easeOutSine" => serde_json::from_value(
                        json!({"progress":{"shape":{"kind":"sine","direction":"out"}}}),
                    )?,
                    other => bail!("Malie Tweener transition {other:?}"),
                };
                let mut values = Vec::new();
                for (key, value) in p {
                    if matches!(key.as_str(), "time" | "delay" | "transition" | "onComplete") {
                        continue;
                    }
                    ensure!(
                        matches!(
                            key.as_str(),
                            "alpha" | "x" | "y" | "cx" | "cy" | "sx" | "sy"
                        ),
                        "Malie Tweener property {key}"
                    );
                    let end = value.as_f64().context("Malie tween numeric property")? as f32;
                    ensure!(
                        end.is_finite() && end.abs() <= 1_000_000.,
                        "Malie tween property budget"
                    );
                    let default = if matches!(key.as_str(), "alpha" | "sx" | "sy") {
                        1.
                    } else {
                        0.
                    };
                    let start = self.nodes[node].value(key, default);
                    values.push((
                        key.clone(),
                        ScalarTween::new(start, end, at, duration, easing)?,
                    ));
                }
                let callback = p
                    .get("onComplete")
                    .map(|v| {
                        v["$function"]
                            .as_u64()
                            .and_then(|n| u32::try_from(n).ok())
                            .context("Malie tween completion closure")
                    })
                    .transpose()?;
                if delay == 0 && duration == 0 {
                    for (key, curve) in values {
                        self.nodes[node].values.insert(key, curve.end);
                    }
                    if let Some(reference) = callback {
                        self.tweens.push(Tween {
                            node,
                            values: Vec::new(),
                            at,
                            duration,
                            callback: Some(reference),
                            begun: true,
                        });
                    }
                } else {
                    self.tweens.push(Tween {
                        node,
                        values,
                        at,
                        duration,
                        callback,
                        begun: false,
                    });
                }
                Ok(Value::Null)
            }
            _ => bail!("Malie UI host operation {op:?} is not connected"),
        }
    }
    pub fn images(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for node in &self.nodes {
            if node.kind == "image" {
                let path = node
                    .attrs
                    .get("xlink:href")
                    .context("Malie SVG image href")?;
                names.push(self.image_path(path)?);
            }
        }
        names.sort();
        names.dedup();
        Ok(names)
    }
    fn image_path(&self, href: &str) -> Result<String> {
        ensure!(
            !href.contains(':') && !href.starts_with('/') && !href.split('/').any(|v| v == ".."),
            "Malie SVG image escapes document root"
        );
        let path = format!("{}/{}", self.root, href.trim_start_matches("./"));
        Ok(crate::picture::libp_path(&if path.to_ascii_lowercase().ends_with(".png") {
            format!("{}.dzi", &path[..path.len() - 4])
        } else {
            path
        }))
    }
    fn tick(&mut self, now: u64) -> Result<Vec<u32>> {
        ensure!(now >= self.now, "Malie DOM clock moved backwards");
        self.now = now;
        let mut complete = Vec::new();
        let mut remaining = Vec::new();
        for mut tween in std::mem::take(&mut self.tweens) {
            if now >= tween.at {
                if !tween.begun {
                    for (key, curve) in &mut tween.values {
                        let default = if matches!(key.as_str(), "alpha" | "sx" | "sy") {
                            1.
                        } else {
                            0.
                        };
                        curve.start = self.nodes[tween.node].value(key, default);
                    }
                    tween.begun = true;
                }
                for (key, curve) in &tween.values {
                    self.nodes[tween.node]
                        .values
                        .insert(key.clone(), curve.sample(now));
                }
            }
            if now >= tween.at && now - tween.at >= tween.duration {
                if let Some(reference) = tween.callback {
                    complete.push(reference);
                }
            } else {
                remaining.push(tween);
            }
        }
        self.tweens = remaining;
        Ok(complete)
    }
    fn emit(
        &mut self,
        index: usize,
        parent: Option<u32>,
        basis: [f32; 4],
        layers: &mut Vec<Value>,
        depth: usize,
        referenced: bool,
    ) -> Result<()> {
        ensure!(
            depth <= 128 && layers.len() < 8192,
            "Malie SVG reference cycle/budget"
        );
        if !referenced && self.templates.contains(&index) {
            return Ok(());
        }
        let n = self.nodes[index].clone();
        if matches!(
            n.kind.as_str(),
            "defs" | "filter" | "m:script" | "animate" | "feBlend" | "clipPath"
        ) {
            return Ok(());
        }
        let alpha = n.value("alpha", n.number("opacity", 1.)?);
        let base = [
            n.number("x", 0.)?,
            n.number("y", 0.)?,
            n.number("width", 0.)?,
            n.number("height", 0.)?,
        ];
        let mut sx = n.value("sx", 1.);
        let mut sy = n.value("sy", 1.);
        let x = n.value("x", 0.) - n.value("cx", 0.);
        let y = n.value("y", 0.) - n.value("cy", 0.);
        if n.attrs.get("id").is_some_and(|i| self.hidden.contains(i)) {
            return Ok(());
        }
        let mut parent = parent;
        let mut basis = basis;
        if let Some(transform) = n.attrs.get("transform") {
            let m = crate::picture::transform_affine(transform)?;
            ensure!(m[1] == 0. && m[2] == 0., "Malie SVG rotated transform {transform:?}");
            if m[4] == 0. && m[5] == 0. {
                sx *= m[0];
                sy *= m[3];
            } else {
                // A translating transform wraps the element's own frame.
                let wrap = self.layer_base + layers.len() as u32;
                layers.push(json!({"op":"put","layer":{"id":wrap,"parent":parent,"rect":[m[4],m[5],1.,1.],"paint":{"kind":"group"},"order":[self.order_major,layers.len() as i32],"transform":{"scale_x":m[0],"scale_y":m[3]}}}));
                parent = Some(wrap);
                basis = [basis[0] * m[0], basis[1] * m[3], basis[2] + basis[0] * m[4], basis[3] + basis[1] * m[5]];
            }
        }
        let id = self.layer_base + layers.len() as u32;
        let paint = match n.kind.as_str() {
            "image" => {
                let dynamic = n.attrs.get("id").and_then(|i| self.images.get(i)).cloned();
                match (dynamic, n.attrs.get("xlink:href")) {
                    (Some(asset), _) => json!({"kind":"image","asset":asset}),
                    (None, Some(href)) => json!({"kind":"image","asset":self.image_path(href)?}),
                    // Placeholder images (thumbnails) draw nothing until assigned.
                    (None, None) => return Ok(()),
                }
            }
            "rect" => {
                let fill = n.attrs.get("fill").map_or("black", String::as_str);
                let c = crate::picture::color_of(fill)?;
                json!({"kind":"solid","color":c})
            }
            "svg" | "g" | "use" | "ui:button" | "ui:custom" | "ui:listbox" | "ui:scrollbar" | "ui:include" => json!({"kind":"group"}),
            "ui:richtext" => {
                let name = n.attrs.get("id").cloned().unwrap_or_default();
                if let Some(text) = self.texts.get(&name).filter(|t| !t.is_empty()).cloned() {
                    let size = n.number("fontsize", 20.)? * basis[1];
                    let color = n.attrs.get("color").filter(|c| !c.is_empty()).map(|c| crate::picture::color_of(c)).transpose()?.unwrap_or([255, 255, 255, 255]);
                    let edge = n.attrs.get("edge_color").filter(|c| !c.is_empty()).map(|c| crate::picture::color_of(c)).transpose()?;
                    let rect = [basis[2] + basis[0] * base[0], basis[3] + basis[1] * base[1], basis[0] * base[2], basis[1] * base[3]];
                    self.text_requests.push(TextRequest { id: name, rect, text, size, color, edge, parent: parent.unwrap_or(self.layer_base), order: self.order_major });
                }
                return Ok(());
            }
            "animateTransform" | "text" | "ui:item" | "ui:textedit" => return Ok(()),
            other => bail!("Malie SVG element {other:?} is not mapped"),
        };
        let mut opacity = (crate::adv::gamma_alpha(alpha) * 255.).round() as u8;
        for &child in &n.children {
            let anim = &self.nodes[child];
            if anim.kind == "animate" {
                ensure!(
                    anim.attrs
                        .get("attributeName")
                        .is_some_and(|s| s == "opacity"),
                    "Malie SVG animation property"
                );
                let duration = anim.number("dur", 0.)?;
                ensure!(duration > 0., "Malie SVG animation duration");
                let indefinite = anim
                    .attrs
                    .get("repeatDur")
                    .is_some_and(|s| s == "indefinite");
                let elapsed = self.now.saturating_sub(self.started) as f32 / 1000.;
                let p = if indefinite {
                    (elapsed / duration).fract()
                } else {
                    (elapsed / duration).min(1.)
                };
                let values = anim
                    .attrs
                    .get("values")
                    .context("Malie SVG animation values")?
                    .split(';')
                    .map(str::parse::<f32>)
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                ensure!(
                    (2..=64).contains(&values.len()),
                    "Malie SVG animation key count"
                );
                let f = p * (values.len() - 1) as f32;
                let i = (f.floor() as usize).min(values.len() - 2);
                let value = values[i] + (values[i + 1] - values[i]) * (f - i as f32);
                opacity = (f32::from(opacity) * crate::adv::gamma_alpha(value)).round() as u8;
            }
        }
        // SVG containers have no raster extent. Shared Layer groups retain
        // a positive carrier rectangle; its size does not scale/clip children.
        let rect = if n.kind == "ui:button" {
            [x, y, 1., 1.]
        } else {
            [base[0] + x, base[1] + y, base[2].max(1.), base[3].max(1.)]
        };
        // `clip-path='url(#id)'`: the clipPath's first rect, in this frame.
        let clip = n.attrs.get("clip-path").and_then(|c| c.strip_prefix("url(#")).and_then(|c| c.strip_suffix(')')).and_then(|c| self.ids.get(c)).and_then(|&cp| {
            self.nodes[cp].children.iter().map(|&r| &self.nodes[r]).find(|r| r.kind == "rect").map(|r| {
                [r.number("x", 0.).unwrap_or(0.) - rect[0], r.number("y", 0.).unwrap_or(0.) - rect[1], r.number("width", 0.).unwrap_or(0.), r.number("height", 0.).unwrap_or(0.)]
            })
        });
        layers.push(json!({"op":"put","layer":{"id":id,"parent":parent,"rect":rect,"paint":paint,"opacity":opacity,"clip":clip,"order":[self.order_major,layers.len() as i32],"transform":{"scale_x":sx,"scale_y":sy,"pivot_x":n.value("cx",0.)-base[0],"pivot_y":n.value("cy",0.)-base[1]}}}));
        let own = [
            basis[0] * sx,
            basis[1] * sy,
            basis[2] + basis[0] * (x + n.value("cx", 0.) * (1. - sx)),
            basis[3] + basis[1] * (y + n.value("cy", 0.) * (1. - sy)),
        ];
        if n.kind == "use" {
            let href = n
                .attrs
                .get("xlink:href")
                .context("Malie SVG use reference")?;
            // Shipped frames reference unpublished templates (e.g. `#exit_n`);
            // the native DOM resolves those to nothing.
            let target = href.strip_prefix('#').context("Malie SVG external use unsupported")?;
            if let Some(target) = self.ids.get(target).copied() {
                self.emit(target, Some(id), own, layers, depth + 1, true)?;
            }
        } else if n.kind == "ui:button" {
            let name = n.attrs.get("id").context("Malie SVG button id")?;
            let enabled = n
                .attrs
                .get("enabled")
                .is_none_or(|s| s != "false" && s != "0")
                && !self.disabled.contains(name);
            let checked = self.checked.get(name).copied().unwrap_or(false) && n.attrs.contains_key("on");
            if !enabled && self.disabled.contains(name) {
                // Disabled native controls keep their normal picture.
                let key = if checked { "on" } else { "off" };
                if let Some(target) = n.attrs.get(key).and_then(|r| self.ids.get(r.trim_start_matches('#'))).copied() {
                    self.emit(target, Some(id), own, layers, depth + 1, true)?;
                }
            }
            if enabled {
                self.hits.push((
                    name.clone(),
                    [
                        own[2] + own[0] * base[0],
                        own[3] + own[1] * base[1],
                        own[0] * base[2],
                        own[1] * base[3],
                    ],
                ));
                let state = match (
                    self.focus.as_deref() == Some(name),
                    self.hover.as_deref() == Some(name),
                ) {
                    (true, true) => "focusover",
                    (true, false) => "focus",
                    (false, true) => "over",
                    _ => "",
                };
                let prefix = if checked { "on" } else { "off" };
                let key = if state.is_empty() { prefix.to_owned() } else { format!("{prefix}_{state}") };
                let reference = n
                    .attrs
                    .get(&key)
                    .or(n.attrs.get(prefix))
                    .context("Malie SVG button state")?;
                if let Some(target) = self.ids.get(reference.trim_start_matches('#')).copied() {
                    self.emit(target, Some(id), own, layers, depth + 1, true)?;
                }
            }
        } else if n.kind == "ui:listbox" {
            let name = n.attrs.get("id").cloned().unwrap_or_default();
            let cursor = self.list_cursor.get(&name).copied();
            let hover = self.list_hover.as_ref().filter(|(l, _)| *l == name).map(|(_, i)| *i);
            let focused = self.focus.as_deref() == Some(name.as_str());
            for (i, &item) in n.children.iter().enumerate() {
                let it = &self.nodes[item];
                if it.kind != "ui:item" {
                    continue;
                }
                let r = [it.number("x", 0.)?, it.number("y", 0.)?, it.number("width", 0.)?, it.number("height", 0.)?];
                let index = it.attrs.get("index").and_then(|v| v.parse::<usize>().ok()).map_or(i, |v| v.saturating_sub(1));
                self.list_hits.push((name.clone(), index, [own[2] + own[0] * (base[0] + r[0]), own[3] + own[1] * (base[1] + r[1]), own[0] * r[2], own[1] * r[3]]));
                for (key, on) in [("cursor", cursor == Some(index)), ("over", hover == Some(index)), ("focus", focused && cursor == Some(index))] {
                    if !on {
                        continue;
                    }
                    if let Some(target) = n.attrs.get(key).and_then(|r| self.ids.get(r.trim_start_matches('#'))).copied() {
                        let wrap = self.layer_base + layers.len() as u32;
                        layers.push(json!({"op":"put","layer":{"id":wrap,"parent":id,"rect":[r[0],r[1],1.,1.],"paint":{"kind":"group"},"order":[self.order_major,layers.len() as i32]}}));
                        let b = [own[0], own[1], own[2] + own[0] * (base[0] + r[0]), own[3] + own[1] * (base[1] + r[1])];
                        self.emit(target, Some(wrap), b, layers, depth + 1, true)?;
                    }
                }
            }
        } else if n.kind == "ui:scrollbar" {
            let name = n.attrs.get("id").cloned().unwrap_or_default();
            let (min, max) = (n.number("min", 0.)?, n.number("max", 100.)?);
            let bar = n.number("bar", 0.)?;
            let value = self.scroll.get(&name).copied().unwrap_or(min).clamp(min.min(max), max.max(min));
            let t = if max > min { (value - min) / (max - min) } else { 0. };
            let rect = [own[2] + own[0] * base[0], own[3] + own[1] * base[1], own[0] * base[2], own[1] * base[3]];
            let vertical = n.attrs.get("align").is_some_and(|a| a == "vertical");
            self.scroll_hits.push((name.clone(), rect, [min, max, if vertical { own[1] } else { own[0] } * bar, f32::from(u8::from(vertical))]));
            let state = if self.focus.as_deref() == Some(name.as_str()) { "slider-focus" } else if self.hover.as_deref() == Some(name.as_str()) { "slider-over" } else { "slider-normal" };
            if let Some(target) = n.attrs.get(state).or(n.attrs.get("slider-normal")).and_then(|r| self.ids.get(r.trim_start_matches('#'))).copied() {
                let vertical = n.attrs.get("align").is_some_and(|a| a == "vertical");
                let (kx, ky) = if vertical {
                    (base[2] / 2., bar / 2. + t * (base[3] - bar).max(0.))
                } else {
                    (bar / 2. + t * (base[2] - bar).max(0.), base[3] / 2.)
                };
                let wrap = self.layer_base + layers.len() as u32;
                layers.push(json!({"op":"put","layer":{"id":wrap,"parent":id,"rect":[kx,ky,1.,1.],"paint":{"kind":"group"},"order":[self.order_major,layers.len() as i32]}}));
                let b = [own[0], own[1], own[2] + own[0] * (base[0] + kx), own[3] + own[1] * (base[1] + ky)];
                self.emit(target, Some(wrap), b, layers, depth + 1, true)?;
            }
        } else {
            for child in n.children {
                self.emit(child, Some(id), own, layers, depth + 1, referenced)?;
            }
        }
        Ok(())
    }
    /// Pointer routing for native dialogs: hover/focus tracking, clicks on
    /// buttons and list items, and scroll-bar drags.
    pub fn pointer(&mut self, x: f32, y: f32, pressed: bool, down: bool) -> Vec<UiEvent> {
        let inside = |r: &[f32; 4]| x >= r[0] && y >= r[1] && x < r[0] + r[2] && y < r[1] + r[3];
        let mut events = Vec::new();
        if let Some(name) = self.dragging.clone() {
            if down {
                if let Some((_, r, [min, max, bar, vertical])) = self.scroll_hits.iter().find(|(n, ..)| *n == name).cloned() {
                    let t = if vertical > 0. {
                        ((y - r[1] - bar / 2.) / (r[3] - bar).max(1.)).clamp(0., 1.)
                    } else {
                        ((x - r[0] - bar / 2.) / (r[2] - bar).max(1.)).clamp(0., 1.)
                    };
                    let v = (min + t * (max - min)).round();
                    if self.scroll.get(&name) != Some(&v) {
                        self.scroll.insert(name.clone(), v);
                        events.push(UiEvent::Scroll(name, v));
                    }
                }
                return events;
            }
            self.dragging = None;
        }
        self.hover = self.hits.iter().rev().find(|(_, r)| inside(r)).map(|(n, _)| n.clone())
            .or_else(|| self.scroll_hits.iter().rev().find(|(_, r, _)| inside(r)).map(|(n, ..)| n.clone()));
        self.list_hover = self.list_hits.iter().rev().find(|(.., r)| inside(r)).map(|(n, i, _)| (n.clone(), *i));
        if !pressed {
            return events;
        }
        if let Some(name) = self.hover.clone() {
            if self.scroll_hits.iter().any(|(n, ..)| *n == name) {
                self.dragging = Some(name.clone());
                self.focus = Some(name.clone());
                return self.pointer(x, y, false, true);
            }
            self.focus = Some(name.clone());
            events.push(UiEvent::Button(name));
        } else if let Some((list, i)) = self.list_hover.clone() {
            self.list_cursor.insert(list.clone(), i);
            self.focus = Some(list.clone());
            events.push(UiEvent::List(list, i));
        }
        events
    }
    /// Reset per-render widget hit lists.
    pub fn clear_hits(&mut self) {
        self.hits.clear();
        self.list_hits.clear();
        self.scroll_hits.clear();
        self.text_requests.clear();
    }
    /// Canvas rectangle of an element after its ancestors' scale groups.
    /// Fire `button:mouseover`/`button:mouseout` listeners for hover changes.
    pub fn hover_events(&mut self, previous: Option<String>) -> Vec<u32> {
        let mut out = Vec::new();
        if previous == self.hover {
            return out;
        }
        for (name, event) in [(previous, "button:mouseout"), (self.hover.clone(), "button:mouseover")] {
            if let Some(index) = name.and_then(|n| self.ids.get(&n).copied()) {
                out.extend(self.listeners.iter().filter(|(n, e, _)| *n == index && e == event).map(|(.., f)| *f));
            }
        }
        out
    }
    pub fn rect_of(&self, id: &str) -> Result<Option<[f32; 4]>> {
        let Some(target) = self.ids.get(id).copied() else { return Ok(None) };
        fn walk(d: &Document, i: usize, s: [f32; 2], target: usize, depth: usize) -> Result<Option<[f32; 4]>> {
            ensure!(depth < 128, "Malie SVG nesting budget");
            let n = &d.nodes[i];
            if i == target {
                return Ok(Some([
                    n.number("x", 0.)? * s[0],
                    n.number("y", 0.)? * s[1],
                    n.number("width", 0.)? * s[0],
                    n.number("height", 0.)? * s[1],
                ]));
            }
            let mut s = s;
            if let Some(t) = n.attrs.get("transform").and_then(|t| t.strip_prefix("scale(")).and_then(|t| t.strip_suffix(')')) {
                let v: Vec<f32> = t.split(|c: char| c == ',' || c.is_ascii_whitespace()).filter(|v| !v.is_empty()).map(str::parse).collect::<std::result::Result<_, _>>()?;
                s = [s[0] * v[0], s[1] * *v.get(1).unwrap_or(&v[0])];
            }
            for &c in &n.children {
                if let Some(r) = walk(d, c, s, target, depth + 1)? {
                    return Ok(Some(r));
                }
            }
            Ok(None)
        }
        walk(self, 0, [1., 1.], target, 0)
    }
    pub fn layers(&mut self) -> Result<Vec<Value>> {
        let mut layers = Vec::new();
        self.clear_hits();
        self.emit(0, None, [1., 1., 0., 0.], &mut layers, 0, false)?;
        Ok(layers)
    }
    pub fn widgets(&mut self) -> Result<Vec<Value>> {
        let mut ui = Vec::new();
        for (name, rect) in &self.hits {
            let id = self.widget_base + self.node(name)? as u32;
            ui.push(json!({"op":"put","widget":{"id":id,"kind":"button","rect":rect,"interaction_only":true,"z":id,"keyboard_activation":true}}));
        }
        // Shared UI focus is a text-entry concept; DOM button focus stays in
        // this document and selects the `off_focus` templates.
        if self.pending_focus && self.focus.as_ref().is_some_and(|name| self.hits.iter().any(|(id, _)| id == name)) {
            self.pending_focus = false;
        }
        Ok(ui)
    }
    pub fn controls(&self) -> Result<Vec<(String, u32)>> {
        self.hits
            .iter()
            .map(|(name, _)| Ok((name.clone(), self.widget_base + self.node(name)? as u32)))
            .collect()
    }
}
/// Removal ops for `gone` layers, skipping those whose ancestor is also
/// removed (the shared layer system removes subtrees).
pub fn removals(gone: &BTreeSet<u32>, parents: &BTreeMap<u32, Option<u32>>) -> Vec<Value> {
    gone.iter()
        .filter(|id| {
            let mut p = parents.get(id).copied().flatten();
            for _ in 0..256 {
                match p {
                    Some(q) if gone.contains(&q) => return false,
                    Some(q) => p = parents.get(&q).copied().flatten(),
                    None => break,
                }
            }
            true
        })
        .map(|id| json!({"op":"remove","id":id}))
        .collect()
}
const HOST: &str = r#"
function print(text) { __host("print", text); }
function __element(id) { return { id=id,
    getAttribute=function(name){return __host("get",this.id,name);},
    setAttribute=function(name,value){return __host("set",this.id,name,value);},
    addEventListener=function(event,f){return __host("listen",this.id,event,f);},
    insertBefore=function(node,ref){return __host("insert",this.id,node.id,ref?ref.id:null);},
    focus=function(){__host("focus",this.id);} }; }
document <- { getElementById=function(id){return __host("exists",id)?__element(id):null;},
    createElement=function(kind){return __element(__host("create",kind));},
    setCurrentTime=function(t){return __host("settime",t);},
    play=function(){__host("play");}, pause=function(){__host("play");} };
Tweener <- { addTween=function(element,spec){return __host("tween",element.id,spec);},
    removeTween=function(element){return __host("untween",element.id);},
    getTime=function(){return __host("time");} };
MalieSystem <- {
    getFlag=function(name){return __host("flag",name);},
    setFlag=function(name,value){return __host("setflag",name,value);},
    getConfigValue=function(name){return __host("config",name);},
    setConfigValue=function(name,value){return __host("setconfig",name,value);},
    tag=function(text){return __host("tag",text);},
    cmd=function(text){return __host("tag",text);},
    playLabel=function(name){return __host("label",name);},
    gotoLabel=function(name){return __host("goto",name);},
    openDialog=function(name){return __host("dialog",name);},
    endDialog=function(){return __host("enddialog");},
    endModal=function(result){return __host("endmodal",result);},
    playSystemSe=function(name){return __host("se",name);},
    isExistFile=function(path){return __host("exists_file",path);},
    isSkip=function(){return __host("isskip");},
    isDebug=function(){return __host("isdebug");}
};
"#;
pub struct Screen {
    pub document: Rc<RefCell<Document>>,
    vm: Interpreter,
    pub live_count: usize,
    /// Parent of each transmitted layer, for cascade-aware removal.
    pub parents: BTreeMap<u32, Option<u32>>,
    pub widgets: std::collections::BTreeSet<u32>,
}
impl Screen {
    pub fn new(mut document: Document, source: &str, now: u64, entry: &str) -> Result<Self> {
        document.now = now;
        document.started = now;
        let document = Rc::new(RefCell::new(document));
        let host = document.clone();
        let mut vm = Interpreter::new(move |args| host.borrow_mut().request(args))?;
        vm.eval(HOST, 100_000)?;
        vm.eval(source, 500_000)?;
        vm.call(entry, 500_000)?;
        Ok(Self {
            document,
            vm,
            live_count: 0,
            parents: BTreeMap::new(),
            widgets: Default::default(),
        })
    }
    pub fn tick(&mut self, now: u64) -> Result<()> {
        let callbacks = self.document.borrow_mut().tick(now)?;
        for reference in callbacks {
            self.vm.callback(reference, 500_000)?;
        }
        Ok(())
    }
    pub fn input(&mut self, states: Vec<(String, Value)>) -> Result<()> {
        let mut activated = None;
        let mut hover = None;
        let mut focus = None;
        for (name, state) in states {
            if state["hovered"].as_bool() == Some(true) {
                hover = Some(name.clone());
            }
            if state["focused"].as_bool() == Some(true) {
                focus = Some(name.clone());
            }
            if state["activated"].as_bool() == Some(true) {
                activated = Some(name);
            }
        }
        let listeners = {
            let mut document = self.document.borrow_mut();
            let previous = document.hover.clone();
            document.hover = hover;
            document.hover_events(previous)
        };
        for f in listeners {
            self.vm.callback(f, 500_000)?;
        }
        {
            let mut document = self.document.borrow_mut();
            if let Some(focus) = focus {
                if !document.pending_focus {
                    document.focus = Some(focus);
                }
            }
        }
        if let Some(id) = activated {
            let name = serde_json::to_string(&id)?;
            self.vm.eval(
                &format!("if ({name} in getroottable()) getroottable()[{name}].onClick();"),
                500_000,
            )?;
        }
        Ok(())
    }
}
