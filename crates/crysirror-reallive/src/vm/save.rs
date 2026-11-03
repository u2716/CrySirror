// SPDX-License-Identifier: MPL-2.0
// Adopted from siglus_rs 256c398 crates/reallive/src/save.rs (MPL-2.0).
//! Saved games: the global file (global memory, read-text flags, settings)
//! and the slot queries. Files are bytes of the client's capability save
//! mount, passed in by the adapter (`Store`); the VM never opens a path.
//! Writing slots restores engine-owned presentation, which needs an engine
//! scene snapshot; slot writes stay with the adapter.

use anyhow::{Result, anyhow, bail};

use crate::vm::bytecode::Command;
use crate::vm::expr::Expr;
use crate::vm::machine::{Frame, FrameKind, Machine};
use crate::vm::memory::{LocalMemory, Memory};
use crate::vm::serial::{Reader, Writer, fill};

/// The client's save mount as the save module sees it.
pub trait Store {
    fn read(&mut self, name: &str) -> Option<Vec<u8>>;
    fn write(&mut self, name: &str, bytes: &[u8]) -> Result<()>;
}

pub fn slot_name(slot: i32) -> String {
    format!("save{slot:03}.sav")
}

pub const GLOBAL_NAME: &str = "global.sav";

const SLOT_MAGIC: &[u8; 8] = b"RLRSSAV1";
const GLOBAL_MAGIC: &[u8; 8] = b"RLRSGLB1";

/// Date and title of a save.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SaveHeader {
    pub title: String,
    /// y, m, d, weekday, hh, mm, ss, ms.
    pub date: [i32; 8],
    pub scene: i32,
    pub line: i32,
}


#[allow(dead_code)]
fn write_local(w: &mut Writer, local: &LocalMemory) {
    for bank in &local.ints {
        w.i32s(bank);
    }
    w.strs(&local.str_s);
    w.strs(&local.local_names);
}

fn read_local(r: &mut Reader, local: &mut LocalMemory) -> Result<()> {
    for bank in &mut local.ints {
        fill(bank, &r.i32s()?);
    }
    fill(&mut local.str_s, &r.strs()?);
    fill(&mut local.local_names, &r.strs()?);
    Ok(())
}

#[allow(dead_code)]
fn write_stack(w: &mut Writer, stack: &[Frame]) {
    w.len(stack.len());
    for frame in stack {
        w.i32(frame.scenario.number);
        w.u32(frame.ip as u32);
        w.u8(match frame.kind {
            FrameKind::Root => 0,
            FrameKind::Gosub => 1,
            FrameKind::Farcall => 2,
        });
        w.i32s(&frame.vars.int_l);
        w.strs(&frame.vars.str_k);
    }
}

#[allow(dead_code)]
fn read_stack(r: &mut Reader, machine: &Machine) -> Result<Vec<Frame>> {
    let count = r.len()?;
    let mut stack = Vec::with_capacity(count);
    for _ in 0..count {
        let scene = r.i32()?;
        let ip = r.u32()? as usize;
        let kind = match r.u8()? {
            1 => FrameKind::Gosub,
            2 => FrameKind::Farcall,
            _ => FrameKind::Root,
        };
        let scenario = machine.archive.scenario(scene)?;
        let mut frame = Frame::new(scenario, ip, kind);
        fill(&mut frame.vars.int_l, &r.i32s()?);
        frame.vars.str_k = r.strs()?;
        stack.push(frame);
    }
    if stack.is_empty() {
        bail!("reallive: the save has an empty call stack");
    }
    Ok(stack)
}


/// What RealLiveMax keeps with a save besides the game: three comments,
/// the text on screen when it was made and some script values
/// (`SET_SAVE_COMMENT*`, `GET_SAVE_MESSAGE`, `GET_SAVE_VALUE`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SlotExtra {
    pub comments: [String; 3],
    pub message: String,
    pub values: Vec<i32>,
}

impl SlotExtra {
    #[allow(dead_code)]
    fn write(&self, w: &mut Writer) {
        for comment in &self.comments {
            w.str(comment);
        }
        w.str(&self.message);
        w.i32s(&self.values);
    }

    fn read(r: &mut Reader) -> Result<Self> {
        Ok(Self {
            comments: [r.str()?, r.str()?, r.str()?],
            message: r.str()?,
            values: r.i32s()?,
        })
    }
}


