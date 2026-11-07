// SPDX-License-Identifier: MPL-2.0
//! Native OBJECT.STRING escape/default policy. Geometry belongs to the shared
//! Text unit-flow kernel. This does not adopt the original font/sprite backend.
use crate::runtime::globals::ObjectStringParam;
use anyhow::{Context, Result, ensure};
use crysirror_text::unit_flow::{self, UnitFlowItem, UnitFlowLayout, UnitFlowOptions};

/// Original effective font policy captured when a string resource rebuilds.
/// These are logical intent values, never a font/device resource pointer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Capture {
    pub font_name: String,
    pub fallback_alias: String,
    pub bold: bool,
    pub shadow_mode: i64,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Definition {
    pub generation: u64,
    pub capture: Capture,
}
#[derive(Clone)]
pub(crate) struct Environment {
    pub capture: Capture,
    pub defaults: ObjectStringParam,
    pub gameexe: Option<std::sync::Arc<crate::gameexe::GameexeConfig>>,
    pub vertical: bool,
}
impl Environment {
    pub fn new(ctx: &crate::runtime::CommandContext) -> Self {
        Self {
            capture: capture(ctx),
            defaults: ObjectStringParam {
                moji_color: ctx.tables.mwnd_render.moji_color,
                shadow_color: ctx.tables.mwnd_render.shadow_color,
                fuchi_color: ctx.tables.mwnd_render.fuchi_color,
                shadow_mode: -1,
                ..Default::default()
            },
            gameexe: ctx.tables.gameexe.clone(),
            vertical: ctx.tables.mwnd_render.vertical_writing,
        }
    }
    pub fn request(&self, object: &crate::runtime::globals::ObjectState) -> Result<Request> {
        let definition = object
            .runtime
            .native_text
            .as_ref()
            .context("native STRING missing resource intent")?;
        Ok(Request {
            definition: definition.clone(),
            text: object.string_value.clone().unwrap_or_default(),
            param: object.string_param.clone(),
            gameexe: self.gameexe.clone(),
            vertical: self.vertical,
        })
    }
}
#[derive(Clone)]
pub(crate) struct Request {
    pub definition: Definition,
    pub text: String,
    pub param: ObjectStringParam,
    pub gameexe: Option<std::sync::Arc<crate::gameexe::GameexeConfig>>,
    pub vertical: bool,
}
/// Framework snapshots retain logical recipes only. Validate them before any
/// restored intent can schedule engine work or collide with another resource.
pub(crate) fn validate(ctx: &crate::runtime::CommandContext) -> Result<()> {
    let env = Environment::new(ctx);
    let mut generations = std::collections::BTreeMap::new();
    let mut todo = ctx
        .globals
        .stage_forms
        .values()
        .flat_map(|s| {
            s.object_lists
                .values()
                .flatten()
                .chain(s.mwnd_lists.values().flatten().flat_map(|w| {
                    w.object_list
                        .iter()
                        .chain(&w.face_list)
                        .chain(&w.button_list)
                }))
        })
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(object) = todo.pop() {
        visited += 1;
        ensure!(visited <= 4096, "native STRING snapshot tree budget");
        if object.object_type == 3 {
            let text = object.string_value.as_deref().unwrap_or_default();
            ensure!(
                text.len() <= 1024 * 1024,
                "native STRING snapshot text budget"
            );
            compile(text, &object.string_param, env.vertical)?;
            if let Some(definition) = &object.runtime.native_text {
                ensure!(
                    definition.generation > 0 && definition.generation <= ctx.native_media_revision,
                    "native STRING snapshot generation outside resource counter"
                );
                let capture = &definition.capture;
                ensure!(
                    capture.font_name.len() <= 256
                        && capture.fallback_alias.len() <= 256
                        && !capture.font_name.chars().any(char::is_control)
                        && !capture.fallback_alias.is_empty()
                        && !capture.fallback_alias.chars().any(char::is_control)
                        && (0..=3).contains(&capture.shadow_mode),
                    "native STRING snapshot font policy"
                );
                let identity = bincode::serialize(&(text, &object.string_param, capture))?;
                if let Some(previous) = generations.insert(definition.generation, identity.clone())
                {
                    ensure!(
                        previous == identity,
                        "native STRING snapshot generation aliases different intents"
                    );
                }
            }
        } else {
            ensure!(
                object.runtime.native_text.is_none(),
                "native STRING intent on a different resource type"
            );
        }
        todo.extend(&object.runtime.child_objects);
    }
    Ok(())
}
impl Request {
    pub fn color(&self, number: i64, fallback: [u8; 4]) -> [u8; 4] {
        let count = self
            .gameexe
            .as_ref()
            .and_then(|cfg| cfg.get_usize("COLOR_TABLE.CNT"))
            .unwrap_or(256)
            .clamp(1, 4096)
            .max(256);
        if number < 0 || number as usize >= count {
            return fallback;
        }
        let defaults = [
            [255, 255, 255, 255],
            [0, 0, 0, 255],
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
            [255, 0, 255, 255],
            [0, 255, 255, 255],
        ];
        let default = defaults.get(number as usize).copied().unwrap_or([255; 4]);
        let Some(cfg) = &self.gameexe else {
            return default;
        };
        let raw = cfg
            .get_indexed_value("COLOR_TABLE", number as usize)
            .or_else(|| cfg.get_indexed_field("COLOR_TABLE", number as usize, "RGB"))
            .or_else(|| cfg.get_indexed_field("COLOR_TABLE", number as usize, "COLOR"));
        let values = raw.map(|raw| {
            raw.split(|c: char| c == ',' || c.is_ascii_whitespace() || c == '(' || c == ')')
                .filter(|s| !s.is_empty())
                .map(str::parse::<i64>)
                .collect::<std::result::Result<Vec<_>, _>>()
        });
        match values {
            Some(Ok(v)) if v.len() >= 3 => [
                v[0].clamp(0, 255) as u8,
                v[1].clamp(0, 255) as u8,
                v[2].clamp(0, 255) as u8,
                255,
            ],
            _ => default,
        }
    }
}
pub(crate) fn dispatch(
    object: &mut crate::runtime::globals::ObjectState,
    ids: &crate::runtime::constants::RuntimeConstants,
    revision: &mut u64,
    op: i32,
    args: &[crate::runtime::Value],
    overload: i64,
    env: &Environment,
) -> Result<Option<crate::runtime::Value>> {
    use crate::runtime::{Value, forms::codes};
    if op == codes::ELM_OBJECT_GET_STRING {
        ensure!(
            overload == 0 && args.is_empty(),
            "native GET_STRING arguments"
        );
        return Ok(Some(Value::Str(
            object.string_value.clone().unwrap_or_default(),
        )));
    }
    let mut next = object.clone();
    if op == codes::ELM_OBJECT_CREATE_STRING || op == codes::ELM_OBJECT_SET_STRING {
        let expected = if op == codes::ELM_OBJECT_CREATE_STRING {
            match overload {
                0 => 1,
                1 => 2,
                2 => 4,
                _ => anyhow::bail!("native CREATE_STRING overload"),
            }
        } else {
            ensure!(overload == 0, "native SET_STRING overload");
            1
        };
        ensure!(args.len() == expected, "native STRING string arguments");
        let text = args[0].as_str().context("native STRING string value")?;
        ensure!(text.len() <= 1024 * 1024, "native STRING byte budget");
        let numbers = args[1..]
            .iter()
            .map(|value| value.as_i64().context("native STRING create integer"))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            numbers.iter().all(|n| i32::try_from(*n).is_ok()),
            "native STRING integer width"
        );
        if op == codes::ELM_OBJECT_CREATE_STRING {
            // DISP is legitimately opcode zero in this native ABI.
            ensure!(ids.obj_x!=0 && ids.obj_y!=0,"native STRING property layout is not mapped");
            next.reinit_like();
            next.used = true;
            next.object_type = 3;
            next.string_param = env.defaults.clone();
            next.set_int_prop(
                ids,
                ids.obj_disp,
                i64::from(numbers.first().is_some_and(|v| *v != 0)),
            );
            next.set_int_prop(ids, ids.obj_x, numbers.get(1).copied().unwrap_or(0));
            next.set_int_prop(ids, ids.obj_y, numbers.get(2).copied().unwrap_or(0));
        }
        next.string_value = Some(text.to_owned());
    } else {
        ensure!(
            op == codes::ELM_OBJECT_SET_STRING_PARAM
                && ((overload == 0 && args.len() == 7) || (overload == 1 && args.len() == 8)),
            "native SET_STRING_PARAM overload/arguments"
        );
        let values = args
            .iter()
            .map(|value| value.as_i64().context("native STRING parameter integer"))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            values.iter().all(|n| i32::try_from(*n).is_ok()),
            "native STRING parameter integer width"
        );
        next.string_param = ObjectStringParam {
            moji_size: values[0],
            moji_space_x: values[1],
            moji_space_y: values[2],
            moji_cnt: values[3],
            moji_color: values[4],
            shadow_color: values[5],
            shadow_mode: values[6],
            fuchi_color: values.get(7).copied().unwrap_or(env.defaults.fuchi_color),
        };
    }
    if next.object_type == 3 {
        let generation = revision
            .checked_add(1)
            .context("native STRING generation overflow")?;
        next.runtime.native_text = Some(Definition {
            generation,
            capture: env.capture.clone(),
        });
        let request = env.request(&next)?;
        compile(&request.text, &request.param, request.vertical)?;
        let size = crate::io_fs::text_size(request)?;
        next.runtime.native_text_size = Some(size);
        *revision = generation;
    }
    *object = next;
    Ok(None)
}
pub(crate) fn capture(ctx: &crate::runtime::CommandContext) -> Capture {
    let config = &ctx.globals.syscom.original_config;
    let override_state = if ctx.globals.syscom.msg_back_open {
        None
    } else if ctx.excall_state.ex_call_flag {
        Some((
            &ctx.excall_state.font_name,
            ctx.excall_state.font_bold,
            ctx.excall_state.font_shadow,
        ))
    } else {
        Some((
            &ctx.globals.script.font_name,
            ctx.globals.script.font_bold,
            ctx.globals.script.font_shadow,
        ))
    };
    let font_name = override_state
        .filter(|(name, _, _)| !name.is_empty())
        .map(|(name, _, _)| name.clone())
        .unwrap_or_else(|| config.font_name.clone());
    let bold = override_state
        .filter(|(_, value, _)| *value >= 0)
        .map(|(_, value, _)| value != 0)
        .unwrap_or(config.font_futoku);
    let shadow_mode = override_state
        .filter(|(_, _, value)| *value >= 0)
        .map(|(_, _, value)| value)
        .unwrap_or(config.font_shadow)
        .clamp(0, 3);
    Capture {
        font_name,
        bold,
        shadow_mode,
        fallback_alias: ctx
            .ui
            .window
            .as_ref()
            .map(|w| w.font.clone())
            .unwrap_or_else(|| "default".into()),
    }
}

