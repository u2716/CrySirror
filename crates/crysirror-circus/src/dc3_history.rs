//! DC3WY402E40/401E60/402B20 native read-history records. This is original
//! save/replay metadata; shared engine history widgets do not own these bytes.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Entry {
    pub scene: i16,
    pub diff_start: u16,
    pub first: u16,
    pub last: u16,
    ///Low WORD of nativeBuf28756 at the beginning of this run.
    pub native_28756: u16,
    ///Low WORD of nativeBuf28752 at the beginning of this run.
    pub native_28752: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Difference {
    pub offset: u16,
    pub value: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    pub entries: Vec<Entry>,
    pub current: usize,
    pub label: u16,
    ///Current read-history run origin, separate from the saved scene counter.
    pub native_28748: u32,
    pub native_28752: u32,
    pub native_28756: u32,
    pub differences: Vec<Difference>,
    pub mirror: Vec<u8>,
}
impl Default for History {
    fn default() -> Self {
        Self {
            entries: vec![Entry::default(); 400],
            current: 0,
            label: 0,
            native_28748: 0,
            native_28752: 0,
            native_28756: 0,
            differences: Vec::new(),
            mirror: vec![0; 1100],
        }
    }
}
impl History {
    ///4024A0 captures changed VM bytes only. At1000 differences the source
    ///stops appending but still updates all1100 mirrored bytes.
    pub fn capture_variables(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(
            bytes.len() == 1100 && self.mirror.len() == 1100 && self.differences.len() <= 1000,
            "DC3WY variable history extent"
        );
        for (offset, (old, value)) in self.mirror.iter_mut().zip(bytes).enumerate() {
            if *old != *value {
                if self.differences.len() < 1000 {
                    self.differences.push(Difference {
                        offset: offset as u16,
                        value: *value,
                    });
                }
                *old = *value;
            }
        }
        Ok(())
    }
    pub fn note_label(
        &mut self,
        settings: &mut crate::dc3_settings::SystemData,
        scene: i32,
        label: u16,
        raw_label: u32,
        special: bool,
    ) -> Result<()> {
        ensure!(
            self.entries.len() == 400 && self.current < 400 && self.differences.len() <= 1000,
            "DC3WY native history state extent"
        );
        ensure!((-1..650).contains(&scene), "DC3WY history scene index");
        self.label = label;
        let current = &self.entries[self.current];
        let extends =
            current.scene == scene as i16 && i32::from(current.last) == i32::from(label) - 1;
        if extends {
            self.entries[self.current].last = label;
        } else {
            if self.current < 399 && current.scene != -1 {
                if current.first != 0 {
                    settings.mark_read(current.scene as usize, current.first, current.last)?;
                }
                //401E60 finds an earlier run with the same scene/end and a
                //start no later than this run. It suppresses index growth;
                //it does not return an index to replace or compact the list.
                let duplicate = self.entries[..self.current].iter().rev().any(|previous| {
                    previous.scene == current.scene
                        && previous.last == current.last
                        && previous.first <= current.first
                });
                if !duplicate {
                    self.current += 1;
                }
            }
            self.entries[self.current] = Entry {
                scene: scene as i16,
                first: label,
                last: label,
                native_28752: self.native_28752 as u16,
                native_28756: self.native_28756 as u16,
                ..Default::default()
            };
        }
        self.entries[self.current].diff_start =
            self.differences.len() as u16 | if special { 0x8000 } else { 0 };
        //42F430 returns the original label word, including its sign bit.
        if (raw_label as i32) < 0 {
            self.native_28756 = self.native_28756.wrapping_add(1);
        }
        Ok(())
    }
    ///Opcode10 resets the run indices/first sentinel and bank mirrors; it
    ///does not delete persisted read intervals in system.dat.
    pub fn reset(&mut self) -> Result<()> {
        ensure!(self.entries.len() == 400, "DC3WY history record count");
        self.current = 0;
        self.entries[0].scene = -1;
        self.native_28748 = 0;
        self.native_28752 = 0;
        self.native_28756 = 0;
        //Opcode10 leavesBuf7185/7186 (label/savepoint) unchanged.
        self.differences.clear();
        Ok(())
    }
}
