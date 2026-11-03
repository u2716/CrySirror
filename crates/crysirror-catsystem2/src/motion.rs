// SPDX-License-Identifier: MPL-2.0
//! Original motion-program KCS role (78A0B0..78A6A0), reusing the same
//! first-class bytecode VM. This module owns native globals/parameters/tick
//! policy only. It has no interpolation graph, renderer, image code or mixer.
//! Pose publication and unsupported numeric profiles remain explicit boundaries.
//! Geometry and random streams use shared kernels; debug uses engine logs.
use crate::kcs::{Event, Machine, NativeCall, Program};
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    service::{HostServices, MAX_LOG_LINE_BYTES},
    state::ClientId,
};
use crysirror_resource::legacy_stream::Mt19937Split;
use std::sync::Arc;

const EXTERNAL_BYTES: u32 = 1192;
const EXTERNAL: u32 = 0x8000_0000;
const FRAME: u32 = 144;
const ACTIVE: u32 = 148;
const SKIP: u32 = 152;
const PARAMETERS: u32 = 156;
const CUSTOM_ADDRESS: u32 = 1180;
const CUSTOM_SIZE: u32 = 1184;

/// Original first144 bytes, without native pointers. Interpretation and actual
/// scoped engine publication depend on the original object class, not this VM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoseWords(pub [u32; 36]);
/// A completed evaluation is distinct from a completed motion. A program can
/// finish its per-frame KCS entry while keeping its native active flag set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tick {
    Delayed,
    Inactive,
    BudgetExhausted,
    Ready {
        clock: u32,
        active: bool,
        pose: PoseWords,
    },
}
/// Original7C40B0 policy: two independent split-high-word streams. Explicit
/// float/integer seeds come from the owning native session; this type never
/// invents a game seed or draws from process/thread-global randomness.
#[derive(Clone, Debug)]
pub struct RandomStreams {
    client: ClientId,
    float: Mt19937Split,
    integer: Mt19937Split,
}
impl RandomStreams {
    pub(crate) fn belongs_to(&self,client:ClientId)->bool {self.client==client}
    pub(crate) fn integer_word(&mut self,client:ClientId)->Result<u32>{
        ensure!(self.client==client,"CatSystem2 foreign native integer random stream");Ok(self.integer.next_u32())
    }
    pub fn new(client: ClientId, float_seed: u32, integer_seed: u32) -> Self {
        Self {
            client,
            float: Mt19937Split::new(float_seed),
            integer: Mt19937Split::new(integer_seed),
        }
    }
}
struct Services<'a> {
    client: ClientId,
    host: &'a mut dyn HostServices,
    random: &'a mut RandomStreams,
}
#[derive(Clone, Debug)]
pub struct Controller {
    program: Arc<Program>,
    machine: Machine,
    delay: u32,
    admitted_clock: Option<u32>,
    terminal_saved: Option<[u32; 3]>,
    owner: Option<ClientId>,
    log_failure: Option<(u32, String)>,
}
impl Controller {
    pub fn new(
        program: Arc<Program>,
        pose: PoseWords,
        parameters: &[u32],
        delay: u32,
    ) -> Result<Self> {
        //The reviewed role uses the1192-byte external block supplied to
        //8773F0, not a private native object address or executable pointer.
        ensure!(
            program.globals_bytes == 0 && program.strings.is_empty(),
            "CatSystem2 motion global/static-string layout outside reviewed external-block profile"
        );
        let mut machine = Machine::new(&program, EXTERNAL_BYTES)?;
        for (i, value) in pose.0.iter().enumerate() {
            machine.write_bytes(EXTERNAL + i as u32 * 4, &value.to_le_bytes())?;
        }
        machine.write_bytes(EXTERNAL + ACTIVE, &1u32.to_le_bytes())?;
        machine.write_bytes(EXTERNAL + SKIP, &0u32.to_le_bytes())?;
        let mut controller = Self {
            program,
            machine,
            delay,
            admitted_clock: None,
            terminal_saved: None,
            owner: None,
            log_failure: None,
        };
        controller.set_parameters(parameters)?;
        Ok(controller)
    }
    ///78A340 copies at most256 raw argument words and zeroes the unused tail.
    ///This is original argument-bank policy; no numeric coercion is implied.
    pub fn set_parameters(&mut self, parameters: &[u32]) -> Result<()> {
        ensure!(
            self.admitted_clock.is_none(),
            "CatSystem2 motion parameters are owned by an unfinished tick"
        );
        let count = parameters.len().min(256);
        for index in 0..256 {
            let value = parameters
                .get(index)
                .copied()
                .filter(|_| index < count)
                .unwrap_or(0);
            self.machine.write_bytes(
                EXTERNAL + PARAMETERS + index as u32 * 4,
                &value.to_le_bytes(),
            )?;
        }
        self.machine
            .write_bytes(EXTERNAL + ACTIVE, &1u32.to_le_bytes())?;
        Ok(())
    }
    ///78A5C0 only sets the original program flag. The next real KCS evaluation
    ///decides endpoints, parameter updates and completion; this never cancels a
    ///shared track or fabricates a final alpha/transform.
    pub fn request_skip(&mut self) -> Result<()> {
        ensure!(
            self.admitted_clock.is_none(),
            "CatSystem2 motion skip flag is owned by an unfinished tick"
        );
        self.machine
            .write_bytes(EXTERNAL + SKIP, &1u32.to_le_bytes())
    }
    pub fn active(&self) -> Result<bool> {
        Ok(self.machine.read_word(EXTERNAL + ACTIVE)? != 0)
    }
    pub fn pending_native(&self) -> Option<&NativeCall> {
        self.machine.pending.as_ref()
    }
    pub fn instructions(&self) -> u64 {
        self.machine.instructions
    }
    pub fn pose(&self) -> Result<PoseWords> {
        let mut words = [0; 36];
        for (index, value) in words.iter_mut().enumerate() {
            *value = self.machine.read_word(EXTERNAL + index as u32 * 4)?;
        }
        Ok(PoseWords(words))
    }
    ///78A480 copies the caller's complete pose before a frame evaluation.
    pub fn set_pose(&mut self, pose: PoseWords) -> Result<()> {
        ensure!(
            self.admitted_clock.is_none(),
            "CatSystem2 motion pose is owned by an unfinished tick"
        );
        let bytes = pose
            .0
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        self.machine.write_bytes(EXTERNAL, &bytes)
    }
    ///78A3F0 copies at most256 raw words; it never coerces floats to integers.
    pub fn parameters(&self, count: usize) -> Result<Vec<u32>> {
        (0..count.min(256))
            .map(|i| self.machine.read_word(EXTERNAL + PARAMETERS + i as u32 * 4))
            .collect()
    }
    pub fn delay(&self) -> u32 {
        self.delay
    }
    ///78A670 stores the original unsigned delay. Owner mutation cannot race
    ///with an unfinished admitted frame.
    pub fn set_delay(&mut self, delay: u32) -> Result<()> {
        ensure!(
            self.admitted_clock.is_none(),
            "CatSystem2 motion delay is owned by an unfinished tick"
        );
        self.delay = delay;
        Ok(())
    }
    ///78A5D0 copies a custom byte block exactly once. The KCS address is an
    ///offset in this VM's separately owned allocation, not a host pointer. A
    ///second call returnsfalse without changing data or the native reference.
    pub fn set_custom_data(&mut self, bytes: &[u8]) -> Result<bool> {
        ensure!(
            self.admitted_clock.is_none(),
            "CatSystem2 motion custom data is owned by an unfinished tick"
        );
        if self.machine.read_word(EXTERNAL + CUSTOM_ADDRESS)? != 0 {
            return Ok(false);
        }
        let size = u32::try_from(bytes.len()).context("CatSystem2 motion custom data extent")?;
        let address = self.machine.allocate_memory(bytes.len())?;
        self.machine.write_bytes(address, bytes)?;
        self.machine
            .write_bytes(EXTERNAL + CUSTOM_ADDRESS, &address.to_le_bytes())?;
        self.machine
            .write_bytes(EXTERNAL + CUSTOM_SIZE, &size.to_le_bytes())?;
        Ok(true)
    }
    ///78A630 reads the original pointer/size words after actual VM execution;
    ///it does not return the stale input copy or decode bytes as a host string.
    pub fn custom_data(&self) -> Result<Option<Vec<u8>>> {
        let address = self.machine.read_word(EXTERNAL + CUSTOM_ADDRESS)?;
        if address == 0 {
            return Ok(None);
        }
        let size = self.machine.read_word(EXTERNAL + CUSTOM_SIZE)? as usize;
        Ok(Some(self.machine.read_bytes(address, size)?))
    }
    pub fn tick(&mut self, clock: u32, budget: u32) -> Result<Tick> {
        self.tick_inner(clock, budget, None, false)
    }
    /// Admits shared effects under one native owner. Streams can be borrowed
    /// across controllers in original evaluation order. A partial tick retains
    /// its clock and owner; another client cannot resume it or consume a draw.
    pub fn tick_with_services(
        &mut self,
        clock: u32,
        budget: u32,
        client: ClientId,
        host: &mut dyn HostServices,
        random: &mut RandomStreams,
    ) -> Result<Tick> {
        ensure!(
            random.client == client,
            "CatSystem2 motion random owner mismatch"
        );
        ensure!(
            self.owner.is_none_or(|owner| owner == client),
            "CatSystem2 motion controller owner mismatch"
        );
        if budget > 0 {
            self.owner = Some(client);
        }
        self.tick_inner(
            clock,
            budget,
            Some(Services {
                client,
                host,
                random,
            }),
            false,
        )
    }
    ///78A540 evaluates the real original entry at frame-1 with skip1, then
    ///restores only frame/active/skip. Pose, parameters and custom data changes
    ///remain actual program output. This differs from terminating a controller.
    pub fn evaluate_terminal(&mut self, budget: u32) -> Result<Tick> {
        self.terminal_inner(budget, None)
    }
    pub fn evaluate_terminal_with_services(
        &mut self,
        budget: u32,
        client: ClientId,
        host: &mut dyn HostServices,
        random: &mut RandomStreams,
    ) -> Result<Tick> {
        ensure!(
            random.client == client && self.owner.is_none_or(|owner| owner == client),
            "CatSystem2 motion terminal owner mismatch"
        );
        if budget > 0 {
            self.owner = Some(client);
        }
        self.terminal_inner(
            budget,
            Some(Services {
                client,
                host,
                random,
            }),
        )
    }
    fn terminal_inner(&mut self, budget: u32, services: Option<Services<'_>>) -> Result<Tick> {
        if budget == 0 {
            return Ok(Tick::BudgetExhausted);
        }
        if self.terminal_saved.is_none() {
            ensure!(
                self.admitted_clock.is_none(),
                "CatSystem2 motion terminal cannot replace an unfinished frame"
            );
            let saved = [
                self.machine.read_word(EXTERNAL + FRAME)?,
                self.machine.read_word(EXTERNAL + ACTIVE)?,
                self.machine.read_word(EXTERNAL + SKIP)?,
            ];
            self.machine
                .write_bytes(EXTERNAL + SKIP, &1u32.to_le_bytes())?;
            self.terminal_saved = Some(saved);
        }
        self.tick_inner(u32::MAX, budget, services, true)
    }
    fn tick_inner(
        &mut self,
        clock: u32,
        mut budget: u32,
        mut services: Option<Services<'_>>,
        terminal: bool,
    ) -> Result<Tick> {
        ensure!(
            terminal == self.terminal_saved.is_some(),
            "CatSystem2 motion evaluation mode belongs to an unfinished terminal entry"
        );
        if budget == 0 {
            return Ok(Tick::BudgetExhausted);
        }
        if let Some(admitted) = self.admitted_clock {
            ensure!(
                clock == admitted,
                "CatSystem2 motion cannot replace the clock of an unfinished tick"
            );
        } else {
            if !terminal && !self.active()? {
                return Ok(Tick::Inactive);
            }
            let skip = self.machine.read_word(EXTERNAL + SKIP)? != 0;
            //78A4E0 deliberately compares unsigned absolute clocks before
            //subtracting delay. Do not silently repair its wrap behavior.
            if !terminal && !skip && self.delay > clock {
                return Ok(Tick::Delayed);
            }
            self.machine.write_bytes(
                EXTERNAL + FRAME,
                &(if terminal {
                    u32::MAX
                } else {
                    clock.wrapping_sub(self.delay)
                })
                .to_le_bytes(),
            )?;
            //877270 restarts PC/stack/frame bases, while retaining actual
            //external data, parameters and stack allocation across frames.
            self.machine.pc = 0;
            self.machine.stack = self.program.globals_bytes;
            self.machine.frame_base = self.program.globals_bytes;
            self.machine.frame_end = self.program.globals_bytes;
            self.machine.halted = false;
            ensure!(
                self.machine.pending.is_none(),
                "CatSystem2 motion restart has a pending native"
            );
            self.admitted_clock = Some(clock);
        }
        while budget > 0 {
            let before = self.machine.instructions;
            let event = self.machine.step(&self.program, budget)?;
            budget = budget.saturating_sub(
                u32::try_from(self.machine.instructions - before).unwrap_or(u32::MAX),
            );
            match event {
                Event::BudgetExhausted => return Ok(Tick::BudgetExhausted),
                Event::Halted => return self.ready(clock),
                Event::Native(call) => {
                    if matches!(call.id, 0 | 39 | 40) {
                        self.service_native(&call,services.as_mut()).with_context(||format!(
                            "CatSystem2 motion native {} at original PC {:#x} remains unacknowledged",call.id,call.pc))?;
                        continue;
                    }
                    if crate::motion_math::handles(call.id) {
                        let(result,writes)=crate::motion_math::prepare(&self.machine,&call).with_context(||format!(
                            "CatSystem2 motion geometric native {} at original PC {:#x} remains unacknowledged",call.id,call.pc))?;
                        self.machine.commit_native(result, &writes)?;
                        continue;
                    }
                    if call.id == 38 {
                        ensure!(call.arguments.is_empty(), "CatSystem2 motion end arguments");
                        self.machine.preflight_native(false, &[])?;
                        //78B230 writes original this39 (external148), then
                        //returns native4. It is not a generic animation stop.
                        self.machine
                            .write_bytes(EXTERNAL + ACTIVE, &0u32.to_le_bytes())?;
                        self.machine.acknowledge(None)?;
                        return self.ready(clock);
                    }
                    let value=scalar(call.id,&call.arguments).with_context(||format!(
                        "CatSystem2 motion native {} at original PC {:#x} remains unacknowledged",call.id,call.pc))?;
                    self.machine.acknowledge(Some(value))?;
                }
            }
        }
        Ok(Tick::BudgetExhausted)
    }
    fn service_native(
        &mut self,
        call: &NativeCall,
        services: Option<&mut Services<'_>>,
    ) -> Result<()> {
        let services =
            services.context("CatSystem2 motion requires explicit shared RNG/debug services")?;
        ensure!(
            self.owner == Some(services.client) && services.random.client == services.client,
            "CatSystem2 motion effect owner mismatch"
        );
        ensure!(
            self.machine.pending.as_ref() == Some(call),
            "CatSystem2 motion effect pending mismatch"
        );
        match call.id {
            39 | 40 => {
                ensure!(
                    call.arguments.is_empty(),
                    "CatSystem2 motion random arguments"
                );
                self.machine.preflight_native(true, &[])?;
                //5C2140 returns all32 bits;5C1FB0 scales double by1/(2^32-1)
                //then returnsf32. Both endpoints are representable/inclusive.
                let word = if call.id == 39 {
                    services.random.integer.next_u32()
                } else {
                    ((f64::from(services.random.float.next_u32()) / 4294967295.0) as f32).to_bits()
                };
                self.machine.acknowledge(Some(word))?;
            }
            0 => {
                ensure!(
                    call.arguments.len() == 1,
                    "CatSystem2 motion debug arguments"
                );
                if let Some((pc, error)) = &self.log_failure {
                    ensure!(
                        *pc == call.pc,
                        "CatSystem2 motion retained log failure belongs to another call"
                    );
                    anyhow::bail!("{error}");
                }
                let bytes = crate::native_values::format_bytes(&self.machine, call.arguments[0])?;
                let message = crysirror_resource::text_stream::decode_shift_jis(&bytes, 4096)?;
                ensure!(
                    message.len() <= MAX_LOG_LINE_BYTES,
                    "CatSystem2 motion decoded debug byte limit"
                );
                self.machine.preflight_native(false, &[])?;
                //A failed/foreign receipt is retained at this original call.
                //Polling the same pending call must not append duplicate logs.
                let published = services
                    .host
                    .append_log(services.client, &message)
                    .map_err(anyhow::Error::from)
                    .and_then(|receipt| {
                        ensure!(
                            receipt.client == services.client,
                            "CatSystem2 motion debug foreign receipt"
                        );
                        Ok(())
                    });
                if let Err(error) = published {
                    self.log_failure = Some((call.pc, format!("{error:#}")));
                    return Err(error);
                }
                self.machine.acknowledge(None)?;
            }
            _ => anyhow::bail!("CatSystem2 motion invalid service native"),
        }
        Ok(())
    }
    fn ready(&mut self, clock: u32) -> Result<Tick> {
        if let Some([frame, active, skip]) = self.terminal_saved {
            self.machine
                .write_bytes(EXTERNAL + FRAME, &frame.to_le_bytes())?;
            self.machine
                .write_bytes(EXTERNAL + ACTIVE, &active.to_le_bytes())?;
            self.machine
                .write_bytes(EXTERNAL + SKIP, &skip.to_le_bytes())?;
            self.terminal_saved = None;
        }
        let pose = self.pose()?;
        let active = self.active()?;
        self.admitted_clock = None;
        Ok(Tick::Ready {
            clock,
            active,
            pose,
        })
    }
}

