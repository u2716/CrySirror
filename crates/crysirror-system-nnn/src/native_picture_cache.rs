// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CPicture::InitWork/LoadDWQ/Flush's native cache and asynchronous read receipt.
//! Metadata does not own pixels. Scoped engine leases remain in the payload,
//! including RGB and independent mask allocations retained after failed opens.
use anyhow::{Context, Result, ensure};
use crysirror_core::state::ClientId;

pub(crate) trait Storage: Clone {
    fn validate_scope(&self, client: ClientId) -> Result<()>;
}
impl Storage for crate::image_bridge::NativePictureViews {
    fn validate_scope(&self, client: ClientId) -> Result<()> {
        ensure!(
            [&self.raw, &self.opaque, &self.keyed]
                .iter()
                .all(|r| r.client() == client && r.size() == self.raw.size()),
            "NNN cached picture scope/size"
        );
        if let Some(mask) = &self.mask_storage {
            ensure!(
                mask.buffer.client() == client && mask.view.client() == client,
                "NNN cached mask scope"
            );
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BeginLoad {
    CachedTrue,
    Read { token: u64 },
    Pending { token: u64 },
}
#[derive(Clone, Debug)]
struct Pending {
    owner: u64,
    token: u64,
}
#[derive(Clone)]
pub(crate) struct PictureCache<T: Storage> {
    client: ClientId,
    pub filename: Vec<u8>,
    pub load_ok: bool,
    pub indexed: bool,
    pub mask_exists: bool,
    pub storage: Option<T>,
    next_token: u64,
    pending: Option<Pending>,
}
impl<T: Storage> PictureCache<T> {
    pub(crate) fn client(&self) -> ClientId {
        self.client
    }
    pub(crate) fn empty(client: ClientId) -> Result<Self> {
        ensure!(client.0 != 0, "NNN picture cache owner");
        Ok(Self {
            client,
            filename: Vec::new(),
            load_ok: false,
            indexed: false,
            mask_exists: false,
            storage: None,
            next_token: 1,
            pending: None,
        })
    }
    pub(crate) fn sized(
        client: ClientId,
        storage: T,
        indexed: bool,
        create_mask: bool,
    ) -> Result<Self> {
        storage.validate_scope(client)?;
        let mut state = Self::empty(client)?;
        state.storage = Some(storage);
        state.load_ok = true;
        state.indexed = indexed;
        state.mask_exists = create_mask;
        Ok(state)
        // Sized native storage was uninitialized; callers provide explicit
        // shared zero planes before publication rather than copying that RAM.
    }
    pub(crate) fn begin(&mut self, owner: u64, name: &[u8], indexed: bool) -> Result<BeginLoad> {
        ensure!(
            name.len() <= 254 && !name.contains(&0),
            "NNN CPicture filename capacity"
        );
        if let Some(pending) = &self.pending {
            ensure!(
                pending.owner == owner && self.filename == name && self.indexed == indexed,
                "NNN cached picture read belongs to another native delivery"
            );
            return Ok(BeginLoad::Pending {
                token: pending.token,
            });
        }
        // Source returnsTRUE BEFORE changing depth, flags or testing loadOk.
        // A repeated missing name therefore returnsTRUE with loadOkFALSE and
        // old pixels still present. Each native caller chooses whether it
        // subsequently tests loadOk or performs a raw GetBuffer operation.
        if self.filename == name {
            return Ok(BeginLoad::CachedTrue);
        }
        let token = self.next_token;
        self.next_token = token
            .checked_add(1)
            .context("NNN picture cache token exhausted")?;
        self.filename = name.to_vec();
        self.indexed = indexed;
        self.mask_exists = false;
        self.pending = Some(Pending { owner, token });
        Ok(BeginLoad::Read { token })
    }
    fn check_token(&self, token: u64) -> Result<()> {
        ensure!(
            self.pending.as_ref().is_some_and(|p| p.token == token),
            "NNN picture completion token mismatch"
        );
        Ok(())
    }
    pub(crate) fn missing(&mut self, token: u64) -> Result<()> {
        self.check_token(token)?;
        self.load_ok = false;
        self.pending = None;
        Ok(())
        // Name/depth/cleared mask fact remain, as do old pixels, palette,
        // dimensions, character clip and independently allocated mask bytes.
    }
    pub(crate) fn loaded(&mut self, token: u64, storage: T, mask_exists: bool) -> Result<()> {
        self.check_token(token)?;
        storage.validate_scope(self.client)?;
        self.storage = Some(storage);
        self.load_ok = true;
        self.mask_exists = mask_exists;
        self.pending = None;
        Ok(())
    }
    pub(crate) fn flush(&mut self) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "NNN Flush during an admitted picture read"
        );
        self.filename.clear();
        Ok(())
        // CPicture::Flush clears only filename/makedFilename, never loadOk,
        // pixel/mask/palette allocations or their dimensions/flags.
    }
}
