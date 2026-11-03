// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Native Gameexe cursor policy on client-scoped engine UI definitions.
use super::*;
use crysirror_scene::{
    ServiceBatch,
    ui::{
        UiCommand,
        cursor::{CursorFrameSpec, CursorSelect, CursorSpec, StandardCursor},
    },
};
#[derive(Default)]
pub(super) struct Cursors {
    defined: BTreeSet<i64>,
    selected: Option<i64>,
    visible: Option<bool>,
}
pub(super) fn default_no(ctx: &CommandContext) -> Result<i64> {
    let Some(cfg) = &ctx.tables.gameexe else {
        return Ok(-1);
    };
    let count = cfg
        .get_unquoted("MOUSE_CURSOR.CNT")
        .map(str::parse::<i64>)
        .transpose()?
        .unwrap_or(16)
        .clamp(0, 256);
    let no = cfg
        .get_unquoted("MOUSE_CURSOR.DEFAULT")
        .map(str::parse::<i64>)
        .transpose()?
        .unwrap_or(-1);
    Ok(if no >= 0 && no < count { no } else { -1 })
}
impl Cursors {
    fn sync(
        &mut self,
        ctx: &mut CommandContext,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        // Policy adopted from the complete original standalone presenter's
        // apply_syscom_window_config cursor section. Reusable deadline state is
        // engine-owned; the host publishes actual movement/entry tokens.
        let activity = host.pointer_activity(client).map_err(err)?;
        let now = crate::platform_time::Instant::now().0;
        let cfg = &ctx.globals.syscom;
        let (default_on, default_time) = crate::runtime::forms::syscom::cursor_defaults(ctx);
        let enabled = match ctx.globals.script.mouse_cursor_hide_onoff {
            0 => false,
            1 => true,
            _ => {
                cfg.config_int
                    .get(&crate::runtime::forms::codes::syscom_op::GET_MOUSE_CURSOR_HIDE_ONOFF)
                    .copied()
                    .unwrap_or(default_on)
                    != 0
            }
        };
        let delay = if ctx.globals.script.mouse_cursor_hide_time >= 0 {
            ctx.globals.script.mouse_cursor_hide_time
        } else {
            cfg.config_int
                .get(&crate::runtime::forms::codes::syscom_op::GET_MOUSE_CURSOR_HIDE_TIME)
                .copied()
                .unwrap_or(default_time)
        };
        ensure!(
            !enabled || delay <= 0 || activity.is_some(),
            "native cursor idle policy requires backend pointer activity observations"
        );
        let idle = ctx
            .ui
            .cursor_idle
            .get_or_insert_with(|| crysirror_scene::activity::ActivityTimer::new(now, activity));
        idle.observe(now, activity);
        ctx.globals.script.cursor_runtime_visible = !ctx.globals.script.cursor_disp_off
            && !(enabled && delay > 0 && idle.inactive_for(now, delay as u64));
        let no = ctx.globals.script.cursor_no;
        let visible =
            ctx.globals.script.cursor_runtime_visible && !ctx.globals.script.cursor_disp_off;
        let mut batch = ServiceBatch::default();
        if self.selected != Some(no) {
            let selection = if no < 0 {
                CursorSelect::Standard {
                    shape: StandardCursor::Default,
                }
            } else {
                let cfg = ctx
                    .tables
                    .gameexe
                    .as_ref()
                    .context("native custom cursor requires Gameexe")?;
                let count = cfg
                    .get_unquoted("MOUSE_CURSOR.CNT")
                    .map(str::parse::<i64>)
                    .transpose()?
                    .unwrap_or(16)
                    .clamp(0, 256);
                ensure!(no < count, "native cursor index outside configured table");
                if !self.defined.contains(&no) {
                    let file = cfg
                        .get_indexed_field_unquoted("MOUSE_CURSOR", no as usize, "FILE")
                        .filter(|s| !s.is_empty())
                        .context("native cursor file missing")?;
                    let speed = cfg
                        .get_indexed_field_unquoted("MOUSE_CURSOR", no as usize, "SPEED")
                        .map(str::parse::<i64>)
                        .transpose()?
                        .unwrap_or(100);
                    let frames = ctx.images.album_frames(file)?;
                    ensure!(frames.len() <= 256, "native cursor frame limit");
                    let frames=frames.into_iter().map(|id|->Result<CursorFrameSpec>{
                        let image=&ctx.images.images[id];
                        ensure!(image.width==32 && image.height==32,"native Siglus cursor must be 32x32");
                        ensure!((0..32).contains(&image.center_x) && (0..32).contains(&image.center_y),"native cursor hotspot outside image requires a padded cursor profile");
                        Ok(CursorFrameSpec{width:32,height:32,hotspot:[image.center_x as u32,image.center_y as u32],rgba:image.rgba.clone()})
                    }).collect::<Result<Vec<_>>>()?;
                    let steps = if speed <= 0 {
                        vec![[0, 0]]
                    } else {
                        let ms = u32::try_from(speed).context("native cursor step duration")?;
                        (0..frames.len()).map(|i| [i as u32, ms]).collect()
                    };
                    let cursor = CursorSpec { frames, steps };
                    cursor.validate()?;
                    batch.ui.push(UiCommand::DefineCursor {
                        id: no as u32,
                        cursor,
                    });
                }
                CursorSelect::Custom { id: no as u32 }
            };
            batch.ui.push(UiCommand::SetCursor { cursor: selection });
        }
        if self.visible != Some(visible) {
            batch.ui.push(UiCommand::SetCursorVisible { visible });
        }
        if !batch.ui.is_empty() {
            host.submit_engine(client, 1, &serde_json::to_vec(&batch)?)?;
        }
        if no >= 0 {
            self.defined.insert(no);
        }
        self.selected = Some(no);
        self.visible = Some(visible);
        Ok(())
    }
}
impl SiglusVm {
    pub(super) fn sync_cursor(&mut self, host: &mut dyn HostServices) -> Result<()> {
        for _ in 0..4 {
            self.state.io.missing.clear();
            let result = io_fs::scoped(&mut self.state.io, || {
                self.cursors.sync(
                    &mut self.state.vm.as_mut().unwrap().ctx,
                    ClientId(self.state.client),
                    host,
                )
            });
            if self.state.io.missing.is_empty() {
                return result;
            }
            for key in std::mem::take(&mut self.state.io.missing) {
                self.fetch(host, &key)?;
            }
        }
        anyhow::bail!("native cursor resource admission retry limit")
    }
}
