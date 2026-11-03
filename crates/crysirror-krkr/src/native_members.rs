// Copyright (c) 2026 CrySirror contributors.
// Distributed under the GNU Affero General Public License, version 3 or
// (at your option) any later version, https://www.gnu.org/licenses/agpl-3.0.html.
//! Native Layer/Window/System/WaveSoundBuffer members that an inventory
//! against krkrz (fd5c4baa6a2ef5978db1bd043634351f48667daf, `visual/LayerIntf.cpp`,
//! `visual/LayerManager.cpp`, `visual/WindowIntf.cpp`, `base/SystemIntf.cpp`,
//! `sound/win32/WaveImpl.cpp`) found unregistered. Each follows the original
//! semantics over existing engine-owned adapter state; members whose
//! original effect needs a platform capability CrySirror lacks fail visibly.
use crate::natives::{
    Rt, focused_layer, has_member, layer_node_enabled, methods, need_int, object_arg, properties,
    read_only, set_focus, this_of, unsupported,
};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{ObjectHandle, Variant},
};
use serde_json::json;

const TVP_SS_SHIFT: i64 = 1;
const TVP_SS_ALT: i64 = 2;
const TVP_SS_CTRL: i64 = 4;
const VK_TAB: i64 = 9;
const VK_RETURN: i64 = 13;
const VK_ESCAPE: i64 = 27;
const VK_LEFT: i64 = 37;
const VK_UP: i64 = 38;
const VK_RIGHT: i64 = 39;
const VK_DOWN: i64 = 40;

pub fn install(rt: &mut Rt) {
    install_layer(rt);
    install_window(rt);
    install_system(rt);
    install_wave(rt);
}

fn layer_arg(rt: &Rt, a: &[Variant], i: usize, member: &str) -> Result<ObjectHandle> {
    let h = a
        .get(i)
        .and_then(object_arg)
        .map(|h| rt.bound_this(h).unwrap_or(h))
        .ok_or_else(|| TjsError::runtime(format!("{member}: specify a Layer")))?;
    rt.host().layer(h)?;
    Ok(h)
}

// ------------------------------------------------------------- z-order

/// krkrz CheckZOrderMoveRule + MoveBefore/MoveBehind (LayerIntf.cpp:1109-1216):
/// leave absolute order mode, then move next to a sibling. "Before" is in
/// front of `lay` (a later painter index), "behind" is directly under it.
fn move_next_to(rt: &mut Rt, t: Option<ObjectHandle>, a: Vec<Variant>, before: bool) -> Result<Variant> {
    let member = if before { "Layer.moveBefore" } else { "Layer.moveBehind" };
    let h = this_of(rt, t, member)?;
    let lay = layer_arg(rt, &a, 0, member)?;
    let parent = rt
        .host()
        .layer(h)?
        .parent
        .ok_or_else(|| TjsError::runtime("Cannot move the primary layer or a layer without siblings"))?;
    rt.host_mut().set_order_mode(ObjectHandle(parent), false)?;
    let mut order = rt.host().sorted_children(parent);
    if order.len() <= 1 {
        return Err(TjsError::runtime("Cannot move the primary layer or a layer without siblings"));
    }
    if lay == h || rt.host().layer(lay)?.parent != Some(parent) {
        return Err(TjsError::runtime("Cannot move next to self or a non-sibling layer"));
    }
    let this_order = order.iter().position(|c| *c == h.0).unwrap_or(0);
    let lay_order = order.iter().position(|c| *c == lay.0).unwrap_or(0);
    // ChildChangeOrder(from, to): remove `from`, insert at `to`.
    let to = match (before, this_order < lay_order) {
        (true, true) => lay_order,
        (true, false) => lay_order + 1,
        (false, true) => lay_order - 1,
        (false, false) => lay_order,
    };
    let id = order.remove(this_order);
    order.insert(to.min(order.len()), id);
    rt.host_mut().layer_mut(ObjectHandle(parent))?.children = order;
    rt.host_mut().layer_mut(h)?.dirty = true;
    Ok(Variant::Void)
}

