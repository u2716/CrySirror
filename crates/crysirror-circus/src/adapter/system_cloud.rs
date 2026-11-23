//! Native optional account operations with real, explicitly authorized HTTP.
//! A missing host origin grant is an environmental failure, never fabricated
//! authentication or transfer success. No operation logs credentials/payloads.
use super::cloud_codec::{self as codec, CloudMetadata};
use super::*;
use crysirror_core::service::{
    CivilTime, HttpMethod, HttpRequest, HttpRequestJob, HttpRequestState,
};
const ROOT: u32 = 0x38f1_0000;
const ID: u32 = ROOT + 1;
const MAIL: u32 = ROOT + 2;
const OK: u32 = ROOT + 3;
const CANCEL: u32 = ROOT + 4;
const ERROR: u32 = ROOT + 5;
const NOTICE: u32 = 50002;
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Credentials {
    #[serde(rename = "MCID")]
    id: String,
    #[serde(rename = "MCMail")]
    mail: String,
}
enum PreparedCloud {
    Loaded {
        credentials: Credentials,
        upload: Vec<u8>,
        profile: Value,
    },
    Committed {
        system: Option<Vec<u8>>,
    },
}
#[derive(Clone, Copy)]
enum Purpose {
    Authenticate,
    Action,
}
enum Phase {
    Load,
    Ready,
    Clock(Purpose),
    Http(Purpose),
    Credentials,
    Confirm,
    Commit,
    Notice { retry: bool, status: bool },
    Complete { status: bool },
}
pub(super) struct AccountJob {
    operation: AccountOperation,
    system: Vec<u8>,
    faces: Vec<u8>,
    meta: Option<CloudMetadata>,
    credentials: Credentials,
    profile: Value,
    upload: Vec<u8>,
    phase: Phase,
    worker: Option<Receiver<std::result::Result<PreparedCloud, String>>>,
    clock: Option<Box<dyn CivilTimeQuery>>,
    http: Option<Box<dyn HttpRequestJob>>,
    response: Vec<u8>,
    authenticated: bool,
    authorized: bool,
    edited: bool,
    result_system: Option<Vec<u8>>,
    opened: bool,
}
impl AccountJob {
    fn new(operation: AccountOperation, system: Vec<u8>, faces: Vec<u8>) -> Self {
        Self {
            operation,
            system,
            faces,
            meta: None,
            credentials: Credentials::default(),
            profile: Value::Null,
            upload: Vec::new(),
            phase: Phase::Load,
            worker: None,
            clock: None,
            http: None,
            response: Vec::new(),
            authenticated: false,
            authorized: false,
            edited: false,
            result_system: None,
            opened: false,
        }
    }
    pub(super) fn ui_waiting(&self) -> bool {
        matches!(
            self.phase,
            Phase::Credentials | Phase::Confirm | Phase::Notice { .. }
        )
    }
    fn spawn(
        &mut self,
        work: impl FnOnce() -> Result<PreparedCloud> + Send + 'static,
    ) -> Result<()> {
        ensure!(self.worker.is_none(), "Circus account worker ownership");
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("circus-account-data".into())
            .spawn(move || {
                //Errors must describe the operation, not credential or body bytes.
                let result =
                    work().map_err(|_| "Circus account storage/codec operation failed".into());
                let _ = sender.send(result);
            })?;
        self.worker = Some(receiver);
        Ok(())
    }
    fn poll_worker(&mut self) -> Result<Option<PreparedCloud>> {
        let Some(worker) = &self.worker else {
            return Ok(None);
        };
        match worker.try_recv() {
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => bail!("Circus account worker disconnected"),
            Ok(result) => {
                self.worker = None;
                Ok(Some(result.map_err(anyhow::Error::msg)?))
            }
        }
    }
}
fn read_json(storage: &dyn ClientStorage, name: &str) -> Result<Option<Value>> {
    let path = VfsPath::new(Mount::Save, name)?;
    if storage.metadata(Mount::Save, name)?.is_none() {
        return Ok(None);
    }
    let reader = storage.open_read(&path)?;
    let bytes = crysirror_resource::read_all(reader.as_ref(), 4096)?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}
