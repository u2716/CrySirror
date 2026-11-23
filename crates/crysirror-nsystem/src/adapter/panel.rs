// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native screen controls (`CDIBBaseScreen` children, `CDIBButtonCtrl`
//! 0x41E880..). A control is a parts-sheet button with five state cells:
//! `SetEnable` 0x41EAC0 / `OnGetForcus` 0x41EC10 / `OnKeyEnterDown` 0x41ECF0
//! pick cell 0 normal, 1 focused, 2 pressed when the control is "on", and 3
//! (normal) or 4 (focused/pressed) when it is "off". Screens dispatch on the
//! control index (vfunc 116, the creation order), not the setup id.
use super::*;

#[derive(Clone, Debug)]
pub(super) struct Control {
    pub pos: [i32; 2],
    pub cells: [i32; 5],
    /// `+298`: on (selectable); off controls show cells 3/4 and do not fire.
    pub on: bool,
    pub visible: bool,
    /// Toggle buttons (MESWND 57/58: ten cells): the second set while set.
    pub alt: Option<[i32; 5]>,
    pub toggled: bool,
    /// `CDIBScrollCtrl` (0x42F4B0): value 0..=255; `alt` holds the knob cells.
    pub slider: Option<Slider>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Slider {
    pub value: i32,
    pub vertical: bool,
    /// Track margins at the start/end (setup keys 170..177).
    pub margins: [i32; 2],
}
#[derive(Clone, Debug)]
pub(super) struct Panel {
    pub sheet: String,
    pub controls: Vec<Control>,
    pub focus: Option<usize>,
    pub base_id: u32,
    pub order: i32,
}
impl Panel {
    pub fn new(sheet: &str, base_id: u32, order: i32) -> Self {
        Self { sheet: sheet.into(), controls: Vec::new(), focus: None, base_id, order }
    }
    /// Control `i` of a setup script: position key `pos_key + i`, cells
    /// `cell_key + i` (screen `screen`).
    pub fn from_setup(vm: &NSystemVm, script: &Script, screen: i32, count: usize, pos_key: i32, cell_key: i32, sheet: &str, base_id: u32, order: i32) -> Result<Self> {
        let mut panel = Self::new(sheet, base_id, order);
        for i in 0..count as i32 {
            let get = |key: i32| -> Result<Vec<i32>> {
                script
                    .setup(screen, key)
                    .with_context(|| format!("N-System setup {screen}/{key}"))?
                    .iter()
                    .map(|o| vm.value(o))
                    .collect()
            };
            let pos = get(pos_key + i)?;
            let cells = get(cell_key + i)?;
            ensure!(pos.len() == 2 && cells.len() >= 5, "N-System control {i} setup shape");
            let alt = (cells.len() >= 10).then(|| [cells[5], cells[6], cells[7], cells[8], cells[9]]);
            panel.controls.push(Control { pos: [pos[0], pos[1]], cells: [cells[0], cells[1], cells[2], cells[3], cells[4]], on: true, visible: true, alt, toggled: false, slider: None });
        }
        Ok(panel)
    }
    fn size(&self, vm: &NSystemVm, i: usize) -> Option<[i32; 2]> {
        let key = format!("mgd/{}.mgd", self.sheet.to_ascii_lowercase());
        let r = vm.regions.get(&key)?.get(usize::try_from(self.controls[i].cells[0]).ok()?)?;
        Some([i32::from(r[2]), i32::from(r[3])])
    }
    /// One input frame: pointer hover focuses, arrows move focus, a click or
    /// Enter on an "on" control returns its index.
    pub fn step(&mut self, vm: &NSystemVm, input: InputFrame) -> Option<usize> {
        let mut picked = None;
        let (px, py) = (input.pointer_x, input.pointer_y);
        for i in 0..self.controls.len() {
            let Some([w, h]) = self.size(vm, i) else { continue };
            let c = &mut self.controls[i];
            if c.visible && px >= c.pos[0] && py >= c.pos[1] && px < c.pos[0] + w && py < c.pos[1] + h {
                self.focus = Some(i);
                match &mut c.slider {
                    // CalcScrollPos 0x430100: position along the track.
                    Some(s) if c.on && input.buttons_down & buttons::MOUSE_LEFT != 0 => {
                        let (at, len) = if s.vertical { (py - c.pos[1], h) } else { (px - c.pos[0], w) };
                        let t = ((at - s.margins[0]) as f64 / (len - s.margins[1] - s.margins[0]).max(1) as f64).clamp(0.0, 1.0);
                        let v = (255.0 * t) as i32;
                        if v != s.value {
                            s.value = v;
                            picked = Some(i);
                        }
                    }
                    None if input.buttons_pressed & buttons::MOUSE_LEFT != 0 && c.on => picked = Some(i),
                    _ => {}
                }
            }
        }
        // OnKeyLeftBtn/OnKeyRightBtn 0x42FB40: ±1 on the focused slider.
        if let Some(i) = self.focus {
            let c = &mut self.controls[i];
            if let (Some(s), true) = (&mut c.slider, c.on) {
                let d = i32::from(input.buttons_pressed & buttons::RIGHT != 0) - i32::from(input.buttons_pressed & buttons::LEFT != 0);
                if d != 0 {
                    s.value = (s.value + d).clamp(0, 255);
                    return Some(i);
                }
            }
        }
        let n = self.controls.len();
        let step = |from: Option<usize>, down: bool, c: &[Control]| -> Option<usize> {
            let mut i = from.unwrap_or(if down { n - 1 } else { 0 });
            for _ in 0..n {
                i = if down { (i + 1) % n } else { (i + n - 1) % n };
                if c[i].visible {
                    return Some(i);
                }
            }
            from
        };
        let on_slider = self.focus.is_some_and(|i| self.controls[i].slider.is_some());
        let lr = if on_slider { 0 } else { buttons::RIGHT };
        let ll = if on_slider { 0 } else { buttons::LEFT };
        if n > 0 && input.buttons_pressed & (buttons::DOWN | lr | buttons::TAB) != 0 {
            self.focus = step(self.focus, true, &self.controls);
        }
        if n > 0 && input.buttons_pressed & (buttons::UP | ll) != 0 {
            self.focus = step(self.focus, false, &self.controls);
        }
        if input.buttons_pressed & (buttons::ENTER | buttons::SPACE) != 0 {
            if let Some(i) = self.focus.filter(|i| self.controls[*i].on && self.controls[*i].visible && self.controls[*i].slider.is_none()) {
                picked = Some(i);
            }
        }
        picked
    }
    pub fn cell(&self, i: usize) -> i32 {
        let c = &self.controls[i];
        let focused = self.focus == Some(i);
        let cells = c.alt.filter(|_| c.toggled && c.slider.is_none()).unwrap_or(c.cells);
        match (c.on, focused) {
            (true, false) => cells[0],
            (true, true) => cells[1],
            (false, false) => cells[3],
            (false, true) => cells[4],
        }
    }
    pub fn layers(&self, vm: &NSystemVm, h: &mut dyn HostServices) -> Result<Vec<Json>> {
        let mut out = Vec::new();
        for (i, c) in self.controls.iter().enumerate() {
            let id = self.base_id + i as u32;
            if !c.visible {
                out.push(json!({"op":"remove","id":id}));
                out.push(json!({"op":"remove","id":id + 200}));
                continue;
            }
            out.push(vm.cell(h, id, self.order, &self.sheet, usize::try_from(self.cell(i))?, c.pos)?);
            // UpdateVertex 0x42FE40: the knob centred on the value along the track.
            if let (Some(s), Some(knob)) = (c.slider, c.alt) {
                let state = (self.cell(i) - c.cells[0]).clamp(0, 4) as usize;
                let key = format!("mgd/{}.mgd", self.sheet.to_ascii_lowercase());
                let regions = vm.regions.get(&key).context("slider sheet regions")?;
                let bar = regions.get(usize::try_from(c.cells[0])?).context("slider bar cell")?;
                let k = regions.get(usize::try_from(knob[state])?).context("slider knob cell")?;
                let t = s.value as f64 / 255.0;
                let at = if s.vertical {
                    [c.pos[0] + (i32::from(bar[2]) - i32::from(k[2])) / 2, c.pos[1] + s.margins[0] + ((i32::from(bar[3]) - s.margins[1] - s.margins[0]) as f64 * t) as i32 - i32::from(k[3]) / 2]
                } else {
                    [c.pos[0] + s.margins[0] + ((i32::from(bar[2]) - s.margins[1] - s.margins[0]) as f64 * t) as i32 - i32::from(k[2]) / 2, c.pos[1] + (i32::from(bar[3]) - i32::from(k[3])) / 2]
                };
                out.push(vm.cell(h, id + 200, self.order + 1, &self.sheet, usize::try_from(knob[state])?, at)?);
            } else {
                out.push(json!({"op":"remove","id":id + 200}));
            }
        }
        Ok(out)
    }
    pub fn remove(&self) -> Vec<Json> {
        (0..self.controls.len()).map(|i| json!({"op":"remove","id":self.base_id + i as u32})).collect()
    }
}
