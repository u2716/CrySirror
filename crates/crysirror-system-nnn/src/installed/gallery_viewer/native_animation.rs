// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Source32 CG preloads the same clamped CPictures as effect137. Each load
//! completes in source order, including missing-file/retained-buffer facts.
use super::*;
impl PictureViewer {
    pub(super) fn poll_animation_buffers(
        &mut self,
        host: &mut dyn HostServices,
        mut vm: Option<&mut NativeAdapter>,
    ) -> Result<bool> {
        let loading = self.loading.as_mut().unwrap();
        if loading.animation_count == 0 {
            return Ok(true);
        }
        if loading.animation_native.is_none() {
            loading.animation_native =
                Some(vm.as_deref().and_then(|v| v.animation_bank()).is_some());
        }
        if !loading.animation_native.unwrap() {
            if let Some(read) = loading.animation_read.as_mut() {
                let Some(source) = read.try_take()? else {
                    return Ok(false);
                };
                loading
                    .prepared
                    .inputs
                    .push_back((source, SpriteAlpha::Opaque));
                loading.animation_read = None;
            }
            if let Some(name) = loading.prepared.animation_names.pop_front() {
                let resources = self.resources.clone();
                let names = self.config.resources.clone();
                let levels =
                    u32::try_from(self.config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
                loading.animation_read = Some(self.pool.spawn(move |cancel| {
                    resources.image_source(&names.picture(&name, levels)?, &cancel)
                })?);
                return Ok(false);
            }
            return Ok(true);
        }
        let owner = vm
            .as_deref_mut()
            .context("NNN source CG animation lost its owner")?;
        if let Some(work) = loading.animation_load.as_mut() {
            let Some(ready) = work.poll(host)? else {
                return Ok(false);
            };
            owner.adopt_animation_bank(ready.bank)?;
            // FALSE is an original missing-file result. It still leaves the
            // cache's filename/loadOk/buffer metadata changed, not a fake image.
            loading.animation_load = None;
        }
        let bank = owner
            .animation_bank()
            .context("NNN source CG animation bank absent")?;
        if let Some(mut name) = loading.prepared.animation_names.pop_front() {
            let index = loading.animation_count - loading.prepared.animation_names.len() - 1;
            // Source uses the live tag, not a separate gallery-only prefix.
            let digits = name.split_off(b"bg\\bganime".len());
            name = bank.config().tag.clone();
            name.extend(digits);
            if let Some(work) = crate::native_animation_session::BufferLoad::begin(
                &bank,
                &owner.animation_context()?,
                index as i32,
                index as u64,
                &name,
            )? {
                loading.animation_load = Some(work);
                return Ok(false);
            }
            return Ok(false);
        }
        if loading.images.is_empty() {
            // Do not snapshot a buffer before later preloads overwrite a
            // clamped alias. GetAnimeBuffer(i) is queried after the whole loop.
            for index in 0..loading.animation_count {
                if let Some((view, load_ok)) = bank.buffer(index as i32)? {
                    loading.images.push((view, SpriteAlpha::Opaque));
                    loading.animation_loaded.push(load_ok);
                }
            }
        }
        Ok(true)
    }
}
