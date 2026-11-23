// SPDX-License-Identifier: MPL-2.0
//! Presenter-owned pipeline validation. Browser frames poll instead of blocking
//! their event loop on a promise that needs that same event loop to complete.
use anyhow::Result;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

/// A render preparation dependency is still pending. Hosts retain their last
/// completed picture and retry encoding; simulation/input must not be replayed.
#[derive(Debug)]
pub struct PreparationPending;
impl std::fmt::Display for PreparationPending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GPU pipeline validation pending")
    }
}
impl std::error::Error for PreparationPending {}

#[cfg(not(target_arch = "wasm32"))]
type PendingValidation = Pin<Box<dyn Future<Output = Option<wgpu::Error>> + Send + Sync>>;
#[cfg(target_arch = "wasm32")]
type PendingValidation = Pin<Box<dyn Future<Output = Option<wgpu::Error>>>>;
pub(crate) struct Validation {
    pending: Option<PendingValidation>,
    failed: Option<String>,
}
impl Validation {
    pub fn new(future: impl Future<Output = Option<wgpu::Error>> + 'static) -> Result<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(error) = pollster::block_on(future) {
                anyhow::bail!("material pipeline: {error}");
            }
            Ok(Self {
                pending: None,
                failed: None,
            })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(Self {
                pending: Some(Box::pin(future)),
                failed: None,
            })
        }
    }
    pub fn ready(&mut self) -> Result<bool> {
        if let Some(error) = &self.failed {
            anyhow::bail!("material pipeline: {error}");
        }
        if let Some(future) = &mut self.pending {
            match future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            {
                Poll::Pending => return Ok(false),
                Poll::Ready(error) => {
                    self.pending = None;
                    if let Some(error) = error {
                        let error = error.to_string();
                        self.failed = Some(error.clone());
                        anyhow::bail!("material pipeline: {error}");
                    }
                }
            }
        }
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_validation_is_polled_and_an_error_remains_failed() {
        let (tx, rx) = futures::channel::oneshot::channel();
        let mut validation = Validation {
            pending: Some(Box::pin(async move { rx.await.unwrap() })),
            failed: None,
        };
        assert!(!validation.ready().unwrap());
        tx.send(None).unwrap();
        assert!(validation.ready().unwrap());
        assert!(validation.ready().unwrap());
        validation.failed = Some("invalid shader".into());
        assert!(validation
            .ready()
            .unwrap_err()
            .to_string()
            .contains("invalid shader"));
        assert!(validation.ready().is_err());
    }
}
