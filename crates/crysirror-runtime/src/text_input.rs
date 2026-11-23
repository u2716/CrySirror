//! Single-owner input broker. Events and native requests meet through scoped
//! handles. Immutable receipts share text; append/take never touch VM state.
use crysirror_core::{Error, Result, service::*, state::ClientId};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
pub struct TextInputBroker {
    owner: u64,
    serial: u64,
    revision: u64,
    available: bool,
    focused: bool,
    active: Option<ClientId>,
    sessions: BTreeMap<ClientId, TextInputSessionInfo>,
}
impl Default for TextInputBroker {
    fn default() -> Self {
        Self {
            owner: NEXT_OWNER
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .unwrap_or(0),
            serial: 0,
            revision: 0,
            available: false,
            focused: false,
            active: None,
            sessions: BTreeMap::new(),
        }
    }
}
impl TextInputBroker {
    pub fn set_presenter_available(&mut self, available: bool) {
        self.available = available;
        if !available {
            self.focus(false);
            for info in self.sessions.values_mut() {
                info.presented_revision = 0;
                info.platform_ime_enabled = None;
            }
        }
    }
    pub fn invalidate(&mut self) {
        self.sessions.clear();
        self.active = None;
    }
    pub fn retire_client(&mut self, client: ClientId) {
        self.sessions.remove(&client);
        if self.active == Some(client) {
            self.active = None;
        }
    }
    fn next_revision(&mut self) -> Result<u64> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::Limit("text input revision"))?;
        Ok(self.revision)
    }
    fn require_presenter(&self) -> Result<()> {
        if !self.available {
            Err(Error::Unsupported("text input presenter"))
        } else {
            Ok(())
        }
    }
    fn get(
        &self,
        client: ClientId,
        handle: TextInputSessionHandle,
    ) -> Result<&TextInputSessionInfo> {
        if handle.client != client || handle.owner != self.owner {
            return Err(Error::Invalid("foreign text input session"));
        }
        self.sessions
            .get(&client)
            .filter(|s| s.handle == handle)
            .ok_or(Error::NotFound)
    }
    fn focus_owner(&self, client: ClientId) -> Result<()> {
        if self.active.is_some_and(|owner| {
            owner != client && self.sessions.get(&owner).is_some_and(|s| s.spec.capture)
        }) {
            return Err(Error::Limit("text input focus lease"));
        }
        Ok(())
    }
    pub fn open(
        &mut self,
        client: ClientId,
        spec: TextInputSessionSpec,
    ) -> Result<TextInputSessionHandle> {
        self.require_presenter()?;
        spec.validate()?;
        self.focus_owner(client)?;
        if self.sessions.contains_key(&client) {
            return Err(Error::Invalid("text input session already open"));
        }
        if self.sessions.len() >= 64 {
            return Err(Error::Limit("text input sessions"));
        }
        let serial = self
            .serial
            .checked_add(1)
            .ok_or(Error::Limit("text input serial"))?;
        if self.owner == 0 {
            return Err(Error::Limit("text input owner"));
        }
        let revision = self.next_revision()?;
        self.serial = serial;
        let handle = TextInputSessionHandle {
            owner: self.owner,
            client,
            serial,
        };
        self.sessions.insert(
            client,
            TextInputSessionInfo {
                handle,
                spec,
                revision,
                presented_revision: 0,
                committed: Arc::new(String::new()),
                preedit: Arc::new(String::new()),
                preedit_cursor: None,
                focused: self.focused,
                platform_ime_enabled: None,
                font_outcome: TextInputFontOutcome::NotRequested,
            },
        );
        self.active = Some(client);
        Ok(handle)
    }
    pub fn configure(
        &mut self,
        client: ClientId,
        handle: TextInputSessionHandle,
        spec: TextInputSessionSpec,
    ) -> Result<()> {
        self.require_presenter()?;
        spec.validate()?;
        self.focus_owner(client)?;
        let info = self.get(client, handle)?;
        if info.committed.len() > spec.max_committed_bytes {
            return Err(Error::Limit("text input retained queue"));
        }
        if info.spec != spec || self.active != Some(client) {
            let revision = self.next_revision()?;
            let info = self.sessions.get_mut(&client).unwrap();
            info.spec = spec;
            info.revision = revision;
            info.font_outcome = TextInputFontOutcome::NotRequested;
        }
        self.active = Some(client);
        Ok(())
    }
    pub fn info(
        &self,
        client: ClientId,
        handle: TextInputSessionHandle,
    ) -> Result<TextInputSessionInfo> {
        Ok(self.get(client, handle)?.clone())
    }
    pub fn take(
        &mut self,
        client: ClientId,
        handle: TextInputSessionHandle,
    ) -> Result<Arc<String>> {
        self.get(client, handle)?;
        Ok(std::mem::replace(
            &mut self.sessions.get_mut(&client).unwrap().committed,
            Arc::new(String::new()),
        ))
    }
    pub fn replace(
        &mut self,
        client: ClientId,
        handle: TextInputSessionHandle,
        text: String,
    ) -> Result<()> {
        let info = self.get(client, handle)?;
        if text.len() > info.spec.max_committed_bytes {
            return Err(Error::Limit("text input queue bytes"));
        }
        if text.contains('\0') {
            return Err(Error::Invalid("text input NUL"));
        }
        self.sessions.get_mut(&client).unwrap().committed = Arc::new(text);
        Ok(())
    }
    pub fn release(&mut self, client: ClientId, handle: TextInputSessionHandle) -> Result<()> {
        self.get(client, handle)?;
        self.retire_client(client);
        Ok(())
    }
    /// Append an actual committed string, without editing text already
    /// consumed/displayed by a game. The presenter keeps key-event controls on
    /// their key route; an IME result is not a synthesized Backspace command.
    pub fn commit(&mut self, text: &str) -> Result<bool> {
        if !self.available || !self.focused {
            return Ok(false);
        }
        let Some(info) = self
            .active
            .and_then(|c| self.sessions.get_mut(&c))
            .filter(|s| s.spec.capture)
        else {
            return Ok(false);
        };
        if text.contains('\0') {
            return Err(Error::Invalid("text input committed NUL"));
        }
        let count = text.len();
        if info
            .committed
            .len()
            .checked_add(count)
            .is_none_or(|n| n > info.spec.max_committed_bytes)
        {
            return Err(Error::Limit("text input committed bytes"));
        }
        Arc::make_mut(&mut info.committed).push_str(text);
        info.preedit = Arc::new(String::new());
        info.preedit_cursor = None;
        Ok(true)
    }
    pub fn preedit(&mut self, text: &str, cursor: Option<(usize, usize)>) -> Result<()> {
        if text.len() > 65536
            || cursor.is_some_and(|(a, b)| {
                a > text.len()
                    || b > text.len()
                    || !text.is_char_boundary(a)
                    || !text.is_char_boundary(b)
            })
        {
            return Err(Error::Invalid("text input preedit extent"));
        }
        if !self.focused {
            return Ok(());
        }
        if let Some(info) = self.active.and_then(|c| self.sessions.get_mut(&c)) {
            info.preedit = Arc::new(text.to_owned());
            info.preedit_cursor = cursor;
        }
        Ok(())
    }
    pub fn focus(&mut self, focused: bool) {
        self.focused = focused;
        for info in self.sessions.values_mut() {
            info.focused = focused;
            if !focused {
                info.preedit = Arc::new(String::new());
                info.preedit_cursor = None;
            }
        }
    }
    pub fn ime_enabled(&mut self, enabled: bool) {
        if let Some(info) = self.active.and_then(|c| self.sessions.get_mut(&c)) {
            info.platform_ime_enabled = Some(enabled);
            if !enabled {
                info.preedit = Arc::new(String::new());
                info.preedit_cursor = None;
            }
        }
    }
    pub fn presentation(&self) -> Option<TextInputSessionInfo> {
        self.active.and_then(|c| self.sessions.get(&c)).cloned()
    }
    pub fn presented(
        &mut self,
        handle: TextInputSessionHandle,
        revision: u64,
        font: TextInputFontOutcome,
    ) -> Result<()> {
        let info = self.get(handle.client, handle)?;
        if revision != info.revision {
            return Err(Error::Invalid("stale text input presentation"));
        }
        let info = self.sessions.get_mut(&handle.client).unwrap();
        info.presented_revision = revision;
        info.font_outcome = font;
        Ok(())
    }
}
