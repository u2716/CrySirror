// SPDX-License-Identifier: MPL-2.0
//! Object picture stacks `0x2310..=0x2317` (CTexture +1216, 0x1400503a0..0x140050520).
//! Scripts build an object's picture from sixteen layers with the shared
//! [`LayerStack`] and replace the object image with the baked result (`0x2312`).
//! A baked picture has no file name, so a save keeps it as the original does: not at all.

use super::visual::{DecodedImage, Objects};
use anyhow::Result;
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_image::{layer_stack::LayerStack, ImageSurface};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Composites {
    stacks: BTreeMap<usize, LayerStack>,
}

fn surface(image: &DecodedImage) -> ImageSurface {
    let [width, height] = image.size();
    ImageSurface { width, height, rgba: image.rgba().to_vec() }
}

impl Composites {
    fn stack(&mut self, slot: usize, layer: usize) -> &mut LayerStack {
        // Layer 0 starts a new stack (0x140045360 and 0x1400452a0).
        if layer == 0 {
            self.stacks.remove(&slot);
        }
        self.stacks.entry(slot).or_default()
    }

    /// `0x2310` completed: a missing picture leaves the layer empty.
    pub fn place(&mut self, slot: usize, layer: usize, image: Option<&DecodedImage>, position: [i32; 2]) {
        let stack = self.stack(slot, layer);
        stack.release(layer);
        if let Some(image) = image {
            stack.place(layer, surface(image), image.has_alpha(), position);
        }
    }

    /// `0x2317`: an opaque black layer.
    pub fn solid(&mut self, slot: usize, layer: usize, size: [u32; 2], position: [i32; 2]) {
        self.stack(slot, layer).solid(layer, size, position);
    }

    /// `0x2316`: a copy of another object's picture.
    pub fn copy(&mut self, slot: usize, layer: usize, image: ImageSurface, alpha: bool, position: [i32; 2]) {
        self.stack(slot, layer).place(layer, image, alpha, position);
    }

    /// `0x2311`: layer 0 drops the stack.
    pub fn release(&mut self, slot: usize, layer: usize) {
        if layer == 0 {
            self.stacks.remove(&slot);
        } else if let Some(stack) = self.stacks.get_mut(&slot) {
            stack.release(layer);
        }
    }

    /// `0x2313..=0x2315` on an existing stack.
    pub fn adjust(&mut self, slot: usize, opcode: u16, layer: usize, values: [u32; 2]) {
        let Some(stack) = self.stacks.get_mut(&slot) else { return };
        match opcode {
            0x2313 => {
                stack.tint(layer, values[0]);
            }
            0x2314 => {
                stack.multiply(layer, values[0]);
            }
            _ => {
                stack.filter(layer, values[0], values[1]);
            }
        }
    }

    /// `0x2312` (0x140047460): the object shows the baked stack.
    pub fn bake(&mut self, host: &mut dyn HostServices, client: ClientId, objects: &mut Objects, slot: usize) -> Result<()> {
        let Some(baked) = self.stacks.get(&slot).and_then(LayerStack::bake) else {
            return Ok(());
        };
        let image = DecodedImage::from_surface(client, baked, Some(true))?;
        objects.image(host, client, slot, image)
    }

    pub fn clear(&mut self) {
        self.stacks.clear();
    }
}
