// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! httprequest.dll: `HttpRequest` over the engine's authorized HTTP service.
//!
//! Contract: krkr2 trunk `src/plugins/win32/httprequest/` (`Main.cpp`,
//! `manual.tjs`). `HttpRequest(window, cert = true, agentName)`;
//! `open(method, url[, user, pass])`, `setRequestHeader`, `send(data[,
//! saveStorage])`, `sendStorage(storage[, saveStorage])`, the `*Sync`
//! variants returning the status, `abort`, `getAllResponseHeaders`,
//! `getResponseHeader`, `getResponseText([encoding])`, read-only
//! `readyState` / `response` / `responseData` / `status` / `statusText` /
//! `contentType` / `contentTypeEncoding` / `contentLength`, the
//! `UNINITIALIZED..LOADED` constants and the `onReadyStateChange(state)` /
//! `onProgress(upload, percent)` events, delivered between engine steps.
//!
//! Transport is `HostServices::begin_http`: only origins the player owner
//! authorized for this client are reachable, as a shared worker job with a
//! deadline and response limit. Any other origin, a transport error or a
//! timeout completes like the reference's failed connection: `status` 0,
//! `statusText` the reason, `readyState` LOADED. A request finishes with its
//! response, so another `send` needs a new `open`. The `*Sync` calls wait at
//! a resumable script boundary instead of blocking the engine thread.
use crate::natives::{Rt, this_of};
use crysirror_core::{
    service::{HostServices, HttpMethod, HttpRequest, HttpRequestJob, HttpRequestState, StorageWrite, StorageWriteState},
    state::ClientId,
};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{NativeArgCount, ObjectHandle, Variant},
};
use std::collections::{BTreeMap, VecDeque};

const UNINITIALIZED: i64 = 0;
const OPEN: i64 = 1;
const SENT: i64 = 2;
const RECEIVING: i64 = 3;
const LOADED: i64 = 4;
const RESPONSE_LIMIT: u32 = 8 << 20;
const TIMEOUT_MS: u32 = 60_000;

