//! Native loads requested inside a running VM cross the owner boundary only
//! after that step has returned. Admission, preparation and final installation
//! reuse the ordinary original-save coordinator; no recursive VM execution.
use super::*;

struct Request {
    ticket: OriginalRestoreTicket,
    status: OriginalRestoreStatus,
    input: Option<Arc<[u8]>>,
    admitted: Option<OriginalRestoreTicket>,
}
pub(in crate::runtime) struct Requests {
    owner: u64,
    epoch: u64,
    next: u64,
    bytes: usize,
    entries: BTreeMap<u64, Request>,
    retired: Vec<(ClientId, OriginalRestoreTicket)>,
}
impl Requests {
    pub(in crate::runtime) fn new() -> Result<Self> {
        Ok(Self {
            owner: NEXT_OWNER
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                })
                .map_err(|_| Error::Limit("restore request identities"))?,
            epoch: 0,
            next: 0,
            bytes: 0,
            entries: BTreeMap::new(),
            retired: Vec::new(),
        })
    }
    fn entry(
        &self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> crysirror_core::Result<&Request> {
        if ticket.owner != self.owner || ticket.client != client {
            return Err(Error::Invalid("foreign original restore request"));
        }
        self.entries
            .get(&ticket.serial)
            .filter(|entry| entry.ticket == ticket)
            .ok_or(Error::NotFound)
    }
    pub(in crate::runtime) fn active(&self, client: ClientId) -> bool {
        self.entries.values().any(|entry| {
            entry.ticket.client == client && entry.status == OriginalRestoreStatus::Preparing
        })
    }
    pub(in crate::runtime) fn any_active(&self) -> bool {
        self.entries
            .values()
            .any(|entry| entry.status == OriginalRestoreStatus::Preparing)
    }
    pub(in crate::runtime) fn begin(
        &mut self,
        client: ClientId,
        bytes: Arc<[u8]>,
    ) -> crysirror_core::Result<OriginalRestoreTicket> {
        if bytes.len() > MAX_INPUT {
            return Err(Error::Limit("original restore input"));
        }
        if self.active(client) {
            return Err(Error::Invalid("original restore request already pending"));
        }
        let pending = self
            .bytes
            .checked_add(bytes.len())
            .ok_or(Error::Limit("restore request bytes"))?;
        if pending > MAX_PENDING_BYTES {
            return Err(Error::Limit("restore request bytes"));
        }
        // The replaced VM cannot release its successful request. Keep bounded
        // recent terminal receipts, evicting only success/cancel, never failure
        // that the surviving original VM still needs to observe.
        if self.entries.len() >= MAX_RECEIPTS {
            let serial = self
                .entries
                .iter()
                .find(|(_, entry)| {
                    entry.admitted.is_none()
                        && matches!(
                            entry.status,
                            OriginalRestoreStatus::Committed | OriginalRestoreStatus::Cancelled
                        )
                })
                .map(|(serial, _)| *serial)
                .ok_or(Error::Limit("restore request receipts"))?;
            self.entries.remove(&serial);
        }
        let serial = self
            .next
            .checked_add(1)
            .ok_or(Error::Limit("restore request serials"))?;
        let ticket = OriginalRestoreTicket {
            owner: self.owner,
            client,
            epoch: self.epoch,
            serial,
        };
        self.entries.insert(
            serial,
            Request {
                ticket,
                status: OriginalRestoreStatus::Preparing,
                input: Some(bytes),
                admitted: None,
            },
        );
        self.next = serial;
        self.bytes = pending;
        Ok(ticket)
    }
    pub(in crate::runtime) fn status(
        &self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> crysirror_core::Result<OriginalRestoreStatus> {
        Ok(self.entry(client, ticket)?.status.clone())
    }
    pub(in crate::runtime) fn cancel(
        &mut self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> crysirror_core::Result<()> {
        if self.entry(client, ticket)?.status != OriginalRestoreStatus::Preparing {
            return Err(Error::Invalid("original restore request is terminal"));
        }
        let entry = self.entries.get_mut(&ticket.serial).unwrap();
        if let Some(input) = entry.input.take() {
            self.bytes -= input.len();
        }
        entry.status = OriginalRestoreStatus::Cancelled;
        Ok(())
    }
    pub(in crate::runtime) fn release(
        &mut self,
        client: ClientId,
        ticket: OriginalRestoreTicket,
    ) -> crysirror_core::Result<()> {
        self.entry(client, ticket)?;
        if self.status(client, ticket)? == OriginalRestoreStatus::Preparing {
            self.cancel(client, ticket)?;
        }
        let entry = self.entries.remove(&ticket.serial).unwrap();
        if let Some(admitted) = entry.admitted {
            self.retired.push((client, admitted));
        }
        Ok(())
    }
    pub(in crate::runtime) fn invalidate(&mut self) {
        self.invalidate_except(None);
    }
    pub(in crate::runtime) fn invalidate_except(&mut self, keep: Option<OriginalRestoreTicket>) {
        for entry in self.entries.values_mut() {
            if keep.is_some() && entry.admitted == keep {
                continue;
            }
            if entry.status == OriginalRestoreStatus::Preparing {
                entry.status = OriginalRestoreStatus::Cancelled;
            }
            entry.input = None;
            if let Some(ticket) = entry.admitted.take() {
                self.retired.push((entry.ticket.client, ticket));
            }
        }
        self.bytes = 0;
        // Serials are independently checked and never reused, even if the
        // terminal receipt epoch has reached its representable upper bound.
        self.epoch = self.epoch.saturating_add(1);
    }
    pub(in crate::runtime) fn inspect(&self) -> serde_json::Value {
        serde_json::json!({"pending_bytes":self.bytes,"entries":self.entries.values().map(|entry| {
            let (state,error)=match &entry.status {
                OriginalRestoreStatus::Preparing=>("preparing",None),
                OriginalRestoreStatus::Ready=>("ready",None),
                OriginalRestoreStatus::Committed=>("committed",None),
                OriginalRestoreStatus::Cancelled=>("cancelled",None),
                OriginalRestoreStatus::Failed(error)=>("failed",Some(error.to_string())),
            };
            serde_json::json!({"client":entry.ticket.client.0,"serial":entry.ticket.serial.to_string(),
                "state":state,"admitted":entry.admitted.is_some(),"error":error})
        }).collect::<Vec<_>>()})
    }
}

fn failure(error: anyhow::Error) -> Error {
    error
        .downcast_ref::<Error>()
        .cloned()
        .unwrap_or_else(|| Error::Io(format!("{error:#}").chars().take(8192).collect()))
}
impl Runtime {
    fn release_retired_restore_requests(&mut self) -> Result<()> {
        for (client, ticket) in std::mem::take(&mut self.host.native_restore_requests.retired) {
            if self
                .native_restores
                .entries
                .get(&ticket.serial)
                .is_some_and(|entry| entry.ticket == ticket)
            {
                self.release_original_save_restore(client, ticket)?;
            }
        }
        Ok(())
    }
    /// Poll only queued original-save dependencies. This never steps a VM or
    /// advances clocks/input. A ready request commits at an idle audio boundary;
    /// failed loads leave the original VM and its observable failure receipt.
    pub fn poll_original_save_restore_requests(&mut self) -> Result<bool> {
        if self.audio_frame.is_some() || self.host.channel_audio.mixer.is_rendering() {
            return Ok(self.host.native_restore_requests.any_active());
        }
        self.release_retired_restore_requests()?;
        let tickets = self
            .host
            .native_restore_requests
            .entries
            .values()
            .filter(|entry| {
                entry.status == OriginalRestoreStatus::Preparing || entry.admitted.is_some()
            })
            .map(|entry| entry.ticket)
            .collect::<Vec<_>>();
        for request in tickets {
            let client = request.client;
            let input = {
                let entry = self
                    .host
                    .native_restore_requests
                    .entries
                    .get_mut(&request.serial)
                    .unwrap();
                let input = entry.input.take();
                if let Some(input) = &input {
                    self.host.native_restore_requests.bytes -= input.len();
                }
                input
            };
            if let Some(input) = input {
                match self.begin_original_save_restore(client, input) {
                    Ok(ticket) => {
                        self.host
                            .native_restore_requests
                            .entries
                            .get_mut(&request.serial)
                            .unwrap()
                            .admitted = Some(ticket)
                    }
                    Err(error) => {
                        self.host
                            .native_restore_requests
                            .entries
                            .get_mut(&request.serial)
                            .unwrap()
                            .status = OriginalRestoreStatus::Failed(failure(error));
                        continue;
                    }
                }
            }
            let Some(ticket) = self
                .host
                .native_restore_requests
                .entries
                .get(&request.serial)
                .and_then(|entry| entry.admitted)
            else {
                continue;
            };
            if self.host.native_restore_requests.status(client, request)?
                == OriginalRestoreStatus::Cancelled
            {
                self.release_original_save_restore(client, ticket)?;
                self.host
                    .native_restore_requests
                    .entries
                    .get_mut(&request.serial)
                    .unwrap()
                    .admitted = None;
                continue;
            }
            let outcome = match self.poll_original_save_restore(client, ticket) {
                Ok(OriginalRestoreStatus::Ready) => {
                    match self.commit_original_save_restore(client, ticket) {
                        Ok(()) => OriginalRestoreStatus::Committed,
                        Err(error) => OriginalRestoreStatus::Failed(failure(error)),
                    }
                }
                Ok(status) => status,
                Err(error) => OriginalRestoreStatus::Failed(failure(error)),
            };
            if matches!(
                outcome,
                OriginalRestoreStatus::Preparing | OriginalRestoreStatus::Ready
            ) {
                continue;
            }
            if self
                .native_restores
                .entries
                .get(&ticket.serial)
                .is_some_and(|entry| entry.ticket == ticket)
            {
                self.release_original_save_restore(client, ticket)?;
            }
            let entry = self
                .host
                .native_restore_requests
                .entries
                .get_mut(&request.serial)
                .unwrap();
            entry.admitted = None;
            entry.status = outcome;
        }
        self.release_retired_restore_requests()?;
        Ok(self.host.native_restore_requests.any_active())
    }
}
