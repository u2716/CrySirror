//! Recovered PAL D3D blend modes, shared with portable software rendering.
//! Legacy PAL recognizes 1..4; newer PAL adds 5..11. Sprite adapters resolve
//! their build profile before emitting a backend-neutral draw command.

pub fn mode_index(mode: u32) -> usize {
    if (1..=11).contains(&mode) {
        mode as usize
    } else {
        1
    }
}

pub fn modulation(mode: u32, color: [f32; 4]) -> [f32; 4] {
    NativeDrawState::default().for_mode(mode).modulation(color)
}

/// The newer D3D callback restores blend factors after a draw, but mode 11
/// leaves ALPHABLENDENABLE and stage-0 COLOROP changed. These are independent
/// of the following sprite's mode word and survive ordinary render walks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeDrawState {
    pub blend_disabled: bool,
    pub texture_rgb: bool,
}

impl NativeDrawState {
    pub fn for_mode(mut self, mode: u32) -> Self {
        if mode == 11 {
            self.blend_disabled = true;
            self.texture_rgb = true;
        }
        self
    }

    pub fn modulation(self, color: [f32; 4]) -> [f32; 4] {
        if self.texture_rgb {
            [1.0, 1.0, 1.0, color[3]]
        } else {
            color
        }
    }

    pub fn blend_rgb(self, mode: u32, dst: [u8; 3], src: [f32; 4]) -> [u8; 3] {
        if self.blend_disabled {
            std::array::from_fn(|i| src[i].round().clamp(0.0, 255.0) as u8)
        } else {
            blend_rgb(mode, dst, src)
        }
    }

    /// Native D3D uses the same blend operation/factors for alpha while
    /// SEPARATEALPHABLENDENABLE is disabled. Target capture preserves it.
    pub fn blend_alpha(self, mode: u32, destination: u8, source: f32) -> u8 {
        if self.blend_disabled {
            source.round().clamp(0.0, 255.0) as u8
        } else {
            blend_rgb(mode, [destination; 3], [source; 4])[0]
        }
    }

    pub fn pipeline_index(self, mode: u32) -> usize {
        if self.blend_disabled {
            11
        } else {
            mode_index(mode)
        }
    }

    /// Ordinary PalRenderListDraw does not reset these states between nodes
    /// or at frame boundaries. Call this after native priority sorting.
    pub fn apply_main_list(&mut self, commands: &mut [super::DrawCommand]) {
        for command in commands {
            if let super::DrawCommand::Sprite(sprite) = command {
                *self = self.for_mode(sprite.render_mode);
                sprite.blend_state = *self;
            }
        }
    }

    /// PalTargetRenderListDraw calls the default-state helper before every
    /// callback and once more after the walk, including an empty target list.
    pub fn apply_target_list(&mut self, commands: &mut [super::DrawCommand]) {
        for command in commands {
            *self = Self::default();
            self.apply_main_list(std::slice::from_mut(command));
        }
        *self = Self::default();
    }
}

