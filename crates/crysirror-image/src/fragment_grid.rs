// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Ordered jittered screen partitions for image-fragment effects. The complete
//! geometry is prepared on a worker and returns its random state atomically.
use crate::temporal_pattern::PatternRng;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FragmentGridStyle {
    StaggeredTriangles,
    Quadrilaterals,
    StaggeredStrips,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FragmentGrid {
    pub size: [u32; 2],
    pub subdivisions: [u32; 2],
    pub style: FragmentGridStyle,
    /// Native tenths of one base cell; zero suppresses all jitter draws.
    pub jitter_tenths: u32,
}
#[derive(Clone, Debug)]
pub struct FragmentMesh {
    pub vertices: Vec<[i32; 2]>,
    pub faces: Vec<Vec<u32>>,
}
impl FragmentGrid {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.size.iter().all(|n| (1..=16384).contains(n))
                && self.subdivisions.iter().all(|n| (1..=1024).contains(n))
                && self.jitter_tenths <= 4,
            "fragment grid domain"
        );
        let [x, y] = self.subdivisions.map(u64::from);
        let (vertices, faces) = match self.style {
            FragmentGridStyle::StaggeredTriangles => {
                ensure!(y % 2 == 0, "staggered triangle even rows");
                ((x + 2) * (y + 1), (x * 2 + 1) * y)
            }
            FragmentGridStyle::Quadrilaterals => ((x + 1) * (y + 1), x * y),
            FragmentGridStyle::StaggeredStrips => {
                ensure!(y % 2 == 0, "staggered strip even rows");
                ((x + 2) * y * 2, (x * 2 + 1) * y / 2)
            }
        };
        ensure!(
            vertices <= 4096 && faces <= 4096,
            "fragment grid geometry budget"
        );
        Ok(())
    }
    pub fn prepare(
        &self,
        random: &mut PatternRng,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<FragmentMesh> {
        self.validate()?;
        random.validate()?;
        check()?;
        let mut candidate = random.clone();
        let [nx, ny] = self.subdivisions.map(|n| n as i32);
        let [width, height] = self.size.map(|n| n as i32);
        let base = [width / nx, height / ny];
        let mut radius = base.map(|n| n * self.jitter_tenths as i32 / 10);
        if matches!(self.style, FragmentGridStyle::StaggeredStrips) {
            radius[0] /= 2;
        }
        let mut jitter = |axis: usize| -> Result<i32> {
            if self.jitter_tenths == 0 {
                Ok(0)
            } else {
                Ok((candidate.next_draw()? % (radius[axis] * 2 + 1) as u32) as i32 - radius[axis])
            }
        };
        let mut vertices = Vec::<[i32; 2]>::new();
        let mut faces = Vec::<Vec<u32>>::new();
        match self.style {
            FragmentGridStyle::StaggeredTriangles | FragmentGridStyle::Quadrilaterals => {
                let staggered = matches!(self.style, FragmentGridStyle::StaggeredTriangles);
                let last_x = nx + i32::from(staggered);
                for j in 0..=ny {
                    check()?;
                    for i in 0..=last_x {
                        let mut x = width * i / nx;
                        let mut y = height * j / ny;
                        if staggered && j & 1 != 0 {
                            x -= base[0] / 2;
                        }
                        // Both draws precede the boundary overwrites. Even
                        // a zero radius still draws modulo1 when roughness>0.
                        x += jitter(0)?;
                        y += jitter(1)?;
                        if j == 0 {
                            y = 0;
                        }
                        if j == ny {
                            y = height - 1;
                        }
                        if i == 0 {
                            x = 0;
                        }
                        if if staggered {
                            if j & 1 == 0 { i >= nx } else { i == nx + 1 }
                        } else {
                            i == nx
                        } {
                            x = width - 1;
                        }
                        vertices.push([x, y]);
                    }
                }
                let stride = last_x + 1;
                if staggered {
                    for j in (0..ny).step_by(2) {
                        check()?;
                        let up = (stride * j) as u32;
                        let middle = (stride * (j + 1)) as u32;
                        let down = (stride * (j + 2)) as u32;
                        for i in 0..nx as u32 {
                            faces.push(
                                if vertices[(up + i) as usize][1]
                                    < vertices[(up + i + 1) as usize][1]
                                {
                                    vec![up + i, middle + i + 1, up + i + 1]
                                } else {
                                    vec![up + i + 1, up + i, middle + i + 1]
                                },
                            );
                            faces.push(vec![up + i, middle + i, middle + i + 1]);
                        }
                        let x = nx as u32;
                        faces.push(vec![up + x, middle + x, middle + x + 1]);
                        for i in 0..x {
                            faces.push(
                                if vertices[(middle + i) as usize][1]
                                    < vertices[(middle + i + 1) as usize][1]
                                {
                                    vec![middle + i, down + i, middle + i + 1]
                                } else {
                                    vec![middle + i + 1, middle + i, down + i]
                                },
                            );
                            faces.push(vec![middle + i + 1, down + i, down + i + 1]);
                        }
                        faces.push(
                            if vertices[(middle + x) as usize][1]
                                < vertices[(middle + x + 1) as usize][1]
                            {
                                vec![middle + x, down + x, middle + x + 1]
                            } else {
                                vec![middle + x + 1, middle + x, down + x]
                            },
                        );
                    }
                } else {
                    for j in 0..ny {
                        check()?;
                        let up = (stride * j) as u32;
                        let down = (stride * (j + 1)) as u32;
                        for i in 0..nx as u32 {
                            faces.push(
                                if vertices[(up + i) as usize][1]
                                    < vertices[(up + i + 1) as usize][1]
                                {
                                    vec![up + i, down + i, down + i + 1, up + i + 1]
                                } else {
                                    vec![up + i + 1, up + i, down + i, down + i + 1]
                                },
                            );
                        }
                    }
                }
            }
            FragmentGridStyle::StaggeredStrips => {
                let mut down = 0;
                for j in 0..ny {
                    check()?;
                    let up = down;
                    down = (j + 1) * base[1] + jitter(1)?;
                    if j == ny - 1 {
                        down = height - 1;
                    }
                    for i in 0..=nx + 1 {
                        let mut x = i * base[0];
                        if j & 1 != 0 {
                            x -= base[0] / 2;
                        }
                        x += jitter(0)?;
                        if i == 0 {
                            x = 0;
                        }
                        if if j & 1 == 0 { i >= nx } else { i == nx + 1 } {
                            x = width - 1;
                        }
                        vertices.push([x, up]);
                        vertices.push([x, down]);
                    }
                }
                for j in 0..ny {
                    check()?;
                    for i in 0..nx + (j & 1) {
                        let v = ((j * (nx + 2) + i) * 2) as u32;
                        faces.push(vec![v, v + 1, v + 3, v + 2]);
                    }
                }
            }
        }
        check()?;
        *random = candidate;
        Ok(FragmentMesh { vertices, faces })
    }
}