/// krkrz BringToBack (LayerIntf.cpp:1233): order index 0, relative mode.
fn bring_to_back(rt: &mut Rt, t: Option<ObjectHandle>, _: Vec<Variant>) -> Result<Variant> {
    let h = this_of(rt, t, "Layer.bringToBack")?;
    let parent = rt
        .host()
        .layer(h)?
        .parent
        .ok_or_else(|| TjsError::runtime("Cannot move the primary layer or a layer without siblings"))?;
    rt.host_mut().set_order_mode(ObjectHandle(parent), false)?;
    let mut order = rt.host().sorted_children(parent);
    order.retain(|c| *c != h.0);
    order.insert(0, h.0);
    rt.host_mut().layer_mut(ObjectHandle(parent))?.children = order;
    rt.host_mut().layer_mut(h)?.dirty = true;
    Ok(Variant::Void)
}

// ------------------------------------------------------------- focus chain

fn primary_of(rt: &Rt, h: ObjectHandle) -> Option<ObjectHandle> {
    let window = rt.host().layer(h).ok()?.window;
    rt.host().windows.get(&window)?.primary.map(ObjectHandle)
}

/// krkrz GetAllNodes: the overall (pre-order) node list from the primary.
fn all_nodes(rt: &Rt, primary: ObjectHandle) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    let mut stack = vec![primary.0];
    while let Some(id) = stack.pop() {
        out.push(id);
        if out.len() > 65536 {
            return Err(TjsError::runtime("layer focus search limit"));
        }
        stack.extend(rt.host().sorted_children(id).into_iter().rev());
    }
    Ok(out)
}

/// krkrz GetNodeFocusable (LayerIntf.h:622-625): Focusable && Visible &&
/// Enabled, every parent visible and enabled, and not disabled by a modal layer.
pub(crate) fn node_focusable(rt: &Rt, h: ObjectHandle) -> Result<bool> {
    let l = rt.host().layer(h)?;
    if !(rt.object_member(h, "__lp_focusable").is_truthy()
        && l.visible
        && rt.object_member(h, "__lp_enabled").is_truthy())
    {
        return Ok(false);
    }
    let mut node = l.parent;
    let mut depth = 0;
    while let Some(p) = node {
        let pl = rt.host().layer(ObjectHandle(p))?;
        if !pl.visible || !rt.object_member(ObjectHandle(p), "__lp_enabled").is_truthy() {
            return Ok(false);
        }
        node = pl.parent;
        depth += 1;
        if depth > 64 {
            return Err(TjsError::runtime("layer focus ancestry limit"));
        }
    }
    Ok(!rt.host().disabled_by_mode(h.0))
}

fn chain_focusable(rt: &Rt, h: ObjectHandle) -> Result<bool> {
    Ok(node_focusable(rt, h)? && rt.object_member(h, "__lp_joinFocusChain").is_truthy())
}

