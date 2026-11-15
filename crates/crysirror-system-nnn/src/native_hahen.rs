// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! HAHEN81's parameter, cached partition and analytic fragment-motion policy.
//! Complete effectHahen.cpp/header read. Calcu starts with return; the later
//! incremental integration is dead code. A shared worker must rasterize these
//! ordered polygon requests (native inclusive/f32 edge and linear-row profile).
use crate::native_hahen_partition::{Key, Partition, Partitions};
use anyhow::{Result, ensure};
use crysirror_image::temporal_pattern::PatternRng;

pub(crate) fn parameters(values: &[i32], canvas: [u32; 2]) -> [i32; 18] {
    let defaults = [
        20,
        100,
        100,
        canvas[0] as i32 / 2,
        canvas[1] as i32 / 2,
        0,
        100,
        0,
        0,
        100,
        100,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let mut p = std::array::from_fn(|i| {
        values.get(i).copied().unwrap_or(if i == 10 {
            values.get(9).copied().unwrap_or(defaults[9])
        } else {
            defaults[i]
        })
    });
    p[6] = p[6].min(p[0]);
    if p[13] == 0 {
        p[14..18].fill(0)
    } else if p[13] == 2 {
        p[15..18].fill(0)
    }
    p
}
pub(crate) fn validate(p: &[i32; 18], frames: i32) -> Result<()> {
    ensure!(
        p.iter().all(|v| v.unsigned_abs() <= 1_000_000),
        "NNN HAHEN parameter budget"
    );
    crate::native_curve::validate(frames, p[8], p[7])?;
    crate::native_curve::validate(frames, p[12], p[11])
}
#[derive(Clone, Copy, Default)]
struct Fragment {
    speed: [f32; 2],
    acceleration_y: f32,
}
#[derive(Clone)]
pub(crate) struct HahenClass {
    partitions: Partitions,
    physics: Vec<Fragment>,
    canvas: [u32; 2],
}
pub(crate) struct Polygon {
    /// Original source-space vertices in authored order, without triangulation.
    pub vertices: Vec<[i32; 2]>,
    pub destination_offset: [i32; 2],
}
pub(crate) struct Paint {
    pub polygons: Vec<Polygon>,
    pub weight: i32,
}
impl HahenClass {
    pub(crate) fn new(canvas: [u32; 2]) -> Result<Self> {
        Ok(Self {
            partitions: Partitions::new(canvas)?,
            physics: vec![Fragment::default(); 32],
            canvas,
        })
    }
    pub(crate) fn needs_partition(&self, p: &[i32; 18]) -> bool {
        self.partitions.get(Key::from_parameters(p)).is_none()
    }
    /// The resource/generation worker resolves only a cache miss. None means
    /// the original default fallback for unknown type, missing or bad file.
    /// Raw InitParameter replays this setter; do not merely copy saved words.
    pub(crate) fn set(&mut self, p: &[i32; 18], generated: Option<Partition>) -> Result<()> {
        validate(p, p[0])?;
        let key = Key::from_parameters(p);
        ensure!(
            self.partitions.get(key).is_some()
                || key.0[0] != 1
                || !(0..=2).contains(&key.0[1])
                || generated.is_some(),
            "NNN HAHEN procedural partition worker not completed"
        );
        let partition = self.partitions.insert(Key::from_parameters(p), generated)?;
        self.physics.resize(
            self.physics.len().max(partition.faces.len()),
            Fragment::default(),
        );
        for (i, face) in partition.faces.iter().enumerate() {
            let mut center = [0i32; 2];
            // Source surfaceBuffer[i*16+1+j] starts at the face COUNT, not its
            // first index. Preserve this count-as-index centroid and omit the
            // last real vertex. For a custom face whose count is not a valid
            // vertex, replace its old out-of-bounds read by bounded vertex 0.
            for index in
                std::iter::once(face.len() as u32).chain(face.iter().copied().take(face.len() - 1))
            {
                let vertex = partition
                    .vertices
                    .get(index as usize)
                    .unwrap_or(&partition.vertices[0]);
                for axis in 0..2 {
                    center[axis] = center[axis].wrapping_add(vertex[axis]);
                }
            }
            for coordinate in &mut center {
                *coordinate /= face.len() as i32;
            }
            let delta = [center[0].wrapping_sub(p[3]), center[1].wrapping_sub(p[4])];
            let radius_squared = delta[0]
                .wrapping_mul(delta[0])
                .wrapping_add(delta[1].wrapping_mul(delta[1]));
            // A signed overflow made sqrt/float conversion undefined in the
            // original. Keep the ordinary native math; bound that case to 0.
            let radius = f64::from(radius_squared.max(0)).sqrt().max(1.0);
            let mut velocity = f64::from(p[1]);
            velocity *= 0.01;
            velocity *= 30.0;
            velocity *= 1.0 + radius / 200.0;
            let angle = f64::from(delta[1]).atan2(f64::from(delta[0]));
            self.physics[i] = Fragment {
                speed: [
                    (angle.cos() * velocity) as f32,
                    (angle.sin() * velocity) as f32,
                ],
                acceleration_y: p[2] as f32 * 0.01_f32,
            };
        }
        Ok(())
    }
    pub(crate) fn paint(
        &self,
        p: &[i32; 18],
        count: i32,
        frames: i32,
        picture_buffer_present: bool,
        random: &mut PatternRng,
    ) -> Result<Option<Paint>> {
        // Source GetBuffer(), not pic==-1, loadOk, mask or CharaClip, guards
        // both curve calls. A retained buffer after failed reload still draws.
        if !picture_buffer_present {
            return Ok(None);
        }
        validate(p, frames)?;
        let phase =
            crate::native_curve::value_with_random(count, frames, 0, frames, p[8], p[7], random)?;
        let weight = crate::native_curve::value_with_random(
            count,
            frames,
            p[10].wrapping_mul(256),
            p[9].wrapping_mul(256),
            p[12],
            p[11],
            random,
        )? / 100;
        let Some(partition) = self.partitions.get(Key::from_parameters(p)) else {
            return Ok(None);
        };
        let mut polygons = Vec::with_capacity(partition.faces.len());
        for (i, face) in partition.faces.iter().enumerate() {
            // Any valid Print has a preceding SetParam. Explicitly reject a
            // malformed restore rather than indexing uninitialized physics.
            let f = self
                .physics
                .get(i)
                .ok_or_else(|| anyhow::anyhow!("NNN HAHEN fragment state missing"))?;
            let hibi = phase.min(p[5]);
            let mut start = f64::from(hibi.wrapping_mul(p[6]));
            start *= f64::from(0.001_f32);
            let mut offset = [start * f64::from(f.speed[0]), start * f64::from(f.speed[1])];
            let dt = phase.wrapping_sub(p[5]);
            if dt > 0 {
                let t = f64::from(dt);
                offset[0] += f64::from(f.speed[0]) * t;
                offset[1] += f64::from(f.speed[1]) * t;
                offset[1] += 0.5 * f64::from(f.acceleration_y) * t * t;
            }
            if offset.iter().zip(self.canvas).any(|(v, size)| {
                !v.is_finite() || *v < -f64::from(size) || *v > f64::from(size) * 2.0
            }) {
                continue;
            }
            // This is +0.5 then truncation even for negative positions.
            let offset = offset.map(|n| (n + 0.5) as i32);
            let vertices = face
                .iter()
                .map(|n| partition.vertices[*n as usize])
                .collect::<Vec<_>>();
            let min = std::array::from_fn::<_, 2, _>(|axis| {
                vertices.iter().map(|v| v[axis]).min().unwrap()
            });
            let max = std::array::from_fn::<_, 2, _>(|axis| {
                vertices.iter().map(|v| v[axis]).max().unwrap()
            });
            if (0..2).any(|axis| {
                i64::from(offset[axis]) + i64::from(min[axis]) >= i64::from(self.canvas[axis])
                    || i64::from(offset[axis]) + i64::from(max[axis]) < 0
            }) {
                continue;
            }
            polygons.push(Polygon {
                vertices,
                destination_offset: offset,
            });
        }
        Ok(Some(Paint { polygons, weight }))
    }
}
