// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native75AAC0/7693A0/76A470/76A570 alias argument projection. This is
//! original parameter/default/expression policy, never interpolation or drawing.
use crate::{fes_program::Alias, native_interpreter::Interpreter};
use anyhow::{ensure, Context, Result};
#[derive(Clone, Copy, Debug)]
pub(crate) struct Values {
    pub base: [f32; 3],
    pub position: [f32; 3],
    pub vram: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
    pub color: u32,
}
impl Values {
    ///76A070/769FC0 deliberately use different native class getters: image
    ///BX is pivot/priority, while Plane BX reads BASE0x40 and PX reads POS0x30.
    ///The labels must not be translated into a guessed generic transform.
    pub(crate) fn from_state(state: &crate::native_class_state::State) -> Self {
        use crate::native_class_state::State;
        let mut values = Self {
            base: [0.; 3],
            position: [0.; 3],
            vram: [0.; 3],
            rotation: [0.; 3],
            scale: [1.; 3],
            color: 0,
        };
        match state {
            State::Base => {}
            State::CImage(image) | State::Image(image) => {
                values.base = [
                    image.pose.pivot[0],
                    image.pose.pivot[1],
                    image.pose.priority as f32,
                ];
                values.position = [image.pose.position[0], image.pose.position[1], 0.];
                values.rotation[2] = image.pose.rotate;
                values.scale = [image.pose.scale[0], image.pose.scale[1], 1.];
                values.color = image.color;
            }
            State::CPlane(plane) | State::Plane(plane) => {
                values.base = plane.pose.base[..3].try_into().unwrap();
                values.position = plane.pose.position[..3].try_into().unwrap();
                values.rotation = plane.pose.euler.map(f32::from_bits);
                values.scale = plane.pose.scale[..3].try_into().unwrap();
                values.color = plane.pose.color;
                if matches!(state, State::Plane(_)) {
                    values.vram = plane.vram[..3].try_into().unwrap();
                }
            }
            State::Sound(sound) => values.color = (sound.pose.volume as u8 as u32) << 24,
        }
        values
    }
}
const KINDS: &[&[u8]] = &[
    b"p", b"BX", b"bx", b"BY", b"by", b"BZ", b"bz", b"PX", b"px", b"PY", b"py", b"PZ", b"pz",
    b"VX", b"vx", b"VY", b"vy", b"VZ", b"vz", b"RX", b"rx", b"RY", b"ry", b"RZ", b"rz", b"SX",
    b"sx", b"SY", b"sy", b"SZ", b"sz", b"CR", b"cr", b"CG", b"cg", b"CB", b"cb", b"CA", b"ca",
    b"CX", b"cx",
];
fn reference(token: &[u8]) -> Result<(&[u8], &[u8])> {
    ensure!(!token.contains(&0), "CatSystem2 alias argument NUL");
    if let Some(at) = token.iter().position(|b| *b == b'[') {
        let tail = &token[at + 1..];
        let end = tail
            .iter()
            .position(|b| *b == b']')
            .context("CatSystem2 alias argument closing bracket")?;
        ensure!(
            at < 32 && end < 32,
            "CatSystem2 alias argument original extent"
        );
        Ok((&token[..at], &tail[..end]))
    } else {
        ensure!(
            token.len() < 32,
            "CatSystem2 alias argument original extent"
        );
        Ok((token, b""))
    }
}
fn integer(raw: &[u8]) -> Result<u32> {
    ensure!(raw.is_ascii(), "CatSystem2 alias integer source profile");
    Ok(crate::native_values::integer(std::str::from_utf8(raw)?))
}
fn x86_default(value: f32) -> u32 {
    if (-2147483648.0..2147483648.0).contains(&value) {
        value as i32 as u32
    } else {
        i32::MIN as u32
    }
}
fn number(
    vm: &mut Interpreter,
    source: &[u8],
    fallback: &[u8],
    ratio: f32,
    default: f32,
    rotation: bool,
) -> Result<f32> {
    let source = if source.is_empty() { fallback } else { source };
    ensure!(
        source.len() < 64 && !source.contains(&0),
        "CatSystem2 alias expression64-byte profile"
    );
    let (source, percent) = source
        .strip_suffix(b"%")
        .map_or((source, false), |v| (v, true));
    let mut value = vm.evaluate_temporary_default(source, x86_default(default))? as i32 as f32;
    //Original MULSS then DIVSS, not degrees-to-radians default conversion.
    if rotation {
        value = (value * std::f32::consts::PI) / 180.0;
    }
    if percent {
        value = (value * ratio) / 100.0;
    }
    ensure!(value.is_finite(), "CatSystem2 alias finite result profile");
    Ok(value)
}
fn parameter(
    kind: usize,
    source: &[u8],
    fallback: &[u8],
    values: Values,
    vm: &mut Interpreter,
) -> Result<f32> {
    if kind == 0 {
        return number(vm, source, fallback, 0., 0., false);
    }
    if kind >= 39 {
        return Ok(0.);
    } //Native switch has no CX/cx case; zeroed v105 remains0 and no expression runs.
    let (current, ratio, delta, angle) = match kind {
        1..=18 => {
            let group = (kind - 1) / 6;
            let component = ((kind - 1) % 6) / 2;
            let vector = match group {
                0 => values.base,
                1 => values.position,
                _ => values.vram,
            };
            let value = vector[component];
            (value, value, kind % 2 == 0, false)
        }
        19..=24 => {
            let value = values.rotation[(kind - 19) / 2];
            (value, value, kind % 2 == 0, true)
        }
        25..=30 => {
            let value = values.scale[(kind - 25) / 2];
            (value, 1., kind % 2 == 0, false)
        }
        31..=38 => {
            let shift = [16, 8, 0, 24][(kind - 31) / 2];
            let value = ((values.color >> shift) & 255) as f32;
            (value, value, kind % 2 == 0, false)
        }
        _ => unreachable!(),
    };
    let mut out = number(vm, source, fallback, ratio, current, angle)?;
    if delta {
        out -= current;
    }
    Ok(out)
}
///Argument entries reference the declaration's parameter kind/default, not an
///arbitrary user-argument index. Each reference consumes the NEXT user word;
///colour constants/literals consume none. Repeated references therefore execute
///distinct actual user expressions, retaining their original order.
pub(crate) fn project(
    alias: &Alias,
    args: &[Vec<u8>],
    values: Values,
    vm: &mut Interpreter,
) -> Result<Vec<f32>> {
    ensure!(
        alias.defaults.len() <= 16 && alias.arguments.len() <= 16,
        "CatSystem2 alias argument count profile"
    );
    let mut working = vm.clone();
    let mut cursor = 0;
    let mut out = Vec::new();
    for token in &alias.arguments {
        let (name, index) = reference(token)?;
        let value = if name.is_empty() {
            ensure!(
                !index.is_empty(),
                "CatSystem2 alias empty uninitialized index unsupported"
            );
            let parameter_index = integer(index)? as usize;
            let (kind, default) = alias
                .defaults
                .get(parameter_index)
                .context("CatSystem2 alias parameter outside declaration")?;
            let kind = KINDS
                .iter()
                .position(|n| *n == kind.as_slice())
                .context("CatSystem2 unknown alias parameter kind")?;
            let source = args.get(cursor).map(Vec::as_slice).unwrap_or(default);
            cursor += 1;
            parameter(kind, source, default, values, &mut working)?
        } else {
            match name {
                b"cxr" => ((values.color >> 16) & 255) as f32,
                b"cxg" => ((values.color >> 8) & 255) as f32,
                b"cxb" => (values.color & 255) as f32,
                b"cxa" | b"CA" => (values.color >> 24) as f32,
                _ => integer(name)? as i32 as f32,
            }
        };
        out.push(value);
    }
    *vm = working;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn values() -> Values {
        Values {
            base: [17., 29., 41.],
            position: [53., 67., 79.],
            vram: [83., 97., 101.],
            rotation: [0.5, 1., 1.5],
            scale: [1.5, 2., 2.5],
            color: 0x7f123456,
        }
    }
    #[test]
    fn original_class_getter_labels_are_distinct_from_generic_position_and_alpha() {
        use crate::native_class_state::State;
        let mut state = State::new(2).unwrap();
        let State::Image(image) = &mut state else {
            unreachable!()
        };
        image.pose.position = [11., 13.];
        image.pose.pivot = [3., 5.];
        image.pose.priority = 27;
        image.pose.alpha = 81;
        image.color = 0x7f123456;
        let values = Values::from_state(&state);
        assert_eq!(values.base, [3., 5., 27.]);
        assert_eq!(values.position, [11., 13., 0.]);
        assert_eq!(
            values.color, 0x7f123456,
            "native colour getter is not the separate image draw alpha"
        );
        let mut state = State::new(4).unwrap();
        let State::Plane(plane) = &mut state else {
            unreachable!()
        };
        plane.pose.position = [2., 3., 4., 1.];
        plane.pose.base = [5., 6., 7., 1.];
        plane.vram = [8., 9., 10., 0.];
        let values = Values::from_state(&state);
        assert_eq!(values.base, [5., 6., 7.]);
        assert_eq!(values.position, [2., 3., 4.]);
        assert_eq!(values.vram, [8., 9., 10.]);
        let mut state = State::new(9).unwrap();
        let State::Sound(sound) = &mut state else {
            unreachable!()
        };
        sound.pose.volume = 383;
        assert_eq!(Values::from_state(&state).color, 0x7f000000);
    }
    #[test]
    fn references_consume_next_word_repeated_defaults_and_raw_constants_in_native_order(
    ) -> Result<()> {
        let mut vm = Interpreter::new(8)?;
        vm.set_default(99);
        let alias = Alias {
            mode: b"oneshot".to_vec(),
            name: b"test".to_vec(),
            resource: b"pBase".to_vec(),
            defaults: vec![
                (b"p".to_vec(), b"7".to_vec()),
                (b"PX".to_vec(), b"50%".to_vec()),
            ],
            arguments: vec![
                b"[1]".to_vec(),
                b"cxa".to_vec(),
                b"[0]".to_vec(),
                b"-1".to_vec(),
                b"[1]".to_vec(),
            ],
        };
        let out = project(
            &alias,
            &[b"++$0".to_vec(), b"++$0".to_vec()],
            values(),
            &mut vm,
        )?;
        assert_eq!(out, [1., 127., 2., -1., 26.5]);
        assert_eq!(vm.flag(0), 2);
        assert_eq!(vm.evaluate(b"@")?, 99);
        Ok(())
    }
    #[test]
    fn lower_case_delta_scale_percent_and_rotation_keep_original_default_and_float_order(
    ) -> Result<()> {
        let mut vm = Interpreter::new(8)?;
        assert_eq!(parameter(8, b"100%", b"0", values(), &mut vm)?, 0.);
        assert_eq!(parameter(26, b"100%", b"0", values(), &mut vm)?, -0.5);
        let angle = parameter(20, b"180", b"0", values(), &mut vm)?;
        assert_eq!(angle, std::f32::consts::PI - 0.5);
        assert_eq!(parameter(39, b"++$0", b"0", values(), &mut vm)?, 0.);
        assert_eq!(vm.flag(0), 0);
        Ok(())
    }
    #[test]
    fn a_later_invalid_reference_rolls_back_all_expression_changes() -> Result<()> {
        let mut vm = Interpreter::new(8)?;
        vm.set_default(91);
        let alias = Alias {
            mode: b"oneshot".to_vec(),
            name: b"test".to_vec(),
            resource: b"pBase".to_vec(),
            defaults: vec![(b"p".to_vec(), b"0".to_vec())],
            arguments: vec![b"[0]".to_vec(), b"[1]".to_vec()],
        };
        assert!(project(&alias, &[b"++$0".to_vec()], values(), &mut vm).is_err());
        assert_eq!(vm.flag(0), 0);
        assert_eq!(vm.evaluate(b"@")?, 91);
        Ok(())
    }
}
