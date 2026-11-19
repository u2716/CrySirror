// SPDX-License-Identifier: MPL-2.0
//! Native message-window state and placement over shared retained layers.

mod field;
mod image_font;
mod measure;
mod output;
#[cfg(test)]
mod settings_tests;

use super::visual::Objects;
use crate::vm::{CmvsVm, DrawProperty};
use anyhow::{ensure, Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const DEFAULT_FACE: &[u8] = b"\x82l\x82r \x83S\x83V\x83b\x83N";

#[derive(Clone, Copy)]
struct Settings {
    output: output::OutputSettings,
    only_read: bool,
    bold: bool,
    voice_cut: u32,
    auto_base: u32,
    auto_per_glyph: u32,
    auto_acceleration: u32,
    auto_voice_wait: u32,
    auto_voice_sync: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output: output::OutputSettings {
                speed: 4,
                outline: 1,
                reveal_mode: 1,
                reveal_duration: 2,
                direct_duration: false,
            },
            only_read: false,
            bold: false,
            voice_cut: 0,
            auto_base: 10,
            auto_per_glyph: 0,
            auto_acceleration: 0,
            auto_voice_wait: 0,
            auto_voice_sync: true,
        }
    }
}

impl Settings {
    fn wait_elapsed(
        self,
        host: &dyn HostServices,
        client: ClientId,
        voice: u32,
        glyphs: usize,
        delta: u32,
        elapsed: &mut u32,
    ) -> Result<bool> {
        if (voice as i32) >= 0 && self.auto_voice_sync {
            if super::sound::position(host, client, voice % 4 + 5)? != u32::MAX {
                *elapsed = 0;
                return Ok(false);
            }
            *elapsed = elapsed.wrapping_add(delta);
            return Ok(*elapsed >= self.auto_voice_wait.wrapping_mul(100).wrapping_add(250));
        }
        *elapsed = elapsed.wrapping_add(delta);
        Ok(*elapsed >= self.text_delay(glyphs))
    }

