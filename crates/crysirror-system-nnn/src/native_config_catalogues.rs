// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonConfig's constructor-loaded configSe/configSeVoice catalogues.
//! Filenames remain native data; decoding/playback and output devices are
//! shared Audio work. Preserve the selected buffer policy in every request.
use crate::{
    config::Names,
    native_config_volume::{Preview, Profile},
    native_resources::ResourceNames,
    resource_bridge::SourceOpener,
};
use anyhow::{Context, Result, ensure};
use crysirror_tasks::{Cancellation, Task, TaskPool};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub(crate) enum AudioRequest {
    NumberedButtonVoice(i32),
    SystemWave { name: Vec<u8>, delta: i32 },
    SystemVoice { name: Vec<u8>, same_buffer: bool },
}
#[derive(Clone)]
pub(crate) struct Catalogues {
    base: Arc<Names>,
    voices: Vec<Arc<Names>>,
    select_voice: bool,
}
impl Catalogues {
    pub(crate) fn read(
        profile: &Profile,
        names: &ResourceNames,
        open: &SourceOpener,
        voices: usize,
        cancel: &Cancellation,
    ) -> Result<Self> {
        ensure!(voices <= 64, "NNN Config system voice catalogue budget");
        let mut budget = 0usize;
        let mut read = |path: &str| -> Result<Arc<Names>> {
            cancel.check()?;
            let Some(reader) = open(&names.path(path)?, cancel)? else {
                return Ok(Arc::new(Names { tokens: vec![] }));
            };
            let bytes = crysirror_resource::read_all(reader.as_ref(), 1 << 20)?;
            budget = budget
                .checked_add(bytes.len())
                .context("NNN Config sound catalogue extent overflow")?;
            ensure!(
                budget <= 64 << 20,
                "NNN Config sound catalogue memory budget"
            );
            let table = Names::parse(&bytes, false)?;
            ensure!(
                table.tokens.len() <= 131072,
                "NNN Config sound name count budget"
            );
            cancel.check()?;
            Ok(Arc::new(table))
        };
        let base = if profile.previews {
            read("nya/configSe.xtx")?
        } else {
            Arc::new(Names { tokens: vec![] })
        };
        let mut catalogues = vec![base.clone()];
        // Original allocates alternative tables only when both conditions
        // hold; type0 always reads m_seList, not the unused m_seVoice[0].
        if profile.previews && profile.use_config_voice && voices > 0 {
            for voice in 1..=voices {
                catalogues.push(read(&format!("nya/configSeVoice{voice}.xtx"))?);
            }
        }
        Ok(Self {
            base,
            voices: catalogues,
            select_voice: profile.use_config_voice && voices > 0,
        })
    }
    pub(crate) fn resolve(
        &self,
        preview: Preview,
        system_voice: i32,
    ) -> Result<Option<AudioRequest>> {
        let Preview::ConfigCatalogue {
            index,
            voice,
            delta,
            same_buffer,
        } = preview
        else {
            let Preview::NumberedButtonVoice(number) = preview else {
                unreachable!()
            };
            return Ok(Some(AudioRequest::NumberedButtonVoice(number)));
        };
        ensure!(index >= 0, "NNN Config negative sound catalogue index");
        let at = (index as usize)
            .checked_mul(2)
            .context("NNN Config preview index overflow")?;
        // PlayExtSe2 compares against the BASE catalogue's pair count even
        // when GetSoundName later chooses an alternative voice table.
        if at >= self.base.tokens.len() / 2 * 2 {
            return Ok(None);
        }
        let table = if self.select_voice && system_voice != 0 {
            self.voices
                .get(usize::try_from(system_voice)?)
                .context("NNN Config selected system voice has no original catalogue")?
        } else {
            &self.base
        };
        // CNameList::GetName checks its actual token bound and returns NULL.
        // A missing/short alternate list is not a fallback to a different
        // character's voice. Define the original null filename as no-play.
        let Some(name) = table.tokens.get(at) else {
            return Ok(None);
        };
        if name.is_empty() {
            return Ok(None);
        }
        ensure!(
            name.len() <= 254 && !name.contains(&0),
            "NNN Config preview filename bounds"
        );
        Ok(Some(if voice {
            AudioRequest::SystemVoice {
                name: name.clone(),
                same_buffer,
            }
        } else {
            AudioRequest::SystemWave {
                name: name.clone(),
                delta,
            }
        }))
    }
}
pub(crate) struct Preparation {
    read: Option<Task<Catalogues>>,
    ready: Option<Catalogues>,
}
impl Preparation {
    pub(crate) fn begin(
        profile: Profile,
        names: ResourceNames,
        open: SourceOpener,
        voices: usize,
        pool: &TaskPool,
    ) -> Result<Self> {
        Ok(Self {
            read: Some(
                pool.spawn(move |cancel| {
                    Catalogues::read(&profile, &names, &open, voices, &cancel)
                })?,
            ),
            ready: None,
        })
    }
    pub(crate) fn poll(&mut self) -> Result<Option<Catalogues>> {
        if let Some(ready) = self.ready.as_ref() {
            return Ok(Some(ready.clone()));
        }
        let Some(catalogues) = self
            .read
            .as_mut()
            .context("NNN Config catalogue job absent")?
            .try_take()?
        else {
            return Ok(None);
        };
        self.read = None;
        self.ready = Some(catalogues.clone());
        Ok(Some(catalogues))
    }
}
