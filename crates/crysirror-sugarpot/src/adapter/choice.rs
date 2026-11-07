// SPDX-License-Identifier: MPL-2.0
//! 44C880 '/' intrinsic and 4328A0 zero-based reg result. Shared widgets own
//! pointer capture, focus, activation and keyboard traversal; shared cell layout
//! owns glyph placement and wrapping. Original records remain logical data.
use super::*;
const FIRST: u32 = 61000;
pub(super) const ROOT: u32 = FIRST - 1;
impl SugarPotVm {
    fn choice_paint(
        &self,
        window: &Window,
        record: &crate::native_storage::slot::Selection,
        row: i32,
        active: bool,
    ) -> Result<(Json, [i32; 2], [i32; 2])> {
        let fields = window.native.fields;
        let size = self.font_size;
        let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(&record.text);
        ensure!(
            !invalid && encoded.len() < 256,
            "SugarPot selection text budget"
        );
        let groups = crate::text::cells::groups(
            &record.text,
            encoded.len(),
            size,
            fields[15],
            fields[17],
            fields[16],
            crate::text::cells::Profile::Body,
            false,
            [0, row],
        )?;
        let mut cursor = crysirror_text::grid::CellCursor::at_start();
        cursor.position = [record.rect[0] - fields[12], record.rect[1] - fields[13]];
        let layout = crysirror_text::grid::layout_cell_groups(
            &groups.cells,
            &crysirror_text::grid::CellLayoutOptions { cell_widths: Vec::new(),
                bounds: [32767, 32767],
                size,
                spacing: [fields[15], fields[14]],
                half_width: true,
                half_spacing: false,
                hanging: 0,
                forbidden_line_start: String::new(),
                indent_pairs: vec![],
            },
            &mut cursor,
            false,
        )?;
        ensure!(
            layout.consumed_groups == groups.cells.len(),
            "SugarPot selection layout budget"
        );
        let shadow = self.font_style[0] as u8 as i32;
        let glyphs = layout.glyphs.iter().map(|g|json!({"character":g.character,
            "baseline":[fields[12]+g.position[0],fields[13]+g.position[1]+size],
            "style":{"shadow":if shadow==0 {Json::Null}else{json!({"offset":[shadow,shadow],
                "color":[self.font_style[1]as u8,(self.font_style[1]>>8)as u8,(self.font_style[1]>>16)as u8,255]})}}})).collect::<Vec<_>>();
        Ok((
            json!({"kind":"glyph_run","font":self.config.font,"size":size,
                "color":if active {self.font_color}else{[192,192,192,255]},"glyphs":glyphs}),
            [
                fields[12] + cursor.position[0],
                fields[13] + cursor.position[1],
            ]
            .map(|n| n as i16 as i32),
            [groups.column, groups.row],
        ))
    }
    pub(super) fn choices(&mut self, h: &mut dyn HostServices, options: &[String]) -> Result<()> {
        self.paint_choices(h, options, None)
    }
    pub(super) fn restore_choices(
        &mut self,
        h: &mut dyn HostServices,
        saved: &[crate::native_storage::slot::Selection],
    ) -> Result<()> {
        let options = saved
            .iter()
            .map(|record| record.text.clone())
            .collect::<Vec<_>>();
        self.paint_choices(h, &options, Some(saved))
    }
    fn paint_choices(
        &mut self,
        h: &mut dyn HostServices,
        options: &[String],
        saved: Option<&[crate::native_storage::slot::Selection]>,
    ) -> Result<()> {
        ensure!(
            !options.is_empty() && options.len() <= 15,
            "SugarPot native choice record budget"
        );
        let window = self
            .windows
            .get(self.current_window)
            .context("SugarPot selection without text window")?;
        let fields = window.native.fields;
        let show_background = window.visibility != 1;
        let background = if show_background {
            self.window_background(window)?
        } else {
            vec![]
        };
        let size = self.font_size;
        let width = fields[17]
            .checked_mul(size + fields[15])
            .context("SugarPot selection width")?;
        let line_height = size + fields[14];
        ensure!(
            width > 0 && line_height > 0,
            "SugarPot selection cell dimensions"
        );
        let mut records = Vec::new();
        let mut paints = Vec::new();
        let mut layers = Vec::new();
        let mut ui = vec![
            json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1024,600],"z":3100,
            "kind":"panel","color":[0,0,0,0],"key_event_scope":true,"keyboard_navigation":false,
            "key_events":[{"buttons":buttons::ENTER|buttons::SPACE,"event":"sel_activate"},
                {"buttons":buttons::UP,"event":"sel_up"},{"buttons":buttons::DOWN,"event":"sel_down"}]}}),
        ];
        let mut y = window.text_cursor[1];
        let mut cell = [0, 0];
        let mut end_cursor = window.text_cursor;
        for (index, text) in options.iter().enumerate() {
            let (encoded, _, invalid) = encoding_rs::SHIFT_JIS.encode(text);
            ensure!(
                !invalid && encoded.len() < 256,
                "SugarPot native choice CP932/256-byte text bound"
            );
            //44C880 expands every later option whenever the current raw pair
            //counter reaches cols, even if the following pair is its slash
            //delimiter. An exactly full line therefore has a trailing row.
            let height = ((encoded.len().div_ceil(2) / usize::try_from(fields[17])?) + 1) as i32
                * line_height;
            let rect = saved.map_or_else(
                || [window.text_cursor[0], y, width, height].map(|n| n as i16 as i32),
                |saved| saved[index].rect,
            );
            ensure!(
                rect[2] > 0 && rect[3] > 0,
                "SugarPot native selection WORD rectangle bounds"
            );
            let record = crate::native_storage::slot::Selection {
                rect,
                text: text.clone(),
            };
            let (paint, cursor, counter) = self.choice_paint(window, &record, cell[1], false)?;
            end_cursor = cursor;
            //43B570 resets column before each body and increments the BYTE
            //row after it. This is not the option's screen y coordinate.
            cell = [counter[0], (counter[1] + 1) as u8 as i32];
            y = y
                .checked_add(height)
                .context("SugarPot selection position")?;
            let id = FIRST + index as u32;
            let absolute = [fields[1] + rect[0], fields[2] + rect[1], rect[2], rect[3]];
            layers.push(json!({"op":"put","layer":{"id":id,"rect":[fields[1],fields[2],fields[3],fields[4]],"order":[3100,index],"paint":paint}}));
            paints.push((paint, false));
            ui.push(
                json!({"op":"put","widget":{"id":id,"parent":ROOT,"rect":absolute,"z":index,
                "kind":"button","interaction_only":true,"focus_follows_pointer":true,
                "keyboard_navigation":false,"keyboard_activation":false}}),
            );
            records.push(record);
        }
        layers.extend(background.clone());
        if show_background {
            layers.extend(self.system_window_layers(true)?);
        }
        self.submit(h, layers, ui)?;
        if show_background {
            self.window_layers = background;
            self.published_layers.extend(
                self.window_layers
                    .iter()
                    .filter_map(|layer| layer["layer"]["id"].as_u64().map(|id| id as u32)),
            );
            self.windows[self.current_window].text_paint.clear();
            self.text.clear();
            self.text_glyphs = 0;
        }
        self.selections = records;
        self.selection_paints = paints;
        self.selection_focus = None;
        self.native_text.waiting = 32;
        self.windows[self.current_window].visibility = 1;
        self.track_system_window_layers(true, false);
        self.windows[self.current_window].text_cursor = end_cursor;
        self.windows[self.current_window].text_cell = cell;
        self.wait = Some(Wait::Selection);
        Ok(())
    }
    pub(super) fn poll_choices(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool> {
        let mut selected = None;
        let mut hovered = None;
        let mut layers = Vec::new();
        let old_focus = self.selection_focus;
        for index in 0..self.selections.len() {
            let id = FIRST + index as u32;
            let state: Json = serde_json::from_slice(
                &h.query_engine(self.client, EngineQuery::UiInteraction(id))?,
            )?;
            //Native selection is the current pointer hit, not retained UI
            //keyboard focus. Up/down explicitly warp that pointer below.
            let active = state["hovered"] == true;
            if active {
                hovered = Some(index);
            }
            if state["activated"] == true {
                selected = Some(index);
            }
        }
        self.selection_focus = hovered;
        let events: Vec<Json> =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiEvents)?)?;
        let (mut up, mut down) = (false, false);
        for event in events {
            if event["widget"]["owner"] != self.client.0 || event["widget"]["id"] != ROOT {
                continue;
            }
            match event["kind"].as_str().unwrap_or("") {
                "sel_activate" => selected = selected.or(self.selection_focus),
                "sel_up" => up = true,
                "sel_down" => down = true,
                _ => {}
            }
        }
        if selected.is_none() && (up || down) {
            let index = self.selection_focus.map_or(0, |old| {
                if up {
                    old.saturating_sub(1)
                } else {
                    (old + 1).min(self.selections.len() - 1)
                }
            });
            self.selection_focus = Some(index);
            let rect = self.selections[index].rect;
            let f = self.windows[self.current_window].native.fields;
            //4316E0/401DE0 use left+4,bottom-4; native up/down stop at
            //the first/last item rather than wrapping generic focus order.
            self.submit(
                h,
                vec![],
                vec![
                    json!({"op":"focus","id":FIRST+index as u32}),
                    json!({"op":"warp_pointer","position":[f[1]+rect[0]+4,f[2]+rect[1]+rect[3]-5]}),
                ],
            )?;
        }
        if let Some(index) = selected {
            let mut text = self.selections[index].text.clone();
            if self.mode == 2 {
                text.push_str("\\f");
            }
            self.capture_history_voice(&text)?;
            self.machine.as_mut().unwrap().set_reg(index as i32);
            self.clear_window_controls(h)?;
            let mut ids = (0..self.selections.len())
                .map(|n| FIRST + n as u32)
                .collect::<Vec<_>>();
            ids.extend(
                self.published_layers
                    .iter()
                    .copied()
                    .filter(|id| (30002..30006).contains(id)),
            );
            self.submit(
                h,
                ids.iter()
                    .map(|id| json!({"op":"remove","id":id}))
                    .collect(),
                vec![json!({"op":"remove","id":ROOT})],
            )?;
            self.published_layers.retain(|id| !ids.contains(id));
            self.window_layers.retain(|layer| {
                layer["layer"]["id"]
                    .as_u64()
                    .is_none_or(|id| !(30002..40000).contains(&id))
            });
            let window = &mut self.windows[self.current_window];
            window.text_cursor = [window.native.fields[12], window.native.fields[13]];
            window.text_cell = [0, 0];
            window.text_paint.clear();
            self.native_text.committed.clear();
            self.native_text.cursor = 0;
            self.native_text.resume_start = 0;
            self.native_text.waiting = 0;
            self.native_text.inline_clear = 0;
            self.text.clear();
            self.text_remainder.clear();
            self.novel_body.clear();
            self.caption.clear();
            self.text_glyphs = 0;
            self.selections.clear();
            self.selection_paints.clear();
            self.selection_focus = None;
            input.buttons_pressed = 0;
            return Ok(true);
        }
        if old_focus != self.selection_focus {
            //43B6F0 redraws the old item, then the new item; each call starts
            //with BYTE row=index+1, even for multi-line records. Preserve the
            //last body's actual WORD cursor for subsequent native commands.
            for (index, active) in old_focus
                .into_iter()
                .map(|i| (i, false))
                .chain(self.selection_focus.into_iter().map(|i| (i, true)))
            {
                let (paint, cursor, cell) = self.choice_paint(
                    &self.windows[self.current_window],
                    &self.selections[index],
                    index as i32 + 1,
                    active,
                )?;
                layers.push(json!({"op":"set_paint","id":FIRST+index as u32,"paint":paint}));
                self.selection_paints[index] = (paint, active);
                self.windows[self.current_window].text_cursor = cursor;
                self.windows[self.current_window].text_cell = cell;
            }
        }
        if !layers.is_empty() {
            self.submit(h, layers, vec![])?;
        }
        Ok(false)
    }
}
