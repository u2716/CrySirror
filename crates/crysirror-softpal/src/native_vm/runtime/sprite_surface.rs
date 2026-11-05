use super::*;

// Game.exe 0x49C320. Entries 14/15 are reserved native values; PAL retains
// their backup but does not apply a recognized image transform.
const FILTER_OPTIONS: [u32; 16] = [
    0,
    1,
    2,
    3,
    4,
    0x105,
    0x205,
    0x305,
    0x405,
    0x505,
    0x605,
    0x705,
    0x805,
    0x905,
    u32::MAX,
    0x4a31d4,
];

impl ScriptRuntime {
    pub(super) fn ext_sprite_filter(
        &mut self,
        sprites: Option<&mut SpriteSystem>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(2);
        if args.len() != 2 {
            return ExtCallOutcome::Block;
        }
        let slot = args[0];
        if !(-1..=128).contains(&slot) {
            return ExtCallOutcome::Value(0);
        }
        let mode = if (0..16).contains(&args[1]) {
            args[1] as usize
        } else {
            0
        };
        if let Some(sprites) = sprites {
            for (&index, &handle) in &self.game_sprites {
                if (slot == -1 && (0..128).contains(&index)) || (slot != -1 && slot == index) {
                    sprites.set_option(handle, FILTER_OPTIONS[mode]);
                }
            }
        }
        ExtCallOutcome::Value(1)
    }

    pub(super) fn ext_sprite_paint(
        &mut self,
        assets: &CoreAssets,
        nls: Nls,
        sprites: Option<&mut SpriteSystem>,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(6);
        if args.len() != 6 {
            return ExtCallOutcome::Block;
        }
        let slot = args[0];
        if !(-1..=128).contains(&slot) {
            return ExtCallOutcome::Value(0);
        }
        let color = if args[1] == 0x0fff_ffff {
            0
        } else {
            self.resolve_resource_string(args[1], assets, nls)
                .and_then(|value| parse_paint_color(&value))
                .unwrap_or(0)
        };
        let rect = if args[2..].iter().all(|&v| v == 0) {
            None
        } else {
            Some(PalRect::new(args[2], args[3], args[4], args[5]))
        };
        if let (Some(sprites), Some(handle)) = (sprites, self.game_sprites.get(&slot).copied()) {
            // Save thumbnails/text are currently separate sprites above their
            // canvas. A full clear must remove these overlay representations.
            if rect.is_none() {
                self.clear_save_drawings(sprites, slot);
            }
            sprites.paint_rect(handle, rect, PalColor(color));
        }
        ExtCallOutcome::Value(1)
    }
}

fn parse_paint_color(value: &str) -> Option<u32> {
    // sub_4259C0 skips the resource marker and sub_44E370 parses raw hex.
    // Six digits do not imply an opaque alpha byte in this API.
    let hex = value.strip_prefix('#')?;
    let hex = hex
        .strip_prefix("0x")
        .or_else(|| hex.strip_prefix("0X"))
        .unwrap_or(hex);
    let digits = hex
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect::<String>();
    if digits.is_empty() {
        return Some(0);
    }
    Some(digits.bytes().fold(0u32, |value, c| {
        value
            .wrapping_mul(16)
            .wrapping_add((c as char).to_digit(16).unwrap())
    }))
}

pub(super) fn write_sprite_options(
    out: &mut Vec<u8>,
    sprites: &[SavedSprite],
) -> std::io::Result<()> {
    let sprites = sprites
        .iter()
        .filter(|sprite| sprite.rgba.len() <= CHECKPOINT_SPRITE_BYTES_CAP)
        .take(SAVE_SPRITE_CAP)
        .collect::<Vec<_>>();
    write_u32(out, sprites.len() as u32)?;
    for sprite in sprites {
        write_i32(out, sprite.slot)?;
        write_u32(out, sprite.option_type)?;
        write_bytes(out, &sprite.option_backup, CHECKPOINT_SPRITE_BYTES_CAP)?;
    }
    Ok(())
}

