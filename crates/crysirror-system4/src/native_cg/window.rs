// GPL-2.0-or-later. Original SACT2 GetSurface7A60 / SactEngine target
//6080/61B0/5180/05870 and pinned scene/gfx present policy read statically.
//! Native WindowSurface state and display policy, using shared resource leases.
use super::*;
impl NativeCg {
    pub(crate) fn current_window_path(&self, owner: ClientId) -> Result<Option<String>> {
        self.current_window
            .as_ref()
            .map(|image| {
                ensure!(
                    image._resource.client() == owner,
                    "foreign native current window"
                );
                Ok(image.path.clone())
            })
            .transpose()
    }
    /// Change raw native WindowSurface bytes without presenting a device frame.
    /// Scene sprite metadata and character records remain independent.
    pub(crate) fn write_window_result(
        &mut self,
        owner: ClientId,
        resource: Arc<dyn ImageResource>,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        self.install_window(owner, resource, bridge, host, false)
    }
    pub(crate) fn present_window_result(
        &mut self,
        owner: ClientId,
        resource: Arc<dyn ImageResource>,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        self.install_window(owner, resource, bridge, host, true)
    }
    fn install_window(
        &mut self,
        owner: ClientId,
        resource: Arc<dyn ImageResource>,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
        present: bool,
    ) -> Result<()> {
        ensure!(
            resource.client() == owner,
            "native WindowSurface foreign image"
        );
        let size = resource.size();
        let [w, h] = size;
        if let Some(sact) = bridge.sact.as_ref().filter(|s| s.initialized) {
            ensure!(
                sact.canvas == size,
                "native WindowSurface physical extent changed"
            );
            sact.sprites
                .as_ref()
                .context("native window sprite set absent")?
                .require_owner(owner)?;
        }
        let publication =
            host.publish_image_resource(owner, crate::sact::MAIN_SPRITE, resource.clone())?;
        let path = publication.path.relative().to_owned();
        let mut layers = Vec::with_capacity(2);
        if present {
            layers.push(serde_json::json!({"op":"put","layer":{"id":16,"rect":[0,0,w,h],
                "paint":{"kind":"image","asset":path,"tint":[255,255,255,255]},"order":[0,-1],"visible":true}}));
        }
        if !layers.is_empty() {
            if let Err(error) = host.submit_engine(
                owner,
                1,
                &serde_json::to_vec(&serde_json::json!({"layers":layers}))?,
            ) {
                host.release_image_resource(owner, &path)?;
                return Err(error.into());
            }
        }
        if let Some(sprites) = bridge
            .sact
            .as_mut()
            .filter(|s| s.initialized)
            .and_then(|s| s.sprites.as_mut())
        {
            sprites.install_window_data(size, path.clone());
        }
        let next = Published {
            path,
            _resource: resource,
        };
        if let Some(old) = self.current_window.replace(next.clone()) {
            self.retired.push(old);
        }
        if present {
            if let Some(old) = self.presented.replace(next) {
                self.retired.push(old);
            }
        }
        self.drain_retired(owner, host)
    }
    pub(super) fn drain_retired(
        &mut self,
        owner: ClientId,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        //The raw-current and displayed paths may be the same generation or
        //different ones. Neither is retired while either logical owner uses it.
        let mut index = 0;
        while index < self.retired.len() {
            let path = &self.retired[index].path;
            if self
                .current_window
                .as_ref()
                .is_some_and(|p| &p.path == path)
                || self.presented.as_ref().is_some_and(|p| &p.path == path)
                || self.images.values().any(|p| &p.path == path)
            {
                self.retired.swap_remove(index);
                continue;
            }
            host.release_image_resource(owner, path)?;
            let old = self.retired.swap_remove(index);
            self.retired.retain(|p| p.path != old.path);
        }
        Ok(())
    }
}

impl NativeCg {
    ///Commit native WP_GetSP's defined raw-window write and its repaired
    ///target contents together. The target's original newly allocated bytes
    ///were NEVER initialized (64D0 mistakenly draws05810 into WindowSurface).
    ///Authorized repair copies the completed backdrop result into that target,
    ///with an opaque logical RGB plane. It is not original random pixel data.
    pub(crate) fn install_wallpaper_snapshot(
        &mut self,
        owner: ClientId,
        number: i32,
        raw: Arc<dyn ImageResource>,
        target: Arc<dyn ImageResource>,
        bridge: &mut Bridge,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        validate_sprite(bridge, number)?;
        let sact = bridge.sact.as_ref().unwrap();
        let size = sact.canvas;
        ensure!(
            raw.client() == owner
                && target.client() == owner
                && raw.size() == size
                && target.size() == size,
            "native wallpaper snapshot scope/extent"
        );
        let mut sprite = sact
            .sprites
            .as_ref()
            .unwrap()
            .get(number)
            .cloned()
            .unwrap_or_default();
        sprite.size = size;
        sprite.alpha = false;
        //Validate the map target before admitting either publication. The
        //temporary path is never submitted or installed.
        sprite.surface = Some(Surface::Image(String::new()));
        let mut command = sprite
            .layer(number)?
            .context("native WP_GetSP target layer absent")?;
        let raw_path = host
            .publish_image_resource(owner, crate::sact::MAIN_SPRITE, raw.clone())?
            .path
            .relative()
            .to_owned();
        let target_path = match host.publish_image_resource(
            owner,
            if number == -1 {
                crate::sact::MAIN_SPRITE
            } else {
                0x41000000 + number as u32
            },
            target.clone(),
        ) {
            Ok(p) => p.path.relative().to_owned(),
            Err(error) => {
                host.release_image_resource(owner, &raw_path)?;
                return Err(error.into());
            }
        };
        sprite.surface = Some(Surface::Image(target_path.clone()));
        command["layer"]["paint"]["asset"] = serde_json::json!(target_path);
        if let Err(error) = host.submit_engine(
            owner,
            1,
            &serde_json::to_vec(&serde_json::json!({"layers":[command]}))?,
        ) {
            host.release_image_resource(owner, &raw_path)?;
            host.release_image_resource(owner, &target_path)?;
            return Err(error.into());
        }
        let sprites = bridge.sact.as_mut().unwrap().sprites.as_mut().unwrap();
        sprites.install_prepared(number, sprite);
        sprites.install_window_data(size, raw_path.clone());
        if let Some(old) = self.current_window.replace(Published {
            path: raw_path,
            _resource: raw,
        }) {
            self.retired.push(old)
        }
        if let Some(old) = self.images.insert(
            number,
            Published {
                path: target_path,
                _resource: target,
            },
        ) {
            self.retired.push(old)
        }
        //Layer16 and its displayed-generation lease are deliberately intact.
        self.drain_retired(owner, host)
    }
}
