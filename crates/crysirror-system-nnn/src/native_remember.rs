// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! REMEMBER26 class-global Print oscillator and native row-edge geometry.
//! Shared RowEdgeLift owns every stored-byte operation.
use anyhow::{Result, ensure};
use crysirror_core::image_processing::ImageInput;
use crysirror_image::{effects::ImageEffect, row_edge_lift::RowEdgeLift};
#[derive(Clone, Debug, Default)]
pub(crate) struct RememberClass {
    pub print_count: i32,
}
impl RememberClass {
    pub(crate) fn image_input(&mut self, canvas: [u32; 2], base: ImageInput) -> Result<ImageInput> {
        ensure!(
            canvas.iter().all(|n| (1..=16384).contains(n)),
            "NNN REMEMBER canvas"
        );
        self.print_count = self.print_count.wrapping_add(1);
        if self.print_count > 100_000_000 {
            self.print_count = 0;
        }
        let count = f64::from(self.print_count);
        let width = canvas[0] as i32;
        let height = canvas[1] as usize;
        let ml = [0.0345698, 0.002178934, 0.04425678, 0.001234786];
        let delta = [0., 4., 11., 15.];
        let omega = [0.05145735, 0.0362343, 0.053232, 0.03417];
        let amplitudes = [15., 23., 17., 20.];
        let omega2 = [0.3, 0.22, 0.27, 0.19];
        let delta2 = [2., 3., 5., 7.];
        let h = std::array::from_fn::<_, 4, _>(|i| {
            amplitudes[i] * (1.5 + (count * omega2[i] + delta2[i]).cos())
        });
        let mut edges = Vec::with_capacity(height);
        for row in 0..height {
            let phases = std::array::from_fn::<_, 4, _>(|i| {
                count * ml[i] * 3.14159 + delta[i] + row as f64 * omega[i]
            });
            edges.push([
                (phases[0].cos() * h[0] + phases[1].cos() * h[1] + 0.5) as i32 + 64,
                (phases[2].cos() * h[2] + phases[3].cos() * h[3] + 0.5) as i32 + width - 64,
            ]);
        }
        // Source indexes row100 even for shorter screens. Bound that undefined
        // profile, preserving literal100 and all overlap when height>=101.
        let radius = 100usize.min(height - 1);
        let center = width / 2;
        if radius > 0 {
            let top_width = edges[radius][1] - edges[radius][0];
            for row in 0..radius {
                let theta = (radius - row) as f64 / (radius * 4) as f64 * 3.14159 * 2.0;
                let lift = (theta.sin() * 0.4 * f64::from(top_width)) as i32;
                edges[row][0] = (edges[row][0] + lift).min(center - 1);
                edges[row][1] = (edges[row][1] - lift).max(center + 1);
            }
            let bottom_width = edges[height - radius - 1][1] - edges[height - radius - 1][0];
            for row in 0..radius {
                let theta = row as f64 / (radius * 4) as f64 * 3.14159 * 2.0;
                let lift = (theta.sin() * 0.4 * f64::from(bottom_width)) as i32;
                let edge = &mut edges[height - radius + row];
                // The original bottom loop shadows screenSizeX with its
                // measured interval width, then uses THAT for its center clamp.
                edge[0] = (edge[0] + lift).min(bottom_width / 2 - 1);
                edge[1] = (edge[1] - lift).max(bottom_width / 2 + 1);
            }
        }
        let edge_groups = edges
            .into_iter()
            .map(|[left, right]| {
                let left = left + 64;
                let right = right - 64;
                [
                    if left < 0 { None } else { Some(left / 2) },
                    if right < 0 || right >= width {
                        None
                    } else {
                        Some(right / 2)
                    },
                ]
            })
            .collect();
        let spec = RowEdgeLift {
            row_start: 0,
            edge_groups,
            group_width: 2,
            step: 4,
            right_full_groups_only: true,
            write_mask: [true, true, true, false],
        };
        Ok(ImageInput::Process {
            input: Box::new(base),
            effects: serde_json::to_vec(&[ImageEffect::RowEdgeLift { spec }])?.into(),
        })
    }
}
// Setter stores NO parameter words, forces count0/max99999; InitParameter
// reruns that setter without resetting this class's Print count. Inherited
// ordinary picture pass precedes the edge lift. CountIsMax returnsTRUE.
