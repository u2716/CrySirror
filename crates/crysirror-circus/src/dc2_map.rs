//! D.C.II P.C. map screen: 41C530 case28 -> 408560(20) enters process 28
//! with A235DC=5. 406B90 runs 40ECC0 (setup) once, then 40FD90 every 40 ms
//! until 40FB10 sets A274D2 and writes var(5,40). The VM snapshots 40ECC0's
//! script-derived state and owns completion; the adapter view owns hit
//! testing, animation clocks and presentation through shared services.
use super::*;
///40ECC0 per-character record (A273B8..A273D0, 28 bytes each).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Chip {
    ///A273CC: var(5, i) != 0.
    pub present: bool,
    ///A273C8: var(5, 20+i).
    pub place: i32,
    ///A273C0: 120 when a later character shares the place.
    pub offset: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct MapWait {
    pub owner: crate::dc2_bitmap::Owner,
    ///A274D0: var(5,41) == 0 selects the school map.
    pub school: bool,
    ///40F620 base x: -120 when var(5,50) == 1, else 55 (school) / 10.
    pub shift: i32,
    pub chips: Vec<Chip>,
    ///var(5,40) written by 40FB10 (place index, or 99 for the menu).
    pub result: Option<i32>,
}
impl Machine {
    ///41C530 case28 -> 408560(selector, operand).
    pub(super) fn execute_map(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        if instruction.opcode != 0x1c {
            return Ok(None);
        }
        match a[0] {
            20 => {}
            //408560's switch has no case for these selectors.
            0 | 3..=5 | 12..=19 | 21..=99 | 102.. => {
                return self.advance(instruction.next, instruction.opcode).map(Some);
            }
            selector => bail!("DC2 408560 selector {selector} is not connected"),
        }
        if let Some(map) = &self.state.map {
            ensure!(
                map.owner.scene == self.state.scene && map.owner.pc == instruction.offset,
                "DC2 process28 instruction owner"
            );
            let Some(result) = map.result else { return Ok(Some(Outcome::Pending)) };
            if !self.assign(5, 40, result, host)? {
                return Ok(Some(Outcome::Pending));
            }
            self.state.map = None;
            return self.advance(instruction.next, instruction.opcode).map(Some);
        }
        let mut read = |machine: &mut Self, bank: i16, index: i16| -> Result<Option<i32>> { machine.value(bank, index, host) };
        let Some(island) = read(self, 5, 41)? else { return Ok(Some(Outcome::Pending)) };
        let Some(shifted) = read(self, 5, 50)? else { return Ok(Some(Outcome::Pending)) };
        let school = island == 0;
        let mut chips = Vec::with_capacity(10);
        for index in 0..10i16 {
            let Some(present) = read(self, 5, index)? else { return Ok(Some(Outcome::Pending)) };
            chips.push(Chip { present: present != 0, place: -1, offset: 0 });
        }
        //The 40ECC0 loop assigns places in order; an earlier character
        //sharing the newly assigned place gets the 120 px offset.
        for index in 0..10usize {
            let Some(place) = read(self, 5, 20 + index as i16)? else { return Ok(Some(Outcome::Pending)) };
            chips[index].place = place;
            for other in 0..10usize {
                if other != index && chips[other].place == place {
                    chips[other].offset = 120;
                }
            }
        }
        for chip in chips.iter().filter(|chip| chip.present) {
            ensure!(
                (0..7).contains(&chip.place),
                "DC2 map chip place {} is outside 40F620's table",
                chip.place
            );
        }
        self.state.map = Some(MapWait {
            owner: crate::dc2_bitmap::Owner { scene: self.state.scene.clone(), pc: instruction.offset },
            school,
            shift: if shifted == 1 { -120 } else if school { 55 } else { 10 },
            chips,
            result: None,
        });
        Ok(Some(Outcome::Pending))
    }
}