///Native scalar arithmetic, not an engine animation implementation. Geometric
///matrix operations are mapped separately; shared RNG/debug stay explicit.
fn scalar(id: u32, words: &[u32]) -> Result<u32> {
    let arity=match id {1..=6|8..=10|12|14..=17=>1,7|11|13|42=>2,41=>5,
        _=>anyhow::bail!("unsupported original motion native family {id}; shared RNG/debug mappings are not fabricated")};
    ensure!(
        words.len() == arity,
        "CatSystem2 motion scalar native argument count"
    );
    let f = |index: usize| f32::from_bits(words[index]);
    let a = f(0);
    let result = match id {
        1 => (f64::from(a).sin() as f32).to_bits(),
        2 => (f64::from(a).cos() as f32).to_bits(),
        3 => (f64::from(a).tan() as f32).to_bits(),
        4 => (f64::from(a).asin() as f32).to_bits(),
        5 => (f64::from(a).acos() as f32).to_bits(),
        6 => (f64::from(a).atan() as f32).to_bits(),
        7 => (f64::from(a).atan2(f64::from(f(1))) as f32).to_bits(),
        8 => (f64::from(a).ceil() as f32).to_bits(),
        9 => a.abs().to_bits(),
        10 => (f64::from(a).floor() as f32).to_bits(),
        11 => ((f64::from(a) % f64::from(f(1))) as f32).to_bits(),
        12 => (f64::from(a).sqrt() as f32).to_bits(),
        13 => (f64::from(a).powf(f64::from(f(1))) as f32).to_bits(),
        14 => ((a * std::f32::consts::PI) / 180.).to_bits(),
        15 => ((a * 180.) / std::f32::consts::PI).to_bits(),
        16 => (words[0] as i32).wrapping_abs() as u32,
        17 => x86_integer((f64::from(a + 0.5).floor()) as f32),
        41 => {
            ensure!(words.iter().all(|word|f32::from_bits(*word).is_finite()),
                "CatSystem2 motion lerp nonfinite comparison profile requires native unordered handling");
            let (from, to, min, max, x) = (a, f(1), f(2), f(3), f(4));
            (if min < x {
                if x < max {
                    ((x - min) / (max - min)) * (to - from) + from
                } else {
                    to
                }
            } else {
                from
            })
            .to_bits()
        }
        42 => {
            ensure!(
                a.is_finite() && f(1).is_finite(),
                "CatSystem2 motion waveform nonfinite profile"
            );
            let time = a.abs();
            let period = f(1).abs();
            let phase = if time > 0. {
                (f64::from(time) % f64::from(period)) as f32
            } else {
                0.
            };
            let quarter = period * 0.25;
            let remainder = if phase > 0. {
                (f64::from(phase) % f64::from(quarter)) as f32
            } else {
                0.
            };
            let mut value = remainder / quarter;
            if quarter <= phase {
                value = if quarter * 2. > phase {
                    1. - value
                } else if quarter * 3. <= phase {
                    value - 1.
                } else {
                    -value
                };
            }
            value.to_bits()
        }
        _ => unreachable!(),
    };
    Ok(result)
}
fn x86_integer(value: f32) -> u32 {
    if !value.is_finite() || value < -2147483648. || value >= 2147483648. {
        0x80000000
    } else {
        value as i32 as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn operand(code: &mut Vec<u8>, flags: u8, value: u32) {
        code.push(flags);
        code.extend(value.to_le_bytes());
    }
    fn binary(code: &mut Vec<u8>, op: u8, a: (u8, u32), b: (u8, u32)) {
        code.push(op);
        operand(code, a.0, a.1);
        operand(code, b.0, b.1);
    }
    fn program(code: Vec<u8>) -> Arc<Program> {
        Arc::new(Program {
            code,
            globals_bytes: 0,
            stack_bytes: 1024,
            strings: Vec::new(),
        })
    }
    #[test]
    fn raw_parameters_and_native_end_preserve_pose_instead_of_inventing_an_endpoint() {
        let mut code = Vec::new();
        binary(
            &mut code,
            0x15,
            (3, EXTERNAL + 12),
            (0x13, EXTERNAL + PARAMETERS),
        );
        code.push(0x2d);
        binary(&mut code, 0x2b, (3, 38), (3, 0));
        code.push(0x3b);
        let mut pose = PoseWords([0; 36]);
        pose.0[28] = 0x12345678;
        let mut controller = Controller::new(program(code), pose, &[5f32.to_bits()], 0).unwrap();
        let Tick::Ready {
            active: false,
            pose,
            ..
        } = controller.tick(4, 100).unwrap()
        else {
            panic!("native end did not complete actual frame");
        };
        assert_eq!(pose.0[3], 5f32.to_bits());
        assert_eq!(pose.0[28], 0x12345678);
        assert_eq!(controller.tick(5, 100).unwrap(), Tick::Inactive);
    }
    #[test]
    fn entry_restart_retains_external_state_and_budget_owns_the_admitted_clock() {
        let mut code = Vec::new();
        binary(&mut code, 0x15, (3, EXTERNAL), (0x13, EXTERNAL + FRAME));
        code.push(0x2d);
        code.push(0x3b);
        let mut controller = Controller::new(program(code), PoseWords([0; 36]), &[7], 2).unwrap();
        assert_eq!(controller.tick(1, 20).unwrap(), Tick::Delayed);
        assert_eq!(controller.tick(6, 1).unwrap(), Tick::BudgetExhausted);
        assert!(controller.tick(7, 20).is_err());
        assert!(controller.request_skip().is_err());
        let Tick::Ready {
            active: true, pose, ..
        } = controller.tick(6, 20).unwrap()
        else {
            panic!()
        };
        assert_eq!(pose.0[0], 4);
        let Tick::Ready {
            active: true, pose, ..
        } = controller.tick(7, 20).unwrap()
        else {
            panic!()
        };
        assert_eq!(pose.0[0], 5);
        assert_eq!(
            controller.machine.read_word(EXTERNAL + PARAMETERS).unwrap(),
            7
        );
        assert!(controller.instructions() > 3);
    }
    #[test]
    fn unsupported_native_retains_exact_pc_and_skip_only_sets_original_program_flag() {
        let mut code = Vec::new();
        binary(&mut code, 0x2b, (3, 39), (3, 0));
        code.push(0x3b);
        let mut controller = Controller::new(program(code), PoseWords([0; 36]), &[], 100).unwrap();
        controller.request_skip().unwrap();
        assert_eq!(controller.pose().unwrap().0, [0; 36]);
        assert!(controller.active().unwrap());
        let error = controller.tick(1, 100).unwrap_err().to_string();
        let before = controller.instructions();
        assert!(error.contains("39") && error.contains("0x0"));
        assert_eq!(controller.pending_native().unwrap().id, 39);
        assert_eq!(controller.machine.pc, 0);
        assert_eq!(controller.tick(1, 100).unwrap_err().to_string(), error);
        assert_eq!(controller.instructions(), before);
        assert!(controller.active().unwrap());
    }
    #[test]
    fn scalar_rounding_wrap_and_waveform_are_native_numeric_values() {
        assert_eq!(scalar(16, &[i32::MIN as u32]).unwrap(), i32::MIN as u32);
        assert_eq!(scalar(17, &[(-1.5f32).to_bits()]).unwrap(), (-1i32) as u32);
        for value in [f32::NAN, f32::INFINITY, 2147483648.] {
            assert_eq!(scalar(17, &[value.to_bits()]).unwrap(), 0x80000000);
        }
        assert_eq!(
            f32::from_bits(
                scalar(
                    41,
                    &[
                        10f32.to_bits(),
                        20f32.to_bits(),
                        5f32.to_bits(),
                        15f32.to_bits(),
                        10f32.to_bits()
                    ]
                )
                .unwrap()
            ),
            15.
        );
        for (time, expected) in [(0f32, 0f32), (2., 1.), (4., 0.), (6., -1.), (8., 0.)] {
            assert_eq!(
                f32::from_bits(scalar(42, &[time.to_bits(), 8f32.to_bits()]).unwrap()),
                expected
            );
        }
        assert!(scalar(41, &[f32::NAN.to_bits(); 5]).is_err());
        assert!(scalar(1, &[]).is_err());
    }
    #[derive(Default)]
    struct LogHost {
        log: crysirror_core::service::LogBook,
        foreign: bool,
        appends: usize,
    }
    impl crysirror_core::service::FileSystem for LogHost {
        fn read(
            &mut self,
            _: ClientId,
            _: &crysirror_core::path::VfsPath,
        ) -> crysirror_core::Result<Vec<u8>> {
            Err(crysirror_core::Error::Unsupported("test filesystem"))
        }
        fn write_atomic(
            &mut self,
            _: ClientId,
            _: &crysirror_core::path::VfsPath,
            _: &[u8],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::ReadOnly)
        }
    }
    impl HostServices for LogHost {
        fn now_ms(&self) -> u64 {
            0
        }
        fn submit(
            &mut self,
            _: ClientId,
            _: &[crysirror_core::state::Command],
        ) -> crysirror_core::Result<()> {
            Err(crysirror_core::Error::Unsupported("test scene"))
        }
        fn append_log(
            &mut self,
            client: ClientId,
            message: &str,
        ) -> crysirror_core::Result<crysirror_core::service::LogReceipt> {
            self.appends += 1;
            self.log.append(
                if self.foreign { ClientId(999) } else { client },
                0,
                message,
            )
        }
    }
    #[test]
    fn independent_random_streams_preserve_native_word_and_inclusive_float_policies() {
        let mut code = Vec::new();
        for id in [39, 40, 39, 40] {
            binary(&mut code, 0x2b, (3, id), (3, 0));
        }
        code.push(0x3b);
        let mut controller = Controller::new(program(code), PoseWords([0; 36]), &[], 0).unwrap();
        let mut rng = RandomStreams::new(ClientId(1), 1, 0);
        let mut host = LogHost::default();
        assert!(matches!(
            controller
                .tick_with_services(0, 100, ClientId(1), &mut host, &mut rng)
                .unwrap(),
            Tick::Ready { .. }
        ));
        let expected = [
            477480905,
            ((3556162021f64 / 4294967295.) as f32).to_bits(),
            3667282373,
            ((4012392791f64 / 4294967295.) as f32).to_bits(),
        ];
        for (index, word) in expected.into_iter().enumerate() {
            assert_eq!(
                controller.machine.read_word(index as u32 * 4).unwrap(),
                word
            );
        }
        let mut cloned = rng.clone();
        assert_eq!(cloned.integer.next_u32(), 2229421029);
        assert_eq!(rng.integer.next_u32(), 2229421029);
        let instructions = controller.instructions();
        let mut foreign = RandomStreams::new(ClientId(2), 1, 0);
        assert!(controller
            .tick_with_services(1, 100, ClientId(2), &mut host, &mut foreign)
            .is_err());
        assert_eq!(controller.instructions(), instructions);
        assert_eq!(foreign.integer.next_u32(), 477480905);
    }
    #[test]
    fn invalid_random_result_slot_never_consumes_a_stream_word() {
        let mut code = Vec::new();
        for _ in 0..4 {
            code.push(0x2e);
            operand(&mut code, 3, 7);
        }
        binary(&mut code, 0x2b, (3, 39), (3, 0));
        code.push(0x3b);
        let mut narrow = (*program(code)).clone();
        narrow.stack_bytes = 16;
        let mut controller = Controller::new(Arc::new(narrow), PoseWords([0; 36]), &[], 0).unwrap();
        assert!(controller.tick(0, 100).is_err());
        let original = controller.machine.pending.clone().unwrap();
        let mut rng = RandomStreams::new(ClientId(1), 1, 0);
        let mut host = LogHost::default();
        assert!(controller
            .tick_with_services(0, 100, ClientId(1), &mut host, &mut rng)
            .is_err());
        assert_eq!(rng.clone().integer.next_u32(), 477480905);
        assert_eq!(controller.machine.pending, Some(original));
        assert_eq!(controller.machine.read_word(0).unwrap(), 7);
    }
    fn debug_controller() -> Controller {
        let mut code = Vec::new();
        //Assignment pushes its result as the native string argument. Bytes are
        //numeric VM data, never a host pointer or private font/text renderer.
        binary(&mut code, 0x15, (3, EXTERNAL), (3, EXTERNAL + 4));
        binary(&mut code, 0x2b, (3, 0), (3, 4));
        code.push(0x3b);
        let mut controller = Controller::new(program(code), PoseWords([0; 36]), &[], 0).unwrap();
        controller
            .machine
            .write_bytes(EXTERNAL + 4, &[0xc2, 0xa5, 0])
            .unwrap();
        controller
    }
    #[test]
    fn original_debug_uses_shared_scoped_log_and_retains_foreign_receipt_failure() {
        let mut host = LogHost {
            foreign: true,
            ..Default::default()
        };
        let mut rng = RandomStreams::new(ClientId(1), 0, 0);
        let mut controller = debug_controller();
        assert!(controller
            .tick_with_services(0, 100, ClientId(1), &mut host, &mut rng)
            .is_err());
        let pc = controller.pending_native().unwrap().pc;
        assert_eq!(host.log.snapshot(ClientId(999)).records[0].message, "ﾂ･");
        for _ in 0..2 {
            assert!(controller
                .tick_with_services(0, 100, ClientId(1), &mut host, &mut rng)
                .is_err());
        }
        assert_eq!(controller.pending_native().unwrap().pc, pc);
        assert_eq!(host.appends, 1);
        assert!(host.log.snapshot(ClientId(1)).records.is_empty());
    }
    #[test]
    fn original_debug_publishes_into_actual_engine_host_without_a_renderer() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("assets")).unwrap();
        let project = root.path().join("project.json");
        std::fs::write(&project,br#"{"schema":2,"name":"motion-log","content_revision":"1","canvas":[64,64],"assets":"assets","clients":[{"name":"game","adapter":"crysirror-script-v1","game":"empty"}]}"#).unwrap();
        let mut runtime =
            crysirror_runtime::Runtime::load(&project, &root.path().join("state")).unwrap();
        let mut rng = RandomStreams::new(ClientId(1), 0, 0);
        let mut controller = debug_controller();
        assert!(matches!(
            controller
                .tick_with_services(0, 100, ClientId(1), &mut runtime.host, &mut rng)
                .unwrap(),
            Tick::Ready { .. }
        ));
        assert!(controller.pending_native().is_none());
        let records = runtime.host.log_snapshot(ClientId(1)).unwrap().records;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "ﾂ･");
        assert!(runtime.host.log_snapshot(ClientId(2)).is_err());
    }
    #[test]
    fn native_custom_data_is_set_once_and_actual_bytecode_mutates_its_owned_block() {
        let mut code = Vec::new();
        code.push(0x2f);
        operand(&mut code, 3, EXTERNAL + CUSTOM_ADDRESS);
        binary(&mut code, 0x15, (0x43, 0), (3, 0xdeadbeef));
        code.push(0x2d);
        code.push(0x3b);
        let mut controller = Controller::new(program(code), PoseWords([0; 36]), &[], 0).unwrap();
        assert_eq!(controller.custom_data().unwrap(), None);
        assert!(controller.set_custom_data(&[]).is_err());
        assert_eq!(controller.custom_data().unwrap(), None);
        assert!(controller
            .set_custom_data(&[1, 2, 3, 4, 5, 6, 7, 8])
            .unwrap());
        assert!(!controller.set_custom_data(&[9; 8]).unwrap());
        let untouched = controller.clone();
        assert!(matches!(
            controller.tick(0, 1).unwrap(),
            Tick::BudgetExhausted
        ));
        assert!(controller.set_custom_data(&[9; 8]).is_err());
        assert!(matches!(
            controller.tick(0, 100).unwrap(),
            Tick::Ready { .. }
        ));
        assert_eq!(
            controller.custom_data().unwrap().unwrap(),
            [0xef, 0xbe, 0xad, 0xde, 5, 6, 7, 8]
        );
        assert_eq!(
            untouched.custom_data().unwrap().unwrap(),
            [1, 2, 3, 4, 5, 6, 7, 8]
        );
        //Address/size remain VM data. A hostile script cannot turn them into a
        //native host read or cross an independently allocated block boundary.
        controller
            .machine
            .write_bytes(EXTERNAL + CUSTOM_SIZE, &9u32.to_le_bytes())
            .unwrap();
        assert!(controller.custom_data().is_err());
    }
    #[test]
    fn terminal_evaluation_runs_original_entry_and_restores_only_native_control_words() {
        let mut code = Vec::new();
        binary(&mut code, 0x15, (3, EXTERNAL), (0x13, EXTERNAL + FRAME));
        code.push(0x2d);
        binary(&mut code, 0x15, (3, EXTERNAL + 4), (0x13, EXTERNAL + SKIP));
        code.push(0x2d);
        binary(&mut code, 0x15, (3, EXTERNAL + PARAMETERS), (3, 77));
        code.push(0x2d);
        binary(&mut code, 0x2b, (3, 38), (3, 0));
        let mut controller = Controller::new(program(code), PoseWords([0; 36]), &[3], 200).unwrap();
        assert!(matches!(
            controller.evaluate_terminal(1).unwrap(),
            Tick::BudgetExhausted
        ));
        assert!(controller.tick(u32::MAX, 100).is_err());
        assert!(controller.set_pose(PoseWords([4; 36])).is_err());
        assert!(controller.set_delay(0).is_err());
        let Tick::Ready {
            pose,
            active,
            clock,
        } = controller.evaluate_terminal(100).unwrap()
        else {
            panic!("original terminal entry did not complete");
        };
        assert!(active);
        assert_eq!(clock, u32::MAX);
        assert_eq!([pose.0[0], pose.0[1]], [u32::MAX, 1]);
        assert_eq!(controller.machine.read_word(EXTERNAL + FRAME).unwrap(), 0);
        assert_eq!(controller.machine.read_word(EXTERNAL + SKIP).unwrap(), 0);
        assert_eq!(controller.parameters(1).unwrap(), [77]);
        assert_eq!(controller.tick(1, 100).unwrap(), Tick::Delayed);
        controller.set_delay(0).unwrap();
        assert_eq!(controller.delay(), 0);
        let Tick::Ready { pose, active, .. } = controller.tick(2, 100).unwrap() else {
            panic!()
        };
        assert!(!active);
        assert_eq!([pose.0[0], pose.0[1]], [2, 0]);
        //78A540 still evaluates an inactive program; its original active0 is
        //restored rather than being fabricated as a fresh running controller.
        let Tick::Ready { active: false, .. } = controller.evaluate_terminal(100).unwrap() else {
            panic!()
        };
        controller.set_pose(PoseWords([9; 36])).unwrap();
        assert_eq!(controller.pose().unwrap().0, [9; 36]);
    }
}
