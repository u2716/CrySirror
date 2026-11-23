// SPDX-License-Identifier: MPL-2.0
//! 430CA0/432520 window controls and 42FFC0 reverse-order prompt sheets.
//! Widgets own hit testing/capture; the adapter only retains native action,
//! cell and wait policy. No screen-space pointer sampler is implemented here.
use super::*;
const FIRST: u32 = 35000;
const PROMPT: u32 = 34999;
#[derive(Default)]
pub(super) struct WindowUi {
    widgets: BTreeSet<u32>,
    layers: BTreeSet<u32>,
    cells: Vec<u32>,
    enabled: bool,
    popup: Option<usize>,
    prompt: Option<(i32, u32)>,
    pub(super) epoch: u64,
    pub(super) auto_start: u64,
    pub(super) auto_units: usize,
    pub(super) skip: bool,
    pub(super) suspended: Option<Box<Wait>>,
}
impl SugarPotVm {
    ///4306F0 leaves the button artwork while withdrawing the completed
    ///prompt's interaction. Keep it visible across the next async resource
    ///admission, rather than exposing a gap until the following text publish.
    pub(super) fn suspend_window_controls(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let mut removed = BTreeSet::new();
        if self.window_ui.layers.contains(&PROMPT) {
            removed.insert(PROMPT);
        }
        if let Some(index) = self.window_ui.popup {
            let id = FIRST + 2 * index as u32 + 1;
            if self.window_ui.layers.contains(&id) {
                removed.insert(id);
            }
        }
        self.submit(
            h,
            removed
                .iter()
                .map(|id| json!({"op":"remove","id":id}))
                .collect(),
            self.window_ui
                .widgets
                .iter()
                .map(|id| json!({"op":"set_enabled","id":id,"enabled":false}))
                .collect(),
        )?;
        self.window_ui.layers.retain(|id| !removed.contains(id));
        self.published_layers.retain(|id| !removed.contains(id));
        self.window_ui.enabled = false;
        self.window_ui.popup = None;
        self.window_ui.prompt = None;
        Ok(())
    }
    fn window_image(&self, name: &str) -> Result<&UploadedImage> {
        let path = self.resolve("grp", name, &["abm", "bmp", "dat"])?;
        self.images
            .get(&path)
            .context("SugarPot declared window image not prepared")
    }
    /// Remove only controls actually published. Native text/frame layers have
    /// their own retained list and remain available for wndon/right-click.
    pub(super) fn clear_window_controls(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.window_ui.layers.is_empty() && self.window_ui.widgets.is_empty() {
            return Ok(());
        }
        let layers = self
            .window_ui
            .layers
            .iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect();
        let ui = self
            .window_ui
            .widgets
            .iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect();
        self.submit(h, layers, ui)?;
        self.published_layers
            .retain(|id| !self.window_ui.layers.contains(id));
        self.published_ui
            .retain(|id| !self.window_ui.widgets.contains(id));
        self.window_ui.layers.clear();
        self.window_ui.widgets.clear();
        self.window_ui.cells.clear();
        self.window_ui.popup = None;
        self.window_ui.prompt = None;
        Ok(())
    }
    /// Returns true when native controls consumed this pump (including hover,
    /// which must suppress the text area's left-click advance).
    pub(super) fn poll_window_controls(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool> {
        let Some(w) = self
            .windows
            .get(self.current_window)
            .filter(|w| w.visibility == 1)
            .cloned()
        else {
            self.clear_window_controls(h)?;
            return Ok(false);
        };
        let waiting = if matches!(self.wait, Some(Wait::Selection)) {
            32
        } else {
            self.native_text.waiting
        };
        let enabled = matches!(waiting, 8 | 16 | 32)
            && matches!(self.wait, Some(Wait::Text | Wait::Input | Wait::Selection));
        ensure!(
            w.native.buttons.len() <= 1024,
            "SugarPot inline window button budget"
        );
        let mut layers = Vec::new();
        let mut ui = Vec::new();
        let mut hover = None;
        let mut action = None;
        let mut cells = Vec::new();
        for (index, button) in w.native.buttons.iter().enumerate() {
            let id = FIRST + 2 * index as u32;
            let image = self.window_image(&button.image)?.clone();
            ensure!(
                image.size[1] > 0 && image.size[1] % 3 == 0,
                "SugarPot inline button three-cell sheet"
            );
            let exists = self.window_ui.widgets.contains(&id);
            let state: Json = if enabled && exists {
                serde_json::from_slice(
                    &h.query_engine(self.client, EngineQuery::UiInteraction(id))?,
                )?
            } else {
                Json::Null
            };
            if state["hovered"] == true {
                hover = Some(index);
            }
            if state["activated"] == true {
                action = Some(button.label.clone());
            }
            let cell =
                if button.label.eq_ignore_ascii_case("automode") && self.settings.values[5] == 1 {
                    2
                } else if state["pressed"] == true {
                    2
                } else if state["hovered"] == true {
                    1
                } else {
                    0
                };
            cells.push(cell);
            if !exists {
                ui.push(json!({"op":"put","widget":{"id":id,"rect":[w.native.fields[1]+button.position[0],w.native.fields[2]+button.position[1],image.size[0],image.size[1]/3],"z":3100+index,"kind":"button","interaction_only":true,"keyboard_focusable":false,"enabled":enabled}}));
            } else if self.window_ui.enabled != enabled {
                ui.push(json!({"op":"set_enabled","id":id,"enabled":enabled}));
            }
            if !self.window_ui.layers.contains(&id)
                || self.window_ui.cells.get(index) != Some(&cell)
            {
                let sprite = Sprite {
                    native_depth: self.depths.get(image.path.relative()).copied().unwrap_or(0),
                    image: Some(image),
                    position: [
                        w.native.fields[1] + button.position[0],
                        w.native.fields[2] + button.position[1],
                    ],
                    visible: true,
                    divisions: 3,
                    cell,
                    ..Default::default()
                };
                layers.push(Self::sprite_layer(id, &sprite, 3100 + index as i32)?.unwrap());
            }
        }
        // 430CA0 selects the last overlapping hovered button's popup.
        if self.window_ui.popup != hover {
            if let Some(old) = self.window_ui.popup {
                let id = FIRST + 2 * old as u32 + 1;
                if self.window_ui.layers.contains(&id) {
                    layers.push(json!({"op":"remove","id":id}));
                }
            }
            if let Some(index) = hover {
                let button = &w.native.buttons[index];
                if !button.overlay.is_empty() {
                    let image = self.window_image(&button.overlay)?.clone();
                    let sprite = Sprite {
                        native_depth: self.depths.get(image.path.relative()).copied().unwrap_or(0),
                        image: Some(image),
                        position: [
                            w.native.fields[1] + button.overlay_position[0],
                            w.native.fields[2] + button.overlay_position[1],
                        ],
                        visible: true,
                        divisions: 1,
                        ..Default::default()
                    };
                    layers.push(
                        Self::sprite_layer(FIRST + 2 * index as u32 + 1, &sprite, 3200)?.unwrap(),
                    );
                }
            }
        }
        let prompt = if enabled && self.window_ui.skip == false && matches!(waiting, 8 | 16) {
            let data = if waiting == 16 {
                w.native
                    .foreground
                    .as_ref()
                    .map(|p| (p.image.as_str(), p.fields[0], p.fields[1], p.fields[2]))
            } else {
                w.native
                    .page
                    .as_ref()
                    .map(|p| (p.image.as_str(), p.fields[0], p.fields[1], p.fields[2]))
            };
            if let Some((name, x, y, last)) = data {
                ensure!(
                    (0..=127).contains(&last),
                    "SugarPot prompt signed cell count"
                );
                let cell = last as u32
                    - (h.now_ms().saturating_sub(self.window_ui.epoch) / 66 % (last as u64 + 1))
                        as u32;
                if self.window_ui.prompt != Some((waiting, cell))
                    || !self.window_ui.layers.contains(&PROMPT)
                {
                    let image = self.window_image(name)?.clone();
                    let sprite = Sprite {
                        native_depth: self.depths.get(image.path.relative()).copied().unwrap_or(0),
                        image: Some(image),
                        position: [w.native.fields[1] + x, w.native.fields[2] + y],
                        visible: true,
                        divisions: last as u32 + 1,
                        cell,
                        ..Default::default()
                    };
                    layers.push(Self::sprite_layer(PROMPT, &sprite, 3099)?.unwrap());
                }
                Some((waiting, cell))
            } else {
                None
            }
        } else {
            None
        };
        if prompt.is_none() && self.window_ui.layers.contains(&PROMPT) {
            layers.push(json!({"op":"remove","id":PROMPT}));
        }
        self.submit(h, layers.clone(), ui)?;
        for op in &layers {
            if let Some(id) = op["layer"]["id"].as_u64() {
                self.window_ui.layers.insert(id as u32);
            } else if let Some(id) = op["id"].as_u64() {
                self.window_ui.layers.remove(&(id as u32));
            }
        }
        self.window_ui.widgets = (0..w.native.buttons.len())
            .map(|i| FIRST + 2 * i as u32)
            .collect();
        self.window_ui.cells = cells;
        self.window_ui.enabled = enabled;
        self.window_ui.popup = hover;
        self.window_ui.prompt = prompt;
        self.published_layers
            .retain(|id| !(*id == PROMPT || (FIRST..FIRST + 2048).contains(id)));
        self.published_layers
            .extend(self.window_ui.layers.iter().copied());
        self.published_ui
            .retain(|id| !(FIRST..FIRST + 2048).contains(id));
        self.published_ui
            .extend(self.window_ui.widgets.iter().copied());
        if let Some(action) = action {
            input.buttons_pressed = 0;
            match action.as_str() {
                "automode" => self.settings.values[5] = i32::from(self.settings.values[5] != 1),
                "startskip" => {
                    if waiting != 32 {
                        self.window_ui.skip = !self.window_ui.skip;
                    }
                }
                "config" => {
                    self.window_ui.suspended = self.wait.take().map(Box::new);
                    self.settings_menu(h)?;
                }
                "savemenu" if !self.save_enabled => {}
                "loadmenu" if !self.load_enabled => {}
                "quicksave" if !self.save_enabled => {}
                "quickload" if !self.load_enabled => {}
                "quickload" => self.start_native_load(100, true, false)?,
                "quicksave" => self.start_native_save(h, 100, false)?,
                "savemenu" => self.start_slot_menu(false)?,
                "loadmenu" => self.start_slot_menu(true)?,
                "calllog" => {
                    self.open_backlog(h)?;
                }
                //432520 resets the clicked state and returns for unknown labels;
                //they are not script branch destinations.
                _ => {}
            }
            return Ok(true);
        }
        Ok(hover.is_some() && input.buttons_pressed & buttons::MOUSE_LEFT != 0)
    }
}
