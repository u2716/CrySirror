// SPDX-License-Identifier: MPL-2.0
//! KCS natives that compute on VM memory without a task receiver.
use super::Ctx;
use anyhow::{ensure, Context, Result};
use crysirror_core::{service::HostServices, state::ClientId};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[168, 279, 280, 281, 282, 283, 287, 288, 320, 347, 348, 570, 611, 612, 613, 614, 615, 617, 619, 620, 621, 567, 600, 601, 904, 1018, 1019, 1020, 1021, 1022, 1023, 1036];

/// One channel's source as `<use3d>` natives left it.
struct Spatial {
    position: [f32; 3],
    minimum: f32,
}
impl Spatial {
    /// Stereo balance (0..1) and attenuation for a listener at the origin.
    fn heard(&self) -> (f32, f32) {
        let [x, y, z] = self.position;
        let distance = (x * x + y * y + z * z).sqrt();
        if distance <= self.minimum {
            return (0.5 + if distance > 0. { x / distance * 0.5 } else { 0. }, 1.);
        }
        (0.5 + x / distance * 0.5, self.minimum / distance)
    }
}

#[derive(Default)]
pub(crate) struct State {
    debug_window_noted: bool,
    spatial: std::collections::BTreeMap<u32, Spatial>,
}

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
        // 645C20/645CB0/6458A0 → SOUND 193/195/196 → 5CC2E0/5CC370/5CC1F0: DirectSound3D
        // position, velocity and distance on channel +0x6C, which only exists for buffers
        // made under <SOUND><use3d>; without it the original changes nothing. The host mixes
        // in stereo, so a listener at the origin hears the source through pan and the inverse
        // distance law with DirectSound's default minimum of 1. Velocity only drives Doppler,
        // which the host has no control for.
        168 | 347 | 348 => {
            cx.machine.preflight_native(false, &[])?;
            let use3d = cx.settings.as_ref().map_or(0, |settings| settings.use_3d_sound);
            if use3d != 0 && !a.is_empty() {
                let channel = a[0];
                let source = cx.state.subsystems.spatial.entry(channel).or_insert(Spatial { position: [0.; 3], minimum: 1. });
                match cx.call.id {
                    168 if a.len() >= 4 => source.position = [f32::from_bits(a[1]), f32::from_bits(a[2]), f32::from_bits(a[3])],
                    348 if a.len() >= 2 => source.minimum = f32::from_bits(a[1]).max(f32::MIN_POSITIVE),
                    _ => {}
                }
                let (pan, gain) = source.heard();
                if cx.call.id != 347 && cx.host.audio_info(cx.client, channel)?.is_some() {
                    cx.host.submit_audio(cx.client, crysirror_core::service::AudioRequest::SetPan { channel, pan, fade_ms: 0. })?;
                    cx.host.submit_audio(cx.client, crysirror_core::service::AudioRequest::SetStereoGain { channel, gains: [gain; 2], fade_ms: 0. })?;
                }
            }
            cx.machine.acknowledge(None)?;
        }
        // Classes 32 ("DebugDlg", 618330), 33 ("scene list", 625FA0) and 47 (627630, 442): the
        // original runs a Win32 developer dialog on its own thread (messages 322..329, 442
        // only create, show and feed that window). This player has no such tool window; nothing in the game reads
        // from it, so the calls end here, noted once.
        279 | 280 | 281 | 282 | 283 | 287 | 288 | 570 => {
            cx.machine.preflight_native(false, &[])?;
            if !core::mem::replace(&mut cx.state.subsystems.debug_window_noted, true) {
                cx.host.append_log(cx.client, "CatSystem2 developer debug windows (classes 32/33/47) are not presented")?;
            }
            cx.machine.acknowledge(None)?;
        }
        // Class 48 text plane (634770). Messages 449..451 demand a block whose first word
        // is 232; Nekopara vol3's Select.kcs passes 196, so 450 returns before message 41
        // ever builds a target and every later message has nothing to draw into. A block
        // of the expected size would need the 696880 text engine, which is not ported.
        611 | 612 | 613 => {
            ensure!(a.len() == 2, "CatSystem2 text plane config arguments");
            cx.machine.preflight_native(false, &[])?;
            let size = cx.machine.read_word(a[1])?;
            ensure!(size != 232 || cx.tasks.opaque_class(a[0]) != Some(48), "CatSystem2 class 48 text plane with a 232-byte config needs the 696880 text engine");
            cx.machine.acknowledge(None)?;
        }
        614 | 615 | 617 | 619 | 620 | 621 => {
            cx.machine.preflight_native(false, &[])?;
            cx.machine.acknowledge(None)?;
        }
        // 6502B0: append the formatted text and CR LF to a file in the game folder.
        567 => {
            ensure!(a.len() == 2, "CatSystem2 text file append arguments");
            cx.machine.preflight_native(false, &[])?;
            let name = String::from_utf8(cx.machine.read_cstring(a[0], 260)?).context("CatSystem2 text file name")?;
            let text = crate::native_values::format_bytes(cx.machine, a[1])?;
            let path = crysirror_core::path::VfsPath::new(crysirror_core::path::Mount::Save, &name.replace('\\', "/"))?;
            let mut bytes = match cx.host.read(cx.client, &path) {
                Ok(bytes) => bytes,
                Err(crysirror_core::Error::NotFound) => Vec::new(),
                Err(error) => return Err(error.into()),
            };
            bytes.extend_from_slice(&text);
            bytes.extend_from_slice(b"\r\n");
            cx.host.write_atomic(cx.client, &path, &bytes)?;
            cx.machine.acknowledge(None)?;
        }
        // 64FF30: comma list a1 is written over list a0; an "@" keeps a0's token at that
        // place, or "0" when a0 has none. 6A5AB0 drops empty tokens.
        904 => {
            ensure!(a.len() == 2, "CatSystem2 list merge arguments");
            let old = cx.machine.read_cstring(a[0], 65534)?;
            let template = crate::native_values::format_bytes(cx.machine, a[1])?;
            let tokens = |bytes: &[u8]| -> Vec<Vec<u8>> { bytes.split(|b| *b == b',').filter(|t| !t.is_empty()).map(<[u8]>::to_vec).collect() };
            let (kept, wanted) = (tokens(&old), tokens(&template));
            let merged: Vec<Vec<u8>> = wanted.iter().enumerate().map(|(i, token)| if token == b"@" { kept.get(i).cloned().unwrap_or_else(|| b"0".to_vec()) } else { token.clone() }).collect();
            let mut out = merged.join(&b',');
            out.push(0);
            cx.machine.commit_native(None, &[(a[0], out)])?;
        }
        // Classes 44 ChangeWP (633640), 46 ChangeSnd (634250) and 62 Tweet (63D4D0) run one
        // worker that changes the Windows wallpaper or sound scheme, or posts to a service
        // that no longer exists (Siglus treats its dialog the same way). The request is
        // logged and ends at once; the tweet's capture (1009) is still written.
        600 | 1021 => {
            cx.machine.preflight_native(false, &[])?;
            cx.host.append_log(cx.client, &format!("CatSystem2 native {}: host system integration is not performed", cx.call.id))?;
            cx.machine.acknowledge(None)?;
        }
        // 64DAB0 → 448: 1 once no worker runs.
        601 => {
            cx.machine.preflight_native(true, &[])?;
            cx.machine.acknowledge(Some(1))?;
        }
        // Tweet account, attachment and text fields only feed that worker.
        1018 | 1019 | 1020 | 1023 => {
            cx.machine.preflight_native(false, &[])?;
            cx.machine.acknowledge(None)?;
        }
        // 6541C0 → 578: the worker's result this[541], which stays 0.
        1022 => {
            cx.machine.preflight_native(true, &[])?;
            cx.machine.acknowledge(Some(0))?;
        }
        // 6471D0: main and sub text language prefixes into two VM buffers.
        1036 => {
            ensure!(a.len() == 2, "CatSystem2 language prefix arguments");
            let settings = cx.settings.as_ref().context("CatSystem2 language table unavailable")?;
            let writes: Vec<(u32, Vec<u8>)> = [1, 2].iter().zip(&a).map(|(slot, at)| {
                let mut bytes = settings.language_prefix_of(*slot).to_vec();
                bytes.push(0);
                (*at, bytes)
            }).collect();
            cx.machine.commit_native(None, &writes)?;
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

#[cfg(test)]
mod spatial_tests {
    use super::Spatial;

    #[test]
    fn distance_attenuates_past_the_minimum_and_x_pans() {
        assert_eq!(Spatial { position: [0., 0., 0.], minimum: 1. }.heard(), (0.5, 1.));
        assert_eq!(Spatial { position: [4., 0., 0.], minimum: 1. }.heard(), (1., 0.25));
        assert_eq!(Spatial { position: [0., 0., 2.], minimum: 1. }.heard(), (0.5, 0.5));
    }
}

