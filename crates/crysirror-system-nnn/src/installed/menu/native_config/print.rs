// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native Print ordering and deferred clock commit, over shared image workers.
//! PrintBackScriptOrBG -> controls -> PrintMessageWindowDirect -> Back/UpDown.
use super::*;
use crate::native_config_frame::{Draw, FrameWork};
use crysirror_image::{
    byte_raster::{ByteRaster, ByteRasterMath},
    effects::ImageEffect,
    row_blit::RowBlit,
};
/// Engine order of the published Config frame; text glyph runs follow it.
pub(in crate::installed) const ORDER: i32 = 1_999_999_999;
impl ConfigMenu {
    fn button_draw(&self, candidate: &mut Session, group: usize, out: &mut Vec<Draw>) -> Result<()> {
        if !candidate.controls[group].visible(candidate.page) {
            return Ok(());
        }
        let (_, count, maximum) =
            candidate.entry.sample(candidate.layout.entry_wait, candidate.layout.appearance_frames);
        let total = candidate.controls[group].len();
        for member in 0..total {
            let picture = self.picture(group, member)?;
            let (spec, state) = candidate.controls[group].active_mut(member)?;
            if spec.button.exists {
                if let Some(view) = picture.paint() {
                    if let Some(paint) = layout::appearance::plan(
                        &spec.button,
                        state,
                        view.raw.size(),
                        count,
                        maximum,
                        candidate.layout.appearance_type,
                        [0; 2],
                        member as i32,
                        total as i32,
                    )? {
                        out.push(Draw::AppearanceButton {
                            paint,
                            spec: spec.button.clone(),
                            state: state.clone(),
                            picture: view.clone(),
                        });
                    }
                }
            }
            // CPicture exists even when LoadDWQ failed. Print still advances
            // animation in that case; a NULL picture pointer alone skips it.
            candidate.controls[group].printed(member, picture.object_exists)?;
        }
        Ok(())
    }
    /// Start one Print. `base` is the background script's current frame when
    /// it has one; otherwise the Layers below this mode are captured.
    pub(in crate::installed) fn begin_print(
        &mut self,
        base: Option<Arc<dyn ImageResource>>,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        ensure!(
            self.ready && !self.print_in_flight(),
            "NNN Config Print while a resource candidate is in flight"
        );
        let mut candidate = self.session.clone();
        let mut draws = vec![];
        let mut order = (0..candidate.controls.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| candidate.layout.groups[*index].paint_order());
        for group in order
            .iter()
            .copied()
            .filter(|index| candidate.layout.groups[*index].paint_order() < 7)
            .collect::<Vec<_>>()
        {
            self.button_draw(&mut candidate, group, &mut draws)?;
            if let layout::Action::Character { slot, .. } = candidate.controls[group].action {
                if !candidate.controls[group].visible(candidate.page) {
                    continue;
                }
                if let Some(slider) = &candidate.layout.characters[slot].slider {
                    if let Some(picture) = self.system_picture(slider.file.as_deref()).and_then(Picture::paint) {
                        let gaze = self
                            .policy
                            .get(&self.context.file, 500 + slot as i32)?
                            .wrapping_sub(slider.minimum);
                        let width = i64::from(slider.size[0]) * i64::from(gaze)
                            / i64::from((slider.maximum - slider.minimum).max(1));
                        let (_, count, maximum) = candidate
                            .entry
                            .sample(candidate.layout.entry_wait, candidate.layout.appearance_frames);
                        draws.push(Draw::Slider {
                            picture: picture.clone(),
                            origin: slider.origin,
                            width: i32::try_from(width)?,
                            height: slider.size[1],
                            percent: (count * 100 / maximum.max(1)).clamp(0, 100),
                            bad: candidate.controls[group].value() == 0,
                            transparent: true,
                        });
                    }
                }
            }
        }
        // Source prints each bar immediately before its own volume switch.
        let shared = candidate.layout.clone();
        for (index, slider) in shared.sliders.sliders.iter().enumerate() {
            if !slider.exists || slider.page as i64 != candidate.page as i64 + 1 {
                continue;
            }
            if let Some(picture) = self.system_picture(Some(&slider.bar)).and_then(Picture::paint) {
                let switch = crate::native_config_volume::SWITCHES[index];
                let (_, count, maximum) =
                    candidate.entry.sample(candidate.layout.entry_wait, candidate.layout.appearance_frames);
                draws.push(Draw::Slider {
                    picture: picture.clone(),
                    origin: slider.origin,
                    width: candidate.sliders[index].fill_width(&candidate.layout.sliders, index)?,
                    height: slider.size[1],
                    percent: (count * 100 / maximum.max(1)).clamp(0, 100),
                    bad: switch != 0 && self.policy.get(&self.context.file, switch)? == 0,
                    transparent: slider.transparent,
                });
            }
            for group in order
                .iter()
                .copied()
                .filter(|group| {
                    matches!(candidate.controls[*group].action,
                        layout::Action::Toggle(id) if id == crate::native_config_volume::SWITCHES[index])
                })
                .collect::<Vec<_>>()
            {
                self.button_draw(&mut candidate, group, &mut draws)?;
            }
        }
        // Remaining ordinary toggles/parameters (paint order 7, not volume).
        for group in order
            .iter()
            .copied()
            .filter(|index| {
                candidate.layout.groups[*index].paint_order() == 7
                    && !matches!(candidate.controls[*index].action,
                        layout::Action::Toggle(id) if crate::native_config_volume::SWITCHES.contains(&id) && id != 0)
            })
            .collect::<Vec<_>>()
        {
            self.button_draw(&mut candidate, group, &mut draws)?;
        }
        // PrintMessageWindowDirect precedes the Back/UpDown/Tab print.
        let mut late = vec![];
        for group in order
            .into_iter()
            .filter(|index| candidate.layout.groups[*index].paint_order() >= 8)
            .collect::<Vec<_>>()
        {
            self.button_draw(&mut candidate, group, &mut late)?;
        }
        if self.window_preview().is_some() && !self.context.windows.is_empty() {
            self.late_commands = late;
        } else {
            draws.extend(late);
            self.late_commands.clear();
        }
        self.draw_commands = draws;
        // Commit this Print's button clocks now: Calcu keeps running while the
        // shared image jobs finish, and must observe the advanced animation.
        self.session = candidate;
        match base {
            Some(image) => self.begin_background(image, client, host)?,
            None if self.back_script => {
                self.base_capture = Some(host.begin_layer_prefix_image_over(
                    client,
                    SIZE,
                    [ORDER, 0],
                    [0, 0, 0, 255],
                )?);
            }
            None => {
                let black = host.image_metadata(client)?.begin_process(
                    ImageInput::Solid { size: SIZE, color: [0, 0, 0, 255] },
                    b"[]".as_slice().into(),
                )?;
                self.base_capture = Some(black);
            }
        }
        Ok(())
    }
    fn begin_background(
        &mut self,
        base: Arc<dyn ImageResource>,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        if self.back_script {
            // PrintBackScriptOrBG: the script frame is the base as-is.
            self.frame = Some(FrameWork::begin(
                self.context.reload.resources.client(),
                base,
                std::mem::take(&mut self.draw_commands),
                host,
            )?);
            return Ok(());
        }
        let mut graph = ImageInput::Resource(base);
        {
            if let Some(view) = self.background().and_then(Picture::paint) {
                if let Some((source, at)) = crysirror_image::coverage_bounds::intersect_source_window(
                    [0, 0, SIZE[0] as i32, SIZE[1] as i32],
                    [0, 0],
                    view.source_clip,
                )? {
                    graph = ImageInput::Process {
                        input: Box::new(ImageInput::Raster {
                            destination: Box::new(graph),
                            source: Box::new(ImageInput::Resource(view.opaque.clone())),
                        }),
                        effects: serde_json::to_vec(&[ImageEffect::ByteRaster {
                            spec: ByteRaster::RowBlit {
                                spec: RowBlit {
                                    source,
                                    destination: at,
                                    row_offsets: vec![0; source[3] as usize],
                                    math: ByteRasterMath::CopyRgb,
                                    row_math: vec![],
                                },
                            },
                        }])?
                        .into(),
                    };
                }
            }
        }
        self.frame_base = Some(host.image_metadata(client)?.begin_process(graph, b"[]".as_slice().into())?);
        Ok(())
    }
    /// Windows translated so the body window's top-left is the preview origin,
    /// at the current window transparency (SetSystemParam 106).
    fn preview_windows(&self) -> Result<Vec<crate::native_text::MessageWindow>> {
        let preview = self.window_preview().context("NNN Config window preview absent")?;
        let body = self.context.windows.first().context("NNN Config message window absent")?;
        let delta = [preview.at[0] - body.rect[0], preview.at[1] - body.rect[1]];
        let percent = 100 - self.policy.get(&self.context.file, 106)?.clamp(0, 100);
        Ok(self
            .context
            .windows
            .iter()
            .cloned()
            .map(|mut window| {
                window.rect[0] += delta[0];
                window.rect[1] += delta[1];
                window.percent = percent as u16;
                window
            })
            .collect())
    }
    pub(in crate::installed) fn poll_print(
        &mut self,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<bool> {
        if let Some(job) = self.base_capture.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => anyhow::bail!("NNN Config background capture: {error}"),
                ImageProcessState::Ready(image) => {
                    self.base_capture = None;
                    self.begin_background(image, client, host)?;
                }
            }
        }
        if let Some(job) = self.frame_base.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => anyhow::bail!("NNN Config background Print: {error}"),
                ImageProcessState::Ready(image) => {
                    self.frame_base = None;
                    self.frame = Some(FrameWork::begin(
                        self.context.reload.resources.client(),
                        image,
                        std::mem::take(&mut self.draw_commands),
                        host,
                    )?);
                }
            }
        }
        if let Some(work) = self.frame.as_mut() {
            let Some(image) = work.poll(host)? else {
                return Ok(false);
            };
            self.frame = None;
            if !self.late_commands.is_empty() {
                let windows = self.preview_windows()?;
                self.window_job = Some(host.image_metadata(client)?.begin_process(
                    crate::native_text::window_composite_input(image, &windows)?,
                    b"[]".as_slice().into(),
                )?);
            } else {
                self.completed = Some(image);
                return Ok(true);
            }
        }
        if let Some(job) = self.window_job.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => anyhow::bail!("NNN Config window preview: {error}"),
                ImageProcessState::Ready(image) => {
                    self.window_job = None;
                    self.frame = Some(FrameWork::begin(
                        self.context.reload.resources.client(),
                        image,
                        std::mem::take(&mut self.late_commands),
                        host,
                    )?);
                    return Ok(false);
                }
            }
        }
        Ok(false)
    }
    /// Preview text as shared glyph runs above the published frame.
    pub(in crate::installed) fn text_layers(
        &self,
        client: ClientId,
        host: &mut dyn HostServices,
        first_id: u32,
    ) -> Result<Vec<Value>> {
        let Some(preview) = self.window_preview() else {
            return Ok(vec![]);
        };
        let Some((body, _)) = self.context_faces() else {
            return Ok(vec![]);
        };
        let metrics = host.text_measurement(client)?;
        let mut out = vec![];
        for (index, line) in preview.messages.iter().chain(preview.name.iter()).enumerate() {
            let face = crate::native_text::TextFace {
                font: body.font.clone(),
                size: u16::try_from(line.size)?,
                spacing: body.spacing,
                color: line.rgb,
            };
            let text = crate::native_text::plain_cells(&line.bytes, None)?;
            let glyphs = crate::native_text::positioned(&text, &face, line.origin, metrics.as_ref())?;
            out.push(json!({"op":"put","layer":{"id":first_id+index as u32,"rect":[0,0,SIZE[0],SIZE[1]],"order":[ORDER,1+index as i32],"paint":{"kind":"glyph_run","font":face.font,"size":face.size,"color":face.color,"glyphs":glyphs}}}));
        }
        Ok(out)
    }
    pub(in crate::installed) fn widgets(&self, font: &str, enabled: bool) -> Result<Vec<Value>> {
        let mut out = vec![json!({"op":"put","widget":{"id":ROOT,"kind":"panel","rect":[0,0,SIZE[0],SIZE[1]],"modal":true,"key_event_scope":true,"color":[0,0,0,0],"z":1000,"font":font}})];
        for (group, control) in self.session.controls.iter().enumerate() {
            for member in 0..control.len() {
                let (spec, _) = control.active(member)?;
                let b = &spec.button;
                let mut widget = json!({"id":Self::button_id(group,member)?,"parent":ROOT,"kind":"button","rect":[b.origin[0]+b.check_delta[0],b.origin[1]+b.check_delta[1],b.check_size[0],b.check_size[1]],"interaction_only":true,"keyboard_focusable":false,"activate_on_press":true,"visible":control.visible(self.session.page)&&b.exists,"enabled":enabled&&b.enabled,"color":[0,0,0,0],"z":group as i32});
                let key_events = b
                    .accelerators
                    .iter()
                    .filter_map(|key| buttons::windows_virtual_key(u32::from(*key)))
                    .map(|buttons| json!({"buttons":buttons,"edge":"press","event":"nnn_config_activate"}))
                    .collect::<Vec<_>>();
                widget["key_events"] = json!(key_events);
                if b.print_mode != 0 {
                    // Off-page controls have no loaded picture; they are hidden.
                    let picture = if control.visible(self.session.page) {
                        self.picture(group, member)?.hit_picture()
                    } else {
                        None
                    };
                    let coverage = if let Some(picture) = picture {
                        let view = if picture.has_mask { &picture.raw } else { &picture.keyed };
                        view.alpha_region([
                            u32::try_from(b.check_origin[0])?,
                            u32::try_from(b.check_origin[1])?,
                            u32::try_from(b.check_size[0])?,
                            u32::try_from(b.check_size[1])?,
                        ])?
                    } else {
                        vec![
                            0;
                            usize::try_from(b.check_size[0])?
                                .checked_mul(usize::try_from(b.check_size[1])?)
                                .context("NNN Config hit extent overflow")?
                        ]
                        .into()
                    };
                    widget["hit_mask"] = json!({"size":b.check_size,"coverage":coverage,"threshold":1,"strict_bounds":false});
                }
                out.push(json!({"op":"put","widget":widget}));
            }
        }
        for (index, slider) in self.session.layout.sliders.sliders.iter().enumerate() {
            if !slider.exists {
                continue;
            }
            let switch = crate::native_config_volume::SWITCHES[index];
            out.push(json!({"op":"put","widget":{"id":ROOT+2000+index as u32,"parent":ROOT,"kind":"slider","rect":[slider.origin[0],slider.origin[1],slider.size[0],slider.size[1]],"interaction_only":true,"keyboard_focusable":false,"visible":slider.page as i64==self.session.page as i64+1,"enabled":enabled&&(switch==0||self.policy.get(&self.context.file,switch)?!=0),"value":(self.session.sliders[index].gaze as f64/slider.divide.max(1) as f64).clamp(0.,1.),"color":[0,0,0,0],"z":500}}));
        }
        for (slot, character) in self.session.layout.characters.iter().enumerate() {
            let Some(slider) = &character.slider else { continue };
            let group = (0..self.session.controls.len()).find(|g| {
                matches!(self.session.controls[*g].action, layout::Action::Character { slot: s, .. } if s == slot)
            });
            let visible = group.is_some_and(|g| self.session.controls[g].visible(self.session.page));
            out.push(json!({"op":"put","widget":{"id":ROOT+3000+slot as u32,"parent":ROOT,"kind":"slider","rect":[slider.origin[0],slider.origin[1],slider.size[0],slider.size[1]],"interaction_only":true,"keyboard_focusable":false,"visible":visible,"enabled":enabled,"value":0.0,"color":[0,0,0,0],"z":500}}));
        }
        Ok(out)
    }
    pub(in crate::installed) fn button_id(group: usize, member: usize) -> Result<u32> {
        ensure!(group < 1024 && member < 256, "NNN Config control widget extent");
        Ok(ROOT + 10000 + u32::try_from(group * 256 + member)?)
    }
}