    fn text_delay(self, glyphs: usize) -> u32 {
        let base = self.auto_base.wrapping_mul(100);
        let glyph_time = self.auto_per_glyph.wrapping_mul(5);
        let linear = base.wrapping_add(glyph_time.wrapping_mul(glyphs as u32));
        if self.auto_acceleration == 0 || glyph_time == 0 {
            return linear;
        }
        let target = glyph_time as i32 as f32;
        let increment = self.auto_acceleration as i32 as f32 * 0.25;
        let mut sum = 0.;
        let mut step = 0.;
        for _ in 0..glyphs {
            sum += step;
            if target > step {
                step += increment;
            }
        }
        let reduction = if (-2147483648.0..2147483648.0).contains(&sum) {
            sum as i32
        } else {
            i32::MIN
        };
        let result = linear.wrapping_sub(reduction as u32);
        if result > 65535 {
            linear.max(base)
        } else {
            result.max(base)
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct Messages {
    windows: [Option<Window>; 8],
    /// Slots 0..=11 are text objects; 12..=19 are the `0x2380` text fields.
    texts: [Option<Text>; 20],
    fields: [field::Field; 8],
    /// Stack words of the aliased command being completed.
    alias_words: Option<u16>,
    font_face: Option<Vec<u8>>,
    alpha_revision: Option<bool>,
    resolved_fonts: [Option<Arc<str>>; 20],
    settings: Settings,
    timed_elapsed: u32,
    /// `0x23c0..=0x23c2` glyph cells drawn in place of outlines.
    image_font: Arc<image_font::ImageFont>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Window {
    text: usize,
    shown: bool,
    hidden: bool,
    positions: [[i32; 2]; 2],
    position: [i32; 2],
    alternate: bool,
    wait_cursor: [i16; 3],
    buttons: [WindowButton; 32],
    pressed_button: Option<usize>,
    #[serde(skip)]
    animations: BTreeMap<Option<usize>, Animation>,
    wait_elapsed: u32,
}

#[derive(Clone, Copy)]
struct Cell {
    next: i32,
    duration: u32,
    crop: [i16; 4],
    origin: [i16; 2],
}

impl Cell {
    fn property(self) -> DrawProperty {
        DrawProperty::Frame {
            crop: self.crop,
            origin: self.origin,
        }
    }
}

#[derive(Clone)]
struct Animation {
    cells: [Option<Cell>; 64],
    playing: Option<(usize, u32)>,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            cells: [None; 64],
            playing: None,
        }
    }
}

#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
struct WindowButton {
    rect: [i16; 4],
    enabled: bool,
    images: [[i16; 6]; 5],
    state: u16,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Text {
    font: Arc<str>,
    explicit_face: bool,
    bold: bool,
    scale: f32,
    position: [i32; 2],
    logical: [u32; 2],
    region: [i32; 4],
    pen: [i32; 2],
    size: u32,
    spacing: [u32; 2],
    order: i32,
    colors: [u32; 2],
    delay: u32,
    animation: [u32; 2],
    decoration: u32,
    ruby: bool,
    ruby_size: u32,
    ruby_spacing: i32,
    reserve_ruby: bool,
    overflow: u32,
    break_characters: [Vec<u8>; 2],
    break_configured: bool,
    ruby_lines: [u32; 32],
    layout_scratch: [u32; 2],
    glyph_parameter: u32,
    glyphs: Vec<output::Glyph>,
    published: bool,
    hang_punctuation: bool,
    outline_mode: u32,
    /// Text field frame set by `0x2396`; glyphs outside it are not drawn.
    #[serde(default)]
    view: Option<[f32; 4]>,
    /// `0x2397` (0x14003f8d0): draw mode 4 with a second texture, a screen-sized
    /// matte the glyphs are stencilled through. Name, published asset and size.
    #[serde(default)]
    matte: Option<(String, String, [u32; 2])>,
}

impl Text {
    fn new(
        position: [i32; 2],
        extent: [i32; 2],
        logical: [u32; 2],
        font: Arc<str>,
        bold: bool,
    ) -> Self {
        Self {
            font,
            explicit_face: false,
            bold,
            scale: 1.0,
            position,
            logical,
            region: [0, 0, extent[0], extent[1]],
            pen: [0; 2],
            size: 27,
            spacing: [0; 2],
            order: 1,
            colors: [0xffffff; 2],
            delay: 100,
            animation: [0; 2],
            decoration: 0,
            ruby: false,
            ruby_size: 12,
            ruby_spacing: 0,
            reserve_ruby: false,
            overflow: 0,
            break_characters: Default::default(),
            break_configured: false,
            ruby_lines: [0; 32],
            layout_scratch: [0; 2],
            glyph_parameter: 0,
            glyphs: Vec::new(),
            published: false,
            hang_punctuation: true,
            outline_mode: 2,
            view: None,
            matte: None,
        }
    }

    /// Constructor state of 0x14006cee0 for a text object created by `0x2100`/`0x211c`.
    fn object(logical: [u32; 2], font: Arc<str>, bold: bool) -> Self {
        Self {
            size: 0,
            delay: 30,
            ..Self::new([0; 2], [0; 2], logical, font, bold)
        }
    }

    fn object_settings(&self) -> output::OutputSettings {
        output::OutputSettings {
            speed: 2,
            outline: self.outline_mode,
            reveal_mode: self.animation[0],
            reveal_duration: self.animation[1],
            direct_duration: true,
        }
    }
}

impl Messages {
    pub fn set_alpha_revision(&mut self, enabled: Option<bool>) {
        self.alpha_revision = enabled;
    }

    pub fn reset(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        let mut scope = crysirror_core::scene_checkpoint::SceneCheckpointScope::default();
        self.capture(&mut scope);
        let layers: Vec<_> = scope
            .layer_roots
            .into_iter()
            .map(|id| json!({"op":"remove","id":id}))
            .collect();
        if !layers.is_empty() {
            host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":layers}))?)?;
        }
        *self = Self {
            settings: self.settings,
            font_face: self.font_face.take(),
            resolved_fonts: std::mem::take(&mut self.resolved_fonts),
            ..Self::default()
        };
        Ok(())
    }
    pub fn configure(&mut self, bytes: &[u8]) {
        let (decoded, _) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
        self.settings.only_read = true;
        self.settings.output.speed = 2;
        self.settings.output.outline = 0;
        let entries = decoded
            .lines()
            .map(str::trim)
            .skip_while(|line| !line.eq_ignore_ascii_case("[MESSAGE_INFO]"))
            .skip(1)
            .take_while(|line| !line.starts_with('['));
        for (key, value) in entries.filter_map(|line| line.split_once('=')) {
            let key = key.trim().to_ascii_uppercase();
            if key == "FONT" {
                let (face, _, invalid) = encoding_rs::SHIFT_JIS.encode(value.trim());
                if !invalid && (1..32).contains(&face.len()) && !face.contains(&0) {
                    self.font_face = Some(face.into_owned());
                }
                continue;
            }
            let Ok(word) = value.trim().parse::<i32>() else {
                continue;
            };
            let settings = &mut self.settings;
            match key.as_str() {
                "TEXT_WAIT" => settings.output.speed = word as u32,
                "TEXT_GANG" => settings.output.outline = word as u32,
                "TEXT_BOLD" => settings.bold = word != 0,
                "EFFECT_TYPE" => settings.output.reveal_mode = word as u32,
                "EFFECT_TIMER" => settings.output.reveal_duration = word as u32,
                "READ_SKIP" => settings.only_read = word != 0,
                "VOICE_CUT_MODE" => settings.voice_cut = word as u32,
                "AUTO_MODE_1" => settings.auto_base = word as u32,
                "AUTO_MODE_2" => settings.auto_per_glyph = word as u32,
                "AUTO_MODE_3" => settings.auto_acceleration = word as u32,
                "AUTO_MODE_4" => settings.auto_voice_wait = word as u32,
                "AUTO_VOICE_SYNC" => settings.auto_voice_sync = word != 0,
                _ => {}
            }
        }
    }

    pub fn capture(&self, scope: &mut crysirror_core::scene_checkpoint::SceneCheckpointScope) {
        let mut fonts = BTreeSet::new();
        for (slot, window) in self.windows.iter().enumerate() {
            if let Some(text) = window
                .as_ref()
                .and_then(|window| self.texts[window.text].as_ref())
                .filter(|text| text.published)
            {
                scope.layer_roots.push(text_layer(slot));
                fonts.extend(text.glyphs.iter().map(|glyph| glyph.font.as_ref()));
            }
        }
        scope
            .font_aliases
            .extend(fonts.into_iter().map(str::to_owned));
    }

    fn default_face(&self) -> &[u8] {
        self.font_face.as_deref().unwrap_or(DEFAULT_FACE)
    }

    /// Shared font alias for native dialogs: the default message face.
    pub fn ui_font(&self, host: &mut dyn HostServices, client: ClientId) -> Result<String> {
        if let Some(alias) = resolve_face(host, client, self.default_face())? {
            return Ok(alias.to_string());
        }
        host.font_library(client)?
            .aliases()?
            .into_iter()
            .next()
            .context("CMVS has no font for native dialogs")
    }

    /// `0x213d` (0x140085450): a chosen face becomes the default and replaces the
    /// face of every text object that follows the default.
    pub fn select_default_face(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        face: Vec<u8>,
    ) -> Result<()> {
        self.font_face = Some(face);
        let alias = resolve_face(host, client, self.default_face())?
            .unwrap_or_else(|| Arc::from("default"));
        for (slot, text) in self.texts.iter_mut().enumerate() {
            if let Some(text) = text.as_mut().filter(|text| !text.explicit_face) {
                text.font = alias.clone();
                self.resolved_fonts[slot] = Some(alias.clone());
            }
        }
        Ok(())
    }

    pub fn check_initial_face(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        if self.font_face.is_some() && resolve_face(host, client, self.default_face())?.is_none() {
            self.font_face = None;
        }
        Ok(())
    }

    pub fn text_setting(&mut self, vm: &mut CmvsVm, opcode: u16) -> Result<()> {
        let [mode] = vm.owner_arguments(opcode)?;
        if opcode == 0x22de {
            if mode == 0 {
                vm.complete_font_face_query(self.default_face())?;
            } else {
                let [_, reference] = vm.owner_arguments(opcode)?;
                let face = bytes(vm, reference, 31)?.to_vec();
                ensure!(!face.is_empty(), "CMVS default font face is empty");
                vm.complete_owner_call(2, None)?;
                self.font_face = Some(face);
            }
            return Ok(());
        }

        // The bounded integer wrappers fetch the value even on reads.
        let raw = if mode != 0 || (0x22e1..=0x22e3).contains(&opcode) {
            vm.owner_argument(opcode, 8)?
        } else {
            0
        };
        if matches!(opcode, 0x22e0 | 0x22e4 | 0x22e6) {
            let field = match opcode {
                0x22e0 => &mut self.settings.bold,
                0x22e4 => &mut self.settings.only_read,
                _ => &mut self.settings.auto_voice_sync,
            };
            vm.complete_owner_call(2, (mode == 0).then_some(u32::from(*field)))?;
            if mode != 0 {
                *field = raw != 0;
                if opcode == 0x22e0 {
                    for text in self
                        .texts
                        .iter_mut()
                        .flatten()
                        .filter(|text| !text.explicit_face)
                    {
                        text.bold = *field;
                    }
                }
            }
            return Ok(());
        }
        let (field, new) = match opcode {
            0x22df => (
                &mut self.settings.output.outline,
                ((raw as i32).wrapping_abs() % 3) as u32,
            ),
            0x22e1 => (
                &mut self.settings.output.speed,
                (raw as i32).clamp(0, 10) as u32,
            ),
            0x22e2 => (
                &mut self.settings.output.reveal_mode,
                (raw as i32).clamp(0, 2) as u32,
            ),
            0x22e3 => (
                &mut self.settings.output.reveal_duration,
                (raw as i32).clamp(0, 4) as u32,
            ),
            0x22e7 => (&mut self.settings.auto_base, raw),
            0x22e8 => (&mut self.settings.auto_per_glyph, raw),
            0x22f8 => (&mut self.settings.auto_voice_wait, raw),
            _ => anyhow::bail!("invalid CMVS text setting {opcode:#x}"),
        };
        vm.complete_owner_call(2, (mode == 0).then_some(*field))?;
        if mode != 0 {
            *field = new;
        }
        Ok(())
    }

    pub fn select_text_face(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
    ) -> Result<()> {
        let [slot, reference] = vm.owner_arguments(0x2106)?;
        let slot = slot as usize;
        ensure!(
            self.texts.get(slot).is_some_and(Option::is_some),
            "CMVS text object missing"
        );
        let requested = bytes(vm, reference, 31)?;
        let alias = resolve_face(
            host,
            client,
            if requested.is_empty() {
                self.default_face()
            } else {
                requested
            },
        )?
        .unwrap_or_else(|| Arc::from("default"));
        let explicit = !requested.is_empty();
        vm.complete_owner_call(2, None)?;
        let text = self.texts[slot].as_mut().expect("validated text slot");
        text.font = alias.clone();
        text.explicit_face = explicit;
        if !explicit {
            text.bold = self.settings.bold;
        }
        self.resolved_fonts[slot] = Some(alias);
        Ok(())
    }

    pub fn voice_cut(&mut self, vm: &mut CmvsVm, opcode: u16) -> Result<()> {
        let [set] = vm.owner_arguments(opcode)?;
        if set != 0 {
            let [_, value] = vm.owner_arguments(opcode)?;
            vm.complete_owner_call(2, None)?;
            self.settings.voice_cut = ((value as i32).wrapping_abs() % 3) as u32;
        } else {
            vm.complete_owner_call(2, Some(self.settings.voice_cut))?;
        }
        Ok(())
    }

    pub fn contains(&self, index: usize) -> bool {
        self.windows.get(index).is_some_and(Option::is_some)
    }

    pub fn apply(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &mut Objects,
        canvas: [u32; 2],
    ) -> Result<()> {
        if matches!(opcode, 0x2112 | 0x2114..=0x2117 | 0x2119 | 0x211e..=0x211f) {
            return self.text_command(opcode, vm);
        }
        let [slot] = vm.owner_arguments(opcode)?;
        let slot = slot as usize;
        ensure!(slot < 8, "CMVS message window outside 0..7");
        let object = object_slot(slot);
        if matches!(opcode, 0x2140 | 0x215d) {
            let (text, xy, extent, logical, words) = if opcode == 0x215d {
                let [_, text, x, y, w, h, lw, lh] = vm.owner_arguments(opcode)?;
                (text, [x, y], [w, h], [lw, lh], 8)
            } else {
                let [_, text, x, y, w, h] = vm.owner_arguments(opcode)?;
                (text, [x, y], [w, h], canvas, 6)
            };
            let text = text as usize;
            ensure!(
                text < 12 && logical.into_iter().all(|axis| (1..=8192).contains(&axis)),
                "CMVS message logical size or text slot"
            );
            ensure!(
                !self.windows.iter().enumerate().any(|(index, other)| {
                    index != slot && other.as_ref().is_some_and(|window| window.text == text)
                }),
                "CMVS message text is used by another window"
            );
            let position = xy.map(|v| v as i32);
            let font = match &self.resolved_fonts[text] {
                Some(font) => font.clone(),
                None => resolve_face(host, client, self.default_face())?
                    .unwrap_or_else(|| Arc::from("default")),
            };
            objects.reset(host, client, object, None, true)?;
            objects.draw(host, client, object, None, DrawProperty::Shown(false))?;
            self.resolved_fonts[text] = Some(font.clone());
            self.texts[text] = Some(Text::new(
                position,
                extent.map(|v| v as i32),
                logical,
                font,
                self.settings.bold,
            ));
            self.windows[slot] = Some(Window {
                text,
                shown: false,
                hidden: false,
                positions: [position; 2],
                position,
                alternate: false,
                wait_cursor: [0; 3],
                buttons: [WindowButton::default(); 32],
                pressed_button: None,
                animations: BTreeMap::new(),
                wait_elapsed: 0,
            });
            vm.set_message_window(slot, Some(false));
            vm.complete_owner_call(words, None)?;
            return Ok(());
        }
        if opcode == 0x2141 {
            if let Some(window) = &self.windows[slot] {
                objects.reset(host, client, object, None, false)?;
                if self.texts[window.text]
                    .as_ref()
                    .is_some_and(|text| text.published)
                {
                    submit_text(host, client, json!({"op":"remove","id":text_layer(slot)}))?;
                }
                self.texts[window.text] = None;
                self.windows[slot] = None;
            }
            vm.set_message_window(slot, None);
            vm.complete_owner_call(1, None)?;
            return Ok(());
        }
        let window = self.windows[slot]
            .as_mut()
            .context("CMVS message window missing")?;
        let text = self.texts[window.text]
            .as_mut()
            .context("CMVS window text object missing")?;
        if opcode == 0x218d {
            let [_, child] = vm.owner_arguments(opcode)?;
            let ([opacity, x, y, order], [sx, sy]) = objects.draw_info(object, child as i32)?;
            vm.complete_owner_typed_results(
                2,
                &[(0, opacity), (1, x), (2, y), (3, order)],
                &[(8, sx.to_bits()), (9, sy.to_bits())],
            )?;
            return Ok(());
        }
        if opcode == 0x2194 {
            let point = vm.pointer();
            let point: [i32; 2] = std::array::from_fn(|axis| {
                point[axis].wrapping_mul(text.logical[axis] as i32) / canvas[axis] as i32
            });
            let [released, held, pressed] = vm.action(0);
            let mut hovered = u32::MAX;
            let mut clicked = false;
            let mut consumed = false;
            if pressed {
                window.pressed_button = None;
            }
            for (index, button) in window.buttons.iter().enumerate() {
                let [x, y, w, h] = button.rect.map(|v| i32::from(v as u16));
                let rect = [
                    window.position[0].wrapping_add(x),
                    window.position[1].wrapping_add(y),
                    w + 1,
                    h + 1,
                ];
                if !button.enabled
                    || !crysirror_core::region_map::contains_rect(
                        rect.map(i64::from),
                        point.map(i64::from),
                    )
                {
                    continue;
                }
                if pressed {
                    window.pressed_button = Some(index);
                }
                hovered = index as u32;
                if released && !consumed {
                    consumed = true;
                    clicked = window.pressed_button == Some(index);
                }
            }
            if released && (consumed || window.pressed_button.is_some()) {
                vm.consume_action(0);
                window.pressed_button = None;
            }
            if !held {
                window.pressed_button = None;
            }
            vm.complete_owner_results(
                1,
                &[
                    (4, hovered),
                    (5, u32::from(clicked)),
                    (6, u32::from(held && hovered != u32::MAX)),
                ],
            )?;
            return Ok(());
        }
        let (words, value) = match opcode {
            0x20ec => {
                let [_, scale] = vm.owner_arguments(opcode)?;
                let scale = f32::from_bits(scale);
                ensure!(scale.is_finite(), "CMVS window scale must be finite");
                let scale = if (0.95..=1.05).contains(&scale) {
                    1.0
                } else {
                    scale
                };
                objects.draw(host, client, object, None, DrawProperty::Scale(scale))?;
                text.scale = scale;
                publish_text(
                    host,
                    client,
                    slot,
                    window,
                    text,
                    canvas,
                    objects.window_offset(),
                )?;
                (2, None)
            }
            0x2171 => {
                objects.release_image(host, client, object)?;
                (1, None)
            }
            0x2179 | 0x217b => {
                let [_, raw] = vm.owner_arguments(opcode)?;
                let child = (raw as i32 >= 0).then_some(raw as usize);
                if opcode == 0x217b {
                    let exists = child.map_or_else(
                        || objects.contains(object),
                        |child| objects.has_child(object, child),
                    );
                    (2, Some(u32::from(exists)))
                } else {
                    if child.is_none_or(|child| child < 768) {
                        objects.reset(host, client, object, child, false)?;
                        if child.is_some() {
                            window.animations.remove(&child);
                        } else {
                            window.animations.clear();
                        }
                    }
                    (2, None)
                }
            }
            0x215f => {
                let [_, x, y, width, height] = vm.owner_arguments(opcode)?;
                let cursor = vm.pointer();
                let [left, top] = [x, y].map(|v| v as i32);
                let [left, top] = [
                    window.position[0].wrapping_add(left),
                    window.position[1].wrapping_add(top),
                ];
                let [px, py] = std::array::from_fn(|axis| {
                    ((cursor[axis] as u32).wrapping_mul(text.logical[axis]) / canvas[axis]) as i32
                });
                let hit = left <= px
                    && top <= py
                    && px < left.wrapping_add(width as i32)
                    && py < top.wrapping_add(height as i32);
                (5, Some(u32::from(hit)))
            }
            0x21cf..=0x21d2 => (
                window.animation(opcode, host, client, vm, objects, object)?,
                None,
            ),
            0x2142 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.order = value as i32;
                (2, None)
            }
            0x2143 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                if value as i32 > 0 {
                    text.size = value;
                }
                (2, None)
            }
            0x2144 => {
                let [_, x, y] = vm.owner_arguments(opcode)?;
                for (spacing, value) in text.spacing.iter_mut().zip([x, y]) {
                    if value as i32 >= 0 {
                        *spacing = value;
                    }
                }
                (3, None)
            }
            0x2145 => {
                let [_, x, y, w, h] = vm.owner_arguments(opcode)?;
                text.region = [x, y, w, h].map(|v| v as i32);
                text.pen = [x as i32, y as i32];
                (5, None)
            }
            0x2146 => {
                let [_, normal, decoration] = vm.owner_arguments(opcode)?;
                for (color, value) in text.colors.iter_mut().zip([normal, decoration]) {
                    if value <= 0xffffff {
                        *color = value;
                    }
                }
                (3, None)
            }
            0x2147 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.delay = value;
                (2, None)
            }
            0x2148 => {
                let [_, mode, duration] = vm.owner_arguments(opcode)?;
                text.animation = [mode, duration];
                (3, None)
            }
            0x2149 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                if value <= 3 {
                    text.decoration = value;
                }
                (2, None)
            }
            0x214a => {
                text.pen = [text.region[0], text.region[1]];
                text.glyphs.clear();
                publish_text(
                    host,
                    client,
                    slot,
                    window,
                    text,
                    canvas,
                    objects.window_offset(),
                )?;
                (1, None)
            }
            0x214c | 0x214e => {
                let [_, flag] = vm.owner_arguments(opcode)?;
                let flag = flag != 0;
                let visibility_changed = if opcode == 0x214c {
                    window.shown && flag != window.hidden
                } else {
                    flag != window.shown
                };
                if visibility_changed {
                    let shown = if opcode == 0x214c { window.shown } else { flag };
                    let hidden = opcode == 0x214c && flag;
                    objects.draw(
                        host,
                        client,
                        object,
                        None,
                        DrawProperty::Shown(shown && !hidden),
                    )?;
                    window.shown = shown;
                    window.hidden = hidden;
                    if shown && !hidden {
                        text.position = window.position;
                    }
                    vm.set_message_window(slot, Some(shown));
                    publish_text(
                        host,
                        client,
                        slot,
                        window,
                        text,
                        canvas,
                        objects.window_offset(),
                    )?;
                }
                (2, None)
            }
            0x214d => (1, Some(u32::from(window.hidden))),
            0x214f => {
                let [_, x, y] = vm.owner_arguments(opcode)?;
                text.pen = [x as i32, y as i32];
                (3, None)
            }
            0x2150 => {
                let [_, mode, x, y] = vm.owner_arguments(opcode)?;
                ensure!(mode <= 1, "unsupported CMVS message move mode {mode}");
                let mut position = [x as i32, y as i32];
                if mode == 1 {
                    position =
                        std::array::from_fn(|i| window.position[i].wrapping_add(position[i]));
                }
                objects.draw(host, client, object, None, DrawProperty::Position(position))?;
                window.position = position;
                window.positions[0] = position;
                text.position = position;
                publish_text(
                    host,
                    client,
                    slot,
                    window,
                    text,
                    canvas,
                    objects.window_offset(),
                )?;
                (4, None)
            }
            0x2151 => {
                let [_, mode, x, y] = vm.owner_arguments(opcode)?;
                window.wait_cursor[0] = mode as i16;
                match mode as u16 {
                    0 | 2 => window.wait_cursor[1..].fill(0),
                    1 | 3 => window.wait_cursor[1..].copy_from_slice(&[x as i16, y as i16]),
                    _ => {}
                }
                (4, None)
            }
            0x2155 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.glyph_parameter = value;
                (2, None)
            }
            0x215a => {
                let [_, x, y] = vm.owner_arguments(opcode)?;
                window.positions[1] = [x as i32, y as i32];
                (3, None)
            }
            0x215b => {
                let [_, value] = vm.owner_arguments(opcode)?;
                let alternate = value != 0;
                if alternate != window.alternate {
                    let position = window.positions[usize::from(alternate)];
                    objects.draw(host, client, object, None, DrawProperty::Position(position))?;
                    window.position = position;
                    text.position = position;
                    window.alternate = alternate;
                }
                (2, None)
            }
            0x2190..=0x2193 | 0x2195..=0x2197 => {
                let [_, number] = vm.owner_arguments(opcode)?;
                let button = window
                    .buttons
                    .get_mut(number as usize)
                    .context("CMVS window button outside 0..31")?;
                let words = match opcode {
                    0x2190 => {
                        let [_, _, state, x, y, w, h, ox, oy] = vm.owner_arguments(opcode)?;
                        let image = button
                            .images
                            .get_mut(state as usize)
                            .context("CMVS window button state outside 0..4")?;
                        *image = [x, y, w, h, ox, oy].map(|v| v as i16);
                        9
                    }
                    0x2191 => {
                        let [_, _, x, y, w, h] = vm.owner_arguments(opcode)?;
                        button.rect = [x, y, w, h].map(|v| v as i16);
                        6
                    }
                    0x2195 => {
                        vm.complete_owner_call(2, Some(u32::from(button.enabled)))?;
                        return Ok(());
                    }
                    0x2196 => {
                        let [_, _, state] = vm.owner_arguments(opcode)?;
                        let state = state as u16;
                        if state != button.state {
                            let rect = button
                                .images
                                .get(state as usize)
                                .context("CMVS window button state outside 0..4")?;
                            let child = number as usize + 16;
                            if objects.has_child(object, child) {
                                objects.draw(
                                    host,
                                    client,
                                    object,
                                    Some(child),
                                    DrawProperty::Crop(rect[..4].try_into().expect("crop fields")),
                                )?;
                            }
                            button.state = state;
                        }
                        3
                    }
                    0x2197 => {
                        vm.complete_owner_call(2, Some(u32::from(button.state)))?;
                        return Ok(());
                    }
                    _ => {
                        // 0x140071b90 enables (0x2192) and 0x140071bd0 disables (0x2193)
                        // the button record and shows or hides its child object.
                        let enabled = opcode == 0x2192;
                        let child = number as usize + 16;
                        if objects.has_child(object, child) {
                            objects.draw(
                                host,
                                client,
                                object,
                                Some(child),
                                DrawProperty::Shown(enabled),
                            )?;
                        }
                        button.enabled = enabled;
                        2
                    }
                };
                (words, None)
            }
            _ => {
                let words = window_object(opcode, host, client, vm, objects, object)?;
                if matches!(opcode, 0x2178 | 0x2180) {
                    let [_, raw] = vm.owner_arguments(opcode)?;
                    let child = ((raw as i32) >= 0).then_some(raw as usize);
                    if opcode == 0x2178 && child.is_none() {
                        window.animations.clear();
                    } else {
                        window.animations.remove(&child);
                    }
                }
                (words, None)
            }
        };
        vm.complete_owner_call(words, value)?;
        Ok(())
    }

    pub fn output(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &mut Objects,
        canvas: [u32; 2],
        delta: u32,
    ) -> Result<bool> {
        let [slot] = vm.owner_arguments(opcode)?;
        let third = vm.owner_argument(opcode, 12)?;
        let slot = slot as usize;
        let window = self
            .windows
            .get_mut(slot)
            .and_then(Option::as_mut)
            .context("CMVS message window missing")?;
        let text = self.texts[window.text]
            .as_mut()
            .context("CMVS window text missing")?;
        let read = if matches!(opcode, 0x2152 | 0x2153 | 0x215c) {
            vm.prepare_message_read(vm.owner_argument(opcode, 8)?, self.settings.only_read)?
        } else {
            true
        };
        let [skip, auto, bypass, gesture] = vm.message_modes();
        let release = vm.action(0)[0] || vm.action(20)[0];
        if matches!(opcode, 0x2153 | 0x2158) {
            if window.hidden {
                vm.wait_owner_call(opcode, None)?;
                return Ok(true);
            }
            let timed = opcode == 0x2158;
            let forced = !timed
                && !gesture
                && (bypass || vm.action(7)[1] || ((skip || vm.action(8)[1]) && read));
            let manual = if timed || forced {
                false
            } else {
                let hit = window.button_hit(logical_pointer(vm.pointer(), text.logical, canvas));
                let [released, held, pressed] = vm.action(0);
                if pressed {
                    window.pressed_button = hit;
                }
                let consume = released && (hit.is_some() || window.pressed_button.is_some());
                if !held {
                    window.pressed_button = None;
                }
                if consume {
                    vm.consume_action(0);
                }
                !gesture && hit.is_none() && ((released && !consume) || vm.action(20)[0])
            };
            let elapsed = if timed {
                &mut self.timed_elapsed
            } else {
                &mut window.wait_elapsed
            };
            let automatic = if timed || (auto && !gesture && !manual && !forced) {
                self.settings.wait_elapsed(
                    host,
                    client,
                    third,
                    text.glyphs.len(),
                    delta,
                    elapsed,
                )?
            } else {
                false
            };
            let done = manual || forced || automatic;
            if done {
                *elapsed = 0;
            }
            let object = object_slot(slot);
            for child in [2, 3] {
                if !objects.has_child(object, child) {
                    continue;
                }
                if !done && window.wait_cursor[0] == 3 {
                    let position = [
                        text.pen[0].wrapping_add(i32::from(window.wait_cursor[1])),
                        text.pen[1].wrapping_add(i32::from(window.wait_cursor[2])),
                    ];
                    objects.draw(
                        host,
                        client,
                        object,
                        Some(child),
                        DrawProperty::Position(position),
                    )?;
                }
                objects.draw(
                    host,
                    client,
                    object,
                    Some(child),
                    DrawProperty::Shown(!done),
                )?;
            }
            if done {
                if manual {
                    vm.consume_action(0);
                    vm.consume_action(20);
                }
                vm.complete_owner_call(3, None)?;
            } else {
                vm.wait_owner_call(opcode, None)?;
            }
            return Ok(!done);
        }
        // `0x215c` (0x14008d690) runs the 0x2152 output step once per call: the script
        // passes the append flag and polls completion in result 0 instead of waiting.
        let polled = opcode == 0x215c;
        let first = if polled {
            vm.owner_argument(opcode, 16)? == 0
        } else {
            !vm.repeats_system()
        };
        let metrics = vm
            .settings()
            .proportional_glyphs
            .then(|| host.text_measurement(client))
            .transpose()?;
        let mut candidate = text.clone();
        if first {
            let source = bytes(vm, third, 64 * 1024)?;
            Arc::make_mut(&mut self.image_font).prepare(host, client, source, &[candidate.size, candidate.ruby_size])?;
            lay_out(&self.image_font, &mut candidate, source, skip, self.settings.output, metrics.as_deref(), self.alpha_revision)
                .map_err(|why| anyhow::anyhow!("CMVS message layout: {why}"))?;
        }
        let active = candidate.glyphs.iter().any(|glyph| glyph.active);
        let skip_preview = matches!(opcode, 0x2152 | 0x215c) && (bypass || vm.skipping());
        let finish_by_input = matches!(opcode, 0x2152 | 0x215c) && release;
        let done = !window.hidden && (!active || skip_preview || finish_by_input);
        if done && active {
            for glyph in &mut candidate.glyphs {
                glyph.finish();
            }
        }
        if first || (done && active) {
            publish_text(
                host,
                client,
                slot,
                window,
                &mut candidate,
                canvas,
                objects.window_offset(),
            )?;
            *text = candidate;
        }
        if done {
            if active && !skip_preview && finish_by_input {
                vm.consume_action(0);
                vm.consume_action(20);
            }
        }
        if polled {
            vm.complete_owner_call(4, Some(u32::from(done)))?;
            return Ok(false);
        }
        if done {
            vm.finish_owner_call(3, None, true)?;
        } else {
            vm.wait_owner_call(opcode, None)?;
        }
        Ok(true)
    }

    fn text_command(&mut self, opcode: u16, vm: &mut CmvsVm) -> Result<()> {
        let [slot, value] = vm.owner_arguments(opcode)?;
        let text = self
            .texts
            .get_mut(slot as usize)
            .and_then(Option::as_mut)
            .context("CMVS text record missing")?;
        let words = match opcode {
            0x2112 => {
                let (lines, [count, max, last]) =
                    output::ruby_layout(text, bytes(vm, value, 64 * 1024)?)
                        .map_err(|why| anyhow::anyhow!("CMVS ruby measurement: {why}"))?;
                ensure!(
                    lines[16..].iter().all(|&mark| mark == 0),
                    "CMVS 0x2112 ruby flag exceeds native caller space"
                );
                vm.complete_owner_results(2, &[(0, count), (1, max), (4, last)])?;
                return Ok(());
            }
            0x2119 => {
                let count = output::encoded_length(bytes(vm, value, 64 * 1024)?)
                    .map_err(|why| anyhow::anyhow!("CMVS encoded text length: {why}"))?;
                vm.complete_owner_call(2, Some(count))?;
                return Ok(());
            }

            0x2114 => {
                text.ruby = value != 0;
                2
            }
            0x2115 => {
                text.ruby_size = value;
                2
            }
            0x2116 => {
                text.ruby_spacing = value as i32;
                2
            }
            0x2117 => {
                text.reserve_ruby = value != 0;
                2
            }
            0x211e => {
                let [_, first, second, overflow] = vm.owner_arguments(opcode)?;
                text.break_characters = [
                    bytes(vm, first, 63)?.to_vec(),
                    bytes(vm, second, 63)?.to_vec(),
                ];
                text.overflow = overflow;
                text.break_configured = true;
                text.layout_scratch = [0; 2];
                4
            }
            _ => {
                let ([max, lines, last], scratch) =
                    measure::measure(text, bytes(vm, value, 64 * 1024)?)
                        .map_err(|error| anyhow::anyhow!("CMVS text measurement: {error}"))?;
                vm.complete_owner_results(2, &[(0, 1), (1, max), (2, lines), (4, last)])?;
                text.layout_scratch = scratch;
                return Ok(());
            }
        };
        vm.complete_owner_call(words, None)?;
        Ok(())
    }

    pub fn advance(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &mut Objects,
        delta: u32,
        canvas: [u32; 2],
        moved_quake: bool,
    ) -> Result<()> {
        let mut owned = [false; 20];
        for (slot, window) in self.windows.iter_mut().enumerate() {
            let Some(window) = window else {
                continue;
            };
            owned[window.text] = true;
            if let Some(text) = &mut self.texts[window.text] {
                let active = text.glyphs.iter().any(|glyph| glyph.active);
                if active || (moved_quake && text.published) {
                    for glyph in &mut text.glyphs {
                        glyph.advance(delta.min(200));
                    }
                    publish_text(
                        host,
                        client,
                        slot,
                        window,
                        text,
                        canvas,
                        objects.window_offset(),
                    )?;
                }
            }
            let object = object_slot(slot);
            let mut ended = Vec::new();
            for (&child, animation) in &mut window.animations {
                let Some((current, elapsed)) = animation.playing else {
                    continue;
                };
                let mut index = current;
                let mut elapsed = elapsed.saturating_add(delta.min(200));
                let mut terminal = None;
                for _ in 0..=64 {
                    let cell =
                        animation.cells[index].context("CMVS movie drawing frame missing")?;
                    if elapsed < cell.duration {
                        break;
                    }
                    if cell.next < 0 {
                        terminal = Some(cell.next);
                        break;
                    }
                    elapsed -= cell.duration;
                    index = cell.next as usize;
                }
                if terminal == Some(-2) {
                    if child.is_some() {
                        objects.reset(host, client, object, child, false)?;
                    }
                } else if index != current {
                    let cell = animation.cells[index].context("CMVS drawing frame missing")?;
                    objects.draw(host, client, object, child, cell.property())?;
                }
                if terminal.is_some() {
                    ended.push(child);
                } else {
                    animation.playing = Some((index, elapsed));
                }
            }
            for child in ended {
                window.animations.remove(&child);
            }
        }
        for (slot, text) in self.texts.iter_mut().enumerate() {
            let Some(text) = text.as_mut().filter(|_| !owned[slot]) else {
                continue;
            };
            let active = text.glyphs.iter().any(|glyph| glyph.active);
            if active || (moved_quake && text.published) {
                for glyph in &mut text.glyphs {
                    glyph.advance(delta.min(200));
                }
                publish_object(host, client, slot, text, canvas, objects.window_offset())?;
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> Value {
        json!({"windows": self.windows, "texts": self.texts, "fields": self.fields})
    }

    /// Replaces all windows and text with a saved page, with its reveal finished.
    /// Text field mattes (`0x2397`) a saved scene needs, as (field, name).
    pub fn saved_mattes(value: &Value) -> Vec<(usize, String)> {
        value["texts"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
            .skip(12)
            .filter_map(|(slot, text)| Some((slot - 12, text["matte"][0].as_str()?.to_owned())))
            .collect()
    }

    pub fn restore(
        &mut self,
        value: &Value,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<()> {
        self.reset(host, client)?;
        self.windows = serde_json::from_value(value["windows"].clone())?;
        self.texts = serde_json::from_value(value["texts"].clone())?;
        self.fields = serde_json::from_value(value["fields"].clone()).unwrap_or_default();
        // Published mattes belong to the saving session; the restore reloads them.
        for text in self.texts.iter_mut().flatten() {
            text.matte = None;
            for glyph in &mut text.glyphs {
                glyph.image = None;
                if let Some(code) = image_font::code(glyph.character) {
                    let font = Arc::make_mut(&mut self.image_font);
                    font.prepare_code(host, client, code, glyph.size)?;
                    glyph.image = font.lookup(code, glyph.size).map(|(asset, _)| asset);
                }
            }
        }
        let offset = objects.window_offset();
        let mut owned = [false; 20];
        for (slot, window) in self.windows.iter().enumerate() {
            let Some(window) = window else {
                continue;
            };
            owned[window.text] = true;
            if let Some(text) = &mut self.texts[window.text] {
                text.published = false;
                text.glyphs.iter_mut().for_each(output::Glyph::finish);
                publish_text(host, client, slot, window, text, canvas, offset)?;
            }
        }
        for (slot, text) in self.texts.iter_mut().enumerate() {
            let Some(text) = text else {
                continue;
            };
            self.resolved_fonts[slot] = Some(text.font.clone());
            if !owned[slot] {
                text.published = false;
                text.glyphs.iter_mut().for_each(output::Glyph::finish);
                publish_object(host, client, slot, text, canvas, offset)?;
            }
        }
        Ok(())
    }

    /// Window text commands. Returns false when the native call latches error bit `0x10000`.
    pub fn window_text(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<bool> {
        let words = match opcode {
            0x2154 => 2,
            0x2159 => 4,
            _ => 1,
        };
        let [slot] = vm.owner_arguments(opcode)?;
        let slot = slot as usize;
        let Some(window) = self.windows.get(slot).and_then(Option::as_ref) else {
            vm.complete_owner_call(words, None)?;
            return Ok(false);
        };
        if opcode == 0x215e {
            let [x, y] = window.position;
            vm.complete_owner_results(1, &[(1, x as u32), (2, y as u32)])?;
            return Ok(true);
        }
        let Some(text) = self.texts[window.text].as_mut() else {
            vm.complete_owner_call(words, None)?;
            return Ok(false);
        };
        let metrics = vm
            .settings()
            .proportional_glyphs
            .then(|| host.text_measurement(client))
            .transpose()?;
        let mut result = None;
        match opcode {
            0x2154 | 0x2159 => {
                let (reference, draw, instant) = if opcode == 0x2154 {
                    let [_, reference] = vm.owner_arguments(opcode)?;
                    (reference, true, true)
                } else {
                    let [_, _, reference, keep] = vm.owner_arguments(opcode)?;
                    (reference, keep == 0, false)
                };
                if draw {
                    let mut candidate = text.clone();
                    let source = bytes(vm, reference, 64 * 1024)?;
                    Arc::make_mut(&mut self.image_font).prepare(host, client, source, &[candidate.size, candidate.ruby_size])?;
                    lay_out(&self.image_font, &mut candidate, source, instant, self.settings.output, metrics.as_deref(), self.alpha_revision)
                        .map_err(|why| anyhow::anyhow!("CMVS window text layout: {why}"))?;
                    *text = candidate;
                }
                if opcode == 0x2159 {
                    result = Some(u32::from(!text.glyphs.iter().any(|glyph| glyph.active)));
                }
            }
            _ => {
                for glyph in text.glyphs.iter_mut().filter(|glyph| glyph.active) {
                    glyph.finish();
                }
            }
        }
        publish_text(host, client, slot, window, text, canvas, objects.window_offset())?;
        vm.complete_owner_call(words, result)?;
        Ok(true)
    }

    /// Completes `0x23c0` with a decoded sheet; a missing image adds none.
    pub fn add_image_font_sheet(&mut self, image: Option<&super::visual::DecodedImage>, cell: u32) {
        if let Some(image) = image {
            Arc::make_mut(&mut self.image_font).add_sheet(image, cell);
        }
    }

    /// `0x23c1` (0x1400189c0) and `0x23c2` (0x140018bf0).
    pub fn image_font_command(&mut self, opcode: u16, vm: &mut CmvsVm) -> Result<()> {
        let font = Arc::make_mut(&mut self.image_font);
        if opcode == 0x23c1 {
            let [index, code] = vm.owner_arguments(opcode)?;
            font.add_glyph(index, code as u16);
            vm.complete_owner_call(2, None)?;
        } else {
            let [code, size, advance] = vm.owner_arguments(opcode)?;
            font.set_advance(code as u16, size, advance as u16);
            vm.complete_owner_call(3, None)?;
        }
        Ok(())
    }

    pub fn has_field(&self, index: usize) -> bool {
        index < 8 && self.texts[12 + index].is_some()
    }

    /// Completes `0x2397`: a missing image leaves the field without a matte.
    #[allow(clippy::too_many_arguments)]
    pub fn set_field_matte(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        index: usize,
        name: String,
        matte: Option<super::visual::DecodedImage>,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<()> {
        let slot = 12 + index;
        let text = self.texts[slot].as_mut().context("CMVS text field missing")?;
        let published = matte.map(|matte| matte.publish(host, client, 710_000 + slot as u32)).transpose()?;
        let previous = std::mem::replace(
            &mut text.matte,
            published.map(|image| (name, image.path.relative().to_owned(), image.size)),
        );
        if let Some((_, asset, _)) = previous.filter(|(_, asset, _)| text.matte.as_ref().is_none_or(|(_, now, _)| now != asset)) {
            host.release_image_resource(client, &asset)?;
        }
        publish_object(host, client, slot, text, canvas, objects.window_offset())
    }

    fn window_owns(&self, slot: usize) -> bool {
        self.windows.iter().flatten().any(|window| window.text == slot)
    }

    /// Text field scrolling, framing and measurement. Returns false for a missing field.
    pub fn field_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<bool> {
        let words = match opcode {
            0x239b | 0x23a0 => 1,
            0x2390 | 0x239d | 0x23bb => 2,
            0x20e4 | 0x239e | 0x23ba => 3,
            0x239c | 0x23b8 => 4,
            0x2396 | 0x239a => 5,
            _ => 6,
        };
        let [index] = vm.owner_arguments(opcode)?;
        let slot = if opcode == 0x20e4 { index as usize } else { 12 + index as usize };
        let limit = if opcode == 0x20e4 { 12 } else { 20 };
        let Some(text) = self.texts[..limit].get_mut(slot).and_then(Option::as_mut) else {
            vm.complete_owner_call(words, None)?;
            return Ok(false);
        };
        if matches!(opcode, 0x20e4 | 0x23ba) {
            let [_, source, target] = vm.owner_arguments(opcode)?;
            let metrics = host.text_measurement(client)?;
            let source = bytes(vm, source, 64 * 1024)?.to_vec();
            let (wrapped, [widest, lines, last]) = field::wrap_words(text, &source, metrics.as_ref())
                .map_err(|why| anyhow::anyhow!("CMVS text wrap: {why}"))?;
            vm.store_owner_string(target, &wrapped)?;
            vm.complete_owner_results(words, &[(0, 1), (1, widest), (2, lines), (4, last)])?;
            return Ok(true);
        }
        let record = &mut self.fields[slot - 12];
        match opcode {
            0x2390 => {
                let [_, source] = vm.owner_arguments(opcode)?;
                let [lines, widest, last] = field::count_columns(text, bytes(vm, source, 64 * 1024)?);
                vm.complete_owner_results(words, &[(0, lines), (1, widest), (4, last)])?;
            }
            0x23bb => {
                let [_, source] = vm.owner_arguments(opcode)?;
                let metrics = host.text_measurement(client)?;
                let [widest, lines, last] =
                    measure::measure_pixels(text, bytes(vm, source, 64 * 1024)?, metrics.as_ref())
                        .map_err(|why| anyhow::anyhow!("CMVS text field measure: {why}"))?;
                vm.complete_owner_results(words, &[(0, 1), (1, widest), (2, lines), (4, last)])?;
            }
            0x2396 => {
                let [_, x, y, width, height] = vm.owner_arguments(opcode)?;
                text.view = Some([x, y, width, height].map(|value| value as i32 as f32));
                publish_object(host, client, slot, text, canvas, objects.window_offset())?;
                vm.complete_owner_call(words, None)?;
            }
            0x239a => {
                let [_, low_x, high_x, low_y, high_y] = vm.owner_arguments(opcode)?;
                record.clamp = [low_x, high_x, low_y, high_y].map(|value| value as i32);
                vm.complete_owner_call(words, None)?;
            }
            0x239b => {
                vm.complete_owner_call(words, Some(u32::from(record.motion.is_some())))?;
            }
            0x239c => {
                let [_, x, y, duration] = vm.owner_arguments(opcode)?;
                record.start(text.position, [x as i32, y as i32], duration, 0);
                vm.complete_owner_call(words, None)?;
            }
            0x239f => {
                let [_, mode, x, y, _, duration] = vm.owner_arguments(opcode)?;
                record.start(text.position, [x as i32, y as i32], duration, mode);
                vm.complete_owner_call(words, None)?;
            }
            0x239d => {
                let [_, object] = vm.owner_arguments(opcode)?;
                record.attached = ((object as i32) >= 0).then_some(object);
                vm.complete_owner_call(words, None)?;
            }
            0x239e => {
                let [_, x, y] = vm.owner_arguments(opcode)?;
                let x = (x as i32).max(record.clamp[0]).min(record.clamp[1]);
                let y = (y as i32).max(record.clamp[2]).min(record.clamp[3]);
                vm.complete_owner_results(words, &[(1, x as u32), (2, y as u32)])?;
            }
            0x23a0 => {
                record.motion = None;
                vm.complete_owner_call(words, None)?;
            }
            0x23b8 => {
                let [_, prefix, suffix, mode] = vm.owner_arguments(opcode)?;
                let prefix = bytes(vm, prefix, 64)?.to_vec();
                let suffix = bytes(vm, suffix, 64)?.to_vec();
                record.rule = Some((prefix, suffix, mode));
                vm.complete_owner_call(words, None)?;
            }
            _ => anyhow::bail!("unhandled CMVS text field command 0x{opcode:04x}"),
        }
        Ok(true)
    }

    /// Clears a text field's scroll state when it is created or destroyed.
    pub fn reset_field(&mut self, index: usize) {
        if let Some(field) = self.fields.get_mut(index) {
            *field = field::Field::default();
        }
    }

    pub fn advance_fields(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        objects: &Objects,
        delta: u32,
        canvas: [u32; 2],
    ) -> Result<()> {
        for (index, field) in self.fields.iter_mut().enumerate() {
            let Some(point) = field.advance(delta) else {
                continue;
            };
            if let Some(text) = self.texts[12 + index].as_mut() {
                text.position = point;
                publish_object(host, client, 12 + index, text, canvas, objects.window_offset())?;
            }
        }
        Ok(())
    }

    pub fn set_alias_words(&mut self, words: Option<u16>) {
        self.alias_words = words;
    }

    /// Text field position commands that take floats or report the position.
    pub fn field_position(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<bool> {
        let [slot] = vm.owner_arguments(opcode)?;
        let words = if opcode == 0x2398 { 1 } else { 3 };
        let Some(text) = self.texts.get_mut(12 + slot as usize).and_then(Option::as_mut) else {
            vm.complete_owner_call(words, None)?;
            return Ok(false);
        };
        if opcode == 0x2398 {
            let [x, y] = text.position;
            vm.complete_owner_results(1, &[(1, x as u32), (2, y as u32)])?;
            return Ok(true);
        }
        let [_, x, y] = vm.owner_arguments(opcode)?;
        let point = [f32::from_bits(x) as i32, f32::from_bits(y) as i32];
        if opcode == 0x2384 {
            text.position = point;
            publish_object(host, client, 12 + slot as usize, text, canvas, objects.window_offset())?;
        } else {
            text.pen = point;
        }
        vm.complete_owner_call(words, None)?;
        Ok(true)
    }

    /// Text object commands. Returns false when the native call latches error bit `0x1000`.
    pub fn object_command(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &mut CmvsVm,
        objects: &Objects,
        canvas: [u32; 2],
    ) -> Result<bool> {
        let words = match opcode {
            0x2101 | 0x2104 | 0x2109 | 0x2111 => 1,
            0x2100 | 0x2102 | 0x2107 | 0x210c | 0x210d => 3,
            0x2103 | 0x211c => 5,
            0x2105 => 4,
            _ => 2,
        };
        let words = self.alias_words.take().unwrap_or(words);
        let [slot] = vm.owner_arguments(opcode)?;
        let slot = slot as i32;
        if slot >= 20 || slot < 0 {
            vm.complete_owner_call(words, None)?;
            return Ok(false);
        }
        let slot = slot as usize;
        let offset = objects.window_offset();
        if matches!(opcode, 0x2100 | 0x211c) {
            let [_, reference] = vm.owner_arguments(opcode)?;
            let logical = if opcode == 0x211c {
                let [_, _, _, width, height] = vm.owner_arguments(opcode)?;
                ensure!(width > 0 && height > 0, "CMVS text object logical size");
                [width, height]
            } else {
                canvas
            };
            let requested = bytes(vm, reference, 31)?;
            let face = if requested.is_empty() {
                self.default_face().to_vec()
            } else {
                requested.to_vec()
            };
            let font = resolve_face(host, client, &face)?.unwrap_or_else(|| Arc::from("default"));
            remove_object(host, client, slot, self.texts[slot].as_mut())?;
            self.resolved_fonts[slot] = Some(font.clone());
            self.texts[slot] = Some(Text::object(logical, font, self.settings.bold));
            vm.complete_owner_call(words, Some(1))?;
            return Ok(true);
        }
        if opcode == 0x2101 {
            remove_object(host, client, slot, self.texts[slot].as_mut())?;
            self.texts[slot] = None;
            vm.complete_owner_call(words, None)?;
            return Ok(true);
        }
        let owned = self.window_owns(slot);
        let Some(text) = self.texts[slot].as_mut() else {
            vm.complete_owner_call(words, None)?;
            return Ok(false);
        };
        let mut result = None;
        let mut changed = false;
        match opcode {
            0x2102 => {
                let [_, x, y] = vm.owner_arguments(opcode)?;
                text.position = [x as i32, y as i32];
                changed = true;
            }
            0x2103 => {
                let [_, x, y, width, height] = vm.owner_arguments(opcode)?;
                text.region = [x as i32, y as i32, width as i32, height as i32];
                text.pen = [x as i32, y as i32];
            }
            0x2104 => {
                text.glyphs.clear();
                text.pen = [text.region[0], text.region[1]];
                changed = true;
            }
            0x2105 => {
                let [_, size, x, y] = vm.owner_arguments(opcode)?;
                if size as i32 > 0 {
                    text.size = size;
                }
                if x as i32 >= 0 {
                    text.spacing[0] = x;
                }
                if y as i32 >= 0 {
                    text.spacing[1] = y;
                }
            }
            0x2107 => {
                let [_, first, second] = vm.owner_arguments(opcode)?;
                for (color, value) in text.colors.iter_mut().zip([first, second]) {
                    if value & 0xff00_0000 == 0 {
                        *color = value;
                    }
                }
            }
            0x2108 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.hang_punctuation = value != 0;
            }
            0x2109 => {
                result = Some(text.glyphs.iter().filter(|glyph| glyph.active).count() as u32);
            }
            0x210a | 0x210b => {
                let [_, reference] = vm.owner_arguments(opcode)?;
                let metrics = vm
                    .settings()
                    .proportional_glyphs
                    .then(|| host.text_measurement(client))
                    .transpose()?;
                let settings = text.object_settings();
                let mut candidate = text.clone();
                let source = bytes(vm, reference, 64 * 1024)?;
                Arc::make_mut(&mut self.image_font).prepare(host, client, source, &[candidate.size, candidate.ruby_size])?;
                lay_out(&self.image_font, &mut candidate, source, opcode == 0x210b, settings, metrics.as_deref(), self.alpha_revision)
                    .map_err(|why| anyhow::anyhow!("CMVS text object layout: {why}"))?;
                *text = candidate;
                changed = true;
            }
            0x210c => {
                let [_, x, y] = vm.owner_arguments(opcode)?;
                text.pen = [x as i32, y as i32];
            }
            0x210d => {
                let [_, mode, duration] = vm.owner_arguments(opcode)?;
                text.animation = [mode, duration];
            }
            0x210e => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.delay = value;
            }
            0x210f => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.decoration = value;
            }
            0x2110 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.order = value as i32;
                changed = true;
            }
            0x2111 => {
                for glyph in text.glyphs.iter_mut().filter(|glyph| glyph.active) {
                    glyph.finish();
                }
                changed = true;
            }
            0x2113 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.outline_mode = value;
            }
            0x2118 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.bold = value != 0;
            }
            0x211a => {
                let [_, value] = vm.owner_arguments(opcode)?;
                text.glyph_parameter = value;
            }
            0x20e7 => {
                let [_, value] = vm.owner_arguments(opcode)?;
                let scale = f32::from_bits(value);
                text.scale = if (0.95..=1.05).contains(&scale) { 1.0 } else { scale };
                changed = true;
            }
            0x211d => {
                let [_, reference] = vm.owner_arguments(opcode)?;
                let metrics = host.text_measurement(client)?;
                let [widest, lines, last] =
                    measure::measure_pixels(text, bytes(vm, reference, 64 * 1024)?, metrics.as_ref())
                        .map_err(|why| anyhow::anyhow!("CMVS text object measure: {why}"))?;
                vm.complete_owner_results(2, &[(0, 1), (1, widest), (2, lines), (4, last)])?;
                return Ok(true);
            }
            _ => anyhow::bail!("unhandled CMVS text object command 0x{opcode:04x}"),
        }
        if changed && !owned {
            publish_object(host, client, slot, text, canvas, offset)?;
        }
        vm.complete_owner_call(words, result)?;
        Ok(true)
    }
}

fn text_layer(slot: usize) -> u32 {
    400_000 + slot as u32
}

fn object_layer(slot: usize) -> u32 {
    400_100 + slot as u32
}

fn remove_object(
    host: &mut dyn HostServices,
    client: ClientId,
    slot: usize,
    text: Option<&mut Text>,
) -> Result<()> {
    if let Some(text) = text.filter(|text| text.published) {
        submit_text(host, client, json!({"op":"remove","id":object_layer(slot)}))?;
        text.published = false;
    }
    Ok(())
}

fn publish_object(
    host: &mut dyn HostServices,
    client: ClientId,
    slot: usize,
    text: &mut Text,
    canvas: [u32; 2],
    offset: [i32; 2],
) -> Result<()> {
    let id = object_layer(slot);
    if text.glyphs.is_empty() {
        return remove_object(host, client, slot, Some(text));
    }
    let factor: [f32; 2] =
        std::array::from_fn(|axis| canvas[axis] as f32 / text.logical[axis] as f32);
    let position: [f32; 2] =
        std::array::from_fn(|axis| text.position[axis] as f32 * factor[axis] + offset[axis] as f32);
    // 0x14003f9b0 keeps a glyph while its cell, with a two pixel margin, touches the frame.
    let framed = text.view.map(|[x, y, width, height]| {
        let mut framed = text.clone();
        framed.glyphs.retain(|glyph| {
            let size = glyph.size as f32;
            let left = (glyph.position[0] + text.position[0]) as f32;
            let top = (glyph.position[1] + text.position[1]) as f32;
            left - 2. <= x + width
                && x <= left + size + 2.
                && top - 2. <= y + height
                && y <= top + size + 2.
        });
        framed
    });
    let glyphs = glyph_values(framed.as_ref().unwrap_or(text));
    let mut layer = json!({"op":"put","layer":{
        "id":id,"rect":[position[0],position[1],text.logical[0] as f32,text.logical[1] as f32],
        "order":[!text.order,0x1000 + slot as i32],"tie_order":u64::MAX,
        "visible":text.scale != 0.,
        "transform":{"scale_x":text.scale * factor[0],"scale_y":text.scale * factor[1]},
        "paint":{"kind":"glyph_run","font":"default","size":text.size,"color":[255,255,255,255],
            "glyphs":glyphs}
    }});
    if let Some((_, asset, size)) = &text.matte {
        layer["layer"]["filter"] = json!({"stencil":{"asset":asset,"canvas":size,"origin":[0.0, 0.0]}});
    }
    submit_text(host, client, layer)?;
    text.published = true;
    Ok(())
}

fn resolve_face(
    host: &mut dyn HostServices,
    client: ClientId,
    bytes: &[u8],
) -> Result<Option<Arc<str>>> {
    let (face, invalid) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
    ensure!(!invalid, "invalid CP932 font face");
    Ok(host
        .font_library(client)?
        .resolve_alias(&face)?
        .map(Arc::from))
}

fn submit_text(host: &mut dyn HostServices, client: ClientId, command: Value) -> Result<()> {
    host.submit_engine(
        client,
        1,
        &serde_json::to_vec(&json!({"layers":[command]}))?,
    )?;
    Ok(())
}

fn rgb(color: u32, alpha: u32) -> [u8; 4] {
    [
        (color >> 16) as u8,
        (color >> 8) as u8,
        color as u8,
        alpha as u8,
    ]
}

fn logical_pointer(pointer: [i32; 2], logical: [u32; 2], canvas: [u32; 2]) -> [i32; 2] {
    std::array::from_fn(|axis| {
        ((pointer[axis] as u32).wrapping_mul(logical[axis]) / canvas[axis]) as i32
    })
}

fn publish_text(
    host: &mut dyn HostServices,
    client: ClientId,
    slot: usize,
    window: &Window,
    text: &mut Text,
    canvas: [u32; 2],
    offset: [i32; 2],
) -> Result<()> {
    let id = text_layer(slot);
    if text.glyphs.is_empty() {
        if text.published {
            submit_text(host, client, json!({"op":"remove","id":id}))?;
            text.published = false;
        }
        return Ok(());
    }
    let factor: [f32; 2] =
        std::array::from_fn(|axis| canvas[axis] as f32 / text.logical[axis] as f32);
    let position: [f32; 2] =
        std::array::from_fn(|axis| text.position[axis] as f32 * factor[axis] + offset[axis] as f32);
    let glyphs = glyph_values(text);
    let transform = [text.scale * factor[0], text.scale * factor[1]];
    submit_text(
        host,
        client,
        json!({"op":"put","layer":{
            "id":id,"rect":[position[0],position[1],text.logical[0] as f32,text.logical[1] as f32],
            "order":[!text.order,object_slot(slot) as i32],"tie_order":u64::MAX,
            "visible":window.shown && !window.hidden && text.scale != 0.,
            "transform":{"scale_x":transform[0],"scale_y":transform[1]},
            "paint":{"kind":"glyph_run","font":"default","size":text.size,"color":[255,255,255,255],"glyphs":glyphs}
        }}),
    )?;
    text.published = true;
    Ok(())
}

/// Lays `bytes` out after the current text. Characters the image font has keep its cell
/// advance under proportional layout (0x14006bd20) and draw from its cell image.
fn lay_out(
    font: &image_font::ImageFont,
    text: &mut Text,
    bytes: &[u8],
    skip: bool,
    settings: output::OutputSettings,
    metrics: Option<&dyn crysirror_core::service::TextMeasurement>,
    alpha_revision: Option<bool>,
) -> Result<(), &'static str> {
    let first = text.glyphs.len();
    let wrapped = metrics.map(|inner| image_font::Metrics { inner, font });
    output::append(text, bytes, skip, settings, wrapped.as_ref().map(|metrics| metrics as _))?;
    for glyph in &mut text.glyphs[first..] {
        glyph.alpha_revision = alpha_revision;
        glyph.image = image_font::code(glyph.character)
            .and_then(|code| font.lookup(code, glyph.size))
            .map(|(asset, _)| asset);
    }
    Ok(())
}

