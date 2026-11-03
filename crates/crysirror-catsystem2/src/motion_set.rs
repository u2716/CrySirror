// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original788AF0..789B40 motion-manager policy over the same KCS Controllers.
//! The ordered pose chain is original VM policy, not another animation graph,
//! interpolator or renderer. Class-specific shared Scene binding is separate.
use crate::{
    kcs::Program,
    motion::{Controller, PoseWords, RandomStreams, Tick},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{service::HostServices, state::ClientId};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
struct Entry {
    name: Vec<u16>,
    group: u32,
    start: u32,
    controller: Controller,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Frame { clock: u32, freeze_unstarted: bool },
    Terminal,
}
#[derive(Clone, Debug)]
struct Pending {
    mode: Mode,
    ids: Vec<u32>,
    next: usize,
    seeded: bool,
    visited: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Evaluation {
    Budget,
    Ready { visited: bool, pose: PoseWords },
}
#[derive(Clone, Debug)]
pub(crate) struct MotionSet {
    base: PoseWords,
    output: Option<PoseWords>,
    entries: BTreeMap<u32, Entry>,
    next: u32,
    speed: f32,
    pending: Option<Pending>,
    owner: Option<ClientId>,
}
impl Default for MotionSet {
    fn default() -> Self {
        let mut pose = [0; 36];
        for i in [9, 14, 19, 24, 25, 26, 27] {
            pose[i] = 1f32.to_bits();
        }
        pose[28] = u32::MAX;
        pose[35] = 1;
        Self {
            base: PoseWords(pose),
            //788AF0 does not initialize the output block before evaluation.
            output: None,
            entries: BTreeMap::new(),
            next: 1,
            speed: 1.,
            pending: None,
            owner: None,
        }
    }
}
//78942D..78945A calls79FE00 (signed64 truncation) and only consumes EAX.
//A wrapped DWORD delta can round to2^32 as f32, whose low DWORD is0, not a
//Rust-saturating u32::MAX and not an invalid stop-at-clock0 request.
fn native_clock_value(clock:u32,start:u32,speed:f32)->Result<u32>{
    let scaled=(clock.wrapping_sub(start)as f32)*speed+0.5f32;
    ensure!(scaled.is_finite()&&scaled>=0.&&f64::from(scaled)<9223372036854775808.,
        "CatSystem2 motion-set clock conversion outside reviewed signed64 lowering profile");
    Ok((scaled as i64)as u32)
}
impl MotionSet {
    pub(crate) fn instructions(&self)->u64{
        self.entries.values().fold(0,|sum,entry|sum.saturating_add(entry.controller.instructions()))
    }
    fn idle(&self) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "CatSystem2 motion-set mutation belongs to an unfinished evaluation"
        );
        Ok(())
    }
    pub(crate) fn base(&self) -> PoseWords {
        self.base
    }
    pub(crate) fn output(&self) -> Result<Option<PoseWords>> {
        self.idle()?;
        Ok(self.output)
    }
    pub(crate) fn set_base(&mut self, pose: PoseWords) -> Result<()> {
        self.idle()?;
        self.base = pose;
        Ok(())
    }
    pub(crate) fn set_output(&mut self, pose: PoseWords) -> Result<()> {
        self.idle()?;
        self.output = Some(pose);
        Ok(())
    }
    ///788E20 commits actual evaluated output; ordinary frames never silently
    ///accumulate last frame's results into the original base pose.
    pub(crate) fn commit_output(&mut self) -> Result<()> {
        self.idle()?;
        self.base = self
            .output
            .context("CatSystem2 motion-set output has not been initialized")?;
        Ok(())
    }
    pub(crate) fn set_speed(&mut self, speed: f32) -> Result<()> {
        self.idle()?;
        ensure!(
            speed.is_finite() && speed >= 0.,
            "CatSystem2 motion-set clock scale outside reviewed finite nonnegative profile"
        );
        self.speed = speed;
        Ok(())
    }
    ///Resource loading remains a separately admitted shared worker job. This
    ///method receives the real immutable KCS program, never a callback or DLL.
    pub(crate) fn insert(
        &mut self,
        name: Vec<u16>,
        program: Arc<Program>,
        parameters: &[u32],
        custom: Option<&[u8]>,
        group: u32,
        start: u32,
        delay: u32,
    ) -> Result<u32> {
        self.idle()?;
        ensure!(
            !name.is_empty() && name.len() < 32 && name.iter().all(|c| *c > 0 && *c <= 127),
            "CatSystem2 motion-set resource-name profile"
        );
        ensure!(
            self.entries.len() < 4096,
            "CatSystem2 motion-set controller count"
        );
        let id = self.next;
        let next = id
            .checked_add(1)
            .context("CatSystem2 motion-set handle overflow")?;
        let mut controller = Controller::new(program, PoseWords([0; 36]), parameters, delay)?;
        if let Some(bytes) = custom {
            controller.set_custom_data(bytes)?;
        }
        self.entries.insert(
            id,
            Entry {
                name,
                group,
                start,
                controller,
            },
        );
        self.next = next;
        Ok(id)
    }
    pub(crate) fn remove(&mut self, id: u32) -> Result<bool> {
        self.idle()?;
        Ok(id != 0 && self.entries.remove(&id).is_some())
    }
    pub(crate) fn remove_group(&mut self, group: u32) -> Result<()> {
        self.idle()?;
        self.entries.retain(|_, e| e.group != group);
        Ok(())
    }
    /// 789290: drop every controller.
    pub(crate) fn clear(&mut self) -> Result<()> {
        self.idle()?;
        self.entries.clear();
        Ok(())
    }
    pub(crate) fn has_tracks(&self) -> bool {
        !self.entries.is_empty()
    }
    pub(crate) fn contains_name(&mut self, name: &[u16]) -> Result<bool> {
        self.idle()?;
        ensure!(
            name.iter().all(|c| *c <= 127),
            "CatSystem2 motion-set non-ASCII case comparison profile"
        );
        //789AA0 resets output even when the queried name is absent.
        self.output = Some(self.base);
        Ok(self.entries.values().any(|e| {
            e.name.len() == name.len()
                && e.name
                    .iter()
                    .zip(name)
                    .all(|(a, b)| (*a as u8).eq_ignore_ascii_case(&(*b as u8)))
        }))
    }
    pub(crate) fn set_start(&mut self, id: u32, clock: u32) -> Result<bool> {
        self.idle()?;
        if id == 0 {
            for e in self.entries.values_mut() {
                e.start = clock;
            }
            return Ok(true);
        }
        let Some(entry) = self.entries.get_mut(&id) else {
            return Ok(false);
        };
        entry.start = clock;
        Ok(true)
    }
    pub(crate) fn set_parameters(&mut self, id: u32, parameters: &[u32]) -> Result<bool> {
        self.idle()?;
        let Some(e) = self.entries.get_mut(&id).filter(|_| id != 0) else {
            return Ok(false);
        };
        e.controller.set_parameters(parameters)?;
        Ok(true)
    }
    pub(crate) fn active(&self, id: u32) -> Result<bool> {
        self.idle()?;
        self.entries
            .get(&id)
            .filter(|_| id != 0)
            .map_or(Ok(false), |e| e.controller.active())
    }
    pub(crate) fn any_active(&self) -> Result<bool> {
        self.idle()?;
        for e in self.entries.values() {
            if e.controller.active()? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(crate) fn group_active(&self, group: u32) -> Result<bool> {
        self.idle()?;
        for e in self.entries.values().filter(|e| e.group == group) {
            if e.controller.active()? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(crate) fn skip(&mut self, id: u32) -> Result<()> {
        self.idle()?;
        if let Some(e) = self.entries.get_mut(&id).filter(|_| id != 0) {
            e.controller.request_skip()?;
        }
        Ok(())
    }
    pub(crate) fn skip_group(&mut self, group: u32) -> Result<()> {
        self.idle()?;
        for e in self.entries.values_mut().filter(|e| e.group == group) {
            e.controller.request_skip()?;
        }
        Ok(())
    }
    pub(crate) fn skip_all(&mut self) -> Result<()> {
        self.idle()?;
        for e in self.entries.values_mut() {
            e.controller.request_skip()?;
        }
        Ok(())
    }
    pub(crate) fn evaluate(
        &mut self,
        clock: u32,
        freeze_unstarted: bool,
        budget: u32,
    ) -> Result<Evaluation> {
        self.evaluate_inner(
            Mode::Frame {
                clock,
                freeze_unstarted,
            },
            budget,
            None,
        )
    }
    pub(crate) fn terminal(&mut self, budget: u32) -> Result<Evaluation> {
        self.evaluate_inner(Mode::Terminal, budget, None)
    }
    pub(crate) fn evaluate_with_services(
        &mut self,
        clock: u32,
        freeze_unstarted: bool,
        budget: u32,
        client: ClientId,
        host: &mut dyn HostServices,
        random: &mut RandomStreams,
    ) -> Result<Evaluation> {
        self.services(
            Mode::Frame {
                clock,
                freeze_unstarted,
            },
            budget,
            client,
            host,
            random,
        )
    }
    pub(crate) fn terminal_with_services(
        &mut self,
        budget: u32,
        client: ClientId,
        host: &mut dyn HostServices,
        random: &mut RandomStreams,
    ) -> Result<Evaluation> {
        self.services(Mode::Terminal, budget, client, host, random)
    }
    fn services(
        &mut self,
        mode: Mode,
        budget: u32,
        client: ClientId,
        host: &mut dyn HostServices,
        random: &mut RandomStreams,
    ) -> Result<Evaluation> {
        ensure!(
            random.belongs_to(client) && self.owner.is_none_or(|owner| owner == client),
            "CatSystem2 motion-set service owner mismatch"
        );
        if let Some(p) = &self.pending {
            ensure!(
                p.mode == mode,
                "CatSystem2 motion-set evaluation clock/mode belongs to an unfinished frame"
            );
        }
        if budget > 0 {
            self.owner = Some(client);
        }
        self.evaluate_inner(mode, budget, Some((client, host, random)))
    }
    fn evaluate_inner(
        &mut self,
        mode: Mode,
        mut budget: u32,
        mut services: Option<(ClientId, &mut dyn HostServices, &mut RandomStreams)>,
    ) -> Result<Evaluation> {
        if let Some(p) = &self.pending {
            ensure!(
                p.mode == mode,
                "CatSystem2 motion-set evaluation clock/mode belongs to an unfinished frame"
            );
        }
        if budget == 0 {
            return Ok(Evaluation::Budget);
        }
        if self.pending.is_none() {
            self.output = Some(self.base);
            self.pending = Some(Pending {
                mode,
                ids: self.entries.keys().copied().collect(),
                next: 0,
                seeded: false,
                visited: false,
            });
        }
        loop {
            let pending = self.pending.as_mut().unwrap();
            let Some(id) = pending.ids.get(pending.next).copied() else {
                let visited = pending.visited;
                self.pending = None;
                return Ok(Evaluation::Ready {
                    visited,
                    pose: self
                        .output
                        .context("CatSystem2 motion-set final output unavailable")?,
                });
            };
            if budget == 0 {
                return Ok(Evaluation::Budget);
            }
            let entry = self
                .entries
                .get_mut(&id)
                .context("CatSystem2 motion-set continuation lost original entry")?;
            if !pending.seeded {
                if let Mode::Frame {
                    clock,
                    freeze_unstarted,
                } = mode
                {
                    if clock != u32::MAX && entry.start == 0 {
                        if freeze_unstarted {
                            pending.next += 1;
                            budget -= 1;
                            continue;
                        }
                        entry.start = clock;
                    }
                }
                entry.controller.set_pose(
                    self.output
                        .context("CatSystem2 motion-set seeded output unavailable")?,
                )?;
                pending.seeded = true;
                pending.visited = true;
            }
            let clock = if let Mode::Frame { clock, .. } = mode {
                native_clock_value(clock,entry.start,self.speed)?
            } else {
                u32::MAX
            };
            let before = entry.controller.instructions();
            let tick = if let Some((client, host, random)) = services.as_mut() {
                if mode == Mode::Terminal {
                    entry.controller.evaluate_terminal_with_services(
                        budget,
                        *client,
                        &mut **host,
                        &mut **random,
                    )?
                } else {
                    entry.controller.tick_with_services(
                        clock,
                        budget,
                        *client,
                        &mut **host,
                        &mut **random,
                    )?
                }
            } else if mode == Mode::Terminal {
                entry.controller.evaluate_terminal(budget)?
            } else {
                entry.controller.tick(clock, budget)?
            };
            let used = u32::try_from(entry.controller.instructions() - before)
                .unwrap_or(u32::MAX)
                .max(1);
            budget = budget.saturating_sub(used);
            if matches!(tick, Tick::BudgetExhausted) {
                return Ok(Evaluation::Budget);
            }
            //78A4B0 copies the CURRENT controller pose even when delayed/inactive.
            //It was seeded from the previous controller's output exactly once above.
            self.output = Some(entry.controller.pose()?);
            pending.next += 1;
            pending.seeded = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const EX: u32 = 0x80000000;
    fn binary(code: &mut Vec<u8>, op: u8, dst: u32, flags: u8, src: u32) {
        code.push(op);
        code.push(3);
        code.extend(dst.to_le_bytes());
        code.push(flags);
        code.extend(src.to_le_bytes());
    }
    fn program(code: Vec<u8>) -> Arc<Program> {
        Arc::new(Program {
            code,
            globals_bytes: 0,
            stack_bytes: 1024,
            strings: Vec::new(),
        })
    }
    fn name(name: &str) -> Vec<u16> {
        name.encode_utf16().collect()
    }
    fn write_constant(slot: u32, value: u32, active: bool) -> Arc<Program> {
        let mut code = Vec::new();
        binary(&mut code, 0x15, EX + slot * 4, 3, value);
        code.push(0x2d);
        if !active {
            binary(&mut code, 0x2b, 38, 3, 0);
        }
        code.push(0x3b);
        program(code)
    }
    fn result(e: Evaluation) -> PoseWords {
        let Evaluation::Ready { pose, .. } = e else {
            panic!("actual frame did not finish")
        };
        pose
    }
    #[test]
    fn constructor_and_ordered_controller_chain_preserve_distinct_base_and_output() {
        let mut set = MotionSet::default();
        assert_eq!(set.base().0[28], u32::MAX);
        assert_eq!(set.output().unwrap(), None);
        assert!(set.commit_output().is_err());
        let first = set
            .insert(name("a"), write_constant(0, 7, false), &[], None, 0, 0, 0)
            .unwrap();
        let mut code = Vec::new();
        binary(&mut code, 0x15, EX + 4, 0x13, EX);
        code.extend([0x2d, 0x3b]);
        let second = set
            .insert(name("b"), program(code), &[], None, 1, 0, 0)
            .unwrap();
        let pose = result(set.evaluate(10, false, 100).unwrap());
        assert_eq!(&pose.0[..2], &[7, 7]);
        assert_eq!(set.base().0[0], 0);
        assert!(!set.active(first).unwrap());
        assert!(set.active(second).unwrap());
        assert!(set.group_active(1).unwrap());
        assert!(set.any_active().unwrap());
        let pose = result(set.evaluate(11, false, 100).unwrap());
        assert_eq!(&pose.0[..2], &[0, 0]);
        assert!(set.has_tracks());
        set.remove_group(1).unwrap();
        assert!(!set.any_active().unwrap());
        assert!(set.has_tracks());
        set.remove(first).unwrap();
        assert!(!set.has_tracks());
    }
    #[test]
    fn budget_continuation_never_reseeds_or_replays_prior_controllers() {
        let mut set = MotionSet::default();
        set.insert(name("a"), write_constant(0, 7, true), &[], None, 0, 10, 0)
            .unwrap();
        let mut code = Vec::new();
        binary(&mut code, 0x15, EX + 4, 0x13, EX);
        binary(&mut code, 0x15, EX + 8, 3, 9);
        code.extend([0x2d, 0x3b]);
        set.insert(name("b"), program(code), &[], None, 0, 10, 0)
            .unwrap();
        assert_eq!(set.evaluate(10, false, 4).unwrap(), Evaluation::Budget);
        assert!(set.evaluate(11, false, 0).is_err());
        assert!(set.terminal(100).is_err());
        assert!(set.set_start(0, 11).is_err());
        assert!(set.set_base(PoseWords([9; 36])).is_err());
        assert!(set.skip_all().is_err());
        let pose = result(set.evaluate(10, false, 100).unwrap());
        assert_eq!(&pose.0[..3], &[7, 7, 9]);
    }
    #[test]
    fn native_start_sentinel_scale_delay_and_frozen_unstarted_entries() {
        let mut set = MotionSet::default();
        let mut code = Vec::new();
        binary(&mut code, 0x15, EX, 0x13, EX + 144);
        code.extend([0x2d, 0x3b]);
        let id = set
            .insert(name("time"), program(code), &[], None, 0, 0, 0)
            .unwrap();
        assert!(matches!(
            set.evaluate(10, true, 100).unwrap(),
            Evaluation::Ready { visited: false, .. }
        ));
        assert_eq!(result(set.evaluate(10, false, 100).unwrap()).0[0], 0);
        set.set_speed(1.5).unwrap();
        assert_eq!(result(set.evaluate(12, false, 100).unwrap()).0[0], 3);
        set.set_start(id, 12).unwrap();
        assert_eq!(result(set.evaluate(13, false, 100).unwrap()).0[0], 2);
        assert!(set.set_speed(f32::NAN).is_err());
        assert!(set.set_speed(-1.).is_err());
    }
    #[test]
    fn terminal_and_group_skip_execute_actual_program_before_base_commit() {
        let mut set = MotionSet::default();
        let mut code = Vec::new();
        binary(&mut code, 0x15, EX, 0x13, EX + 144);
        binary(&mut code, 0x15, EX + 4, 0x13, EX + 152);
        code.extend([0x2d, 0x3b]);
        let id = set
            .insert(name("fade"), program(code), &[], Some(b"owned"), 7, 4, 10)
            .unwrap();
        assert_eq!(result(set.evaluate(5, false, 100).unwrap()).0[0], 0); //Delay: no fabricated sample.
        set.skip_group(7).unwrap();
        let p = result(set.evaluate(6, false, 100).unwrap());
        assert_eq!(&p.0[..2], &[u32::MAX - 7, 1]);
        let p = result(set.terminal(100).unwrap());
        assert_eq!(&p.0[..2], &[u32::MAX, 1]);
        assert!(set.active(id).unwrap());
        set.commit_output().unwrap();
        assert_eq!(set.base(), p);
        let mut changed = PoseWords([0; 36]);
        changed.0[0] = 33;
        set.set_output(changed).unwrap();
        assert!(set.contains_name(&name("FADE")).unwrap());
        assert_eq!(set.output().unwrap(), Some(set.base()));
        assert!(!set.contains_name(&name("absent")).unwrap());
    }
    #[test]
    fn shared_log_receipts_resume_order_once_and_reject_foreign_service_owners() -> Result<()> {
        let root = tempfile::tempdir()?;
        let project: crysirror_runtime::Project = serde_json::from_value(serde_json::json!({
            "schema":2,"name":"authored motion set","content_revision":"1","canvas":[4,4],"assets":"game",
            "clients":[{"name":"native","adapter":"crysirror-script-v1","game":"unused"}]}))?;
        let mut runtime = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )?;
        let mut code = Vec::new();
        binary(&mut code, 0x15, EX, 3, EX + 156);
        binary(&mut code, 0x2b, 0, 3, 4);
        code.push(0x3b);
        let program = program(code);
        let mut set = MotionSet::default();
        let first = set.insert(
            name("first"),
            program.clone(),
            &[u32::from_le_bytes(*b"one\0")],
            None,
            0,
            1,
            0,
        )?;
        set.insert(
            name("second"),
            program,
            &[u32::from_le_bytes(*b"two\0")],
            None,
            1,
            1,
            0,
        )?;
        let mut random = RandomStreams::new(ClientId(1), 1, 0);
        assert_eq!(
            set.evaluate_with_services(1, false, 2, ClientId(1), &mut runtime.host, &mut random)?,
            Evaluation::Budget
        );
        assert!(set.output().is_err());
        assert!(set.any_active().is_err());
        let records = runtime.host.log_snapshot(ClientId(1))?.records;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "one");
        let mut foreign = RandomStreams::new(ClientId(2), 1, 0);
        assert!(
            set.evaluate_with_services(1, false, 100, ClientId(2), &mut runtime.host, &mut foreign)
                .is_err()
        );
        assert!(matches!(
            set.evaluate_with_services(1, false, 100, ClientId(1), &mut runtime.host, &mut random)?,
            Evaluation::Ready { visited: true, .. }
        ));
        let records = runtime.host.log_snapshot(ClientId(1))?.records;
        assert_eq!(
            records
                .iter()
                .map(|r| r.message.as_str())
                .collect::<Vec<_>>(),
            ["one", "two"]
        );
        assert!(runtime.host.log_snapshot(ClientId(2)).is_err());
        set.skip(first)?;
        set.set_parameters(first, &[u32::from_le_bytes(*b"end\0")])?;
        assert!(matches!(
            set.terminal_with_services(100, ClientId(1), &mut runtime.host, &mut random)?,
            Evaluation::Ready { visited: true, .. }
        ));
        let records = runtime.host.log_snapshot(ClientId(1))?.records;
        assert_eq!(
            records
                .iter()
                .map(|r| r.message.as_str())
                .collect::<Vec<_>>(),
            ["one", "two", "end", "two"]
        );
        Ok(())
    }
    #[test]
    fn native_clock_lowering_uses_signed64_low_word_at_float32_rounding_boundaries(){
        assert_eq!(native_clock_value(0,60,1.).unwrap(),0,"native stop unsigned delta rounds to2^32 then EAX0");
        assert_eq!(native_clock_value(0,129,1.).unwrap(),0xffffff00);
        assert_eq!(native_clock_value(0,256,1.).unwrap(),0xffffff00);
        assert_eq!(native_clock_value(0,0x80000000,1.).unwrap(),0x80000000);
        assert_eq!(native_clock_value(u32::MAX,0,2.).unwrap(),0,"2^33 also retains only low DWORD");
        assert_eq!(native_clock_value(100,100,1.).unwrap(),0);
        assert_eq!(native_clock_value(101,100,1.5).unwrap(),2);
        assert!(native_clock_value(100,0,f32::INFINITY).is_err());
        assert!(native_clock_value(u32::MAX,0,2147483648.).is_err(),"invalid signed64 lowering keeps explicit FPU-profile boundary");
    }

}
