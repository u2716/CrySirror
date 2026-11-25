// SPDX-License-Identifier: MPL-2.0
//! RSX fragment programs of `MATERIAL_TABLE_1` shaders: instruction decode
//! and the recognition of material layering the programs compute.
//!
//! A shader entry is 16 bytes `{?, ?, vertex program, fragment program}`
//! from table +0x1c (count at +8); a pass's data word 0 holds the shader
//! index in its high half. A fragment program stores its total size at +0
//! and the microcode offset at +0x10. Instruction words are big-endian with
//! their 16-bit halves swapped; word 0 holds the opcode (bits 24–29),
//! destination (1–6, half precision bit 7), write mask (9–12), input
//! (13–16), texture unit (17–20), saturation (31) and end (0); words 1–3
//! the sources (type bits 0–1: temporary, input or constant; index 2–7,
//! half 8, swizzle 9–16, negate 17). An instruction with a constant source
//! is followed by the 16-byte constant.
use crate::mesh::Loaded;
use anyhow::{Context, Result, ensure};

pub const MOV: u8 = 0x01;
pub const MUL: u8 = 0x02;
pub const ADD: u8 = 0x03;
pub const MAD: u8 = 0x04;
pub const TEX: u8 = 0x17;
pub const NRM: u8 = 0x39;
pub const DIV: u8 = 0x3a;

/// Fragment inputs by index (`TEX2` is 6).
pub const INPUTS: [&str; 16] = ["WPOS", "COL0", "COL1", "FOGC", "TEX0", "TEX1", "TEX2", "TEX3", "TEX4", "TEX5", "TEX6", "TEX7", "TEX8", "TEX9", "SSA", "FACE"];
const INPUT_TEX2: u8 = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operand {
    /// Temporary register (`H` half or `R` full precision).
    Temp { half: bool, index: u8 },
    Input(u8),
    /// The instruction's constant.
    Const,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Source {
    pub operand: Operand,
    pub swizzle: [u8; 4],
    pub negate: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub op: u8,
    pub saturate: bool,
    pub half: bool,
    pub dst: u8,
    pub mask: [bool; 4],
    pub texture: u8,
    pub sources: [Source; 3],
    pub constant: Option<[f32; 4]>,
}

fn word(bytes: &[u8], at: usize) -> Option<u32> {
    let w = u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
    Some(w.rotate_left(16))
}

/// Decode microcode until the end bit (at most `bytes.len()`).
pub fn decode(bytes: &[u8]) -> Vec<Instruction> {
    let mut out = Vec::new();
    let mut at = 0;
    while let (Some(w0), Some(w1), Some(w2), Some(w3)) = (word(bytes, at), word(bytes, at + 4), word(bytes, at + 8), word(bytes, at + 12)) {
        at += 16;
        let input = ((w0 >> 13) & 0xf) as u8;
        let source = |w: u32| Source {
            operand: match w & 3 {
                0 => Operand::Temp { half: (w >> 8) & 1 != 0, index: ((w >> 2) & 0x3f) as u8 },
                1 => Operand::Input(input),
                _ => Operand::Const,
            },
            swizzle: std::array::from_fn(|k| ((w >> (9 + 2 * k)) & 3) as u8),
            negate: (w >> 17) & 1 != 0,
        };
        let sources = [source(w1), source(w2), source(w3)];
        let constant = if sources.iter().any(|s| s.operand == Operand::Const) {
            let c: Option<Vec<f32>> = (0..4).map(|k| word(bytes, at + 4 * k).map(f32::from_bits)).collect();
            at += 16;
            c.map(|c| [c[0], c[1], c[2], c[3]])
        } else {
            None
        };
        out.push(Instruction {
            op: ((w0 >> 24) & 0x3f) as u8,
            saturate: (w0 >> 31) & 1 != 0,
            half: (w0 >> 7) & 1 != 0,
            dst: ((w0 >> 1) & 0x3f) as u8,
            mask: std::array::from_fn(|k| (w0 >> (9 + k)) & 1 != 0),
            texture: ((w0 >> 17) & 0xf) as u8,
            sources,
            constant,
        });
        if w0 & 1 != 0 {
            break;
        }
    }
    out
}

/// The fragment program of pass data `data` (`MATERIAL_TABLE_1` shader).
pub fn pass_program(p: &Loaded, data: usize) -> Result<Vec<Instruction>> {
    let shader = (p.u32(data)? >> 16) as usize;
    let table = p.directory.items.iter().find(|i| i.kind == "MATERIAL_TABLE_1").context("ND material table")?.payload_offset as usize;
    ensure!(shader < p.u32(table + 8)? as usize, "ND shader index");
    let fp = p.ptr(p.ptr(table + 0x1c)? + 16 * shader + 12)?;
    let (size, code) = (p.u32(fp)? as usize, p.u32(fp + 0x10)? as usize);
    ensure!(code < size && size <= 1 << 16, "ND fragment program size");
    Ok(decode(p.data.get(fp + code..fp + size).context("ND fragment program outside package")?))
}

/// The upper layer of a slope blend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Top {
    /// The pass sampler with this index.
    Sampler(u8),
    /// A colour the program computes (the `cliffwall-snow` programs: a
    /// light grey of 0.5 plus saturated terms).
    Computed,
}

