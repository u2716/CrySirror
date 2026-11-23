// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SAVEPANEL (class 12, built on IMAGE): `setsave` opens a slot file; `getsavestr` and
//! `getsavedate` read it; `ptype SIMAGE` shows its thumbnail record where the panel's own
//! bitmap keeps that part's frame. Files and decoding stay on workers (76DDA0 → 762420).
use crate::{
    fes_program::PendingOperation,
    fes_references::{self, IndexedName},
    native_class_state::State,
    native_image_publication::{Draw, ImageList},
    native_image_resource::{Outcome, Pending},
    native_interpreter::Interpreter,
    native_save::Container,
    native_values::StringVariables,
    task_policy::NativeTaskPolicy,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};
use crysirror_tasks::{Task, TaskPool};
use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};

pub(crate) fn supports(method: &[u8]) -> bool {
    matches!(method, b"make" | b"get" | b"setnum" | b"setsave" | b"getflag" | b"getsavestr" | b"getsavedate" | b"ptype" | b"dispid")
}

/// `ptype SIMAGE part key width height`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Thumbnail {
    part: u32,
    key: u32,
    size: [u32; 2],
}

enum Step {
    Read(Task<Option<(Container, Option<[u32; 6]>)>>),
    Prepare,
    Decode(Pending),
    Place(Pending, Arc<dyn crysirror_core::image_processing::ImageResource>),
}

#[derive(Default)]
pub(crate) struct Panel {
    thumbnail: Option<Thumbnail>,
    pub(crate) hidden: BTreeSet<u32>,
    /// 75C5F0 initializes the native visible-part vector empty; only dispid
    /// inserts/removes its sorted member IDs.
    pub(crate) shown: BTreeSet<u32>,
    pub(crate) saved: Option<Container>,
    pub(crate) date: [u32; 6],
    pub(crate) parts: BTreeMap<u32, crate::fes_panel_parts::Part>,
    /// 762620 materializes configured records into the separate PANEL
    /// renderer map. Later ptype changes wait for the next rebuild.
    pub(crate) live_parts: BTreeMap<u32, crate::fes_panel_parts::Part>,
    pub(crate) revision: u64,
    /// 762350 only registers part descriptions. 762620 first materializes
    /// their native renderer values/text allocations during setsave.
    pub(crate) rebuilt: bool,
    /// 762420 → 728A60: partial DWORD writes into the panel's independent
    /// flag bank. A missing slot/record keeps its earlier flag values.
    pub(crate) flags: BTreeMap<u32, u32>,
    step: Option<(u32, Step)>,
    published: Option<u32>,
}

/// Panel state lives beside the object bank, keyed by object id.
#[derive(Default)]
pub(crate) struct Panels {
    pub(crate) panels: BTreeMap<u32, Panel>,
    files: crate::fes_file_list::Lists,
    request: Option<SaveRequest>,
}
struct SaveRequest { owner: u32, pc: u32, op: PendingOperation, id: u32, slot: u32, key: u32, start: u32, count: u32 }
impl Panels {
    pub(crate) fn reap(&mut self, tasks: &NativeTaskPolicy) {
        self.panels.retain(|id, _| tasks.fes(id.saturating_sub(1) / 4096).is_ok());
        self.files.reap(tasks);
        if self.request.as_ref().is_some_and(|request| tasks.fes(request.owner).is_err()) { self.request = None; }
    }
}

fn targets(tasks: &mut NativeTaskPolicy, owner: u32, target: &[u8], vm: &mut Interpreter, strings: &StringVariables) -> Result<Vec<u32>> {
    let bank = tasks.fes_object_bank_mut(owner)?.context("CatSystem2 SAVEPANEL has no native object bank")?;
    let reference = fes_references::evaluate(target, vm, strings, &|name| bank.count(name))?;
    let Some(count) = bank.count(&reference.name) else { return Ok(Vec::new()) };
    let range = if reference.index == -1 { 0..count as i32 } else { reference.index..reference.index + 1 };
    Ok(range.filter_map(|index| bank.lookup(&IndexedName { name: reference.name.clone(), index })).collect())
}

/// 75B200: `a` or `a-b`.
fn id_range(text: &[u8], vm: &mut Interpreter) -> Result<(u32, u32)> {
    let text = if text.starts_with(b"[") && text.ends_with(b"]") { &text[1..text.len() - 1] } else { text };
    match text.iter().position(|b| *b == b'-').filter(|at| *at > 0) {
        Some(at) => Ok((vm.evaluate(&text[..at])?, vm.evaluate(&text[at + 1..])?)),
        None => {
            let value = vm.evaluate(text)?;
            Ok((value, value))
        }
    }
}

