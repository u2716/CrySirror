// SPDX-License-Identifier: MPL-2.0
// NitroSharp legacy Graphics/Choice.cs + Entity.cs, 36701328 (MIT).
// Shared widgets own touch/pointer capture, focus and activation receipts.
use super::*;
use crysirror_scene::{layer::LayerSpec, projection::HierarchyFrame, ui::WidgetInteraction};
pub(super) const STATES: [&str; 4] = ["MouseUsual", "MouseOver", "MouseClick", "MouseLeave"];
#[derive(Default)]
pub(super) struct Choice {
    pub widget: bool,
    pub state: u8,
    pub activation: u32,
    pub next: [Option<u32>; 4],
    pub disabled: bool,
}
impl N2SystemVm {
    pub(super) fn choice_owner(&self, mut id: u32) -> Option<u32> {
        for _ in 0..128 {
            if self.choices.contains_key(&id) {
                return Some(id);
            }
            id = self.entities.nodes.get(&id)?.parent?;
        }
        None
    }
    pub(super) fn create_choice(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        path: &str,
    ) -> Result<()> {
        let id = self.create(host, current, path, Kind::Name, [0.; 2])?;
        for name in STATES {
            let child = self
                .entities
                .add(name.into(), Some(id), Kind::Name, [0.; 2])?;
            let mut op = self.entity_group(child)?;
            op["layer"]["visible"] = json!(name == "MouseUsual" || name == "MouseLeave");
            self.submit(host, vec![op], vec![])?;
        }
        self.choices.insert(id, Choice::default());
        Ok(())
    }
    fn usual(&self, id: u32) -> Option<u32> {
        let group = self.entities.child(id, "MouseUsual")?;
        let mut visuals = self
            .entities
            .nodes
            .values()
            .filter(|e| e.parent == Some(group) && matches!(e.kind, Kind::Visual));
        let id = visuals.next()?.id;
        if visuals.next().is_some() {
            return None;
        }
        Some(id)
    }
    fn canvas_rect(
        &self,
        host: &mut dyn HostServices,
        id: u32,
    ) -> Result<Option<([f32; 4], i32, bool)>> {
        let layer: LayerSpec = serde_json::from_value(self.layer(host, id)?)?;
        let mut ancestors = Vec::new();
        let mut parent = layer.parent;
        while let Some(id) = parent {
            ensure!(ancestors.len() < 128, "N2System UI parent depth");
            let layer: LayerSpec = serde_json::from_value(self.layer(host, id)?)?;
            parent = layer.parent;
            ancestors.push(layer);
        }
        let visible = layer.visible && ancestors.iter().all(|l| l.visible);
        let mut hierarchy = HierarchyFrame::default();
        for parent in ancestors.iter().rev() {
            hierarchy.push(&parent.transform, [parent.rect[0], parent.rect[1]])?;
        }
        let Some(quad) = hierarchy.quad(
            &layer.transform,
            [layer.rect[0], layer.rect[1]],
            [0., 0., layer.rect[2], layer.rect[3]],
            self.config.canvas.map(|n| n as f32),
        ) else {
            return Ok(None);
        };
        let min = [
            quad.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min),
            quad.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min),
        ];
        let max = [
            quad.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max),
            quad.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max),
        ];
        Ok(Some((
            [min[0], min[1], max[0] - min[0], max[1] - min[1]],
            layer.order[0],
            visible,
        )))
    }
    pub(super) fn refresh_choices(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let ids: Vec<_> = self.choices.keys().copied().collect();
        let mut ui = Vec::new();
        for id in ids {
            let geometry = if let Some(visual) = self.usual(id) {
                self.canvas_rect(host, visual)?
            } else {
                None
            };
            let widget = self.choices[&id].widget;
            if let Some((rect, z, visible)) = geometry {
                if !widget {
                    ui.push(json!({"op":"put","widget":{"id":id,"rect":rect,"kind":"button","z":z,"visible":visible,
                        "enabled":!self.choices[&id].disabled,
                        "interaction_only":true,"activate_on_press":true,"keyboard_navigation":false,"focus_follows_pointer":true}}));
                    self.choices.get_mut(&id).unwrap().widget = true;
                } else {
                    ui.push(json!({"op":"set_rect","id":id,"rect":rect}));
                    ui.push(json!({"op":"set_visible","id":id,"visible":visible}));
                    ui.push(
                        json!({"op":"set_enabled","id":id,"enabled":!self.choices[&id].disabled}),
                    );
                }
            } else if widget {
                ui.push(json!({"op":"set_visible","id":id,"visible":false}));
            }
        }
        self.submit(host, vec![], ui)
    }
    pub(super) fn choice_input(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        query: &str,
        input: &InputFrame,
    ) -> Result<bool> {
        let Some(id) = self
            .query(current, query)?
            .into_iter()
            .find_map(|id| self.choice_owner(id))
        else {
            return Ok(false);
        };
        if !self.choices[&id].widget || self.choices[&id].disabled {
            return Ok(false);
        }
        let interaction: WidgetInteraction = serde_json::from_slice(
            &host.query_engine(self.client, EngineQuery::UiInteraction(id))?,
        )?;
        if interaction.focused {
            let direction = [buttons::LEFT, buttons::UP, buttons::RIGHT, buttons::DOWN]
                .iter()
                .position(|button| input.buttons_pressed & button != 0);
            if let Some(target) = direction
                .and_then(|axis| self.choices[&id].next[axis])
                .filter(|target| {
                    self.choices
                        .get(target)
                        .is_some_and(|c| c.widget && !c.disabled)
                })
            {
                self.submit(host, vec![], vec![json!({"op":"focus","id":target})])?;
            }
        }
        let activated =
            interaction.activated && interaction.activation_serial != self.choices[&id].activation;
        if activated {
            self.choices.get_mut(&id).unwrap().activation = interaction.activation_serial;
        }
        let next = if interaction.pressed || activated {
            2
        } else if interaction.hovered || interaction.focused {
            1
        } else {
            0
        };
        let old = self.choices[&id].state;
        if next != old {
            self.choices.get_mut(&id).unwrap().state = next;
            let mut layers = Vec::new();
            for (index, name) in STATES[..3].iter().enumerate() {
                if let Some(group) = self.entities.child(id, name) {
                    layers.push(json!({"op":"set_visible","id":group,"visible":true}));
                    layers.push(json!({"op":"begin_motion","id":group,"track":{"property":"opacity","to":if index==next as usize{255}else{0},"duration":if next==2||index==2{0}else{200}}}));
                }
            }
            self.submit(host, layers, vec![])?;
            for (name, restart) in [("MouseOver", next == 1), ("MouseLeave", next == 0)] {
                if let Some(group) = self.entities.child(id, name) {
                    let children: Vec<_> = self
                        .entities
                        .nodes
                        .values()
                        .filter(|e| e.parent == Some(group) && matches!(e.kind, Kind::Thread))
                        .map(|e| e.id)
                        .collect();
                    for child in children {
                        if let Some(execution) = self.threads.get_mut(&child) {
                            if restart {
                                let entry = execution.entry.clone();
                                execution.thread = self.machine.start(
                                    &entry.module,
                                    Some(&entry.routine),
                                    execution.thread.name.clone(),
                                )?;
                                execution.enabled = true;
                                execution.wait = Wait::None;
                            } else {
                                execution.enabled = false;
                            }
                        }
                    }
                }
            }
        }
        Ok(activated)
    }
    pub(super) fn end_selection(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let ui = self
            .choices
            .iter()
            .filter(|(_, c)| c.widget)
            .map(|(id, _)| json!({"op":"blur","id":id}))
            .collect();
        self.submit(host, vec![], ui)
    }
}
