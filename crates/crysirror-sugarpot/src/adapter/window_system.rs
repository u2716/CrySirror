// SPDX-License-Identifier: MPL-2.0
//!433310/4383B0 system-sprite window policy, independent of their resources.
use super::*;
impl SugarPotVm {
    pub(super) fn system_window_layers(&self, visible: bool) -> Result<Vec<Json>> {
        let mut layers = Vec::new();
        //Native sprite0 is always the base, even when its SYS byte is nonzero.
        for (index, sprite) in self.sprites.iter().enumerate().skip(1) {
            if sprite.system == 0 {
                continue;
            }
            let id = index as u32 + 1;
            if visible {
                if let Some(mut layer) = Self::sprite_layer(id, sprite, 2000 + index as i32)? {
                    if sprite.system & 0x7f == 2 {
                        //43F580 overrides the caller's opacity for SYS2 with
                        //global4654. Normal native settings are0..100; bound
                        //foreign preference values before public admission.
                        layer["layer"]["opacity"] = json!(Self::sprite_opacity(
                            sprite,
                            false,
                            255 * self.settings.values[10].clamp(0, 100) / 100
                        ));
                    }
                    layers.push(layer);
                    continue;
                }
            }
            // Only a published system sprite has a layer to withdraw; a
            // sprite without an image (or never shown) has none.
            if self.published_layers.contains(&id) {
                layers.push(json!({"op":"remove","id":id}));
            }
        }
        Ok(layers)
    }
    pub(super) fn track_system_window_layers(&mut self, visible: bool, hide_flag: bool) {
        for (index, sprite) in self.sprites.iter_mut().enumerate().skip(1) {
            if sprite.system == 0 {
                continue;
            }
            let id = index as u32 + 1;
            self.published_layers.retain(|old| *old != id);
            if visible {
                //433310(0) clears the signed suppression bit. Its value is
                //native saved state, distinct from an effect's window state2.
                sprite.system &= 0x7f;
                if sprite.visible && sprite.image.is_some() {
                    self.published_layers.push(id);
                }
            } else if hide_flag && sprite.visible {
                sprite.system = (sprite.system as u8 | 0x80) as i8 as i32;
            }
        }
    }
}
