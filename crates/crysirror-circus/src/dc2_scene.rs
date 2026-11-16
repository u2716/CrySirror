//! DC2 41C530 case46,422E60 process46 and41A050. A scene read is an
//! asynchronous engine resource receipt; native PC/history/flags commit only
//! after the target program and original release operations are ready.
use super::*;
use crate::machine::{
    Event,
    resource_ops::{ResourceReply, ResourceRequest},
    sprite_ops::{SpriteResourceBinding, SpriteResourceRequest},
};
impl Machine {
    pub(super) fn execute_scene(
        &mut self,
        instruction: &Instruction,
        selector: i16,
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x2e {
            return Ok(None);
        }
        ensure!(
            !text.is_empty() && text.len() < 128 && !text.contains(['\\', '/', ':', '\0']),
            "DC2 native scene basename"
        );
        //A screen frozen by 0x30 (capture without a running transition) stays
        //presented across the transfer, as the native surface does; the next
        //scene's presentation consumes it.
        ensure!(
            self.state.resources.transition.is_none(),
            "DC2 scene transfer before original bitmap completion"
        );
        let mode = match selector {
            4 => 2,
            3 | 5 => 3,
            _ => 1,
        };
        let Some(target) = host.scene(text)? else {
            return Ok(Some(Outcome::Pending));
        };
        ensure!(
            target.profile == Profile::Dc2PlusCommunication,
            "DC2 scene transfer changed native ABI"
        );
        let stem = |name: &str| name.split('.').next().unwrap_or("").to_owned();
        let Some(previous) = host.scene_index(&stem(&self.state.scene))? else {
            return Ok(Some(Outcome::Pending));
        };
        let Some(incoming) = host.scene_index(&stem(text))? else {
            return Ok(Some(Outcome::Pending));
        };
        let mut settings = self.state.settings.clone();
        let mut completed = self.state.scene_completed.clone();
        //The immediate selector4 path does not invoke425790 on the old scene.
        if selector != 4 && previous >= 0 {
            ensure!(previous < 1500, "DC2 completed native scene index");
            settings.set_byte(98, previous as i16, 1)?;
            completed[previous as usize] = 1;
        }
        //425810 executes after425790 in the deferred path. A self-transfer
        //therefore observes the newly written read bit, not its old value.
        let incoming_read = if incoming < 0 {
            0
        } else {
            i32::from(
                settings
                    .byte(98, incoming as i16)?
                    .context("DC2 incoming read bank")?,
            )
        };
        let target_pc = if self.state.scene_jump.is_empty() {
            2
        } else {
            named_label(&target, &self.state.scene_jump)?
        };
        if mode == 1 || selector == 5 {
            //419B60(-1/-2) releases native0..8,10, retaining system11..14.
            //Native0 is mapped to host1; no implemented producer owns native5.
            for slot in [1, 2, 3, 4, 5, 6, 7, 8, 10] {
                match host.resource(&ResourceRequest::Unload { slot })? {
                    None => return Ok(Some(Outcome::Pending)),
                    Some(ResourceReply::Done) => {}
                    Some(_) => bail!("DC2 scene release foreign receipt"),
                }
            }
            //The screen bound by the feedback compositor is transient, separate
            //from the encoded native0 texture. Clear its drawable binding too.
            if host
                .sprite_resources(&SpriteResourceRequest {
                    bindings: vec![SpriteResourceBinding {
                        slot: 1,
                        object: 0,
                        crop: None,
                    }],
                })?
                .is_none()
            {
                return Ok(Some(Outcome::Pending));
            }
        }
        if !host.emit(&Event::ClearUi)? {
            return Ok(Some(Outcome::Pending));
        }
        self.state.settings = settings;
        self.state.scene_completed = completed;
        self.state.builder.count = 0;
        self.state.runtime.insert(252, -1);
        self.state.runtime.insert(244, -1);
        self.state.history_count = 0;
        self.state.history[0] = 0;
        self.state.current_label = 0;
        if mode != 2 {
            self.state.runtime.insert(251, incoming_read);
            self.state.runtime.insert(313, 0);
            if incoming_read == 0 {
                self.state.runtime.insert(242, 0);
            }
        }
        if mode == 1 || selector == 5 {
            self.state.resources.bitmap_discard_pending = true;
            self.state.primary_names = Default::default();
            self.state.primary_ids = [-1; 5];
            self.state
                .resources
                .system_names
                .retain(|slot, _| *slot >= 11);
            self.state.draw.planes.iter_mut().for_each(Vec::clear);
            self.state.resources.display_base.clear();
            self.state.draw.native_composition_pending = false;
        }
        self.state.scene_jump.clear();
        self.state.other_jump.clear();
        self.advance(instruction.next, instruction.opcode)?;
        self.state.scene = text.into();
        self.state.pc = target_pc;
        Ok(Some(Outcome::Advanced))
    }
}
