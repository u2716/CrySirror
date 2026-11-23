//! Koikake original save reader, scoped to the exact audited PAL/Game pair.
//!
//! Stream order comes from Game.exe 0x424530/0x424EA0 and their paired
//! subsystem readers/writers in reverse/evidence/koikake-save-subsystems.json.
//! Native address words are consumed as data, never dereferenced.

use super::*;
use crate::native_vm::save_format::PalSpriteInfoKoikake32;

const NATIVE_SAVE_CAP: usize = 2 * 1024 * 1024;
const NATIVE_HISTORY_COUNT: usize = 128;
const NATIVE_HISTORY_STRIDE: usize = 542;

#[derive(Clone)]
struct NativeSprite {
    resource: i32,
    info: PalSpriteInfoKoikake32,
    wrapper: [u8; 92],
    name: Option<[u8; 32]>,
}

struct NativeButton {
    group: i32,
    index: i32,
    control: [u32; 4],
    sprite: NativeSprite,
}

struct NativeWindow {
    fields: [i32; 9],
    base: Option<NativeSprite>,
}

struct NativeImage {
    user_memory: Vec<i32>,
    string_cursor: usize,
    strings: Vec<[u8; 255]>,
    stack: Vec<i32>,
    argument_base: i32,
    arguments: Vec<i32>,
    buttons: Vec<NativeButton>,
    groups: Vec<SavedButtonGroup>,
    sprites: Vec<(i32, NativeSprite)>,
    bgm: Vec<[u8; 92]>,
    bgv: Vec<[u8; 76]>,
    history: Vec<[u8; NATIVE_HISTORY_STRIDE]>,
    window: Option<NativeWindow>,
    return_addresses: Vec<u32>,
    omitted: BTreeSet<&'static str>,
}

struct NativeReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> NativeReader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(count)?;
        let bytes = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(bytes)
    }

    fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N)?.try_into().ok()
    }

    fn word(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.array()?))
    }

    fn words(&mut self, count: usize) -> Option<Vec<i32>> {
        Some(
            self.take(count.checked_mul(4)?)?
                .chunks_exact(4)
                .map(|word| i32::from_le_bytes(word.try_into().unwrap()))
                .collect(),
        )
    }

    fn sprite_info(&mut self) -> Option<PalSpriteInfoKoikake32> {
        let w: [i32; 19] = self.words(19)?.try_into().ok()?;
        let info = PalSpriteInfoKoikake32 {
            visible: w[0] as u32,
            color: w[1] as u32,
            x: f32::from_bits(w[2] as u32),
            y: f32::from_bits(w[3] as u32),
            z: f32::from_bits(w[4] as u32),
            offset_x: w[5],
            offset_y: w[6],
            texture_width: w[7] as u32,
            texture_height: w[8] as u32,
            cell_width: w[9] as u32,
            cell_height: w[10] as u32,
            center_x: w[11],
            center_y: w[12],
            priority: w[13],
            scale: f32::from_bits(w[14] as u32),
            rotation_x: f32::from_bits(w[15] as u32),
            rotation_y: f32::from_bits(w[16] as u32),
            rotation_z: f32::from_bits(w[17] as u32),
            render_mode: w[18] as u32,
        };
        [
            info.x,
            info.y,
            info.z,
            info.scale,
            info.rotation_x,
            info.rotation_y,
            info.rotation_z,
        ]
        .iter()
        .all(|value| value.is_finite())
        .then_some(info)
    }

    fn sprite(&mut self, resource: i32) -> Option<NativeSprite> {
        Some(NativeSprite {
            resource,
            info: self.sprite_info()?,
            wrapper: self.array()?,
            name: None,
        })
    }
}