#[derive(Default)]
pub(crate) struct Http {
    requests: BTreeMap<usize, Request>,
    /// `(object, method, args)` script events awaiting delivery.
    pub(crate) events: VecDeque<(ObjectHandle, &'static str, Vec<Variant>)>,
}

impl Http {
    pub(crate) fn roots(&self, out: &mut Vec<ObjectHandle>) {
        out.extend(self.requests.iter().filter(|(_, r)| r.busy()).map(|(h, _)| ObjectHandle(*h)));
        out.extend(self.events.iter().map(|(h, ..)| *h));
    }
    pub(crate) fn invalidate(&mut self, object: usize) {
        self.requests.remove(&object);
        self.events.retain(|(h, ..)| h.0 != object);
    }
}

#[derive(Default)]
struct Request {
    ready_state: i64,
    method: Option<HttpMethod>,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    save_storage: Option<String>,
    job: Option<Box<dyn HttpRequestJob>>,
    write: Option<Box<dyn StorageWrite>>,
    sync_waiting: bool,
    status: i64,
    status_text: String,
    response_headers: Vec<(String, String)>,
    output: Vec<u8>,
    content_type: String,
    encoding: String,
}

impl Request {
    fn busy(&self) -> bool {
        self.body.is_some() || self.job.is_some() || self.write.is_some()
    }
}

fn instance(rt: &mut Rt, this: Option<ObjectHandle>, what: &str) -> Result<ObjectHandle> {
    let h = this_of(rt, this, what)?;
    if !rt.host().http.requests.contains_key(&h.0) {
        return Err(TjsError::runtime(format!("{what}: native class crash (no HttpRequest instance)")));
    }
    Ok(h)
}

fn request(rt: &mut Rt, h: ObjectHandle) -> &mut Request {
    rt.host_mut().http.requests.get_mut(&h.0).expect("checked HttpRequest instance")
}

fn set_state(rt: &mut Rt, h: ObjectHandle, state: i64) {
    request(rt, h).ready_state = state;
    rt.host_mut().http.events.push_back((h, "onReadyStateChange", vec![Variant::Integer(state)]));
}

pub(crate) fn install(rt: &mut Rt) {
    if rt.global_member("HttpRequest").object_handle().is_some() {
        return;
    }
    let class = rt.alloc_native_constructor(|rt: &mut Rt, this: Option<ObjectHandle>, args: Vec<Variant>| {
        let window = args.first().ok_or_else(TjsError::bad_param_count)?;
        let window = window.object_handle().filter(|w| rt.object_class_infos(*w).iter().any(|c| c == "Window"));
        if window.is_none() {
            return Err(TjsError::runtime("InvalidObject"));
        }
        let h = this
            .map(|h| rt.bound_this(h).unwrap_or(h))
            .filter(|h| *h != rt.global_handle())
            .unwrap_or_else(|| rt.alloc_ordinary_object());
        rt.add_object_class_info(h, "HttpRequest");
        members(rt, h);
        rt.host_mut().http.requests.insert(h.0, Request::default());
        Ok(Variant::Object(h))
    });
    rt.add_object_class_info(class, "HttpRequest");
    members(rt, class);
    for (name, value) in [("UNINITIALIZED", UNINITIALIZED), ("OPEN", OPEN), ("SENT", SENT), ("RECEIVING", RECEIVING), ("LOADED", LOADED)] {
        rt.set_object_member(class, name, Variant::Integer(value));
    }
    rt.set_global_member("HttpRequest", Variant::Object(class));
}

fn members(rt: &mut Rt, h: ObjectHandle) {
    rt.register_object_native_with_arg_count(h, "open", NativeArgCount::AtLeast(2), |rt: &mut Rt, this: Option<ObjectHandle>, a: Vec<Variant>| {
        let h = instance(rt, this, "HttpRequest.open")?;
        if request(rt, h).busy() {
            return Err(TjsError::runtime("already running"));
        }
        let method = match a[0].to_tjs_string()?.to_ascii_uppercase().as_str() {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head,
            other => return Err(TjsError::runtime(format!("unsupported HTTP method {other}"))),
        };
        let url = a[1].to_tjs_string()?;
        let scheme = url.split_once("://").map(|(s, _)| s.to_ascii_lowercase());
        if !matches!(scheme.as_deref(), Some("http" | "https")) {
            return Err(TjsError::runtime("failed to crack url"));
        }
        let mut headers = Vec::new();
        if let Some(user) = a.get(2).filter(|v| !matches!(v, Variant::Void)) {
            use base64::Engine as _;
            let pass = a.get(3).map(Variant::to_tjs_string).transpose()?.unwrap_or_default();
            let credentials = base64::engine::general_purpose::STANDARD.encode(format!("{}:{pass}", user.to_tjs_string()?));
            headers.push(("Authorization".to_owned(), format!("Basic {credentials}")));
        }
        *request(rt, h) = Request { method: Some(method), url, headers, ..Request::default() };
        set_state(rt, h, OPEN);
        Ok(Variant::Void)
    });
    rt.register_object_native_with_arg_count(h, "setRequestHeader", NativeArgCount::AtLeast(2), |rt: &mut Rt, this: Option<ObjectHandle>, a: Vec<Variant>| {
        let h = instance(rt, this, "HttpRequest.setRequestHeader")?;
        let (name, value) = (a[0].to_tjs_string()?, a[1].to_tjs_string()?);
        let r = request(rt, h);
        if r.method.is_none() {
            return Err(TjsError::runtime("not open"));
        }
        if r.headers.len() >= 64 {
            return Err(TjsError::runtime("too many request headers"));
        }
        r.headers.push((name, value));
        Ok(Variant::Void)
    });
    for (name, storage, sync) in [("send", false, false), ("sendSync", false, true), ("sendStorage", true, false), ("sendStorageSync", true, true)] {
        rt.register_object_native(h, name, move |rt: &mut Rt, this: Option<ObjectHandle>, a: Vec<Variant>| send(rt, this, a, storage, sync));
    }
    rt.register_object_native(h, "abort", |rt: &mut Rt, this: Option<ObjectHandle>, _| {
        let h = instance(rt, this, "HttpRequest.abort")?;
        let r = request(rt, h);
        r.body = None;
        r.job = None;
        r.write = None;
        r.sync_waiting = false;
        r.method = None;
        r.ready_state = UNINITIALIZED;
        Ok(Variant::Void)
    });
    rt.register_object_native(h, "getAllResponseHeaders", |rt: &mut Rt, this: Option<ObjectHandle>, _| {
        let h = instance(rt, this, "HttpRequest.getAllResponseHeaders")?;
        let headers = request(rt, h).response_headers.clone();
        let d = rt.alloc_dictionary_object();
        for (name, value) in headers {
            rt.set_object_member(d, &name, Variant::String(value));
        }
        Ok(Variant::Object(d))
    });
    rt.register_object_native_with_arg_count(h, "getResponseHeader", NativeArgCount::AtLeast(1), |rt: &mut Rt, this: Option<ObjectHandle>, a: Vec<Variant>| {
        let h = instance(rt, this, "HttpRequest.getResponseHeader")?;
        let name = a[0].to_tjs_string()?;
        Ok(request(rt, h).response_headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name)).map_or(Variant::Void, |(_, v)| Variant::String(v.clone())))
    });
    rt.register_object_native(h, "getResponseText", |rt: &mut Rt, this: Option<ObjectHandle>, a: Vec<Variant>| {
        let h = instance(rt, this, "HttpRequest.getResponseText")?;
        let encoding = match a.first() {
            Some(v) if !matches!(v, Variant::Void) => v.to_tjs_string()?,
            _ => request(rt, h).encoding.clone(),
        };
        Ok(Variant::String(decode(&request(rt, h).output, &encoding)))
    });
    rt.register_object_native(h, "finalize", |_: &mut Rt, _, _| Ok(Variant::Void));
    type Get = fn(&Request) -> Variant;
    let props: [(&str, Get); 8] = [
        ("readyState", |r| Variant::Integer(r.ready_state)),
        ("response", |r| {
            if r.content_type.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("text/")) {
                Variant::String(decode(&r.output, &r.encoding))
            } else if r.output.is_empty() {
                Variant::Void
            } else {
                Variant::Octet(r.output.clone())
            }
        }),
        ("responseData", |r| if r.output.is_empty() { Variant::Void } else { Variant::Octet(r.output.clone()) }),
        ("status", |r| Variant::Integer(r.status)),
        ("statusText", |r| Variant::String(r.status_text.clone())),
        ("contentType", |r| Variant::String(r.content_type.clone())),
        ("contentTypeEncoding", |r| Variant::String(r.encoding.clone())),
        ("contentLength", |r| Variant::Integer(r.output.len() as i64)),
    ];
    for (name, get) in props {
        rt.register_object_native_property(
            h,
            name,
            move |rt, this| {
                let Some(h) = this.map(|o| rt.bound_this(o).unwrap_or(o)) else { return Ok(Variant::Void) };
                Ok(rt.host().http.requests.get(&h.0).map_or(Variant::Void, get))
            },
            move |_, _, _| Err(TjsError::runtime(format!("HttpRequest.{name} is read-only"))),
        );
    }
}