/// krkrz _GetNextFocusable/_GetPrevFocusable: walk the looped overall order.
fn neighbor_focusable(rt: &Rt, h: ObjectHandle, forward: bool) -> Result<Option<ObjectHandle>> {
    let Some(primary) = primary_of(rt, h) else { return Ok(None) };
    let nodes = all_nodes(rt, primary)?;
    let Some(index) = nodes.iter().position(|n| *n == h.0) else { return Ok(None) };
    let n = nodes.len();
    for step in 1..n {
        let i = if forward { (index + step) % n } else { (index + n - step) % n };
        let candidate = ObjectHandle(nodes[i]);
        if chain_focusable(rt, candidate)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn layer_or_null(h: Option<ObjectHandle>) -> Variant {
    h.map_or(Variant::Null, Variant::self_bound)
}

fn focus_work(rt: &Rt, h: ObjectHandle) -> Option<ObjectHandle> {
    rt.object_member(h, "__focusWork")
        .object_handle()
        .filter(|w| rt.object_valid(*w) && rt.host().layer(*w).is_ok())
}

/// krkrz GetNextFocusable/GetPrevFocusable: the found layer is offered to
/// onSearchNext/PrevFocusable, whose native default writes FocusWork back.
pub(crate) fn search_focusable(rt: &mut Rt, h: ObjectHandle, forward: bool) -> Result<Option<ObjectHandle>> {
    let found = neighbor_focusable(rt, h, forward)?;
    rt.set_object_member(h, "__focusWork", layer_or_null(found));
    let event = if forward { "onSearchNextFocusable" } else { "onSearchPrevFocusable" };
    if rt.object_valid(h) && has_member(rt, h, event) {
        rt.call_object_method(h, event, vec![layer_or_null(found)])?;
    }
    Ok(focus_work(rt, h))
}

/// krkrz tTVPLayerManager::SetFocusTo (LayerManager.cpp:760): onBeforeFocus
/// may redirect the target before onBlur/onFocus run.
pub(crate) fn set_focus_to(rt: &mut Rt, window: ObjectHandle, layer: Option<ObjectHandle>, forward: bool) -> Result<bool> {
    let mut layer = layer;
    if let Some(l) = layer {
        if !node_focusable(rt, l)? {
            return Ok(false);
        }
        rt.set_object_member(l, "__focusWork", Variant::self_bound(l));
        if has_member(rt, l, "onBeforeFocus") {
            let blurred = layer_or_null(focused_layer(rt, window));
            rt.call_object_method(
                l,
                "onBeforeFocus",
                vec![Variant::self_bound(l), blurred, Variant::Integer(forward as i64)],
            )?;
        }
        layer = focus_work(rt, l);
    }
    if let Some(l) = layer
        && !node_focusable(rt, l)?
    {
        return Ok(false);
    }
    if focused_layer(rt, window) == layer {
        return Ok(false);
    }
    set_focus(rt, window, layer, forward)?;
    Ok(true)
}

/// krkrz tTVPLayerManager::FocusNext/FocusPrev (LayerManager.cpp:724-747).
pub(crate) fn focus_step(rt: &mut Rt, window: ObjectHandle, forward: bool) -> Result<Option<ObjectHandle>> {
    let target = match focused_layer(rt, window) {
        Some(f) => search_focusable(rt, f, forward)?,
        None => {
            let Some(primary) = rt.host().windows.get(&window.0).and_then(|w| w.primary) else {
                return Ok(None);
            };
            let mut found = None;
            for id in all_nodes(rt, ObjectHandle(primary))? {
                if chain_focusable(rt, ObjectHandle(id))? {
                    found = Some(ObjectHandle(id));
                    break;
                }
            }
            found
        }
    };
    if let Some(t) = target {
        set_focus_to(rt, window, Some(t), forward)?;
    }
    Ok(target)
}

fn window_of_layer(rt: &Rt, h: ObjectHandle) -> Result<ObjectHandle> {
    Ok(ObjectHandle(rt.host().layer(h)?.window))
}

// ------------------------------------------------------------- events

/// TVP_ACTION_INVOKE_BEGIN/MEMBER/END (base/EventIntf.h:195-216): build the
/// event dictionary and call the owner's `action`, when it has one.
fn action(rt: &mut Rt, owner: ObjectHandle, target: ObjectHandle, event: &str, fields: &[&str], a: &[Variant]) -> Result<()> {
    if a.len() < fields.len() {
        return Err(TjsError::bad_param_count());
    }
    if !rt.object_valid(owner) || !has_member(rt, owner, "action") {
        return Ok(());
    }
    let ev = rt.alloc_dictionary_object();
    rt.set_object_member(ev, "type", Variant::String(event.into()));
    rt.set_object_member(ev, "target", Variant::self_bound(target));
    for (name, value) in fields.iter().zip(a) {
        rt.set_object_member(ev, *name, value.clone());
    }
    rt.call_object_method(owner, "action", vec![Variant::self_bound(ev)])?;
    Ok(())
}

fn layer_action(rt: &mut Rt, t: Option<ObjectHandle>, event: &str, fields: &[&str], a: &[Variant]) -> Result<ObjectHandle> {
    let h = this_of(rt, t, event)?;
    let owner = window_of_layer(rt, h)?;
    action(rt, owner, h, event, fields, a)?;
    Ok(h)
}

fn no_shift(shift: i64) -> bool {
    shift & (TVP_SS_SHIFT | TVP_SS_ALT | TVP_SS_CTRL) == 0
}

/// FireKeyDown/Up/Press on the parent when it is node-enabled.
fn pass_to_parent(rt: &mut Rt, h: ObjectHandle, event: &'static str, args: Vec<Variant>) -> Result<()> {
    if let Some(p) = rt.host().layer(h)?.parent.map(ObjectHandle)
        && layer_node_enabled(rt, p)?
        && has_member(rt, p, event)
    {
        rt.call_object_method(p, event, args)?;
    }
    Ok(())
}

/// krkrz tTJSNI_BaseLayer::DefaultKeyDown (LayerIntf.cpp:3553).
fn default_key_down(rt: &mut Rt, h: ObjectHandle, key: i64, shift: i64) -> Result<()> {
    let window = window_of_layer(rt, h)?;
    if matches!(key, VK_TAB | VK_RIGHT | VK_DOWN) && no_shift(shift) {
        focus_step(rt, window, true)?;
    } else if (key == VK_TAB && shift & TVP_SS_SHIFT != 0 && shift & (TVP_SS_ALT | TVP_SS_CTRL) == 0)
        || matches!(key, VK_LEFT | VK_UP)
    {
        focus_step(rt, window, false)?;
    } else if matches!(key, VK_RETURN | VK_ESCAPE) && no_shift(shift) {
        pass_to_parent(rt, h, "onKeyDown", vec![Variant::Integer(key), Variant::Integer(shift), Variant::Integer(1)])?;
    }
    Ok(())
}

fn default_key_up(rt: &mut Rt, h: ObjectHandle, key: i64, shift: i64) -> Result<()> {
    if matches!(key, VK_RETURN | VK_ESCAPE) && no_shift(shift) {
        pass_to_parent(rt, h, "onKeyUp", vec![Variant::Integer(key), Variant::Integer(shift), Variant::Integer(1)])?;
    }
    Ok(())
}

fn default_key_press(rt: &mut Rt, h: ObjectHandle, key: &Variant) -> Result<()> {
    let text = key.to_tjs_string()?;
    if matches!(text.chars().next().map(u32::from), Some(13 | 0x1b)) {
        pass_to_parent(rt, h, "onKeyPress", vec![key.clone(), Variant::Integer(1)])?;
    }
    Ok(())
}

fn process(a: &[Variant], i: usize) -> bool {
    a.get(i).is_none_or(Variant::is_truthy)
}

/// Window input delivered to the layer tree (krkrz tTVPLayerManager::
/// PrimaryKeyDown/Up/Press, LayerManager.cpp:1012-1033): the focused layer
/// receives the event, otherwise the primary layer's default behaviour runs.
pub(crate) fn primary_key(rt: &mut Rt, window: ObjectHandle, event: &'static str, key: i64, shift: i64) -> Result<()> {
    if let Some(f) = focused_layer(rt, window) {
        if has_member(rt, f, event) {
            rt.call_object_method(f, event, vec![Variant::Integer(key), Variant::Integer(shift), Variant::Integer(1)])?;
        }
        return Ok(());
    }
    let Some(primary) = rt.host().windows.get(&window.0).and_then(|w| w.primary).map(ObjectHandle) else {
        return Ok(());
    };
    match event {
        "onKeyDown" => default_key_down(rt, primary, key, shift),
        _ => default_key_up(rt, primary, key, shift),
    }
}

/// krkrz PrimaryMouseWheel: only a focused layer receives the wheel.
pub(crate) fn primary_wheel(rt: &mut Rt, window: ObjectHandle, shift: i64, delta: i64, x: i64, y: i64) -> Result<()> {
    if let Some(f) = focused_layer(rt, window)
        && has_member(rt, f, "onMouseWheel")
    {
        rt.call_object_method(f, "onMouseWheel", [shift, delta, x, y].map(Variant::Integer).to_vec())?;
    }
    Ok(())
}

fn install_layer(rt: &mut Rt) {
    let Some(class) = rt.global_member("Layer").object_handle() else { return };
    methods(
        rt,
        class,
        &[
            ("moveBefore", |rt, t, a| move_next_to(rt, t, a, true)),
            ("moveBehind", |rt, t, a| move_next_to(rt, t, a, false)),
            ("bringToBack", bring_to_back),
            // krkrz Layer.focus (LayerIntf.cpp:7705) is SetFocus(direction):
            // node-focusable check plus onBeforeFocus redirection.
            ("focus", |rt, t, a| {
                let h = this_of(rt, t, "Layer.focus")?;
                let window = window_of_layer(rt, h)?;
                let forward = a.first().is_none_or(Variant::is_truthy);
                Ok(Variant::Integer(set_focus_to(rt, window, Some(h), forward)? as i64))
            }),
            ("focusNext", |rt, t, _| {
                let h = this_of(rt, t, "Layer.focusNext")?;
                let window = window_of_layer(rt, h)?;
                Ok(layer_or_null(focus_step(rt, window, true)?))
            }),
            ("focusPrev", |rt, t, _| {
                let h = this_of(rt, t, "Layer.focusPrev")?;
                let window = window_of_layer(rt, h)?;
                Ok(layer_or_null(focus_step(rt, window, false)?))
            }),
            // Native event defaults: action dispatch, then the default
            // behaviour when `process` is true (LayerIntf.cpp:7945-8440).
            ("onKeyDown", |rt, t, a| {
                let h = layer_action(rt, t, "onKeyDown", &["key", "shift", "process"], &a)?;
                if process(&a, 2) {
                    default_key_down(rt, h, a[0].to_integer()?, a[1].to_integer()?)?;
                }
                Ok(Variant::Void)
            }),
            ("onKeyUp", |rt, t, a| {
                let h = layer_action(rt, t, "onKeyUp", &["key", "shift", "process"], &a)?;
                if process(&a, 2) {
                    default_key_up(rt, h, a[0].to_integer()?, a[1].to_integer()?)?;
                }
                Ok(Variant::Void)
            }),
            ("onKeyPress", |rt, t, a| {
                let h = layer_action(rt, t, "onKeyPress", &["key", "process"], &a)?;
                if process(&a, 1) {
                    default_key_press(rt, h, &a[0])?;
                }
                Ok(Variant::Void)
            }),
            ("onMouseWheel", |rt, t, a| {
                layer_action(rt, t, "onMouseWheel", &["shift", "delta", "x", "y"], &a)?;
                Ok(Variant::Void)
            }),
            ("onDoubleClick", |rt, t, a| {
                layer_action(rt, t, "onDoubleClick", &["x", "y"], &a)?;
                Ok(Variant::Void)
            }),
            ("onTouchDown", |rt, t, a| {
                layer_action(rt, t, "onTouchDown", &["x", "y", "cx", "cy", "id"], &a)?;
                Ok(Variant::Void)
            }),
            ("onTouchUp", |rt, t, a| {
                layer_action(rt, t, "onTouchUp", &["x", "y", "cx", "cy", "id"], &a)?;
                Ok(Variant::Void)
            }),
            ("onTouchMove", |rt, t, a| {
                layer_action(rt, t, "onTouchMove", &["x", "y", "cx", "cy", "id"], &a)?;
                Ok(Variant::Void)
            }),
            ("onTouchScaling", |rt, t, a| {
                layer_action(rt, t, "onTouchScaling", &["startdistance", "currentdistance", "cx", "cy", "flag"], &a)?;
                Ok(Variant::Void)
            }),
            ("onTouchRotate", |rt, t, a| {
                layer_action(rt, t, "onTouchRotate", &["startangle", "currentangle", "distance", "cx", "cy", "flag"], &a)?;
                Ok(Variant::Void)
            }),
            ("onMultiTouch", |rt, t, a| {
                layer_action(rt, t, "onMultiTouch", &[], &a)?;
                Ok(Variant::Void)
            }),
            // onSearchNext/PrevFocusable and onBeforeFocus: action, then the
            // passed layer becomes FocusWork (null/void clears it).
            ("onSearchPrevFocusable", |rt, t, a| focus_work_event(rt, t, "onSearchPrevFocusable", &["layer"], a)),
            ("onSearchNextFocusable", |rt, t, a| focus_work_event(rt, t, "onSearchNextFocusable", &["layer"], a)),
            ("onBeforeFocus", |rt, t, a| focus_work_event(rt, t, "onBeforeFocus", &["layer", "blurred", "direction"], a)),
            // Layer images are never shared: assignImages copies into the
            // destination's own canvas and province map (natives.rs
            // layer_assign_images), so each layer already owns independent
            // image buffers -- the state independMainImage establishes.
            ("independMainImage", |rt, t, _| {
                this_of(rt, t, "Layer.independMainImage")?;
                Ok(Variant::Void)
            }),
            ("independProvinceImage", |rt, t, _| {
                this_of(rt, t, "Layer.independProvinceImage")?;
                Ok(Variant::Void)
            }),
            // krkrz SetProvincePixel (LayerIntf.cpp:2647): allocate, clip,
            // store the low byte of `n`.
            ("setProvincePixel", |rt, t, a| {
                const M: &str = "Layer.setProvincePixel";
                let h = this_of(rt, t, M)?;
                let (x, y, n) = (need_int(&a, 0, M)?, need_int(&a, 1, M)?, need_int(&a, 2, M)?);
                let owner = rt.host().client;
                let l = rt.host().layer(h)?;
                let clip = l.draw_clip.unwrap_or([0, 0, l.image_width, l.image_height]);
                let inside = x >= clip[0].max(0)
                    && y >= clip[1].max(0)
                    && x < (clip[0] + clip[2]).min(l.image_width)
                    && y < (clip[1] + clip[3]).min(l.image_height);
                let layer = rt.host_mut().layer_mut(h)?;
                layer.province_allocated = true;
                if !inside {
                    return Ok(Variant::Void);
                }
                layer
                    .regions
                    .fill(owner, [x, y, 1, 1], (n & 0xff) as u32)
                    .map_err(|e| TjsError::runtime(e.to_string()))?;
                layer.image_modified = true;
                Ok(Variant::Void)
            }),
        ],
    );
    properties(
        rt,
        class,
        &[
            (
                "nodeFocusable",
                |rt, t| {
                    let h = this_of(rt, t, "Layer.nodeFocusable")?;
                    Ok(Variant::Integer(node_focusable(rt, h)? as i64))
                },
                read_only,
            ),
            (
                "nextFocusable",
                |rt, t| {
                    let h = this_of(rt, t, "Layer.nextFocusable")?;
                    Ok(layer_or_null(search_focusable(rt, h, true)?))
                },
                read_only,
            ),
            (
                "prevFocusable",
                |rt, t| {
                    let h = this_of(rt, t, "Layer.prevFocusable")?;
                    Ok(layer_or_null(search_focusable(rt, h, false)?))
                },
                read_only,
            ),
            // krkrz `cached` keeps a CPU cache bitmap of the composed layer;
            // it changes no pixels. The engine retains composition itself.
            (
                "cached",
                |rt, t| {
                    let h = this_of(rt, t, "Layer.cached")?;
                    Ok(Variant::Integer(rt.object_member(h, "__cached").is_truthy() as i64))
                },
                |rt, t, v| {
                    let h = this_of(rt, t, "Layer.cached")?;
                    rt.set_object_member(h, "__cached", Variant::Integer(v.is_truthy() as i64));
                    Ok(())
                },
            ),
        ],
    );
}

fn focus_work_event(rt: &mut Rt, t: Option<ObjectHandle>, event: &str, fields: &[&str], a: Vec<Variant>) -> Result<Variant> {
    let h = layer_action(rt, t, event, fields, &a)?;
    let work = match &a[0] {
        Variant::Void | Variant::Null => Variant::Null,
        v => {
            let l = object_arg(v)
                .map(|o| rt.bound_this(o).unwrap_or(o))
                .filter(|o| rt.host().layer(*o).is_ok())
                .ok_or_else(|| TjsError::runtime("Specify a Layer class object"))?;
            Variant::self_bound(l)
        }
    };
    rt.set_object_member(h, "__focusWork", work);
    Ok(Variant::Void)
}

// ------------------------------------------------------------- Window

fn window_action(rt: &mut Rt, t: Option<ObjectHandle>, event: &str, fields: &[&str], a: &[Variant]) -> Result<Variant> {
    let h = this_of(rt, t, event)?;
    action(rt, h, h, event, fields, a)?;
    Ok(Variant::Void)
}

fn stored(rt: &Rt, t: Option<ObjectHandle>, slot: &str, default: i64) -> Result<Variant> {
    let h = this_of(rt, t, "Window")?;
    Ok(match rt.object_member(h, slot) {
        Variant::Void => Variant::Integer(default),
        v => v,
    })
}

fn store(rt: &mut Rt, t: Option<ObjectHandle>, slot: &str, v: Variant) -> Result<()> {
    let h = this_of(rt, t, "Window")?;
    rt.set_object_member(h, slot, v);
    Ok(())
}

pub(crate) fn mouse_cursor_hidden(rt: &Rt, window: ObjectHandle) -> bool {
    rt.object_member(window, "__mouse_cursor_hidden").is_truthy()
}

fn install_window(rt: &mut Rt) {
    let Some(class) = rt.global_member("Window").object_handle() else { return };
    methods(
        rt,
        class,
        &[
            // krkrz Window key/wheel defaults invoke `action` on the window
            // itself (WindowIntf.cpp:1154-1200).
            ("onKeyDown", |rt, t, a| window_action(rt, t, "onKeyDown", &["key", "shift"], &a)),
            ("onKeyUp", |rt, t, a| window_action(rt, t, "onKeyUp", &["key", "shift"], &a)),
            ("onKeyPress", |rt, t, a| window_action(rt, t, "onKeyPress", &["key"], &a)),
            ("onMouseWheel", |rt, t, a| window_action(rt, t, "onMouseWheel", &["shift", "delta", "x", "y"], &a)),
        ],
    );
    properties(
        rt,
        class,
        &[
            // krkrz tTVPMouseCursorState (WindowFormUnit.cpp:604-650):
            // mcsVisible 0, mcsTempHidden 1 (until the pointer moves; the
            // existing __temporary_cursor_hide), mcsHidden 2.
            (
                "mouseCursorState",
                |rt, t| {
                    let h = this_of(rt, t, "Window.mouseCursorState")?;
                    Ok(Variant::Integer(if mouse_cursor_hidden(rt, h) {
                        2
                    } else if rt.object_member(h, "__temporary_cursor_hide").is_truthy() {
                        1
                    } else {
                        0
                    }))
                },
                |rt, t, v| {
                    let h = this_of(rt, t, "Window.mouseCursorState")?;
                    let state = v.to_integer()?;
                    if !(0..=2).contains(&state) {
                        return Err(TjsError::runtime(format!("Window.mouseCursorState: invalid state {state}")));
                    }
                    rt.set_object_member(h, "__mouse_cursor_hidden", Variant::Integer((state == 2) as i64));
                    rt.set_object_member(h, "__temporary_cursor_hide", Variant::Integer((state == 1) as i64));
                    let host = rt.host_mut();
                    host.ui_op(json!({"op":"set_cursor_visible","visible":state == 0}));
                    host.cursors.invalidate();
                    Ok(())
                },
            ),
            // Window.focusable (WindowIntf.cpp): whether the OS window takes
            // keyboard focus; the engine window does by default.
            ("focusable", |rt, t| stored(rt, t, "__focusable", 1), |rt, t, v| store(rt, t, "__focusable", Variant::Integer(v.is_truthy() as i64))),
            // useMouseKey: arrow keys move the pointer between focusable
            // layers; trapKey: keys are not passed to the OS menu. Both are
            // retained and read by the input bridge.
            ("useMouseKey", |rt, t| stored(rt, t, "__useMouseKey", 0), |rt, t, v| store(rt, t, "__useMouseKey", Variant::Integer(v.is_truthy() as i64))),
            ("trapKey", |rt, t| stored(rt, t, "__trapKey", 0), |rt, t, v| store(rt, t, "__trapKey", Variant::Integer(v.is_truthy() as i64))),
            // imeMode: the engine has no IME composition; the mode is script state only.
            (
                "imeMode",
                |rt, t| stored(rt, t, "__imeMode", 0),
                |rt, t, v| {
                    store(rt, t, "__imeMode", Variant::Integer(v.to_integer()?))
                },
            ),
            (
                "stayOnTop",
                |rt, t| stored(rt, t, "__stayOnTop", 0),
                |rt, t, v| {
                    // The desktop host owns z-order; the script reads back what it set.
                    store(rt, t, "__stayOnTop", Variant::Integer(v.is_truthy() as i64))
                },
            ),
            // Presentation is paced by the engine's vsync'd swap chain.
            ("waitVSync", |rt, t| stored(rt, t, "__waitVSync", 1), |rt, t, v| store(rt, t, "__waitVSync", Variant::Integer(v.is_truthy() as i64))),
            // Tooltip delay (ms) used by the shared tooltip presenter.
            ("hintDelay", |rt, t| stored(rt, t, "__hintDelay", 500), |rt, t, v| store(rt, t, "__hintDelay", Variant::Integer(v.to_integer()?.max(0)))),
            // No touch digitizer is exposed by the engine input frame.
            ("enableTouch", |rt, t| stored(rt, t, "__enableTouch", 0), |rt, t, v| store(rt, t, "__enableTouch", Variant::Integer(v.is_truthy() as i64))),
            ("touchScaleThreshold", |rt, t| stored(rt, t, "__touchScaleThreshold", 5), |rt, t, v| store(rt, t, "__touchScaleThreshold", Variant::Real(v.to_real()?))),
            ("touchRotateThreshold", |rt, t| stored(rt, t, "__touchRotateThreshold", 5), |rt, t, v| store(rt, t, "__touchRotateThreshold", Variant::Real(v.to_real()?))),
            ("touchPointCount", |rt, t| { this_of(rt, t, "Window.touchPointCount")?; Ok(Variant::Integer(0)) }, read_only),
            // Landscape, unrotated desktop display (krkrz orientUnknown=0,
            // orientPortrait=1, orientLandscape=2).
            ("displayOrientation", |rt, t| { this_of(rt, t, "Window.displayOrientation")?; Ok(Variant::Integer(2)) }, read_only),
            ("displayRotate", |rt, t| { this_of(rt, t, "Window.displayRotate")?; Ok(Variant::Integer(0)) }, read_only),
        ],
    );
}

// ------------------------------------------------------------- System

fn install_system(rt: &mut Rt) {
    let Some(system) = rt.global_member("System").object_handle() else { return };
    properties(
        rt,
        system,
        &[
            // TVPSetSystemEventDisabledState (EventImpl.cpp:108): while set,
            // input and immediate/discardable events are not delivered. The
            // adapter's input and timer bridges read this flag.
            (
                "eventDisabled",
                |rt, _| Ok(Variant::Integer(rt.host().event_disabled as i64)),
                |rt, _, v| {
                    rt.host_mut().event_disabled = v.is_truthy();
                    Ok(())
                },
            ),
            ("exeBits", |rt, _| Ok(Variant::Integer(rt.host().exe_bits)), read_only),
            ("osBits", |_, _| Ok(Variant::Integer(64)), read_only),
            (
                "processorNum",
                |_, _| {
                    Ok(Variant::Integer(
                        std::thread::available_parallelism().map_or(1, |n| n.get() as i64),
                    ))
                },
                read_only,
            ),
            ("touchDevice", |_, _| Ok(Variant::Integer(0)), read_only),
        ],
    );
}

// ------------------------------------------------------------- WaveSoundBuffer

fn install_wave(rt: &mut Rt) {
    let Some(class) = rt.global_member("WaveSoundBuffer").object_handle() else { return };
    methods(
        rt,
        class,
        &[
            // krkrz freeDirectSound -> TVPReleaseDirectSound (WaveImpl.cpp:3360):
            // release the primary device objects so the next buffer recreates
            // them with current options. The output device is owned by the
            // engine audio service, never by this adapter, so there is no
            // adapter-held device object to release; buffers are unaffected
            // exactly as in the original.
            ("freeDirectSound", |rt, _, _| {
                rt.host_mut().push_log(
                    "WaveSoundBuffer.freeDirectSound: output device is engine-owned; no adapter device object".into(),
                );
                Ok(Variant::Void)
            }),
        ],
    );
}