impl NativeImage {
    fn read(bytes: &[u8], prefix_len: usize) -> Option<Self> {
        if bytes.len() > NATIVE_SAVE_CAP {
            return None;
        }
        let mut r = NativeReader {
            bytes,
            offset: prefix_len,
        };
        let mut omitted = BTreeSet::new();
        let user_memory = r.words(0x400)?;
        // VM private storage, dynamic-string ring, and two dispatch fields.
        // The original does not serialize the system or temporary memory bank.
        if r.take(0x8004)?.iter().any(|byte| *byte != 0) {
            omitted.insert("VM private 0x8004-byte bank");
        }
        let string_cursor = r.word()? as usize;
        if string_cursor >= 8 {
            return None;
        }
        let strings = (0..8).map(|_| r.array()).collect::<Option<Vec<_>>>()?;
        r.take(8)?;
        let stack_count = r.word()? as usize;
        let mut stack = r.words(64)?;
        if stack_count > stack.len() {
            return None;
        }
        stack.truncate(stack_count);
        let argument_base = r.word()? as i32;
        let argument_count = r.word()? as usize;
        let mut arguments = r.words(512)?;
        if argument_count > arguments.len() {
            return None;
        }
        arguments.truncate(argument_count);
        r.take(40)?; // Fixed subsystem state, 0x4166C0.
        let mut groups = Vec::new();
        let mut buttons = Vec::new();
        for group in 0..4 {
            if r.word()? == 0 {
                continue;
            }
            let images = r.words(2)?;
            let group_data = r.words(4)?;
            groups.push(SavedButtonGroup {
                group,
                normal_image: images[0],
                hover_image: images[1],
                onmouse_index: -1,
                offmouse_index: -1,
                push_index: -1,
                slider_ended: false,
                raw_toggle: 0,
                locked: false,
                lock_started_ms: 0,
                lock_duration_ms: 0,
            });
            if group_data.iter().any(|word| *word != 0) {
                omitted.insert("button group trailing control words");
            }
            for index in 0..128 {
                let resource = r.word()? as i32;
                if resource == 0 {
                    continue;
                }
                let control: [u32; 4] = r
                    .words(4)?
                    .into_iter()
                    .map(|v| v as u32)
                    .collect::<Vec<_>>()
                    .try_into()
                    .ok()?;
                let mut sprite = r.sprite(resource)?;
                if resource & 0x1000_0000 != 0 {
                    sprite.name = Some(r.array()?);
                }
                buttons.push(NativeButton {
                    group,
                    index,
                    control,
                    sprite,
                });
            }
        }
        r.take(28)?; // Select wrapper, 0x427820.
        r.take(16)?; // Process wrapper, 0x421E00.
        let effects = r.word()?;
        if effects & !0x1d != 0 {
            return None;
        }
        for mask in [1, 4, 8, 16] {
            if effects & mask != 0 {
                r.take(16)?;
                omitted.insert("active global effect");
            }
        }
        // 0x418800 reads at most eight effect entries and requires a sentinel
        // before the next section. Reject the native writer's overflowing case.
        let mut terminated = false;
        for _ in 0..8 {
            let state = r.word()?;
            if state == u32::MAX {
                terminated = true;
                break;
            }
            if state == 1 {
                r.word()?;
                if r.word()? == 0 {
                    r.take(12)?;
                }
                omitted.insert("active effect slot");
            }
        }
        if !terminated {
            return None;
        }
        r.take(128)?; // Sprite-bank aggregate state.
        let mut sprites = Vec::new();
        for slot in 0..64 {
            let resource = r.word()? as i32;
            if resource == 0 {
                continue;
            }
            let mut sprite = r.sprite(resource)?;
            r.take(132)?; // Retained GRAPHIC.DAT metadata.
            let action_pointer = r.word()?;
            let flags = r.word()?;
            r.word()?;
            r.take(16)?;
            r.word()?;
            r.take(256)?;
            if resource & 0x1000_0000 != 0 {
                sprite.name = Some(r.array()?);
            }
            if action_pointer != 0 || flags & 0x00e4_fffe != 0 {
                omitted.insert("sprite filter, movie, or action playback state");
            }
            sprites.push((slot, sprite));
        }
        let bgm = (0..2)
            .map(|_| r.array::<92>())
            .collect::<Option<Vec<_>>>()?;
        for sound in &bgm {
            if word_at(sound, 4) != 0 {
                if word_at(sound, 12) != 100 || word_at(sound, 8) & 0x1000_0000 != 0 {
                    omitted.insert("BGM channel gain or paused state");
                }
                if sound[64..80].iter().any(|byte| *byte != 0) {
                    omitted.insert("BGM byte-range descriptor");
                }
            }
        }
        r.take(116)?; // SE shared state.
        for _ in 0..16 {
            let se: [u8; 64] = r.array()?;
            if word_at(&se, 4) != 0 && word_at(&se, 8) & 0x2000_0001 != 0 {
                omitted.insert("active SE channel");
            }
        }
        r.take(384)?; // BGV shared state.
        let bgv = (0..8).map(|_| r.array()).collect::<Option<Vec<_>>>()?;
        let history_count = r.word()? as usize;
        if history_count > NATIVE_HISTORY_COUNT {
            return None;
        }
        let mut history = Vec::with_capacity(history_count);
        for index in 0..NATIVE_HISTORY_COUNT {
            let record = r.array()?;
            if index < history_count {
                history.push(record);
            }
        }
        let window = if r.word()? != 0 {
            let mode = r.word()? as i32;
            r.word()?;
            let fields = r.words(9)?.try_into().ok()?;
            r.take(0x824)?;
            r.take(92)?; // Text body wrapper.
            r.take(92)?; // Name wrapper.
            if mode == 1 {
                r.take(92)?;
            }
            let resource = r.word()? as i32;
            let base = if resource != 0 {
                Some(r.sprite(resource)?)
            } else {
                None
            };
            for _ in 0..3 {
                let resource = r.word()? as i32;
                if resource != 0 {
                    r.sprite(resource)?;
                    r.word()?;
                    omitted.insert("text icon animation");
                }
            }
            Some(NativeWindow { fields, base })
        } else {
            None
        };
        r.take(36)?; // Child-process manager fields.
        for _ in 0..8 {
            if r.word()? != 0 {
                r.take(0x1229c)?;
                omitted.insert("child process resume state");
            }
        }
        let return_count = r.word()? as usize;
        if return_count > DEFAULT_STACK_LIMIT {
            return None;
        }
        let return_addresses = r
            .words(return_count)?
            .into_iter()
            .map(|v| v as u32)
            .collect();
        if r.offset != bytes.len() {
            return None;
        }
        Some(Self {
            user_memory,
            string_cursor,
            strings,
            stack,
            argument_base,
            arguments,
            buttons,
            groups,
            sprites,
            bgm,
            bgv,
            history,
            window,
            return_addresses,
            omitted,
        })
    }
}

