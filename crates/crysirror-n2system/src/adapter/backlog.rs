// SPDX-License-Identifier: MPL-2.0
//! Native BacklogView/Backlog policy, NitroSharp 36701328 (MIT).
//! Layout and glyph rasterization remain shared services. No voice controls or
//! dialogue execution are reconstructed from history text.
use super::*;
use crate::text::{self, Page, Part};
use crysirror_text::line_viewport::LineViewport;

pub(super) struct Backlog {
    style: text::Style,
    width: u32,
    rows: u32,
    pitch: u32,
    revision: Option<u64>,
    pending: Option<Task<(u64, text::Layout)>>,
    layout: Option<text::Layout>,
    first: Option<u32>,
}
impl N2SystemVm {
    pub(super) fn append_backlog(&mut self, mut text: String) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        if self.history_newline {
            text.insert(0, '\n');
        }
        ensure!(
            self.history.len() < 4096 && text.len() <= 16384,
            "N2System backlog entry budget"
        );
        self.machine
            .variables
            .insert("SYSTEM_last_text".into(), Value::String(text.clone()));
        self.history.push((text, String::new(), String::new()));
        self.history_newline = false;
        self.history_revision = self
            .history_revision
            .checked_add(1)
            .context("backlog revision")?;
        Ok(())
    }
    pub(super) fn create_backlog(
        &mut self,
        host: &mut dyn HostServices,
        current: u32,
        args: &Arguments<'_>,
    ) -> Result<()> {
        let number = |name: &str, fallback: f32| -> Result<f32> {
            let value = self
                .machine
                .variables
                .get(name)
                .map(Value::number)
                .transpose()?
                .unwrap_or(fallback);
            ensure!(
                value.is_finite() && (0. ..=16384.).contains(&value),
                "N2System backlog {name}"
            );
            Ok(value)
        };
        let x = number("SYSTEM_backlog_position_x", 0.)?;
        let y = number("SYSTEM_backlog_position_y", 0.)?;
        let pitch = number("SYSTEM_backlog_row_interval", self.style.size as f32)? as u32;
        let rows = number("SYSTEM_backlog_row_max", 12.)? as u32;
        let cell = number("SYSTEM_backlog_character_width", self.style.size as f32)? as u32;
        let width = cell.checked_mul(32).context("N2System backlog width")?;
        let height = rows.checked_mul(pitch).context("N2System backlog height")?;
        ensure!(
            width > 0 && width <= 16384 && height > 0 && height <= 16384,
            "N2System backlog extent"
        );
        let id = self.create(
            host,
            current,
            &args.string(0)?,
            Kind::Visual,
            [width as f32, height as f32],
        )?;
        let mut command = self.entity_group(id)?;
        command["layer"]["rect"] = json!([x, y, width, height]);
        command["layer"]["order"] = json!([args.priority(1)?, 0]);
        self.submit(host, vec![command], vec![])?;
        self.backlogs.insert(
            id,
            Backlog {
                style: self.style.clone(),
                width,
                rows,
                pitch,
                revision: None,
                pending: None,
                layout: None,
                first: None,
            },
        );
        Ok(())
    }

    pub(super) fn update_backlogs(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let ids = self.backlogs.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let backlog = self.backlogs.get_mut(&id).unwrap();
            if let Some(task) = &mut backlog.pending {
                if let Some((revision, layout)) = task.try_take()? {
                    backlog.pending = None;
                    if revision == self.history_revision {
                        backlog.revision = Some(revision);
                        backlog.layout = Some(layout);
                        backlog.first = None;
                    }
                }
            }
            if backlog.revision != Some(self.history_revision) && backlog.pending.is_none() {
                let mut plain = String::new();
                for (text, _, _) in &self.history {
                    plain.push_str(text);
                    ensure!(
                        plain.len() <= 1024 * 1024,
                        "N2System backlog text byte budget"
                    );
                }
                let page = Page {
                    vertical_glyphs: backlog.style.vertical_glyphs,
                    parts: vec![Part::Text(plain, backlog.style.clone())],
                    bounds: [backlog.width, 32767],
                    spacing: [0, backlog.pitch as i32],
                    margin: [0, 0],
                    shown: 1,
                    disable_animation: true,
                    line_read: true,
                    thread: None,
                    order: 0,
                    revision: self.history_revision,
                };
                let measurement = self.measurement.as_ref().unwrap().clone();
                match self.pool.as_ref().unwrap().spawn(move |cancel| {
                    cancel.check()?;
                    let layout = text::layout(&page, measurement.as_ref())?;
                    cancel.check()?;
                    Ok((page.revision, layout))
                }) {
                    Ok(task) => backlog.pending = Some(task),
                    Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => {}
                    Err(e) => return Err(e),
                }
            }
            let position = self
                .scrollbars
                .values()
                .find(|b| b.target == Some(id))
                .map_or(1., |bar| bar.track.value);
            let backlog = self.backlogs.get_mut(&id).unwrap();
            let Some(layout) = &backlog.layout else {
                continue;
            };
            // Native positive scroll values move toward the oldest line.
            let total = layout.lines;
            let viewport = LineViewport::select(total, backlog.rows, backlog.pitch, 1. - position)?;
            if backlog.first == Some(viewport.first) {
                continue;
            }
            let mut glyphs = Vec::new();
            for glyph in &layout.glyphs {
                let baseline = glyph["baseline"][1]
                    .as_f64()
                    .context("backlog glyph baseline")? as f32;
                let size = glyph["size"].as_f64().context("backlog glyph size")? as f32;
                let y = baseline - viewport.offset as f32;
                let height = backlog.rows * backlog.pitch;
                if y + size < 0. || y - size > height as f32 {
                    continue;
                }
                let mut glyph = glyph.clone();
                glyph["baseline"][1] = json!(y);
                glyph["crop"] = json!([0., 0., backlog.width, height]);
                glyphs.push(glyph);
            }
            let paint = json!({"kind":"glyph_run","font":backlog.style.font,
                "size":backlog.style.size,"glyphs":glyphs});
            backlog.first = Some(viewport.first);
            self.submit(
                host,
                vec![json!({"op":"set_paint","id":id,"paint":paint})],
                vec![],
            )?;
        }
        Ok(())
    }
}
