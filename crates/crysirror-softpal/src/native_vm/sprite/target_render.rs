//! Ordered borrowed-object collection and temporary render capture used by
//! the newer EXE's GRAPH_CHILD_MODE_RENDER_LIST_ADD/RUN operations.

use super::{DrawCommand, SceneTexture, SceneTextureId, SpriteHandle, SpriteSystem};
use crate::native_vm::scene::{rasterize_target_rgba, FrameScene};

#[derive(Debug)]
pub(super) struct TargetRenderList {
    objects: Vec<Option<SpriteHandle>>,
    size: (u32, u32),
    active: bool,
    suspended: bool,
    capture: Option<SceneTexture>,
}

impl Default for TargetRenderList {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            size: (
                FrameScene::PAL_DEFAULT_WIDTH,
                FrameScene::PAL_DEFAULT_HEIGHT,
            ),
            active: true,
            suspended: false,
            capture: None,
        }
    }
}

impl SpriteSystem {
    pub fn target_render_list_objects(&self) -> &[Option<SpriteHandle>] {
        &self.target_render.objects
    }

    pub fn target_render_capture(&self) -> Option<&SceneTexture> {
        self.target_render.capture.as_ref()
    }

    pub fn restore_target_render_capture(&mut self, capture: Option<SceneTexture>) {
        self.target_render.capture = capture;
    }

    pub fn native_draw_state(&self) -> crate::native_vm::scene::blend::NativeDrawState {
        self.native_draw_state
    }

    pub fn restore_native_draw_state(&mut self, state: crate::native_vm::scene::blend::NativeDrawState) {
        self.native_draw_state = state;
    }

    pub fn configure_target_render_size(&mut self, width: u32, height: u32) {
        self.target_render.size = (width.max(1), height.max(1));
    }

    /// Host-side active-list/suspension context, separate from the zero-argument
    /// native Draw ABI. A SpriteSystem ordinarily owns one active render list.
    pub fn configure_target_render_state(&mut self, active: bool, suspended: bool) {
        self.target_render.active = active;
        self.target_render.suspended = suspended;
    }

    pub fn resolve_main_draw_state(&mut self, commands: &mut [DrawCommand]) {
        self.native_draw_state.apply_main_list(commands);
    }

    /// Borrow the sprite identity, retaining duplicates and null entries.
    /// The native Add return type remains unresolved; this is an arena adapter.
    pub fn target_render_list_add(&mut self, object: Option<SpriteHandle>) {
        self.target_render.objects.push(object);
    }

    /// Clear borrowed entries without releasing sprites or the last capture.
    pub fn target_render_list_clear(&mut self) {
        self.target_render.objects.clear();
    }

    pub fn target_render_list_draw(&mut self) -> i32 {
        if self.target_render.suspended {
            return 1;
        }
        if !self.target_render.active {
            return 0;
        }
        let (width, height) = self.target_render.size;
        let mut scene = FrameScene::boot().with_logical_size(width, height);
        scene.clear_color = [0.0; 4];
        scene.diagnostic_label = "PAL temporary target".to_owned();
        let mut stale_nodes = Vec::new();
        for object in self.target_render.objects.iter().flatten() {
            for node in self
                .render_nodes
                .values()
                .filter(|node| node.sprite == *object)
            {
                let Some(sprite) = self.sprites.get(object) else {
                    // Native sprite callback returns zero for a null object;
                    // stale arena nodes likewise cease participating in draws.
                    stale_nodes.push(node.id);
                    continue;
                };
                if sprite.transition_block != 0 || sprite.locked {
                    continue;
                }
                // Temporary-render mode bypasses visibility, but does not
                // bypass transition ownership. Preserve collection order.
                if let Some(command) = sprite.draw_command_inner(self) {
                    scene.commands.push(command);
                }
            }
        }
        for node in stale_nodes {
            self.render_nodes.remove(&node);
        }
        self.native_draw_state
            .apply_target_list(&mut scene.commands);
        let used: std::collections::HashSet<_> = scene
            .commands
            .iter()
            .filter_map(|command| {
                if let DrawCommand::Sprite(sprite) = command {
                    Some(sprite.texture_id)
                } else {
                    None
                }
            })
            .collect();
        scene
            .textures
            .extend(self.textures().filter(|texture| used.contains(&texture.id)));
        let pixels = rasterize_target_rgba(&scene);
        let generation = self
            .target_render
            .capture
            .as_ref()
            .map_or(1, |capture| capture.generation.wrapping_add(1));
        self.target_render.capture = Some(SceneTexture::rgba8(
            SceneTextureId(0),
            generation,
            width,
            height,
            pixels,
        ));
        1
    }

    /// PalSpriteRenderCopy returns success for a valid sprite even when its
    /// target cannot be copied. RGB and alpha are copied without modulation.
    pub fn render_copy(&mut self, destination: SpriteHandle) -> i32 {
        let Some(surface_id) = self.get(destination).map(|sprite| sprite.surface) else {
            return 0;
        };
        let Some(capture) = self.target_render.capture.as_ref().cloned() else {
            return 1;
        };
        let Some(surface) = self.surface_mut(surface_id) else {
            return 1;
        };
        if surface.width < capture.width || surface.height < capture.height {
            return 1;
        }
        let width = surface.width as usize;
        let left = (surface.width - capture.width) as usize / 2;
        let top = (surface.height - capture.height) as usize / 2;
        let pixels = surface.pixels_mut();
        pixels.fill(0);
        for y in 0..capture.height as usize {
            let source = y * capture.width as usize * 4;
            let destination = ((y + top) * width + left) * 4;
            let length = capture.width as usize * 4;
            pixels[destination..destination + length]
                .copy_from_slice(&capture.pixels[source..source + length]);
        }
        1
    }
}
