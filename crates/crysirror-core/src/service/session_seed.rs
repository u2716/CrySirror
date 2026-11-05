// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Bounded seed material for owned game random streams. Sampling and stream
//! algorithms are distinct; this API never reads the simulation/ambient clock.
use crate::{Error, Result, state::ClientId};
use alloc::{string::String, vec::Vec};
pub const MAX_SESSION_SEED_WORDS: usize = 16;
/// Explicit, portable initialization source. Fixed words support reproducible
/// sessions/tests; Os delegates bounded sampling to an authorized host worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionSeedSource {
    Os { words: u8 },
    Fixed { words: Vec<u32> },
}
impl SessionSeedSource {
    pub fn word_count(&self) -> usize {
        match self {
            Self::Os { words } => usize::from(*words),
            Self::Fixed { words } => words.len(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !(1..=MAX_SESSION_SEED_WORDS).contains(&self.word_count()) {
            return Err(Error::Invalid("session seed material requires1..=16 words"));
        }
        Ok(())
    }
}
/// Completed owned data, never an OS handle or executable/stateful RNG object.
/// A native stream retains its own exact algorithm/state after initialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionSeedMaterial {
    pub client: ClientId,
    pub words: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionSeedState {
    Pending,
    Ready(SessionSeedMaterial),
    Failed(String),
    Cancelled,
}
/// One nonblocking owner dependency. Dropping it cancels unfinished work;
/// explicit cancel only changes Pending, never replaces a completed result.
pub trait SessionSeedQuery: Send {
    fn client(&self) -> ClientId;
    fn poll(&mut self) -> Result<SessionSeedState>;
    fn cancel(&mut self);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_bounds_and_fixed_words_are_exact() {
        for words in [0, 17, 255] {
            assert!(SessionSeedSource::Os { words }.validate().is_err());
        }
        for words in [1, 2, 16] {
            assert!(SessionSeedSource::Os { words }.validate().is_ok());
        }
        assert!(
            SessionSeedSource::Fixed { words: Vec::new() }
                .validate()
                .is_err()
        );
        assert!(
            SessionSeedSource::Fixed {
                words: alloc::vec![0;17]
            }
            .validate()
            .is_err()
        );
        let source = SessionSeedSource::Fixed {
            words: alloc::vec![0, u32::MAX],
        };
        assert_eq!(source.word_count(), 2);
        assert!(source.validate().is_ok());
    }
    #[test]
    fn host_without_capability_explicitly_rejects_without_reading_clock() {
        struct Host;
        impl crate::service::FileSystem for Host {
            fn read(&mut self, _: ClientId, _: &crate::path::VfsPath) -> Result<Vec<u8>> {
                panic!("seed default must not read assets")
            }
            fn write_atomic(
                &mut self,
                _: ClientId,
                _: &crate::path::VfsPath,
                _: &[u8],
            ) -> Result<()> {
                panic!("seed default must not write assets")
            }
        }
        impl crate::service::HostServices for Host {
            fn submit(&mut self, _: ClientId, _: &[crate::state::Command]) -> Result<()> {
                panic!("seed default must not fabricate commands")
            }
            fn now_ms(&self) -> u64 {
                panic!("seed default must not use ambient/simulation time")
            }
        }
        assert!(matches!(
            crate::service::HostServices::begin_session_seed(
                &mut Host,
                ClientId(1),
                SessionSeedSource::Os { words: 2 }
            ),
            Err(Error::Unsupported("session seed material"))
        ));
    }
}
