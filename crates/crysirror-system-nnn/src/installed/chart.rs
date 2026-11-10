// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonChartMenu native layout/input policy. Assets, clipping, integer
//! strokes, scaling, masks and text are engine resources/jobs/layers/UI.
use super::*;
use crate::{
    config::Names,
    expression::Variables,
    native_chart::{ChartRuntime, ChartState},
};
use crysirror_core::{
    image_processing::ImageResource,
    service::{EngineQuery, buttons},
};
use crysirror_image::{
    effects::ImageEffect, integer_lines::IntegerLine, sprite_alpha::SpriteAlpha,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};
pub(super) const ROOT: u32 = 0x66050000;
const LAYER: u32 = ROOT + 1;
const ORDER: i32 = 2_000_000_002;
pub(super) enum Event {
    Choice,
    System,
}
struct Prepared {
    inputs: VecDeque<(String, ImageInput, Vec<ImageEffect>)>,
    background: Option<menu::Background>,
    sheet: [i32; 2],
    back_height: i32,
}
struct Asset {
    path: String,
    size: [u32; 2],
}
struct Loading {
    prepared: Prepared,
    job: Option<(String, ImageJob)>,
    images: Vec<(String, Arc<dyn ImageResource>)>,
}
#[derive(Clone, Copy)]
enum Drag {
    Main(i32, i32),
    Mini(i32, i32),
    Node(i32, [i32; 2], [i32; 2]),
}
pub(super) struct ChartPlayer {
    config: Arc<crate::native_bootstrap::NativeConfiguration>,
    resources: NativeResources,
    pool: TaskPool,
    client: ClientId,
    task: Option<Task<Prepared>>,
    loading: Option<Loading>,
    assets: BTreeMap<String, Asset>,
    retired_paths: Vec<String>,
    pub background: Option<menu::Background>,
    pub background_started: bool,
    group: usize,
    current: i32,
    scroll: i32,
    horizontal: i32,
    ticks: u64,
    last_ms: u64,
    remainder: u64,
    hovered: Option<i32>,
    description: Option<i32>,
    description_up: bool,
    pointer: [i32; 2],
    drag: Option<Drag>,
    moved: BTreeMap<i32, [i32; 2]>,
    confirmed: Option<i32>,
    pub selected: Option<i32>,
    sounds: VecDeque<i32>,
    sheet: [i32; 2],
    back_height: i32,
    line_job: Option<ImageJob>,
    line_path: Option<String>,
    layer_count: u32,
    ui_nodes: Vec<u32>,
    dirty: bool,
    queued_input: InputFrame,
}
fn n(t: &Names, key: &str, default: i32) -> Result<i32> {
    Ok(t.integer(key.as_bytes())?.unwrap_or(default))
}
fn size(t: &Names, key: &str, default: i32) -> Result<i32> {
    let value = n(t, key, default)?;
    ensure!((1..=16384).contains(&value), "NNN chart {key} size bounds");
    Ok(value)
}
fn scroll_height(runtime: &ChartRuntime, group: usize) -> Result<i32> {
    let height = runtime.definition.maps[group]
        .height
        .checked_add(n(&runtime.definition.setup, "bottomSpace", 60)?)
        .context("NNN chart scroll extent overflow")?;
    ensure!(
        (601..=1_048_576).contains(&height),
        "NNN chart scroll extent"
    );
    Ok(height)
}
fn pulse(ticks: u64, period: i32) -> Result<i32> {
    ensure!((2..=864000).contains(&period), "NNN chart animation period");
    let phase = (ticks % period as u64) as i32;
    Ok(phase.min(period - phase))
}
impl ChartPlayer {
    fn queue_input(&mut self, input: InputFrame) {
        self.queued_input.buttons_pressed |= input.buttons_pressed;
        self.queued_input.buttons_down = input.buttons_down;
        self.queued_input.pointer_x = input.pointer_x;
        self.queued_input.pointer_y = input.pointer_y;
    }
    pub fn has_sound(&self) -> bool {
        !self.sounds.is_empty()
    }
    pub fn dependency_pending(&self) -> bool {
        self.task.is_some()
            || self.loading.is_some()
            || self.line_job.is_some()
            || self.selected.is_some()
            || !self.sounds.is_empty()
    }
    /// Complete admitted asset/raster work only. No input drain, native clock
    /// step, background-script batch or new graph frame is scheduled here.
    pub fn poll_dependencies(
        &mut self,
        vm: &NativeAdapter,
        host: &mut dyn HostServices,
        font: &str,
    ) -> Result<()> {
        if !self.prepare(host)? {
            return Ok(());
        }
        if let Some(job) = self.line_job.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => (),
                ImageProcessState::Failed(error) => {
                    anyhow::bail!("NNN chart graph completion: {error}")
                }
                ImageProcessState::Ready(image) => {
                    let path = host
                        .publish_image_resource(self.client, ROOT + 9000, image)?
                        .path
                        .relative()
                        .to_owned();
                    let old = self.line_path.replace(path);
                    self.line_job = None;
                    let (runtime, state) = vm.chart_view()?;
                    let mut variables = vm.gallery_variables()?;
                    self.draw(runtime, state, &mut variables, host, font)?;
                    if let Some(old) = old {
                        host.release_image_resource(self.client, &old)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn new(
        vm: &NativeAdapter,
        resources: NativeResources,
        pool: TaskPool,
        client: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<Self> {
        let config = vm
            .configuration()
            .cloned()
            .context("NNN chart configuration absent")?;
        let (runtime, _) = vm.chart_view()?;
        let mut variables = vm.gallery_variables()?;
        let current = runtime.slot(&mut variables)?;
        let group = if let Some(id) = runtime.definition.print_variable {
            usize::try_from(variables.read_stored(id))?
        } else {
            runtime.definition.chart(current)?
        };
        ensure!(
            group < runtime.definition.maps.len(),
            "NNN chart displayed map bounds"
        );
        let scroll = (runtime.definition.node(current)?.position[1] - 300)
            .clamp(0, scroll_height(runtime, group)? - 600);
        let mut this = Self {
            config,
            resources,
            pool,
            client,
            task: None,
            loading: None,
            assets: BTreeMap::new(),
            retired_paths: vec![],
            background: None,
            background_started: false,
            group,
            current,
            scroll,
            horizontal: 0,
            ticks: 0,
            last_ms: host.now_ms(),
            remainder: 0,
            hovered: None,
            description: None,
            description_up: false,
            pointer: [0; 2],
            drag: None,
            moved: BTreeMap::new(),
            confirmed: None,
            selected: None,
            sounds: VecDeque::new(),
            sheet: [800, 600],
            back_height: 1200,
            line_job: None,
            line_path: None,
            layer_count: 0,
            ui_nodes: vec![],
            dirty: true,
            queued_input: InputFrame::default(),
        };
        this.load(runtime, host)?;
        Ok(this)
    }
    fn load(&mut self, runtime: &ChartRuntime, host: &mut dyn HostServices) -> Result<()> {
        self.line_job = None;
        let config = self.config.clone();
        let resources = self.resources.clone();
        let pool = self.pool.clone();
        let storage = host.storage(self.client)?;
        let definition = runtime.definition.clone();
        let group = self.group;
        self.task = Some(pool.spawn(move |cancel| {
            let table = &definition.setup;
            crate::native_general::ModeReturn::validate_entry(table)?;
            let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
            let width = size(table, "slotSizeX", 200)?;
            let height = size(table, "slotSizeY", 50)?;
            let mut inputs = VecDeque::new();
            let mut budget = 0u64;
            let mut push = |key: String, name: Vec<u8>, opaque: bool| -> Result<Option<[u32; 2]>> {
                cancel.check()?;
                let path = [b"sys/".as_slice(), &name].concat();
                let Some(source) = resources
                    .optional_image_source(&config.resources.picture(&path, levels)?, &cancel)?
                else {
                    return Ok(None);
                };
                let size = source.size;
                budget = budget
                    .checked_add(u64::from(size[0]) * u64::from(size[1]) * 4 + source.encoded_len as u64)
                    .context("NNN chart asset budget overflow")?;
                ensure!(
                    budget <= 256 << 20 && inputs.len() < 2048,
                    "NNN chart resident asset budget"
                );
                let alpha = if opaque {
                    SpriteAlpha::Opaque
                } else if source.has_mask {
                    SpriteAlpha::Preserve
                } else {
                    SpriteAlpha::CornerKey { right: false }
                };
                if key.starts_with("slot") {
                    let spec = crysirror_image::fixed_luma::FixedLumaSpec {
                        coefficients: if source.has_mask {
                            [76, 150, 29]
                        } else {
                            [0, 256, 0]
                        },
                        groups: if source.has_mask { [1, 0, 0] } else { [0; 3] },
                        fractional_bits: 8,
                        amount256: 256,
                        tint: None,
                    };
                    inputs.push_back((
                        format!("grey_{key}"),
                        source.input.clone(),
                        vec![
                            ImageEffect::SpriteAlpha {
                                spec: alpha.clone(),
                            },
                            ImageEffect::FixedLuma { spec },
                        ],
                    ));
                    budget = budget
                        .checked_add(u64::from(size[0]) * u64::from(size[1]) * 4)
                        .context("NNN chart grey asset budget overflow")?;
                    ensure!(budget <= 256 << 20, "NNN chart grey asset memory budget");
                }
                inputs.push_back((
                    key,
                    source.input,
                    vec![ImageEffect::SpriteAlpha { spec: alpha }],
                ));
                Ok(Some(size))
            };
            let slot_tag = table
                .parameter(b"tagNameChartSlot")
                .unwrap_or(b"ta_chart_button");
            let first = [slot_tag, b"1_1"].concat();
            let sheet = push("dimension".into(), first, false)?.unwrap_or([800, 600]);
            let sheet = [i32::try_from(sheet[0])?, i32::try_from(sheet[1])?];
            let cells = (sheet[0] / width)
                .checked_mul(sheet[1] / height)
                .context("NNN chart atlas cell overflow")?;
            ensure!(cells > 0, "NNN chart slot larger than original atlas");
            let maximum = definition.maps[group]
                .nodes
                .iter()
                .max()
                .copied()
                .unwrap_or((group * definition.block_size) as i32)
                - (group * definition.block_size) as i32;
            for index in 0..(maximum + 1 + cells - 1) / cells {
                push(
                    format!("slot{index}"),
                    [slot_tag, format!("{}_{}", group + 1, index + 1).as_bytes()].concat(),
                    false,
                )?;
            }
            let common = n(table, "backPicNumberCommon", 2)?.max(1);
            ensure!(common <= 64, "NNN chart common background page budget");
            let default_height = n(table, "backPicSizeY", 600 * common)?;
            let (count, back_height) = if let Some(count) =
                table.integer(format!("backPicNumber{}", group + 1).as_bytes())?
            {
                let count = count.max(1);
                ensure!(count <= 64, "NNN chart background page budget");
                (count, count * 600)
            } else if let Some(height) =
                table.integer(format!("backPicSizeY{}", group + 1).as_bytes())?
            {
                ensure!(
                    (1..=38400).contains(&height),
                    "NNN chart background height budget"
                );
                (((height + 599) / 600).max(1), height)
            } else {
                (common, default_height)
            };
            ensure!(
                count <= 64 && back_height > 0,
                "NNN chart background extent"
            );
            if n(table, "picLoadBackScript", 0)? == 0 {
                for index in 0..count {
                    push(
                        format!("back{index}"),
                        [
                            table.parameter(b"tagNameBack").unwrap_or(b"chart_back"),
                            format!("{}_{}", group + 1, index + 1).as_bytes(),
                        ]
                        .concat(),
                        true,
                    )?;
                }
                push(
                    "center".into(),
                    [
                        table
                            .parameter(b"tagNameCenter")
                            .unwrap_or(b"ta_chart_center"),
                        format!("{}", group + 1).as_bytes(),
                    ]
                    .concat(),
                    false,
                )?;
            }
            for (key, field, default) in [
                ("unknown", "filenameNotLook", "ta_chart_button0"),
                ("cursor", "filenameButtonCursor", "ta_chart_buttoncur"),
                ("complete", "fileNameComp", "ta_compmark"),
                ("description", "filenameDescWindow", "ta_chart_meswin1"),
                ("mini", "filenameMiniMapButton", "ta_chart_minimap01"),
                ("mini_window", "filenameMiniMapWindow", "ta_chart_minimap03"),
                ("percent_base", "filenameTasseido", "ta_chart_comp"),
                ("digits", "filenameChartFont", "ta_chart_font01"),
                ("now", "filenameNowChart", "ta_chartmap_cur01"),
            ] {
                push(
                    key.into(),
                    table
                        .parameter(field.as_bytes())
                        .unwrap_or(default.as_bytes())
                        .to_vec(),
                    false,
                )?;
            }
            push(
                "mini_base".into(),
                table
                    .parameter(format!("filenameMiniMapBase{}", group + 1).as_bytes())
                    .or_else(|| table.parameter(b"filenameMiniMapBase"))
                    .unwrap_or(b"ta_chart_minimap02")
                    .to_vec(),
                false,
            )?;
            if n(table, "mainCaptureOk", 1)? != 0 {
                push(
                    "capture".into(),
                    table
                        .parameter(b"filenameCapturePic")
                        .unwrap_or(b"ta_capture")
                        .to_vec(),
                    false,
                )?;
            }
            let background = if n(table, "backScriptFlag", 0)? != 0 {
                let name = table
                    .parameter(b"backScriptName")
                    .context("NNN chart background script name absent")?
                    .to_vec();
                let path = config.resources.script(&name)?;
                let bytes = read(storage.as_ref(), &path, 16 << 20, &cancel)?
                    .context("NNN chart background script missing")?;
                Some(menu::Background {
                    name,
                    program: Arc::new(Program::parse(&bytes, config.scripts)?),
                    looping: n(table, "backScriptLoopFlag", 1)? != 0,
                })
            } else {
                None
            };
            Ok(Prepared {
                inputs,
                background,
                sheet,
                back_height,
            })
        })?);
        self.dirty = true;
        Ok(())
    }
    pub fn prepare(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if let Some(task) = self.task.as_mut() {
            let Some(prepared) = task.try_take()? else {
                return Ok(false);
            };
            self.task = None;
            self.loading = Some(Loading {
                prepared,
                job: None,
                images: vec![],
            });
        }
        if let Some(loading) = self.loading.as_mut() {
            if let Some((key, job)) = loading.job.as_mut() {
                match job.poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => anyhow::bail!("NNN chart image: {e}"),
                    ImageProcessState::Ready(image) => loading.images.push((key.clone(), image)),
                };
                loading.job = None;
            }
            if let Some((key, input, effects)) = loading.prepared.inputs.pop_front() {
                loading.job = Some((
                    key,
                    host.image_metadata(self.client)?
                        .begin_process(input, serde_json::to_vec(&effects)?.into())?,
                ));
                return Ok(false);
            }
            let loading = self.loading.take().unwrap();
            let mut assets = BTreeMap::new();
            for (index, (key, image)) in loading.images.into_iter().enumerate() {
                let size = image.size();
                let published = match host.publish_image_resource(
                    self.client,
                    ROOT + 10000 + index as u32,
                    image,
                ) {
                    Ok(published) => published,
                    Err(error) => {
                        for asset in assets.values() {
                            let asset: &Asset = asset;
                            let _ = host.release_image_resource(self.client, &asset.path);
                        }
                        return Err(error.into());
                    }
                };
                assets.insert(
                    key,
                    Asset {
                        path: published.path.relative().to_owned(),
                        size,
                    },
                );
            }
            for (_, asset) in std::mem::replace(&mut self.assets, assets) {
                self.retired_paths.push(asset.path);
            }
            self.background = loading.prepared.background;
            self.sheet = loading.prepared.sheet;
            self.back_height = loading.prepared.back_height;
            self.last_ms = host.now_ms();
            self.dirty = true;
        }
        Ok(true)
    }
    fn area(&self) -> Result<[i32; 4]> {
        let t = self.config.chart_setup.as_ref().unwrap();
        let x = n(t, "slotAreaStartX", 200)?;
        let y = n(t, "slotAreaStartY", 0)?;
        ensure!(
            x.unsigned_abs() <= 1_000_000 && y.unsigned_abs() <= 1_000_000,
            "NNN chart area origin budget"
        );
        Ok([
            x,
            y,
            size(t, "slotAreaSizeX", 800 - x)?,
            size(t, "slotAreaSizeY", 600 - y)?,
        ])
    }
    fn mini(&self) -> Result<[i32; 4]> {
        let t = self.config.chart_setup.as_ref().unwrap();
        let g = self.group + 1;
        let field =
            |key: &str, default| -> Result<i32> { n(t, &format!("{key}{g}"), n(t, key, default)?) };
        let x = field("miniMapBasePrintX", 10)?
            .checked_add(field("miniMapPrintX", 0)?)
            .context("NNN chart minimap X overflow")?;
        let y = field("miniMapBasePrintY", 300)?
            .checked_add(field("miniMapPrintY", 30)?)
            .context("NNN chart minimap Y overflow")?;
        let width = field("miniMapSizeX", 120)?.max(1);
        let height = field("miniMapSizeY", 180)?.max(1);
        ensure!(
            x.unsigned_abs() <= 1_000_000
                && y.unsigned_abs() <= 1_000_000
                && width <= 16384
                && height <= 16384,
            "NNN chart minimap bounds"
        );
        Ok([x, y, width, height])
    }
    fn position(&self, runtime: &ChartRuntime, slot: i32) -> Result<[i32; 2]> {
        Ok(self
            .moved
            .get(&slot)
            .copied()
            .unwrap_or(runtime.definition.node(slot)?.position))
    }
    fn point(&self, runtime: &ChartRuntime, slot: i32) -> Result<[i32; 2]> {
        let p = self.position(runtime, slot)?;
        let a = self.area()?;
        Ok([p[0] + a[0], p[1] + a[1] - self.scroll])
    }
    fn available(
        &self,
        runtime: &ChartRuntime,
        state: &ChartState,
        variables: &mut Variables,
        slot: i32,
    ) -> Result<(bool, bool)> {
        let allowed = runtime.selectable(state, slot, variables)?;
        let direct = allowed && runtime.definition.node(self.current)?.next.contains(&slot);
        Ok((direct, allowed && runtime.seen(slot)?))
    }
    pub fn poll(
        &mut self,
        vm: &mut NativeAdapter,
        host: &mut dyn HostServices,
        input: InputFrame,
        font: &str,
    ) -> Result<Option<Event>> {
        self.queue_input(input);
        if !self.prepare(host)? {
            return Ok(None);
        }
        let (runtime, state) = vm.chart_view()?;
        let runtime = runtime.clone();
        let state = state.clone();
        let runtime = &runtime;
        let state = &state;
        let mut variables = vm.gallery_variables()?;
        if let Some(job) = self.line_job.as_mut() {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(None),
                ImageProcessState::Failed(e) => anyhow::bail!("NNN chart graph lines: {e}"),
                ImageProcessState::Ready(image) => {
                    let path = host
                        .publish_image_resource(self.client, ROOT + 9000, image)?
                        .path
                        .relative()
                        .to_owned();
                    let previous = self.line_path.replace(path);
                    self.line_job = None;
                    self.draw(runtime, state, &mut variables, host, font)?;
                    if let Some(previous) = previous {
                        host.release_image_resource(self.client, &previous)?;
                    }
                }
            }
        }
        let input = self.queued_input;
        self.queued_input.buttons_pressed = 0;
        let t = &runtime.definition.setup;
        let now = host.now_ms();
        let period = vm.frame_period_ms().max(1);
        self.remainder = self
            .remainder
            .saturating_add(now.saturating_sub(self.last_ms));
        self.last_ms = now;
        let ticks = self.remainder / period;
        self.remainder %= period;
        self.ticks = self.ticks.saturating_add(ticks);
        self.horizontal = (i64::from(self.horizontal)
            - i64::from(n(t, &format!("scrollSpeedX{}", self.group + 1), -1)?)
                * (ticks % 800) as i64)
            .rem_euclid(800) as i32;
        let events: Vec<Value> = serde_json::from_slice(
            &host.query_engine(self.client, EngineQuery::UiSubtreeEvents(ROOT))?,
        )?;
        if self.confirmed.is_none() && input.buttons_pressed & buttons::ESCAPE != 0 {
            return Ok(Some(Event::System));
        }
        for event in events {
            let Some(id) = event["widget"]["id"]
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
            else {
                continue;
            };
            if event["kind"] != "activate" {
                continue;
            }
            if id == ROOT + 1 {
                return Ok(Some(Event::System));
            }
            if id == ROOT + 2 && self.confirmed.is_some() {
                let slot = self.confirmed.take().unwrap();
                self.selected = Some(slot);
                return Ok(Some(Event::Choice));
            }
            if id == ROOT + 3 && self.confirmed.is_some() {
                self.confirmed = None;
                self.dirty = true;
                continue;
            }
            if self.confirmed.is_some() {
                continue;
            }
            if id >= ROOT + 100 && id < ROOT + 100 + runtime.definition.maps.len() as u32 {
                let group = (id - ROOT - 100) as usize;
                if group != self.group && runtime.open(group, &mut variables)? {
                    self.group = group;
                    self.scroll = if group == runtime.definition.chart(self.current)? {
                        (runtime.definition.node(self.current)?.position[1] - 300)
                            .clamp(0, scroll_height(runtime, group)? - 600)
                    } else {
                        0
                    };
                    self.hovered = None;
                    self.drag = None;
                    if let Some(id) = runtime.definition.print_variable {
                        variables.write_native(id, group as i32)?;
                    }
                    vm.publish_general_variables(variables)?;
                    self.load(runtime, host)?;
                    return Ok(None);
                }
            }
        }
        if self.confirmed.is_none() {
            let a = self.area()?;
            let mini = self.mini()?;
            let map = &runtime.definition.maps[self.group];
            let scroll_height = scroll_height(runtime, self.group)?;
            let pointer = [input.pointer_x, input.pointer_y];
            self.pointer = pointer;
            let inside = |r: [i32; 4]| {
                pointer[0] >= r[0]
                    && pointer[1] >= r[1]
                    && pointer[0] < r[0] + r[2]
                    && pointer[1] < r[1] + r[3]
            };
            let w = size(t, "slotSizeX", 200)?;
            let h = size(t, "slotSizeY", 50)?;
            let old = self.hovered;
            self.hovered = None;
            self.description = None;
            for slot in &map.nodes {
                let p = self.point(runtime, *slot)?;
                let (direct, replay) = self.available(runtime, state, &mut variables, *slot)?;
                if inside([p[0] - w / 2, p[1] - h / 2, w, h])
                    && (direct || (n(t, "allClickOk", 1)? != 0 && replay))
                {
                    self.hovered = Some(*slot);
                }
                if inside([p[0] - w / 2, p[1] - h / 2, w, h])
                    && (runtime.seen(*slot)?
                        || direct && runtime.definition.chart(self.current)? == self.group)
                {
                    self.description = Some(*slot);
                }
            }
            if let Some(window) = self.assets.get("description") {
                let height = window.size[1] as i32;
                let lower = n(
                    t,
                    "lowerDescWindowPrintY",
                    600 - (300 - height) / 2 - height,
                )?;
                let upper = n(t, "upperDescWindowPrintY", (300 - height) / 2)?;
                if !self.description_up && pointer[1] >= lower - 64 {
                    self.description_up = true;
                } else if self.description_up && pointer[1] < upper + height + 64 {
                    self.description_up = false;
                }
            }
            if old != self.hovered {
                self.dirty = true;
                if self.hovered.is_some() {
                    let sound = n(t, "overSound", -1)?;
                    if sound > 0 {
                        self.sounds.push_back(sound);
                    }
                }
            }
            if input.buttons_pressed & buttons::WHEEL_UP != 0 {
                self.scroll -= n(t, "scrollSpeed", 20)?;
                self.dirty = true;
            }
            if input.buttons_pressed & buttons::WHEEL_DOWN != 0 {
                self.scroll += n(t, "scrollSpeed", 20)?;
                self.dirty = true;
            }
            if input.buttons_pressed & buttons::MOUSE_RIGHT != 0 && n(t, "dontRightMove", 0)? == 0 {
                for slot in &map.nodes {
                    let p = self.point(runtime, *slot)?;
                    if inside([p[0] - w / 2, p[1] - h / 2, w, h]) {
                        self.drag =
                            Some(Drag::Node(*slot, pointer, self.position(runtime, *slot)?));
                        break;
                    }
                }
            }
            if input.buttons_pressed & buttons::MOUSE_LEFT != 0 {
                if inside(mini) {
                    self.scroll = ((pointer[1] - mini[1]) * scroll_height / mini[3] - 300)
                        .clamp(0, scroll_height - 600);
                    self.drag = Some(Drag::Mini(pointer[1], self.scroll));
                    self.dirty = true;
                } else if let Some(slot) = self.hovered {
                    let (direct, _) = self.available(runtime, state, &mut variables, slot)?;
                    let sound = n(t, "selectSound", -1)?;
                    if sound > 0 {
                        self.sounds.push_back(sound);
                    }
                    if direct || n(t, "warningPrintFlag", 1)? == 0 {
                        self.selected = Some(slot);
                        return Ok(Some(Event::Choice));
                    } else {
                        self.confirmed = Some(slot);
                        self.dirty = true;
                    }
                } else if inside(a) && n(t, "mainCaptureOk", 1)? != 0 {
                    self.drag = Some(Drag::Main(pointer[1], self.scroll));
                }
            }
            if let Some(drag) = self.drag {
                match drag {
                    Drag::Main(start, scroll) if input.buttons_down & buttons::MOUSE_LEFT != 0 => {
                        let mut dy = pointer[1] - start;
                        if n(t, "mainCaptureOk", 1)? == 3 {
                            dy = dy * (scroll_height - 600) / 600;
                        }
                        if n(t, "captureReverse", 0)? != 0 {
                            dy = -dy;
                        }
                        self.scroll = scroll - dy;
                        self.dirty = true;
                    }
                    Drag::Mini(start, scroll) if input.buttons_down & buttons::MOUSE_LEFT != 0 => {
                        self.scroll = scroll + (pointer[1] - start) * scroll_height / mini[3];
                        self.dirty = true;
                    }
                    Drag::Node(slot, start, position)
                        if input.buttons_down & buttons::MOUSE_RIGHT != 0 =>
                    {
                        self.moved.insert(
                            slot,
                            [
                                (position[0] + pointer[0] - start[0])
                                    .clamp(w / 2, (a[2] - w / 2).max(w / 2)),
                                (position[1] + pointer[1] - start[1])
                                    .clamp(h / 2, scroll_height - h / 2),
                            ],
                        );
                        self.dirty = true;
                    }
                    _ => self.drag = None,
                }
            }
            self.scroll = self.scroll.clamp(0, scroll_height - 600);
        }
        if ticks > 0 {
            self.dirty = true;
        }
        if self.dirty && self.line_job.is_none() {
            let lines = self.lines(runtime, state, &mut variables)?;
            self.line_job = Some(host.image_metadata(self.client)?.begin_process(
                ImageInput::Solid {
                    size: SIZE,
                    color: [0; 4],
                },
                serde_json::to_vec(&vec![ImageEffect::IntegerLines { lines }])?.into(),
            )?);
            self.dirty = false;
        }
        Ok(None)
    }
    fn lines(
        &self,
        runtime: &ChartRuntime,
        state: &ChartState,
        variables: &mut Variables,
    ) -> Result<Vec<IntegerLine>> {
        let t = &runtime.definition.setup;
        let mut colors = [
            [128, 180, 255],
            [36, 99, 192],
            [62, 85, 101],
            [191, 217, 255],
            [72, 148, 255],
        ];
        for (i, color) in colors.iter_mut().enumerate() {
            for (j, c) in color.iter_mut().enumerate() {
                *c = n(t, &format!("lineColor{}{}", ["R", "G", "B"][j], i + 1), *c)?;
            }
        }
        let period = n(t, "lineColorCount", 20)?;
        let phase = pulse(self.ticks, period)?;
        let mut lines = vec![];
        for slot in &runtime.definition.maps[self.group].nodes {
            let node = runtime.definition.node(*slot)?;
            let from = self.point(runtime, *slot)?;
            for (road, target) in node.next.iter().enumerate() {
                if *target < 0 || runtime.definition.chart(*target)? != self.group {
                    continue;
                }
                let to = self.point(runtime, *target)?;
                if (from[1] < -100 && to[1] < -100) || (from[1] > 700 && to[1] > 700) {
                    continue;
                }
                let mut style = if road < 8 && runtime.road_traversed(*slot, road)? {
                    1
                } else {
                    2
                };
                if usize::from(state.road(*slot as usize)?) == road + 1 {
                    style = 0;
                }
                if *slot == self.current && runtime.selectable(state, *target, variables)? {
                    style = if self.hovered == Some(*target) { 4 } else { 3 };
                }
                let rgb = if style == 4 {
                    std::array::from_fn(|i| {
                        colors[3][i] + (colors[4][i] - colors[3][i]) * phase / (period / 2).max(1)
                    })
                } else {
                    colors[style]
                };
                for offset in [[0, 0], [1, 0], [0, 1], [-1, 0], [0, -1]]
                    .into_iter()
                    .take(if style == 0 || style == 4 { 5 } else { 3 })
                {
                    lines.push(IntegerLine {
                        from: [from[0] + offset[0], from[1] + offset[1]],
                        to: [to[0] + offset[0], to[1] + offset[1]],
                        color: [rgb[0] as u8, rgb[1] as u8, rgb[2] as u8, 255],
                    });
                }
            }
        }
        Ok(lines)
    }
    fn draw(
        &mut self,
        runtime: &ChartRuntime,
        state: &ChartState,
        variables: &mut Variables,
        host: &mut dyn HostServices,
        font: &str,
    ) -> Result<()> {
        let t = &runtime.definition.setup;
        let area = self.area()?;
        let map = &runtime.definition.maps[self.group];
        let scroll_height = scroll_height(runtime, self.group)?;
        let mut layers = vec![];
        let mut count = 0u32;
        for i in 0..self.layer_count {
            layers.push(json!({"op":"remove","id":LAYER+i}));
        }
        let mut sprite = |key: &str, rect: [i32; 4], crop: Option<[i32; 4]>, percent: i32| {
            if rect[2] <= 0 || rect[3] <= 0 {
                return;
            }
            let Some(asset) = self.assets.get(key) else {
                return;
            };
            let mode = if percent >= 100 {
                json!("mask_separate256")
            } else {
                json!({"mask_piecewise256":{"weight":percent.clamp(0,100)*256/100}})
            };
            layers.push(json!({"op":"put","layer":{"id":LAYER+count,"rect":rect,"clip":[0,0,800,600],"source_crop":crop,"sampling":{"linear":false},"order":[ORDER,count],"blend":{"stored_byte":{"mode":mode,"force_output_opaque":true}},"paint":{"kind":"image","asset":asset.path}}}));
            count += 1;
        };
        let mut y = -(i64::from((self.back_height - 600).max(0)) * i64::from(self.scroll)
            / i64::from(scroll_height - 600)) as i32;
        for index in 0..64 {
            let key = format!("back{index}");
            let Some(asset) = self.assets.get(&key) else {
                continue;
            };
            sprite(
                &key,
                [0, y, asset.size[0] as i32, asset.size[1] as i32],
                None,
                100,
            );
            y += asset.size[1] as i32;
        }
        if let Some(center) = self.assets.get("center") {
            let [w, h] = center.size.map(|v| v as i32);
            let dy = (-(i64::from((self.back_height - 600).max(0))
                * i64::from(n(t, "centerMulti", 200)?)
                * i64::from(self.scroll)
                / (i64::from(scroll_height - 600) * 100))
                % (h as i64)) as i32;
            ensure!(
                (800 / w + 3) * (600 / h + 3) <= 4096,
                "NNN chart center tile budget"
            );
            for ix in -1..=800 / w + 1 {
                for iy in -1..=600 / h + 1 {
                    sprite(
                        "center",
                        [ix * w - self.horizontal, iy * h + dy, w, h],
                        None,
                        100,
                    );
                }
            }
        }
        drop(sprite);
        if let Some(path) = &self.line_path {
            layers.push(json!({"op":"put","layer":{"id":LAYER+count,"rect":[0,0,800,600],"order":[ORDER,count],"paint":{"kind":"image","asset":path}}}));
            count += 1;
        }

        let w = size(t, "slotSizeX", 200)?;
        let h = size(t, "slotSizeY", 50)?;
        let cols = self.sheet[0] / w;
        let rows = self.sheet[1] / h;
        let cells = cols * rows;
        let period = n(t, "buttonMultiCount", 40)?;
        let multi = 100
            + (n(t, "buttonMulti", 140)? - 100) * pulse(self.ticks, period)? / (period / 2).max(1);
        let mut put = |key: &str, rect: [i32; 4], crop: Option<[i32; 4]>, percent: i32| {
            let Some(asset) = self.assets.get(key) else {
                return;
            };
            if rect[2] <= 0 || rect[3] <= 0 {
                return;
            }
            let mode = if percent >= 100 {
                json!("mask_separate256")
            } else {
                json!({"mask_piecewise256":{"weight":percent.clamp(0,100)*256/100}})
            };
            layers.push(json!({"op":"put","layer":{"id":LAYER+count,"rect":rect,"source_crop":crop,"sampling":{"linear":false},"order":[ORDER,count],"blend":{"stored_byte":{"mode":mode,"force_output_opaque":true}},"paint":{"kind":"image","asset":asset.path}}}));
            count += 1;
        };
        let mini = self.mini()?;
        if let Some(asset) = self.assets.get("mini_base") {
            put(
                "mini_base",
                [
                    n(
                        t,
                        &format!("miniMapBasePrintX{}", self.group + 1),
                        n(t, "miniMapBasePrintX", 10)?,
                    )?,
                    n(
                        t,
                        &format!("miniMapBasePrintY{}", self.group + 1),
                        n(t, "miniMapBasePrintY", 300)?,
                    )?,
                    asset.size[0] as i32,
                    asset.size[1] as i32,
                ],
                None,
                100,
            );
        }
        for slot in &map.nodes {
            let node = runtime.definition.node(*slot)?;
            let p = self.point(runtime, *slot)?;
            let (direct, replay) = self.available(runtime, state, variables, *slot)?;
            let seen = runtime.seen(*slot)?;
            let hover = self.hovered == Some(*slot);
            let scale = if hover {
                n(
                    t,
                    if direct {
                        "selectButtonMulti"
                    } else {
                        "selectButtonMulti2"
                    },
                    if direct { 140 } else { 120 },
                )?
            } else if direct {
                multi
            } else {
                100
            };
            let scaled = [w * scale / 100, h * scale / 100];
            let rect = [
                p[0] - scaled[0] / 2,
                p[1] - scaled[1] / 2,
                scaled[0],
                scaled[1],
            ];
            if p[1] + h >= 0 && p[1] - h < 600 {
                let local = *slot - self.group as i32 * runtime.definition.block_size as i32;
                let crop = [(local % cols) * w, ((local / cols) % rows) * h, w, h];
                let key = if !seen && !direct {
                    "unknown".to_owned()
                } else if seen && !direct && !replay {
                    format!("grey_slot{}", local / cells)
                } else {
                    format!("slot{}", local / cells)
                };
                let percent = if hover {
                    n(t, "selectSlotTransPercent", 80)?
                } else if direct {
                    n(t, "selectOkSlotTransPercent", 80)?
                } else {
                    100
                };
                put(
                    &key,
                    rect,
                    if key == "unknown" {
                        Some([0, 0, w, h])
                    } else {
                        Some(crop)
                    },
                    if *slot == self.current { 100 } else { percent },
                );
                if *slot == self.current {
                    put("cursor", rect, Some([0, 0, w, h]), 100);
                }
                if seen {
                    if let Some(comp) = self.assets.get("complete") {
                        let cw = n(t, "compSizeX", comp.size[0] as i32)?;
                        let ch = n(t, "compSizeY", comp.size[1] as i32)?;
                        for route in 0..usize::from(node.routes) {
                            let yes = runtime.route_seen(*slot, route)?;
                            let dx = n(t, "compNextX", cw)? * route as i32;
                            let dy = n(t, "compNextY", 0)? * route as i32;
                            put(
                                "complete",
                                [
                                    p[0] + dx + n(t, "compPrintX", 60)? - scaled[0] / 2,
                                    p[1] + dy + n(t, "compPrintY", 5)? - scaled[1] / 2,
                                    cw,
                                    ch,
                                ],
                                Some([0, 0, cw, ch]),
                                if yes { 100 } else { n(t, "compPercent", 20)? },
                            );
                        }
                    }
                }
            }
            let position = self.position(runtime, *slot)?;
            let mw = size(t, "miniMapButtonSizeX", 12)?;
            let mh = size(t, "miniMapButtonSizeY", 3)?;
            let md = if direct {
                3
            } else if *slot == self.current {
                2
            } else if runtime.complete(*slot)? {
                1
            } else {
                0
            };
            if md != 3 || (self.ticks / size(t, "miniButtonTime", 10)? as u64) % 2 != 0 {
                put(
                    "mini",
                    [
                        mini[0] + position[0] * mini[2] / area[2] - mw / 2,
                        mini[1] + position[1] * mini[3] / scroll_height - mh / 2,
                        mw,
                        mh,
                    ],
                    Some([md * mw, 0, mw, mh]),
                    100,
                );
            }
            let _ = replay;
        }
        if let Some(window) = self.assets.get("mini_window") {
            let up = n(t, "miniMapWindowUpSize", 15)?;
            let down = n(t, "miniMapWindowDownSize", 15)?;
            let width = n(t, "miniMapWindowSizeX", window.size[0] as i32)?;
            let center = window.size[1] as i32 - up - down;
            ensure!(
                up >= 0 && down >= 0 && center > 0,
                "NNN chart mini-window slices"
            );
            let x = mini[0]
                + n(
                    t,
                    "miniMapWindowPrintX",
                    (mini[2] - window.size[0] as i32) / 2,
                )?;
            let y =
                mini[1] + n(t, "miniMapWindowPrintY", -up)? + self.scroll * mini[3] / scroll_height;
            let height = mini[3] * 600 / scroll_height;
            let period = n(t, "miniWindowTime", 30)?;
            let percent = n(t, "miniWindowPercent1", 30)?
                + (n(t, "miniWindowPercent2", 90)? - n(t, "miniWindowPercent1", 30)?)
                    * pulse(self.ticks, period)?
                    / (period / 2).max(1);
            put(
                "mini_window",
                [x, y - up, width, up],
                Some([0, 0, width, up]),
                percent,
            );
            put(
                "mini_window",
                [x, y, width, height],
                Some([0, up, width, center]),
                percent,
            );
            put(
                "mini_window",
                [x, y + height, width, down],
                Some([0, up + center, width, down]),
                percent,
            );
        }
        if let Some(asset) = self.assets.get("percent_base") {
            put(
                "percent_base",
                [
                    n(t, "tasseidoBaseX", 0)?,
                    n(t, "tasseidoBaseY", 0)?,
                    asset.size[0] as i32,
                    asset.size[1] as i32,
                ],
                None,
                100,
            );
        }
        let dw = size(t, "chartFontSizeX", 11)?;
        let dh = size(t, "chartFontSizeY", 18)?;
        let step = n(t, "chartFontNextX", dw)?;
        for (x, y, number) in [
            (
                n(t, "totalTasseidoX", 80)?,
                n(t, "totalTasseidoY", 20)?,
                runtime.percent(-1)?,
            ),
            (
                n(t, "tasseidoX", 80)?,
                n(t, "tasseidoY", 34)?,
                runtime.percent(self.group as i32 + 1)?,
            ),
        ] {
            let digits = if n(t, "chartFontUpZeroFlag", 0)? != 0 {
                format!("{number:03}")
            } else {
                number.to_string()
            };
            let offset = if n(t, "chartFontLeftFlag", 0)? != 0 {
                0
            } else {
                3 - digits.len() as i32
            };
            for (i, ch) in digits.bytes().enumerate() {
                put(
                    "digits",
                    [x + (offset + i as i32) * step, y, dw, dh],
                    Some([i32::from(ch - b'0') * dw, 0, dw, dh]),
                    100,
                );
            }
        }
        if let Some(asset) = self.assets.get("now") {
            let group = runtime.definition.chart(self.current)? + 1;
            let state = if group - 1 == self.group { 2 } else { 1 };
            put(
                "now",
                [
                    n(t, &format!("grow{group}Button{state}PrintX"), 0)?,
                    n(t, &format!("grow{group}Button{state}PrintY"), 0)?,
                    asset.size[0] as i32,
                    asset.size[1] as i32,
                ],
                None,
                100,
            );
        }
        if matches!(self.drag, Some(Drag::Main(..))) {
            if let Some(asset) = self.assets.get("capture") {
                put(
                    "capture",
                    [
                        self.pointer[0] + n(t, "capturePrintX", 0)?,
                        self.pointer[1] + n(t, "capturePrintY", 0)?,
                        asset.size[0] as i32,
                        asset.size[1] as i32,
                    ],
                    None,
                    100,
                );
            }
        }
        let description_rect = self
            .assets
            .get("description")
            .map(|asset| -> Result<[i32; 4]> {
                let w = asset.size[0] as i32;
                let h = asset.size[1] as i32;
                Ok([
                    n(
                        t,
                        if self.description_up {
                            "upperDescWindowPrintX"
                        } else {
                            "lowerDescWindowPrintX"
                        },
                        (800 - w) / 2,
                    )?,
                    n(
                        t,
                        if self.description_up {
                            "upperDescWindowPrintY"
                        } else {
                            "lowerDescWindowPrintY"
                        },
                        if self.description_up {
                            (300 - h) / 2
                        } else {
                            600 - (300 - h) / 2 - h
                        },
                    )?,
                    w,
                    h,
                ])
            })
            .transpose()?;
        if self.description.is_some() {
            if let Some(rect) = description_rect {
                put("description", rect, None, 100);
            }
        }
        drop(put);
        self.layer_count = count;
        let mut ui = vec![
            json!({"op":"put","widget":{"id":ROOT,"kind":"panel","rect":[0,0,800,600],"modal":true,"color":[0,0,0,0],"z":1000,"font":font}}),
        ];
        let mut ids = vec![];
        let mut widget = |id: u32, kind: &str, rect: [i32; 4], text: String, enabled: bool| {
            ids.push(id);
            ui.push(json!({"op":"put","widget":{"id":id,"parent":ROOT,"kind":kind,"rect":rect,"text":text,"enabled":enabled,"font":font,"text_size":20}}));
        };
        widget(ROOT + 1, "button", [0, 0, 180, 30], "システム".into(), true);
        for group in 0..runtime.definition.maps.len() {
            if runtime.open(group, variables)? {
                widget(
                    ROOT + 100 + group as u32,
                    "button",
                    [0, 60 + group as i32 * 30, 180, 28],
                    format!("{}  {}%", group + 1, runtime.percent(group as i32 + 1)?),
                    true,
                );
            }
        }
        if let Some(slot) = self.description {
            let node = runtime.definition.node(slot)?;
            let unseen = usize::from(!runtime.seen(slot)? && !node.descriptions[1].is_empty());
            if let Some(rect) = description_rect {
                for (i, row) in node.descriptions[unseen].iter().enumerate() {
                    let text = encoding_rs::SHIFT_JIS
                        .decode_without_bom_handling(row)
                        .0
                        .into_owned();
                    let font_size = size(t, "descFontSize", 20)?;
                    let line = n(t, "descNextY", font_size + 8)?;
                    widget(
                        ROOT + 2000 + i as u32,
                        "label",
                        [
                            rect[0] + n(t, "descPrintX", 32)?,
                            rect[1] + n(t, "descPrintY", 16)? + line * i as i32,
                            rect[2],
                            line.max(font_size),
                        ],
                        text,
                        true,
                    );
                }
            }
        }
        if self.confirmed.is_some() {
            widget(
                ROOT + 5,
                "panel",
                [200, 200, 400, 160],
                "この場面から再開しますか？".into(),
                true,
            );
            widget(ROOT + 2, "button", [230, 300, 130, 40], "はい".into(), true);
            widget(
                ROOT + 3,
                "button",
                [430, 300, 130, 40],
                "いいえ".into(),
                true,
            );
        }
        for id in &self.ui_nodes {
            if !ids.contains(id) {
                ui.push(json!({"op":"remove","id":id}));
            }
        }
        for command in &mut ui {
            if command["widget"]["kind"] == "label" {
                command["widget"]["text_inset"] = json!([0, 0]);
                command["widget"]["color"] = json!([0, 0, 0, 0]);
                command["widget"]["text_size"] = json!(n(t, "descFontSize", 20)?);
                command["widget"]["text_color"] = json!([
                    n(t, "descColorR", 255)? as u8,
                    n(t, "descColorG", 255)? as u8,
                    n(t, "descColorB", 255)? as u8,
                    255
                ]);
            }
        }
        self.ui_nodes = ids;
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?,
        )?;
        for path in std::mem::take(&mut self.retired_paths) {
            host.release_image_resource(self.client, &path)?;
        }
        Ok(())
    }
    fn remove_layers(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let layers = (0..self.layer_count)
            .map(|i| json!({"op":"remove","id":LAYER+i}))
            .collect::<Vec<_>>();
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"layers":layers,"ui":[{"op":"remove","id":ROOT}]}))?,
        )?;
        self.layer_count = 0;
        self.ui_nodes.clear();
        Ok(())
    }
    pub fn hide(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.remove_layers(host)?;
        self.dirty = true;
        Ok(())
    }
    pub fn resume_clock(&mut self, now: u64) {
        self.last_ms = now;
    }
    pub fn cleanup(&self) -> (Vec<Value>, Vec<Value>, Vec<String>) {
        (
            (0..self.layer_count)
                .map(|i| json!({"op":"remove","id":LAYER+i}))
                .collect(),
            vec![json!({"op":"remove","id":ROOT})],
            self.assets
                .values()
                .map(|asset| asset.path.clone())
                .chain(self.line_path.iter().cloned())
                .chain(self.retired_paths.iter().cloned())
                .collect(),
        )
    }
    pub fn remove(&mut self, host: &mut dyn HostServices) -> Result<()> {
        self.task = None;
        self.loading = None;
        self.line_job = None;
        self.remove_layers(host)?;
        for path in std::mem::take(&mut self.retired_paths) {
            host.release_image_resource(self.client, &path)?;
        }
        for (_, asset) in std::mem::take(&mut self.assets) {
            host.release_image_resource(self.client, &asset.path)?;
        }
        if let Some(path) = self.line_path.take() {
            host.release_image_resource(self.client, &path)?;
        }
        Ok(())
    }
}
impl InstalledAdapter {
    pub(super) fn poll_chart(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
    ) -> Result<()> {
        let vm = self.vm.as_mut().context("NNN ChartMenu VM absent")?;
        let chart = self.chart.as_mut().unwrap();
        if let Some(number) = chart.sounds.front().copied() {
            if vm.poll_system_sound(host, number)? {
                chart.sounds.pop_front();
            }
            return Ok(());
        }
        if let Some(slot) = chart.selected {
            if !vm.end_menu_background(host)? {
                return Ok(());
            }
            if vm.poll_chart_return(slot, host)? {
                chart.remove(host)?;
                self.chart = None;
                vm.resume_after_menu(host)?;
            }
            return Ok(());
        }
        // Never start a background VM batch, sample input or advance native
        // graph time while merely draining an admitted resource dependency.
        let waiting = chart.dependency_pending() || vm.dependency_pending();
        if !admit || waiting {
            if admit {
                chart.queue_input(input);
            }
            if vm.dependency_pending() {
                vm.poll_dependency(host)?;
            }
            chart.poll_dependencies(vm, host, &self.config.font)?;
            chart.resume_clock(host.now_ms());
            return Ok(());
        }
        if !chart.prepare(host)? {
            return Ok(());
        }
        if !chart.background_started {
            if let Some(script) = chart.background.clone() {
                if !vm.start_menu_background(
                    script.name,
                    script.program,
                    script.looping,
                    34,
                    host,
                )? {
                    return Ok(());
                }
            }
            chart.background_started = true;
        }
        if vm.has_menu_background() {
            if !vm.switch_menu_background(true, host)? {
                return Ok(());
            }
            vm.tick_menu_background(host, input)?;
        }
        let event = chart.poll(vm, host, input, &self.config.font)?;
        match event {
            Some(Event::System) => {
                if !vm.end_menu_background(host)? {
                    return Ok(());
                }
                chart.background_started = false;
                self.open_menu(menu::Kind::System, host)?;
                self.chart.as_mut().unwrap().hide(host)?;
            }
            Some(Event::Choice) | None => (),
        }
        Ok(())
    }
}
