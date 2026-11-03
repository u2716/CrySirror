// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original SNDTABLE (class 10): a SOUND with a named track table. 761880 entry,
//! 761810 size, 761930 order reset, 761990 shuffle, 761B20/761E10/761E30/761E40
//! cursor and 761B00 repeat. Playback stays with the SOUND class and shared audio.
use crate::{fes_object_bank::Bank, motion::RandomStreams, native_class_state::{SoundState, State}, native_interpreter::Interpreter, native_values::StringVariables};
use anyhow::{ensure, Context, Result};
use crysirror_core::state::ClientId;
use serde::{Deserialize, Serialize};

const MAX_ENTRIES: u32 = 4096;
const NAME_BYTES: usize = 32;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SoundTable {
    /// (enabled flag, track file name), 36-byte native records.
    pub entries: Vec<(bool, Vec<u8>)>,
    pub order: Vec<usize>,
    pub cursor: i32,
    pub repeat: bool,
    pub shuffled: bool,
}
impl SoundTable {
    /// 761810: reallocates both arrays, then 761930 resets the order.
    pub fn resize(&mut self, count: u32) -> Result<()> {
        ensure!(count <= MAX_ENTRIES, "CatSystem2 SNDTABLE size budget");
        self.entries = vec![(false, Vec::new()); count as usize];
        self.order = vec![0; count as usize];
        self.reset_order();
        Ok(())
    }
    /// 761930: identity order, keeping the cursor on the same original entry.
    pub fn reset_order(&mut self) {
        let current = usize::try_from(self.cursor).ok().and_then(|at| self.order.get(at)).copied();
        for (at, slot) in self.order.iter_mut().enumerate() {
            *slot = at;
        }
        if self.cursor != -1 {
            if let Some(entry) = current {
                self.cursor = entry as i32;
            }
        }
        self.shuffled = false;
    }
    fn enabled(&self, at: usize) -> bool {
        self.order.get(at).and_then(|entry| self.entries.get(*entry)).is_some_and(|e| e.0)
    }
    /// 761B20 body: wrap per repeat, take the next enabled entry forward, and
    /// otherwise step back to the last enabled entry and stop.
    fn select(&mut self, sound_requested: i32, sound: &mut SoundFields<'_>) {
        let count = self.order.len() as i32;
        let mut at = sound_requested;
        if at < 0 {
            at = if self.repeat { count - 1 } else { 0 };
        } else if at >= count {
            at = if self.repeat { 0 } else { count - 1 };
        }
        while at < count && !self.enabled(at as usize) {
            at += 1;
        }
        if at == count && self.repeat {
            at = 0;
            while at < self.cursor && !self.enabled(at as usize) {
                at += 1;
            }
        }
        if at < count {
            self.cursor = at;
            self.load_current(sound);
        } else {
            let mut back = count - 1;
            while back >= 0 && !self.enabled(back as usize) {
                back -= 1;
            }
            self.cursor = back;
            if *sound.playing {
                sound.stop();
            }
        }
    }
    fn load_current(&self, sound: &mut SoundFields<'_>) {
        if let Some(entry) = usize::try_from(self.cursor).ok().and_then(|at| self.order.get(at)) {
            sound.filename.clone_from(&self.entries[*entry].1);
            *sound.load_dirty = true;
        }
    }
    /// 761E10: original entry at the cursor.
    pub fn current(&self) -> u32 {
        usize::try_from(self.cursor).ok().and_then(|at| self.order.get(at)).map_or(0, |e| *e as u32)
    }
    /// 761E40: the previous enabled entry, then the 761B20 selection.
    fn reverse(&mut self, sound: &mut SoundFields<'_>) {
        let mut at = self.cursor - 1;
        while at >= 0 && !self.enabled(at as usize) {
            at -= 1;
        }
        self.select(at, sound);
    }
    /// 761990: 1000 random swaps; `first` moves to the head; cursor to the first
    /// enabled entry. A loaded track is replaced only while one is pending.
    fn shuffle(&mut self, first: i32, random: &mut RandomStreams, client: ClientId, sound: &mut SoundFields<'_>) -> Result<()> {
        let count = self.order.len() as u32;
        if count == 0 {
            return Ok(());
        }
        if *sound.playing && self.shuffled {
            *sound.load_dirty = true;
        }
        self.reset_order();
        for _ in 0..1000 {
            let a = (random.integer_word(client)? % count) as usize;
            let b = (random.integer_word(client)? % count) as usize;
            self.order.swap(a, b);
        }
        if first >= 0 {
            if let Some(at) = self.order.iter().position(|entry| *entry == first as usize) {
                self.order.swap(0, at);
            }
        }
        if let Some(at) = (0..count as usize).find(|at| self.enabled(*at)) {
            self.cursor = at as i32;
        }
        if *sound.load_dirty && self.cursor != -1 {
            self.load_current(sound);
        }
        self.shuffled = true;
        Ok(())
    }
}
struct SoundFields<'a> {
    filename: &'a mut Vec<u8>,
    load_dirty: &'a mut bool,
    playing: &'a mut bool,
    stop_requested: &'a mut bool,
    play_dirty: &'a mut bool,
}
impl SoundFields<'_> {
    /// 74C2B0(this, 0).
    fn stop(&mut self) {
        *self.playing = false;
        *self.stop_requested = true;
        *self.play_dirty = true;
    }
}
fn fields(sound: &mut SoundState) -> (&mut SoundTable, SoundFields<'_>) {
    let SoundState { table, filename, load_dirty, playing, stop_requested, play_dirty, .. } = sound;
    (
        table.as_mut().expect("SNDTABLE state"),
        SoundFields { filename, load_dirty, playing, stop_requested, play_dirty },
    )
}
pub(crate) fn supports(method: &[u8]) -> bool {
    matches!(method, b"sndtbl" | b"setid" | b"getid" | b"next" | b"rev" | b"repeat" | b"shuffle")
}
/// True when `target` names SNDTABLE objects; other classes keep their own paths.
pub(crate) fn is_table(bank: &Bank, target: &[u8], vm: &Interpreter, strings: &StringVariables) -> Result<bool> {
    let mut staged = vm.clone();
    let reference = crate::fes_references::evaluate(target, &mut staged, strings, &|name| bank.count(name))?;
    let index = if reference.index == -1 { 0 } else { reference.index };
    let Some(id) = bank.lookup(&crate::fes_references::IndexedName { name: reference.name.clone(), index }) else {
        return Ok(false);
    };
    Ok(matches!(&bank.objects[&id].state, State::Sound(sound) if sound.table.is_some()))
}
pub(crate) fn apply(
    bank: &mut Bank,
    target: &[u8],
    arguments: &[Vec<u8>],
    vm: &mut Interpreter,
    strings: &StringVariables,
    random: Option<&mut RandomStreams>,
    client: ClientId,
) -> Result<()> {
    let method = arguments.first().context("CatSystem2 SNDTABLE method missing")?.clone();
    let args = &arguments[1..];
    let mut staged = vm.clone();
    let reference = crate::fes_references::evaluate(target, &mut staged, strings, &|name| bank.count(name))?;
    let count = bank.count(&reference.name).context("CatSystem2 SNDTABLE target name missing")?;
    let word = |vm: &mut Interpreter, at: usize, default: u32| args.get(at).map_or(Ok(default), |value| vm.evaluate(value));
    // Single-target natives return at once for the all-array sentinel.
    if reference.index == -1 && matches!(method.as_slice(), b"getid" | b"next" | b"rev" | b"repeat" | b"shuffle") {
        *vm = staged;
        return Ok(());
    }
    let indices: Vec<i32> = if reference.index == -1 { (0..count as i32).collect() } else { vec![reference.index] };
    // sndtbl and setid evaluate their parameters once, before the member loop.
    let entry = if method == b"sndtbl" {
        let index = word(&mut staged, 0, 0)? as i32;
        let name = args.get(1).map_or(Ok(Vec::new()), |value| strings.expand(value, NAME_BYTES - 1))?;
        ensure!(name.len() < NAME_BYTES && !name.contains(&0), "CatSystem2 SNDTABLE entry name extent");
        let enabled = word(&mut staged, 2, 0)? != 0;
        Some((index, name, enabled))
    } else {
        None
    };
    let request = if method == b"setid" { word(&mut staged, 0, 0)? as i32 } else { 0 };
    let destination = if method == b"getid" { word(&mut staged, 0, 0)? } else { 0 };
    let (shuffle_on, shuffle_first) = if method == b"shuffle" {
        (word(&mut staged, 0, 0)? != 0, word(&mut staged, 1, u32::MAX)? as i32)
    } else {
        (false, -1)
    };
    let repeat = if method == b"repeat" { word(&mut staged, 0, 0)? != 0 } else { false };
    let mut random = random;
    let mut updated = Vec::with_capacity(indices.len());
    for index in indices {
        let id = bank.lookup(&crate::fes_references::IndexedName { name: reference.name.clone(), index }).context("CatSystem2 SNDTABLE target outside array")?;
        let State::Sound(original) = &bank.objects[&id].state else {
            anyhow::bail!("CatSystem2 SNDTABLE command on another native class")
        };
        ensure!(original.table.is_some(), "CatSystem2 SNDTABLE command on a SOUND object");
        let mut sound = original.clone();
        let (table, mut fields) = fields(&mut sound);
        match method.as_slice() {
            b"sndtbl" => {
                let (at, name, enabled) = entry.as_ref().unwrap();
                if let Some(slot) = usize::try_from(*at).ok().and_then(|at| table.entries.get_mut(at)) {
                    *slot = (*enabled, name.clone());
                }
            }
            b"setid" => table.select(request, &mut fields),
            b"getid" => {
                staged.set_flag(destination, table.current());
            }
            b"next" => {
                let at = table.cursor + 1;
                table.select(at, &mut fields);
            }
            b"rev" => table.reverse(&mut fields),
            b"repeat" => table.repeat = repeat,
            b"shuffle" => {
                if shuffle_on {
                    let random = random.as_deref_mut().context("CatSystem2 SNDTABLE shuffle requires an owned random stream")?;
                    table.shuffle(shuffle_first, random, client, &mut fields)?;
                } else {
                    table.reset_order();
                }
            }
            _ => anyhow::bail!("CatSystem2 unknown SNDTABLE method"),
        }
        updated.push((id, State::Sound(sound)));
    }
    for (id, state) in updated {
        bank.objects.get_mut(&id).unwrap().state = state;
    }
    *vm = staged;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn table(count: u32) -> (SoundTable, SoundState) {
        let mut table = SoundTable::default();
        table.resize(count).unwrap();
        (table, SoundState::default())
    }
    fn select(table: &mut SoundTable, sound: &mut SoundState, at: i32) {
        let SoundState { filename, load_dirty, playing, stop_requested, play_dirty, .. } = sound;
        table.select(at, &mut SoundFields { filename, load_dirty, playing, stop_requested, play_dirty });
    }
    #[test]
    fn selection_skips_disabled_tracks_and_stops_past_the_end() {
        let (mut t, mut s) = table(3);
        t.entries[0] = (true, b"a".to_vec());
        t.entries[1] = (false, b"b".to_vec());
        t.entries[2] = (true, b"c".to_vec());
        select(&mut t, &mut s, 1);
        assert_eq!((t.cursor, t.current()), (2, 2));
        assert_eq!(s.filename, b"c");
        assert!(s.load_dirty);
        t.entries[2].0 = false;
        s.playing = true;
        select(&mut t, &mut s, 1);
        assert_eq!(t.cursor, 0, "falls back to the last enabled entry");
        assert!(!s.playing && s.stop_requested);
    }
    #[test]
    fn repeat_wraps_both_ways() {
        let (mut t, mut s) = table(2);
        t.entries[0] = (true, b"a".to_vec());
        t.entries[1] = (true, b"b".to_vec());
        t.repeat = true;
        select(&mut t, &mut s, 2);
        assert_eq!(t.cursor, 0);
        select(&mut t, &mut s, -1);
        assert_eq!(t.cursor, 1);
    }
}
