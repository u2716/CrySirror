//! DC2 ordinary-control policy over shared Ui/Image/Audio. Rect hit tests,
//! pointer capture, focus and activation are engine services; native ordinals,
//! state9/1/2/3/4 paints, SFX IDs and MES labels remain Circus policy.
use super::*;
use crate::dc2::menu::MenuWait;
use crate::dc2_bitmap::{Draw, Request as BitmapRequest};
use std::collections::VecDeque;
const ROOT: u32 = 0x3c02_1000;
const FIRST: u32 = ROOT + 1;
#[derive(Clone)]
enum Sound {
    Id(u16),
    Voice(String),
}
#[derive(Default)]
pub(super) struct View {
    owner: Option<crate::dc2_bitmap::Owner>,
    states: Vec<i16>,
    redraw: Option<Vec<i16>>,
    sounds: VecDeque<Sound>,
    sound: Option<crate::dc2_audio::Request>,
    selected: Option<usize>,
    last_sfx: u32,
    ///Cells last published for a process9 choice (message + rows).
    text: Option<Vec<crate::dc2::Glyph>>,
}
pub(super) enum Frame {
    Busy,
    Waiting,
    Selected(usize),
}
impl View {
    ///Glyphs a choice left in the message layer; they stay until the next
    ///native redraw, exactly as 41AD40's cells stay in native0.
    pub(super) fn take_text(&mut self) -> Option<Vec<crate::dc2::Glyph>> {
        self.text.take()
    }
    pub(super) fn close(&mut self, bridge: &mut Bridge<'_>) -> Result<()> {
        if self.owner.is_some() {
            bridge.submit(vec![], vec![json!({"op":"remove","id":ROOT})])?;
        }
        *self = Self::default();
        Ok(())
    }
    pub(super) fn tick(
        &mut self,
        bridge: &mut Bridge<'_>,
        menu: &MenuWait,
        settings: &crate::dc2_settings::SystemData,
    ) -> Result<Frame> {
        if let Some(owner) = &self.owner {
            ensure!(owner == &menu.owner, "DC2 Ui retained instruction owner");
        } else {
            let mut widgets = vec![
                json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,800,600],"modal":true,"interaction_only":true,"keyboard_focusable":false,"keyboard_navigation":false,"keyboard_activation":false}}),
            ];
            for (index, (_, button)) in menu.rows.iter().enumerate() {
                let [left, top, right, bottom] = button.bounds.map(i32::from);
                ensure!(
                    right > left + 1 && bottom > top + 1,
                    "DC2 native button has no strict interior"
                );
                //410980/411470 use strict inequalities at all four edges.
                widgets.push(json!({"op":"put","widget":{"id":FIRST+index as u32,"parent":ROOT,"rect":[left+1,top+1,right-left-1,bottom-top-1],"z":100-index,"kind":"button","interaction_only":true,"focus_follows_pointer":true}}));
                widgets.push(json!({"op":"blur","id":FIRST+index as u32}));
            }
            bridge.submit(vec![], widgets)?;
            self.owner = Some(menu.owner.clone());
            self.states = vec![9; menu.rows.len()];
            //41C530 case10 -> 41B3C0(0,0) -> 411410(1): the idle (state 9)
            //records are painted as soon as process10 opens.
            self.redraw = Some(self.states.clone());
            self.last_sfx = 2;
        }
        if let Some(states) = &self.redraw {
            let draw = paint(menu, states)?;
            if !bridge.dc2_bitmap(&BitmapRequest::Present {
                owner: menu.owner.clone(),
                slot: 1,
                draw,
            })? {
                return Ok(Frame::Busy);
            }
            if let Some(base) = &menu.text {
                //40FF00 -> 41AD40 draws the row text into native0 above the
                //row images; the retained glyph run sits above native0 too.
                let mut glyphs = base.glyphs.clone();
                for (row, record, cells) in &menu.glyphs {
                    let state = states[*row];
                    if visible(menu.rows[*row].1.records[*record].state, state) {
                        glyphs.extend(cells.iter().cloned());
                    }
                }
                let frame = crate::dc2::TextFrame { window: base.window.clone(), glyphs };
                if !bridge.dc2_text_frame(Some(&frame))? {
                    return Ok(Frame::Busy);
                }
                self.text = Some(frame.glyphs);
            }
            self.states = self.redraw.take().unwrap();
        }
        loop {
            if let Some(request) = &self.sound {
                if !bridge.dc2_direct_audio(request)? {
                    return Ok(Frame::Busy);
                }
                self.sound = None;
                self.sounds.pop_front();
            }
            let Some(sound) = self.sounds.front().cloned() else {
                break;
            };
            if i32::from_le_bytes(settings.bytes()[48..52].try_into()?) == 0 {
                self.sounds.pop_front();
                continue;
            }
            let id = match sound {
                Sound::Voice(name) => {
                    //4277F0 tests SFX enable, then calls427540 with centre
                    //pan. This replaces the actual sole voice channel8.
                    ensure!(name.len() < 260 && !name.contains('\0'), "DC2 button voice resource path");
                    self.sound = crate::dc2_audio::voice_request(settings, &name, 0)?;
                    if self.sound.is_none() { self.sounds.pop_front(); }
                    continue;
                }
                Sound::Id(id) => id,
            };
            if id == 255 { self.sounds.pop_front(); continue; }
            match sfx_request(bridge, settings, id, &mut self.last_sfx)? {
                None => return Ok(Frame::Busy),
                Some(None) => {
                    self.sounds.pop_front();
                }
                Some(Some(request)) => self.sound = Some(request),
            }
        }
        if let Some(ordinal) = self.selected.take() {
            return Ok(Frame::Selected(ordinal));
        }
        let events: Vec<Value> = serde_json::from_slice(
            &bridge
                .host
                .query_engine(bridge.client, EngineQuery::UiSubtreeEvents(ROOT))?,
        )?;
        let mut states = self.states.clone();
        for (index, (ordinal, button)) in menu.rows.iter().enumerate() {
            let id = FIRST + index as u32;
            let status: Value = serde_json::from_slice(
                &bridge
                    .host
                    .query_engine(bridge.client, EngineQuery::UiInteraction(id))?,
            )?;
            let hovered = status["hovered"] == true || status["focused"] == true;
            let state = if hovered && status["pressed"] == true {
                2
            } else if hovered {
                1
            } else {
                9
            };
            if state != self.states[index] {
                if self.states[index] == 9 && state != 9 && button.sound_ids[0] != 255 {
                    self.sounds.push_back(Sound::Id(button.sound_ids[0].into()));
                }
                if self.states[index] == 9 && state != 9 {
                    if let Some(name) = &button.sounds[0] { self.sounds.push_back(Sound::Voice(name.clone())); }
                }
                states[index] = state;
            }
            if events.iter().any(|event| {
                event["widget"]["id"].as_u64() == Some(u64::from(id)) && event["kind"] == "activate"
            }) {
                ensure!(
                    self.selected.is_none(),
                    "DC2 ambiguous native button activation"
                );
                self.selected = Some(*ordinal);
                if button.sound_ids[1] != 255 {
                    self.sounds.push_back(Sound::Id(button.sound_ids[1].into()));
                }
                if let Some(name) = &button.sounds[1] { self.sounds.push_back(Sound::Voice(name.clone())); }
            }
        }
        ensure!(self.sounds.len() <= 32, "DC2 native Ui sound event budget");
        if states != self.states {
            self.redraw = Some(states);
        }
        Ok(
            if self.redraw.is_some() || !self.sounds.is_empty() || self.selected.is_some() {
                Frame::Busy
            } else {
                Frame::Waiting
            },
        )
    }
}
///4107F0/4108A0: idle shows state9; hover/press show state s and s+2.
fn visible(record: i16, state: i16) -> bool {
    if state == 9 {
        record == 9
    } else {
        record == state || record == state + 2
    }
}
///427070(0, id, 0): SFX directory row `id` with its attenuation. `None`
///while the directory loads; `Some(None)` when the row names nothing.
pub(super) fn sfx_request(
    bridge: &mut Bridge<'_>,
    settings: &crate::dc2_settings::SystemData,
    id: u16,
    last_sfx: &mut u32,
) -> Result<Option<Option<crate::dc2_audio::Request>>> {
    if bridge.services.dc2_names.is_none() {
        let Some(Prepared::Names(names)) = bridge.services.prepare(Work::Names, bridge.client)? else {
            return Ok(None);
        };
        bridge.services.names = Some(names);
    }
    let names = bridge.services.dc2_names.as_ref().context("DC2 Ui SFX directory")?;
    let Some(row) = names.sfx.get(usize::from(id)) else { return Ok(Some(None)) };
    let name = row.name()?;
    if name.is_empty() {
        return Ok(Some(None));
    }
    let attenuation = i32::from_le_bytes(row.bytes()[128..132].try_into()?);
    let volume = i32::from_le_bytes(settings.bytes()[52..56].try_into()?).wrapping_mul(2);
    let gain_db100 = if volume <= 0 {
        -10000
    } else {
        attenuation
            .wrapping_mul(-100)
            .wrapping_add(20i32.wrapping_mul(5i32.wrapping_mul(volume).wrapping_sub(300)) / 2)
            .clamp(-3000, 0)
    };
    //427070(0,id,0): use idle native1/2 first, otherwise alternate.
    //No admitted producer has reserved native1 for looping SFX.
    let first = NativeHost::audio_playing(bridge, 2)?;
    let second = NativeHost::audio_playing(bridge, 3)?;
    let channel = if !first {
        2
    } else if !second {
        3
    } else if *last_sfx == 2 {
        3
    } else {
        2
    };
    *last_sfx = channel;
    Ok(Some(Some(crate::dc2_audio::Request {
        name,
        channel,
        voice: false,
        gain_db100,
        balance_db100: 0,
        looping: false,
        streamed: false,
    })))
}
fn paint(menu: &MenuWait, states: &[i16]) -> Result<Vec<Draw>> {
    ensure!(
        states.len() == menu.rows.len(),
        "DC2 native Ui paint state extent"
    );
    let mut draw = menu.base.clone();
    for (row, ((_, button), state)) in menu.rows.iter().zip(states).enumerate() {
        for (index, record) in button.records.iter().enumerate() {
            if !visible(record.state, *state) {
                continue;
            }
            if record.text.is_some() {
                ensure!(
                    menu.glyphs.iter().any(|(r, i, _)| *r == row && *i == index),
                    "DC2 native Ui text record has no 41AD40 cells"
                );
                continue;
            }
            ensure!(
                record.resource & 0x40 == 0 && record.resource & 0x7f != 9,
                "DC2 native Ui paint profile is not mapped"
            );
            let [x, y, width, height] = record.source;
            if width <= 0 || height <= 0 {
                continue;
            }
            let slot = (record.resource & 0x7f) as u8;
            let slot = if slot == 0 { 1 } else { slot };
            let source = [
                i32::from(record.position[0]) - i32::from(x),
                i32::from(record.position[1]) - i32::from(y),
                width.into(),
                height.into(),
            ];
            let destination = record.position.map(i32::from);
            draw.push(match menu.weight {
                Some(weight) if record.resource & 0x80 != 0 => Draw::Blend { slot, source, destination, weight },
                _ => Draw::Image { slot, source, destination },
            });
        }
    }
    Ok(draw)
}
