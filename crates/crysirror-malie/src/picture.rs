// SPDX-License-Identifier: MPL-2.0
//! Malie picture documents. Every `src` in a layer/char/msgframe tag names an
//! SVG picture whose root `width`/`height` is the layer's logical quad; its
//! nested `scale()/translate()` groups place DZI images, solid rectangles and
//! `m:movie` streams. `name?key=value` substitutes `{$key}` in hrefs.
//! Exported animations reference `<defs>` images through `<use>` with
//! additive `animateTransform` (translate/rotate/scale) and `animate opacity`
//! key lists; they are evaluated per frame as exact affine matrices.
//! Resolution order follows the shipped directory layout (bg, event, effect,
//! chara/<who>/<pose>/<face>). Unknown names fail visibly at the tag PC.
use anyhow::{bail, ensure, Context, Result};
use quick_xml::{events::Event, Reader};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Paint {
    Image(String),
    Solid([u8; 4]),
    /// Asset path and whether the stream loops. Decoding WebM/VP8 is not yet
    /// a shared capability; the adapter records the gap and draws nothing.
    Movie(String, bool),
}
/// Affine [a, b, c, d, e, f]: x' = a*x + c*y + e, y' = b*x + d*y + f.
pub type Affine = [f32; 6];
pub const IDENTITY: Affine = [1., 0., 0., 1., 0., 0.];
pub fn mul(m: Affine, n: Affine) -> Affine {
    [
        m[0] * n[0] + m[2] * n[1],
        m[1] * n[0] + m[3] * n[1],
        m[0] * n[2] + m[2] * n[3],
        m[1] * n[2] + m[3] * n[3],
        m[0] * n[4] + m[2] * n[5] + m[4],
        m[1] * n[4] + m[3] * n[5] + m[5],
    ]
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MotionKind {
    Translate,
    Rotate,
    Scale,
    Opacity,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Motion {
    pub kind: MotionKind,
    pub times: Vec<f32>,
    pub values: Vec<[f32; 2]>,
    pub duration_ms: u64,
    pub repeat: bool,
}
impl Motion {
    fn sample(&self, t: u64) -> [f32; 2] {
        let d = self.duration_ms.max(1);
        let p = if self.repeat { (t % d) as f32 / d as f32 } else { (t.min(d)) as f32 / d as f32 };
        let i = self.times.partition_point(|k| *k <= p).clamp(1, self.times.len().max(2) - 1);
        let (t0, t1) = (self.times[i - 1], self.times[i]);
        let f = if t1 > t0 { ((p - t0) / (t1 - t0)).clamp(0., 1.) } else { 1. };
        let (a, b) = (self.values[i - 1], self.values[i]);
        [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
    }
    fn matrix(&self, t: u64) -> Affine {
        let v = self.sample(t);
        match self.kind {
            MotionKind::Translate => [1., 0., 0., 1., v[0], v[1]],
            MotionKind::Scale => [v[0], 0., 0., v[1], 0., 0.],
            MotionKind::Rotate => {
                let (s, c) = v[0].to_radians().sin_cos();
                [c, s, -s, c, 0., 0.]
            }
            MotionKind::Opacity => IDENTITY,
        }
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Element {
    /// Rectangle in picture units after the SVG group transforms.
    pub rect: [f32; 4],
    pub paint: Paint,
    pub opacity: f32,
    pub additive: bool,
    /// Animated elements draw the [0,0,w,h] source through `outer * motions
    /// * inner`, where `inner` places the referenced definition.
    #[serde(default)]
    pub animated: Option<Animated>,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Animated {
    pub size: [f32; 2],
    pub outer: Affine,
    pub inner: Affine,
    pub motions: Vec<Motion>,
}
impl Element {
    /// Local matrix and opacity at `t` ms after the picture was shown.
    pub fn sample(&self, t: u64) -> Option<(Affine, f32)> {
        let a = self.animated.as_ref()?;
        let mut m = a.outer;
        let mut opacity = self.opacity;
        for motion in &a.motions {
            if motion.kind == MotionKind::Opacity {
                opacity = motion.sample(t)[0];
            } else {
                m = mul(m, motion.matrix(t));
            }
        }
        Some((mul(m, a.inner), opacity.clamp(0., 1.)))
    }
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Picture {
    pub name: String,
    pub size: [f32; 2],
    pub elements: Vec<Element>,
}
impl Picture {
    pub fn movie(&self) -> Option<(&str, bool)> {
        self.elements.iter().find_map(|e| match &e.paint {
            Paint::Movie(path, looping) => Some((path.as_str(), *looping)),
            _ => None,
        })
    }
    pub fn animated(&self) -> bool {
        self.elements.iter().any(|e| e.animated.is_some())
    }
}
fn number(text: &str) -> Result<f32> {
    let v = text
        .trim()
        .trim_end_matches("px")
        .parse::<f32>()
        .with_context(|| format!("Malie picture number {text:?}"))?;
    ensure!(v.is_finite() && v.abs() <= 1.0e6, "Malie picture number range");
    Ok(v)
}
fn numbers(text: &str) -> Result<Vec<f32>> {
    text.split(|c: char| c == ',' || c.is_ascii_whitespace()).filter(|v| !v.is_empty()).map(number).collect()
}
fn color(text: &str) -> Result<[u8; 4]> {
    let t = text.trim().to_ascii_lowercase();
    Ok(match t.as_str() {
        "black" => [0, 0, 0, 255],
        "white" => [255, 255, 255, 255],
        "red" => [255, 0, 0, 255],
        "yellow" => [255, 255, 0, 255],
        "blue" => [0, 0, 255, 255],
        "green" => [0, 128, 0, 255],
        "none" | "transparent" => [0, 0, 0, 0],
        _ => {
            let hex = t.strip_prefix('#').context("Malie picture fill colour")?;
            ensure!(hex.len() == 6, "Malie picture fill colour {text:?}");
            let v = u32::from_str_radix(hex, 16)?;
            [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255]
        }
    })
}
/// SVG `transform` lists (scale, translate, rotate, matrix).
fn transform(text: &str) -> Result<Affine> {
    let mut m = IDENTITY;
    let mut rest = text.trim();
    while !rest.is_empty() {
        let open = rest.find('(').context("Malie picture transform syntax")?;
        let close = rest.find(')').context("Malie picture transform syntax")?;
        let name = rest[..open].trim();
        let args = numbers(&rest[open + 1..close])?;
        let n = match (name, args.as_slice()) {
            ("scale", [s]) => [*s, 0., 0., *s, 0., 0.],
            ("scale", [sx, sy]) => [*sx, 0., 0., *sy, 0., 0.],
            ("translate", [x]) => [1., 0., 0., 1., *x, 0.],
            ("translate", [x, y]) => [1., 0., 0., 1., *x, *y],
            ("rotate", [r]) => {
                let (s, c) = r.to_radians().sin_cos();
                [c, s, -s, c, 0., 0.]
            }
            ("matrix", [a, b, c, d, e, f]) => [*a, *b, *c, *d, *e, *f],
            _ => bail!("Malie picture transform {name}{args:?}"),
        };
        m = mul(m, n);
        rest = rest[close + 1..].trim_start();
    }
    Ok(m)
}
/// Public entry for SVG transform lists.
pub fn transform_affine(text: &str) -> Result<Affine> {
    transform(text)
}
/// SVG colour value (named or `#rrggbb`), RGBA8.
pub fn color_of(text: &str) -> Result<[u8; 4]> {
    color(text)
}
fn axis_aligned(m: &Affine) -> bool {
    m[1] == 0. && m[2] == 0. && m[0] > 0. && m[3] > 0.
}
struct Definition {
    rect: [f32; 4],
    transform: Affine,
    paint: Paint,
    opacity: f32,
    additive: bool,
}
struct PendingUse {
    target: String,
    outer: Affine,
    opacity: f32,
    motions: Vec<Motion>,
}
fn motion(kind: &str, attrs: &BTreeMap<String, String>) -> Result<Option<Motion>> {
    let kind = match (kind, attrs.get("type").map(String::as_str), attrs.get("attributeName").map(String::as_str)) {
        ("animateTransform", Some("translate"), _) => MotionKind::Translate,
        ("animateTransform", Some("rotate"), _) => MotionKind::Rotate,
        ("animateTransform", Some("scale"), _) => MotionKind::Scale,
        ("animate", _, Some("opacity")) => MotionKind::Opacity,
        _ => bail!("Malie picture animation {kind} {attrs:?}"),
    };
    let duration = number(attrs.get("dur").context("Malie animation duration")?)?;
    ensure!((0. ..=3600.).contains(&duration), "Malie animation duration range");
    let values: Vec<[f32; 2]> = attrs
        .get("values")
        .context("Malie animation values")?
        .split(';')
        .map(|v| {
            let n = numbers(v)?;
            Ok(match (kind, n.as_slice()) {
                (MotionKind::Scale, [s]) => [*s, *s],
                (_, [a]) => [*a, 0.],
                (_, [a, b, ..]) => [*a, *b],
                _ => bail!("Malie animation value {v:?}"),
            })
        })
        .collect::<Result<_>>()?;
    let times = match attrs.get("keyTimes") {
        Some(k) => numbers(&k.replace(';', " "))?,
        None => (0..values.len()).map(|i| i as f32 / (values.len().max(2) - 1) as f32).collect(),
    };
    ensure!(
        (2..=4096).contains(&values.len()) && times.len() == values.len() && times.windows(2).all(|w| w[0] <= w[1]),
        "Malie animation key layout"
    );
    Ok(Some(Motion {
        kind,
        times,
        values,
        duration_ms: (duration * 1000.) as u64,
        repeat: attrs.get("repeatCount").is_some_and(|v| v == "indefinite") || attrs.get("repeatDur").is_some_and(|v| v == "indefinite"),
    }))
}
/// Parse a picture document located at `dir/name.svg`.
pub fn parse(name: &str, dir: &str, text: &str, params: &BTreeMap<String, String>) -> Result<Picture> {
    ensure!(text.len() <= 256 * 1024, "Malie picture source budget");
    // Several shipped pictures carry a stray `//` comment line before <svg>.
    let start = text.find("<svg").context("Malie picture root")?;
    let mut reader = Reader::from_str(&text[start..]);
    reader.config_mut().check_end_names = false;
    let mut stack: Vec<Affine> = Vec::new();
    let mut size = None;
    let mut elements = Vec::new();
    let mut defs = 0usize;
    let mut definitions: BTreeMap<String, Definition> = BTreeMap::new();
    let mut pending: Option<PendingUse> = None;
    loop {
        let event = reader.read_event()?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let kind = String::from_utf8(e.name().as_ref().to_vec())?;
                let mut attrs = BTreeMap::new();
                for a in e.attributes().with_checks(false) {
                    let a = a?;
                    attrs.insert(
                        String::from_utf8(a.key.as_ref().to_vec())?,
                        a.decode_and_unescape_value(reader.decoder())?.into_owned(),
                    );
                }
                let outer = *stack.last().unwrap_or(&IDENTITY);
                let get = |k: &str, d: f32| attrs.get(k).map_or(Ok(d), |v| number(v));
                let rect = [get("x", 0.)?, get("y", 0.)?, get("width", 0.)?, get("height", 0.)?];
                let opacity = get("opacity", 1.)?;
                let additive = attrs.get("filter").is_some_and(|f| f.contains("Add"));
                let own = attrs.get("transform").map(|t| transform(t)).transpose()?.unwrap_or(IDENTITY);
                let paint = match kind.as_str() {
                    "image" => Some(Paint::Image(resolve(dir, attrs.get("xlink:href").context("Malie picture image href")?, params)?)),
                    "rect" => Some(Paint::Solid(color(attrs.get("fill").map_or("black", String::as_str))?)),
                    "m:movie" => {
                        let href = attrs.get("xlink:href").context("Malie picture movie href")?;
                        let looping = attrs.get("loop").is_some_and(|v| v.eq_ignore_ascii_case("true"));
                        Some(Paint::Movie(resolve(dir, href, params)?, looping))
                    }
                    _ => None,
                };
                match kind.as_str() {
                    "svg" => size = Some([get("width", 0.)?, get("height", 0.)?]),
                    "defs" | "filter" => defs += usize::from(!empty),
                    "image" | "rect" | "m:movie" if defs > 0 => {
                        if let Some(id) = attrs.get("id") {
                            ensure!(definitions.len() < 256, "Malie picture definition budget");
                            definitions.insert(id.clone(), Definition { rect, transform: own, paint: paint.unwrap(), opacity, additive });
                        }
                    }
                    _ if defs > 0 => {}
                    "image" | "rect" | "m:movie" => {
                        let m = mul(outer, own);
                        let paint = paint.unwrap();
                        if axis_aligned(&m) {
                            elements.push(Element {
                                rect: [m[4] + m[0] * rect[0], m[5] + m[3] * rect[1], m[0] * rect[2], m[3] * rect[3]],
                                paint,
                                opacity,
                                additive,
                                animated: None,
                            });
                        } else {
                            elements.push(Element {
                                rect,
                                paint,
                                opacity,
                                additive,
                                animated: Some(Animated { size: [rect[2], rect[3]], outer: m, inner: [1., 0., 0., 1., rect[0], rect[1]], motions: Vec::new() }),
                            });
                        }
                    }
                    "use" => {
                        let target = attrs.get("xlink:href").and_then(|h| h.strip_prefix('#')).context("Malie picture use reference")?;
                        let p = PendingUse { target: target.to_owned(), outer: mul(outer, own), opacity, motions: Vec::new() };
                        if empty {
                            pending = Some(p);
                            finish_use(&mut pending, &definitions, &mut elements)?;
                        } else {
                            ensure!(pending.is_none(), "Malie nested picture <use>");
                            pending = Some(p);
                        }
                    }
                    "animate" | "animateTransform" => {
                        let p = pending.as_mut().context("Malie picture animation outside <use>")?;
                        ensure!(p.motions.len() < 32, "Malie picture animation budget");
                        if let Some(m) = motion(&kind, &attrs)? {
                            p.motions.push(m);
                        }
                    }
                    "g" | "feBlend" => {}
                    other => bail!("Malie picture element <{other}> in {name}"),
                }
                if !empty && !matches!(kind.as_str(), "defs" | "filter" | "use" | "animate" | "animateTransform") {
                    let m = match kind.as_str() {
                        "g" if defs == 0 => mul(outer, own),
                        _ => outer,
                    };
                    ensure!(stack.len() < 64, "Malie picture nesting budget");
                    stack.push(m);
                }
            }
            Event::End(e) => match e.name().as_ref() {
                b"defs" | b"filter" => defs = defs.saturating_sub(1),
                b"use" => finish_use(&mut pending, &definitions, &mut elements)?,
                b"animate" | b"animateTransform" => {}
                _ => {
                    stack.pop();
                }
            },
            Event::Eof => break,
            _ => {}
        }
    }
    let size = size.context("Malie picture without <svg> extent")?;
    ensure!(size[0] > 0. && size[1] > 0., "Malie picture extent");
    ensure!(elements.len() <= 256, "Malie picture element budget");
    Ok(Picture { name: name.to_owned(), size, elements })
}
fn finish_use(pending: &mut Option<PendingUse>, defs: &BTreeMap<String, Definition>, out: &mut Vec<Element>) -> Result<()> {
    let p = pending.take().context("Malie picture </use> without <use>")?;
    let d = defs.get(&p.target).with_context(|| format!("Malie picture use target {:?}", p.target))?;
    out.push(Element {
        rect: d.rect,
        paint: d.paint.clone(),
        opacity: p.opacity * d.opacity,
        additive: d.additive,
        animated: Some(Animated {
            size: [d.rect[2], d.rect[3]],
            outer: p.outer,
            inner: mul(d.transform, [1., 0., 0., 1., d.rect[0], d.rect[1]]),
            motions: p.motions,
        }),
    });
    Ok(())
}
fn resolve(dir: &str, href: &str, params: &BTreeMap<String, String>) -> Result<String> {
    let mut href = href.to_owned();
    for (k, v) in params {
        href = href.replace(&format!("{{${k}}}"), v);
    }
    ensure!(!href.contains("{$"), "Malie picture parameter missing in {href:?}");
    let href = href.replace('\\', "/");
    let href = href.trim_start_matches("./");
    ensure!(
        !href.contains(':') && !href.starts_with('/') && !href.split('/').any(|v| v == ".."),
        "Malie picture href escapes its directory"
    );
    let path = format!("{dir}/{href}");
    Ok(libp_path(&if path.to_ascii_lowercase().ends_with(".png") {
        // Shipped pictures reference authoring PNGs that were published as
        // DZI tile manifests (the native loader applies the same rename).
        format!("{}.dzi", &path[..path.len() - 4])
    } else {
        path
    }))
}
/// LIBP entries store each path component in a 20-byte name field, so the
/// published name of a longer component is its first 20 bytes; native
/// lookups therefore resolve the truncated entry.
pub fn libp_path(path: &str) -> String {
    path.split('/').map(|c| if c.len() > 20 && c.is_ascii() { &c[..20] } else { c }).collect::<Vec<_>>().join("/")
}
/// Split `name?key=value&key2=value2`.
pub fn split_source(src: &str) -> Result<(String, BTreeMap<String, String>)> {
    let (name, query) = src.split_once('?').unwrap_or((src, ""));
    let mut params = BTreeMap::new();
    for pair in query.split('&').filter(|v| !v.is_empty()) {
        let (k, v) = pair.split_once('=').context("Malie picture parameter syntax")?;
        params.insert(k.trim().to_owned(), v.trim().to_owned());
    }
    ensure!(!name.is_empty() && name.len() <= 128, "Malie picture name");
    Ok((name.trim().to_ascii_lowercase(), params))
}
/// Candidate directories for a non-character picture, in lookup order.
pub const DIRECTORIES: [&str; 4] = ["data/picture/bg", "data/picture/event", "data/picture/effect", "data/picture"];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animated_use_composes_motion_and_definition() {
        let svg = "<svg width='10' height='10'><defs><image id='a' transform='scale(1 -1) translate(0 -4)' x='0' y='0' width='8' height='4' xlink:href='./a.png'/></defs>\
            <g transform='scale(0.5)'><use xlink:href='#a' opacity='0'>\
            <animateTransform attributeName='transform' type='translate' dur='2' keyTimes='0;1' values='0,0;10,20' additive='sum'/>\
            <animate attributeName='opacity' dur='2' keyTimes='0;1' values='1;1'/></use></g></svg>";
        let p = parse("t", "d", svg, &BTreeMap::new()).unwrap();
        let (m, o) = p.elements[0].sample(1000).unwrap();
        assert_eq!(o, 1.);
        // scale(0.5) * translate(5,10) * flip: source (0,0) maps to (2.5, 5+2).
        assert_eq!([m[4], m[5]], [2.5, 7.]);
        assert_eq!(m[3], -0.5);
        assert_eq!(p.elements[0].paint, Paint::Image("d/a.dzi".into()));
    }
}
