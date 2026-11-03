// SPDX-License-Identifier: MPL-2.0
//! KCS natives that compute on VM memory without a task receiver.
use super::Ctx;
use anyhow::{ensure, Result};
use crysirror_core::{service::HostServices, state::ClientId};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[320];

#[derive(Default)]
pub(crate) struct State {}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    let a = cx.call.arguments.clone();
    match cx.call.id {
        // 659350: cubic spline coefficients over `count` VM records of five
        // floats {x, y, b, c, d}, rewritten in place.
        320 => {
            ensure!(a.len() == 2, "CatSystem2 spline arguments");
            let count = a[1] as i32;
            if count - 1 < 2 {
                cx.machine.preflight_native(false, &[])?;
                cx.machine.acknowledge(None)?;
                return Ok(true);
            }
            ensure!(count <= 1 << 20, "CatSystem2 spline point count");
            let bytes = cx.machine.read_bytes(a[0], count as usize * 20)?;
            let mut points: Vec<[f32; 5]> = bytes.chunks_exact(20).map(|r| core::array::from_fn(|i| f32::from_le_bytes(r[i * 4..i * 4 + 4].try_into().unwrap()))).collect();
            spline(&mut points);
            let out: Vec<u8> = points.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
            cx.machine.commit_native(None, &[(a[0], out)])?;
        }
        id => anyhow::bail!("CatSystem2 native {id} has no handler in this family"),
    }
    Ok(true)
}

/// 659350, single-precision as the original SSE code (squares through pow in
/// double). Forsythe-Malcolm-Moler end conditions, with the original's
/// x[3]-x[1] first-end denominator and an unscaled last c.
fn spline(p: &mut [[f32; 5]]) {
    const X: usize = 0;
    const Y: usize = 1;
    const B: usize = 2;
    const C: usize = 3;
    const D: usize = 4;
    let n = p.len();
    let last = n - 1;
    let m = n - 2;
    p[0][D] = p[1][X] - p[0][X];
    p[1][C] = (p[1][Y] - p[0][Y]) / p[0][D];
    for k in 1..=m {
        let h = p[k + 1][X] - p[k][X];
        p[k][D] = h;
        p[k][B] = (p[k - 1][D] + h) * 2.0;
        let slope = (p[k + 1][Y] - p[k][Y]) / h;
        p[k + 1][C] = slope;
        p[k][C] = slope - p[k][C];
    }
    p[0][B] = -p[0][D];
    p[last][B] = -p[last - 1][D];
    if n == 3 {
        p[0][C] = 0.;
        p[last][C] = 0.;
    } else {
        p[0][C] = p[2][C] / (p[3][X] - p[1][X]) - p[1][C] / (p[2][X] - p[0][X]);
        p[last][C] = p[last - 1][C] / (p[last][X] - p[last - 2][X]) - p[last - 2][C] / (p[last - 1][X] - p[last - 3][X]);
        let square = |h: f32| (f64::from(h) * f64::from(h)) as f32;
        p[0][C] = square(p[0][D]) * p[0][C] / (p[3][X] - p[1][X]);
        p[last][C] = -(square(p[last - 1][D]) * p[last][C] / (p[last][X] - p[last - 3][X]));
    }
    for i in 0..last {
        let t = p[i][D] / p[i][B];
        let carry = p[i][C] * t;
        p[i + 1][B] -= p[i][D] * t;
        p[i + 1][C] -= carry;
    }
    p[last][C] /= p[last][B];
    for i in (0..=m).rev() {
        p[i][C] = (p[i][C] - p[i][D] * p[i + 1][C]) / p[i][B];
    }
    p[last][B] = (p[last][Y] - p[m][Y]) / p[m][D] + (p[last][C] * 2.0 + p[m][C]) * p[m][D];
    for i in 0..=m {
        let (c, h, next) = (p[i][C], p[i][D], p[i + 1][C]);
        let b = (p[i + 1][Y] - p[i][Y]) / h - (c * 2.0 + next) * h;
        p[i][C] = c * 3.0;
        p[i][D] = (next - c) / h;
        p[i][B] = b;
    }
    p[last][D] = p[last - 1][D];
}

#[allow(unused_variables)]
pub(crate) fn tick(
    host: &mut dyn HostServices,
    client: ClientId,
    tasks: &mut crate::task_policy::NativeTaskPolicy,
    state: &mut State,
    now_ms: u64,
) -> Result<()> {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn retire(host: &mut dyn HostServices, client: ClientId, state: &mut State, id: u32) -> Result<()> {
    Ok(())
}
