// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Exact-origin HTTP capability transport. Policy grants belong to the host;
//! adapters only hold cancellable jobs. Diagnostics never contain URL/body.
use crysirror_core::{
    Error, Result,
    service::{HttpMethod, HttpRequest, HttpRequestJob, HttpRequestState, HttpResponse},
    state::ClientId,
};
use crysirror_tasks::{Task, TaskPool};
use std::{
    io::Read,
    time::{Duration, Instant},
};
use url::Url;
const MAX_REQUEST: usize = 4 * 1024 * 1024;
const MAX_RESPONSE: u32 = 8 * 1024 * 1024;
const MAX_TIMEOUT: u32 = 60_000;

/// Canonical exact origin, with no credentials, path, fragment or query.
pub fn canonical_origin(origin: &str) -> Result<String> {
    let url = Url::parse(origin).map_err(|_| Error::Invalid("invalid HTTP origin"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Invalid("HTTP grant must be an exact origin"));
    }
    Ok(url.origin().ascii_serialization())
}
/// Credential-free origin of a permitted HTTP URL; does not perform I/O.
pub fn request_origin(value: &str) -> Result<String> {
    let url = Url::parse(value).map_err(|_| Error::Invalid("invalid HTTP request URL"))?;
    let origin = url.origin().ascii_serialization();
    request_url(value, &origin).map_err(|_| Error::Invalid("invalid HTTP request URL"))?;
    Ok(origin)
}
fn request_url(value: &str, origin: &str) -> anyhow::Result<Url> {
    let url = Url::parse(value).map_err(|_| anyhow::anyhow!("invalid HTTP request URL"))?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
            && url.origin().ascii_serialization() == origin,
        "HTTP origin is not authorized"
    );
    Ok(url)
}
struct Job {
    client: ClientId,
    task: Option<Task<HttpResponse>>,
    state: HttpRequestState,
}
impl HttpRequestJob for Job {
    fn client(&self) -> ClientId {
        self.client
    }
    fn poll(&mut self) -> Result<HttpRequestState> {
        if let Some(task) = &mut self.task {
            match task.try_take() {
                Ok(None) => {}
                Ok(Some(response)) => {
                    self.state = HttpRequestState::Ready(response);
                    self.task = None;
                }
                Err(error) => {
                    self.state = HttpRequestState::Failed(error.to_string());
                    self.task = None;
                }
            }
        }
        Ok(self.state.clone())
    }
}
/// Admit one bounded job against a grant already checked for this client.
/// Redirect authority cannot grow: every target is rechecked before sending.
pub fn begin(
    pool: &TaskPool,
    client: ClientId,
    origin: &str,
    request: HttpRequest,
) -> Result<Box<dyn HttpRequestJob>> {
    let origin = canonical_origin(origin)?;
    request_url(&request.url, &origin)
        .map_err(|_| Error::Invalid("HTTP origin is not authorized"))?;
    if request.body.len() > MAX_REQUEST
        || !(1..=MAX_RESPONSE).contains(&request.response_limit)
        || !(1..=MAX_TIMEOUT).contains(&request.timeout_ms)
        || request.headers.len() > 64
        || request
            .headers
            .iter()
            .map(|(key, value)| key.len() + value.len())
            .sum::<usize>()
            > 8192
    {
        return Err(Error::Limit("HTTP request"));
    }
    let task = pool
        .spawn(move |cancel| {
            cancel.check()?;
            let mut url = request_url(&request.url, &origin)?;
            let mut headers = reqwest::header::HeaderMap::new();
            for (key, value) in request.headers {
                anyhow::ensure!(
                    ![
                        "host",
                        "content-length",
                        "transfer-encoding",
                        "connection",
                        "upgrade",
                        "proxy-authorization"
                    ]
                    .iter()
                    .any(|blocked| key.eq_ignore_ascii_case(blocked)),
                    "HTTP transport-owned header"
                );
                let key = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|_| anyhow::anyhow!("invalid HTTP header name"))?;
                let value = reqwest::header::HeaderValue::from_str(&value)
                    .map_err(|_| anyhow::anyhow!("invalid HTTP header value"))?;
                headers.append(key, value);
            }
            let client = reqwest::blocking::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .connect_timeout(Duration::from_secs(10))
                .build()
                .map_err(|_| anyhow::anyhow!("HTTP transport initialization failed"))?;
            let deadline = Instant::now() + Duration::from_millis(u64::from(request.timeout_ms));
            let mut method = request.method;
            let mut body = request.body;
            for hop in 0..=5 {
                cancel.check()?;
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .ok_or_else(|| anyhow::anyhow!("HTTP deadline expired"))?;
                let verb = match method {
                    HttpMethod::Get => reqwest::Method::GET,
                    HttpMethod::Post => reqwest::Method::POST,
                };
                let mut response = client
                    .request(verb, url.clone())
                    .headers(headers.clone())
                    .body(body.clone())
                    .timeout(remaining)
                    .send()
                    .map_err(|error| {
                        if error.is_timeout() {
                            anyhow::anyhow!("HTTP deadline expired")
                        } else {
                            anyhow::anyhow!("HTTP transport failed")
                        }
                    })?;
                cancel.check()?;
                let status = response.status().as_u16();
                if matches!(status, 301 | 302 | 303 | 307 | 308) {
                    if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
                        anyhow::ensure!(hop < 5, "HTTP redirect limit exceeded");
                        let location = location
                            .to_str()
                            .map_err(|_| anyhow::anyhow!("invalid HTTP redirect"))?;
                        let next = url
                            .join(location)
                            .map_err(|_| anyhow::anyhow!("invalid HTTP redirect"))?;
                        url = request_url(next.as_str(), &origin)?;
                        if status == 303
                            || matches!(status, 301 | 302) && method == HttpMethod::Post
                        {
                            method = HttpMethod::Get;
                            body.clear();
                            headers.remove(reqwest::header::CONTENT_TYPE);
                        }
                        continue;
                    }
                }
                if let Some(length) = response.content_length() {
                    anyhow::ensure!(
                        length <= u64::from(request.response_limit),
                        "HTTP response exceeds limit"
                    );
                }
                let mut bytes = Vec::new();
                let mut chunk = [0u8; 16 * 1024];
                loop {
                    cancel.check()?;
                    anyhow::ensure!(Instant::now() < deadline, "HTTP deadline expired");
                    let count = response
                        .read(&mut chunk)
                        .map_err(|_| anyhow::anyhow!("HTTP response read failed"))?;
                    if count == 0 {
                        break;
                    }
                    anyhow::ensure!(
                        bytes.len() + count <= request.response_limit as usize,
                        "HTTP response exceeds limit"
                    );
                    bytes.extend_from_slice(&chunk[..count]);
                }
                cancel.check()?;
                return Ok(HttpResponse {
                    status,
                    body: bytes,
                });
            }
            anyhow::bail!("HTTP redirect limit exceeded")
        })
        .map_err(|_| Error::Limit("HTTP task admission"))?;
    Ok(Box::new(Job {
        client,
        task: Some(task),
        state: HttpRequestState::Pending,
    }))
}
