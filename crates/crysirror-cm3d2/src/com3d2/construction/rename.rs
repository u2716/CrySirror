// SPDX-License-Identifier: MPL-2.0
//! SceneFacilityManagement.FacilityDetailsUI owns the input until end-edit.
use super::*;
use crysirror_core::service::{TextInputSessionHandle, TextInputSessionSpec};

pub(super) struct Edit {
    pub slot: usize,
    pub value: String,
    cursor: usize,
    selected: bool,
    handle: TextInputSessionHandle,
    rect: Rect,
}
impl Screen {
    pub(super) fn start_name(
        &mut self,
        s: &TjsState,
        host: &mut dyn HostServices,
        c: ClientId,
        slot: usize,
    ) -> Result<()> {
        let mut rect = self
            .layout
            .rect(&format!("{ROOM}/InputField Facility Name"))?;
        let src = self.layout.rect(ROOM)?;
        let cell = self.layout.cell(ROOM, slot, SLOTS)?;
        rect[0] += cell[0] - src[0];
        rect[1] += cell[1] - src[1];
        let value = self.catalog.display_name(&s.retail, slot)?.to_owned();
        let handle = host.open_text_input_session(
            c,
            TextInputSessionSpec {
                capture: true,
                cursor_area: Some(rect.map(|v| v as i32)),
                ..Default::default()
            },
        )?;
        self.editing = Some(Edit {
            slot,
            cursor: value.len(),
            value,
            selected: true,
            handle,
            rect,
        });
        Ok(())
    }

    pub(in crate::com3d2) fn close_name(
        &mut self,
        host: &mut dyn HostServices,
        c: ClientId,
    ) -> Result<()> {
        if let Some(e) = &self.editing {
            host.release_text_input_session(c, e.handle)?;
        }
        self.editing = None;
        Ok(())
    }
    pub(super) fn update_name(
        &mut self,
        s: &mut TjsState,
        widgets: &mut Vec<Widget>,
        assets: &mut Assets,
        op: &mut Opener,
        host: &mut dyn HostServices,
        c: ClientId,
        input: &InputFrame,
    ) -> Result<bool> {
        let e = self
            .editing
            .as_mut()
            .context("missing facility name field")?;
        let added = host.take_text_input_committed(c, e.handle)?;
        let keys = input.buttons_pressed;
        let left = |s: &str, n: usize| s[..n].char_indices().next_back().map_or(0, |(i, _)| i);
        if keys & (buttons::LEFT | buttons::RIGHT | buttons::HOME | buttons::END) != 0 {
            e.cursor = if keys & buttons::HOME != 0 || (e.selected && keys & buttons::LEFT != 0) {
                0
            } else if keys & buttons::END != 0 || e.selected {
                e.value.len()
            } else if keys & buttons::LEFT != 0 {
                left(&e.value, e.cursor)
            } else {
                e.cursor + e.value[e.cursor..].chars().next().map_or(0, char::len_utf8)
            };
            e.selected = false;
        }
        let changed = !added.is_empty() || keys & buttons::BACKSPACE != 0;
        if changed {
            if e.selected {
                e.value.clear();
                e.cursor = 0;
                e.selected = false;
            } else if keys & buttons::BACKSPACE != 0 {
                let from = left(&e.value, e.cursor);
                e.value.drain(from..e.cursor);
                e.cursor = from;
            }
            let mut units = e.value.encode_utf16().count();
            for ch in added.chars().filter(|ch| !ch.is_control()) {
                units += ch.len_utf16();
                if self.layout.name_limit > 0 && units > self.layout.name_limit {
                    break;
                }
                e.value.insert(e.cursor, ch);
                e.cursor += ch.len_utf8();
            }
        }
        let outside = keys & buttons::MOUSE_LEFT != 0 && !hit(e.rect, input);
        if outside || keys & (buttons::ENTER | buttons::ESCAPE | buttons::TAB) != 0 {
            if keys & buttons::ESCAPE == 0 {
                Catalog::rename(&mut s.retail, e.slot, &e.value)?;
            }
            self.close_name(host, c)?;
            *widgets = self.widgets(s, assets, op, host, c)?;
            return Ok(!outside);
        }
        if changed {
            *widgets = self.widgets(s, assets, op, host, c)?;
        }
        Ok(true)
    }
}
