// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CCommonChartMenu new-chart acknowledgement and Print-owned animation.
//! Widget hit testing and all mark pixels remain shared UI/image operations.
use super::*;
use crate::{
    expression::Variables,
    native_anime_parts::{AnimePartClock, AnimePartSpec, PartPrint},
    native_bootstrap::NativeConfiguration,
};

#[derive(Clone, Debug)]
pub(crate) struct ChartNewMark {
    pub clock: AnimePartClock,
    variable: Option<u16>,
    offset: [i32; 2],
    pending_maps: u32,
}
impl ChartNewMark {
    pub(crate) fn read(config: &NativeConfiguration, table: &Names) -> Result<Option<Self>> {
        if table.integer(b"newMarkPrintFlag")?.unwrap_or(0) == 0 {
            return Ok(None);
        }
        let name = table
            .parameter(b"newMarkVarName")
            .unwrap_or(b"newchartflag");
        Ok(Some(Self {
            clock: AnimePartClock::new(AnimePartSpec::read(table, b"newmark")?),
            variable: config.variable_names.index(name),
            offset: [
                table.integer(b"newMarkPrintX")?.unwrap_or(0),
                table.integer(b"newMarkPrintY")?.unwrap_or(0),
            ],
            pending_maps: 0,
        }))
    }
    pub(crate) fn init(
        &mut self,
        runtime: &ChartRuntime,
        state: &ChartState,
        current: i32,
        variables: &mut Variables,
    ) -> Result<()> {
        // Native m_bitPattern has32 entries; more enabled groups formerly
        // read beyond its allocation. Fail before indexing instead of inventing
        // an acknowledgement bit for a map the native format cannot identify.
        ensure!(
            runtime.definition.maps.len() <= 32,
            "NNN chart new-mark32-bit map identity"
        );
        self.clock.init();
        self.pending_maps = 0;
        // An absent variable uses original oldNewMark=-1, displaying no marks.
        let old = self
            .variable
            .map(|id| variables.read_stored(id))
            .unwrap_or(-1) as u32;
        let next = &runtime.definition.node(current)?.next;
        for (group, map) in runtime.definition.maps.iter().enumerate() {
            if !runtime.open(group, variables)? {
                continue;
            }
            let mut enabled = false;
            for slot in &map.nodes {
                if runtime.selectable(state, *slot, variables)?
                    && (next.contains(slot) || runtime.seen(*slot)?)
                {
                    enabled = true;
                    break;
                }
            }
            let bit = 1u32 << group;
            if enabled && old & bit == 0 {
                self.pending_maps |= bit;
            }
        }
        Ok(())
    }
    pub(crate) fn print(
        &self,
        group: usize,
        native_button_origin: [i32; 2],
    ) -> Result<Option<PartPrint>> {
        ensure!(group < 32, "NNN chart mark map identity");
        if self.pending_maps & (1u32 << group) == 0 {
            return Ok(None);
        }
        let point =
            std::array::from_fn(|axis| native_button_origin[axis].wrapping_add(self.offset[axis]));
        // PrintNewMark uses Print(pt), not DeltaPrint: tagged PrintX/Y are
        // not added a second time to the selected grow-button's origin.
        Ok(Some(self.clock.spec.print(
            self.clock.count,
            Some(point),
            false,
            None,
        )?))
    }
    pub(crate) fn printed(&mut self) -> Result<()> {
        // CCommonChartMenu::Print calls AnimeNewMark after all grow buttons,
        // including frames on which pending_maps is zero. Calcu does not tick it.
        self.clock.calcu(1)
    }
    pub(crate) fn acknowledge_choice(&self, group: usize, variables: &mut Variables) -> Result<()> {
        ensure!(group < 32, "NNN chart mark selected map identity");
        if self.pending_maps != 0 {
            if let Some(id) = self.variable {
                let value =
                    variables.read_stored(id) as u32 | (self.pending_maps & (1u32 << group));
                variables.write_native(id, value as i32)?;
            }
        }
        Ok(())
    }
}