/// Original native pass placement/padding. Coverage generation is the shared
/// Text square-face kernel; this module only retains native pass policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decoration {
    pub body: u8,
    pub shadow: Option<(i32, u8)>,
    pub edge: Option<(i32, u8)>,
    pub padding: i32,
}
pub(crate) fn decoration(size: u16, bold: bool, mode: i64) -> Decoration {
    let body = u8::from(bold);
    let mode = mode.clamp(0, 3);
    let edge = matches!(mode, 2 | 3).then_some((-1, 2 + body));
    let shadow = matches!(mode, 1 | 3).then(|| {
        let t = (f32::from(size) / 32.).clamp(0., 1.);
        let offset = if mode == 3 {
            (0.5 + t - 1.).round() as i32
        } else {
            ((0.5 + 1.5 * t).round() as i32).max(1)
        };
        (offset, if mode == 3 { 2 + body } else { body })
    });
    let mut padding = i32::from(body);
    if let Some((_, extent)) = edge {
        padding = padding.max(i32::from(extent));
    }
    if let Some((offset, extent)) = shadow {
        padding = padding.max(offset.max(0).saturating_add(i32::from(extent)));
    }
    Decoration {
        body,
        shadow,
        edge,
        padding: padding + 1,
    }
}

pub(crate) struct StringPlan {
    pub items: Vec<UnitFlowItem>,
    /// Original signed colour-table numbers; shared glyph styles use indices.
    pub colors: Vec<i64>,
    pub options: UnitFlowOptions,
}

