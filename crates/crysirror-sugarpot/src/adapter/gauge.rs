// SPDX-License-Identifier: MPL-2.0
//! Original GG slot fields and selector policy. Shared Layers own drawing.
use super::*;
use crate::native_storage::slot::SlotGauge;
impl SugarPotVm {
    pub(super) fn gauge(
        &mut self,
        h: &mut dyn HostServices,
        name: &str,
        args: &[String],
    ) -> Result<bool> {
        let id = self.number(args, 0, 0)?;
        if name == "gg" {
            let selector = self.number(args, 1, 0)?;
            if !(1..=8).contains(&selector) {
                ensure!((1..=256).contains(&id), "SugarPot gg allocation count");
                self.gauges = (0..id).map(|_| SlotGauge { fields: [0; 10] }).collect();
                return Ok(true);
            }
            let a = self.number(args, 2, 0)?;
            let b = self.number(args, 3, 0)?;
            // Portable repair: native ALL setters2..8 omit sentinel checks.
            // Apply the same determined stores to bounded allocated records.
            let ids = if id == -1 {
                (0..self.gauges.len()).collect::<Vec<_>>()
            } else {
                vec![usize::try_from(id)?]
            };
            for id in ids {
                let f = &mut self.gauges.get_mut(id).context("SugarPot gg ID")?.fields;
                match selector {
                    1 => {
                        f[3] = a as i16 as i32;
                        f[4] = b as i16 as i32;
                    }
                    2 => {
                        f[1] = a as i16 as i32;
                        f[2] = b as i16 as i32;
                    }
                    3 => f[5] = a as u8 as i32,
                    4 => f[8] = a as i16 as i32,
                    5 => f[7] = a as i16 as i32,
                    6 => f[0] = ((a as u32 & 0xffffff) << 8) as i32,
                    7 => f[9] = a as i16 as i32,
                    8 => f[9] = (a.wrapping_mul(f[8]) / 100) as i16 as i32,
                    _ => unreachable!(),
                }
            }
            return Ok(true);
        }
        if name == "ggoff" {
            let ids = if id == -1 {
                (0..self.gauges.len()).collect::<Vec<_>>()
            } else {
                vec![usize::try_from(id)?]
            };
            let mut layers = vec![];
            for id in ids {
                self.gauges.get_mut(id).context("SugarPot ggoff ID")?.fields[6] = 0;
                layers.push(json!({"op":"remove","id":45000+id as u32}));
            }
            self.submit(h, layers, vec![])?;
            return Ok(true);
        }
        //44B660 switches on the FIRST gauge direction, even for another ID.
        let direction = self
            .gauges
            .first()
            .context("SugarPot ggon before allocation")?
            .fields[7];
        if matches!(direction, 1..=3) {
            return Ok(true);
        } // Explicit native drawless branches.
        ensure!(direction == 0, "SugarPot ggon unknown native direction");
        let gauge = self
            .gauges
            .get_mut(usize::try_from(id)?)
            .context("SugarPot ggon ID")?;
        let fields = gauge.fields;
        ensure!(
            self.number(args, 1, 0)? == 0,
            "SugarPot animated gauge controller requires shared stored-colour transition"
        );
        ensure!(
            fields[5] == 0,
            "SugarPot percentage-add gauge requires frozen shared image job"
        );
        ensure!(
            fields[3] > 0 && fields[4] > 0,
            "SugarPot empty gauge rectangle"
        );
        let color = fields[0] as u32;
        self.submit(h,vec![json!({"op":"put","layer":{"id":45000+id as u32,"rect":[fields[1],fields[2],fields[3],fields[4]],"order":[4500,id],"paint":{"kind":"solid","color":[(color>>24)&255,(color>>16)&255,(color>>8)&255,255]}}})],vec![])?;
        self.gauges[id as usize].fields[6] = 1;
        Ok(true)
    }
}
