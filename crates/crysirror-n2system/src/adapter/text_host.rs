// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::text::{self, Page, Part};

impl N2SystemVm {
    pub(super) fn native_text(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        state: &mut Execution,
        args: &Arguments<'_>,
        call: &Call,
    ) -> Result<bool> {
        match call.function {
            Builtin::SetTextOrientation => self.style.vertical_glyphs = args.truth(0)?,
            Builtin::SetFont => {
                let mut style = self.style.clone();
                let requested = args.string(0)?;
                if !requested.is_empty() {
                    style.vertical_glyphs = requested.starts_with('@');
                    style.font = self
                        .fonts
                        .as_ref()
                        .unwrap()
                        .resolve_alias(requested.trim_start_matches('@'))?
                        // The native Windows font mapper substitutes missing
                        // MS faces. Portable hosts retain their selected CJK
                        // default rather than treating that substitution as a
                        // malformed script or distributing Microsoft fonts.
                        .or_else(|| {
                            requested
                                .trim_start_matches('@')
                                .replace(' ', "")
                                .starts_with("ＭＳ")
                                .then(|| self.style.font.clone())
                        })
                        .with_context(|| {
                            format!("N2System font family not installed: {requested}")
                        })?;
                }
                let size = args.integer(1)?;
                ensure!((1..=512).contains(&size), "N2System font size range");
                style.size = size as u16;
                style.color = args.color(2)?;
                style.outline = if args.len() > 5
                    && !args.value(5).is_constant("LightDown")
                    && !args.value(5).is_constant("None")
                {
                    Some(args.color(3)?)
                } else {
                    None
                };
                style.bold = matches!(args.value(4),Value::Number(n) if *n>=700.);
                self.style = style;
            }
            Builtin::CreateWindow => {
                let w = args.integer(4)?;
                let h = args.integer(5)?;
                ensure!(
                    (1..=16384).contains(&w) && (1..=16384).contains(&h),
                    "N2System dialogue box extent"
                );
                let id = self.visual(
                    host,
                    current,
                    args,
                    json!({"kind":"group"}),
                    [w as u32, h as u32],
                    None,
                )?;
                self.entities.nodes.get_mut(&id).unwrap().container = Some(args.truth(6)?);
            }
            Builtin::CreateText => {
                let dimension = |at: usize, axis: usize| -> Result<u32> {
                    if matches!(args.value(at), Value::Constant(_)) {
                        return Ok(self.config.canvas[axis]);
                    }
                    let n = args.integer(at)?;
                    ensure!((1..=16384).contains(&n), "N2System text dimension");
                    Ok(n as u32)
                };
                let bounds = [dimension(4, 0)?, dimension(5, 1)?];
                let parts = text::parse(&args.string(6)?, self.style.clone())?;
                ensure!(
                    parts
                        .iter()
                        .all(|p| matches!(p, Part::Text(..) | Part::Ruby(..))),
                    "N2System CreateText requires a text segment without dialogue controls"
                );
                let id = self.visual(host, current, args, json!({"kind":"group"}), bounds, None)?;
                let shown = parts.len();
                self.pages.insert(
                    id,
                    Page {
                        vertical_glyphs: self.style.vertical_glyphs,
                        parts,
                        bounds,
                        spacing: [0, 0],
                        margin: [0, 15],
                        shown,
                        disable_animation: true,
                        line_read: true,
                        thread: None,
                        order: args.priority(1)?,
                        revision: 1,
                    },
                );
                self.text_dirty.insert(id);
                state.pending_text = Some((id, 1));
                return Ok(false);
            }
            Builtin::LoadText => {
                let routine = args.string(0)?;
                let window = args.string(1)?;
                let name = args.string(2)?;
                let name = name.trim_start_matches('@');
                let module = state
                    .thread
                    .frames
                    .iter()
                    .rev()
                    .find_map(|frame| {
                        self.machine
                            .modules
                            .get(&frame.module)
                            .filter(|m| m.routines.contains_key(&routine))
                            .map(|_| frame.module.clone())
                    })
                    .context("N2System dialogue owning routine not on the call stack")?;
                let block = self.machine.modules[&module].routines[&routine]
                    .blocks
                    .get(name)
                    .context("N2System dialogue block not found")?
                    .clone();
                let parent = self
                    .query(current, &window)?
                    .first()
                    .copied()
                    .context("N2System dialogue box not found")?;
                let extent = self.entities.nodes[&parent].size;
                let w = args.integer(3)?;
                let h = args.integer(4)?;
                let bounds = [
                    if w > 0 { w as u32 } else { extent[0] as u32 },
                    if h > 0 { h as u32 } else { extent[1] as u32 },
                ];
                ensure!(
                    bounds.iter().all(|n| (1..=16384).contains(n)),
                    "N2System dialogue page bounds"
                );
                if let Some(old) = self.entities.child(parent, name) {
                    self.destroy(host, old)?;
                }
                let id = self.entities.add(
                    name.into(),
                    Some(parent),
                    Kind::Visual,
                    bounds.map(|n| n as f32),
                )?;
                self.entities.alias(id, name.into())?;
                let order = self.layer(host, parent)?["order"][0]
                    .as_i64()
                    .context("N2System dialogue box draw order")?
                    as i32;
                let render_parent = self.entities.render_parent(id)?;
                let x =
                    self.native_position(host, id, &Value::Number(0.), 0., bounds[0] as f32, 0)?;
                let y =
                    self.native_position(host, id, &Value::Number(0.), 0., bounds[1] as f32, 1)?;
                self.submit(host,vec![json!({"op":"put","layer":{"id":id,"parent":render_parent,"rect":[x,y,bounds[0],bounds[1]],"order":[order,1],"paint":{"kind":"group"}}})],vec![])?;
                ensure!(
                    self.threads.len() < 255,
                    "N2System dialogue thread capacity"
                );
                let thread_id = self.entities.root(format!("dialogue:{name}"))?;
                self.submit(host, vec![self.entity_group(thread_id)?], vec![])?;
                let mut execution =
                    Execution::new(self.machine.start(&module, Some(&block.entry), routine)?);
                execution.page = Some(id);
                execution.thread.scopes = state.thread.inherited_scopes()?;
                execution.interrupt = state.interrupt;
                execution.enabled = false;
                execution.retire_when_done = true;
                self.threads.insert(thread_id, execution);
                self.pages.insert(
                    id,
                    Page {
                        vertical_glyphs: self.style.vertical_glyphs,
                        parts: Vec::new(),
                        bounds,
                        spacing: [args.integer(5)?, args.integer(6)?],
                        margin: [0, 10],
                        shown: 0,
                        disable_animation: false,
                        line_read: false,
                        thread: Some(thread_id),
                        order,
                        revision: 1,
                    },
                );
            }
            Builtin::WaitText => {
                let ids = self.query(current, &args.string(0)?)?;
                let timeout = args.time(1)?;
                state.wait = Wait::Text {
                    ids,
                    until: if timeout > 0 {
                        Some(host.now_ms().saturating_add(timeout))
                    } else {
                        None
                    },
                };
            }
            _ => bail!("N2System invalid text dispatch"),
        }
        self.machine.complete(&mut state.thread, Value::Null)?;
        Ok(true)
    }
    pub(super) fn clear_text(&mut self, host: &mut dyn HostServices, id: u32) -> Result<()> {
        self.text_fades.remove(&id);
        let page = self
            .pages
            .get_mut(&id)
            .context("N2System missing dialogue page")?;
        page.parts.clear();
        page.shown = 0;
        page.line_read = false;
        page.revision = page
            .revision
            .checked_add(1)
            .context("N2System text revision exhausted")?;
        self.text_dirty.remove(&id);
        self.text_jobs.remove(&id);
        self.submit(
            host,
            vec![json!({"op":"set_paint","id":id,"paint":{"kind":"group"}})],
            vec![],
        )?;
        Ok(())
    }
    pub(super) fn append_text(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        markup: &str,
    ) -> Result<()> {
        let mut style = self.style.clone();
        style.vertical_glyphs = self
            .pages
            .get(&id)
            .context("N2System missing dialogue page")?
            .vertical_glyphs;
        let parts = text::parse(markup, style)?;
        let page = self
            .pages
            .get_mut(&id)
            .context("N2System missing dialogue page")?;
        ensure!(
            page.parts.len() + parts.len() <= 65536,
            "N2System dialogue part capacity"
        );
        page.parts.extend(parts);
        page.line_read = false;
        self.advance_text(host, id)?;
        Ok(())
    }
    fn advance_text(&mut self, host: &mut dyn HostServices, id: u32) -> Result<()> {
        let mut voices = Vec::new();
        let mut logged = String::new();
        let page = self
            .pages
            .get_mut(&id)
            .context("N2System missing dialogue page")?;
        while let Some(part) = page.parts.get(page.shown) {
            page.shown += 1;
            match part {
                Part::Halt => break,
                Part::Voice {
                    character,
                    source,
                    stop,
                } => voices.push((character.clone(), source.clone(), *stop)),
                Part::Text(s, _) | Part::Ruby(s, _, _) => logged.push_str(s),
                _ => {}
            }
        }
        page.revision = page
            .revision
            .checked_add(1)
            .context("N2System text revision exhausted")?;
        self.text_dirty.insert(id);
        if !self.no_log.contains(&id) {
            self.append_backlog(logged)?;
        }
        for (character, source, stop) in voices {
            if let Some(id) = self.voices.get(&character) {
                host.submit_audio(
                    self.client,
                    AudioRequest::Stop {
                        channel: *id,
                        fade_ms: 0.,
                    },
                )?;
            }
            if stop {
                continue;
            }
            let source = resource::resolve(self.catalog.as_ref().unwrap(), &source)?;
            let channel = if let Some(id) = self.voices.get(&character) {
                *id
            } else {
                let id = self.entities.add(
                    format!("\u{1f}voice:{character}"),
                    Some(self.main),
                    Kind::Sound {
                        source: source.clone(),
                        kind: "Voice".into(),
                        gain: 1.,
                        looping: false,
                        playing: true,
                    },
                    [0.; 2],
                )?;
                self.submit(host, vec![self.entity_group(id)?], vec![])?;
                self.voices.insert(character, id);
                id
            };
            let ticket = host.begin_audio(
                self.client,
                AudioRequest::PlayAsset {
                    channel,
                    source: AudioAsset {
                        path: VfsPath::new(Mount::Asset, &source)?,
                        frame_limit: None,
                    },
                    intro: None,
                    parameters: AudioParameters {
                        mix_group: Some("n2system-Voice".into()),
                        ..Default::default()
                    },
                },
            )?;
            self.voice_tickets.push(ticket);
        }
        Ok(())
    }
    pub(super) fn publish_text(
        &mut self,
        host: &mut dyn HostServices,
        id: u32,
        layout: text::Layout,
    ) -> Result<()> {
        let page = self.pages.get(&id).context("N2System text page retired")?;
        let layer = self.layer(host, id)?;
        let initial = if layer["paint"]["kind"] == "glyph_run" {
            layer["paint"]["glyphs"].as_array().map_or(0, Vec::len)
        } else {
            0
        };
        let extent = [
            layout.size[0].min(page.bounds[0]),
            layout.size[1].min(page.bounds[1]),
        ];
        let instant = page.disable_animation || page.thread.is_none();
        let paint = json!({"kind":"glyph_run","font":self.config.font,"size":self.style.size,"glyphs":layout.glyphs});
        let paint: crysirror_scene::layer::Paint = serde_json::from_value(paint)?;
        let rect = json!([layer["rect"][0], layer["rect"][1], extent[0], extent[1]]);
        self.submit(
            host,
            vec![json!({"op":"set_rect","id":id,"rect":rect})],
            vec![],
        )?;
        self.entities.nodes.get_mut(&id).unwrap().size = extent.map(|n| n as f32);
        self.begin_text_fade(host, id, paint, layout.reveal_positions, initial, instant)?;
        Ok(())
    }
    pub(super) fn poll_text_layout(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let ids: Vec<_> = self.text_jobs.keys().copied().collect();
        for id in ids {
            let Some((revision, layout)) = self.text_jobs.get_mut(&id).unwrap().try_take()? else {
                continue;
            };
            self.text_jobs.remove(&id);
            if self
                .pages
                .get(&id)
                .is_some_and(|page| page.revision == revision)
            {
                self.publish_text(host, id, layout)?;
                self.text_published.insert(id, revision);
                self.text_dirty.remove(&id);
            }
        }
        for id in self.text_dirty.clone() {
            if self.text_jobs.contains_key(&id) {
                continue;
            }
            let Some(page) = self.pages.get(&id).cloned() else {
                self.text_dirty.remove(&id);
                continue;
            };
            let measurement = self.measurement.as_ref().unwrap().clone();
            match self.pool.as_ref().unwrap().spawn(move |cancel| {
                cancel.check()?;
                let layout = text::layout(&page, measurement.as_ref())?;
                cancel.check()?;
                Ok((page.revision, layout))
            }) {
                Ok(job) => {
                    self.text_jobs.insert(id, job);
                }
                Err(e)
                    if e.downcast_ref::<crysirror_tasks::CapacityExceeded>()
                        .is_some() =>
                {
                    break;
                }
                Err(e) => return Err(e),
            }
        }
        let mut pending = Vec::new();
        for ticket in std::mem::take(&mut self.voice_tickets) {
            let status = host.audio_preparation(self.client, ticket)?;
            match status.phase {
                AudioPreparePhase::Pending => pending.push(ticket),
                AudioPreparePhase::Ready => host.release_audio(self.client, ticket)?,
                AudioPreparePhase::Failed => {
                    host.release_audio(self.client, ticket)?;
                    bail!(
                        "N2System dialogue voice: {}",
                        status.error.unwrap_or_default()
                    );
                }
            }
        }
        self.voice_tickets = pending;
        Ok(())
    }
    pub(super) fn update_text_input(
        &mut self,
        host: &mut dyn HostServices,
        input: &InputFrame,
    ) -> Result<()> {
        let advance =
            input.buttons_pressed & (buttons::ENTER | buttons::SPACE | buttons::MOUSE_LEFT) != 0;
        if self.text_switch("SYSTEM_text_erase", false)? {
            if advance {
                self.machine
                    .variables
                    .insert("SYSTEM_text_erase".into(), Value::Boolean(false));
            }
            return Ok(());
        }
        let skip =
            input.buttons_down & buttons::CTRL != 0 || self.text_switch("SYSTEM_skip", false)?;
        let auto = self.text_switch("SYSTEM_text_auto", false)?;
        let wait_key = self.text_switch("SYSTEM_text_waitkey", true)?;
        let auto_wait = self.text_number("SYSTEM_auto_wait_per_character", true, 100.)?;
        let mut pages: Vec<_> = self
            .pages
            .iter()
            .filter(|(_, p)| {
                p.thread.is_some()
                    && !p.line_read
                    && p.thread
                        .and_then(|id| self.threads.get(&id))
                        .is_none_or(|s| s.enabled)
            })
            .map(|(id, p)| (p.order, *id))
            .collect();
        pages.sort_unstable();
        for (_, id) in pages.into_iter().rev() {
            if self.text_dirty.contains(&id) || self.text_jobs.contains_key(&id) {
                continue;
            }
            if self
                .text_fades
                .get(&id)
                .is_some_and(|f| !f.clock.complete())
            {
                if skip || (advance && wait_key) {
                    self.finish_text_fade(host, id)?;
                    if !skip {
                        break;
                    }
                } else {
                    continue;
                }
            }
            let mut auto_ready = false;
            if let Some(fade) = self.text_fades.get_mut(&id) {
                let since = *fade.auto_since.get_or_insert(host.now_ms());
                auto_ready = auto
                    && host.now_ms().saturating_sub(since) as f64
                        >= f64::from(auto_wait) * fade.characters.max(1) as f64;
            }
            if !skip && !auto_ready && !(advance && wait_key) {
                continue;
            }
            if self.pages[&id].shown < self.pages[&id].parts.len() {
                self.advance_text(host, id)?;
            } else {
                self.pages.get_mut(&id).unwrap().line_read = true;
            }
            break;
        }
        Ok(())
    }
}
