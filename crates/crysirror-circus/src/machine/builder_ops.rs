//!4284B0's v488 selects one of three native builder objects. Lists, pending
//!button, offsets and flags belong to that object; selecting another does not
//!finalize or discard the previous builder. Rendering remains one host UI API.
use super::*;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct BuilderContext {
    group: Group,
    button: Option<Button>,
    offset: [i16; 2],
}
pub(super) fn deserialize_scope<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u8, D::Error> {
    let value = u8::deserialize(deserializer)?;
    if value > 2 {
        return Err(serde::de::Error::custom(
            "Circus builder namespace must be0..2",
        ));
    }
    Ok(value)
}
impl State {
    pub(super) fn selected_builder(&self) -> u8 {
        self.selected_builder
    }
    pub(super) fn select_builder(&mut self, scope: u8) {
        debug_assert!(scope < 3 && self.selected_builder < 3);
        if scope == self.selected_builder {
            return;
        }
        self.builder_contexts[usize::from(self.selected_builder)] = BuilderContext {
            group: std::mem::take(&mut self.builder),
            button: self.current_button.take(),
            offset: self.builder_offset,
        };
        let selected = std::mem::take(&mut self.builder_contexts[usize::from(scope)]);
        self.builder = selected.group;
        self.current_button = selected.button;
        self.builder_offset = selected.offset;
        self.selected_builder = scope;
    }
    pub(super) fn finalize_builders(&mut self, all: bool) {
        if !all {
            self.finish_button();
            return;
        }
        let selected = self.selected_builder;
        for scope in 0..3 {
            self.select_builder(scope);
            self.finish_button();
        }
        self.select_builder(selected);
    }
    pub(super) fn place_side_builders(&mut self) {
        let selected = self.selected_builder;
        //412B00 only assigns232/233; it does not erase movement declarations.
        for (scope, x) in [(1, 1280), (2, -1280)] {
            self.select_builder(scope);
            self.builder.position = [x, 0];
        }
        self.select_builder(selected);
    }
}
impl Machine {
    pub(super) fn scoped_ui(&self, event: Event) -> Event {
        Event::UiContext {
            context: self.state.selected_builder(),
            event: Box::new(event),
        }
    }
    pub(super) fn begin_all_builder_menus(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        self.state.finalize_builders(true);
        self.state.place_side_builders();
        let selected = self.state.selected_builder();
        let pc = self.state.pc;
        let cursor = if let Some((owner, cursor)) = self.state.builder_menu_progress {
            ensure!(owner == pc, "Circus builder finalize instruction ownership");
            cursor
        } else {
            self.state.builder_menu_progress = Some((pc, 0));
            0
        };
        for scope in cursor..3 {
            self.state.select_builder(scope);
            let mut group = self.state.builder.clone();
            self.resolve_group_values(host, &mut group)?;
            if !host.emit(&self.scoped_ui(Event::BeginMenu(group)))? {
                self.state.select_builder(selected);
                return Ok(false);
            }
            self.state.builder_menu_progress = Some((pc, scope + 1));
        }
        self.state.select_builder(selected);
        if !self.finalize_panel_owner(host, 0)? {
            return Ok(false);
        }
        self.state.builder_menu_progress = None;
        Ok(true)
    }
    pub(super) fn finalize_panel_owner(
        &mut self,
        host: &mut dyn NativeHost,
        phase: u8,
    ) -> Result<bool> {
        let declarations = if self.state.dialogue.panel.has_viewport() {
            self.state.dialogue.panel.finalized()?
        } else {
            self.state.dialogue.panel.clone()
        };
        if !host.emit(&Event::FinalizePanel {
            declarations: declarations.clone(),
            scroll: self.state.value(99, 246)?,
            selected: self.state.value(99, 254)?,
            focus: self.state.value(99, 245)?,
            ticket: (self.state.instructions, phase),
        })? {
            return Ok(false);
        }
        self.state.dialogue.panel = declarations;
        Ok(true)
    }
    pub(super) fn builder_stage(&mut self, pc: u32) -> Result<u8> {
        if let Some((owner, stage)) = self.state.builder_instruction_stage {
            ensure!(owner == pc, "Circus builder command retry ownership");
            Ok(stage)
        } else {
            self.state.builder_instruction_stage = Some((pc, 0));
            Ok(0)
        }
    }
    pub(super) fn set_builder_stage(&mut self, pc: u32, stage: u8) {
        self.state.builder_instruction_stage = Some((pc, stage));
    }
    pub(super) fn finish_builder_command(&mut self) {
        self.state.builder_instruction_stage = None;
    }
}
