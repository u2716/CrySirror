//! Source40BF90 live host observations. Native selectors4/21 are sampled
//! once for their owning instruction and retained through worker retries.
use super::*;
use crate::machine::{
    resource_ops::{ResourceReply, ResourceRequest},
    system_ops::{SystemReply, SystemRequest},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observations {
    pub pc: u32,
    pub values: BTreeMap<i16, i32>,
}
impl Machine {
    pub(super) fn prepare_value_reads(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<bool> {
        let mut reads = Vec::new();
        match instruction.opcode {
            0x78 if matches!(a[3], 1 | 2) => reads.push((a[0], a[1])),
            0x79 => reads.push((a[2], a[3])),
            0x7b => {
                reads.push((a[0], a[1]));
                reads.push((a[2], a[3]));
            }
            0x7c => {
                reads.push((a[0], a[1]));
                if (a[3] as u8 as i8) < 0 {
                    reads.push((a[2] >> 8, i16::from(a[2] as u8)));
                }
            }
            0x7a => {
                let mut cursor = instruction.next;
                for _ in 0..crate::MAX_INSTRUCTIONS {
                    if program
                        .native_byte(cursor)
                        .context("DC3WY sum observation terminator")?
                        == 2
                    {
                        break;
                    }
                    let word = |at| -> Result<i16> {
                        Ok(i16::from_le_bytes([
                            program
                                .native_byte(at)
                                .context("DC3WY sum observation low byte")?,
                            program
                                .native_byte(at + 1)
                                .context("DC3WY sum observation high byte")?,
                        ]))
                    };
                    reads.push((word(cursor + 1)?, word(cursor + 3)?));
                    cursor = cursor
                        .checked_add(9)
                        .context("DC3WY sum observation cursor")?;
                }
                ensure!(
                    program.instruction(cursor)?.opcode == 2,
                    "DC3WY sum observation budget exceeded"
                );
            }
            _ => return Ok(true),
        }
        let selectors = reads
            .into_iter()
            .filter_map(|(bank, index)| (bank == 99 && matches!(index, 4 | 21)).then_some(index))
            .collect::<BTreeSet<_>>();
        if selectors.is_empty() {
            return Ok(true);
        }
        if self
            .state
            .observations
            .as_ref()
            .is_none_or(|observation| observation.pc != instruction.offset)
        {
            self.state.observations = Some(Observations {
                pc: instruction.offset,
                values: BTreeMap::new(),
            });
        }
        for selector in selectors {
            if self
                .state
                .observations
                .as_ref()
                .unwrap()
                .values
                .contains_key(&selector)
            {
                continue;
            }
            let value = match selector {
                4 => match host.system(SystemRequest::NativeWindowMode)? {
                    None => return Ok(false),
                    Some(SystemReply::NativeWindowMode(mode)) => i32::from(mode),
                    Some(_) => bail!("DC3WY native window observation returned a foreign receipt"),
                },
                21 => match host.resource(&ResourceRequest::ProfileSlotReadable {
                    profile: Profile::Dc3WithYou,
                    slot: 2,
                })? {
                    None => return Ok(false),
                    Some(ResourceReply::Readable(readable)) => i32::from(readable),
                    Some(_) => bail!("DC3WY slot open observation returned a foreign receipt"),
                },
                _ => unreachable!(),
            };
            self.state
                .observations
                .as_mut()
                .unwrap()
                .values
                .insert(selector, value);
        }
        Ok(true)
    }
}
