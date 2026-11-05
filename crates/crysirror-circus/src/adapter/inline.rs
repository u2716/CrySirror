//! Scheduled41FBE0 commands are acknowledged only after their real resources
//! have been published. No text control is discarded as presentation markup.
use super::*;
use crate::machine::text_ops::InlineEvent;
impl Bridge<'_> {
    pub(super) fn native_inline_event(&mut self, event: &InlineEvent) -> Result<bool> {
        match event.kind {
            1..=10 => {
                let source = 19
                    + if event.kind <= 5 {
                        event.kind - 1
                    } else {
                        event.kind - 6
                    };
                let target = if event.kind <= 5 {
                    1
                } else {
                    u8::try_from(event.argument).context("Circus inline image target")?
                };
                ensure!(
                    crate::machine::visual_ops::resource_object(target).is_some(),
                    "Circus inline image target has no native object"
                );
                let image = self.services.images.get(&source).cloned();
                let extent = self.services.image_extents.get(&source).copied();
                let name = self
                    .services
                    .image_names
                    .get(&source)
                    .cloned()
                    .unwrap_or_default();
                if let Some(image) = image {
                    self.services.images.insert(target, image);
                } else {
                    self.services.images.remove(&target);
                }
                if let Some(extent) = extent {
                    self.services.image_extents.insert(target, extent);
                } else {
                    self.services.image_extents.remove(&target);
                }
                self.services.image_names.insert(target, name);
                let object = crate::machine::visual_ops::resource_object(target).unwrap();
                ensure!(
                    self.native_sprite_resources(
                        &crate::machine::sprite_ops::SpriteResourceRequest {
                            bindings: vec![crate::machine::sprite_ops::SpriteResourceBinding {
                                slot: target,
                                object,
                                crop: None
                            }]
                        }
                    )?
                    .is_some(),
                    "Circus inline texture bind acknowledgement"
                );
                self.services.displayed.insert(target);
                self.services.images.remove(&source);
                self.services.image_extents.remove(&source);
                //431F20 clears the source texture pointer, retaining its
                //filename.40EA70 separately marks the CG as seen.
                Ok(true)
            }
            11 => {
                if self.services.inline_music_stopped.as_ref() != Some(event) {
                    if !self.release_music(true, 30)? {
                        return Ok(false);
                    }
                    self.services.inline_music_stopped = Some(event.clone());
                }
                let Some(asset) = self.native_music(u16::try_from(event.argument)?, true)? else {
                    return Ok(false);
                };
                if !self.play_native_music(&asset, true, false)? {
                    return Ok(false);
                }
                self.services.inline_music_stopped = None;
                Ok(true)
            }
            12 => self.release_music(
                false,
                u32::try_from(event.argument).context("Circus inline music release divisor")?,
            ),
            13 => {
                let Some(name) = self.sound_name(u16::try_from(event.argument)?)? else {
                    return Ok(false);
                };
                Ok(self
                    .publish_native_sound(&crate::machine::media_ops::AudioDeclaration {
                        name,
                        voice: false,
                        looping: event.looping,
                        voice_volume: None,
                    })?
                    .is_some())
            }
            // Movement/masks and macros are issued by Machine with their
            //native state seeds; only transfer/audio events enter this bridge.
            kind => bail!(
                "Circus scheduled native inline effect{kind} requires its extracted engine operation"
            ),
        }
    }
}
