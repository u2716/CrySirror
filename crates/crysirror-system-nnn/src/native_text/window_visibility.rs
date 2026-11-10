// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original PrintMessage WindowOn flag, distinct from the saved global
//! messageWindowPrintFlag. Glyphs/windows retain resources and reveal clocks;
//! engine Layer visibility performs the hide/restore. Restore clicks never ACK.
use super::*;
impl NativeTextBridge {
    pub(crate) fn configure_window_controls(&mut self, allowed: bool) {
        self.window_toggle_enabled = allowed;
    }
    pub(crate) fn take_window_redraw(&mut self) -> bool {
        std::mem::take(&mut self.window_visibility_changed)
    }
    /// A new/saved layout keeps its normal visibility in RAM while the live
    /// window-off flag projects hidden Layers. No hidden-state bits are added
    /// to the original native save format.
    pub(super) fn project_window_visibility(
        &self,
        batch: &mut Value,
    ) -> Result<BTreeMap<u32, bool>> {
        let mut original = BTreeMap::new();
        if self.window_on {
            return Ok(original);
        }
        for node in batch
            .get_mut("layers")
            .and_then(Value::as_array_mut)
            .context("NNN text visibility layer batch")?
        {
            if node.get("op").and_then(Value::as_str) != Some("put") {
                continue;
            }
            let layer = node
                .get_mut("layer")
                .context("NNN text visibility layer spec")?;
            let id = u32::try_from(
                layer
                    .get("id")
                    .and_then(Value::as_u64)
                    .context("NNN text visibility layer identity")?,
            )?;
            original.insert(
                id,
                layer
                    .get("visible")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            );
            layer["visible"] = json!(false);
        }
        Ok(original)
    }
    pub(crate) fn set_window_on(&mut self, on: bool, host: &mut dyn HostServices) -> Result<()> {
        ensure!(
            !self.dependency_pending() && self.ready.is_none(),
            "NNN window visibility requires a completed native presentation"
        );
        if self.window_on == on {
            return Ok(());
        }
        let mut original = BTreeMap::new();
        let mut operations = Vec::new();
        let ids = if on {
            self.hidden_layers.keys().copied().collect()
        } else {
            self.restored_layer_ids()
        };
        for id in ids {
            let bytes = host.query_engine(self.profile.client, EngineQuery::Layer(id))?;
            self.api_calls += 1;
            let value: Value = serde_json::from_slice(&bytes)?;
            if value.is_null() {
                continue;
            }
            let layer: LayerSpec = serde_json::from_value(value)?;
            let visible = if on {
                *self.hidden_layers.get(&id).unwrap()
            } else {
                original.insert(id, layer.visible);
                false
            };
            operations.push(json!({"op":"set_visible","id":id,"visible":visible}));
        }
        if !operations.is_empty() {
            host.submit_engine(
                self.profile.client,
                1,
                &serde_json::to_vec(&json!({"layers":operations}))?,
            )?;
            self.api_calls += 1;
        }
        self.hidden_layers = original;
        self.window_on = on;
        self.window_visibility_changed = true;
        self.blocked_click_at = Some(host.now_ms());
        Ok(())
    }
    /// CalcuPrintMode tests window-off only after reveal completion. While
    /// revealing, ordinary next clicks may finish the hidden reveal, and an
    /// allowed native skip may still retire it. Once complete, either mouse
    /// click, Space or the configured mouse button restores without advance.
    pub(super) fn poll_window_controls(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        complete: bool,
    ) -> Result<bool> {
        use crysirror_core::service::buttons as B;
        if !complete {
            return Ok(false);
        }
        let edge = self.blocked_click_at != Some(host.now_ms());
        if !self.window_on {
            let restore = B::MOUSE_LEFT | B::MOUSE_RIGHT | B::SPACE | self.window_off_mouse;
            if edge && input.buttons_pressed & restore != 0 {
                self.set_window_on(true, host)?;
                self.disable_auto_requested = true;
            }
            // Voice-tail time still advances, while user/script advancement
            // is gated by the original window-off early return.
            let _ = self.timing.poll(host.now_ms(), false)?;
            return Ok(true);
        }
        if edge
            && self.window_toggle_enabled
            && !self.timing.script()
            && input.buttons_pressed & (B::SPACE | self.window_off_mouse) != 0
        {
            self.set_window_on(false, host)?;
            let _ = self.timing.poll(host.now_ms(), false)?;
            return Ok(true);
        }
        Ok(false)
    }
}
