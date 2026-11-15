//! KRKR2 Pad text-editor state and windowEx close notifications.
//! Reference: krkr2 dec49af97e174d31059c3ccd7efc700ba3c6b788,
//! 2.32stable PadIntf.cpp, PadImpl.cpp, PadFormUnit.dfm, windowEx/main.cpp.
//! W.Dee/contributors' native contracts are translated to a shared UI editor
//! viewport; this is not a VCL/Win32 child window or executable plug-in host.
use crate::natives::*;
use crysirror_core::{
    service::{EngineQuery, HostServices, InputFrame},
    state::ClientId,
};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{ObjectHandle, Variant},
};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};
#[derive(Default)]
pub(crate) struct State {
    objects: BTreeMap<usize, Pad>,
    next: u32,
    last_poll: Option<u64>,
    pub closes: VecDeque<(ObjectHandle, bool)>,
}
struct Pad {
    values: BTreeMap<&'static str, Variant>,
    widget: u32,
    dirty: bool,
    published: bool,
    ex_events: bool,
    close_queued: bool,
    scroll: u32,
    focused: bool,
    status_present: bool,
    scroll_present: bool,
}
const STRINGS: &[&str] = &["text", "fileName", "title", "fontFace", "statusText"];
const BOOLS: &[&str] = &[
    "visible",
    "readOnly",
    "wordWrap",
    "showStatusBar",
    "fontBold",
    "fontItalic",
    "fontUnderline",
    "fontStrikeOut",
];
const NUMBERS: &[&str] = &[
    "color",
    "fontColor",
    "fontHeight",
    "fontSize",
    "opacity",
    "showScrollBars",
    "borderStyle",
    "width",
    "height",
    "top",
    "left",
];
impl State {
    pub(crate) fn roots(&self, out: &mut Vec<ObjectHandle>) {
        out.extend(self.closes.iter().map(|v| v.0));
    }
    pub(crate) fn remove(&mut self, h: ObjectHandle) -> Option<u32> {
        self.closes.retain(|v| v.0 != h);
        self.objects
            .remove(&h.0)
            .filter(|p| p.published)
            .map(|p| p.widget)
    }
}
fn value<'a>(rt: &'a Rt, h: ObjectHandle, key: &str) -> Result<&'a Variant> {
    rt.host()
        .pads
        .objects
        .get(&h.0)
        .and_then(|p| p.values.get(key))
        .ok_or_else(|| TjsError::runtime("Pad is not constructed"))
}
fn number(p: &Pad, key: &str) -> i64 {
    p.values[key].to_integer().unwrap_or(0)
}
fn text(p: &Pad, key: &str) -> String {
    p.values[key].to_tjs_string().unwrap_or_default()
}
pub(crate) fn register_ex(rt: &mut Rt, t: Option<ObjectHandle>) -> Result<Variant> {
    let h = this_of(rt, t, "Pad.registerExEvent")?;
    let p = rt
        .host_mut()
        .pads
        .objects
        .get_mut(&h.0)
        .ok_or_else(|| TjsError::runtime("Pad.registerExEvent requires a constructed Pad"))?;
    p.ex_events = true;
    Ok(Variant::Void)
}
pub(crate) fn install(rt: &mut Rt) {
    let c = native_class(rt, "Pad", |rt, t, _| {
        let h = this_of(rt, t, "Pad")?;
        if rt.host().pads.objects.contains_key(&h.0) {
            return Ok(Variant::Void);
        }
        let state = &mut rt.host_mut().pads;
        if state.objects.len() >= 64 {
            return Err(TjsError::runtime("Pad window limit"));
        }
        state.next = state
            .next
            .checked_add(1)
            .filter(|v| *v < 65536)
            .ok_or_else(|| TjsError::runtime("Pad identifier limit"))?;
        let mut values = BTreeMap::new();
        for key in STRINGS {
            values.insert(
                *key,
                Variant::String(if *key == "title" { "Pad" } else { "" }.into()),
            );
        }
        for key in BOOLS {
            values.insert(
                *key,
                Variant::Integer(matches!(*key, "wordWrap" | "showStatusBar") as i64),
            );
        }
        for (key, v) in [
            ("color", 0xffffff),
            ("fontColor", 0),
            ("fontHeight", -12),
            ("fontSize", 9),
            ("opacity", 255),
            ("showScrollBars", 3),
            ("borderStyle", 2),
            ("width", 538),
            ("height", 352),
            ("top", 278),
            ("left", 74),
        ] {
            values.insert(key, Variant::Integer(v));
        }
        let widget = 0x6a000000 + state.next * 8;
        state.objects.insert(
            h.0,
            Pad {
                values,
                widget,
                dirty: true,
                published: false,
                ex_events: false,
                close_queued: false,
                scroll: 0,
                focused: false,
                status_present: false,
                scroll_present: false,
            },
        );
        Ok(Variant::Void)
    });
    methods(
        rt,
        c,
        &[
            ("finalize", |_, _, _| Ok(Variant::Void)),
            ("__crysirror_pad_close", |rt, t, _| {
                let h = this_of(rt, t, "Pad close")?;
                if let Some(p) = rt.host_mut().pads.objects.get_mut(&h.0) {
                    p.values.insert("visible", Variant::Integer(0));
                    p.close_queued = false;
                    p.dirty = true;
                }
                Ok(Variant::Void)
            }),
        ],
    );
    for key in STRINGS.iter().chain(BOOLS).chain(NUMBERS) {
        let key = *key;
        rt.register_object_native_property(
            c,
            key,
            move |rt: &mut Rt, t| {
                let h = this_of(rt, t, "Pad property")?;
                Ok(value(rt, h, key)?.clone())
            },
            move |rt: &mut Rt, t, v| {
                let h = this_of(rt, t, "Pad property")?;
                let v = if STRINGS.contains(&key) {
                    let s = v.to_tjs_string()?;
                    if s.chars().count() > 65536 {
                        return Err(TjsError::runtime("Pad text length limit"));
                    }
                    Variant::String(s)
                } else if BOOLS.contains(&key) {
                    if key.starts_with("font") && v.is_truthy() {
                        return Err(unsupported(
                            &format!("Pad.{key}"),
                            "styled editor text presentation",
                        ));
                    }
                    Variant::Integer(v.is_truthy() as i64)
                } else {
                    let n = v.to_integer()?;
                    if matches!(key, "width" | "height") && !(70..=8192).contains(&n)
                        || key == "opacity" && !(0..=255).contains(&n)
                        || key == "showScrollBars" && !(0..=3).contains(&n)
                        || matches!(key, "fontHeight" | "fontSize")
                            && (!(1..=512).contains(&n.unsigned_abs()))
                    {
                        return Err(TjsError::runtime("Pad property out of range"));
                    }
                    Variant::Integer(n)
                };
                let p = rt
                    .host_mut()
                    .pads
                    .objects
                    .get_mut(&h.0)
                    .ok_or_else(|| TjsError::runtime("Pad is not constructed"))?;
                if key == "fontSize" {
                    p.values.insert(
                        "fontHeight",
                        Variant::Integer(-(v.to_integer()?.unsigned_abs() as i64 * 4 / 3).max(1)),
                    );
                }
                if key == "fontHeight" {
                    p.values.insert(
                        "fontSize",
                        Variant::Integer((v.to_integer()?.unsigned_abs() as i64 * 3 / 4).max(1)),
                    );
                }
                p.values.insert(key, v);
                p.dirty = true;
                Ok(())
            },
        );
    }
}
pub(crate) fn sync(rt: &mut Rt) -> Result<()> {
    let ids: Vec<_> = rt.host().pads.objects.keys().copied().collect();
    for id in ids {
        let p = &rt.host().pads.objects[&id];
        if !p.dirty {
            continue;
        }
        let widget = p.widget;
        let status_present=p.status_present;
        let scroll_present=p.scroll_present;
        let visible = p.values["visible"].is_truthy();
        if !visible {
            if p.published {
                rt.host_mut()
                    .ui_op(json!({"op":"set_visible","id":widget,"visible":false}));
            }
            rt.host_mut().pads.objects.get_mut(&id).unwrap().dirty = false;
            continue;
        }
        let [cw, ch] = rt.host().canvas.map(i64::from);
        let width = number(p, "width").min(cw);
        let height = number(p, "height").min(ch);
        let left = number(p, "left").clamp(0, cw - width);
        let top = number(p, "top").clamp(0, ch - height);
        let opacity = number(p, "opacity") as u8;
        // Pad's colour properties are VCL TColor (BGR), not Layer's ARGB.
        let color = |n: i64| [n as u8, (n >> 8) as u8, (n >> 16) as u8, opacity];
        let title = text(p, "title");
        let body = text(p, "text");
        let status = text(p, "statusText");
        let status_height = if p.values["showStatusBar"].is_truthy() {
            20
        } else {
            0
        };
        let scroll_visible = number(p, "showScrollBars") & 2 != 0;
        let sw = if scroll_visible { 16 } else { 0 };
        let readonly = p.values["readOnly"].is_truthy();
        let no_wrap = !p.values["wordWrap"].is_truthy();
        let scroll = p.scroll;
        let bg = color(number(p, "color"));
        let fg = color(number(p, "fontColor"));
        let size = number(p, "fontHeight").unsigned_abs().clamp(1, 512);
        let face = text(p, "fontFace");
        let font = if face.is_empty() {
            rt.host().font.clone()
        } else {
            rt.host()
                .fonts
                .library
                .as_ref()
                .and_then(|lib| lib.resolve_alias(&face).ok().flatten())
                .unwrap_or_else(|| rt.host().font.clone())
        };
        let root = json!({"op":"put","widget":{"id":widget,"rect":[left,top,width,height],"color":[220,220,220,opacity],"visible":true,"clip_children":true,"z":10000}});
        let header = json!({"op":"put","widget":{"id":widget+1,"parent":widget,"kind":"label","rect":[0,0,width-24,24],"text":title,"font":font,"text_color":[0,0,0,opacity],"color":[220,220,220,opacity],"text_size":12,"z":1}});
        let close = json!({"op":"put","widget":{"id":widget+2,"parent":widget,"kind":"button","rect":[width-24,0,24,24],"text":"X","font":font,"text_color":[0,0,0,opacity],"color":[200,200,200,opacity],"text_size":12,"z":3}});
        let editor = json!({"op":"put","widget":{"id":widget+3,"parent":widget,"kind":"text_field","rect":[0,24,width-sw,height-24-status_height],"font":font,"text":body,"text_color":fg,"color":bg,"text_size":size,"max_length":65536,"multiline":true,"read_only":readonly,"no_wrap":no_wrap,"scroll_lines":scroll,"keyboard_navigation":false,"keyboard_activation":false,"z":1}});
        for op in [root, header, close, editor] {
            rt.host_mut().ui_op(op);
        }
        if status_height != 0 {
            rt.host_mut().ui_op(json!({"op":"put","widget":{"id":widget+4,"parent":widget,"kind":"label","rect":[0,height-20,width,20],"font":font,"text":status,"text_color":[0,0,0,opacity],"color":[220,220,220,opacity],"text_size":12,"z":2}}));
        } else if status_present {
            rt.host_mut().ui_op(json!({"op":"remove","id":widget+4}));
        }
        if scroll_visible {
            let lines = body.lines().count().max(1);
            rt.host_mut().ui_op(json!({"op":"put","widget":{"id":widget+5,"parent":widget,"kind":"slider","slider_axis":"vertical","rect":[width-16,24,16,height-24-status_height],"value":(scroll as f32/lines as f32).clamp(0.,1.),"keyboard_navigation":false,"keyboard_activation":false,"z":2}}));
        } else if scroll_present {
            rt.host_mut().ui_op(json!({"op":"remove","id":widget+5}));
        }
        let p = rt.host_mut().pads.objects.get_mut(&id).unwrap();
        p.dirty = false;
        p.published = true;
        p.status_present=status_height!=0;
        p.scroll_present=scroll_visible;
    }
    Ok(())
}
pub(crate) fn poll(rt: &mut Rt, host: &mut dyn HostServices, client: ClientId) -> anyhow::Result<()> {
    if rt.host().pads.last_poll == Some(rt.host().now) {
        return Ok(());
    }
    rt.host_mut().pads.last_poll = Some(rt.host().now);
    let ids: Vec<_> = rt
        .host()
        .pads
        .objects
        .iter()
        .filter(|(_, p)| p.published && p.values["visible"].is_truthy())
        .map(|(id, p)| (*id, p.widget))
        .collect();
    for (id, widget) in ids {
        let editor: Option<crysirror_scene::ui::WidgetSpec> =
            serde_json::from_slice(&host.query_engine(client, EngineQuery::UiWidget(widget + 3))?)?;
        if let Some(editor) = editor {
            rt.host_mut()
                .pads
                .objects
                .get_mut(&id)
                .unwrap()
                .values
                .insert("text", Variant::String(editor.text));
        }
        let close: crysirror_scene::ui::WidgetInteraction = serde_json::from_slice(
            &host.query_engine(client, EngineQuery::UiInteraction(widget + 2))?,
        )?;
        let interaction: crysirror_scene::ui::WidgetInteraction = serde_json::from_slice(
            &host.query_engine(client, EngineQuery::UiInteraction(widget + 3))?,
        )?;
        let scroll = if number(&rt.host().pads.objects[&id], "showScrollBars") & 2 != 0 {
            let interaction: crysirror_scene::ui::WidgetInteraction = serde_json::from_slice(
                &host.query_engine(client, EngineQuery::UiInteraction(widget + 5))?,
            )?;
            let spec: Option<crysirror_scene::ui::WidgetSpec> = serde_json::from_slice(
                &host.query_engine(client, EngineQuery::UiWidget(widget + 5))?,
            )?;
            spec.filter(|_| interaction.pressed || interaction.activated)
                .map(|s| s.value)
        } else {
            None
        };
        let p = rt.host_mut().pads.objects.get_mut(&id).unwrap();
        p.focused = interaction.focused;
        if let Some(scroll) = scroll {
            p.scroll = (scroll
                * p.values["text"]
                    .to_tjs_string()
                    .unwrap_or_default()
                    .lines()
                    .count() as f32) as u32;
            p.dirty = true;
        }
        if close.activated && !p.close_queued {
            p.close_queued = true;
            let notify = p.ex_events;
            rt.host_mut()
                .pads
                .closes
                .push_back((ObjectHandle(id), notify));
        }
    }
    Ok(())
}
pub(crate) fn filter_input(rt: &Rt, mut input: InputFrame) -> InputFrame {
    if !rt.host().modal_windows.is_empty() {
        return input;
    }
    for p in rt
        .host()
        .pads
        .objects
        .values()
        .filter(|p| p.published && p.values["visible"].is_truthy())
    {
        let [cw, ch] = rt.host().canvas.map(i64::from);
        let w = number(p, "width").min(cw);
        let h = number(p, "height").min(ch);
        let x = number(p, "left").clamp(0, cw - w);
        let y = number(p, "top").clamp(0, ch - h);
        if p.focused
            || (x..x + w).contains(&i64::from(input.pointer_x))
                && (y..y + h).contains(&i64::from(input.pointer_y))
        {
            input.buttons_down = 0;
            input.buttons_pressed = 0;
            input.pointer_x = i32::MIN;
            input.pointer_y = i32::MIN;
            return input;
        }
    }
    input
}