fn glyph_values(text: &Text) -> Vec<Value> {
    // GGO_GRAY4_BITMAP bins, in the order used by PC20211007's 0x14006b8e0.
    const LINEAR_ALPHA: [u8; 17] = [
        0, 15, 31, 47, 63, 79, 95, 111, 127, 143, 159, 175, 191, 207, 223, 239, 255,
    ];
    const REVISED_ALPHA: [u8; 17] = [
        0, 31, 60, 87, 112, 135, 156, 175, 192, 207, 220, 231, 240, 247, 252, 254, 255,
    ];
    text
        .glyphs
        .iter()
        .filter(|glyph| glyph.shown && glyph.opacity != 0)
        .map(|glyph| {
            let [x, y] = glyph.position;
            let outline = glyph.outline.map(
                |color| json!({"radius":1,"color":rgb(color,glyph.opacity),"kernel":"square"}),
            );
            let crop = (glyph.active && glyph.reveal_mode == 2).then(|| {
                [
                    x as f32 - 2.,
                    y as f32 - 2.,
                    glyph.reveal_width as f32,
                    glyph.size as f32 + 4.,
                ]
            });
            let mut value = json!({"character":glyph.character,"baseline":[x as f32,y as f32 + glyph.size as f32],
                "size":glyph.size,"color":rgb(glyph.color,glyph.opacity),"crop":crop,
                "font":glyph.font.as_ref(),"style":{"bold":glyph.bold,"outline":outline}});
            // The image cell is coverage tinted by the glyph colour.
            if let Some(asset) = &glyph.image {
                let size = glyph.size as f32;
                value["image"] = json!({"asset":asset.as_ref(),"rect":[x as f32,y as f32,size,size],
                    "uv":[0.0,0.0,1.0,1.0],"tint":rgb(glyph.color,glyph.opacity)});
            } else if let Some(enabled) = glyph.alpha_revision {
                let table = if enabled { REVISED_ALPHA } else { LINEAR_ALPHA };
                value["style"]["raster"] = json!({"kind":"supersample_alpha4","table":table});
            }
            value
        })
        .collect()
}