/// Answers false while a file or image is still on its way.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply(
    host: &mut dyn HostServices,
    client: ClientId,
    pool: &TaskPool,
    owner: u32,
    native_pc: u32,
    op: &PendingOperation,
    panels: &mut Panels,
    tasks: &mut NativeTaskPolicy,
    vm: &mut Interpreter,
    strings: &mut StringVariables,
) -> Result<bool> {
    panels.reap(tasks);
    if let Some(request) = &panels.request {
        ensure!(request.owner == owner && request.pc == native_pc && request.op == *op, "CatSystem2 SAVEPANEL pending operation changed");
        let done = advance(host, client, pool, owner, native_pc, panels.panels.entry(request.id).or_default(), request.slot, request.key, request.start, request.count, request.id, tasks)?;
        if done { panels.request = None; }
        return Ok(done);
    }
    let method = op.arguments.first().context("CatSystem2 SAVEPANEL method")?.clone();
    let args = &op.arguments[1..];
    if matches!(method.as_slice(), b"make" | b"get") {
        let bank = tasks.fes_object_bank_mut(owner)?.context("CatSystem2 FILELIST object bank")?;
        return panels.files.apply(host.storage(client)?, pool, owner, native_pc, bank, &op.name, &op.arguments, vm, strings);
    }
    let ids = targets(tasks, owner, &op.name, vm, strings)?;
    if matches!(method.as_slice(), b"getflag" | b"setsave") {
        if ids.len() > 1 { return Ok(true); }
        if let Some(id) = ids.first() {
            let bank = tasks.fes_object_bank_mut(owner)?.context("CatSystem2 SAVEPANEL object bank")?;
            if bank.objects.get(id).is_none_or(|object| object.declared_type != 12) { return Ok(true); }
        }
    }
    match method.as_slice() {
        // 76D3E0 → 762350: other part kinds draw frames of the panel bitmap, which IMAGE does.
        b"ptype" => {
            let bank = tasks.fes_object_bank_mut(owner)?.context("CatSystem2 panel ptype object bank")?;
            let (part_id, part) = crate::fes_panel_parts::parse(args, vm, strings, &|name| bank.count(name))?;
            for id in ids {
                if bank.objects.get(&id).is_none_or(|object| object.declared_type != 12) { continue; }
                let panel = panels.panels.entry(id).or_default();
                ensure!(panel.parts.len() < 4096 || panel.parts.contains_key(&part_id), "CatSystem2 panel part allocation budget");
                if let crate::fes_panel_parts::Kind::SavedImage { key, size: Some(size), .. } = part.kind {
                    panel.thumbnail = Some(Thumbnail { part: part_id, key, size });
                }
                panel.parts.insert(part_id, part.clone());
                panel.revision = panel.revision.wrapping_add(1);
            }
        }
        // 76DC90: only PANEL/SAVEPANEL; no record means no mutation.
        b"setnum" => {
            if args.len() < 2 { return Ok(true); }
            let part_id = vm.evaluate(&args[0])?;
            let value = vm.evaluate(&args[1])?;
            let bank = tasks.fes_object_bank_mut(owner)?.context("CatSystem2 panel setnum object bank")?;
            for id in ids {
                let Some(kind) = bank.objects.get(&id).map(|object| object.declared_type).filter(|kind| matches!(*kind, 11 | 12)) else { continue };
                let Some(panel) = panels.panels.get_mut(&id) else { continue };
                if kind == 12 {
                    // 762590 stores record[29] before trying the live renderer;
                    // this survives the next rebuild for a literal source.
                    let Some(configured) = panel.parts.get_mut(&part_id) else { continue };
                    configured.value = value;
                }
                if let Some(part) = panel.live_parts.get_mut(&part_id) {
                    part.value = value;
                    part.number_override = Some(value);
                    panel.revision = panel.revision.wrapping_add(1);
                }
            }
        }
        // 76D240 → 75C870: parts by id; an absent word means shown.
        b"dispid" => {
            let shown = match args.first() {
                Some(value) => vm.evaluate(value)? != 0,
                None => true,
            };
            for text in args.iter().skip(1) {
                let (low, high) = id_range(text, vm)?;
                ensure!(high.saturating_sub(low) < 4096, "CatSystem2 SAVEPANEL part range");
                for id in &ids {
                    let panel = panels.panels.entry(*id).or_default();
                    for part in low..=high {
                        if shown {
                            panel.hidden.remove(&part);
                            panel.shown.insert(part);
                        } else {
                            panel.hidden.insert(part);
                            panel.shown.remove(&part);
                        }
                    }
                    panel.revision = panel.revision.wrapping_add(1);
                }
            }
        }
        // 76DDA0: an index is required; the file opens as save%04d.dat.
        b"setsave" => {
            ensure!(args.len() >= 3, "CatSystem2 setsave slot/record/flag-base arguments");
            let Some(id) = ids.first().copied() else { return Ok(true) };
            let slot = vm.evaluate(&args[0])?;
            let key = vm.evaluate(&args[1])?;
            let start = vm.evaluate(&args[2])?;
            panels.request = Some(SaveRequest { owner, pc: native_pc, op: op.clone(), id, slot, key, start, count: vm.flag_count() });
            return apply(host, client, pool, owner, native_pc, op, panels, tasks, vm, strings);
        }
        // 76F720 → 762C40 → 728970: source flag index, destination flag index.
        // Other object classes leave the destination untouched.
        b"getflag" => {
            if args.len() < 2 { return Ok(true); }
            let source = vm.evaluate(&args[0])?;
            let dest = vm.evaluate(&args[1])?;
            let Some(id) = ids.first() else { return Ok(true) };
            let bank = tasks.fes_object_bank_mut(owner)?.context("CatSystem2 getflag object bank")?;
            if bank.objects.get(id).is_some_and(|object| object.declared_type == 12) {
                let value = panels.panels.get(id).and_then(|panel| panel.flags.get(&source)).copied().unwrap_or(0);
                vm.set_flag(dest, value);
            }
        }
        // 7791B0 → 762C60: the record's text, or empty, into a string variable.
        b"getsavestr" => {
            ensure!(args.len() >= 2, "CatSystem2 getsavestr arguments");
            let (dest, key) = (vm.evaluate(&args[0])?, vm.evaluate(&args[1])?);
            let Some(id) = ids.first() else { return Ok(true) };
            let record = panels.panels.get(id).and_then(|panel| panel.saved.as_ref()).and_then(|saved| saved.first(key));
            let text = record.map_or(Vec::new(), |bytes| bytes.split(|b| *b == 0).next().unwrap_or(&[]).to_vec());
            strings.assign(dest, text)?;
        }
        // 7798B0 → 762C90.
        b"getsavedate" => {
            let Some(id) = ids.first() else { return Ok(true) };
            let date = panels.panels.get(id).map_or([0; 6], |panel| panel.date);
            for (source, value) in args.iter().zip(date) {
                let dest = vm.evaluate(source)?;
                vm.set_flag(dest, value);
            }
        }
        other => anyhow::bail!("CatSystem2 SAVEPANEL method {} is not implemented", String::from_utf8_lossy(other)),
    }
    Ok(true)
}