/// `colour = mix(top, side, saturate(1 − n.y))` with `n` the normalised
/// world normal input `TEX2`: the side layer on walls, the top layer on
/// upward-facing surfaces (the `train-wreck` snow programs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlopeBlend {
    pub side: u8,
    pub top: Top,
}

/// What the recogniser knows of one register component.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    Unknown,
    Const(f32),
    /// Component `k` of the normalised world normal.
    Normal(u8),
    /// `a · n.y + b`, saturated or not.
    Linear { a: f32, b: f32, saturated: bool },
    Texture(u8),
    /// `texture − other` (`None`: a computed value).
    Difference(u8, Option<u8>),
    Blend(SlopeBlend),
}

/// Texture units 12 and up hold the light pre-pass buffers; the units
/// below take the pass's samplers in order (the bark programs sample from
/// units 1–3, the snow ones from 0–2, each for samplers 0–2; inferred from
/// what each unit is sampled with).
pub const FIRST_GLOBAL_UNIT: u8 = 12;

/// The pass sampler each texture unit below [`FIRST_GLOBAL_UNIT`] reads.
pub fn unit_samplers(code: &[Instruction]) -> Vec<u8> {
    let mut units: Vec<u8> = code.iter().filter(|i| matches!(i.op, TEX | 0x18 | 0x19 | 0x2f | 0x31) && i.texture < FIRST_GLOBAL_UNIT).map(|i| i.texture).collect();
    units.sort_unstable();
    units.dedup();
    units
}

/// Find a slope blend: a texture difference scaled by
/// `saturate(1 − n.y)` and added back to its second term. Layers are
/// reported as pass sampler indices.
pub fn slope_blend(code: &[Instruction]) -> Option<SlopeBlend> {
    let units = unit_samplers(code);
    let sampler = |unit: u8| units.iter().position(|u| *u == unit).map(|i| i as u8);
    let blend = slope_units(code)?;
    Some(SlopeBlend {
        side: sampler(blend.side)?,
        top: match blend.top {
            Top::Sampler(u) => Top::Sampler(sampler(u)?),
            Top::Computed => Top::Computed,
        },
    })
}