fn send(rt: &mut Rt, this: Option<ObjectHandle>, a: Vec<Variant>, storage: bool, sync: bool) -> Result<Variant> {
    let h = instance(rt, this, "HttpRequest.send")?;
    if sync && request(rt, h).sync_waiting {
        // Resumed after ResourcePending: answer once the request finished.
        if request(rt, h).busy() {
            return Err(TjsError::resource_pending("HttpRequest.sendSync"));
        }
        request(rt, h).sync_waiting = false;
        return Ok(Variant::Integer(request(rt, h).status));
    }
    if request(rt, h).busy() {
        return Err(TjsError::runtime("already running"));
    }
    if request(rt, h).method.is_none() {
        return Err(TjsError::runtime("not open"));
    }
    if sync && !rt.host().can_suspend {
        return Err(crate::natives::unsupported("HttpRequest.sendSync in this callback", "a resumable TJS boundary"));
    }
    let save_storage = a.get(1).filter(|v| !matches!(v, Variant::Void)).map(Variant::to_tjs_string).transpose()?;
    let body = if storage {
        let name = a.first().ok_or_else(TjsError::bad_param_count)?.to_tjs_string()?;
        rt.read_storage_binary(&name, "").map_err(|_| TjsError::runtime("sendStorage open failed"))?
    } else {
        match a.first() {
            Some(Variant::String(text)) => text.as_bytes().to_vec(),
            Some(Variant::Octet(bytes)) => bytes.clone(),
            _ => Vec::new(),
        }
    };
    let r = request(rt, h);
    r.body = Some(body);
    r.save_storage = save_storage;
    r.sync_waiting = sync;
    set_state(rt, h, SENT);
    if sync {
        return Err(TjsError::resource_pending("HttpRequest.sendSync"));
    }
    Ok(Variant::Void)
}