pub fn blend_rgb(mode: u32, dst: [u8; 3], src: [f32; 4]) -> [u8; 3] {
    let alpha = (src[3] / 255.0).clamp(0.0, 1.0);
    std::array::from_fn(|i| {
        let s = src[i];
        let d = f32::from(dst[i]);
        let result = match mode {
            2 => s * alpha + d,
            3 | 5 => d * s / 255.0,
            4 => d * d / 255.0,
            6 => s * (1.0 - d / 255.0) + d,
            7 => s.min(d),
            8 => s.max(d),
            9 => s + d,
            10 => d - s,
            11 => s,
            _ => s * alpha + d * (1.0 - alpha),
        };
        result.round().clamp(0.0, 255.0) as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_vm::scene::{
        rasterize_scene_rgba, DrawCommand, FrameScene, SceneTexture, SceneTextureId, SpriteDraw,
    };

    fn sequence(modes: &[u32]) -> FrameScene {
        let texture = SceneTexture::rgba8(SceneTextureId(1), 1, 1, 1, vec![200, 100, 30, 128]);
        let mut scene = FrameScene::boot().with_logical_size(modes.len() as u32, 1);
        scene.clear_color = [80.0 / 255.0, 40.0 / 255.0, 160.0 / 255.0, 1.0];
        for (x, mode) in modes.iter().copied().enumerate() {
            let mut sprite = SpriteDraw::full_frame(&texture, 1, 1);
            sprite.dst.x = x as f32;
            sprite.render_mode = mode;
            sprite.color = [0.5, 0.25, 0.75, 0.0];
            scene.commands.push(DrawCommand::Sprite(sprite));
        }
        scene.textures.push(texture);
        scene
    }

    #[test]
    fn native_main_walk_keeps_mode_eleven_state_across_modes_and_frames() {
        let mut state = NativeDrawState::default();
        let mut scene = sequence(&[11, 1, 9]);
        state.apply_main_list(&mut scene.commands);
        assert_eq!(rasterize_scene_rgba(&scene), [200, 100, 30, 255].repeat(3));
        for (command, mode) in scene.commands.iter().zip([11, 1, 9]) {
            let DrawCommand::Sprite(sprite) = command else {
                panic!("sprite");
            };
            assert_eq!(sprite.render_mode, mode);
            assert!(sprite.blend_state.blend_disabled);
            assert!(sprite.blend_state.texture_rgb);
        }
        let mut next_frame = sequence(&[4]);
        state.apply_main_list(&mut next_frame.commands);
        assert_eq!(rasterize_scene_rgba(&next_frame), [200, 100, 30, 255]);
    }

    #[test]
    fn native_target_walk_resets_before_each_callback_and_after_empty_lists() {
        let mut state = NativeDrawState::default().for_mode(11);
        let mut scene = sequence(&[11, 1, 9]);
        state.apply_target_list(&mut scene.commands);
        assert_eq!(
            rasterize_scene_rgba(&scene),
            [200, 100, 30, 255, 80, 40, 160, 255, 180, 65, 183, 255]
        );
        assert_eq!(state, NativeDrawState::default());
        state = state.for_mode(11);
        state.apply_target_list(&mut []);
        let mut normal = sequence(&[4]);
        state.apply_main_list(&mut normal.commands);
        assert_eq!(rasterize_scene_rgba(&normal), [25, 6, 100, 255]);
    }

    #[test]
    fn draw_flags_preserve_separate_color_and_blend_states() {
        let tint = [0.5, 0.25, 0.75, 0.0];
        let state = NativeDrawState {
            blend_disabled: true,
            texture_rgb: false,
        };
        assert_eq!(state.modulation(tint), tint);
        assert_eq!(
            state.blend_rgb(4, [80, 40, 160], [100.0, 25.0, 22.5, 0.0]),
            [100, 25, 23]
        );
        let state = NativeDrawState {
            blend_disabled: false,
            texture_rgb: true,
        };
        assert_eq!(state.modulation(tint), [1.0, 1.0, 1.0, 0.0]);
        assert_eq!(
            state.blend_rgb(1, [80, 40, 160], [200.0, 100.0, 30.0, 0.0]),
            [80, 40, 160]
        );
    }

    #[test]
    fn native_d3d_mode_equations_cover_all_operations() {
        let expected = [
            [140, 70, 95],
            [180, 90, 175],
            [63, 16, 19],
            [25, 6, 100],
            [63, 16, 19],
            [217, 124, 171],
            [80, 40, 30],
            [200, 100, 160],
            [255, 140, 190],
            [0, 0, 130],
            [200, 100, 30],
        ];
        for mode in 1..=11 {
            assert_eq!(
                blend_rgb(mode, [80, 40, 160], [200.0, 100.0, 30.0, 127.5]),
                expected[mode as usize - 1],
                "mode {mode}"
            );
        }
        assert_eq!(
            blend_rgb(99, [80, 40, 160], [200.0, 100.0, 30.0, 127.5]),
            expected[0]
        );
        assert_eq!(blend_rgb(4, [80, 40, 160], [0.0; 4]), expected[3]);
    }
}
