// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native chart records, persistent seen/traversed bits and registered functions.
//! Input geometry and drawing are shared UI/Layer operations, not bitmap code.
use super::*;
use crate::{
    expression::Variables, native_bootstrap::NativeConfiguration,
    native_resources::open_installed_asset,
};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{ClientStorage, StorageWrite, StorageWriteState},
};
use crysirror_tasks::Cancellation;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Debug)]
pub(crate) struct ChartNode {
    pub slot: i32,
    pub position: [i32; 2],
    pub routes: u8,
    pub next: Vec<i32>,
    pub descriptions: [Vec<Vec<u8>>; 2],
    pub variable: Option<u16>,
    pub top: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct ChartMap {
    pub height: i32,
    pub nodes: Vec<i32>,
}
#[derive(Clone, Debug)]
pub(crate) struct ChartDefinition {
    pub maps: Vec<ChartMap>,
    pub nodes: BTreeMap<i32, ChartNode>,
    pub block_size: usize,
    pub basic: usize,
    pub slot_variable: Option<u16>,
    pub from_variable: Option<u16>,
    pub print_variable: Option<u16>,
    pub open_variable: Option<u16>,
    pub auto_next: bool,
    pub setup: Names,
    pub commands: Names,
    pub default_command: bool,
    functions: Vec<(Vec<u8>, usize)>,
    percent_variable: Option<u16>,
    complete_variable: Option<u16>,
}
fn native_var(
    config: &NativeConfiguration,
    table: &Names,
    key: &[u8],
    default: &str,
) -> Option<u16> {
    let default = encoding_rs::SHIFT_JIS.encode(default).0;
    config
        .variable_names
        .index(table.parameter(key).unwrap_or(default.as_ref()))
}
fn read_asset(
    config: &NativeConfiguration,
    storage: &dyn ClientStorage,
    name: &str,
    cancel: &Cancellation,
) -> Result<Option<Vec<u8>>> {
    cancel.check()?;
    let path = VfsPath::new(Mount::Asset, &config.resources.path(name)?)?;
    let reader = match open_installed_asset(storage, &path) {
        Ok(reader) => reader,
        Err(crysirror_core::Error::NotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(Some(crysirror_resource::read_all(
        reader.as_ref(),
        1 << 20,
    )?))
}
impl ChartDefinition {
    pub fn prepare(
        config: &NativeConfiguration,
        storage: &dyn ClientStorage,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        let Some(table) = config.chart_setup.as_ref() else {
            return Ok(None);
        };
        let layout = ChartLayout::read(table)?;
        let maps = usize::try_from(table.integer(b"chartListNumber")?.unwrap_or(1))?;
        let block_size = layout.slots / maps;
        let desc_max = usize::try_from(table.integer(b"chartDescMax")?.unwrap_or(3))?;
        ensure!(
            (1..=64).contains(&desc_max),
            "NNN ChartMenu description row budget"
        );
        let cipher = table.integer(b"chartAngouFlag")?.unwrap_or(0) != 0;
        let mut charts = Vec::with_capacity(maps);
        let mut nodes = BTreeMap::new();
        for group in 0..maps {
            let path = format!(
                "chart/chart{}.{}",
                group + 1,
                if cipher { "fxf" } else { "xtx" }
            );
            let data = read_asset(config, storage, &path, cancel)?.unwrap_or_default();
            let names = Names::parse(&data, cipher)?;
            let (map, entries) =
                Self::parse_map(&names, group, block_size, desc_max, &config.variable_names)?;
            for node in entries {
                ensure!(
                    nodes.insert(node.slot, node).is_none(),
                    "NNN duplicate chart slot"
                );
            }
            charts.push(map);
        }
        let targets: Vec<_> = nodes
            .values()
            .flat_map(|n| n.next.iter().copied())
            .collect();
        for target in targets {
            if target < 0 {
                continue;
            }
            let target = nodes.get(&target).map(|n| n.slot).or_else(|| {
                charts
                    .get(target as usize / block_size)
                    .and_then(|m| m.nodes.first().copied())
            });
            if let Some(node) = target.and_then(|target| nodes.get_mut(&target)) {
                node.top = false;
            }
        }
        let mut functions = Vec::new();
        for (kind, (flag, key, default, enabled)) in [
            (
                b"useSetRouteFunction".as_slice(),
                b"setRouteFunctionName".as_slice(),
                b"setroute".as_slice(),
                1,
            ),
            (
                b"useGetChartPercentFunction",
                b"getChartPercentFunctionName",
                b"getchartpercent",
                1,
            ),
            (
                b"useCheckCompSlotFunction",
                b"checkCompSlotFunctionName",
                b"checkcompslot",
                1,
            ),
            (
                b"useOpenChartFunction",
                b"openChartFunctionName",
                b"openchart",
                1,
            ),
            (b"useSlotOnFunction", b"slotOnFunctionName", b"sloton", 0),
            (
                b"useSlotOpenFunction",
                b"slotOpenFunctionName",
                b"slotopen",
                0,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            if table.integer(flag)?.unwrap_or(enabled) != 0 {
                functions.push((table.parameter(key).unwrap_or(default).to_vec(), kind));
            }
        }
        let basic = usize::try_from(table.integer(b"basicChartNumber")?.unwrap_or(maps as i32))?;
        ensure!(basic <= maps, "NNN ChartMenu basic chart count bounds");
        Ok(Some(Self {
            maps: charts,
            nodes,
            block_size,
            basic,
            setup: table.clone(),
            commands: config.commands.clone(),
            default_command: config.setup.integer(b"selectchartIsDefault")?.unwrap_or(0) != 0,
            functions,
            slot_variable: native_var(config, table, b"slotVarName", "チャートスロット番号"),
            from_variable: native_var(config, table, b"workVarName", "ワークスロット番号"),
            print_variable: if table.integer(b"savePrintChartFlag")?.unwrap_or(0) != 0 {
                native_var(config, table, b"workVarName2", "ワークスロット番号２")
            } else {
                None
            },
            open_variable: if maps > basic {
                native_var(config, table, b"chartOpenVar", "チャートオープン情報")
            } else {
                None
            },
            percent_variable: native_var(config, table, b"chartPercentVarName", "answer"),
            complete_variable: native_var(config, table, b"checkCompVarName", "answer"),
            auto_next: table.integer(b"autoNextOn")?.unwrap_or(0) != 0,
        }))
    }
    pub fn parse_map(
        table: &Names,
        group: usize,
        block: usize,
        desc_max: usize,
        variables: &Names,
    ) -> Result<(ChartMap, Vec<ChartNode>)> {
        let height = table
            .tokens
            .first()
            .map(|n| crate::config::integer(n))
            .transpose()?
            .unwrap_or(0)
            .max(1800);
        ensure!(height <= 1_048_576, "NNN chart map height budget");
        let mut nodes: Vec<ChartNode> = Vec::new();
        let mut fields = 0;
        for token in table.tokens.iter().skip(1) {
            if let Some(number) = token.strip_prefix(b"@") {
                ensure!(
                    nodes.is_empty() || fields >= 3,
                    "NNN truncated chart node coordinates"
                );
                let slot = crate::config::integer(number)?;
                let index = usize::try_from(slot)?;
                ensure!(
                    index >= group * block && index < (group + 1) * block,
                    "NNN chart slot outside its chart block"
                );
                ensure!(nodes.len() < block, "NNN chart node budget");
                nodes.push(ChartNode {
                    slot,
                    position: [0, 0],
                    routes: 1,
                    next: vec![],
                    descriptions: [vec![], vec![]],
                    variable: None,
                    top: true,
                });
                fields = 0;
                continue;
            }
            let node = nodes
                .last_mut()
                .context("NNN chart field before first @slot")?;
            fields += 1;
            match fields {
                1 => {
                    let routes = crate::config::integer(token)?.max(1);
                    ensure!(
                        routes <= 8,
                        "NNN chart route bits exceed original byte record"
                    );
                    node.routes = routes as u8;
                }
                2 | 3 => node.position[fields - 2] = crate::config::integer(token)?,
                _ => match token.first() {
                    Some(b'0'..=b'9' | b'-') => {
                        ensure!(node.next.len() < 256, "NNN chart outgoing edge budget");
                        node.next.push(crate::config::integer(token)?);
                    }
                    Some(b'$') => node.variable = variables.index(&token[1..]),
                    marker => {
                        let unseen = usize::from(marker == Some(&b'!'));
                        ensure!(
                            node.descriptions[unseen].len() < desc_max,
                            "NNN chart description exceeds configured rows"
                        );
                        node.descriptions[unseen].push(token[unseen..].to_vec());
                    }
                },
            }
        }
        ensure!(
            nodes.is_empty() || fields >= 3,
            "NNN truncated chart node coordinates"
        );
        ensure!(
            nodes
                .iter()
                .all(|node| node.position.iter().all(|v| v.unsigned_abs() <= 1_000_000)),
            "NNN chart node coordinate budget"
        );
        Ok((
            ChartMap {
                height,
                nodes: nodes.iter().map(|n| n.slot).collect(),
            },
            nodes,
        ))
    }
    pub fn function(&self, name: &[u8]) -> Option<usize> {
        self.functions
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, kind)| *kind)
    }
    pub fn node(&self, slot: i32) -> Result<&ChartNode> {
        if let Some(node) = self.nodes.get(&slot) {
            return Ok(node);
        }
        // CChartList zero-initializes its slot-to-index table: an unlisted
        // in-range slot (notably startup slot zero) aliases the first record.
        let group = self.chart(slot)?;
        self.maps[group]
            .nodes
            .first()
            .and_then(|slot| self.nodes.get(slot))
            .context("NNN chart has no node records")
    }
    pub fn chart(&self, slot: i32) -> Result<usize> {
        let index = usize::try_from(slot)? / self.block_size;
        ensure!(index < self.maps.len(), "NNN chart slot group bounds");
        Ok(index)
    }
}
#[derive(Clone, Debug)]
pub(crate) struct ChartRuntime {
    pub definition: Arc<ChartDefinition>,
    /// Original four slots per little-endian word, eight route bits per slot.
    seen: Vec<u8>,
    traversed: Vec<u8>,
    revision: u64,
    saved_revision: Option<u64>,
}
#[derive(Clone, Debug)]
pub(crate) struct ChartFunctionEffect {
    pub checkpoint: Option<ChartSnapshotRequest>,
    pub save_system: bool,
}
#[derive(Clone, Debug)]
pub(crate) struct ChartSnapshotRequest {
    pub slot: i32,
    pub variables: Variables,
    pub state: ChartState,
}
pub(crate) struct ChartWrite {
    seen: Box<dyn StorageWrite>,
    traversed: Box<dyn StorageWrite>,
    done: [bool; 2],
    revision: u64,
}
impl ChartRuntime {
    pub fn prepare(
        config: &NativeConfiguration,
        storage: &dyn ClientStorage,
        cancel: &Cancellation,
    ) -> Result<Option<Self>> {
        let Some(definition) = ChartDefinition::prepare(config, storage, cancel)? else {
            return Ok(None);
        };
        let layout = ChartLayout::read(config.chart_setup.as_ref().unwrap())?;
        let mut seen = vec![0; layout.slots.div_ceil(4) * 4];
        let mut traversed = vec![0; layout.slots];
        let mut present = true;
        for (name, destination) in [("mita.dat", &mut seen), ("tsuka.dat", &mut traversed)] {
            cancel.check()?;
            match storage.open_read(&VfsPath::new(Mount::Save, name)?) {
                Ok(reader) => {
                    let n = usize::try_from(reader.len()?.min(destination.len() as u64))?;
                    crysirror_resource::read_exact_at(reader.as_ref(), 0, &mut destination[..n])?;
                }
                Err(crysirror_core::Error::NotFound) => present = false,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(Some(Self {
            definition: Arc::new(definition),
            seen,
            traversed,
            revision: 0,
            saved_revision: present.then_some(0),
        }))
    }
    pub fn seen(&self, slot: i32) -> Result<bool> {
        let node = self.definition.node(slot)?;
        let _ = node;
        Ok(self.seen[usize::try_from(slot)?] != 0)
    }
    pub fn complete(&self, slot: i32) -> Result<bool> {
        let node = self.definition.node(slot)?;
        let mask = (1u16 << node.routes) - 1;
        Ok(u16::from(self.seen[slot as usize]) & mask == mask)
    }
    pub fn route_seen(&self, slot: i32, route: usize) -> Result<bool> {
        let node = self.definition.node(slot)?;
        ensure!(route < usize::from(node.routes), "NNN chart route index");
        Ok(self.seen[slot as usize] & (1 << route) != 0)
    }
    pub fn road_traversed(&self, slot: i32, road: usize) -> Result<bool> {
        self.definition.node(slot)?;
        ensure!(road < 8, "NNN chart road exceeds persistent byte");
        Ok(self.traversed[slot as usize] & (1 << road) != 0)
    }
    pub fn mark(&mut self, slot: i32, route: i32) -> Result<()> {
        let node = self.definition.node(slot)?;
        let bit = if node.routes <= 1 {
            Some(0)
        } else if (1..=8).contains(&route) {
            Some(route as u32 - 1)
        } else {
            None
        };
        if let Some(bit) = bit {
            let target = &mut self.seen[slot as usize];
            let old = *target;
            *target |= 1 << bit;
            if *target != old {
                self.revision = self
                    .revision
                    .checked_add(1)
                    .context("NNN chart progress revision overflow")?;
            }
        }
        Ok(())
    }
    fn counts(&self, range: std::ops::Range<usize>) -> Result<(u64, u64)> {
        let mut seen = 0;
        let mut total = 0;
        for chart in range {
            let map = self
                .definition
                .maps
                .get(chart)
                .context("NNN chart completion group bounds")?;
            for slot in &map.nodes {
                let node = self.definition.node(*slot)?;
                let mask = (1u16 << node.routes) - 1;
                seen += u64::from((u16::from(self.seen[*slot as usize]) & mask).count_ones());
                total += u64::from(node.routes);
            }
        }
        Ok((seen, total))
    }
    pub fn percent(&self, chart: i32) -> Result<i32> {
        let range = match chart {
            -1 => 0..self.definition.maps.len(),
            0 => 0..self.definition.basic,
            n if n > 0 => (n as usize - 1)..n as usize,
            _ => anyhow::bail!("NNN chart percentage group index"),
        };
        let (seen, total) = self.counts(range)?;
        // Empty original charts divide by zero. Define empty completion as 0,
        // while retaining the nonempty 1/99 percent visibility rules.
        if total == 0 {
            return Ok(0);
        }
        let percent = seen * 100 / total;
        Ok(if percent == 0 && seen > 0 {
            1
        } else if percent >= 100 && seen < total {
            99
        } else {
            percent as i32
        })
    }
    pub fn slot(&self, variables: &mut Variables) -> Result<i32> {
        self.definition
            .slot_variable
            .map(|id| variables.read_stored(id))
            .map_or(Ok(0), Ok)
    }
    pub fn set_slot(&self, variables: &mut Variables, slot: i32) -> Result<()> {
        if let Some(id) = self.definition.slot_variable {
            variables.write_native(id, slot)?;
        }
        Ok(())
    }
    pub fn from(&self, variables: &mut Variables) -> Result<i32> {
        self.definition
            .from_variable
            .map(|id| variables.read_stored(id))
            .map_or(Ok(0), Ok)
    }
    pub fn set_from(&self, variables: &mut Variables, slot: i32) -> Result<()> {
        if let Some(id) = self.definition.from_variable {
            variables.write_native(id, slot)?;
        }
        Ok(())
    }
    pub fn open(&self, group: usize, variables: &mut Variables) -> Result<bool> {
        if group < self.definition.basic || self.definition.open_variable.is_none() {
            return Ok(true);
        }
        let flags = variables.read_stored(self.definition.open_variable.unwrap()) as u32;
        Ok(flags & (1 << ((group - self.definition.basic) & 31)) != 0)
    }
    pub fn selectable(
        &self,
        state: &ChartState,
        slot: i32,
        variables: &mut Variables,
    ) -> Result<bool> {
        let node = self.definition.node(slot)?;
        Ok(state.selection(usize::try_from(slot)?)?
            && node.variable.map(|v| variables.read_stored(v)).unwrap_or(1) != 0)
    }
    pub fn apply(
        &mut self,
        state: &mut ChartState,
        name: &[u8],
        parameters: &[i32],
        variables: &mut Variables,
    ) -> Result<Option<ChartFunctionEffect>> {
        let Some(kind) = self.definition.function(name) else {
            return Ok(None);
        };
        let first = parameters.first().copied().unwrap_or(0);
        let mut effect = ChartFunctionEffect {
            checkpoint: None,
            save_system: false,
        };
        match kind {
            0 => {
                let slot = self.slot(variables)?;
                self.mark(slot, first)?;
            }
            1 => {
                let percent = self.percent(first)?;
                if let Some(id) = self.definition.percent_variable {
                    variables.write_native(id, percent)?;
                }
            }
            2 => {
                let slot = self.slot(variables)?;
                let complete = i32::from(self.complete(slot)?);
                if let Some(id) = self.definition.complete_variable {
                    variables.write_native(id, complete)?;
                }
            }
            3 => {
                let bit = first - self.definition.basic as i32 - 1;
                if (0..31).contains(&bit) {
                    if let Some(id) = self.definition.open_variable {
                        let flags = variables.read_stored(id) as u32;
                        variables.write_native(id, (flags | (1 << bit)) as i32)?;
                        effect.save_system = true;
                    }
                }
            }
            4 => state.set_selection(first, parameters.get(1).copied().unwrap_or(1) != 0)?,
            5 => {
                let old = self.from(variables)?;
                self.set_from(variables, first)?;
                self.mark(first, 0)?;
                let slot = self.slot(variables)?;
                effect.checkpoint = Some(ChartSnapshotRequest {
                    slot,
                    variables: variables.clone(),
                    state: state.clone(),
                });
                self.set_from(variables, old)?;
            }
            _ => unreachable!(),
        }
        ensure!(
            variables.native_writes().is_empty(),
            "NNN chart variables need an atomic shared-resource binding plan"
        );
        Ok(Some(effect))
    }
    pub fn start(
        &mut self,
        state: &mut ChartState,
        variables: &mut Variables,
    ) -> Result<Vec<ChartSnapshotRequest>> {
        let slot = self.slot(variables)?;
        if slot == 0 {
            self.set_slot(variables, 1)?;
            self.mark(1, 0)?;
        } else {
            let from = self.from(variables)?;
            if let Some(node) = self.definition.nodes.get(&from) {
                if let Some(road) = node.next.iter().position(|next| *next == slot) {
                    state.set_road(usize::try_from(from)?, road as i32)?;
                }
            }
            if self.definition.node(slot)?.routes <= 1 {
                self.mark(slot, 0)?;
            }
        }
        // Source retains the pre-initialization local slot value here.
        self.set_from(variables, slot)?;
        let checkpoint_slot = self.slot(variables)?;
        let mut snapshots = vec![ChartSnapshotRequest {
            slot: checkpoint_slot,
            variables: variables.clone(),
            state: state.clone(),
        }];
        if self.definition.auto_next {
            let node = self.definition.node(slot)?;
            if node.next.len() == 1 {
                let next = node.next[0];
                if self.definition.chart(next)? != self.definition.chart(slot)? {
                    self.set_slot(variables, next)?;
                    let from = self.from(variables)?;
                    if let Some(node) = self.definition.nodes.get(&from) {
                        if let Some(road) = node.next.iter().position(|n| *n == next) {
                            state.set_road(usize::try_from(from)?, road as i32)?;
                        }
                    }
                    self.mark(next, 0)?;
                    self.set_from(variables, next)?;
                    if let Some(id) = self.definition.print_variable {
                        variables.write_native(id, self.definition.chart(next)? as i32)?;
                    }
                    snapshots.push(ChartSnapshotRequest {
                        slot: next,
                        variables: variables.clone(),
                        state: state.clone(),
                    });
                }
            }
        }
        ensure!(
            variables.native_writes().is_empty(),
            "NNN ChartMenu command has shared-resource variable writes"
        );
        Ok(snapshots)
    }
    pub fn dirty(&self) -> bool {
        self.saved_revision != Some(self.revision)
    }
    pub fn begin_save(&self, storage: &dyn ClientStorage) -> Result<ChartWrite> {
        let seen = storage.begin_write_atomic(
            &VfsPath::new(Mount::Save, "mita.dat")?,
            Arc::from(self.seen.clone()),
        )?;
        let traversed = storage.begin_write_atomic(
            &VfsPath::new(Mount::Save, "tsuka.dat")?,
            Arc::from(self.traversed.clone()),
        )?;
        Ok(ChartWrite {
            seen,
            traversed,
            done: [false; 2],
            revision: self.revision,
        })
    }
}
impl ChartWrite {
    pub fn poll(&mut self, runtime: &mut ChartRuntime) -> Result<bool> {
        for (i, ticket) in [&mut self.seen, &mut self.traversed]
            .into_iter()
            .enumerate()
        {
            if self.done[i] {
                continue;
            }
            match ticket.poll()? {
                StorageWriteState::Pending => (),
                StorageWriteState::Ready => self.done[i] = true,
                StorageWriteState::Failed(e) => anyhow::bail!("NNN chart progress storage: {e}"),
            }
        }
        if self.done == [true; 2] {
            runtime.saved_revision = Some(self.revision);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (ChartRuntime, ChartState, Variables) {
        let vars = Names::parse(b"slot from gate answer open", false).unwrap();
        let table = Names::parse(
            b"1800 @1 1 100 200 2 start !unknown @2 2 300 400 $gate 3 branch @3 1 500 600 end",
            false,
        )
        .unwrap();
        let (map, nodes) = ChartDefinition::parse_map(&table, 0, 1000, 3, &vars).unwrap();
        let definition = ChartDefinition {
            maps: vec![map],
            nodes: nodes.into_iter().map(|n| (n.slot, n)).collect(),
            block_size: 1000,
            basic: 1,
            slot_variable: Some(0),
            from_variable: Some(1),
            print_variable: None,
            open_variable: None,
            auto_next: false,
            setup: Names { tokens: vec![] },
            commands: Names { tokens: vec![] },
            default_command: true,
            functions: vec![
                (b"setroute".to_vec(), 0),
                (b"getchartpercent".to_vec(), 1),
                (b"checkcompslot".to_vec(), 2),
                (b"sloton".to_vec(), 4),
                (b"slotopen".to_vec(), 5),
            ],
            percent_variable: Some(3),
            complete_variable: Some(3),
        };
        let layout = ChartLayout::read(&Names { tokens: vec![] }).unwrap();
        let runtime = ChartRuntime {
            definition: Arc::new(definition),
            seen: vec![0; 1000],
            traversed: vec![0; 1000],
            revision: 0,
            saved_revision: Some(0),
        };
        (
            runtime,
            ChartState::fresh(layout),
            Variables::fresh(crate::expression::VariableProfile::Legacy1000),
        )
    }
    #[test]
    fn native_chart_route_bits_gates_completion_and_checkpoint_variable_order() {
        let (mut runtime, mut state, mut vars) = fixture();
        assert_eq!(runtime.definition.node(0).unwrap().slot, 1);
        assert!(runtime.definition.node(-1).is_err());
        let snapshots = runtime.start(&mut state, &mut vars).unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].slot, 1);
        assert_eq!(vars.read_stored(0), 1);
        assert_eq!(vars.read_stored(1), 0);
        assert_eq!(runtime.percent(-1).unwrap(), 25);
        assert!(!runtime.selectable(&state, 2, &mut vars).unwrap());
        vars.write_stored(2, 1);
        assert!(runtime.selectable(&state, 2, &mut vars).unwrap());
        runtime
            .apply(&mut state, b"sloton", &[2, 0], &mut vars)
            .unwrap();
        assert!(!runtime.selectable(&state, 2, &mut vars).unwrap());
        runtime
            .apply(&mut state, b"sloton", &[2], &mut vars)
            .unwrap();
        vars.write_stored(0, 2);
        vars.write_stored(1, 1);
        let snapshots = runtime.start(&mut state, &mut vars).unwrap();
        assert_eq!(state.road(1).unwrap(), 1);
        assert_eq!(snapshots[0].variables.read_stored(1), 2);
        assert!(!runtime.seen(2).unwrap());
        runtime
            .apply(&mut state, b"setroute", &[1], &mut vars)
            .unwrap();
        assert!(runtime.seen(2).unwrap());
        assert!(!runtime.complete(2).unwrap());
        assert_eq!(runtime.percent(0).unwrap(), 50);
        runtime
            .apply(&mut state, b"setroute", &[2], &mut vars)
            .unwrap();
        assert!(runtime.complete(2).unwrap());
        runtime
            .apply(&mut state, b"getchartpercent", &[-1], &mut vars)
            .unwrap();
        assert_eq!(vars.read_stored(3), 75);
        let effect = runtime
            .apply(&mut state, b"slotopen", &[3], &mut vars)
            .unwrap()
            .unwrap();
        let snapshot = effect.checkpoint.unwrap();
        assert_eq!(snapshot.slot, 2);
        assert_eq!(snapshot.variables.read_stored(1), 3);
        assert_eq!(vars.read_stored(1), 2);
        assert_eq!(runtime.percent(1).unwrap(), 100);
        assert!(!runtime.road_traversed(1, 0).unwrap()); // source SetTsuka call is commented out
    }
    #[test]
    fn native_chart_rejects_truncated_and_out_of_block_records() {
        let names = Names { tokens: vec![] };
        for data in [
            b"1800 @1 1 2".as_slice(),
            b"1800 @1000 1 0 0",
            b"1800 @1 9 0 0",
        ] {
            assert!(
                ChartDefinition::parse_map(&Names::parse(data, false).unwrap(), 0, 1000, 3, &names)
                    .is_err()
            );
        }
    }
}
