// SPDX-License-Identifier: MPL-2.0
//! Original782880 GetLocalTime dependency. The host owns clock sampling;
//! simulation time/InputObservation is never substituted for local civil time.
use anyhow::{ensure, Result};
use crysirror_core::{
    service::{CivilTime, CivilTimeQuery, CivilTimeState, HostServices},
    state::ClientId,
};
pub(crate) struct Pending {
    pub owner: u32,
    pub pc: u32,
    pub operation: crate::fes_program::PendingOperation,
    client: ClientId,
    query: Option<Box<dyn CivilTimeQuery>>,
    ready: Option<CivilTime>,
    failed: Option<String>,
}
impl Pending {
    pub fn begin(
        host: &mut dyn HostServices,
        client: ClientId,
        owner: u32,
        pc: u32,
        operation: crate::fes_program::PendingOperation,
    ) -> Result<Self> {
        let query = host.begin_civil_time(client)?;
        ensure!(
            query.client() == client,
            "CatSystem2 foreign civil clock query"
        );
        Ok(Self {
            owner,
            pc,
            operation,
            client,
            query: Some(query),
            ready: None,
            failed: None,
        })
    }
    pub fn poll(&mut self) -> Result<Option<CivilTime>> {
        if let Some(error) = &self.failed {
            anyhow::bail!("{error}");
        }
        if let Some(value) = self.ready {
            return Ok(Some(value));
        }
        let result = (|| {
            let query = self
                .query
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("CatSystem2 civil clock query missing"))?;
            ensure!(
                query.client() == self.client,
                "CatSystem2 changed civil clock query owner"
            );
            match query.poll()? {
                CivilTimeState::Pending => Ok(None),
                CivilTimeState::Failed(message) => {
                    anyhow::bail!("CatSystem2 civil clock: {message}")
                }
                CivilTimeState::Ready(value) => {
                    ensure!(
                        value.client == self.client,
                        "CatSystem2 foreign civil clock receipt"
                    );
                    ensure!(
                        value.year > 0
                            && value.year <= u16::MAX as i32
                            && value.hour < 24
                            && value.minute < 60
                            && value.second < 60
                            && value.millisecond < 1000
                            && value.weekday().is_some(),
                        "CatSystem2 invalid/out-of-profile local SYSTEMTIME receipt"
                    );
                    Ok(Some(value))
                }
            }
        })();
        match result {
            Ok(Some(value)) => {
                self.ready = Some(value);
                self.query = None;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.failed = Some(format!("{error:#}"));
                self.query = None;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct Query {
        client: ClientId,
        value: CivilTime,
        polls: Arc<AtomicUsize>,
    }
    impl CivilTimeQuery for Query {
        fn client(&self) -> ClientId {
            self.client
        }
        fn poll(&mut self) -> crysirror_core::Result<CivilTimeState> {
            Ok(if self.polls.fetch_add(1, Ordering::SeqCst) == 0 {
                CivilTimeState::Pending
            } else {
                CivilTimeState::Ready(self.value)
            })
        }
    }
    fn sampled(client: ClientId) -> CivilTime {
        CivilTime {
            client,
            unix_ms: 0,
            utc_offset_seconds: 32400,
            year: 2026,
            month: 11,
            day: 1,
            hour: 5,
            minute: 50,
            second: 7,
            millisecond: 5,
        }
    }
    fn pending(value: CivilTime, polls: Arc<AtomicUsize>) -> Pending {
        Pending {
            owner: 8,
            pc: 0x1943,
            operation: crate::fes_program::PendingOperation {
            raw:Vec::new(),
                kind: crate::fes_program::PendingKind::Native,
                section: b"START".to_vec(),
                ordinal: 1,
                line: 1,
                byte_offset: 7,
                name: b"gettime".to_vec(),
                arguments: vec![b"1".to_vec()],
            },
            client: ClientId(1),
            query: Some(Box::new(Query {
                client: ClientId(1),
                value,
                polls,
            })),
            ready: None,
            failed: None,
        }
    }
    #[test]
    fn one_sample_remains_owned_and_foreign_or_invalid_receipts_stay_failed() {
        let polls = Arc::new(AtomicUsize::new(0));
        let mut query = pending(sampled(ClientId(1)), polls.clone());
        assert_eq!(query.poll().unwrap(), None);
        assert_eq!(query.poll().unwrap(), Some(sampled(ClientId(1))));
        assert_eq!(query.poll().unwrap(), Some(sampled(ClientId(1))));
        assert_eq!(polls.load(Ordering::SeqCst), 2);
        for value in [
            sampled(ClientId(2)),
            CivilTime {
                day: 31,
                month: 2,
                ..sampled(ClientId(1))
            },
        ] {
            let polls = Arc::new(AtomicUsize::new(1));
            let mut query = pending(value, polls.clone());
            let error = query.poll().unwrap_err().to_string();
            assert_eq!(query.poll().unwrap_err().to_string(), error);
            assert_eq!(polls.load(Ordering::SeqCst), 2);
        }
    }
}
