//! Native destination-filter policy over engine display-prefix image jobs.
use super::*;
#[derive(Default)]
pub(super) struct NativeFilter {
    mode: i32,
    color: i32,
    shift: i32,
    pub mask: Option<(i32, u64)>,
    pub signature: Option<Value>,
}
impl BgiVm {
    pub(super) fn native_filter_call(
        &mut self,
        host: &mut dyn HostServices,
        call: &NativeCall,
    ) -> Result<Option<i32>> {
        let a = &call.args;
        if call.id == 0x60 {
            let object = self.graph.allocate(0x90000000, 8, Kind::Filter);
            ensure!(object != 0, "BGI filter pool exhausted");
            self.graph.objects.entry(object).or_default().blend_mode = 192;
            self.graph.filters.insert(
                object,
                NativeFilter {
                    mode: 7,
                    ..Default::default()
                },
            );
            return Ok(Some(object));
        }
        let object = a[0];
        ensure!(
            self.graph.tree.kind(object) == Some(Kind::Filter),
            "BGI invalid filter handle"
        );
        if call.id == 0x61 {
            if let Some(parent) = self.graph.tree.parent(object) {
                if let Some(members) = self.graph.members.get_mut(&parent) {
                    members.remove(&object);
                }
            }
            self.hide_graph_object(host, object)?;
            self.graph.images.remove(object);
            self.graph.filters.remove(&object);
            self.graph.objects.remove(&object);
            self.graph.tree.remove(object);
            self.graph.mark_visual(object);
            return Ok(None);
        }
        if call.id == 0x64 {
            self.graph.tree.set_enabled(object, a[1] != 0);
            self.graph.objects.entry(object).or_default().native.enabled = u32::from(a[1] != 0);
            return Ok(None);
        }
        let (mode, color, mask, shift, alpha, priority) = if call.id == 0x65 {
            (0, a[1], -1, 0, a[2], a[3])
        } else {
            (a[1], a[2], a[3], a[4], a[5], a[6])
        };
        target_checks::priority(priority)?;
        target_checks::effect_level(alpha)?;
        let mask = if mask == -1 {
            None
        } else {
            let (_, b) = self.bitmap(mask)?;
            ensure!(b.format == 3, "BGI filter bitmap is not grayscale");
            ensure!(
                b.size == self.graph.screen,
                "BGI filter bitmap is not screen-sized"
            );
            Some((mask, b.generation))
        };
        ensure!(
            mask.is_some() || matches!(mode, 0..=3),
            "BGI unsupported destination filter {mode}"
        );
        let p = self.graph.objects.entry(object).or_default();
        p.set_alpha_parameter(alpha);
        p.native.priority = priority as u32;
        p.native.draw_enabled = 1;
        self.graph.sort(object);
        self.graph.filters.insert(
            object,
            NativeFilter {
                mode,
                color,
                shift,
                mask,
                signature: None,
            },
        );
        self.publish_graph(host)?;
        self.rebuild_native_filter_plan(object)?;
        Ok(None)
    }
    fn native_filter_signature(&self, object: i32) -> Result<Value> {
        let filter = self
            .graph
            .filters
            .get(&object)
            .context("BGI filter state")?;
        let p = self
            .graph
            .objects
            .get(&object)
            .context("BGI filter object")?;
        let cut = self
            .graph
            .tree
            .chain_position(object)
            .context("BGI filter chain position")?;
        let revisions = self
            .graph
            .tree
            .chain_handles()
            .iter()
            .take(cut as usize)
            .map(|id| {
                (
                    *id,
                    self.graph.visual_versions.get(id).copied().unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        let mask_valid = filter.mask.is_none_or(|(id, g)| {
            self.bitmaps
                .get(&(id as u32))
                .is_some_and(|b| b.generation == g)
        });
        let n = p.native;
        Ok(
            json!({"screen":self.graph.screen,"cut":[n.priority,cut],"origin":self.native_filter_origin(object)?,"mode":filter.mode,"color":filter.color,"shift":filter.shift,"level":p.transparency_parameter(),"mask":filter.mask,"mask_valid":mask_valid,"revisions":revisions}),
        )
    }
    fn native_filter_origin(&self, object: i32) -> Result<[i32; 2]> {
        let n = &self
            .graph
            .objects
            .get(&object)
            .context("BGI filter object")?
            .native;
        let offset = if n.global_display_offset_enabled != 0 {
            self.graph.display_offset
        } else {
            [0; 2]
        };
        Ok([
            n.position_x
                .wrapping_add(n.primary_offset_x)
                .wrapping_add(n.secondary_offset_x)
                .wrapping_add(offset[0]),
            n.position_y
                .wrapping_add(n.primary_offset_y)
                .wrapping_add(n.secondary_offset_y)
                .wrapping_add(offset[1]),
        ])
    }
    fn rebuild_native_filter_plan(&mut self, object: i32) -> Result<()> {
        let signature = self.native_filter_signature(object)?;
        let f = self
            .graph
            .filters
            .get(&object)
            .context("BGI filter state")?;
        let p = self
            .graph
            .objects
            .get(&object)
            .context("BGI filter object")?;
        let mask = f.mask.filter(|&(id, g)| {
            self.bitmaps
                .get(&(id as u32))
                .is_some_and(|b| b.generation == g)
        });
        let operation = Raster::Display {
            spec: DisplayRaster::DestinationFilter {
                enabled: f.mask.is_none() || mask.is_some(),
                mode: f.mode,
                color: f.color,
                level: p.transparency_parameter() as u16,
                shift: f.shift,
                masked: mask.is_some(),
                origin: self.native_filter_origin(object)?,
            },
        };
        let cut = [
            p.native.priority as i32,
            self.graph
                .tree
                .chain_position(object)
                .context("BGI filter order")? as i32,
        ];
        let inputs = vec![
            GraphInput::Prefix(self.graph.screen, cut),
            mask.map_or(GraphInput::Solid(self.graph.screen, [0; 4]), |(id, _)| {
                GraphInput::Bitmap(id)
            }),
        ];
        self.set_graph_plan(
            object,
            GraphPlan {
                inputs,
                operation: Some(operation),
                quad: None,
                format: 2,
                force_opaque: false,
                filters: Vec::new(),
            },
        );
        self.graph.filters.get_mut(&object).unwrap().signature = Some(signature);
        Ok(())
    }
    pub(super) fn refresh_native_filters(&mut self, host: &mut dyn HostServices) -> Result<()> {
        let objects = self
            .graph
            .tree
            .chain_handles()
            .iter()
            .copied()
            .filter(|id| self.graph.filters.contains_key(id))
            .collect::<Vec<_>>();
        for object in objects {
            // Finish the captured immutable prefix before scheduling a newer
            // one; continuously changing animation must not starve completion.
            if self.graph.images.dirty.contains(&object) {
                continue;
            }
            let p = self
                .graph
                .objects
                .get(&object)
                .context("BGI filter object")?;
            if self.graph.global_enabled == 0
                || p.native.draw_enabled == 0
                || p.native.suppress_draw != 0
                || p.mask_alpha >= 256
                || p.alpha_multiplier == 0
                || !self.graph.tree.chain_drawable(object)
            {
                continue;
            }
            let signature = self.native_filter_signature(object)?;
            if self.graph.filters[&object].signature.as_ref() == Some(&signature) {
                continue;
            }
            self.rebuild_native_filter_plan(object)?;
            if self.refresh_graph_image(host, object)? {
                self.publish_graph(host)?;
            }
        }
        Ok(())
    }
}