impl Window {
    fn button_hit(&self, point: [i32; 2]) -> Option<usize> {
        self.buttons
            .iter()
            .enumerate()
            .rfind(|(_, button)| {
                let [x, y, w, h] = button.rect.map(|value| i32::from(value as u16));
                let left = self.position[0].wrapping_add(x);
                let top = self.position[1].wrapping_add(y);
                button.enabled
                    && left <= point[0]
                    && top <= point[1]
                    && point[0] <= left.wrapping_add(w)
                    && point[1] <= top.wrapping_add(h)
            })
            .map(|(i, _)| i)
    }

    fn animation(
        &mut self,
        opcode: u16,
        host: &mut dyn HostServices,
        client: ClientId,
        vm: &CmvsVm,
        objects: &mut Objects,
        object: usize,
    ) -> Result<u16> {
        let [_, raw_child] = vm.owner_arguments(opcode)?;
        let child = (raw_child as i32 >= 0).then_some(raw_child as usize);
        ensure!(
            child.is_none_or(|index| objects.has_child(object, index)),
            "CMVS window animation child missing"
        );
        let words = match opcode {
            0x21cf => {
                self.animations.insert(child, Animation::default());
                2
            }
            0x21d0 => {
                let [_, _, frame, next, duration, x, y, w, h, ox, oy] =
                    vm.owner_arguments(opcode)?;
                if let Some(animation) = self.animations.get_mut(&child) {
                    ensure!((next as i32) < 64, "CMVS next drawing frame outside 0..63");
                    let cell = animation
                        .cells
                        .get_mut(frame as usize)
                        .context("CMVS drawing frame outside 0..63")?;
                    *cell = Some(Cell {
                        next: next as i32,
                        duration,
                        crop: [x, y, w, h].map(|v| v as i16),
                        origin: [ox, oy].map(|v| v as i16),
                    });
                }
                11
            }
            0x21d1 => {
                let [_, _, frame] = vm.owner_arguments(opcode)?;
                if let Some(animation) = self.animations.get_mut(&child) {
                    let cell = animation
                        .cells
                        .get(frame as usize)
                        .and_then(|cell| *cell)
                        .context("CMVS drawing frame missing")?;
                    objects.draw(host, client, object, child, cell.property())?;
                    animation.playing = Some((frame as usize, 0));
                }
                3
            }
            _ => {
                self.animations.remove(&child);
                2
            }
        };
        Ok(words)
    }
}

