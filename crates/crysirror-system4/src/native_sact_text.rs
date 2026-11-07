// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. Pinned SACT2.c/sprite.c/text.c plus complete authorized
// SACT2 native Text wrappers and SactEngine vtable slots22..38 were read.
//! Native retained character-list policy over scoped Glyph/Image/Layer services.
//! There is no private font, glyph, text-image compositor or pixel plane here.
use crate::{
    adapter::{Bridge, Dispatch},
    sprites::Sprite,
    vm::{
        ExternalCall, ExternalTarget, Machine,
        external::{Argument, HllReturn},
        heap::PageKind,
    },
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::{
        GlyphImageBatchQuery, GlyphImageBatchResource, GlyphImageBatchSpec, GlyphImageBatchState,
        GlyphImageCell, GlyphImageOrigin, GlyphRasterOptions, GlyphSizeBasis, ImageResource,
    },
    service::{FontLibrary, Glyph, HostServices},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
const MAX_RECORDS: usize = 4096;
#[derive(Clone)]
struct Plane {
    id: u32,
    path: String,
    resource: Arc<dyn ImageResource>,
    color: [u8; 3],
}
#[derive(Clone)]
struct Record {
    position: [i32; 2],
    padding: [i32; 2],
    size: [u32; 2],
    planes: Vec<Plane>,
}
impl Record {
    fn puts(&self, number: i32, index: usize, brightness: u8) -> Vec<Value> {
        let parent = if number == -1 {
            crate::sact::MAIN_SPRITE
        } else {
            0x4100_0000 + number as u32
        };
        self.planes.iter().enumerate().map(|(n,p)|json!({"op":"put","layer":{"id":p.id,"parent":parent,
            "rect":[self.position[0]-self.padding[0],self.position[1]-self.padding[1],self.size[0],self.size[1]],
            "paint":{"kind":"image","asset":p.path,"tint":[p.color[0],p.color[1],p.color[2],255]},
            "order":[0,index*2+n],"filter":{"dark":1.0-f64::from(brightness)/255.0}}})).collect()
    }
}
struct Cell {
    position: [i32; 2],
    padding: [i32; 2],
    glyph: GlyphImageCell,
}
struct Ready {
    position: [i32; 2],
    padding: [i32; 2],
    size: [u32; 2],
    planes: Vec<(Arc<dyn ImageResource>, [u8; 3])>,
}
struct Draw {
    cells: Vec<Cell>,
    face: Box<dyn GlyphImageBatchQuery>,
    edge: Option<Box<dyn GlyphImageBatchQuery>>,
    face_ready: Option<Arc<dyn GlyphImageBatchResource>>,
    edge_ready: Option<Arc<dyn GlyphImageBatchResource>>,
    color: [u8; 3],
    edge_color: [u8; 3],
}
enum Work {
    Draw(Draw),
    Copy(Task<Vec<Ready>>),
    Scroll(Task<(Vec<Record>, Vec<Value>)>),
    Ready(Vec<Ready>),
}
struct Pending {
    call: ExternalCall,
    number: i32,
    sprite: Sprite,
    replace: bool,
    work: Work,
    staged: Vec<Record>,
    cursor: usize,
    next_plane: usize,
    current: Option<Record>,
}
pub(crate) struct NativeText {
    owner: ClientId,
    pool: TaskPool,
    next: u32,
    records: BTreeMap<i32, Arc<Vec<Record>>>,
    pending: Option<Pending>,
    retired: Vec<Plane>,
}
impl NativeText {
    pub(crate) fn new(owner: ClientId, pool: TaskPool) -> Self {
        Self {
            owner,
            pool,
            next: 0,
            records: BTreeMap::new(),
            pending: None,
            retired: vec![],
        }
    }
    pub(crate) fn has_state(&self) -> bool {
        self.pending.is_some() || !self.records.is_empty()
    }
    pub(crate) fn published_paths(&self) -> Vec<String> {
        self.records
            .values()
            .flat_map(|r| r.iter())
            .flat_map(|r| r.planes.iter().map(|p| p.path.clone()))
            .collect()
    }
    pub(crate) fn close(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(pending) = self.pending.as_mut() {
            for record in &mut pending.staged {
                release_planes(self.owner, &mut record.planes, host)?;
            }
            if let Some(record) = pending.current.as_mut() {
                release_planes(self.owner, &mut record.planes, host)?;
            }
        }
        self.pending = None;
        self.drain_retired(host)?;
        Ok(())
    }
    pub(crate) fn forget_deleted(
        &mut self,
        number: i32,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        if let Some(records) = self.records.remove(&number) {
            self.retired
                .extend(records.iter().flat_map(|r| r.planes.iter()).cloned());
        }
        self.drain_retired(host)
    }
    ///Complete native052A0's existing-record clear for SetCG/FromFile.
    ///Create69B0 is deliberately excluded: it replaces only the base and
    ///preserves character records. Even a later failed CG lookup keeps this
    ///clear, exactly as the native6630/66D0 ordering specifies.
    pub(crate) fn clear_for_cg(
        &mut self,
        number: i32,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "native CG clear while text work pending"
        );
        let sprites = bridge
            .sact
            .as_mut()
            .filter(|s| s.initialized)
            .and_then(|s| s.sprites.as_mut())
            .context("native CG clear before sprite Init")?;
        sprites.require_owner(self.owner)?;
        let Some(mut sprite) = sprites.get(number).cloned() else {
            return Ok(());
        };
        sprite.surface = None;
        sprite.size = [0, 0];
        sprite.alpha = false;
        let mut operations = vec![sprite.layer(number)?.context("native CG clear parent")?];
        if let Some(records) = self.records.get(&number) {
            operations.extend(
                records
                    .iter()
                    .flat_map(|r| r.planes.iter())
                    .map(|p| json!({"op":"remove","id":p.id})),
            );
        }
        submit(self.owner, operations, host)?;
        sprites.install_prepared(number, sprite);
        self.forget_deleted(number, host)
    }
    fn drain_retired(&mut self, host: &mut dyn HostServices) -> Result<()> {
        release_planes(self.owner, &mut self.retired, host)
    }
    pub(crate) fn diagnostics(&self) -> Value {
        json!({"pending":self.pending.is_some(),"sprites":self.records.len(),
        "glyphs":self.records.values().map(|v|v.len()).sum::<usize>(),"raster_profile":"mapped outline face with explicit3x coverage; native GDI identity not claimed"})
    }
    pub(crate) fn dispatch(
        &mut self,
        owner: ClientId,
        m: &mut Machine,
        call: &ExternalCall,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<Option<Dispatch>> {
        let ExternalTarget::Hll { library, function } = call.target else {
            ensure!(self.pending.is_none(), "native SACT text target changed");
            return Ok(None);
        };
        let program = m.program.clone();
        let library = &program.libraries[library];
        let f = &library.functions[function];
        let known = library.name == b"SACT2"
            && matches!(
                f.name.as_slice(),
                b"SP_TextDraw"
                    | b"SP_TextClear"
                    | b"SP_TextCopy"
                    | b"SP_TextScroll"
                    | b"SP_TextBackSpace"
                    | b"SP_TextHome"
                    | b"SP_TextNewLine"
                    | b"SP_SetTextPos"
                    | b"SP_SetBrightness"
            );
        if !known {
            ensure!(self.pending.is_none(), "native SACT text HLL changed");
            return Ok(None);
        }
        ensure!(owner == self.owner, "native SACT text foreign owner");
        self.drain_retired(host)?;
        ensure!(
            f.return_kind == if f.name == b"SP_TextScroll" { 47 } else { 10 },
            "native SACT text return ABI"
        );
        let args = m.hll_arguments(call)?;
        let aliases = bridge
            .sact
            .as_ref()
            .context("native SACT text before Init")?
            .font_aliases
            .clone();
        let sprites = bridge
            .sact
            .as_mut()
            .filter(|s| s.initialized)
            .and_then(|s| s.sprites.as_mut())
            .context("native SACT text before Init")?;
        sprites.require_owner(owner)?;
        if self.pending.is_some() {
            return self.poll(m, call, sprites, host).map(Some);
        }
        let number = match args.first() {
            Some(Argument::Scalar { kind: 10, bits }) => *bits,
            _ => anyhow::bail!("native SACT text sprite ABI"),
        };
        ensure!(
            (-1..65536).contains(&number),
            "native SACT text sprite budget"
        );
        let original = sprites.get(number).cloned();
        let mut sprite = original.clone().unwrap_or_default();
        let records = self.records.get(&number).cloned().unwrap_or_default();
        let scalars = args
            .iter()
            .map(|a| match a {
                Argument::Scalar { kind: 10, bits } => Some(*bits),
                _ => None,
            })
            .collect::<Option<Vec<_>>>();
        let mut operations = vec![];
        let result = match (f.name.as_slice(), args.as_slice(), scalars.as_deref()) {
            (
                b"SP_TextDraw",
                [
                    Argument::Scalar { .. },
                    Argument::String { bytes, .. },
                    Argument::Page { kind: 13, slot },
                ],
                _,
            ) => {
                let style = Style::read(m, *slot)?;
                if style.size == 0 {
                    0
                } else if bytes.first().is_none_or(|b| *b == 0) || original.is_none() {
                    // Original wrapper succeeds for an empty string or absent
                    // target without asking SactEngine to allocate a sprite.
                    m.complete_hll(call, HllReturn::Scalar(1))?;
                    return Ok(Some(Dispatch::Completed));
                } else {
                    let fonts = host.font_library(owner)?;
                    let images = host.image_metadata(owner)?;
                    let (draw, cursor, line) = prepare_draw(
                        bytes,
                        style,
                        &sprite,
                        &aliases,
                        fonts.as_ref(),
                        images.as_ref(),
                    )?;
                    ensure!(
                        records.len() + draw.cells.len() <= MAX_RECORDS,
                        "native SACT retained text count budget"
                    );
                    sprite.text_position = cursor;
                    sprite.text_line_height = line;
                    self.pending = Some(Pending {
                        call: call.clone(),
                        number,
                        sprite,
                        replace: false,
                        work: Work::Draw(draw),
                        staged: vec![],
                        cursor: 0,
                        next_plane: 0,
                        current: None,
                    });
                    return Ok(Some(wait(host)));
                }
            }
            (b"SP_TextCopy", _, Some([_, source])) => {
                if original.is_none() {
                    sprite = Sprite::default();
                }
                let Some(source) = self.records.get(source).cloned() else {
                    if sprites.get(*source).is_none() {
                        let parent = sprite.layer(number)?.context("native SACT copy parent")?;
                        host.submit_engine(
                            owner,
                            1,
                            &serde_json::to_vec(&json!({"layers":[parent]}))?,
                        )?;
                        sprites.install_prepared(number, sprite);
                        m.complete_hll(call, HllReturn::Scalar(0))?;
                        return Ok(Some(Dispatch::Completed));
                    }
                    return self
                        .clear(number, sprite, records, m, call, sprites, host)
                        .map(Some);
                };
                // Original100074B0 clones glyph records, preserving each glyph's
                // relative position and style, but NOT home/cursor/spacing.
                let permit = self.pool.reserve()?;
                let task = permit.spawn(move |cancel| {
                    let mut output = Vec::with_capacity(source.len());
                    for r in source.iter() {
                        cancel.check()?;
                        output.push(Ready {
                            position: r.position,
                            padding: r.padding,
                            size: r.size,
                            planes: r
                                .planes
                                .iter()
                                .map(|p| (p.resource.clone(), p.color))
                                .collect(),
                        });
                    }
                    Ok(output)
                })?;
                self.pending = Some(Pending {
                    call: call.clone(),
                    number,
                    sprite,
                    replace: true,
                    work: Work::Copy(task),
                    staged: vec![],
                    cursor: 0,
                    next_plane: 0,
                    current: None,
                });
                return Ok(Some(wait(host)));
            }
            (b"SP_TextClear", _, Some([_])) => {
                if number < 1 {
                    0
                } else if original.is_none() {
                    m.complete_hll(call, HllReturn::Scalar(1))?;
                    return Ok(Some(Dispatch::Completed));
                } else {
                    return self
                        .clear(number, sprite, records, m, call, sprites, host)
                        .map(Some);
                }
            }
            (b"SP_TextBackSpace", _, Some([_])) => {
                let Some(last) = records.last() else {
                    m.complete_hll(call, HllReturn::Scalar(0))?;
                    return Ok(Some(Dispatch::Completed));
                };
                sprite.text_position = last.position;
                operations.extend(last.planes.iter().map(|p| json!({"op":"remove","id":p.id})));
                operations.insert(
                    0,
                    sprite
                        .layer(number)?
                        .context("native SACT backspace parent")?,
                );
                host.submit_engine(
                    owner,
                    1,
                    &serde_json::to_vec(&json!({"layers":operations}))?,
                )?;
                self.retired.extend(last.planes.iter().cloned());
                let mut next = records.as_ref().clone();
                next.pop();
                self.records.insert(number, Arc::new(next));
                sprites.install_prepared(number, sprite);
                self.drain_retired(host)?;
                m.complete_hll(call, HllReturn::Scalar(1))?;
                return Ok(Some(Dispatch::Completed));
            }
            (b"SP_TextScroll", _, Some([_, x, y])) => {
                if original.is_none() {
                    0
                } else {
                    let (x, y) = (*x, *y);
                    let count = records.len();
                    let brightness = sprite.brightness;
                    let permit = self.pool.reserve()?;
                    let task = permit.spawn(move |cancel| {
                        let mut next = Vec::with_capacity(count);
                        let mut ops = vec![];
                        for (index, r) in records.iter().enumerate() {
                            cancel.check()?;
                            let mut r = r.clone();
                            r.position = [
                                r.position[0]
                                    .checked_add(x)
                                    .context("native text scroll X overflow")?,
                                r.position[1]
                                    .checked_add(y)
                                    .context("native text scroll Y overflow")?,
                            ];
                            ensure!(
                                r.position.iter().all(|v| v.unsigned_abs() <= 1_000_000),
                                "native text scroll position budget"
                            );
                            ops.extend(r.puts(number, index, brightness));
                            next.push(r);
                        }
                        Ok((next, ops))
                    })?;
                    self.pending = Some(Pending {
                        call: call.clone(),
                        number,
                        sprite,
                        replace: false,
                        work: Work::Scroll(task),
                        staged: vec![],
                        cursor: 0,
                        next_plane: 0,
                        current: None,
                    });
                    return Ok(Some(wait(host)));
                }
            }
            (b"SP_TextHome", _, Some([_, size])) => {
                if number < 1 {
                    0
                } else {
                    sprite.text_position = sprite.text_home;
                    sprite.text_line_height = *size;
                    1
                }
            }
            (b"SP_TextNewLine", _, Some([_, size])) => {
                if number < 1 {
                    0
                } else {
                    // Actual10007AE0 uses max(current,requested), then retains the
                    // requested height. The public zero-after-newline shortcut is
                    // not equivalent for subsequent mixed-size native text.
                    sprite.text_position = [
                        sprite.text_home[0],
                        sprite.text_position[1]
                            .checked_add(sprite.text_line_height.max(*size))
                            .and_then(|v| v.checked_add(sprite.text_line_space))
                            .context("native text newline overflow")?,
                    ];
                    sprite.text_line_height = *size;
                    1
                }
            }
            (b"SP_SetTextPos", _, Some([_, x, y])) => {
                sprite.text_position = [*x, *y];
                1
            }
            (b"SP_SetBrightness", _, Some([_, value])) => {
                if original.is_none() {
                    0
                } else {
                    sprite.brightness = (*value).clamp(0, 255) as u8;
                    for (index, record) in records.iter().enumerate() {
                        operations.extend(record.puts(number, index, sprite.brightness));
                    }
                    1
                }
            }
            _ => anyhow::bail!(
                "native SACT text argument ABI {}",
                String::from_utf8_lossy(&f.name)
            ),
        };
        if result != 0 {
            operations.insert(0, sprite.layer(number)?.context("native SACT text parent")?);
            submit(owner, operations, host)?;
            sprites.install_prepared(number, sprite);
        }
        m.complete_hll(call, HllReturn::Scalar(result))?;
        Ok(Some(Dispatch::Completed))
    }
    fn clear(
        &mut self,
        number: i32,
        sprite: Sprite,
        records: Arc<Vec<Record>>,
        m: &mut Machine,
        call: &ExternalCall,
        sprites: &mut crate::sprites::Sprites,
        host: &mut dyn HostServices,
    ) -> Result<Dispatch> {
        let mut operations = vec![sprite.layer(number)?.context("native SACT clear parent")?];
        operations.extend(
            records
                .iter()
                .flat_map(|r| r.planes.iter())
                .map(|p| json!({"op":"remove","id":p.id})),
        );
        submit(self.owner, operations, host)?;
        self.forget_deleted(number, host)?;
        sprites.install_prepared(number, sprite);
        m.complete_hll(call, HllReturn::Scalar(1))?;
        Ok(Dispatch::Completed)
    }
    fn poll(
        &mut self,
        m: &mut Machine,
        call: &ExternalCall,
        sprites: &mut crate::sprites::Sprites,
        host: &mut dyn HostServices,
    ) -> Result<Dispatch> {
        let p = self.pending.as_mut().unwrap();
        ensure!(p.call == *call, "native SACT stale text receipt");
        if let Work::Scroll(task) = &mut p.work {
            let Some((records, operations)) = task.try_take()? else {
                return Ok(wait(host));
            };
            submit(self.owner, operations, host)?;
            self.records.insert(p.number, Arc::new(records));
            self.pending = None;
            m.complete_hll(call, HllReturn::Scalar(1))?;
            return Ok(Dispatch::Completed);
        }
        if let Work::Copy(task) = &mut p.work {
            let Some(ready) = task.try_take()? else {
                return Ok(wait(host));
            };
            p.work = Work::Ready(ready);
        }
        if let Work::Draw(draw) = &mut p.work {
            if draw.face_ready.is_none() {
                draw.face_ready = poll_batch(self.owner, draw.face.as_mut())?;
            }
            if let Some(query) = draw.edge.as_mut() {
                if draw.edge_ready.is_none() {
                    draw.edge_ready = poll_batch(self.owner, query.as_mut())?;
                }
            }
            if draw.face_ready.is_none() || draw.edge.is_some() && draw.edge_ready.is_none() {
                return Ok(wait(host));
            }
            let foreground = draw.face_ready.as_ref().unwrap();
            ensure!(
                foreground.images().len() == draw.cells.len(),
                "native SACT glyph count"
            );
            if let Some(edge) = &draw.edge_ready {
                ensure!(
                    edge.images().len() == draw.cells.len(),
                    "native SACT outline count"
                );
            }
            let mut ready = Vec::with_capacity(draw.cells.len());
            for (index, cell) in draw.cells.iter().enumerate() {
                let mut planes = vec![];
                if let Some(edge) = &draw.edge_ready {
                    planes.push((edge.images()[index].clone(), draw.edge_color));
                }
                planes.push((foreground.images()[index].clone(), draw.color));
                ready.push(Ready {
                    position: cell.position,
                    padding: cell.padding,
                    size: cell.glyph.canvas,
                    planes,
                });
            }
            p.work = Work::Ready(ready);
        }
        let Work::Ready(ready) = &p.work else {
            anyhow::bail!("native SACT text phase")
        };
        for _ in 0..64 {
            let Some(cell) = ready.get(p.cursor) else {
                break;
            };
            let record = p.current.get_or_insert_with(|| Record {
                position: cell.position,
                padding: cell.padding,
                size: cell.size,
                planes: vec![],
            });
            if p.next_plane < cell.planes.len() {
                let (resource, color) = &cell.planes[p.next_plane];
                ensure!(
                    resource.client() == self.owner && resource.size() == cell.size,
                    "native SACT foreign glyph resource"
                );
                ensure!(self.next < 0x0100_0000, "native SACT glyph identity budget");
                let id = 0x5400_0000 + self.next;
                self.next += 1;
                let published = host.publish_image_resource(self.owner, id, resource.clone())?;
                record.planes.push(Plane {
                    id,
                    path: published.path.relative().to_owned(),
                    resource: resource.clone(),
                    color: *color,
                });
                p.next_plane += 1;
            }
            if p.next_plane == cell.planes.len() {
                p.staged.push(p.current.take().unwrap());
                p.cursor += 1;
                p.next_plane = 0;
            }
        }
        if p.cursor != ready.len() {
            return Ok(wait(host));
        }
        let previous = self.records.get(&p.number).cloned().unwrap_or_default();
        let mut operations = vec![
            p.sprite
                .layer(p.number)?
                .context("native SACT glyph parent")?,
        ];
        if p.replace {
            operations.extend(
                previous
                    .iter()
                    .flat_map(|r| r.planes.iter())
                    .map(|p| json!({"op":"remove","id":p.id})),
            );
        }
        let base = if p.replace { 0 } else { previous.len() };
        for (index, record) in p.staged.iter().enumerate() {
            operations.extend(record.puts(p.number, base + index, p.sprite.brightness));
        }
        submit(self.owner, operations, host)?;
        let mut next = if p.replace {
            vec![]
        } else {
            previous.as_ref().clone()
        };
        next.append(&mut p.staged);
        let number = p.number;
        let sprite = p.sprite.clone();
        let replace = p.replace;
        self.records.insert(number, Arc::new(next));
        sprites.install_prepared(number, sprite);
        self.pending = None;
        if replace {
            self.retired
                .extend(previous.iter().flat_map(|r| r.planes.iter()).cloned());
        }
        self.drain_retired(host)?;
        m.complete_hll(call, HllReturn::Scalar(1))?;
        Ok(Dispatch::Completed)
    }
}
fn release_planes(
    owner: ClientId,
    planes: &mut Vec<Plane>,
    host: &mut dyn HostServices,
) -> Result<()> {
    while let Some(plane) = planes.last() {
        host.release_image_resource(owner, &plane.path)?;
        planes.pop();
    }
    Ok(())
}
fn submit(owner: ClientId, operations: Vec<Value>, host: &mut dyn HostServices) -> Result<()> {
    if operations.is_empty() {
        return Ok(());
    }
    let bytes = serde_json::to_vec(&json!({"layers":operations}))?;
    ensure!(
        bytes.len() <= 1024 * 1024,
        "native SACT text atomic batch budget"
    );
    host.submit_engine(owner, 1, &bytes)?;
    Ok(())
}
fn wait(host: &dyn HostServices) -> Dispatch {
    Dispatch::WaitUntil(host.now_ms().saturating_add(1))
}
fn poll_batch(
    owner: ClientId,
    query: &mut dyn GlyphImageBatchQuery,
) -> Result<Option<Arc<dyn GlyphImageBatchResource>>> {
    ensure!(query.client() == owner, "native SACT foreign glyph query");
    Ok(match query.poll()? {
        GlyphImageBatchState::Pending => None,
        GlyphImageBatchState::Ready(resource) => {
            ensure!(
                resource.client() == owner,
                "native SACT foreign glyph batch"
            );
            Some(resource)
        }
        GlyphImageBatchState::Failed(error) => anyhow::bail!("native SACT glyph worker: {error}"),
        GlyphImageBatchState::Cancelled => anyhow::bail!("native SACT glyph worker cancelled"),
    })
}
struct Style {
    color: [u8; 3],
    size: u16,
    weight: u16,
    face: u16,
    padding: [u16; 4],
    edge_color: [u8; 3],
}
impl Style {
    fn read(m: &Machine, slot: i32) -> Result<Self> {
        let page = m.heap.page(slot)?;
        let PageKind::Structure(index) = page.kind else {
            anyhow::bail!("native SACT metrics structure")
        };
        let members = &m.program.structures[index].members;
        ensure!(
            page.values.len() >= 13
                && members.len() >= 13
                && members[..13].iter().all(|v| v.kind == 10),
            "native SACT metrics13-integer ABI"
        );
        let values = &page.values;
        Ok(Self {
            color: [values[0] as u8, values[1] as u8, values[2] as u8],
            size: values[3] as u16,
            weight: values[4] as u16,
            face: values[5] as u16,
            padding: [
                values[6] as u16,
                values[7] as u16,
                values[8] as u16,
                values[9] as u16,
            ],
            edge_color: [values[10] as u8, values[11] as u8, values[12] as u8],
        })
    }
}
fn prepare_draw(
    bytes: &[u8],
    style: Style,
    sprite: &Sprite,
    aliases: &[String; 2],
    fonts: &dyn FontLibrary,
    images: &dyn crysirror_core::service::ImageMetadata,
) -> Result<(Draw, [i32; 2], i32)> {
    ensure!(
        (1..=170).contains(&style.size),
        "native SACT mapped3x font size profile"
    );
    ensure!(
        style.weight <= 1000,
        "native SACT heavy-font geometry profile not mapped"
    );
    let font = fonts
        .resolve_alias(&aliases[usize::from(style.face == 1)])?
        .context("native SACT font not published")?;
    let face = fonts
        .face_style(&font)?
        .context("native SACT font lacks actual weight metadata")?;
    ensure!(!face.italic, "native SACT regular face profile");
    let weight = if style.weight <= 550 || style.weight == 1000 {
        400
    } else {
        700
    };
    ensure!(
        weight == i32::from(face.weight) || (weight >= 600 && (350..=550).contains(&face.weight)),
        "native SACT mapped face weight profile"
    );
    let bold = if weight > i32::from(face.weight) {
        ((f32::from(style.size) * 3. * 0.025).ceil() as u8).min(16)
    } else {
        0
    };
    let edge = style.padding.iter().copied().max().unwrap_or(0);
    ensure!(
        edge <= 5 && u16::from(bold) + edge * 3 <= 16,
        "native SACT outline worker radius profile"
    );
    let raster = GlyphRasterOptions {
        size_basis: GlyphSizeBasis::Em,
        origin: GlyphImageOrigin::Top,
        supersample: 3,
        binary_cutoff: Some(128),
        coverage_lut: Some(Arc::from([0, 28, 56, 113, 141, 169, 198, 226, 254, 255])),
        embolden_radius: bold,
        ..Default::default()
    };
    let raw = bytes.split(|b| *b == 0).next().unwrap();
    ensure!(raw.len() <= 4096, "native SACT text bytes");
    let mut at = 0;
    let mut cursor = sprite.text_position;
    let mut cells = vec![];
    while at < raw.len() {
        ensure!(cells.len() < 1024, "native SACT glyph call budget");
        let lead = raw[at];
        let count = if (0x81..=0x9f).contains(&lead) || (0xe0..=0xfc).contains(&lead) {
            2
        } else {
            1
        };
        let unit = raw
            .get(at..at + count)
            .context("native SACT truncated CP932 character")?;
        let text = crysirror_resource::text_stream::decode_shift_jis(unit, 8)?;
        let mut chars = text.chars();
        let character = chars.next().context("native SACT empty character")?;
        ensure!(
            chars.next().is_none() && !character.is_control(),
            "native SACT character profile"
        );
        let width = if count == 2 {
            i32::from(style.size)
        } else {
            i32::from(style.size) / 2
        };
        ensure!(width > 0, "native SACT half-cell extent");
        let padding = [i32::from(style.padding[0]), i32::from(style.padding[1])];
        let size = [
            width as u32 + u32::from(style.padding[0]) + u32::from(style.padding[2]),
            u32::from(style.size) + u32::from(style.padding[1]) + u32::from(style.padding[3]),
        ];
        cells.push(Cell {
            position: cursor,
            padding,
            glyph: GlyphImageCell {
                canvas: size,
                glyphs: vec![Glyph {
                    glyph_id: fonts.glyph_id(&font, character)?,
                    x_26_6: padding[0] * 64,
                    y_26_6: padding[1] * 64,
                    advance_26_6: width * 64,
                }],
            },
        });
        cursor[0] = cursor[0]
            .checked_add(width)
            .and_then(|v| v.checked_add(sprite.text_char_space))
            .context("native SACT cursor overflow")?;
        ensure!(
            cursor.iter().all(|v| v.unsigned_abs() <= 1_000_000),
            "native SACT cursor position budget"
        );
        at += count;
    }
    let spec = GlyphImageBatchSpec {
        font,
        size: style.size,
        cells: cells.iter().map(|c| c.glyph.clone()).collect(),
        raster: raster.clone(),
    };
    let face = images.begin_glyph_image_batch(spec.clone())?;
    let outline = if edge > 0 {
        let mut spec = spec;
        spec.raster.embolden_radius = bold + edge as u8 * 3;
        Some(images.begin_glyph_image_batch(spec)?)
    } else {
        None
    };
    Ok((
        Draw {
            cells,
            face,
            edge: outline,
            face_ready: None,
            edge_ready: None,
            color: style.color,
            edge_color: style.edge_color,
        },
        cursor,
        sprite.text_line_height.max(i32::from(style.size)),
    ))
}
