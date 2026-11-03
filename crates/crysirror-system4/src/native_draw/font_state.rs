// GPL-2.0-or-later; native wrappers/profile from pinned DrawGraph.c/text.c and
// reviewed owner DrawGraph/DrawText2 static source facts, recorded in SOURCE.
use super::*;
#[derive(Clone, Debug)]
pub(super) struct FontState {
    pub size: i32,
    pub name: Vec<u8>,
    pub weight: i32,
    pub underline: bool,
    pub strikeout: bool,
    pub space: i32,
    pub color: [u8; 3],
}
impl Default for FontState {
    fn default() -> Self {
        Self {
            size: 16,
            name: vec![
                0x82, 0x6c, 0x82, 0x72, 0x20, 0x83, 0x53, 0x83, 0x56, 0x83, 0x62, 0x83, 0x4e,
            ],
            weight: 700,
            underline: false,
            strikeout: false,
            space: 0,
            color: [255; 3],
        }
    }
}
impl FontState {
    pub(super) fn dispatch(
        &mut self,
        machine: &mut Machine,
        call: &ExternalCall,
        name: &[u8],
        return_kind: i32,
        args: &[Argument],
        native_profile: bool,
    ) -> Result<Option<Dispatch>> {
        let known = matches!(
            name,
            b"SetFontSize"
                | b"SetFontName"
                | b"SetFontWeight"
                | b"SetFontUnderline"
                | b"SetFontStrikeOut"
                | b"SetFontSpace"
                | b"SetFontColor"
                | b"GetFontSize"
                | b"GetFontName"
                | b"GetFontWeight"
                | b"GetFontUnderline"
                | b"GetFontStrikeOut"
                | b"GetFontSpace"
                | b"GetFontColor"
        );
        if !known {
            return Ok(None);
        }
        let getter = name.starts_with(b"Get");
        let expected = if getter && name != b"GetFontColor" {
            if name == b"GetFontName" {
                12
            } else {
                10
            }
        } else {
            0
        };
        ensure!(return_kind == expected, "System4 font control return ABI");
        let mut next = self.clone();
        let result = match (name, args) {
            (b"SetFontSize", [Argument::Scalar { kind: 10, bits }]) => {
                next.size = *bits;
                HllReturn::Void
            }
            (b"SetFontWeight", [Argument::Scalar { kind: 10, bits }]) => {
                next.weight = *bits;
                HllReturn::Void
            }
            (b"SetFontSpace", [Argument::Scalar { kind: 10, bits }]) => {
                next.space = *bits;
                HllReturn::Void
            }
            (b"SetFontUnderline", [Argument::Scalar { kind: 10, bits }]) => {
                next.underline = if native_profile {
                    *bits == 1
                } else {
                    *bits != 0
                };
                HllReturn::Void
            }
            (b"SetFontStrikeOut", [Argument::Scalar { kind: 10, bits }]) => {
                next.strikeout = if native_profile {
                    *bits == 1
                } else {
                    *bits != 0
                };
                HllReturn::Void
            }
            (b"SetFontName", [Argument::String { bytes, .. }]) => {
                let bytes = bytes.split(|b| *b == 0).next().unwrap();
                ensure!(bytes.len() <= 4096, "System4 font name budget");
                next.name = bytes.to_vec();
                HllReturn::Void
            }
            (
                b"SetFontColor",
                [Argument::Scalar { kind: 10, bits: r }, Argument::Scalar { kind: 10, bits: g }, Argument::Scalar { kind: 10, bits: b }],
            ) => {
                next.color = [
                    (*r).clamp(0, 255) as u8,
                    (*g).clamp(0, 255) as u8,
                    (*b).clamp(0, 255) as u8,
                ];
                HllReturn::Void
            }
            (b"GetFontSize", []) => HllReturn::Scalar(self.size),
            (b"GetFontWeight", []) => HllReturn::Scalar(self.weight),
            (b"GetFontSpace", []) => HllReturn::Scalar(self.space),
            (b"GetFontUnderline", []) => HllReturn::Scalar(i32::from(self.underline)),
            (b"GetFontStrikeOut", []) => HllReturn::Scalar(i32::from(self.strikeout)),
            (b"GetFontName", []) => HllReturn::String(self.name.clone()),
            (b"GetFontColor", refs) if refs.len() == 3 => {
                let mut outputs = Vec::new();
                for arg in refs {
                    let Argument::Reference {
                        kind: 18,
                        page,
                        index,
                    } = arg
                    else {
                        bail!("System4 font color reference ABI");
                    };
                    machine.heap.get(*page, *index)?;
                    outputs.push((*page, *index));
                }
                for ((page, index), value) in outputs.into_iter().zip(self.color) {
                    machine.heap.set(page, index, i32::from(value))?;
                }
                HllReturn::Void
            }
            _ => bail!("System4 original font control argument ABI"),
        };
        machine.complete_hll(call, result)?;
        *self = next;
        Ok(Some(Dispatch::Completed))
    }
}