/// Read the slot, then its thumbnail, then the part's frame for placement.
#[allow(clippy::too_many_arguments)]
fn advance(host: &mut dyn HostServices, client: ClientId, pool: &TaskPool, owner: u32, native_pc: u32, panel: &mut Panel, slot: u32, key: u32, start: u32, flag_count: u32, id: u32, tasks: &mut NativeTaskPolicy) -> Result<bool> {
    if !panel.step.as_ref().is_some_and(|(s, _)| *s == slot) {
        let storage = host.storage(client)?;
        let permit = match pool.reserve() {
            Ok(permit) => permit,
            Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
            Err(e) => return Err(e),
        };
        let task = permit.spawn(move |cancel| {
            let name = crate::fes_save::slot_name(slot);
            let Some(saved) = crate::native_save::load(storage.as_ref(), client, &name, cancel)? else { return Ok(None) };
            Ok(Some((saved, crate::fes_save::date(storage.as_ref(), slot)?)))
        })?;
        panel.step = Some((slot, Step::Read(task)));
        return Ok(false);
    }
    let (_, step) = panel.step.as_mut().unwrap();
    match step {
        Step::Read(task) => {
            let Some(found) = task.try_take()? else { return Ok(false) };
            let (saved, date) = found.map_or((None, None), |(saved, date)| (Some(saved), date));
            panel.date = date.unwrap_or([0; 6]);
            panel.saved = saved;
            panel.rebuilt = true;
            panel.live_parts = panel.parts.clone();
            for part in panel.live_parts.values_mut() { part.number_override = None; }
            panel.revision = panel.revision.wrapping_add(1);
            if let Some(bytes) = panel.saved.as_ref().and_then(|saved| saved.first(key)) {
                for (offset, word) in bytes.chunks_exact(4).take(flag_count.saturating_sub(start) as usize).enumerate() {
                    panel.flags.insert(start + offset as u32, u32::from_le_bytes(word.try_into().unwrap()));
                }
            }
            panel.step = Some((slot, Step::Prepare));
            return advance(host, client, pool, owner, native_pc, panel, slot, key, start, flag_count, id, tasks);
        }
        Step::Prepare => {
            let thumbnail = panel.thumbnail.filter(|thumbnail| panel.shown.contains(&thumbnail.part));
            let bytes = thumbnail.and_then(|thumbnail| panel.saved.as_ref()?.first(thumbnail.key).map(<[u8]>::to_vec));
            match bytes {
                Some(bytes) => {
                    let Some(pending) = Pending::begin_bytes(pool, host, client, owner, native_pc, "save thumbnail", Arc::from(bytes), 0)? else { return Ok(false) };
                    panel.step = Some((slot, Step::Decode(pending)));
                    Ok(false)
                }
                None => {
                    panel.step = None;
                    remove_draw(host, client, owner, panel, tasks)?;
                    Ok(true)
                }
            }
        }
        Step::Decode(pending) => {
            let Some(outcome) = pending.poll()? else { return Ok(false) };
            let Outcome::Ready(frame) = outcome else {
                panel.step = None;
                remove_draw(host, client, owner, panel, tasks)?;
                return Ok(true);
            };
            let bank = tasks.fes_object_bank_mut(owner)?.context("CatSystem2 SAVEPANEL bank")?;
            let State::Image(image) = &bank.objects.get(&id).context("CatSystem2 SAVEPANEL object")?.state else {
                panel.step = None;
                return Ok(true);
            };
            let name = String::from_utf8_lossy(crate::native_scene::stem(&image.filename)).into_owned();
            let part = panel.thumbnail.map_or(0, |thumbnail| thumbnail.part);
            let Some(place) = Pending::begin_named(pool, host, client, owner, native_pc, &name, part)? else { return Ok(false) };
            panel.step = Some((slot, Step::Place(place, frame.image)));
            Ok(false)
        }
        Step::Place(place, thumbnail) => {
            let Some(outcome) = place.poll()? else { return Ok(false) };
            let thumbnail = thumbnail.clone();
            // The part's own frame marks where the thumbnail belongs on the panel canvas.
            let at = match outcome {
                Outcome::Ready(frame) => frame.geometry.origin,
                _ => [0; 2],
            };
            panel.step = None;
            publish_draw(host, client, owner, id, panel, tasks, thumbnail, at)?;
            Ok(true)
        }
    }
}