fn word_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn native_string(bytes: &[u8], nls: Nls) -> Option<String> {
    let end = bytes
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(bytes.len());
    nls.decode(&bytes[..end]).ok()
}

impl ScriptRuntime {
    pub(super) fn import_original_save_with_resources(
        &mut self,
        resources: &mut ResourceManager,
        slot: i32,
        assets: &CoreAssets,
    ) -> Option<RuntimeSaveSnapshot> {
        if !assets.legacy_koikake() {
            return self.import_original_save(resources.root(), slot, assets);
        }
        let path = find_loose_save_file(resources.root(), &original_save_filename(slot))?;
        let size = crate::native_asset::vfs::file_len(&path).ok()?;
        if !(HEADER_BEFORE_PIXELS..=NATIVE_SAVE_CAP).contains(&size) {
            return None;
        }
        let bytes = crate::native_asset::vfs::read_range(&path, 0, size).ok()?;
        let (prefix, trailer) = decode_original_save(&bytes)?;
        if trailer.is_some() {
            return None;
        }
        let old_strings = self.dynamic_strings.clone();
        let old_cursor = self.dynamic_string_cursor;
        let result = self.import_koikake_save(&bytes, &prefix, assets, resources);
        if result.is_none() {
            self.dynamic_strings = old_strings;
            self.dynamic_string_cursor = old_cursor;
            log::warn!("native Koikake save slot {slot} failed bounded record validation or resource reconstruction");
        }
        result
    }

