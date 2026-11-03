// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Owned game seed sampling on bounded workers. No clock-based fallback and no
//! cryptographic/credential API; adapters choose and retain their own RNG state.
use crysirror_core::{
    service::{
        SessionSeedMaterial, SessionSeedQuery, SessionSeedSource, SessionSeedState,
        MAX_SESSION_SEED_WORDS,
    },
    state::ClientId,
    Error, Result,
};
use crysirror_tasks::{Task, TaskPool};
struct Query {
    client: ClientId,
    task: Option<Task<SessionSeedMaterial>>,
    state: SessionSeedState,
}
impl SessionSeedQuery for Query {
    fn client(&self) -> ClientId {
        self.client
    }
    fn poll(&mut self) -> Result<SessionSeedState> {
        if let Some(task) = &mut self.task {
            match task.try_take() {
                Ok(None) => {}
                Ok(Some(value)) => {
                    self.state = SessionSeedState::Ready(value);
                    self.task = None;
                }
                Err(error) => {
                    self.state = SessionSeedState::Failed(format!("{error:#}"));
                    self.task = None;
                }
            }
        }
        Ok(self.state.clone())
    }
    fn cancel(&mut self) {
        if matches!(self.state, SessionSeedState::Pending) {
            if let Some(task) = self.task.take() {
                task.cancel();
            }
            self.state = SessionSeedState::Cancelled;
        }
    }
}
pub(crate) fn begin(
    pool: &TaskPool,
    client: ClientId,
    source: SessionSeedSource,
) -> Result<Box<dyn SessionSeedQuery>> {
    begin_with(pool, client, source, |bytes| {
        getrandom::fill(bytes).map_err(|e| format!("session seed OS source: {e}"))
    })
}
fn begin_with(
    pool: &TaskPool,
    client: ClientId,
    source: SessionSeedSource,
    fill: impl FnOnce(&mut [u8]) -> std::result::Result<(), String> + Send + 'static,
) -> Result<Box<dyn SessionSeedQuery>> {
    source.validate()?;
    let permit = pool.reserve().map_err(|error| {
        if error.is::<crysirror_tasks::CapacityExceeded>() {
            Error::Limit("session seed workers")
        } else {
            Error::Io(format!("session seed admission: {error:#}"))
        }
    })?;
    let task = permit
        .spawn(move |cancel| {
            cancel.check()?;
            let words = match source {
                SessionSeedSource::Fixed { words } => words,
                SessionSeedSource::Os { words } => {
                    let count = usize::from(words);
                    let mut bytes = [0u8; MAX_SESSION_SEED_WORDS * 4];
                    fill(&mut bytes[..count * 4]).map_err(anyhow::Error::msg)?;
                    cancel.check()?;
                    bytes[..count * 4]
                        .chunks_exact(4)
                        .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
                        .collect()
                }
            };
            cancel.check()?;
            Ok(SessionSeedMaterial { client, words })
        })
        .map_err(|error| Error::Io(format!("session seed submission: {error:#}")))?;
    Ok(Box::new(Query {
        client,
        task: Some(task),
        state: SessionSeedState::Pending,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc, Arc,
        },
        time::{Duration, Instant},
    };
    fn ready(query: &mut dyn SessionSeedQuery) -> Result<SessionSeedState> {
        let start = Instant::now();
        loop {
            let state = query.poll()?;
            if state != SessionSeedState::Pending {
                return Ok(state);
            }
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "seed worker timeout"
            );
            std::thread::yield_now();
        }
    }
    #[test]
    fn explicit_words_are_owned_and_stable_without_entropy_or_clock() -> anyhow::Result<()> {
        let pool = TaskPool::new("seed-fixed", 1, 1)?;
        let calls = Arc::new(AtomicUsize::new(0));
        let record = calls.clone();
        let mut query = begin_with(
            &pool,
            ClientId(7),
            SessionSeedSource::Fixed {
                words: vec![0, u32::MAX],
            },
            move |_| {
                record.fetch_add(1, Ordering::SeqCst);
                Err("fixed source must never read OS entropy".into())
            },
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        assert_eq!(query.client(), ClientId(7));
        let value = ready(query.as_mut()).map_err(|e| anyhow::anyhow!("{e}"))?;
        assert_eq!(
            value,
            SessionSeedState::Ready(SessionSeedMaterial {
                client: ClientId(7),
                words: vec![0, u32::MAX]
            })
        );
        query.cancel();
        assert_eq!(query.poll().map_err(|e| anyhow::anyhow!("{e}"))?, value);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        Ok(())
    }
    #[test]
    fn entropy_shape_little_endian_and_failure_are_real_not_clock_fallback() -> anyhow::Result<()> {
        let pool = TaskPool::new("seed-os-shape", 1, 2)?;
        let mut query = begin_with(
            &pool,
            ClientId(3),
            SessionSeedSource::Os { words: 2 },
            |bytes| {
                assert_eq!(bytes.len(), 8);
                for (i, byte) in bytes.iter_mut().enumerate() {
                    *byte = i as u8;
                }
                Ok(())
            },
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        assert_eq!(
            ready(query.as_mut()).map_err(|e| anyhow::anyhow!("{e}"))?,
            SessionSeedState::Ready(SessionSeedMaterial {
                client: ClientId(3),
                words: vec![0x03020100, 0x07060504]
            })
        );
        let mut failed = begin_with(
            &pool,
            ClientId(4),
            SessionSeedSource::Os { words: 2 },
            |_| Err("authored OS entropy unavailable".into()),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        let state = ready(failed.as_mut()).map_err(|e| anyhow::anyhow!("{e}"))?;
        assert_eq!(
            state,
            SessionSeedState::Failed("authored OS entropy unavailable".into())
        );
        assert_eq!(failed.poll().map_err(|e| anyhow::anyhow!("{e}"))?, state);
        Ok(())
    }
    #[test]
    fn capacity_is_reserved_before_any_entropy_work() -> anyhow::Result<()> {
        let pool = TaskPool::new("seed-admission", 1, 1)?;
        let permit = pool.reserve()?;
        let calls = Arc::new(AtomicUsize::new(0));
        let record = calls.clone();
        let result = begin_with(
            &pool,
            ClientId(5),
            SessionSeedSource::Os { words: 2 },
            move |_| {
                record.fetch_add(1, Ordering::SeqCst);
                Ok(())
            },
        );
        assert!(matches!(result, Err(Error::Limit("session seed workers"))));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        drop(permit);
        Ok(())
    }
    #[test]
    fn queued_cancel_and_drop_never_publish_seed_or_run_entropy() -> anyhow::Result<()> {
        let pool = TaskPool::new("seed-cancel", 1, 3)?;
        let (started, receive) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let blocker = pool.spawn(move |_| {
            started.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(())
        })?;
        receive.recv_timeout(Duration::from_secs(5))?;
        let calls = Arc::new(AtomicUsize::new(0));
        for explicit in [false, true] {
            let record = calls.clone();
            let mut query = begin_with(
                &pool,
                ClientId(6),
                SessionSeedSource::Os { words: 2 },
                move |_| {
                    record.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            if explicit {
                query.cancel();
                assert_eq!(
                    query.poll().map_err(|e| anyhow::anyhow!("{e}"))?,
                    SessionSeedState::Cancelled
                );
            }
            drop(query);
        }
        release.send(())?;
        blocker.wait()?;
        let start = Instant::now();
        while pool.stats().outstanding != 0 {
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::yield_now();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(pool.stats().cancelled, 2);
        Ok(())
    }
}