fn slot_sections(bytes: &[u8]) -> Result<Vec<(String, &[u8])>> {
    let body = bytes
        .strip_prefix(SLOT_MAGIC.as_slice())
        .ok_or_else(|| anyhow!("reallive: not a save file"))?;
    Reader::new(body).sections()
}

pub fn read_header(bytes: &[u8]) -> Result<SaveHeader> {
    for (name, body) in slot_sections(bytes)? {
        if name == "header" {
            let mut r = Reader::new(body);
            let title = r.str()?;
            let mut date = [0; 8];
            for value in &mut date {
                *value = r.i32()?;
            }
            return Ok(SaveHeader {
                title,
                date,
                scene: r.i32()?,
                line: r.i32()?,
            });
        }
    }
    bail!("reallive: the save has no header")
}


fn slot_bytes(machine: &Machine, store: &mut dyn Store, slot: i32) -> Option<Vec<u8>> {
    if let Some(bytes) = machine.saved_in_memory.get(&slot) {
        return Some(bytes.clone());
    }
    store.read(&slot_name(slot))
}

pub fn slot_exists(machine: &Machine, store: &mut dyn Store, slot: i32) -> bool {
    slot_bytes(machine, store, slot).is_some()
}

pub fn any_slot_header(machine: &Machine, store: &mut dyn Store, slot: i32) -> Option<SaveHeader> {
    read_header(&slot_bytes(machine, store, slot)?).ok()
}

pub fn slot_extra(machine: &Machine, store: &mut dyn Store, slot: i32) -> Option<SlotExtra> {
    let bytes = slot_bytes(machine, store, slot)?;
    let sections = slot_sections(&bytes).ok()?;
    match sections.into_iter().find(|(name, _)| name == "extra") {
        Some((_, body)) => SlotExtra::read(&mut Reader::new(body)).ok(),
        None => Some(SlotExtra::default()),
    }
}

/// The global file (original `save_global`).
pub fn save_global(machine: &Machine, store: &mut dyn Store) -> Result<()> {
    let mut w = Writer::new();
    w.bytes.extend_from_slice(GLOBAL_MAGIC);
    let global = &machine.memory.global;
    w.section("memory", |w| {
        w.i32s(&global.int_g);
        w.i32s(&global.int_z);
        w.strs(&global.str_m);
        w.strs(&global.global_names);
    });
    w.section("kidoku", |w| {
        w.len(global.kidoku.len());
        let mut scenes: Vec<_> = global.kidoku.iter().collect();
        scenes.sort_by_key(|(scene, _)| **scene);
        for (scene, bits) in scenes {
            w.i32(*scene);
            w.len(bits.len());
            for word in bits {
                w.u64(*word);
            }
        }
    });
    w.section("misc", |w| w.i32(machine.latest_save));
    w.section("settings", |w| crate::vm::settings_io::write(w, &machine.sys));
    store.write(GLOBAL_NAME, &w.bytes)
}

/// Serializes global state for change detection without writing it.
pub fn global_fingerprint(machine: &Machine) -> Vec<u8> {
    struct Capture(Vec<u8>);
    impl Store for Capture {
        fn read(&mut self, _: &str) -> Option<Vec<u8>> {
            None
        }
        fn write(&mut self, _: &str, bytes: &[u8]) -> Result<()> {
            self.0 = bytes.to_vec();
            Ok(())
        }
    }
    let mut capture = Capture(Vec::new());
    let _ = save_global(machine, &mut capture);
    capture.0
}

/// Original `load_global`: absent files keep the Gameexe defaults.
pub fn load_global(machine: &mut Machine, store: &mut dyn Store) -> Result<()> {
    let Some(bytes) = store.read(GLOBAL_NAME) else {
        return Ok(());
    };
    let body = bytes
        .strip_prefix(GLOBAL_MAGIC.as_slice())
        .ok_or_else(|| anyhow!("reallive: not a global save file"))?;
    for (name, body) in Reader::new(body).sections()? {
        let mut r = Reader::new(body);
        let global = &mut machine.memory.global;
        match name.as_str() {
            "memory" => {
                fill(&mut global.int_g, &r.i32s()?);
                fill(&mut global.int_z, &r.i32s()?);
                fill(&mut global.str_m, &r.strs()?);
                fill(&mut global.global_names, &r.strs()?);
            }
            "kidoku" => {
                let count = r.len()?;
                for _ in 0..count {
                    let scene = r.i32()?;
                    let words = r.len()?;
                    let bits = (0..words).map(|_| r.u64()).collect::<Result<Vec<_>>>()?;
                    global.kidoku.insert(scene, bits);
                }
            }
            "misc" => machine.latest_save = r.i32()?,
            "settings" => crate::vm::settings_io::read(&mut r, &mut machine.sys)?,
            _ => {}
        }
    }
    Ok(())
}