/// Prepared native pass declarations on shared retained glyph runs. The font
/// worker result remains a transient engine lease, outside serialized VM data.
pub(crate) struct PreparedString {
    pub layout: UnitFlowLayout,
    pub size: [u32; 2],
    font: std::sync::Arc<dyn crysirror_core::service::GlyphBoundsResource>,
    baselines: Vec<f32>,
    colors: Vec<[u8; 4]>,
    shadow_color: [u8; 4],
    edge_color: [u8; 4],
    decorations: Vec<Decoration>,
}
impl PreparedString {
    pub fn new(
        client: crysirror_core::state::ClientId,
        plan: &StringPlan,
        capture: &Capture,
        param: &ObjectStringParam,
        font: std::sync::Arc<dyn crysirror_core::service::GlyphBoundsResource>,
        baselines: Vec<f32>,
        colors: Vec<[u8; 4]>,
        shadow_color: [u8; 4],
        edge_color: [u8; 4],
    ) -> Result<Self> {
        let layout = plan.layout()?;
        ensure!(
            !plan.options.vertical,
            "native vertical STRING glyph orientation still requires its shared rendering profile"
        );
        ensure!(
            font.client() == client
                && font.glyphs().len() == layout.glyphs.len()
                && baselines.len() == layout.glyphs.len(),
            "native STRING glyph receipt ownership/count"
        );
        ensure!(
            baselines
                .iter()
                .all(|v| v.is_finite() && (0.0..=1024.).contains(v)),
            "native STRING baseline budget"
        );
        ensure!(
            colors.len() == plan.colors.len(),
            "native STRING palette receipt"
        );
        let mode = if param.shadow_mode == -1 {
            capture.shadow_mode
        } else {
            param.shadow_mode.clamp(0, 3)
        };
        let decorations = layout
            .glyphs
            .iter()
            .map(|g| decoration(g.size, capture.bold, mode))
            .collect::<Vec<_>>();
        let mut minimum = [0i64; 2];
        // Pinned native backend starts from the returned layout WIDTH/HEIGHT,
        // not its minimum/maximum. Keep this even for negative inline moves.
        let mut maximum = layout.extent.map(i64::from);
        for ((glyph, bounds), decoration) in
            layout.glyphs.iter().zip(font.glyphs()).zip(&decorations)
        {
            for axis in 0..2 {
                let origin = i64::from(bounds.origin[axis]);
                let pad = i64::from(decoration.padding);
                let lo = 0.min(origin.saturating_sub(pad));
                let hi = 1.max(
                    origin
                        .saturating_add(i64::from(bounds.size[axis]))
                        .saturating_add(pad),
                );
                minimum[axis] =
                    minimum[axis].min(i64::from(glyph.position[axis]).saturating_add(lo));
                maximum[axis] =
                    maximum[axis].max(i64::from(glyph.position[axis]).saturating_add(hi));
            }
        }
        let size = std::array::from_fn(|i| {
            maximum[i]
                .saturating_sub(minimum[i])
                .clamp(1, i64::from(u32::MAX)) as u32
        });
        Ok(Self {
            layout,
            size,
            font,
            baselines,
            colors,
            shadow_color,
            edge_color,
            decorations,
        })
    }
    pub fn layers(
        &self,
        next: &mut u32,
        parent: Option<u32>,
        order: [i32; 2],
        opacity: u8,
        visible: bool,
        blend: crysirror_scene::layer::Blend,
        filter: Option<crysirror_scene::layer::LayerFilter>,
        source_crop: Option<[f32;4]>,
    ) -> Result<Vec<crysirror_scene::layer::LayerSpec>> {
        use crysirror_scene::layer::{
            GlyphBlur, GlyphCoverageKernel, LayerSpec, Paint, PositionedGlyph,
        };
        let mut out = Vec::new();
        // Regular STRING sprites have no MWND pass-layer offsets. Their
        // authoritative creation order is glyph0 shadow/edge/body, then glyph1.
        for (((glyph, baseline), decoration), bounds) in self
            .layout
            .glyphs
            .iter()
            .zip(&self.baselines)
            .zip(&self.decorations)
            .zip(self.font.glyphs())
        {
            // Original STRING creates one tight texture per glyph and pass.
            // Translate its native 0-based source crop into the containing run;
            // face/shadow offsets live inside that same padded texture.
            let crop=source_crop.map(|mut crop| {
                for axis in 0..2 {
                    let minimum=(bounds.origin[axis]-decoration.padding).min(0);
                    crop[axis]+=glyph.position[axis] as f32+minimum as f32;
                }
                crop
            });
            let color = *self
                .colors
                .get(glyph.style as usize)
                .context("native STRING style index")?;
            for (offset, extent, color) in decoration
                .shadow
                .map(|(o, e)| (o, e, self.shadow_color))
                .into_iter()
                .chain(decoration.edge.map(|(o, e)| (o, e, self.edge_color)))
                .chain(Some((0, decoration.body, color)))
            {
                ensure!(out.len() < 1900, "native STRING pass budget");
                let id = *next;
                *next = next
                    .checked_add(1)
                    .context("native STRING layer identity overflow")?;
                out.push(LayerSpec { tie_order:None, clock_paused: false, source_crop: None,
                    viewport_clip: None,
                    id,
                    parent,
                    rect: [0., 0., self.size[0] as f32, self.size[1] as f32],
                    paint: Paint::GlyphRun {outline: None, 
                        font: self.font.font().to_owned(),
                        size: f32::from(glyph.size),
                        color,
                        bold: false,
                        glyphs: vec![PositionedGlyph { image: None, style: None, font: None,horizontal_scale: None, font_width: None, cell_width: None,  crop,
                            character: glyph.character,
                            baseline: [
                                glyph.position[0] as f32 + offset as f32,
                                glyph.position[1] as f32 + *baseline + offset as f32,
                            ],
                            size: None,
                            color: None,
                            bold: None,
                        }],
                        reveal: None,
                        blur: (extent > 0).then_some(GlyphBlur {
                            radius: extent,
                            level: 256,
                            kernel: GlyphCoverageKernel::SquareFace,
                        }),
                    },
                    transform: Default::default(),
                    order,
                    clip: None,
                    opacity,
                    visible,
                    blend,
                    uv: None,
                    sampling: Default::default(),
                    filter: filter.clone(),
                });
            }
        }
        Ok(out)
    }
}
impl StringPlan {
    pub fn layout(&self) -> Result<UnitFlowLayout> {
        unit_flow::layout_unit_flow(&self.items, self.options)
    }
}