/// Start, complete and commit requests; queue their script events.
pub(crate) fn poll(rt: &mut Rt, h: &mut dyn HostServices, client: ClientId) -> anyhow::Result<()> {
    let ids: Vec<usize> = rt.host().http.requests.iter().filter(|(_, r)| r.busy()).map(|(id, _)| *id).collect();
    for id in ids {
        let object = ObjectHandle(id);
        if let Some(body) = request(rt, object).body.take() {
            let r = request(rt, object);
            let mut headers = r.headers.clone();
            if !headers.iter().any(|(n, _)| n.eq_ignore_ascii_case("user-agent")) {
                headers.push(("User-Agent".into(), "KIRIKIRI".into()));
            }
            let upload = !body.is_empty();
            let job = h.begin_http(client, HttpRequest {
                url: r.url.clone(),
                method: r.method.unwrap_or(HttpMethod::Get),
                headers,
                body,
                response_limit: RESPONSE_LIMIT,
                timeout_ms: TIMEOUT_MS,
            });
            match job {
                Ok(job) => {
                    request(rt, object).job = Some(job);
                    if upload {
                        rt.host_mut().http.events.push_back((object, "onProgress", vec![Variant::Integer(1), Variant::Integer(100)]));
                    }
                }
                Err(e) => fail(rt, object, e.to_string()),
            }
        }
        if let Some(job) = request(rt, object).job.as_mut() {
            match job.poll() {
                Ok(HttpRequestState::Pending) => {}
                Ok(HttpRequestState::Failed(reason)) => {
                    request(rt, object).job = None;
                    fail(rt, object, reason);
                }
                Err(e) => {
                    request(rt, object).job = None;
                    fail(rt, object, e.to_string());
                }
                Ok(HttpRequestState::Ready(response)) => {
                    let r = request(rt, object);
                    r.job = None;
                    r.status = i64::from(response.status);
                    r.status_text = reason_phrase(response.status).to_owned();
                    let content_type = response.headers.iter().find(|(n, _)| n == "content-type").map(|(_, v)| v.clone()).unwrap_or_default();
                    let mut parts = content_type.split(';');
                    r.content_type = parts.next().unwrap_or("").trim().to_owned();
                    r.encoding = parts
                        .filter_map(|p| p.trim().split_once('='))
                        .find(|(k, _)| k.trim().eq_ignore_ascii_case("charset"))
                        .map(|(_, v)| v.trim().trim_matches('"').to_owned())
                        .unwrap_or_default();
                    r.response_headers = response.headers;
                    r.output = response.body;
                    rt.host_mut().http.events.push_back((object, "onProgress", vec![Variant::Integer(0), Variant::Integer(100)]));
                    set_state(rt, object, RECEIVING);
                    if let Some(name) = request(rt, object).save_storage.clone() {
                        let data: std::sync::Arc<[u8]> = request(rt, object).output.clone().into();
                        match rt.host().storage().map_err(|e| anyhow::anyhow!("{e}"))?.begin_write(&name, data) {
                            Ok(write) => request(rt, object).write = Some(write),
                            Err(e) => fail(rt, object, format!("saveStorage open failed: {e:#}")),
                        }
                    } else {
                        finish(rt, object);
                    }
                }
            }
        }
        if let Some(write) = request(rt, object).write.as_mut() {
            match write.poll() {
                Ok(StorageWriteState::Pending) => {}
                Ok(StorageWriteState::Ready) => {
                    request(rt, object).write = None;
                    finish(rt, object);
                }
                Ok(StorageWriteState::Failed(reason)) => {
                    request(rt, object).write = None;
                    fail(rt, object, reason);
                }
                Err(e) => {
                    request(rt, object).write = None;
                    fail(rt, object, e.to_string());
                }
            }
        }
    }
    Ok(())
}

fn finish(rt: &mut Rt, object: ObjectHandle) {
    request(rt, object).method = None;
    set_state(rt, object, LOADED);
}

fn fail(rt: &mut Rt, object: ObjectHandle, reason: String) {
    let r = request(rt, object);
    r.status = 0;
    r.status_text = reason;
    r.output.clear();
    r.response_headers.clear();
    finish(rt, object);
}

fn decode(bytes: &[u8], encoding: &str) -> String {
    let label = if encoding.is_empty() { "utf-8" } else { encoding };
    let encoding = encoding_rs::Encoding::for_label(label.as_bytes()).unwrap_or(encoding_rs::UTF_8);
    encoding.decode(bytes).0.into_owned()
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}
