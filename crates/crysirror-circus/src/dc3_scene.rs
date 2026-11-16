//! DC3WY48/49/420370 scene-transfer policy. Prefetch is an owned shared
//! resource transaction; the old PC/state stays committed until it finishes.
use super::*;
static DEBUG_START: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
impl Machine {
    pub(super) fn execute_scene(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        match instruction.opcode {
            0x3a if a[0] == 0 => {
                return self.advance(instruction.next, instruction.opcode).map(Some);
            } //actual source early return
            0x48 | 0x49 => {}
            _ => return Ok(None),
        }
        //420260/41EB70 also finish/render native primary resources. Until
        //their producer declarations are connected, admit this path only
        //while every primary object and non-system slot is genuinely empty.
        //This supports the original bootstrap transfer without treating a
        //live DC3 actor as a DC4PH resource or discarding its queued work.
        ensure!(
            self.state
                .sprites
                .iter()
                .all(
                    //420260(1) for 0x49 neither finishes queues (43AE10) nor
                    //releases resources (41EB70): the scene continues over
                    //the live objects.
                    |(_, sprite)| instruction.opcode == 0x49 || !sprite.queued
                ),
            "DC3WY bound scene transfer requires its native primary-resource fence"
        );
        //Pending audio records are 4233A0 locals (zeroed only on entry);
        //48/49 do not yield, so they publish at the next dispatcher fence.
        if instruction.opcode == 0x49 {
            ensure!(
                self.state.builder.group.buttons.is_empty()
                    && self.state.builder.group.images.is_empty()
                    && self.state.builder.group.texts.is_empty()
                    && self.state.builder.pending_button.is_none(),
                "DC3WY retained-builder transfer requires its native owner projection"
            );
        }
        //Development smoke hook (not original behaviour): the first
        //transfer to MAIN goes to CIRCUS_DC3_START instead, once per process.
        let redirect = (text.eq_ignore_ascii_case("MAIN"))
            .then(|| std::env::var("CIRCUS_DC3_START").ok())
            .flatten()
            .filter(|_| !DEBUG_START.load(std::sync::atomic::Ordering::Relaxed));
        let text = redirect.as_deref().unwrap_or(text);
        ensure!(
            !text.is_empty() && text.len() <= 128 && !text.contains(['\\', '/', ':', '\0']),
            "DC3WY scene basename"
        );
        let Some(target) = host.scene(text)? else {
            return Ok(Some(Outcome::Pending));
        };
        ensure!(
            target.profile == Profile::Dc3WithYou,
            "DC3WY transfer changed bytecode ABI"
        );
        let Some(scene) = host.scene_index(&self.state.scene)? else {
            return Ok(Some(Outcome::Pending));
        };
        let mut data = self.state.system_data.clone();
        if scene >= 0 {
            ensure!(
                data.set_byte(98, scene as i16, 1)?,
                "DC3WY native scene-seen bank"
            );
        }
        let mut history = self.state.history.clone();
        if instruction.opcode == 0x49 || scene >= 0 {
            history.capture_variables(&self.state.script_bytes)?;
        }
        if instruction.opcode == 0x48 {
            history.native_28752 = history.native_28748;
        }
        history.label = 0;
        let mut pc = 3;
        ensure!(
            self.state.handlers.labels.len() == 16,
            "DC3WY handler extent"
        );
        let deferred_jump = &self.state.handlers.labels[12];
        if !deferred_jump.is_empty() {
            if let Some(label) = target
                .instructions
                .iter()
                .take_while(|item| item.opcode != 0)
                .find(|item| {
                    item.opcode == 0x44
                        && target.literal_bytes(item.offset) == Some(deferred_jump.as_slice())
                })
            {
                pc = label.offset;
            }
        }
        target.instruction(pc)?;
        let count = self
            .state
            .instructions
            .checked_add(1)
            .context("DC3WY instruction counter overflow")?;
        if instruction.opcode == 0x48 && !self.release_scene_resources(host)? {
            return Ok(Some(Outcome::Pending));
        }
        if instruction.opcode == 0x48 && !host.emit(&Event::ResetScene)? {
            return Ok(Some(Outcome::Pending));
        }
        self.state.system_data = data;
        self.state.history = history;
        self.state.audio.reset_envelopes(); //40D060, both48 and49
        if instruction.opcode == 0x48 {
            self.state.audio.last_voice.clear();
            self.state.builder = Builder::default();
            for sprite in self.state.sprites.values_mut() {
                sprite.resource = None;
            }
        }
        //Both clear the Buf+17872 name records outside 28..34. 0x48 has
        //released those textures; 0x49 keeps them loaded, so only their
        //saved names become empty.
        if instruction.opcode == 0x49 {
            self.state.nameless = self
                .state
                .image_names
                .iter()
                .filter(|(slot, _)| !(28..=34).contains(*slot))
                .map(|(slot, name)| (*slot, name.clone()))
                .collect();
        } else {
            self.state.nameless.clear();
            self.state
                .image_names
                .retain(|slot, _| (28..=34).contains(slot));
        }
        self.state.runtime_values.insert(245, -1);
        self.state.tint = 0; //420370 clears Buf23569
        //401ED0 only scans to '.' and writes a discarded local NUL. It
        //does not transform the stored scene name; do not invent strip_ext.
        self.state.scene = text.to_owned();
        self.state.pc = pc;
        self.state.instructions = count;
        self.state.last_offset = instruction.offset;
        self.state.last_opcode = Some(instruction.opcode);
        self.state.observations = None;
        //4233A0 clears the entire10332..11356 handler block after transfer.
        for handler in &mut self.state.handlers.labels {
            handler.clear();
        }
        if redirect.is_some() {
            DEBUG_START.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(Some(Outcome::Advanced))
    }
}