fn write_ints(machine: &mut Machine, command: &Command, from: usize, values: &[i32]) -> Result<()> {
    for (offset, value) in values.iter().enumerate() {
        if from + offset >= command.params.len() {
            break;
        }
        let target = machine.int_target_param(command, from + offset)?;
        machine.set_target(target, *value)?;
    }
    Ok(())
}

/// `SaveExists`, `SaveDate`, `SaveTime`, `SaveDateTime`, `SaveInfo`,
/// `GetSaveFlag`, `LatestSave`.
pub fn sys_query(machine: &mut Machine, store: &mut dyn Store, command: &Command) -> Result<()> {
    let opcode = command.op.opcode;
    if opcode == 1421 {
        machine.store = machine.latest_save;
        return Ok(());
    }
    let slot = machine.int_param(command, 0)?;
    if opcode == 1409 {
        machine.store = i32::from(slot_exists(machine, store, slot));
        return Ok(());
    }
    if opcode == 1414 {
        let Some(local) = slot_bytes(machine, store, slot).and_then(|bytes| {
            let sections = slot_sections(&bytes).ok()?;
            let (_, body) = sections.into_iter().find(|(name, _)| name == "memory")?;
            let mut local = LocalMemory::default();
            read_local(&mut Reader::new(body), &mut local).ok()?;
            Some(local)
        }) else {
            machine.store = 0;
            return Ok(());
        };
        let saved = Memory {
            local,
            ..Memory::default()
        };
        for param in command.params.iter().skip(1) {
            let Expr::Special { tag, pieces } = &param.value else {
                continue;
            };
            let (Some(src), Some(dst), Some(count)) =
                (pieces.first(), pieces.get(1), pieces.get(2))
            else {
                continue;
            };
            let count = machine.eval_int(count)?.max(0);
            if *tag == 1 {
                let src = machine.str_target(src)?;
                let dst = machine.str_target(dst)?;
                for i in 0..count {
                    let frame = crate::vm::memory::FrameMemory::default();
                    let value = saved
                        .string(src.bank, src.index + i, &frame)
                        .unwrap_or("")
                        .to_owned();
                    machine.write_string(
                        crate::vm::machine::StrTarget {
                            bank: dst.bank,
                            index: dst.index + i,
                        },
                        value,
                    )?;
                }
            } else {
                let src = machine.int_target(src)?;
                let dst = machine.int_target(dst)?;
                let sources = Machine::int_range(src, count as usize);
                let dests = Machine::int_range(dst, count as usize);
                for (source, dest) in sources.into_iter().zip(dests) {
                    let value = match source {
                        crate::vm::machine::IntTarget::Mem(reference) if reference.bank <= 5 => {
                            let frame = crate::vm::memory::FrameMemory::default();
                            saved.int(reference, &frame).unwrap_or(0)
                        }
                        other => machine.get_target(other)?,
                    };
                    machine.set_target(dest, value)?;
                }
            }
        }
        machine.store = 1;
        return Ok(());
    }
    let header = slot_bytes(machine, store, slot).and_then(|bytes| read_header(&bytes).ok());
    let Some(header) = header else {
        machine.store = 0;
        return Ok(());
    };
    match opcode {
        1410 => write_ints(machine, command, 1, &header.date[..4])?,
        1411 => write_ints(machine, command, 1, &header.date[4..])?,
        1412 => write_ints(machine, command, 1, &header.date)?,
        _ => {
            write_ints(machine, command, 1, &header.date)?;
            if command.params.len() > 9 {
                let target = machine.str_target_param(command, 9)?;
                machine.write_string(target, header.title.clone())?;
            }
        }
    }
    machine.store = 1;
    Ok(())
}

