// SPDX-License-Identifier: MPL-2.0
//! Motion native ABI over the workspace's shared glam math primitives.
//! Original row-vector matrix order and numeric VM addresses are policy here;
//! no renderer, image processing, retained animation or GPU state is embedded.
use crate::kcs::{Machine, NativeCall};
use anyhow::{ensure, Context, Result};
use glam::{Mat3, Mat4, Vec3, Vec4};

pub(crate) fn handles(id: u32) -> bool {
    matches!(id,18..=37|43..=44)
}
type Writes = Vec<(u32, Vec<u8>)>;
pub(crate) fn prepare(machine: &Machine, call: &NativeCall) -> Result<(Option<u32>, Writes)> {
    ensure!(
        machine.pending.as_ref() == Some(call),
        "CatSystem2 geometric request does not own the original pending native"
    );
    let a = &call.arguments;
    let count = match call.id {
        18..=20 | 23..=25 | 43 => 4,
        21..=22 => 6,
        26..=27 | 31 | 34..=35 => 1,
        28..=30 | 32..=33 | 36..=37 => 2,
        44 => 3,
        _ => anyhow::bail!("unsupported CatSystem2 motion geometric native {}", call.id),
    };
    ensure!(
        a.len() == count,
        "CatSystem2 motion geometric argument count"
    );
    let mut writes = Vec::new();
    let mut result = None;
    let float = |index| f32::from_bits(a[index]);
    match call.id {
        18..=20 => {
            validate_word_aliases(&a[..3])?;
            let value = Vec3::new(
                read_floats::<1>(machine, a[0])?[0],
                read_floats::<1>(machine, a[1])?[0],
                read_floats::<1>(machine, a[2])?[0],
            );
            let angle = float(3);
            ensure!(
                angle.is_finite(),
                "CatSystem2 vector rotation angle nonfinite"
            );
            //67E910/67E9A0/67EA30 use the opposite angle from the matrix
            //helpers. E.g Z writes x*c+y*s,y*c-x*s. Do not unify their sign.
            let matrix = match call.id {
                18 => Mat3::from_rotation_x(-angle),
                19 => Mat3::from_rotation_y(-angle),
                _ => Mat3::from_rotation_z(-angle),
            };
            let value = (matrix * value).to_array();
            for (address, value) in a[..3].iter().zip(value) {
                writes.push((*address, bytes(&[value])?));
            }
        }
        21..=22 => {
            validate_word_aliases(&a[..3])?;
            //Original stores x/y/z sequentially. Exact pointer aliases use
            //the preceding write; partial unaligned aliases fail explicitly.
            let mut prior = std::collections::BTreeMap::new();
            for index in 0..3 {
                let address = a[index];
                let value = if let Some(value) = prior.get(&address) {
                    *value
                } else {
                    read_floats::<1>(machine, address)?[0]
                };
                let operand = float(index + 3);
                ensure!(operand.is_finite(), "CatSystem2 vector delta nonfinite");
                let value = if call.id == 21 {
                    value + operand
                } else {
                    value * operand
                };
                writes.push((address, bytes(&[value])?));
                prior.insert(address, value);
            }
        }
        23..=25 | 43 => {
            let current = matrix(machine, a[0])?;
            let values = Vec3::new(float(1), float(2), float(3));
            ensure!(values.is_finite(), "CatSystem2 matrix arguments nonfinite");
            //Native row matrices are stored in the same array order as
            //column-major glam transposes. Postmultiply therefore reverses
            //the operand order, including the original X thenY thenZ sequence.
            let factor = match call.id {
                23 => {
                    Mat4::from_rotation_z(values.z)
                        * Mat4::from_rotation_y(values.y)
                        * Mat4::from_rotation_x(values.x)
                }
                24 => Mat4::from_translation(values),
                25 => Mat4::from_scale(values),
                //D3DX yaw(Y),pitch(X),roll(Z): roll is applied first.
                _ => {
                    Mat4::from_rotation_y(values.x)
                        * Mat4::from_rotation_x(values.y)
                        * Mat4::from_rotation_z(values.z)
                }
            };
            writes.push((a[0], bytes(&(factor * current).to_cols_array())?));
        }
        26 => writes.push((a[0], bytes(&Mat4::IDENTITY.to_cols_array())?)),
        27 => {
            let matrix = matrix(machine, a[0])?;
            ensure!(
                matrix.determinant().is_finite() && matrix.determinant() != 0.,
                "CatSystem2 singular/nonfinite native inverse profile"
            );
            writes.push((a[0], bytes(&matrix.inverse().to_cols_array())?));
        }
        28 => {
            no_partial_overlap(a[0], 64, a[1], 64)?;
            //Native SIMD copies preserve raw bit patterns, including NaNs.
            writes.push((
                a[0],
                read_words::<16>(machine, a[1])?
                    .into_iter()
                    .flat_map(u32::to_le_bytes)
                    .collect(),
            ));
        }
        29..=30 => {
            no_partial_overlap(a[0], 64, a[1], 64)?;
            if call.id == 29 {
                writes.push((
                    a[0],
                    bytes(&(matrix(machine, a[1])? * matrix(machine, a[0])?).to_cols_array())?,
                ));
            } else {
                let left = read_floats::<16>(machine, a[0])?;
                let right = read_floats::<16>(machine, a[1])?;
                writes.push((
                    a[0],
                    bytes(&std::array::from_fn::<_, 16, _>(|i| left[i] - right[i]))?,
                ));
            }
        }
        31 => {
            let value = Vec4::from_array(read_floats::<4>(machine, a[0])?);
            //Original scalar dispatch4D15E2 first rounds the four-product
            //sum to f32, copies vectors within epsilon of unit length, and
            //writes zero below/equal MIN_POSITIVE. No zero-vector NaN guess.
            let value = value.to_array();
            let squared = value
                .iter()
                .map(|v| f64::from(*v) * f64::from(*v))
                .sum::<f64>() as f32;
            ensure!(squared.is_finite(), "CatSystem2 vector length nonfinite");
            let normalized = if (f64::from(squared) - 1.).abs() <= f64::from(f32::EPSILON) {
                value
            } else if squared <= f32::MIN_POSITIVE {
                [0.; 4]
            } else {
                let inverse = 1. / f64::from(squared).sqrt();
                value.map(|v| (f64::from(v) * inverse) as f32)
            };
            writes.push((a[0], bytes(&normalized)?));
        }
        32..=33 => {
            no_partial_overlap(a[0], 16, a[1], 16)?;
            let left = read_floats::<4>(machine, a[0])?;
            let right = read_floats::<4>(machine, a[1])?;
            let value = std::array::from_fn::<_, 4, _>(|i| {
                if call.id == 32 {
                    left[i] + right[i]
                } else {
                    left[i] - right[i]
                }
            });
            writes.push((a[0], bytes(&value)?));
        }
        34..=35 => {
            let value = read_floats::<4>(machine, a[0])?;
            //67E840's ordered scalar sum is four-dimensional, not Vec3.
            let squared = ((value[0] * value[0] + value[1] * value[1]) + value[2] * value[2])
                + value[3] * value[3];
            let value = if call.id == 34 {
                (f64::from(squared).sqrt()) as f32
            } else {
                squared
            };
            result = Some(value.to_bits());
        }
        36 => {
            let value = Vec4::from_array(read_floats::<4>(machine, a[0])?);
            //78B1B0 reads the scalar through its SECOND numeric pointer.
            let scale = read_floats::<1>(machine, a[1])?[0];
            writes.push((a[0], bytes(&(value * scale).to_array())?));
        }
        37 => {
            let value = Vec4::from_array(read_floats::<4>(machine, a[0])?);
            writes.push((a[0], bytes(&(matrix(machine, a[1])? * value).to_array())?));
        }
        44 => {
            let current = matrix(machine, a[0])?;
            let basis = matrix(machine, a[1])?;
            let angles = read_floats::<3>(machine, a[2])?;
            let mut x = basis * (-Vec4::X);
            x.y = 0.;
            let z = basis * (-Vec4::Z);
            ensure!(
                x.is_finite()
                    && z.is_finite()
                    && x.truncate().length_squared() > 0.
                    && z.truncate().length_squared() > 0.,
                "CatSystem2 native local rotation has a singular/nonfinite axis"
            );
            let rx = Mat4::from_axis_angle(x.truncate().normalize(), angles[0]);
            let ry = Mat4::from_axis_angle(-Vec3::Y, angles[1]);
            let rz = Mat4::from_axis_angle(z.truncate().normalize(), angles[2]);
            writes.push((a[0], bytes(&(rz * ry * rx * current).to_cols_array())?));
        }
        _ => unreachable!(),
    }
    //All reads and outputs are validated before the first VM write/ACK.
    machine.preflight_native(result.is_some(), &writes)?;
    Ok((result, writes))
}
fn read_words<const N: usize>(machine: &Machine, address: u32) -> Result<[u32; N]> {
    let mut out = [0; N];
    for (index, word) in out.iter_mut().enumerate() {
        let at = address
            .checked_add(index as u32 * 4)
            .context("CatSystem2 motion numeric address overflow")?;
        *word = machine.read_word(at)?;
    }
    Ok(out)
}
fn read_floats<const N: usize>(machine: &Machine, address: u32) -> Result<[f32; N]> {
    let value = read_words::<N>(machine, address)?.map(f32::from_bits);
    ensure!(
        value.iter().all(|v| v.is_finite()),
        "CatSystem2 motion nonfinite geometric input profile"
    );
    Ok(value)
}
fn matrix(machine: &Machine, address: u32) -> Result<Mat4> {
    Ok(Mat4::from_cols_array(&read_floats::<16>(machine, address)?))
}
fn bytes(value: &[f32]) -> Result<Vec<u8>> {
    ensure!(
        value.iter().all(|v| v.is_finite()),
        "CatSystem2 motion geometric output nonfinite"
    );
    Ok(value
        .iter()
        .flat_map(|v| v.to_bits().to_le_bytes())
        .collect())
}
fn no_partial_overlap(left: u32, left_size: u32, right: u32, right_size: u32) -> Result<()> {
    ensure!(
        left == right
            || u64::from(left) >= u64::from(right) + u64::from(right_size)
            || u64::from(right) >= u64::from(left) + u64::from(left_size),
        "CatSystem2 unsupported partially overlapping native motion operands"
    );
    Ok(())
}
fn validate_word_aliases(addresses: &[u32]) -> Result<()> {
    for (index, left) in addresses.iter().enumerate() {
        for right in &addresses[index + 1..] {
            no_partial_overlap(*left, 4, *right, 4)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kcs::{Event, Program};
    const P: u32 = 0x80000000;
    fn operand(code: &mut Vec<u8>, value: u32) {
        code.push(3);
        code.extend(value.to_le_bytes());
    }
    fn pending(id: u32, args: &[u32]) -> (Machine, NativeCall) {
        let mut code = Vec::new();
        for arg in args {
            code.push(0x25);
            operand(&mut code, *arg);
        }
        code.push(0x2b);
        operand(&mut code, id);
        operand(&mut code, args.len() as u32 * 4);
        code.push(0x3b);
        let program = Program {
            code,
            globals_bytes: 0,
            stack_bytes: 1024,
            strings: Vec::new(),
        };
        let mut machine = Machine::new(&program, 1192).unwrap();
        let Event::Native(call) = machine.step(&program, 100).unwrap() else {
            panic!()
        };
        (machine, call)
    }
    fn write(machine: &mut Machine, address: u32, values: &[f32]) {
        machine
            .write_bytes(address, &bytes(values).unwrap())
            .unwrap();
    }
    fn apply(machine: &mut Machine, call: &NativeCall) -> Option<u32> {
        let (result, writes) = prepare(machine, call).unwrap();
        machine.commit_native(result, &writes).unwrap();
        result
    }
    fn almost(left: &[f32], right: &[f32]) {
        for (index, (a, b)) in left.iter().zip(right).enumerate() {
            assert!((a - b).abs() < 1e-5, "component{index}: {a} != {b}");
        }
    }
    #[test]
    fn row_vector_postmultiply_scales_translation_and_vec4_keeps_w() {
        let (matrix_vm, call) = pending(25, &[P, 2f32.to_bits(), 3f32.to_bits(), 4f32.to_bits()]);
        let mut machine = matrix_vm;
        write(
            &mut machine,
            P,
            &Mat4::from_translation(Vec3::new(2., 3., 4.)).to_cols_array(),
        );
        apply(&mut machine, &call);
        let scaled = read_floats::<16>(&machine, P).unwrap();
        assert_eq!(&scaled[12..], [4., 9., 16., 1.]);
        let (mut machine, call) = pending(37, &[P + 80, P]);
        write(&mut machine, P, &scaled);
        write(&mut machine, P + 80, &[1., 0., 0., 1.]);
        apply(&mut machine, &call);
        assert_eq!(
            read_floats::<4>(&machine, P + 80).unwrap(),
            [6., 9., 16., 1.]
        );
    }
    #[test]
    fn vector_axis_rotation_has_opposite_sign_and_exact_aliases_are_sequential() {
        let (mut machine, call) = pending(
            20,
            &[P, P + 4, P + 8, std::f32::consts::FRAC_PI_2.to_bits()],
        );
        write(&mut machine, P, &[1., 0., 0.]);
        apply(&mut machine, &call);
        almost(&read_floats::<3>(&machine, P).unwrap(), &[0., -1., 0.]);
        let (mut machine, call) = pending(
            21,
            &[P, P, P, 1f32.to_bits(), 2f32.to_bits(), 3f32.to_bits()],
        );
        write(&mut machine, P, &[1.]);
        apply(&mut machine, &call);
        assert_eq!(read_floats::<1>(&machine, P).unwrap(), [7.]);
        let (mut machine, call) = pending(23, &[P, 0, 0, std::f32::consts::FRAC_PI_2.to_bits()]);
        write(&mut machine, P, &Mat4::IDENTITY.to_cols_array());
        apply(&mut machine, &call);
        let value = matrix(&machine, P).unwrap() * Vec4::X;
        almost(&value.to_array(), &[0., 1., 0., 0.]);
    }
    #[test]
    fn normalization_length_and_raw_copy_preserve_four_word_contract() {
        let (mut machine, call) = pending(31, &[P]);
        write(&mut machine, P, &[3., 0., 0., 4.]);
        apply(&mut machine, &call);
        almost(&read_floats::<4>(&machine, P).unwrap(), &[0.6, 0., 0., 0.8]);
        let (mut machine, call) = pending(31, &[P]);
        write(&mut machine, P, &[0.; 4]);
        apply(&mut machine, &call);
        assert_eq!(read_floats::<4>(&machine, P).unwrap(), [0.; 4]);
        let (mut machine, call) = pending(34, &[P]);
        write(&mut machine, P, &[3., 0., 0., 4.]);
        assert_eq!(f32::from_bits(apply(&mut machine, &call).unwrap()), 5.);
        let (mut machine, call) = pending(28, &[P, P + 64]);
        let raw: [u32; 16] = std::array::from_fn(|i| 0x7fc01234 + i as u32);
        machine
            .write_bytes(
                P + 64,
                &raw.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
            )
            .unwrap();
        apply(&mut machine, &call);
        assert_eq!(read_words::<16>(&machine, P).unwrap(), raw);
    }
    #[test]
    fn unhandled_alias_singular_inverse_and_foreign_pending_do_not_write_or_ack() {
        let (mut machine, call) = pending(32, &[P, P + 4]);
        write(&mut machine, P, &[1., 2., 3., 4., 5.]);
        let before = machine.heap.clone();
        assert!(prepare(&machine, &call).is_err());
        assert_eq!(machine.heap, before);
        assert_eq!(machine.pending.as_ref(), Some(&call));
        let (mut machine, call) = pending(27, &[P]);
        write(&mut machine, P, &[0.; 16]);
        let before = machine.heap.clone();
        assert!(prepare(&machine, &call).is_err());
        assert_eq!(machine.heap, before);
        assert_eq!(machine.pending.as_ref(), Some(&call));
        let mut wrong = call.clone();
        wrong.id = 26;
        assert!(prepare(&machine, &wrong).is_err());
    }
}