fn window_object(
    opcode: u16,
    host: &mut dyn HostServices,
    client: ClientId,
    vm: &CmvsVm,
    objects: &mut Objects,
    object: usize,
) -> Result<u16> {
    let [_, child] = vm.owner_arguments(opcode)?;
    let child = child as i32;
    ensure!(child < 768, "CMVS window child outside 0..767");
    let child = (child >= 0).then_some(child as usize);
    if opcode == 0x2178 {
        objects.reset(host, client, object, child, true)?;
        objects.draw(host, client, object, child, DrawProperty::Shown(false))?;
        return Ok(2);
    }
    let (prop, words) = match opcode {
        0x217a => {
            let [_, _, value] = vm.owner_arguments(opcode)?;
            (DrawProperty::Shown(value != 0), 3)
        }
        0x217c => {
            let [_, _, width, height] = vm.owner_arguments(opcode)?;
            (DrawProperty::LogicalSize([width, height]), 4)
        }
        0x217f => (DrawProperty::ClearQuakeFollow, 2),
        0x2180 => (DrawProperty::Reset, 2),
        0x2182 => {
            let [_, _, x, y, w, h] = vm.owner_arguments(opcode)?;
            (DrawProperty::Crop([x, y, w, h].map(|v| v as i16)), 6)
        }
        0x2183 => {
            let [_, _, x, y] = vm.owner_arguments(opcode)?;
            (DrawProperty::Position([x as i32, y as i32]), 4)
        }
        0x2184 => {
            let [_, _, value] = vm.owner_arguments(opcode)?;
            (DrawProperty::Priority(value as i32), 3)
        }
        0x2185 => {
            let [_, _, x, y] = vm.owner_arguments(opcode)?;
            (DrawProperty::Origin([x as i16, y as i16]), 4)
        }
        0x2186 => {
            let [_, _, value] = vm.owner_arguments(opcode)?;
            (DrawProperty::Opacity(value as i32), 3)
        }
        _ => unreachable!("recognized message object operation"),
    };
    objects.draw(host, client, object, child, prop)?;
    Ok(words)
}

pub(super) fn object_slot(window: usize) -> usize {
    256 + window
}

fn bytes(vm: &CmvsVm, raw: u32, limit: usize) -> Result<&[u8]> {
    let bytes = vm
        .text_reference(raw)
        .context("CMVS text reference missing")?;
    ensure!(bytes.len() <= limit, "CMVS text byte limit");
    Ok(bytes)
}