    pub(super) fn import_koikake_save(
        &mut self,
        bytes: &[u8],
        prefix: &OriginalSavePrefix,
        assets: &CoreAssets,
        resources: &mut ResourceManager,
    ) -> Option<RuntimeSaveSnapshot> {
        let native = NativeImage::read(bytes, original_prefix_len(bytes)?)?;
        if prefix.resume_pc <= 0 {
            return None;
        }
        let resume_next = u32::try_from(prefix.resume_pc).ok()?.checked_add(12)?;
        if resume_next as usize >= assets.script.bytes.len()
            || native
                .return_addresses
                .iter()
                .any(|pc| *pc as usize >= assets.script.bytes.len())
        {
            return None;
        }
        let nls = resources.nls();
        let strings = native
            .strings
            .iter()
            .map(|bytes| native_string(bytes, nls))
            .collect::<Option<Vec<_>>>()?;
        self.dynamic_strings = strings;
        self.dynamic_string_cursor = native.string_cursor;
        let mut snapshot = self.capture_save_snapshot();
        snapshot.ui = None;
        snapshot.pc = resume_next;
        snapshot.call_stack = native.return_addresses;
        snapshot.user_mem = native.user_memory;
        snapshot.vars.fill(0); // 0x436000 clears 255 temporary register words.
        snapshot.stack = native.stack;
        snapshot.argument_base = native.argument_base;
        snapshot.argument_stack = native.arguments;
        snapshot.sprites.clear();
        snapshot.buttons.clear();
        snapshot.button_groups = native.groups;
        snapshot.bgm_tracks.clear();
        snapshot.bgv_tracks.clear();
        snapshot.history_records.clear();
        snapshot.history_texts.clear();
        snapshot.tips = SavedTipsState::default();
        snapshot.actions = Default::default();
        snapshot.children = Default::default();
        for (slot, sprite) in native.sprites {
            snapshot
                .sprites
                .push(self.import_native_sprite(slot, &sprite, assets, resources)?);
        }
        for button in native.buttons {
            let sprite =
                self.import_native_sprite(button.index, &button.sprite, assets, resources)?;
            snapshot.buttons.push(SavedButton {
                base_position: [
                    word_at(&button.sprite.wrapper, 0) as i32,
                    word_at(&button.sprite.wrapper, 4) as i32,
                ],
                slider_position: [
                    word_at(&button.sprite.wrapper, 12) as i32,
                    word_at(&button.sprite.wrapper, 16) as i32,
                ],
                registration_order: snapshot.buttons.len() as u64 + 1,
                // 0x41007A calls two-argument PalButtonEntry. Its native
                // default flags are 1; the descriptor stores callback points.
                native_flags: 1,
                hover_callbacks: [button.control[2], button.control[3]]
                    .map(|point| (point != 0 && point != u32::MAX).then_some(point)),
                base_row: 0,
                group: button.group,
                index: button.index,
                visible: sprite.visible,
                enabled: true,
                alpha: (sprite.color >> 24) as u8,
                gosub_point: button.control[1] as i32,
                x: sprite.x,
                y: sprite.y,
                z: sprite.z,
                priority: sprite.priority,
                width: sprite.width,
                height: sprite.height,
                cell_width: button.sprite.info.cell_width,
                cell_height: button.sprite.info.cell_height,
                rect: sprite.rect,
                color: sprite.color,
                name: self.native_save_resource(&button.sprite, assets, nls)?,
                rgba: sprite.rgba.as_ref().clone(),
            });
        }
        for (slot, sound) in native.bgm.into_iter().enumerate() {
            let resource = word_at(&sound, 4) as i32;
            if resource == 0 || word_at(&sound, 8) & 0x2000_0000 != 0 {
                continue;
            }
            let name = self.native_sound_resource(resource, &sound[32..64], assets, nls)?;
            snapshot.bgm_tracks.push(SavedBgmTrack {
                slot: slot as i32,
                name,
                looping: true,
                loop_start: -1,
                loop_end: -1,
            });
        }
        for (slot, sound) in native.bgv.into_iter().enumerate() {
            let resource = word_at(&sound, 4) as i32;
            if resource == 0 || word_at(&sound, 8) & 1 == 0 {
                continue;
            }
            let name = self.resolve_resource_string(resource, assets, nls)?;
            snapshot
                .bgv_tracks
                .push(SavedBgvTrack::native_restore(slot as i32, name));
        }
        // Native records are newest first; the portable backlog appends.
        for record in native.history.iter().rev() {
            let name = native_string(&record[..255], nls)?;
            let body = native_string(&record[255..510], nls)?;
            let voice = native_string(&record[510..542], nls)?;
            snapshot.history_records.push([0; 9]);
            snapshot
                .history_texts
                .push(Some(FrozenHistoryText { name, body, voice }));
        }
        snapshot.text_visible = native.window.is_some();
        snapshot.text_initialized = native.window.is_some();
        snapshot.text_args = [0, 0, 0x0fff_ffff, 0x0fff_ffff];
        if let Some(text) = snapshot.history_texts.last().and_then(Option::as_ref) {
            let name = self.store_dynamic_string(text.name.clone());
            let body = self.store_dynamic_string(text.body.clone());
            snapshot.text_args = [0, body, name, 0x0fff_ffff];
        }
        if let Some(window) = native.window {
            snapshot.text_mode = window.fields[1];
            snapshot.text_init_args = [
                window.fields[1],
                window.fields[7],
                window.fields[6],
                window.fields[5],
                window.fields[4],
                window.fields[3],
                window.fields[2],
                window.fields[0],
            ];
            snapshot.text_base = window.base.as_ref().map_or(0, |base| base.resource);
            snapshot.text_alpha = Some(
                window
                    .base
                    .as_ref()
                    .map_or(255, |base| (base.info.color >> 24) as i32),
            );
        }
        snapshot.show_wait_mark = snapshot.text_visible;
        snapshot.resume_wait_click = snapshot.text_visible;
        snapshot.title_bytes = prefix.title.clone();
        snapshot.thumb_width = prefix.thumb_width;
        snapshot.thumb_height = prefix.thumb_height;
        snapshot.thumb_pixels = prefix.pixels.clone();
        if !native.omitted.is_empty() {
            log::warn!(
                "native save has additional state not yet reconstructed: {:?}",
                native.omitted
            );
        }
        Some(snapshot)
    }

