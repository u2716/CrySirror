// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original BUTTON (class 6): an IMAGE with focus, press and script events. 768190
//! property family, 749F10 setid, 7484D0 enable, 74B880 lock, and the 629850/756B10
//! `name.FOCUS`/`UNFOCUS`/`PUSH_*`/`RELEASE_*` section calls. Pixels use the IMAGE path.
use crate::{fes_object_bank::Bank, native_class_state::State, native_interpreter::Interpreter, native_values::StringVariables};
use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub(crate) struct ButtonState {
    pub enabled: bool,
    /// 74B780: shared event-name prefix, separate from the object name.
    pub group: Vec<u8>,
    pub kind: u32,
    pub lock: u32,
    /// `noact` target: 62BED0 cases 4..7 focus this button when an arrow key has no
    /// current selection. It never disarms pointer presses.
    pub no_action: bool,
    pub focused: bool,
    pub pressed: bool,
    pub attach:Option<crate::fes_references::IndexedName>,
    pub drag:bool,
    pub navigation:[i32;3],
}
impl Default for ButtonState {
    fn default()->Self{Self{enabled:false,group:Vec::new(),kind:0,lock:0,no_action:false,focused:false,pressed:false,attach:None,drag:false,navigation:[-1,0,0]}}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Focus,
    Unfocus,
    PushLeft,
    PushRight,
    ReleaseLeft,
    ReleaseRight,
    /// A slider's value moved under the pointer: `name.SCRL_P`, `_SCRL_P_`.
    ScrollPosition,
}
/// SLIDER (15), SCROLL (13), SCROLL2 (14): a knob bitmap moving along a rail with a value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Slider {
    pub horizontal: bool,
    pub range: [i32; 2],
    pub value: i32,
    /// Rail origin and length, from the object's position frame (POS2).
    rail: Option<[f32; 2]>,
    across: f32,
}
impl Slider {
    pub fn new(horizontal: bool) -> Self {
        Self { horizontal, range: [0, 100], value: 0, rail: None, across: 0. }
    }
    pub fn place(&mut self, origin: [f32; 2], size: [f32; 2]) {
        let axis = usize::from(!self.horizontal);
        self.rail = Some([origin[axis], size[axis]]);
        self.across = origin[1 - axis];
    }
    fn set(&mut self, value: i32) -> bool {
        let [low, high] = self.range;
        let value = value.clamp(low.min(high), low.max(high));
        let changed = value != self.value;
        self.value = value;
        changed
    }
    pub fn set_value(&mut self, value: i32) {
        self.set(value);
    }
    /// The knob's place for the current value, once the rail is known.
    fn knob(&self, knob: [i32; 2]) -> Option<[f32; 2]> {
        let [start, length] = self.rail?;
        let axis = usize::from(!self.horizontal);
        let travel = (length - knob[axis] as f32).max(0.);
        let span = (self.range[1] - self.range[0]) as f32;
        let along = if span == 0. { 0. } else { (self.value - self.range[0]) as f32 / span };
        let moved = start + along * travel;
        Some(if self.horizontal { [moved, self.across] } else { [self.across, moved] })
    }
    /// The value under a pointer coordinate relative to the object's plane.
    fn value_at(&self, pointer: [f32; 2], knob: [i32; 2]) -> Option<i32> {
        let [start, length] = self.rail?;
        let axis = usize::from(!self.horizontal);
        let travel = (length - knob[axis] as f32).max(1.);
        let along = ((pointer[axis] - start - knob[axis] as f32 / 2.) / travel).clamp(0., 1.);
        Some(self.range[0] + (along * (self.range[1] - self.range[0]) as f32).round() as i32)
    }
    fn hit(&self, pointer: [f32; 2], knob: [i32; 2]) -> bool {
        let Some([start, length]) = self.rail else { return false };
        let axis = usize::from(!self.horizontal);
        pointer[axis] >= start && pointer[axis] < start + length && pointer[1 - axis] >= self.across && pointer[1 - axis] < self.across + knob[1 - axis] as f32
    }
}
impl Event {
    fn suffix(self) -> &'static str {
        match self {
            Self::Focus => "FOCUS",
            Self::Unfocus => "UNFOCUS",
            Self::PushLeft => "PUSH_L",
            Self::PushRight => "PUSH_R",
            Self::ReleaseLeft => "RELEASE_L",
            Self::ReleaseRight => "RELEASE_R",
            Self::ScrollPosition => "SCRL_P",
        }
    }
}
/// A script section call owed to the player, `@` being the array index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Dispatch {
    pub object: u32,
    pub event: Event,
    pub section: Vec<u8>,
    pub name: Vec<u8>,
    pub index: u32,
}
pub(crate) fn supports(method: &[u8]) -> bool {
    matches!(method, b"setid" | b"getid" | b"enable" | b"noact" | b"lock" | b"attach" | b"drag")
}
/// True when `target` names a BUTTON or, for frame selection, a plain IMAGE.
pub(crate) fn applies(bank: &Bank, target: &[u8], method: &[u8], vm: &Interpreter, strings: &StringVariables) -> Result<bool> {
    let mut staged = vm.clone();
    let reference = crate::fes_references::evaluate(target, &mut staged, strings, &|name| bank.count(name))?;
    let index = if reference.index == -1 { 0 } else { reference.index };
    let Some(id) = bank.lookup(&crate::fes_references::IndexedName { name: reference.name, index }) else {
        return Ok(false);
    };
    Ok(match &bank.objects[&id].state {
        State::Image(image) => image.button.is_some() || matches!(method, b"setid" | b"getid"),
        _ => false,
    })
}
pub(crate) fn apply(bank: &mut Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut Interpreter, strings: &StringVariables) -> Result<()> {
    let method = arguments.first().context("CatSystem2 BUTTON method missing")?.clone();
    let args = &arguments[1..];
    let mut staged = vm.clone();
    let reference = crate::fes_references::evaluate(target, &mut staged, strings, &|name| bank.count(name))?;
    let count = bank.count(&reference.name).context("CatSystem2 BUTTON target name missing")?;
    // getid and noact return at once for the all-array sentinel.
    if reference.index == -1 && matches!(method.as_slice(), b"getid" | b"noact") {
        *vm = staged;
        return Ok(());
    }
    if method==b"attach" {
        let attached=args.first().map(|source|crate::fes_references::evaluate(source,&mut staged,strings,&|name|bank.count(name))).transpose()?;
        for id in bank.targets(target,&mut staged,strings)? {
            if let Some(State::Image(image))=bank.objects.get_mut(&id).map(|o|&mut o.state){if let Some(button)=&mut image.button{button.attach=attached.clone();}}
        }
        *vm=staged;return Ok(());
    }
    let value = args.first().map_or(Ok(0), |source| staged.evaluate(source))?;
    let indices: Vec<i32> = if reference.index == -1 { (0..count as i32).collect() } else { vec![reference.index] };
    for index in indices {
        let id = bank.lookup(&crate::fes_references::IndexedName { name: reference.name.clone(), index }).context("CatSystem2 BUTTON target outside array")?;
        let State::Image(image) = &mut bank.objects.get_mut(&id).unwrap().state else {
            anyhow::bail!("CatSystem2 BUTTON command on another native class")
        };
        match method.as_slice() {
            b"setid" => {
                if image.frame != value {
                    image.frame = value;
                    image.bitmap_dirty = true;
                }
            }
            b"getid" => {
                staged.set_flag(value, image.frame);
            }
            b"enable" => {
                let button = image.button.as_mut().context("CatSystem2 enable on a plain IMAGE")?;
                button.enabled = value != 0;
                if !button.enabled {
                    button.pressed = false;
                }
            }
            b"drag" => image.button.as_mut().context("CatSystem2 drag on a plain IMAGE")?.drag=value!=0,
            b"noact" => image.button.as_mut().context("CatSystem2 noact on a plain IMAGE")?.no_action = value != 0,
            b"lock" => image.button.as_mut().context("CatSystem2 lock on a plain IMAGE")?.lock = value,
            _ => anyhow::bail!("CatSystem2 unknown BUTTON method"),
        }
    }
    *vm = staged;
    Ok(())
}
/// Pointer containment in plane coordinates: the image is drawn relative to its
/// plane's VRAM origin, which the plane then presents at its own position.
fn contains(bank: &Bank, id: u32, pointer: [i32; 2]) -> bool {
    let State::Image(image) = &bank.objects[&id].state else {
        return false;
    };
    if image.button.as_ref().is_some_and(|b|b.drag){return true;}
    if image.size[0] <= 0 || image.size[1] <= 0 {
        return false;
    }
    let plane = bank.lookup(&image.plane).and_then(|target| match &bank.objects[&target].state {
        State::Plane(plane) => Some(plane),
        _ => None,
    });
    let origin = plane.map_or([0.; 2], |plane| [plane.pose.position[0], plane.pose.position[1]]);
    if plane.is_some_and(|plane| plane.pose.visible == 0) {
        return false;
    }
    if let Some(slider) = &image.slider {
        return slider.hit([pointer[0] as f32 - origin[0], pointer[1] as f32 - origin[1]], image.size);
    }
    let left = origin[0] + image.pose.position[0] + image.pose.offset[0] - image.pose.pivot[0];
    let top = origin[1] + image.pose.position[1] + image.pose.offset[1] - image.pose.pivot[1];
    let (x, y) = (pointer[0] as f32, pointer[1] as f32);
    x >= left && y >= top && x < left + image.size[0] as f32 && y < top + image.size[1] as f32
}
fn section(owner_name: &[u8], group: &[u8], event: Event) -> Vec<u8> {
    let base = if group.is_empty() { owner_name } else { group };
    let mut name = base.to_vec();
    name.push(b'.');
    name.extend_from_slice(event.suffix().as_bytes());
    name
}
/// One frame of 629850's button policy: top priority hit keeps focus; edges become
/// section calls in the native order (UNFOCUS, FOCUS, then press and release).
pub(crate) fn step(bank: &mut Bank, pointer: [i32; 2], left: [bool; 2], right: [bool; 2]) -> Vec<Dispatch> {
    let ids = bank.ordered_ids();
    // Knobs follow their values; a pressed slider follows the pointer even off the rail.
    let mut moved = Vec::new();
    for id in &ids {
        let origin = match &bank.objects[id].state {
            State::Image(image) if image.slider.is_some() => bank.lookup(&image.plane).and_then(|target| match &bank.objects[&target].state {
                State::Plane(plane) => Some([plane.pose.position[0], plane.pose.position[1]]),
                _ => None,
            }).unwrap_or([0.; 2]),
            _ => continue,
        };
        let State::Image(image) = &mut bank.objects.get_mut(id).unwrap().state else { continue };
        let pressed = image.button.as_ref().is_some_and(|b| b.pressed);
        let size = image.size;
        let Some(slider) = &mut image.slider else { continue };
        if pressed && left[1] {
            if let Some(value) = slider.value_at([pointer[0] as f32 - origin[0], pointer[1] as f32 - origin[1]], size) {
                if slider.set(value) { moved.push(*id); }
            }
        }
        if let Some(knob) = slider.knob(size) {
            image.pose.position[0] = knob[0];
            image.pose.position[1] = knob[1];
        }
    }
    let mut top: Option<(i32, u32)> = None;
    for id in &ids {
        let State::Image(image) = &bank.objects[id].state else { continue };
        let Some(button) = &image.button else { continue };
        // Display does not gate a hit: title scripts click invisible map buttons, and they
        // pair `disp 0` with `enable 0` to take a button away.
        if !button.enabled || !contains(bank, *id, pointer) {
            continue;
        }
        let priority = if button.drag {100000}else{image.pose.priority as i32};
        if top.is_none_or(|(best, _)| priority >= best) {
            top = Some((priority, *id));
        }
    }
    let target = top.map(|(_, id)| id);
    let mut out = vec![];
    let push = |bank: &Bank, id: u32, event: Event, out: &mut Vec<Dispatch>| {
        let object = &bank.objects[&id];
        if let State::Image(image) = &object.state {
            let group = image.button.as_ref().map_or(&[][..], |b| b.group.as_slice());
            out.push(Dispatch { section: section(&object.name, group, event), name: object.name.clone(), index: object.index, object: id, event });
        }
    };
    for id in &ids {
        let State::Image(image) = &mut bank.objects.get_mut(id).unwrap().state else { continue };
        let Some(button) = &mut image.button else { continue };
        if button.focused && Some(*id) != target {
            button.focused = false;
            button.pressed = false;
            push(bank, *id, Event::Unfocus, &mut out);
        }
    }
    for id in moved {
        push(bank, id, Event::ScrollPosition, &mut out);
    }
    if let Some(id) = target {
        let State::Image(image) = &mut bank.objects.get_mut(&id).unwrap().state else { return out };
        let button = image.button.as_mut().unwrap();
        let newly = !button.focused;
        button.focused = true;
        let was_pressed = button.pressed;
        if left == [false, true] {
            button.pressed = true;
        }
        if left == [true, false] {
            button.pressed = false;
        }
        if newly {
            push(bank, id, Event::Focus, &mut out);
        }
        if left == [false, true] {
            push(bank, id, Event::PushLeft, &mut out);
        }
        if right == [false, true] {
            push(bank, id, Event::PushRight, &mut out);
        }
        if was_pressed && left == [true, false] {
            push(bank, id, Event::ReleaseLeft, &mut out);
        }
        if right == [true, false] {
            push(bank, id, Event::ReleaseRight, &mut out);
        }
    }
    out
}
/// 77B640: native key anchor plus image pose and its displayed PLANE origin.
pub(crate) fn key_position(bank:&Bank,id:u32)->Option<[i32;2]>{
    let object=bank.objects.get(&id)?;
    let image=match &object.state{State::Image(v)|State::CImage(v)=>v,_=>return None};
    let mut point=[(image.pose.position[0]+image.pose.offset[0]-image.pose.pivot[0]) as i32+image.key_position[0],(image.pose.position[1]+image.pose.offset[1]-image.pose.pivot[1]) as i32+image.key_position[1]];
    if let Some(State::Plane(p)|State::CPlane(p))=bank.lookup(&image.plane).and_then(|id|bank.objects.get(&id)).map(|o|&o.state){if p.pose.visible!=0{point[0]+=p.pose.position[0] as i32;point[1]+=p.pose.position[1] as i32;}}
    Some(point)
}
/// 757C20: when grid navigation has no next button, an attached scrollbar/slider
/// consumes the direction by moving its native value by one and dispatching SCRL_P.
pub(crate) fn attached_direction(bank:&mut Bank,direction:usize)->Vec<Dispatch>{
    let focused=bank.ordered_ids().into_iter().find(|id|matches!(&bank.objects[id].state,State::Image(image) if image.button.as_ref().is_some_and(|b|b.enabled&&b.focused)));
    let Some(id)=focused else{return Vec::new()};
    let attached=match &bank.objects[&id].state{State::Image(v)=>v.button.as_ref().and_then(|b|b.attach.clone()),_=>None};
    let Some(target)=attached.and_then(|name|bank.lookup(&name))else{return Vec::new()};
    let object=bank.objects.get_mut(&target).unwrap();
    let State::Image(image)=&mut object.state else{return Vec::new()};
    let Some(slider)=&mut image.slider else{return Vec::new()};
    let relevant=if slider.horizontal{direction==2||direction==3}else{direction<2};
    if !relevant{return Vec::new()};
    let delta=if direction==0||direction==2{-1}else{1};
    if !slider.set(slider.value.saturating_add(delta)){return Vec::new()};
    let group=image.button.as_ref().map_or(&[][..],|b|b.group.as_slice());
    vec![Dispatch{object:target,event:Event::ScrollPosition,section:section(&object.name,group,Event::ScrollPosition),name:object.name.clone(),index:object.index}]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_sections_prefer_the_group_name() {
        assert_eq!(section(b"btn1", b"", Event::Focus), b"btn1.FOCUS");
        assert_eq!(section(b"btn1", b"grp", Event::PushLeft), b"grp.PUSH_L");
        assert_eq!(section(b"b", b"", Event::ReleaseRight), b"b.RELEASE_R");
    }
}