fn origin(endpoint: &str) -> Result<String> {
    let parts = endpoint.split('/').take(3).collect::<Vec<_>>();
    ensure!(
        parts.len() == 3 && parts[1].is_empty(),
        "Circus cloud endpoint profile"
    );
    Ok(parts.join("/"))
}
fn request(
    meta: &CloudMetadata,
    job: &AccountJob,
    purpose: Purpose,
    time: &CivilTime,
) -> Result<HttpRequest> {
    let token_fields = |game: &str| {
        let (cic, snm) = codec::token(time, game);
        format!(
            "{}={cic}&{}=10&{}={snm}&{}={game}",
            meta.token_keys[0], meta.token_keys[1], meta.token_keys[2], meta.token_keys[3]
        )
    };
    let mut headers = vec![("User-Agent".into(), meta.game.clone())];
    let mut body = Vec::new();
    let (url, method) = match purpose {
        Purpose::Authenticate => {
            //455770's pinned format is "%scic=...", without inserting a
            //question mark. Preserve the actual source URL, even if a server
            //rejects it; do not repair the original protocol by intention.
            let mut url = format!(
                "{}{}&{}={}&{}=",
                meta.endpoint,
                token_fields(&meta.game),
                meta.action,
                meta.verify,
                meta.identity
            )
            .into_bytes();
            url.extend(codec::cp932(&job.credentials.id)?);
            url.extend(format!("&{}=", meta.mail).bytes());
            url.extend(codec::cp932(&job.credentials.mail)?);
            (codec::ansi_url(&url), HttpMethod::Get)
        }
        Purpose::Action => match job.operation {
            AccountOperation::Upload => {
                body.extend(
                    format!(
                        "{}&{}=sv&{}=",
                        token_fields(&meta.storage_game),
                        meta.action,
                        meta.identity
                    )
                    .bytes(),
                );
                body.extend(codec::cp932(&job.credentials.id)?);
                body.extend(format!("&{}={}:", meta.document, meta.game).bytes());
                body.extend(codec::cp932(&job.credentials.id)?);
                body.extend(format!("&{}=", meta.data).bytes());
                body.extend_from_slice(&job.upload);
                (meta.endpoint.clone(), HttpMethod::Post)
            }
            AccountOperation::Download => {
                let mut url = format!(
                    "{}{}&{}=ld&{}=",
                    meta.endpoint,
                    token_fields(&meta.storage_game),
                    meta.action,
                    meta.identity
                )
                .into_bytes();
                url.extend(codec::cp932(&job.credentials.id)?);
                url.extend(format!("&{}={}:", meta.document, meta.game).bytes());
                url.extend(codec::cp932(&job.credentials.id)?);
                (codec::ansi_url(&url), HttpMethod::Get)
            }
            AccountOperation::ClearReport => {
                let name = codec::cp932(
                    job.profile["CLEARNAME"]
                        .as_str()
                        .context("Circus completion name")?,
                )?;
                let mail = codec::cp932(
                    job.profile["CLEARMAIL"]
                        .as_str()
                        .context("Circus completion email")?,
                )?;
                let datetime = format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                    time.year, time.month, time.day, time.hour, time.minute, time.second
                );
                let parts = meta.report_template.split("%s").collect::<Vec<_>>();
                ensure!(parts.len() == 4, "Circus report hash template profile");
                let mut material = parts[0].as_bytes().to_vec();
                material.extend(&name);
                material.extend(parts[1].bytes());
                material.extend(&mail);
                material.extend(parts[2].bytes());
                material.extend(datetime.bytes());
                material.extend(parts[3].bytes());
                body.extend(format!("{}=", meta.report_keys[0]).bytes());
                body.extend(name);
                body.extend(format!("&{}=", meta.report_keys[1]).bytes());
                body.extend(mail);
                body.extend(
                    format!(
                        "&{}={datetime}&{}={}",
                        meta.report_keys[2],
                        meta.report_keys[3],
                        codec::digest(&material)
                    )
                    .bytes(),
                );
                (meta.endpoint.clone(), HttpMethod::Post)
            }
            AccountOperation::Credentials => {
                bail!("Circus credential request has no transfer action")
            }
        },
    };
    if method == HttpMethod::Post {
        headers.push((
            "Content-Type".into(),
            "application/x-www-form-urlencoded".into(),
        ));
    }
    Ok(HttpRequest {
        url,
        method,
        headers,
        body,
        response_limit: 8 * 1024 * 1024,
        timeout_ms: 30_000,
    })
}
impl Bridge<'_> {
    pub(super) fn native_account(
        &mut self,
        operation: AccountOperation,
        system: Vec<u8>,
        faces: Vec<u8>,
    ) -> Result<Option<SystemReply>> {
        let mut job = self
            .services
            .system
            .account
            .take()
            .unwrap_or_else(|| AccountJob::new(operation, system.clone(), faces.clone()));
        ensure!(
            job.operation == operation && job.system == system && job.faces == faces,
            "Circus account instruction ownership"
        );
        let result = self.tick_account(&mut job);
        if !matches!(result, Ok(Some(_))) {
            self.services.system.account = Some(job);
        }
        result
    }
    fn account_notice(
        &mut self,
        job: &mut AccountJob,
        message: &str,
        retry: bool,
        status: bool,
    ) -> Result<()> {
        self.submit(vec![],vec![json!({"op":"open_dialog","dialog":{"id":NOTICE,"title":"D.C.4 Plus Harmony","message":message,"buttons":[{"id":1,"label":"OK"}],"default":1,"font":self.font}})])?;
        job.phase = Phase::Notice { retry, status };
        Ok(())
    }
    fn tick_account(&mut self, job: &mut AccountJob) -> Result<Option<SystemReply>> {
        for _ in 0..16 {
            match job.phase {
                Phase::Load => {
                    if job.meta.is_none() {
                        let Some(meta) = self.system_metadata()? else {
                            return Ok(None);
                        };
                        job.meta = Some(meta.cloud);
                    }
                    if job.worker.is_none() {
                        let storage = self
                            .services
                            .storage
                            .as_ref()
                            .context("Circus account storage capability")?
                            .clone();
                        let op = job.operation;
                        let system = job.system.clone();
                        job.spawn(move || {
                            let credentials = read_json(storage.as_ref(), "circus-account.json")?
                                .map(serde_json::from_value)
                                .transpose()?
                                .unwrap_or_default();
                            let profile = read_json(storage.as_ref(), "circus-profile.json")?
                                .unwrap_or(json!({"schema":1,"CLEARNAME":"","CLEARMAIL":""}));
                            let mut slots = Vec::new();
                            if op == AccountOperation::Upload {
                                for slot in 0..5 {
                                    let path = VfsPath::new(
                                        Mount::Save,
                                        &crate::native_save::SlotKind::Pre
                                            .filename(slot as i32 + 110),
                                    )?;
                                    if let Some(metadata) =
                                        storage.metadata(Mount::Save, path.relative())?
                                    {
                                        if !metadata.is_directory {
                                            let Ok(reader) = storage.open_read(&path) else {
                                                continue;
                                            };
                                            if reader.len().unwrap_or(0)
                                                < crate::native_save::BODY_BYTES as u64
                                            {
                                                continue;
                                            }
                                            let mut body = vec![0; crate::native_save::BODY_BYTES];
                                            if crysirror_resource::read_exact_at(
                                                reader.as_ref(),
                                                0,
                                                &mut body,
                                            )
                                            .is_err()
                                            {
                                                continue;
                                            }
                                            slots.push((slot, body));
                                        }
                                    }
                                }
                            }
                            let upload = if op == AccountOperation::Upload {
                                codec::upload(&system, &slots)?
                            } else {
                                Vec::new()
                            };
                            Ok(PreparedCloud::Loaded {
                                credentials,
                                upload,
                                profile,
                            })
                        })?;
                        return Ok(None);
                    }
                    let Some(PreparedCloud::Loaded {
                        credentials,
                        upload,
                        profile,
                    }) = job.poll_worker()?
                    else {
                        return Ok(None);
                    };
                    job.credentials = credentials;
                    job.upload = upload;
                    job.profile = profile;
                    job.phase = Phase::Ready;
                }
                Phase::Ready => {
                    if job.operation == AccountOperation::ClearReport
                        && ["CLEARNAME", "CLEARMAIL"]
                            .iter()
                            .any(|key| job.profile[*key].as_str().is_none_or(str::is_empty))
                    {
                        job.phase = Phase::Complete { status: false };
                        continue;
                    }
                    if job.operation == AccountOperation::ClearReport {
                        self.submit(vec![],vec![json!({"op":"open_dialog","dialog":{"id":NOTICE,"title":"D.C.4 Plus Harmony","message":"Send the completion report using the saved name and email address?","buttons":[{"id":6,"label":"Yes"},{"id":7,"label":"No"}],"default":6,"font":self.font}})])?;
                        job.phase = Phase::Confirm;
                        return Ok(None);
                    }
                    if job.operation != AccountOperation::Credentials
                        && (job.credentials.id.is_empty() || job.credentials.mail.is_empty())
                    {
                        job.phase = Phase::Complete { status: false };
                        continue;
                    }
                    let meta = job.meta.as_ref().unwrap();
                    job.authorized = match self
                        .host
                        .http_origin_allowed(self.client, &origin(&meta.endpoint)?)
                    {
                        Ok(allowed) => allowed,
                        Err(crysirror_core::Error::Unsupported(_)) => false,
                        Err(error) => return Err(error.into()),
                    };
                    if !job.authorized {
                        self.account_notice(
                            job,
                            "The account service has no HTTP origin authorization in this host.",
                            false,
                            false,
                        )?;
                        return Ok(None);
                    }
                    if job.credentials.id.is_empty() || job.credentials.mail.is_empty() {
                        if job.operation == AccountOperation::Credentials {
                            job.phase = Phase::Credentials;
                        } else {
                            job.phase = Phase::Complete { status: false };
                        }
                    } else {
                        job.phase = Phase::Clock(Purpose::Authenticate);
                    }
                }
                Phase::Clock(purpose) => {
                    if job.operation == AccountOperation::ClearReport && !job.authorized {
                        job.authorized = match self.host.http_origin_allowed(
                            self.client,
                            &origin(&job.meta.as_ref().unwrap().endpoint)?,
                        ) {
                            Ok(allowed) => allowed,
                            Err(crysirror_core::Error::Unsupported(_)) => false,
                            Err(error) => return Err(error.into()),
                        };
                        if !job.authorized {
                            self.account_notice(job,"The account service has no HTTP origin authorization in this host.",false,false)?;
                            return Ok(None);
                        }
                    }
                    if job.clock.is_none() {
                        job.clock = Some(self.host.begin_civil_time(self.client)?);
                        return Ok(None);
                    }
                    let clock = job.clock.as_mut().unwrap();
                    ensure!(
                        clock.client() == self.client,
                        "Circus account clock ownership"
                    );
                    match clock.poll()? {
                        CivilTimeState::Pending => return Ok(None),
                        CivilTimeState::Failed(_) => {
                            self.account_notice(
                                job,
                                "The host civil clock is unavailable.",
                                false,
                                false,
                            )?;
                            return Ok(None);
                        }
                        CivilTimeState::Ready(time) => {
                            ensure!(time.client == self.client, "Circus foreign account clock");
                            let request = request(job.meta.as_ref().unwrap(), job, purpose, &time)?;
                            job.clock = None;
                            job.http = match self.host.begin_http(self.client, request) {
                                Ok(job) => Some(job),
                                Err(crysirror_core::Error::NotFound) => {
                                    bail!("Circus account client is not registered")
                                }
                                Err(_) => {
                                    self.account_notice(
                                        job,
                                        "The host could not admit the account request.",
                                        matches!(purpose, Purpose::Authenticate)
                                            && job.operation == AccountOperation::Credentials,
                                        false,
                                    )?;
                                    return Ok(None);
                                }
                            };
                            job.phase = Phase::Http(purpose);
                            return Ok(None);
                        }
                    }
                }
                Phase::Http(purpose) => {
                    let http = job.http.as_mut().context("Circus account HTTP owner")?;
                    ensure!(http.client() == self.client, "Circus foreign HTTP job");
                    match http.poll()? {
                        HttpRequestState::Pending => return Ok(None),
                        HttpRequestState::Failed(_) => {
                            job.http = None;
                            self.account_notice(
                                job,
                                "The account request failed.",
                                matches!(purpose, Purpose::Authenticate)
                                    && job.operation == AccountOperation::Credentials,
                                false,
                            )?;
                            return Ok(None);
                        }
                        HttpRequestState::Ready(response) => {
                            job.http = None;
                            if response.status != 200 {
                                self.account_notice(
                                    job,
                                    "The account server returned an unsuccessful HTTP status.",
                                    matches!(purpose, Purpose::Authenticate)
                                        && job.operation == AccountOperation::Credentials,
                                    false,
                                )?;
                                return Ok(None);
                            }
                            match purpose {
                                Purpose::Authenticate => {
                                    job.authenticated =
                                        codec::response_ansi(&response.body).starts_with(b"true");
                                    if !job.authenticated {
                                        self.account_notice(
                                            job,
                                            "The account could not be verified.",
                                            job.operation == AccountOperation::Credentials,
                                            false,
                                        )?;
                                        return Ok(None);
                                    }
                                    if job.operation == AccountOperation::Credentials {
                                        job.phase = if job.edited {
                                            Phase::Commit
                                        } else {
                                            Phase::Complete { status: true }
                                        };
                                    } else {
                                        job.phase = Phase::Clock(Purpose::Action);
                                    }
                                }
                                Purpose::Action => {
                                    job.response = response.body;
                                    job.phase = Phase::Commit;
                                }
                            }
                        }
                    }
                }
                Phase::Credentials => {
                    if !job.opened {
                        self.submit(vec![],vec![json!({"op":"put","widget":{"id":ROOT,"rect":[340,210,600,300],"modal":true,"color":[24,24,30,255]}}),
                            json!({"op":"put","widget":{"id":ID,"parent":ROOT,"rect":[30,45,540,45],"kind":"text_field","text":job.credentials.id,"font":self.font,"text_size":24,"max_length":259,"color":[255,255,255,255],"text_color":[0,0,0,255]}}),
                            json!({"op":"put","widget":{"id":MAIL,"parent":ROOT,"rect":[30,110,540,45],"kind":"text_field","text":job.credentials.mail,"font":self.font,"text_size":24,"max_length":259,"color":[255,255,255,255],"text_color":[0,0,0,255]}}),
                            json!({"op":"put","widget":{"id":ERROR,"parent":ROOT,"rect":[30,170,540,45],"text":"Account ID and email","font":self.font,"text_size":20}}),
                            json!({"op":"put","widget":{"id":OK,"parent":ROOT,"rect":[300,240,120,40],"kind":"button","text":"OK","font":self.font}}),
                            json!({"op":"put","widget":{"id":CANCEL,"parent":ROOT,"rect":[440,240,120,40],"kind":"button","text":"Cancel","font":self.font}}),json!({"op":"focus","id":ID})])?;
                        job.opened = true;
                        return Ok(None);
                    }
                    let events: Vec<Value> = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::UiSubtreeEvents(ROOT))?,
                    )?;
                    let mut action = None;
                    for event in events {
                        let id = event["widget"]["id"].as_u64();
                        if event["kind"] == "cancel" {
                            action = Some(CANCEL)
                        } else if event["kind"] == "submit" {
                            action = Some(OK)
                        } else if event["kind"] == "activate" {
                            action = id.and_then(|id| u32::try_from(id).ok());
                        }
                    }
                    match action {
                        Some(CANCEL) => {
                            self.submit(vec![], vec![json!({"op":"remove","id":ROOT})])?;
                            job.opened = false;
                            job.phase = Phase::Complete { status: false };
                        }
                        Some(OK) => {
                            let mut values = Vec::new();
                            for field in [ID, MAIL] {
                                let widget: Value = serde_json::from_slice(
                                    &self
                                        .host
                                        .query_engine(self.client, EngineQuery::UiWidget(field))?,
                                )?;
                                let text = widget["text"]
                                    .as_str()
                                    .context("Circus account text control")?;
                                let mut encoded = codec::cp932(text)?;
                                encoded.truncate(259);
                                while encoding_rs::SHIFT_JIS
                                    .decode_without_bom_handling(&encoded)
                                    .1
                                {
                                    encoded.pop();
                                }
                                values.push(
                                    encoding_rs::SHIFT_JIS
                                        .decode_without_bom_handling(&encoded)
                                        .0
                                        .into_owned(),
                                );
                            }
                            if values.iter().any(String::is_empty) {
                                self.submit(vec![],vec![json!({"op":"set_text","id":ERROR,"text":"Both fields are required."})])?;
                                return Ok(None);
                            }
                            job.credentials = Credentials {
                                id: values.remove(0),
                                mail: values.remove(0),
                            };
                            job.edited = true;
                            self.submit(vec![], vec![json!({"op":"remove","id":ROOT})])?;
                            job.opened = false;
                            job.phase = Phase::Clock(Purpose::Authenticate);
                        }
                        _ => return Ok(None),
                    }
                }
                Phase::Confirm | Phase::Notice { .. } => {
                    let events: Vec<Value> = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::DialogEvents)?,
                    )?;
                    let Some(value) = events
                        .iter()
                        .find(|event| event["widget"]["id"].as_u64() == Some(u64::from(NOTICE)))
                        .and_then(|event| event["value"].as_f64())
                    else {
                        return Ok(None);
                    };
                    job.phase = match job.phase {
                        Phase::Confirm => {
                            if value == 6. {
                                Phase::Clock(Purpose::Action)
                            } else {
                                Phase::Complete { status: false }
                            }
                        }
                        Phase::Notice { retry: true, .. } => Phase::Credentials,
                        Phase::Notice { status, .. } => Phase::Complete { status },
                        _ => unreachable!(),
                    };
                }
                Phase::Commit => {
                    if job.worker.is_none() {
                        let storage = self
                            .services
                            .storage
                            .as_ref()
                            .context("Circus account storage capability")?
                            .clone();
                        let operation = job.operation;
                        let credentials = job.credentials.clone();
                        let mut profile = job.profile.clone();
                        let response = std::mem::take(&mut job.response);
                        let system = job.system.clone();
                        let faces = job.faces.clone();
                        let meta = job.meta.clone().unwrap();
                        job.spawn(move || {
                            let mut replacement = None;
                            match operation {
                                AccountOperation::Credentials => storage.write_atomic(
                                    &VfsPath::new(Mount::Save, "circus-account.json")?,
                                    &serde_json::to_vec(&credentials)?,
                                )?,
                                AccountOperation::ClearReport => {
                                    profile["CLEARNAME"] = json!("true");
                                    let validated: super::super::profile::ProfileData =
                                        serde_json::from_value(profile)?;
                                    validated.validate()?;
                                    storage.write_atomic(
                                        &VfsPath::new(Mount::Save, "circus-profile.json")?,
                                        &serde_json::to_vec(&validated)?,
                                    )?;
                                }
                                AccountOperation::Download => {
                                    let (system, slots) =
                                        codec::download(&response, &system, &faces, &meta)?;
                                    for (slot, body) in slots {
                                        for kind in [
                                            crate::native_save::SlotKind::Normal,
                                            crate::native_save::SlotKind::Pre,
                                        ] {
                                            storage.write_atomic(
                                                &VfsPath::new(
                                                    Mount::Save,
                                                    &kind.filename(slot as i32 + 110),
                                                )?,
                                                &body,
                                            )?;
                                        }
                                    }
                                    replacement = Some(system);
                                }
                                AccountOperation::Upload => {}
                            }
                            Ok(PreparedCloud::Committed {
                                system: replacement,
                            })
                        })?;
                        return Ok(None);
                    }
                    let Some(PreparedCloud::Committed { system }) = job.poll_worker()? else {
                        return Ok(None);
                    };
                    job.result_system = system;
                    job.phase = Phase::Complete { status: true };
                }
                Phase::Complete { status } => {
                    return Ok(Some(SystemReply::Account {
                        authenticated: job.authenticated,
                        available: job.authorized,
                        status,
                        system: job.result_system.take(),
                    }));
                }
            }
        }
        bail!("Circus account phase progression exceeded bound")
    }
}