fn list_of(tasks: &NativeTaskPolicy, owner: u32) -> Result<u32> {
    tasks.fes(owner)?.image_list.context("CatSystem2 SAVEPANEL has no image list")
}

#[allow(clippy::too_many_arguments)]
fn publish_draw(host: &mut dyn HostServices, client: ClientId, owner: u32, id: u32, panel: &mut Panel, tasks: &mut NativeTaskPolicy, thumbnail: Arc<dyn crysirror_core::image_processing::ImageResource>, at: [i32; 2]) -> Result<()> {
    let bank = tasks.fes_object_bank_mut(owner)?.context("CatSystem2 SAVEPANEL bank")?;
    let State::Image(image) = &bank.objects.get(&id).context("CatSystem2 SAVEPANEL object")?.state else { return Ok(()) };
    if image.draw_id < 0 {
        return Ok(());
    }
    let pose = image.pose.clone();
    // One step in front of the panel bitmap, on its own draw number.
    let draw = (image.draw_id as u32).wrapping_add(0x1000);
    let key = ImageList::key(owner, draw)?;
    let upload = host.publish_image_resource(client, 0x4c80_0000 | (id & 0x7f_ffff), thumbnail)?;
    let list = list_of(tasks, owner)?;
    let publication = tasks.list_publication(list, client)?;
    publication.put(Draw {
        key,
        image: upload.path.relative().to_owned(),
        size: upload.size,
        position: [pose.position[0] + pose.offset[0] - pose.pivot[0] + at[0] as f32, pose.position[1] + pose.offset[1] - pose.pivot[1] + at[1] as f32],
        pivot: [0.; 2],
        alpha: u8::try_from(pose.alpha).unwrap_or(255),
        blend: crysirror_scene::layer::Blend::Normal,
        visible: pose.visible != 0,
        clip: None,
    })?;
    publication.publish(host, client, Vec::new())?;
    panel.published = Some(key);
    Ok(())
}

fn remove_draw(host: &mut dyn HostServices, client: ClientId, owner: u32, panel: &mut Panel, tasks: &mut NativeTaskPolicy) -> Result<()> {
    if let Some(key) = panel.published.take() {
        let list = list_of(tasks, owner)?;
        let publication = tasks.list_publication(list, client)?;
        publication.remove(key);
        publication.publish(host, client, Vec::new())?;
    }
    Ok(())
}
