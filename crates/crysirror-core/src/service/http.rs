// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Explicit host-authorized HTTP operations, separate from asset path reads.
use crate::{Result, state::ClientId};
use alloc::{string::String, vec::Vec};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}
/// Request admission is limited to an exact origin granted by the host owner.
/// Bodies and URL query strings must not be copied to diagnostics.
#[derive(Clone)]
pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// Maximum response body bytes (host limit also applies).
    pub response_limit: u32,
    /// Total operation deadline including redirects, in milliseconds.
    pub timeout_ms: u32,
}
#[derive(Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}
impl core::fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("body_bytes", &self.body.len())
            .finish()
    }
}
#[derive(Clone, Debug)]
pub enum HttpRequestState {
    Pending,
    Ready(HttpResponse),
    Failed(String),
}
/// Dropping a job cancels delivery; the transport checks cancellation at each
/// bounded I/O boundary. Deadline bounds a currently blocked system operation.
pub trait HttpRequestJob: Send {
    fn client(&self) -> ClientId;
    fn poll(&mut self) -> Result<HttpRequestState>;
}