    fn native_sound_resource(
        &self,
        resource: i32,
        inline: &[u8],
        assets: &CoreAssets,
        nls: Nls,
    ) -> Option<String> {
        if resource & 0x1000_0000 != 0 {
            native_string(inline, nls)
        } else {
            self.resolve_resource_string(resource, assets, nls)
        }
    }

    fn native_save_resource(
        &self,
        sprite: &NativeSprite,
        assets: &CoreAssets,
        nls: Nls,
    ) -> Option<String> {
        match sprite.name {
            Some(name) => native_string(&name, nls),
            None => self.resolve_resource_string(sprite.resource, assets, nls),
        }
    }

    fn import_native_sprite(
        &self,
        slot: i32,
        sprite: &NativeSprite,
        assets: &CoreAssets,
        resources: &mut ResourceManager,
    ) -> Option<SavedSprite> {
        let mut name = self.native_save_resource(sprite, assets, resources.nls())?;
        if let Some(replacement) = assets
            .graphic_index
            .as_ref()
            .and_then(|index| index.lookup(&name))
            .and_then(|record| record.replacement_resource())
        {
            name = replacement;
        }
        let info = sprite.info;
        let (width, height, rgba) = if let Some((a, r, g, b)) = parse_solid_color_name_argb(&name) {
            let count = (info.texture_width as usize).checked_mul(info.texture_height as usize)?;
            if count == 0 || count.checked_mul(4)? > SAVE_SPRITE_BYTES_CAP {
                return None;
            }
            let mut rgba = Vec::with_capacity(count * 4);
            for _ in 0..count {
                rgba.extend_from_slice(&[r, g, b, a]);
            }
            (info.texture_width, info.texture_height, rgba)
        } else {
            let asset = open_resource_variant(resources, &name, IMAGE_EXTENSIONS).ok()?;
            let image = decode_asset_image(resources, &asset).ok()?;
            (image.width, image.height, image.rgba)
        };
        if info.rotation_x != 0.0
            || info.rotation_y != 0.0
            || info.rotation_z != 0.0
            || info.center_x != 0
            || info.center_y != 0
            || info.render_mode != 0
        {
            log::warn!("native save sprite {slot} carries rotation/center/render-mode fields outside the portable snapshot");
        }
        if rgba.len() > SAVE_SPRITE_BYTES_CAP {
            return None;
        }
        Some(SavedSprite {
            slot,
            transition_source: false,
            x: info.x as i32,
            y: info.y as i32,
            z: info.z as i32,
            offset_x: info.offset_x,
            offset_y: info.offset_y,
            priority: info.priority,
            scale_bits: info.scale.to_bits(),
            scale_y_bits: None,
            color: info.color,
            visible: info.visible != 0,
            rect: [0, 0, info.cell_width as i32, info.cell_height as i32],
            width,
            height,
            native_projection: None,
            center_scale: false,
            rgba: Arc::new(rgba),
            option_type: 0,
            option_backup: Vec::new(),
        })
    }
}