/// [`slope_blend`] in texture units.
fn slope_units(code: &[Instruction]) -> Option<SlopeBlend> {
    let mut regs: std::collections::HashMap<(bool, u8), [Value; 4]> = std::collections::HashMap::new();
    let mut found = None;
    for ins in code {
        // The component a source supplies to result component `k`, and
        // whether it is negated.
        let read = |s: &Source, k: usize| -> (Value, bool) {
            let c = usize::from(s.swizzle[k]);
            let v = match s.operand {
                Operand::Const => Value::Const(ins.constant.map_or(0., |c4| c4[c])),
                Operand::Input(_) => Value::Unknown,
                Operand::Temp { half, index } => regs.get(&(half, index)).map_or(Value::Unknown, |r| r[c]),
            };
            (v, s.negate)
        };
        let signed = |(v, negate): (Value, bool)| match (v, negate) {
            (Value::Const(x), true) => Value::Const(-x),
            (Value::Linear { a, b, .. }, true) => Value::Linear { a: -a, b: -b, saturated: false },
            (Value::Texture(_) | Value::Difference(..) | Value::Normal(_) | Value::Blend(_), true) => Value::Unknown,
            (v, _) => v,
        };
        let mut result = [Value::Unknown; 4];
        for (k, slot) in result.iter_mut().enumerate() {
            if !ins.mask[k] {
                continue;
            }
            let (a, b, c) = (signed(read(&ins.sources[0], k)), signed(read(&ins.sources[1], k)), signed(read(&ins.sources[2], k)));
            // `DIV` divides by the divisor's first component.
            let divisor = signed(read(&ins.sources[1], 0));
            let linear = |v: Value| match v {
                Value::Normal(1) => Some((1., 0.)),
                Value::Linear { a, b, .. } => Some((a, b)),
                Value::Const(x) => Some((0., x)),
                _ => None,
            };
            let folded = |a: f32, b: f32| if a == 0. { Value::Const(b) } else { Value::Linear { a, b, saturated: false } };
            *slot = match ins.op {
                MOV => a,
                TEX => Value::Texture(ins.texture),
                NRM if ins.sources[0].operand == Operand::Input(INPUT_TEX2) => Value::Normal(ins.sources[0].swizzle[k]),
                NRM => match a {
                    Value::Normal(_) => Value::Normal(k as u8),
                    _ => Value::Unknown,
                },
                ADD => match (a, read(&ins.sources[1], k)) {
                    (Value::Texture(t), (Value::Texture(u), true)) => Value::Difference(t, Some(u)),
                    (Value::Texture(t), (Value::Unknown, true)) => Value::Difference(t, None),
                    _ => match (linear(a), linear(b)) {
                        (Some((p, q)), Some((r, s))) => folded(p + r, q + s),
                        _ => Value::Unknown,
                    },
                },
                MUL | DIV => match (linear(a), if ins.op == DIV { divisor } else { b }) {
                    (Some((p, q)), Value::Const(x)) if x != 0. => {
                        let f = if ins.op == DIV { 1. / x } else { x };
                        folded(p * f, q * f)
                    }
                    _ => Value::Unknown,
                },
                MAD => match (a, b, c) {
                    (Value::Linear { a, b, saturated: true }, Value::Difference(side, top), third) if (a + 1.).abs() < 1e-6 && (b - 1.).abs() < 1e-6 => {
                        let top = match (top, third) {
                            (Some(u), Value::Texture(v)) if u == v => Some(Top::Sampler(u)),
                            (None, Value::Unknown) => Some(Top::Computed),
                            _ => None,
                        };
                        top.map_or(Value::Unknown, |top| Value::Blend(SlopeBlend { side, top }))
                    }
                    _ => Value::Unknown,
                },
                _ => Value::Unknown,
            };
            if ins.saturate {
                if let Value::Linear { a, b, .. } = *slot {
                    *slot = Value::Linear { a, b, saturated: true };
                }
            }
            if let Value::Blend(s) = *slot {
                found = Some(s);
            }
        }
        let entry = regs.entry((ins.half, ins.dst)).or_insert([Value::Unknown; 4]);
        for k in 0..4 {
            if ins.mask[k] {
                entry[k] = result[k];
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(operand: Operand, swizzle: [u8; 4], negate: bool) -> Source {
        Source { operand, swizzle, negate }
    }
    fn ins(op: u8, saturate: bool, dst: u8, mask: [bool; 4], texture: u8, sources: [Source; 3], constant: Option<[f32; 4]>) -> Instruction {
        Instruction { op, saturate, half: true, dst, mask, texture, sources, constant }
    }

    #[test]
    fn recognises_two_texture_slope_blend() {
        let h = |i| Operand::Temp { half: true, index: i };
        let none = src(h(0), [0, 1, 2, 3], false);
        let xyz = [true, true, true, false];
        let w = [false, false, false, true];
        let x = [true, false, false, false];
        // From `trnwrk-snow-mod-cliff-cheap` (shader 34 of `train-wreck-b`).
        let code = vec![
            // Unit 0: the screen-space buffer of sampler 0.
            ins(TEX, false, 8, x, 0, [none; 3], None),
            ins(TEX, false, 6, xyz, 1, [none; 3], None),
            ins(NRM, false, 5, xyz, 0, [src(Operand::Input(INPUT_TEX2), [0, 1, 2, 3], false), none, none], None),
            ins(ADD, false, 7, x, 0, [src(Operand::Const, [0; 4], false), src(Operand::Const, [1; 4], true), none], Some([0., 1., 0., 0.])),
            ins(MOV, false, 0, xyz, 0, [src(h(5), [0, 1, 2, 3], false), none, none], None),
            ins(NRM, false, 1, xyz, 0, [src(h(0), [0, 1, 2, 3], false), none, none], None),
            ins(ADD, false, 6, w, 0, [src(h(1), [1; 4], false), src(Operand::Const, [1; 4], true), none], Some([0., 1., 0., 0.])),
            ins(DIV, true, 6, w, 0, [src(h(6), [3; 4], false), src(h(7), [0; 4], false), none], None),
            ins(TEX, false, 5, xyz, 2, [none; 3], None),
            ins(ADD, false, 6, xyz, 0, [src(h(6), [0, 1, 2, 3], false), src(h(5), [0, 1, 2, 3], true), none], None),
            ins(MAD, false, 0, xyz, 0, [src(h(6), [3; 4], false), src(h(6), [0, 1, 2, 3], false), src(h(5), [0, 1, 2, 3], false)], None),
        ];
        assert_eq!(slope_blend(&code), Some(SlopeBlend { side: 1, top: Top::Sampler(2) }));
    }

    #[test]
    fn decodes_swapped_halves() {
        // MOV R0.xyzw, TEX0 with the end bit: halves swapped in memory.
        let w0: u32 = (MOV as u32) << 24 | 0xf << 9 | 4 << 13 | 1;
        let w1: u32 = 1 | (0 << 9) | (1 << 11) | (2 << 13) | (3 << 15);
        let mut bytes = Vec::new();
        for w in [w0, w1, 0, 0] {
            bytes.extend_from_slice(&w.rotate_left(16).to_be_bytes());
        }
        let code = decode(&bytes);
        assert_eq!(code.len(), 1);
        assert_eq!(code[0].op, MOV);
        assert_eq!(code[0].sources[0].operand, Operand::Input(4));
        assert_eq!(code[0].sources[0].swizzle, [0, 1, 2, 3]);
    }
}
