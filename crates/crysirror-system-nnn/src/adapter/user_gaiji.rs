//! CMyMessage::SetUserGaiji + CMyFont::SetUserGaijiFont extension boundary.
//! The base SPT dispatcher has no such opcode; game extensions call this typed
//! API. Resources and generic key/lookup/resize/reduction jobs stay engine-owned.
use super::*;
use crate::native_gaiji::Gaiji;
use std::collections::{BTreeMap, VecDeque};
pub(super) struct UserGaiji {
    name: Vec<u8>,
    characters: Vec<u8>,
    task: Option<Task<crysirror_core::image_processing::ImageInput>>,
    builder: Option<Gaiji>,
    mapping: BTreeMap<u16, u8>,
    pending: VecDeque<(u8, u16)>,
    paths: Vec<String>,
    done: bool,
}
impl UserGaiji {
    fn matches(&self, name: &[u8], characters: &[u8]) -> bool {
        self.name == name && self.characters == characters
    }
    fn pending(&self) -> bool {
        !self.done
    }
}
impl NativeAdapter {
    pub(super) fn cancel_user_gaiji(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(mut change) = self.user_gaiji.take() {
            change.task = None;
            change.builder = None;
            for path in change.paths.drain(..) {
                host.release_image_resource(self.bridge.client(), &path)?;
            }
        }
        Ok(())
    }
    /// Asynchronous native extension call. Retry with identical arguments until
    /// true. It creates no SPT receipt, fake instruction or synthetic success.
    /// A different request is accepted only after the previous one completed.
    pub fn change_user_gaiji(
        &mut self,
        host: &mut dyn HostServices,
        font: &[u8],
        characters: &[u8],
    ) -> Result<bool> {
        if let Some(change) = &self.user_gaiji {
            if change.matches(font, characters) {
                return self.poll_user_gaiji(host);
            }
            anyhow::ensure!(
                !change.pending(),
                "NNN user gaiji request changed before completion"
            );
        }
        if self.worker.is_some() || self.bridge.dependency_pending() {
            return Ok(false);
        }
        anyhow::ensure!(
            !font.is_empty()
                && font.len() <= 128
                && !font.contains(&0)
                && !characters.contains(&0)
                && characters.len() <= 60
                && characters.len() % 2 == 0,
            "NNN two-byte user gaiji name/list bounds"
        );
        let config = self
            .configuration
            .as_ref()
            .context("NNN user gaiji native configuration absent")?
            .clone();
        let resources = self.bridge.image_resources()?;
        let sizes = self.bridge.user_gaiji_sizes()?;
        let mut mapping = BTreeMap::new();
        let mut pending = VecDeque::new();
        for (index, ch) in characters.chunks_exact(2).enumerate() {
            let command = u16::from_be_bytes(ch.try_into().unwrap());
            if command == 0x8f49 {
                break;
            }
            let number = index as u8 + 25;
            mapping.entry(command).or_insert(number);
            for size in &sizes {
                pending.push_back((number, *size));
            }
        }
        anyhow::ensure!(
            self.bridge.user_gaiji_lease_count() + pending.len() <= 2048,
            "NNN user gaiji resource lease budget"
        );
        let name = font.to_vec();
        let source = name.clone();
        let task = self.pool.spawn(move |cancel| {
            let logical = [b"sys/".as_slice(), &source].concat();
            let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
            resources.image(&config.resources.picture(&logical, levels)?, &cancel)
        })?;
        self.user_gaiji = Some(UserGaiji {
            name,
            characters: characters.to_vec(),
            task: Some(task),
            builder: None,
            mapping,
            pending,
            paths: vec![],
            done: false,
        });
        self.poll_user_gaiji(host)
    }
    pub(super) fn user_gaiji_pending(&self) -> bool {
        self.user_gaiji.as_ref().is_some_and(UserGaiji::pending)
    }
    pub(super) fn poll_user_gaiji(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let result = self.poll_user_gaiji_inner(host);
        if result.is_err() {
            self.cancel_user_gaiji(host)?;
        }
        result
    }
    fn poll_user_gaiji_inner(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let Some(change) = self.user_gaiji.as_mut() else {
            return Ok(true);
        };
        if change.done {
            return Ok(true);
        }
        if let Some(task) = change.task.as_mut() {
            let Some(input) = task.try_take()? else {
                return Ok(false);
            };
            change.builder = Some(Gaiji::custom(input, std::mem::take(&mut change.pending))?);
            change.task = None;
        }
        let ready = change
            .builder
            .as_mut()
            .context("NNN user gaiji builder absent")?
            .poll(self.bridge.client(), host, &mut change.paths);
        let ready = match ready {
            Ok(ready) => ready,
            Err(error) => {
                for path in change.paths.drain(..) {
                    let _ = host.release_image_resource(self.bridge.client(), &path);
                }
                return Err(error);
            }
        };
        if !ready {
            return Ok(false);
        }
        self.bridge.install_user_gaiji(
            change.mapping.clone(),
            change.builder.as_ref().unwrap().images.clone(),
            std::mem::take(&mut change.paths),
        )?;
        change.done = true;
        Ok(true)
    }
}