pub(super) fn read_sprite_options(
    reader: &mut impl Read,
    sprites: &mut [SavedSprite],
) -> std::io::Result<()> {
    let count = read_u32_from(reader)? as usize;
    if count != sprites.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "sprite option count mismatch",
        ));
    }
    for sprite in sprites {
        if read_i32_from(reader)? != sprite.slot {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "sprite option slot mismatch",
            ));
        }
        sprite.option_type = read_u32_from(reader)?;
        let original = read_bytes_capped(reader, CHECKPOINT_SPRITE_BYTES_CAP)?;
        if !original.is_empty() && original.len() != sprite.rgba.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "sprite option dimensions mismatch",
            ));
        }
        sprite.option_backup = original;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_all_only_touches_native_slots_and_invalid_mode_restores() {
        let mut vm = ScriptRuntime::boot(0, ScriptRuntimeConfig::default());
        let mut sprites = SpriteSystem::new();
        for slot in [-1, 0, 127, 128] {
            let handle = sprites
                .create_rgba_sprite(
                    1,
                    1,
                    vec![100, 150, 200, 255],
                    PalVec3::default(),
                    0,
                    "test",
                )
                .unwrap();
            vm.game_sprites.insert(slot, handle);
        }
        vm.stack.extend([2, -1]);
        assert!(matches!(
            vm.ext_sprite_filter(Some(&mut sprites)),
            ExtCallOutcome::Value(1)
        ));
        for (&slot, &handle) in &vm.game_sprites {
            let id = sprites.get(handle).unwrap().surface;
            assert_eq!(
                sprites.surface(id).unwrap().pixels[0],
                if (0..128).contains(&slot) { 140 } else { 100 }
            );
        }
        vm.stack.extend([-1, 0]);
        vm.ext_sprite_filter(Some(&mut sprites));
        let id = sprites.get(vm.game_sprites[&0]).unwrap().surface;
        assert_eq!(&*sprites.surface(id).unwrap().pixels, &[100, 150, 200, 255]);
    }

    #[test]
    fn paint_colors_keep_raw_alpha_and_hex_prefix_rules() {
        assert_eq!(parse_paint_color("#112233"), Some(0x00112233));
        assert_eq!(parse_paint_color("#0x80402010"), Some(0x80402010));
        assert_eq!(parse_paint_color("#AABBCCDD stop"), Some(0xaabbccdd));
        assert_eq!(parse_paint_color("#"), Some(0));
    }

    #[test]
    fn saved_texture_readers_reject_overflow_and_mismatched_pixels() {
        for (width, height) in [(0x8000_0000u32, 0x8000_0000u32), (2, 2), (0, 1)] {
            // Version 2 records contain no projection or cell-window suffix.
            let mut sprite = Vec::new();
            write_u32(&mut sprite, 1).unwrap();
            for _ in 0..9 {
                write_u32(&mut sprite, 0).unwrap();
            }
            sprite.push(1);
            for _ in 0..4 {
                write_u32(&mut sprite, 0).unwrap();
            }
            write_u32(&mut sprite, width).unwrap();
            write_u32(&mut sprite, height).unwrap();
            write_u32(&mut sprite, 0).unwrap();
            assert_eq!(
                read_saved_sprites(&mut sprite.as_slice(), 2)
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::InvalidData
            );

            let mut button = Vec::new();
            write_u32(&mut button, 1).unwrap();
            for _ in 0..2 {
                write_u32(&mut button, 0).unwrap();
            }
            button.extend_from_slice(&[1, 1, 255]);
            for _ in 0..5 {
                write_u32(&mut button, 0).unwrap();
            }
            write_u32(&mut button, width).unwrap();
            write_u32(&mut button, height).unwrap();
            write_u32(&mut button, 0).unwrap();
            write_u32(&mut button, 0).unwrap();
            assert_eq!(
                read_saved_buttons(&mut button.as_slice(), 2)
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn filtered_save_restores_pixels_and_keeps_reset_backup() {
        let mut vm = ScriptRuntime::boot(12, ScriptRuntimeConfig::default());
        let mut sprites = SpriteSystem::new();
        let handle = sprites
            .create_rgba_sprite(
                2,
                2,
                [100, 150, 200, 255].repeat(4),
                PalVec3::default(),
                0,
                "test",
            )
            .unwrap();
        vm.game_sprites.insert(0, handle);
        sprites.set_option(handle, 2);
        let snapshot = RuntimeSaveSnapshot {
            sprites: vm.capture_sprite_records(&sprites),
            ..vm.capture_save_snapshot()
        };
        let bytes = encode_runtime_save_snapshot(&snapshot).unwrap();
        let snapshot = decode_runtime_save_snapshot(&mut bytes.as_slice()).unwrap();
        vm.restore_checkpoint_scene(&snapshot, &mut sprites);
        let handle = vm.game_sprites[&0];
        let id = sprites.get(handle).unwrap().surface;
        assert_eq!(
            &sprites.surface(id).unwrap().pixels[..4],
            &[140, 140, 140, 255]
        );
        sprites.set_option(handle, 0);
        assert_eq!(
            &sprites.surface(id).unwrap().pixels[..4],
            &[100, 150, 200, 255]
        );
    }
}
