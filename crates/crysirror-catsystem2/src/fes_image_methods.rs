// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! FES IMAGE-family object methods: frame ID maps (settbl), the owning PLANE (pl), pose
//! queries (getbase, getoffset) and `.anm` frame animation (setanm, anm).
use crate::{
    fes_object_bank::Bank,
    native_class_state::State,
    native_interpreter::Interpreter,
    native_values::StringVariables,
};
use anyhow::{ensure, Context, Result};
use crysirror_core::service::ClientStorage;
use std::collections::BTreeMap;

pub(crate) fn supports(method: &[u8]) -> bool {
    matches!(method, b"settbl" | b"pl" | b"getbase" | b"getoffset" | b"anm" | b"clip" | b"tone" | b"filter" | b"coordmode" | b"blur" | b"keypos")
}

/// One `.anm` file: {frame, frames to hold} rows behind a 32-byte header (8931EF checks the
/// magic, +8 holds the row count; rows are 68 bytes).
pub(crate) fn parse_anm(bytes: &[u8]) -> Result<Vec<(u32, u32)>> {
    ensure!(bytes.len() >= 32 && &bytes[..4] == b"ANM\0", "CatSystem2 anm header");
    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    ensure!(bytes.len() >= 32 + count * 68, "CatSystem2 anm extent");
    Ok((0..count)
        .map(|index| {
            let row = &bytes[32 + index * 68..32 + (index + 1) * 68];
            let word = |at: usize| u32::from_le_bytes(row[at * 4..at * 4 + 4].try_into().unwrap());
            (word(2), word(4).max(1))
        })
        .collect())
}

/// 74A100 / 74A160 per object.
#[derive(Clone, Debug, Default)]
struct Animation {
    rows: Vec<(u32, u32)>,
    running: bool,
}

#[derive(Default)]
pub(crate) struct Animations {
    objects: BTreeMap<u32, Animation>,
    load: Option<(u32, crysirror_tasks::Task<Option<Vec<u8>>>)>,
}

/// `name.anm` beside the bitmap: 770E10 expands variables and defaults the extension.
fn read_anm(storage: &dyn ClientStorage, name: &str) -> Result<Option<Vec<u8>>> {
    let catalog = crate::native_resource_paths::Catalog::load(storage)?;
    for path in catalog.candidates(storage, "image", &format!("{name}.anm"), &|| Ok(()))? {
        if let Some(path) = crate::native_resource_paths::resolve(storage, &path)? {
            return Ok(Some(crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 16 * 1024 * 1024)?));
        }
    }
    Ok(None)
}

impl Animations {
    /// 770E10: answers false while the file is on a worker. A missing one animates nothing.
    #[allow(clippy::too_many_arguments)]
    pub fn set(&mut self, storage: std::sync::Arc<dyn ClientStorage>, pool: &crysirror_tasks::TaskPool, pc: u32, bank: &mut Bank, target: &[u8], name: &[u8], vm: &mut Interpreter, strings: &StringVariables) -> Result<bool> {
        if self.load.as_ref().is_none_or(|(at, _)| *at != pc) {
            let text = String::from_utf8_lossy(&strings.expand(name, 2560)?).into_owned();
            let permit = match pool.reserve() {
                Ok(permit) => permit,
                Err(e) if e.is::<crysirror_tasks::CapacityExceeded>() => return Ok(false),
                Err(e) => return Err(e),
            };
            self.load = Some((pc, permit.spawn(move |_| read_anm(storage.as_ref(), &text))?));
            return Ok(false);
        }
        let Some(bytes) = self.load.as_mut().unwrap().1.try_take()? else { return Ok(false) };
        self.load = None;
        let rows = bytes.map_or(Ok(Vec::new()), |bytes| parse_anm(&bytes))?;
        for id in bank.targets(target, vm, strings)? {
            self.objects.entry(id).or_default().rows = rows.clone();
        }
        Ok(true)
    }
    /// 770F30: any word but STOP runs it.
    pub fn run(&mut self, bank: &mut Bank, target: &[u8], word: Option<&[u8]>, vm: &mut Interpreter, strings: &StringVariables) -> Result<()> {
        let running = word.is_none_or(|word| !word.eq_ignore_ascii_case(b"STOP"));
        for id in bank.targets(target, vm, strings)? {
            self.objects.entry(id).or_default().running = running;
        }
        Ok(())
    }
    /// The frame at this 60 Hz clock, for every running animation.
    pub fn tick(&self, bank: &mut Bank, clock: u32) {
        for (id, animation) in &self.objects {
            if !animation.running || animation.rows.is_empty() {
                continue;
            }
            let span: u32 = animation.rows.iter().map(|row| row.1).sum();
            let mut at = clock % span.max(1);
            let frame = animation.rows.iter().find(|row| {
                if at < row.1 {
                    true
                } else {
                    at -= row.1;
                    false
                }
            });
            if let (Some((frame, _)), Some(object)) = (frame, bank.objects.get_mut(id)) {
                if let State::Image(image) = &mut object.state {
                    image.select_frame(*frame);
                }
            }
        }
    }
}

