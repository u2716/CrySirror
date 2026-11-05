// SPDX-License-Identifier: MPL-2.0
//! SDK-free console host seam. No VM, renderer, mixer or scheduler is implemented here.
//! Platform drivers supply devices; an engine-owned session supplies immutable frames.
#![no_std]
#![forbid(unsafe_code)]
use crysirror_core::service::InputFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    PsVita,
    Switch,
}
impl Platform {
    pub const fn rust_target(self) -> &'static str {
        match self {
            Self::PsVita => "armv7-sony-vita-newlibeabihf",
            Self::Switch => "aarch64-nintendo-switch-freestanding",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub renderer: bool,
    pub audio: bool,
    pub scoped_storage: bool,
    pub monotonic_clock: bool,
    pub async_io: bool,
    pub cpu_tasks: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Renderer,
    Audio,
    ScopedStorage,
    MonotonicClock,
    AsyncIo,
    CpuTasks,
}
impl Capabilities {
    /// Console boot may not acknowledge missing devices or silently become a
    /// single-threaded host. Drivers advertise only implemented capabilities.
    pub fn validate(self) -> Result<(), Missing> {
        for (ready, missing) in [
            (self.renderer, Missing::Renderer),
            (self.audio, Missing::Audio),
            (self.scoped_storage, Missing::ScopedStorage),
            (self.monotonic_clock, Missing::MonotonicClock),
            (self.async_io, Missing::AsyncIo),
            (self.cpu_tasks, Missing::CpuTasks),
        ] {
            if !ready {
                return Err(missing);
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    SimulationPending,
    PresentationPending,
    Suspended,
    Stopped,
    Failed,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Error<E> {
    Missing(Missing),
    Busy,
    ClockRegression,
    InvalidPhase,
    Driver(E),
}
/// Engine integration, not a console VM. The production implementation must
/// delegate to the shared Runtime, audio jobs and render extraction graph.
/// The console layer never owns game image/text/composition algorithms.
pub trait EngineSession {
    type Frame;
    type Error;
    fn begin_frame(&mut self, elapsed_ms: u64, input: InputFrame) -> Result<(), Self::Error>;
    fn poll_frame(&mut self) -> Result<Option<Self::Frame>, Self::Error>;
    fn suspend(&mut self, suspended: bool) -> Result<(), Self::Error>;
    /// Idempotent; also called during owner drop to release in-flight resources.
    fn shutdown(&mut self) -> Result<(), Self::Error>;
}
/// The output owns a submitted immutable engine frame until its ticket completes.
/// A ticket is host state: it must never be serialized into project/VM saves.
/// shutdown must cancel/join device tasks and release submitted frame resources.
pub trait Output {
    type Frame;
    type Error;
    type Ticket;
    fn capabilities(&self) -> Capabilities;
    fn submit(&mut self, frame: Self::Frame) -> Result<Self::Ticket, Self::Error>;
    fn poll(&mut self, ticket: &mut Self::Ticket) -> Result<bool, Self::Error>;
    fn suspend(&mut self, suspended: bool) -> Result<(), Self::Error>;
    /// Idempotent; also called during owner drop to release in-flight resources.
    fn shutdown(&mut self) -> Result<(), Self::Error>;
}
/// One owner drives this coordinator; platform workers publish completions only.
/// A pending frame is polled before new simulation/input can be consumed.
pub struct Host<S: EngineSession, O: Output<Frame = S::Frame, Error = S::Error>> {
    session: S,
    output: O,
    phase: Phase,
    ticket: Option<O::Ticket>,
    clock_ms: Option<u64>,
}
impl<S: EngineSession, O: Output<Frame = S::Frame, Error = S::Error>> Host<S, O> {
    pub fn new(session: S, output: O) -> Result<Self, Error<S::Error>> {
        output.capabilities().validate().map_err(Error::Missing)?;
        Ok(Self {
            session,
            output,
            phase: Phase::Ready,
            ticket: None,
            clock_ms: None,
        })
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn begin_frame(&mut self, now_ms: u64, input: InputFrame) -> Result<(), Error<S::Error>> {
        if self.phase != Phase::Ready {
            return Err(Error::Busy);
        }
        let elapsed = match self.clock_ms {
            Some(previous) => now_ms.checked_sub(previous).ok_or(Error::ClockRegression)?,
            None => 0,
        };
        if let Err(error) = self.session.begin_frame(elapsed, input) {
            self.phase = Phase::Failed;
            return Err(Error::Driver(error));
        }
        self.clock_ms = Some(now_ms);
        self.phase = Phase::SimulationPending;
        Ok(())
    }
    pub fn poll(&mut self) -> Result<bool, Error<S::Error>> {
        match self.poll_inner() {
            Ok(ready) => Ok(ready),
            Err(error) => {
                self.phase = Phase::Failed;
                Err(Error::Driver(error))
            }
        }
    }
    fn poll_inner(&mut self) -> Result<bool, S::Error> {
        if self.phase == Phase::SimulationPending {
            if let Some(frame) = self.session.poll_frame()? {
                self.ticket = Some(self.output.submit(frame)?);
                self.phase = Phase::PresentationPending;
            }
        }
        if self.phase == Phase::PresentationPending {
            if let Some(ticket) = &mut self.ticket {
                if self.output.poll(ticket)? {
                    self.ticket = None;
                    self.phase = Phase::Ready;
                }
            }
        }
        Ok(self.phase == Phase::Ready)
    }
    /// Suspend only at a completion boundary. The platform retries after pending
    /// work completes, keeping ticket ownership and pressed-edge delivery explicit.
    pub fn suspend(&mut self, suspended: bool) -> Result<(), Error<S::Error>> {
        if !matches!(
            (self.phase, suspended),
            (Phase::Ready, true) | (Phase::Suspended, false)
        ) {
            return Err(Error::InvalidPhase);
        }
        if let Err(error) = self
            .output
            .suspend(suspended)
            .and_then(|()| self.session.suspend(suspended))
        {
            self.phase = Phase::Failed;
            return Err(Error::Driver(error));
        }
        self.clock_ms = None;
        self.phase = if suspended {
            Phase::Suspended
        } else {
            Phase::Ready
        };
        Ok(())
    }
    /// Both components are always shut down, even if one fails. No background
    /// work or outstanding device submission may survive a successful return.
    pub fn shutdown(&mut self) -> Result<(), Error<S::Error>> {
        let engine = self.session.shutdown();
        let output = self.output.shutdown();
        self.ticket = None;
        self.clock_ms = None;
        match engine.and(output) {
            Ok(()) => {
                self.phase = Phase::Stopped;
                Ok(())
            }
            Err(e) => {
                self.phase = Phase::Failed;
                Err(Error::Driver(e))
            }
        }
    }
}

impl<S: EngineSession, O: Output<Frame = S::Frame, Error = S::Error>> Drop for Host<S, O> {
    fn drop(&mut self) {
        if self.phase != Phase::Stopped {
            let _ = self.session.shutdown();
            let _ = self.output.shutdown();
        }
    }
}
