// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonSelectMessage special windows. The count-indexed coordinate table,
//! variable-selected atlas cell and cursor counter are native policy; images,
//! retained hit regions, glyphs and animation presentation remain shared Layers.
use super::*;
use crate::{config::Names, expression::Variables};
mod cursor;
pub(super) use cursor::Presenter;

#[derive(Clone)]
pub(crate) struct Atlas {
    pub path: String,
    pub size: [u32; 2],
    pub image: Option<Arc<dyn crysirror_core::image_processing::ImageResource>>,
}
#[derive(Clone)]
pub(crate) struct Policy {
    pub setup: Names,
    pub variables: Names,
    pub window: Atlas,
    pub cursor: Option<Atlas>,
}
#[derive(Clone)]
pub(crate) struct Layout {
    policy: Arc<Policy>,
    windows: Vec<[i32; 2]>,
    variables: Vec<Option<u16>>,
    ordinary_face: TextFace,
    center: Option<i32>,
    cursor_count: u64,
    cursor_at: Option<u64>,
}
impl Policy {
    fn n(&self, key: &str, default: i32) -> Result<i32> {
        Ok(self.setup.integer(key.as_bytes())?.unwrap_or(default))
    }
    pub fn layout(
        self: &Arc<Self>,
        profile: &mut SelectionProfile,
        choices: usize,
    ) -> Result<Layout> {
        let count = usize::try_from(self.n("useSpecialNumber", 0)?)?;
        ensure!(
            choices > 0 && choices <= count && count <= 64,
            "NNN special selection count/table bounds"
        );
        let ordinary_face = profile.face.clone();
        profile.face.size =
            u16::try_from(self.n("specialFontSize", i32::from(profile.face.size))?)?;
        ensure!(
            (1..=512).contains(&profile.face.size),
            "NNN special selection font size"
        );
        let size = [
            self.n("specialWindowSizeX", 800)?,
            self.n("specialWindowSizeY", 60)?,
        ];
        ensure!(
            size.iter().all(|v| (1..=8192).contains(v)),
            "NNN special selection window size"
        );
        let message = [
            self.n("specialmessagePrintX", 0)?,
            self.n("specialmessagePrintY", 0)?,
        ];
        let delta = [
            self.n("specialCursorDeltaX", self.n("cursorDeltaX", -8)?)?,
            self.n("specialCursorDeltaY", self.n("cursorDeltaY", -2)?)?,
        ];
        let cursor = [
            self.n("specialCursorSizeX", self.n("cursorSizeX", 469)?)?,
            self.n("specialCursorSizeY", self.n("cursorSizeY", 27)?)?,
        ];
        let mut windows = Vec::new();
        let mut variables = Vec::new();
        profile.rows.clear();
        for i in 0..choices {
            // Original initialization uses j * windowHeight for every row in
            // a count-j layout, not i * height. Keep that surprising default.
            let point = [
                self.n(&format!("specialWindow{choices}printX{}", i + 1), 0)?,
                self.n(
                    &format!("specialWindow{choices}printY{}", i + 1),
                    (choices as i32 - 1) * size[1],
                )?,
            ];
            let origin = [
                point[0]
                    .checked_add(message[0])
                    .context("NNN special selection x overflow")?,
                point[1]
                    .checked_add(message[1])
                    .context("NNN special selection y overflow")?,
            ];
            profile.rows.push(SelectionRow {
                text_origin: origin,
                hit_rect: [
                    origin[0]
                        .checked_add(delta[0])
                        .context("NNN special cursor x overflow")?,
                    origin[1]
                        .checked_add(delta[1])
                        .context("NNN special cursor y overflow")?,
                    cursor[0],
                    cursor[1],
                ],
            });
            windows.push(point);
            let name = self
                .setup
                .parameter(format!("specialWindowVarName{}", i + 1).as_bytes());
            variables.push(name.and_then(|name| self.variables.index(name)));
        }
        profile.validate()?;
        let center = (self.n("centeringFlag", 0)? != 0)
            .then(|| self.n("centerAreaSizeX", size[0]))
            .transpose()?;
        Ok(Layout {
            policy: self.clone(),
            windows,
            variables,
            ordinary_face,
            center,
            cursor_count: 0,
            cursor_at: None,
        })
    }
}
impl Layout {
    pub fn face<'a>(&'a self, profile: &'a SelectionProfile, supplemental: bool) -> &'a TextFace {
        if supplemental {
            &self.ordinary_face
        } else {
            &profile.face
        }
    }
    pub fn center(&self, origin: &mut [i32; 2], text: &str, size: u16) -> Result<()> {
        if let Some(area) = self.center {
            let width = i32::try_from(text.chars().count())?
                .checked_mul(i32::from(size) + 1)
                .and_then(|v| v.checked_sub(1))
                .context("NNN special message width overflow")?;
            let shift = i32::try_from((i64::from(area) - i64::from(width)) / 2)?;
            origin[0] = origin[0]
                .checked_add(shift)
                .context("NNN special centering overflow")?;
        }
        Ok(())
    }
    pub fn node_ids(&self, profile: &SelectionProfile) -> Vec<u32> {
        (0..self.windows.len())
            .map(|i| profile.layer_base + 256 + i as u32)
            .chain([profile.layer_base + 384, profile.layer_base + 385])
            .collect()
    }
    pub fn windows(&self, profile: &SelectionProfile, variables: &Variables) -> Result<Vec<Value>> {
        let size = [
            self.policy.n("specialWindowSizeX", 800)?,
            self.policy.n("specialWindowSizeY", 60)?,
        ];
        let atlas = &self.policy.window;
        ensure!(
            atlas.size[0] >= size[0] as u32 && atlas.size[1] >= size[1] as u32,
            "NNN special window atlas is smaller than a cell"
        );
        let columns = atlas.size[0] / size[0] as u32;
        let rows = atlas.size[1] / size[1] as u32;
        self.windows.iter().enumerate().map(|(i,point)|{
            let value=self.variables[i].map(|id|variables.read_calculated(id)).transpose()?.unwrap_or(0);
            let cell=if value>0 {u32::try_from(value-1)?} else {i as u32};
            let source=[(cell%columns)*size[0] as u32,((cell/columns)%rows)*size[1] as u32];
            Ok(json!({"op":"put","layer":{"id":profile.layer_base+256+i as u32,"parent":profile.root,"rect":[point[0],point[1],size[0],size[1]],"order":[profile.order-1,200+i],"paint":{"kind":"image","asset":atlas.path},"uv":[source[0] as f32/atlas.size[0] as f32,source[1] as f32/atlas.size[1] as f32,(source[0]+size[0] as u32) as f32/atlas.size[0] as f32,(source[1]+size[1] as u32) as f32/atlas.size[1] as f32]}}))
        }).collect()
    }
    fn cursor_plan(
        &mut self,
        profile: &SelectionProfile,
        host: &mut dyn HostServices,
    ) -> Result<Option<cursor::Plan>> {
        let mut selected = None;
        for i in 0..profile.rows.len() {
            let state: Value = serde_json::from_slice(&host.query_engine(
                profile.client,
                EngineQuery::UiInteraction(profile.widget_base + i as u32),
            )?)?;
            if state["hovered"].as_bool() == Some(true) {
                selected = Some(i);
                break;
            }
            if state["focused"].as_bool() == Some(true) {
                selected = Some(i);
            }
        }
        let Some(atlas) = &self.policy.cursor else {
            return Ok(None);
        };
        let n = |key: &str, default| self.policy.n(key, default);
        let patterns = n("specialAnimePatternNumber", n("animePatternNumber", 8)?)?.max(1) as u64;
        let frames = n("specialAnimeFrame", n("animeFrame", 1)?)?.max(1) as u64;
        let reverse = n("specialAnimeReverse", n("animeReverse", 0)?)? != 0;
        let period = if reverse {
            (patterns - 1) * 2
        } else {
            patterns
        }
        .max(1);
        if self.cursor_at != Some(host.now_ms()) {
            if self.cursor_at.is_some() {
                self.cursor_count = (self.cursor_count + 1) % (period * frames);
            }
            self.cursor_at = Some(host.now_ms());
        }
        let mut first = self.cursor_count / frames;
        let mut next = (first + 1) % period;
        if reverse {
            if first >= patterns {
                first = (patterns - 1) * 2 - first;
            }
            if next >= patterns {
                next = (patterns - 1) * 2 - next;
            }
        }
        let fraction = (self.cursor_count % frames) * 100 / frames;
        // Native PrintMessageSub reads the ordinary print-type member even in
        // the special branch. Preserve it rather than guessing a corrected ABI.
        let mix = n("animePrintType", 0)? != 0;
        let Some(index) = selected else {
            return Ok(None);
        };
        let rect = profile.rows[index].hit_rect;
        for frame in [first, next] {
            ensure!(
                rect[2] as u32 <= atlas.size[0]
                    && (frame + 1) * rect[3] as u64 <= u64::from(atlas.size[1]),
                "NNN special cursor atlas frame bounds"
            );
        }
        Ok(Some(cursor::Plan {
            atlas: atlas.clone(),
            rect,
            sources: [
                [0, i32::try_from(first)? * rect[3], rect[2], rect[3]],
                [0, i32::try_from(next)? * rect[3], rect[2], rect[3]],
            ],
            weights: [
                ((100 - if mix { fraction } else { 0 }) * 256 / 100) as u16,
                (if mix { fraction * 256 / 100 } else { 0 }) as u16,
            ],
        }))
    }
}