// Whole original native numeric escape reader; saturation precedes sign.
fn integer(chars: &[char], start: usize) -> Option<(i64, usize)> {
    let mut i = start;
    let mut sign = 1i64;
    if chars.get(i) == Some(&'-') {
        sign = -1;
        i += 1;
    } else if chars.get(i) == Some(&'+') {
        i += 1;
    }
    let begin = i;
    let mut value = 0i64;
    while let Some(ch) = chars.get(i) {
        let Some(digit) = ch.to_digit(10) else {
            break;
        };
        value = value.saturating_mul(10).saturating_add(i64::from(digit));
        i += 1;
    }
    (i > begin).then_some((value.saturating_mul(sign), i))
}

/// Compile only native #C/#S/#X/#Y/#RX/#RY/#D/## syntax. Invalid escapes
/// retain the original literal '#' branch. No font discovery or layout math.
pub(crate) fn compile(text: &str, param: &ObjectStringParam, vertical: bool) -> Result<StringPlan> {
    ensure!(text.len() <= 1024 * 1024, "native STRING byte budget");
    let chars = text.chars().collect::<Vec<_>>();
    ensure!(
        chars.len() <= unit_flow::MAX_FLOW_ITEMS,
        "native STRING scalar budget"
    );
    let options = UnitFlowOptions {
        vertical,
        spacing: [
            param
                .moji_space_x
                .try_into()
                .context("native STRING horizontal spacing width")?,
            param
                .moji_space_y
                .try_into()
                .context("native STRING line spacing width")?,
        ],
        wrap_units: param
            .moji_cnt
            .max(0)
            .saturating_mul(2)
            .try_into()
            .context("native STRING wrap-unit width")?,
    };
    let mut size = param.moji_size.max(1);
    ensure!(size <= 512, "native STRING glyph-size budget");
    let mut colors = vec![param.moji_color];
    let mut indices = std::collections::BTreeMap::from([(param.moji_color, 0u32)]);
    let mut style = 0u32;
    let mut i = 0;
    let mut items = Vec::new();
    while i < chars.len() {
        let mut ch = chars[i];
        if ch == '#' {
            if chars.get(i + 1) == Some(&'#') {
                ch = '#';
                i += 2;
            } else if chars.get(i + 1) == Some(&'D') {
                i += 2;
                items.push(UnitFlowItem::LineBreak { size: size as u16 });
                continue;
            } else if let Some((number, mut next)) = integer(&chars, i + 1) {
                let command = if chars.get(next) == Some(&'R')
                    && matches!(chars.get(next + 1).copied(), Some('X' | 'Y'))
                {
                    let axis = chars[next + 1];
                    next += 2;
                    Some((true, axis))
                } else if matches!(chars.get(next).copied(), Some('C' | 'S' | 'X' | 'Y')) {
                    let axis = chars[next];
                    next += 1;
                    Some((false, axis))
                } else {
                    None
                };
                if let Some((relative, command)) = command {
                    match command {
                        'C' => {
                            style = *indices.entry(number).or_insert_with(|| {
                                let index = colors.len() as u32;
                                colors.push(number);
                                index
                            });
                        }
                        'S' => {
                            size = number.max(1);
                            ensure!(size <= 512, "native inline STRING size budget");
                        }
                        'X' => items.push(UnitFlowItem::Move {
                            x: Some(number),
                            y: None,
                            relative,
                        }),
                        'Y' => items.push(UnitFlowItem::Move {
                            x: None,
                            y: Some(number),
                            relative,
                        }),
                        _ => unreachable!(),
                    }
                    i = next;
                    continue;
                }
                i += 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
        items.push(UnitFlowItem::Glyph {
            character: ch,
            size: size as u16,
            units: if unit_flow::half_cell(ch) { 1 } else { 2 },
            style,
        });
    }
    Ok(StringPlan {
        items,
        colors,
        options,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_glyph_receipts_build_engine_passes_in_original_per_character_order() {
        use crysirror_core::{
            service::{GlyphBounds, GlyphBoundsResource},
            state::ClientId,
        };
        use crysirror_scene::{
            EngineSystems, ServiceBatch,
            layer::{Blend, LayerCommand, Paint},
        };
        struct Fixture {
            owner: ClientId,
            glyphs: Vec<GlyphBounds>,
        }
        impl GlyphBoundsResource for Fixture {
            fn client(&self) -> ClientId {
                self.owner
            }
            fn font(&self) -> &str {
                "fixture-font"
            }
            fn digest(&self) -> &str {
                "fixture-digest"
            }
            fn glyphs(&self) -> &[GlyphBounds] {
                &self.glyphs
            }
        }
        let owner = ClientId(1);
        let font = std::sync::Arc::new(Fixture {
            owner,
            glyphs: vec![
                GlyphBounds {
                    glyph_id: 4,
                    origin: [-1, 2],
                    size: [4, 5]
                };
                2
            ],
        });
        let param = ObjectStringParam::default();
        let plan = compile("AB", &param, false).unwrap();
        let capture = Capture {
            font_name: "requested".into(),
            fallback_alias: "fixture-font".into(),
            bold: true,
            shadow_mode: 3,
        };
        let prepared = PreparedString::new(
            owner,
            &plan,
            &capture,
            &param,
            font.clone(),
            vec![9.; 2],
            vec![[255, 0, 0, 255]],
            [0, 0, 0, 255],
            [0, 255, 0, 255],
        )
        .unwrap();
        assert_eq!(prepared.size, [23, 14]);
        let mut next = 100;
        let layers = prepared
            .layers(&mut next, None, [5, 7], 128, true, Blend::Add, None, None)
            .unwrap();
        let cropped=prepared.layers(&mut 200,None,[5,7],128,true,Blend::Add,None,Some([2.,3.,4.,5.])).unwrap();
        // bounds.origin=-1,2; original full-style padding=4 => tight origin=-5,-2.
        // The second glyph keeps its own translated tight texture, not one whole-text clip.
        for (index,layer) in cropped.iter().enumerate() {
            let Paint::GlyphRun{glyphs,..}=&layer.paint else {panic!()};
            let glyph=&prepared.layout.glyphs[index/3];
            assert_eq!(glyphs[0].crop,Some([glyph.position[0] as f32-3.,glyph.position[1] as f32+1.,4.,5.]));
        }
        assert_eq!(next, 106);
        let colors = layers
            .iter()
            .map(|layer| match &layer.paint {
                Paint::GlyphRun { color, .. } => *color,
                _ => panic!("shared glyph paint"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            colors,
            [
                [0, 0, 0, 255],
                [0, 255, 0, 255],
                [255, 0, 0, 255],
                [0, 0, 0, 255],
                [0, 255, 0, 255],
                [255, 0, 0, 255]
            ]
        );
        assert!(layers.iter().all(|layer| layer.opacity == 128
            && layer.blend == Blend::Add
            && layer.order == [5, 7]));
        let mut engine = EngineSystems::default();
        engine
            .submit(
                owner,
                &ServiceBatch {
                    layers: layers
                        .into_iter()
                        .map(|layer| LayerCommand::Put { layer })
                        .collect(),
                    ..Default::default()
                },
            )
            .unwrap();
        let frame = engine.frame([64, 64]).unwrap();
        assert_eq!(frame.len(), 6);
        assert_eq!(
            frame.iter().map(|part| part.id).collect::<Vec<_>>(),
            [100, 101, 102, 103, 104, 105]
        );
        assert!(
            PreparedString::new(
                ClientId(2),
                &plan,
                &capture,
                &param,
                font.clone(),
                vec![9.; 2],
                vec![[0; 4]],
                [0; 4],
                [0; 4]
            )
            .is_err()
        );
        let vertical = compile("AB", &param, true).unwrap();
        assert!(
            PreparedString::new(
                owner,
                &vertical,
                &capture,
                &param,
                font,
                vec![9.; 2],
                vec![[0; 4]],
                [0; 4],
                [0; 4]
            )
            .is_err()
        );
    }
    #[test]
    fn string_font_capture_uses_original_config_excall_and_backlog_precedence() {
        let mut ctx = crate::runtime::CommandContext::new(".".into());
        ctx.globals.script.font_name = "local".into();
        ctx.globals.script.font_bold = 2;
        ctx.globals.script.font_shadow = 99;
        let local = capture(&ctx);
        assert_eq!(
            (local.font_name.as_str(), local.bold, local.shadow_mode),
            ("local", true, 3)
        );
        ctx.excall_state.ex_call_flag = true;
        ctx.excall_state.font_name = "private".into();
        ctx.excall_state.font_bold = 0;
        ctx.excall_state.font_shadow = 1;
        let private = capture(&ctx);
        assert_eq!(
            (
                private.font_name.as_str(),
                private.bold,
                private.shadow_mode
            ),
            ("private", false, 1)
        );
        ctx.globals.syscom.original_config.font_name = "config".into();
        ctx.globals.syscom.original_config.font_futoku = true;
        ctx.globals.syscom.original_config.font_shadow = 2;
        ctx.globals.syscom.msg_back_open = true;
        let backlog = capture(&ctx);
        assert_eq!(
            (
                backlog.font_name.as_str(),
                backlog.bold,
                backlog.shadow_mode
            ),
            ("config", true, 2)
        );
        assert_eq!(backlog.fallback_alias, "default");
    }
    #[test]
    fn complete_native_four_mode_pass_policy_keeps_face_offsets_and_padding() {
        assert_eq!(
            decoration(12, false, 0),
            Decoration {
                body: 0,
                shadow: None,
                edge: None,
                padding: 1
            }
        );
        assert_eq!(
            decoration(12, true, 1),
            Decoration {
                body: 1,
                shadow: Some((1, 1)),
                edge: None,
                padding: 3
            }
        );
        assert_eq!(
            decoration(12, false, 2),
            Decoration {
                body: 0,
                shadow: None,
                edge: Some((-1, 2)),
                padding: 3
            }
        );
        assert_eq!(
            decoration(32, true, 3),
            Decoration {
                body: 1,
                shadow: Some((1, 3)),
                edge: Some((-1, 3)),
                padding: 5
            }
        );
        assert_eq!(decoration(512, true, 99), decoration(32, true, 3));
    }
    #[test]
    fn whole_native_escape_family_retains_units_styles_and_shared_geometry() {
        let param = ObjectStringParam {
            moji_size: 13,
            moji_space_x: -2,
            moji_space_y: 3,
            moji_cnt: 2,
            moji_color: 7,
            ..Default::default()
        };
        let plan = compile("A#5Cあ#9S#-3RX#12YB#D##", &param, false).unwrap();
        assert_eq!(plan.colors, [7, 5]);
        let layout = plan.layout().unwrap();
        assert_eq!(
            layout
                .glyphs
                .iter()
                .map(|g| g.character)
                .collect::<String>(),
            "AあB#"
        );
        assert_eq!(
            layout.glyphs.iter().map(|g| g.position).collect::<Vec<_>>(),
            [[0, 0], [5, 0], [13, 12], [0, 24]]
        );
        assert_eq!(
            layout.glyphs.iter().map(|g| g.style).collect::<Vec<_>>(),
            [0, 1, 1, 1]
        );
        assert_eq!(layout.extent, [22, 33]);
        assert!(compile("#513SA", &param, false).is_err());
        assert_eq!(
            compile("#-9SA", &param, false)
                .unwrap()
                .layout()
                .unwrap()
                .glyphs[0]
                .size,
            1
        );
    }
    #[test]
    fn unknown_escape_is_literal_and_absolute_moves_saturate_like_original_numbers() {
        let param = ObjectStringParam::default();
        let plan = compile("#12Q#R#", &param, false).unwrap();
        assert_eq!(
            plan.layout()
                .unwrap()
                .glyphs
                .iter()
                .map(|g| g.character)
                .collect::<String>(),
            "#12Q#R#"
        );
        let plan = compile("#9223372036854775808X#-9223372036854775808YA", &param, true).unwrap();
        let result = plan.layout().unwrap();
        assert_eq!(result.glyphs[0].position, [i32::MAX, i32::MIN]);
        assert_eq!(result.minimum[1], -i64::MAX);
        assert_eq!(result.extent, [u32::MAX, u32::MAX]);
    }
}
