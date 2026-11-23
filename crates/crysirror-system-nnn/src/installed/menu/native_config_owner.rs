// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Menu-side Config owner: Init from the drained Bridge, admitted Calcu at the
//! General frame period, Print publication and effect hand-off to the host.
use super::native_config::{ConfigMenu, Effect, Frame};
use super::*;
use std::collections::BTreeSet;
const TEXT_LAYERS: u32 = ROOT + 4900;
impl Menu {
    pub(in crate::installed) fn native_config_wanted(&self) -> bool {
        self.kind == Kind::Config && self.native_config.is_none() && self.ready.as_ref().is_some_and(|p| p.table.is_some())
    }
    /// CCommonConfig::Init. inSetVar is written before any condition is read.
    pub(in crate::installed) fn begin_native_config(
        &mut self,
        vm: &mut NativeAdapter,
        resources: Option<&NativeResources>,
        pool: &TaskPool,
    ) -> Result<()> {
        let table = self.native_table()?.clone();
        if let Some((id, value)) = ConfigMenu::in_variable(&table, &self.config)? {
            vm.write_modal_variable(id, value)?;
        }
        let variables = vm.gallery_variables()?;
        let context = vm.config_ui_context()?;
        let owner = ConfigMenu::begin(
            &table,
            &self.config,
            &variables,
            context,
            0,
            self.back_script.is_some(),
        )?;
        if owner.session.layout.sliders.previews {
            let resources = resources.context("NNN Config sound catalogue resources absent")?;
            let voices = usize::try_from(self.config.setup.integer(b"useSystemVoice")?.unwrap_or(0))?;
            self.config_catalogues = Some(crate::native_config_catalogues::Preparation::begin(
                owner.session.layout.sliders.clone(),
                self.config.resources.clone(),
                resources.file_view(),
                voices,
                pool,
            )?);
        }
        self.native_config = Some(owner);
        self.published = false;
        Ok(())
    }
    pub(in crate::installed) fn config_catalogues(&mut self) -> Result<Option<crate::native_config_catalogues::Catalogues>> {
        match self.config_catalogues.as_mut() {
            Some(work) => work.poll(),
            None => Ok(None),
        }
    }
    pub(in crate::installed) fn native_config_published(&self) -> bool {
        self.native_config.as_ref().is_some_and(|c| c.published_page.is_some())
    }
    /// Drain one owner effect, finish resource/Print work, then (when admitted
    /// and due) run one Calcu and start the following Print.
    pub(in crate::installed) fn poll_native_config(
        &mut self,
        client: ClientId,
        font: &str,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
        vm: &mut NativeAdapter,
    ) -> Result<Option<Effect>> {
        let config = self.native_config.as_mut().context("NNN native Config owner absent")?;
        if let Some(effect) = config.effects.pop_front() {
            return Ok(Some(effect));
        }
        if !config.poll_resources(host)? {
            return Ok(None);
        }
        if config.print_in_flight() && config.poll_print(client, host)? {
            self.publish_native_config(client, font, host, false)?;
        }
        let config = self.native_config.as_mut().unwrap();
        if config.completed.is_none() {
            if config.print_in_flight() {
                return Ok(None);
            }
            let base = if config.back_script() { vm.presented_frame() } else { None };
            config.begin_print(base, client, host)?;
            return Ok(None);
        }
        // Print applied m_pageYoyaku after drawing the old page.
        if !config.print_in_flight() && config.commit_page()? {
            return Ok(config.effects.pop_front());
        }
        if !admit || self.entry.is_some() || config.session.page_pending.is_some() {
            return Ok(None);
        }
        let now = host.now_ms();
        if self.config_next_ms.is_some_and(|next| now < next) {
            return Ok(None);
        }
        self.config_next_ms = Some(now.checked_add(vm.frame_period_ms()).context("NNN Config frame deadline overflow")?);
        let events: Vec<Value> =
            serde_json::from_slice(&host.query_engine(client, EngineQuery::UiSubtreeEvents(ROOT))?)?;
        let activated = events
            .iter()
            .filter(|e| matches!(e["kind"].as_str(), Some("activate" | "nnn_config_activate")))
            .filter_map(|e| e["widget"]["id"].as_u64().and_then(|id| u32::try_from(id).ok()))
            .collect::<BTreeSet<u32>>();
        let config = self.native_config.as_mut().unwrap();
        config.counter = config.counter.checked_add(1).context("NNN Config Calcu counter")?;
        let ticket = config.counter;
        config.calcu(host, client, &Frame { input, activated: &activated, ticket })?;
        if let Some(effect) = config.effects.pop_front() {
            return Ok(Some(effect));
        }
        if config.ready && config.session.page_pending.is_none() && !config.print_in_flight() {
            let base = if config.back_script() { vm.presented_frame() } else { None };
            config.begin_print(base, client, host)?;
        }
        Ok(None)
    }
    pub(in crate::installed) fn publish_native_config(
        &mut self,
        client: ClientId,
        font: &str,
        host: &mut dyn HostServices,
        force_widgets: bool,
    ) -> Result<()> {
        let config = self.native_config.as_mut().context("NNN native Config owner absent")?;
        let Some(image) = config.completed.clone() else {
            return Ok(());
        };
        let path = host
            .publish_image_resource(client, ROOT, image)?
            .path
            .relative()
            .to_owned();
        let mut layers = vec![json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,SIZE[0],SIZE[1]],"order":[super::native_config::ORDER,0],"paint":{"kind":"image","asset":path}}})];
        let text = config.text_layers(client, host, TEXT_LAYERS)?;
        let text_ids = text
            .iter()
            .filter_map(|t| t["layer"]["id"].as_u64().map(|id| id as u32))
            .collect::<Vec<_>>();
        let stale = self
            .nodes
            .iter()
            .copied()
            .filter(|id| *id != ROOT && !text_ids.contains(id))
            .collect::<Vec<_>>();
        layers.extend(stale.iter().map(|id| json!({"op":"remove","id":id})));
        layers.extend(text);
        let page = config.session.page;
        let mut ui = vec![];
        if force_widgets || config.published_page != Some(page) {
            let current: Value = serde_json::from_slice(&host.query_engine(client, EngineQuery::UiWidget(ROOT))?)?;
            if !current.is_null() {
                ui.push(json!({"op":"remove","id":ROOT}));
            }
            ui.extend(config.widgets(font, true)?);
        } else if config.widgets_dirty {
            ui.extend(config.widgets(font, true)?.into_iter().skip(1));
        }
        config.widgets_dirty = false;
        config.published_page = Some(page);
        if let Err(error) = host.submit_engine(client, 1, &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?) {
            let _ = host.release_image_resource(client, &path);
            return Err(error.into());
        }
        for old in std::mem::replace(&mut self.paths, vec![path]) {
            host.release_image_resource(client, &old)?;
        }
        self.nodes = std::iter::once(ROOT).chain(text_ids).collect();
        self.published = true;
        Ok(())
    }
}