pub(crate) fn apply(bank: &mut Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut Interpreter, strings: &StringVariables) -> Result<()> {
    let method = arguments.first().context("CatSystem2 image method")?.clone();
    let args = &arguments[1..];
    let word = |vm: &mut Interpreter, at: usize| args.get(at).map_or(Ok(0), |source| vm.evaluate(source));
    match method.as_slice() {
        b"keypos" => {
            let values=[word(vm,0)?,word(vm,1)?,word(vm,2)?];
            for id in bank.targets(target,vm,strings)? {
                if let Some(State::Image(image)|State::CImage(image))=bank.objects.get_mut(&id).map(|o|&mut o.state) {
                    image.key_position_mode=values[0];
                    if values[0]!=0 {image.key_position=[values[1] as i32,values[2] as i32];}
                    else {
                        let step=|size:i32|if size<=16{size/2}else if size<=32{size-8}else{size-16};
                        for axis in 0..2{image.key_position[axis]=image.key_position[axis].wrapping_add(step(image.size[axis]));}
                    }
                }
            }
        }
        // 76EC00/7703E0/7775D0/770810: parse once, then apply to IMAGE bases.
        b"clip" | b"tone" | b"filter" | b"coordmode" | b"blur" => {
            let mut values = [0u32;4];
            if method == b"tone" { values[..3].fill(100); }
            if method == b"blur" {values[..2].fill(1); }
            if method == b"filter" {
                let name = strings.expand(args.first().map(Vec::as_slice).unwrap_or_default(),255)?;
                values[0] = if name.eq_ignore_ascii_case(b"mono") {1} else if name.eq_ignore_ascii_case(b"sepia") {2} else if name.eq_ignore_ascii_case(b"np") {3} else {0};
            } else {
                let count = if method == b"clip" {4} else if method == b"tone" {3} else if method==b"blur"{2}else{1};
                for (at,value) in values.iter_mut().enumerate().take(count) {
                    if let Some(source) = args.get(at) {*value=vm.evaluate(source)?;}
                }
            }
            let reference=crate::fes_references::evaluate(target,vm,strings,&|name|bank.count(name))?;
            if method==b"clip" && reference.index == -1 { return Ok(()); }
            for id in bank.targets(target,vm,strings)? {
                if let Some(State::Image(image)|State::CImage(image)) = bank.objects.get_mut(&id).map(|o|&mut o.state) {
                    match method.as_slice() {
                        b"clip" => {
                            let mut rect=[values[0] as i32,values[1] as i32,(values[0] as i32).wrapping_add(values[2] as i32),(values[1] as i32).wrapping_add(values[3] as i32)];
                            // 7497C0 reads the prior Surface clip for an empty RECT;
                            // 68D640 ignores a rectangle wholly outside the source.
                            if rect[0]<rect[2]&&rect[1]<rect[3] {
                                if image.size.iter().all(|v|*v>0){rect=[rect[0].max(0),rect[1].max(0),rect[2].min(image.size[0]),rect[3].min(image.size[1])];}
                                if rect[0]<rect[2]&&rect[1]<rect[3]{image.clip=Some(rect);}
                            }
                        },
                        b"tone" => image.tone=values[..3].try_into().unwrap(),
                        b"filter" => image.filter=values[0],
                        b"blur" => image.blur=[(values[0] as i32).clamp(0,255) as u8,(values[1] as i32).clamp(0,255) as u8],
                        _ => image.coordinate_mode=values[0],
                    }
                }
            }
        }

        // 76CD30 → 749EB0: ID slot a (at most 31) reads frame b and the map turns on.
        b"settbl" => {
            let (slot, value) = (word(vm, 0)?, word(vm, 1)?);
            for id in bank.targets(target, vm, strings)? {
                if let Some(State::Image(image)) = bank.objects.get_mut(&id).map(|object| &mut object.state) {
                    if let Some(entry) = image.ids.get_mut(slot as usize) {
                        *entry = value;
                        image.mapped_ids = true;
                        image.map_changed = true;
                    }
                }
            }
        }
        // 76E2E0 → 749090: draw through this PLANE; a name that resolves to none keeps the
        // default whole-screen rectangle.
        b"pl" => {
            let source = args.first().context("CatSystem2 pl argument")?;
            let reference = crate::fes_references::evaluate(source, vm, strings, &|name| bank.count(name))?;
            for id in bank.targets(target, vm, strings)? {
                if let Some(State::Image(image) | State::CImage(image)) = bank.objects.get_mut(&id).map(|object| &mut object.state) {
                    image.plane = reference.clone();
                }
            }
        }
        // 777A10 / 777BE0: pose words into up to three flags.
        b"getbase" | b"getoffset" => {
            let ids = bank.targets(target, vm, strings)?;
            let Some(id) = ids.first() else { return Ok(()) };
            let pose = match &bank.objects.get(id).context("CatSystem2 pose object")?.state {
                State::Image(image) | State::CImage(image) => {
                    if method == b"getbase" { image.pose.pivot } else { image.pose.offset }
                }
                _ => return Ok(()),
            };
            for (at, value) in pose.iter().enumerate().take(args.len()) {
                let dest = vm.evaluate(&args[at])?;
                vm.set_flag(dest, *value as i32 as u32);
            }
        }
        other => anyhow::bail!("CatSystem2 image method {} is not implemented", String::from_utf8_lossy(other)),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anm_rows_read_frame_and_hold() {
        let mut bytes = vec![0u8; 32 + 2 * 68];
        bytes[..4].copy_from_slice(b"ANM\0");
        bytes[8] = 2;
        bytes[32 + 8] = 7;
        bytes[32 + 16] = 3;
        bytes[32 + 68 + 8] = 9;
        assert_eq!(parse_anm(&bytes).unwrap(), vec![(7, 3), (9, 1)]);
        assert!(parse_anm(b"nope").is_err());
    }
}
