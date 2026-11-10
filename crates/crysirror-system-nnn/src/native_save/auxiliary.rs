// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native COmakeClassSupport 100x32 table and CUTIN record consumers.
//! MUGEN2 0x43d980/0x43d9f0 copy the whole table. 0x4fe8a0/0x4fe8f0
//! copy 128 CUTIN words; 0x534490/0x5344d0 own word255. All other CUTIN
//! words are cleared by 0x4d9d40 before the producer is called.
use super::*;
#[derive(Clone, Debug)]
pub struct OmakeClasses {
    words: Vec<i32>,
}
impl Default for OmakeClasses {
    fn default() -> Self {
        Self {
            words: vec![0; 3200],
        }
    }
}
impl OmakeClasses {
    pub fn row(&self, parent: usize) -> Result<&[i32]> {
        ensure!(parent < 100, "NNN Omake parent mode bounds");
        Ok(&self.words[parent * 32..parent * 32 + 32])
    }
    pub fn find(&self, parent: usize, mode: i32) -> Result<Option<usize>> {
        Ok(self.row(parent)?.iter().position(|word| word / 256 == mode))
    }
    pub fn add(
        &mut self,
        parent: usize,
        mode: i32,
        flags: u8,
        place: Option<usize>,
    ) -> Result<bool> {
        if self.find(parent, mode)?.is_some() {
            return Ok(false);
        }
        ensure!(
            (1..=0x7fffff).contains(&mode),
            "NNN Omake child mode bounds"
        );
        let row = self.row(parent)?;
        let place = match place {
            Some(place) => {
                ensure!(place < 32, "NNN Omake insertion bounds");
                place
            }
            None => match row.iter().position(|v| *v == 0) {
                Some(place) => place,
                None => return Ok(false),
            },
        };
        if row[31] != 0 {
            return Ok(false);
        }
        let row = &mut self.words[parent * 32..parent * 32 + 32];
        row.copy_within(place..31, place + 1);
        row[place] = mode * 256 | i32::from(flags);
        Ok(true)
    }
    pub fn delete(&mut self, parent: usize, mode: i32) -> Result<bool> {
        let Some(place) = self.find(parent, mode)? else {
            return Ok(false);
        };
        let row = &mut self.words[parent * 32..parent * 32 + 32];
        row.copy_within(place + 1..32, place);
        row[31] = 0;
        Ok(true)
    }
    pub fn flags(&self, parent: usize, mode: i32, bit: Option<u8>) -> Result<i32> {
        let Some(place) = self.find(parent, mode)? else {
            return Ok(-1);
        };
        let flags = self.row(parent)?[place] & 255;
        Ok(match bit {
            None => flags,
            Some(bit) => {
                ensure!(bit < 8, "NNN Omake flag bit bounds");
                (flags >> bit) & 1
            }
        })
    }
    pub fn set(
        &mut self,
        parent: usize,
        mode: Option<i32>,
        value: u8,
        bit: Option<u8>,
    ) -> Result<()> {
        self.row(parent)?;
        if let Some(bit) = bit {
            ensure!(bit < 8, "NNN Omake flag bit bounds");
        }
        for word in &mut self.words[parent * 32..parent * 32 + 32] {
            let child = *word / 256;
            if child == 0 || mode.is_some_and(|mode| mode != child) {
                continue;
            }
            let flags = match bit {
                None => i32::from(value),
                Some(bit) => (*word & 255 & !(1 << bit)) | if value != 0 { 1 << bit } else { 0 },
            };
            *word = child * 256 | flags;
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub(crate) struct AuxiliaryState {
    pub omake: OmakeClasses,
    pub cutin_rows: [[i32; 2]; 64],
    pub cutin_mode: i32,
    pub chart: Option<crate::native_chart::ChartState>,
    chart_blocks: Vec<usize>,
    chart_slot_function: Option<Vec<u8>>,
}
impl Default for AuxiliaryState {
    fn default() -> Self {
        Self {
            omake: OmakeClasses::default(),
            cutin_rows: [[0; 2]; 64],
            cutin_mode: 0,
            chart: None,
            chart_blocks: vec![],
            chart_slot_function: None,
        }
    }
}
impl AuxiliaryState {
    pub(crate) fn configure_chart(&mut self, configuration: &crate::native_bootstrap::NativeConfiguration) -> Result<()> {
        let mut blocks = Vec::new(); let mut automatic = 0;
        for index in 1..=256 {
            let Some(name) = configuration.setup.parameter(format!("autoExtDataMode{index}").as_bytes()) else { break; };
            let mode = crate::native_mode::number(name)?;
            if mode > 0 {
                if mode == 34 { blocks.push(automatic); }
                automatic += 1;
            }
        }
        if blocks.is_empty() && configuration.chart_setup.is_none() { return Ok(()); }
        let table = configuration.chart_setup.as_ref().context("NNN ChartMenu setup absent")?;
        let layout = crate::native_chart::ChartLayout::read(table)?;
        if let Some(chart) = &self.chart { ensure!(chart.layout==layout,"NNN ChartMenu layout changed during native session"); }
        else { self.chart = Some(crate::native_chart::ChartState::fresh(layout)); }
        self.chart_blocks = blocks;
        self.chart_slot_function = (table.integer(b"useSlotOnFunction")?.unwrap_or(0)!=0)
            .then(|| table.parameter(b"slotOnFunctionName").unwrap_or(b"sloton").to_vec());
        Ok(())
    }
    pub(crate) fn chart_function(&self, name: &[u8], parameters: &[i32]) -> Result<Option<crate::native_chart::ChartState>> {
        if !self.chart_slot_function.as_ref().is_some_and(|registered|registered.eq_ignore_ascii_case(name)) { return Ok(None); }
        let mut chart = self.chart.clone().context("NNN ChartMenu sloton actor absent")?;
        chart.set_selection(parameters.first().copied().unwrap_or(0),parameters.get(1).copied().unwrap_or(1)!=0)?;
        Ok(Some(chart))
    }
    pub fn restore(file: &NativeSaveFile, profile: &LegacySaveProfile) -> Result<Self> {
        Self::restore_using(file,profile,&Self::default())
    }
    pub fn restore_using(file: &NativeSaveFile, profile: &LegacySaveProfile, seed: &Self) -> Result<Self> {
        file.validate(profile)?;
        let mut out = Self::default();
        let mut at = 8;
        if profile.omake {
            out.omake.words = file.blocks[at]
                .payload
                .chunks_exact(4)
                .map(|w| i32::from_le_bytes(w.try_into().unwrap()))
                .collect();
            at += 1;
        }
        if profile.cutin {
            let payload = &file.blocks[at].payload;
            for (i, row) in out.cutin_rows.iter_mut().enumerate() {
                for lane in 0..2 {
                    let start = (i * 2 + lane) * 4;
                    row[lane] = i32::from_le_bytes(payload[start..start + 4].try_into().unwrap());
                }
            }
            out.cutin_mode = i32::from_le_bytes(payload[1020..1024].try_into().unwrap());
        }
        out.chart = seed.chart.clone();
        out.chart_blocks = seed.chart_blocks.clone();
        out.chart_slot_function = seed.chart_slot_function.clone();
        let first = 8+usize::from(profile.omake)+usize::from(profile.cutin);
        for index in &out.chart_blocks {
            let block = file.blocks.get(first+index).context("NNN ChartMenu automatic EXT index outside save")?;
            out.chart.as_mut().unwrap().load(&block.payload)?;
        }
        Ok(out)
    }
    pub fn write(&self, file: &mut NativeSaveFile, profile: &LegacySaveProfile) -> Result<()> {
        file.validate(profile)?;
        let mut at = 8;
        if profile.omake {
            for (word, bytes) in self
                .omake
                .words
                .iter()
                .zip(file.blocks[at].payload.chunks_exact_mut(4))
            {
                bytes.copy_from_slice(&word.to_le_bytes());
            }
            at += 1;
        }
        if profile.cutin {
            let bytes = &mut file.blocks[at].payload;
            bytes.fill(0);
            for (i, row) in self.cutin_rows.iter().enumerate() {
                for lane in 0..2 {
                    let start = (i * 2 + lane) * 4;
                    bytes[start..start + 4].copy_from_slice(&row[lane].to_le_bytes());
                }
            }
            bytes[1020..1024].copy_from_slice(&self.cutin_mode.to_le_bytes());
        }
        // The reviewed CGame vtable+548/+552 are the actual empty fallback
        // callbacks. 0x4d9dd0 clears EXT payloads first, so declared fallback
        // extensions are really zero on save and ignored on load. Automatic
        // standard mode extensions retain their zero-sized blocks. ChartMenu's
        // real actor writes its selection/road payload after this clear.
        let first = 8 + usize::from(profile.omake) + usize::from(profile.cutin);
        for block in file.blocks.iter_mut().skip(first) {
            block.payload.fill(0);
        }
        if let Some(chart) = &self.chart {
            for index in &self.chart_blocks {
                let block = file.blocks.get_mut(first+index).context("NNN ChartMenu automatic EXT index outside save")?;
                ensure!(block.payload.len()==chart.layout.bytes(),"NNN ChartMenu configured EXT extent mismatch");
                block.payload = chart.store();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_omake_insert_delete_flags_and_full_table() {
        let mut table = OmakeClasses::default();
        assert!(table.add(7, 9, 3, None).unwrap());
        assert!(table.add(7, 11, 0, Some(0)).unwrap());
        assert!(!table.add(7, 9, 0, None).unwrap());
        assert_eq!(&table.row(7).unwrap()[..3], &[11 * 256, 9 * 256 + 3, 0]);
        table.set(7, None, 1, Some(2)).unwrap();
        assert_eq!(table.flags(7, 9, None).unwrap(), 7);
        assert_eq!(table.flags(7, 11, Some(2)).unwrap(), 1);
        table.set(7, Some(9), 0, Some(1)).unwrap();
        assert_eq!(table.flags(7, 9, None).unwrap(), 5);
        assert!(table.delete(7, 11).unwrap());
        for child in 1..=31 {
            assert!(table.add(7, 100 + child, 0, None).unwrap());
        }
        assert!(!table.add(7, 500, 0, None).unwrap());
        assert_eq!(table.flags(7, 500, None).unwrap(), -1);
        assert!(table.row(100).is_err());
    }
    #[test]
    fn native_auxiliary_sections_round_trip_actual_words_and_clear_only_reserved_bytes() {
        let profile = LegacySaveProfile {
            header_name: b"aux".to_vec(),
            version: 3,
            extension_sizes: vec![12, 0],
            omake: true,
            cutin: true,
        };
        let mut file = NativeSaveFile::for_live(
            &profile,
            crate::native_system_file::SystemStamp([2026, 11, 9, 1, 12, 30, 5, 0]),
            crate::native_thumbnail::NativeThumbnail {
                size: [1, 1],
                bgrx: vec![1, 2, 3, 0],
            },
        )
        .unwrap();
        let mut state = AuxiliaryState::default();
        state.omake.add(3, 19, 7, None).unwrap();
        state.cutin_rows[2] = [8, 3];
        state.cutin_mode = 2;
        file.blocks[9].payload.fill(0xcc);
        file.blocks[10].payload.fill(0xdd);
        state.write(&mut file, &profile).unwrap();
        let bytes = file.encode(&profile).unwrap();
        let file = NativeSaveFile::parse(&bytes, &profile).unwrap();
        let restored = AuxiliaryState::restore(&file, &profile).unwrap();
        assert_eq!(restored.omake.flags(3, 19, None).unwrap(), 7);
        assert_eq!(restored.cutin_rows[2], [8, 3]);
        assert_eq!(restored.cutin_mode, 2);
        assert!(file.blocks[9].payload[512..1020].iter().all(|b| *b == 0));
        assert_eq!(file.blocks[10].payload, vec![0; 12]);
    }
}
