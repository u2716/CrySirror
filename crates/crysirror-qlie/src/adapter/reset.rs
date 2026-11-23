//! sys `r,label,file` (6D9434 -> request 8 -> 707F80 -> 707A30): reset the
//! engine and continue at `label` in `file` (defaults `@@reset` and
//! `startup.s`).
//!
//! 707A30 clears both readers (6D5D38), the object registry (5D395C), the
//! mixer (5C4490), the two effect lists (5D59E0) and the XML store, then
//! executes the jump on the game reader. SystemData (svar), registry
//! entries, read-line flags and process-wide strings belong to the engine
//! process and survive; host resources of every object are released.
use super::*;

impl QlieVm {
    pub(super) fn reset_engine(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        label: &str,
        file: &str,
    ) -> Result<()> {
        let label = if label.is_empty() { "@@reset" } else { label };
        let file = if file.is_empty() { "startup.s" } else { file };
        // A system pause holds the game graphic; bring it back so that its
        // objects are released with everything else.
        if let Some(saved) = self.suspended_host.take() {
            self.discard_system_objects(h)?;
            self.host_thread = ThreadKind::Game;
            self.restore_host_context(saved, h.now_ms());
        }
        self.host_thread = ThreadKind::Game;
        self.effect_reset(h)?;
        let ids: Vec<String> = self.objects.keys().cloned().collect();
        for id in ids {
            if let Some(o) = self.objects.remove(&id) {
                self.free_object(h, o)?;
            }
        }
        for (_, layer) in std::mem::take(&mut self.anchor_proxies) {
            self.submit(h, json!({"layers":[{"op":"remove","id":layer}]}))?;
        }
        let system_root = self.root_layer_for(ThreadKind::System);
        self.submit(h, json!({"layers":[
            {"op":"set_clock_paused","id":1,"paused":false},
            {"op":"set_visible","id":1,"visible":true},
            {"op":"set_visible","id":system_root,"visible":false}
        ]}))?;
        m.reset_to(file, label)?;
        let mut fresh = QlieVm::new(self.client, self.config.clone())?;
        std::mem::swap(&mut fresh.pool, &mut self.pool);
        fresh.text = std::mem::take(&mut self.text);
        fresh.pack_slots = std::mem::take(&mut self.pack_slots);
        fresh.random = self.random.take();
        fresh.math_random = self.math_random.take();
        fresh.system_data = std::mem::take(&mut self.system_data);
        fresh.registry = std::mem::take(&mut self.registry);
        fresh.read_lines = std::mem::take(&mut self.read_lines);
        fresh.global_strings = std::mem::take(&mut self.global_strings);
        fresh.boot_settings = self.boot_settings.take();
        fresh.unavailable = std::mem::take(&mut self.unavailable);
        fresh.api_calls = self.api_calls;
        // Layer, image and media identities stay unique for the host.
        fresh.next_layer = self.next_layer;
        fresh.next_image = self.next_image;
        fresh.media_ids = std::mem::take(&mut self.media_ids);
        fresh.initialized = self.initialized;
        fresh.system_root = self.system_root;
        fresh.input = self.input.clone();
        fresh.last_tick_ms = self.last_tick_ms;
        *self = fresh;
        Ok(())
    }
}
