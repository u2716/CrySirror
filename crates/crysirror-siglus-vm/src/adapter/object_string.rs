//! Native string resource intents on scoped engine font tasks and glyph layers.
use super::*;
use crate::native_string::{self, PreparedString, Request, StringPlan};
use crysirror_core::service::{
    FontLibrary, GlyphBoundsProgress, GlyphBoundsQuery, GlyphBoundsRequest, GlyphBoundsSpec,
};
struct Entry {
    request: Request,
    plan: StringPlan,
    query: Option<Box<dyn GlyphBoundsQuery>>,
    deferred: Option<(Arc<dyn FontLibrary>, GlyphBoundsSpec)>,
    baselines: Vec<f32>,
    ready: Option<PreparedString>,
}
#[derive(Default)]
pub(crate) struct ObjectStrings {
    entries: BTreeMap<u64, Entry>,
}
impl ObjectStrings {
    pub(super) fn preparing(&self) -> bool {
        self.entries
            .values()
            .any(|entry| entry.query.is_some() || entry.deferred.is_some())
    }
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn frame(&self, generation: u64) -> Option<&PreparedString> {
        self.entries.get(&generation)?.ready.as_ref()
    }
    pub(super) fn prepare(
        &mut self,
        client: ClientId,
        requests: BTreeMap<u64, Request>,
        host: &mut dyn HostServices,
    ) -> Result<()> {
        for (generation, request) in requests {
            ensure!(
                generation == request.definition.generation,
                "native STRING request generation"
            );
            if let Some(old) = self.entries.get(&generation) {
                ensure!(
                    old.request.text == request.text
                        && old.request.definition.capture == request.definition.capture
                        && bincode::serialize(&old.request.param)?
                            == bincode::serialize(&request.param)?
                        && old.request.vertical == request.vertical,
                    "native STRING generation reused for a different resource"
                );
                continue;
            }
            ensure!(
                self.entries.len() < 1024,
                "native STRING resident resource count"
            );
            let plan = native_string::compile(&request.text, &request.param, request.vertical)?;
            ensure!(
                !request.vertical,
                "native vertical STRING glyph orientation is not mapped"
            );
            let fonts = host.font_library(client)?;
            let capture = &request.definition.capture;
            let alias = if capture.font_name.is_empty() {
                None
            } else {
                fonts.resolve_alias(&capture.font_name)?
            }
            .or(fonts.resolve_alias(&capture.fallback_alias)?)
            .context("native STRING has no installed engine font or fallback")?;
            let layout = plan.layout()?;
            let mut baselines = Vec::with_capacity(layout.glyphs.len());
            let mut glyphs = Vec::with_capacity(layout.glyphs.len());
            for glyph in &layout.glyphs {
                let ascent = fonts
                    .metrics(&alias, f32::from(glyph.size), glyph.character)?
                    .ascent
                    .ceil()
                    .max(1.);
                baselines.push(ascent);
                glyphs.push(GlyphBoundsRequest {
                    character: glyph.character,
                    size: f32::from(glyph.size),
                    baseline: [0., ascent],
                });
            }
            let spec = GlyphBoundsSpec {
                font: alias,
                glyphs,
            };
            self.entries.insert(
                generation,
                Entry {
                    request,
                    plan,
                    query: None,
                    deferred: Some((fonts, spec)),
                    baselines,
                    ready: None,
                },
            );
        }
        Ok(())
    }
    pub(super) fn poll(
        &mut self,
        client: ClientId,
        sizes: &mut BTreeMap<u64, [u32; 2]>,
    ) -> Result<bool> {
        let mut complete = true;
        // Restore may discover hundreds of recipes at once. Admit one engine
        // task at a time instead of reserving scratch for the entire scene.
        // The shared worker still owns execution, cancellation and accounting.
        let mut in_flight = self
            .entries
            .values()
            .filter(|entry| entry.query.is_some())
            .count();
        for (generation, entry) in &mut self.entries {
            if entry.deferred.is_some() && in_flight == 0 {
                let (fonts, spec) = entry.deferred.as_ref().unwrap();
                entry.query = Some(fonts.begin_glyph_bounds(spec.clone())?);
                entry.deferred = None;
                in_flight += 1;
            }
            if entry.deferred.is_some() {
                complete = false;
                continue;
            }
            if let Some(query) = entry.query.as_mut() {
                match query.poll()? {
                    GlyphBoundsProgress::Pending => {
                        complete = false;
                        continue;
                    }
                    GlyphBoundsProgress::Failed(error) => return Err(error.into()),
                    GlyphBoundsProgress::Cancelled => {
                        anyhow::bail!("native STRING glyph task was cancelled")
                    }
                    GlyphBoundsProgress::Ready(font) => {
                        let r = &entry.request;
                        let body = r.color(r.param.moji_color, [255; 4]);
                        let colors = entry
                            .plan
                            .colors
                            .iter()
                            .map(|n| r.color(*n, body))
                            .collect();
                        entry.ready = Some(PreparedString::new(
                            client,
                            &entry.plan,
                            &r.definition.capture,
                            &r.param,
                            font,
                            entry.baselines.clone(),
                            colors,
                            r.color(r.param.shadow_color, [0, 0, 0, 255]),
                            r.color(r.param.fuchi_color, [0, 0, 0, 255]),
                        )?);
                        entry.query = None;
                        in_flight -= 1;
                    }
                }
            }
            if let Some(ready) = &entry.ready {
                sizes.insert(*generation, ready.size);
            }
        }
        Ok(complete)
    }
}
impl SiglusVm {
    /// Call only after a whole opcode/frame commits. A rolled-back callback
    /// may replay several resource generations and still needs their receipts.
    pub(super) fn retire_strings(&mut self) -> Result<()> {
        if self.object_strings.entries.is_empty() && self.state.io.text_sizes.is_empty() {
            return Ok(());
        }
        let Some(vm) = &self.state.vm else {
            return Ok(());
        };
        let mut live = std::collections::BTreeSet::new();
        let mut todo =
            vm.ctx
                .globals
                .stage_forms
                .values()
                .flat_map(|s| {
                    s.object_lists.values().flatten().chain(
                        s.mwnd_lists.values().flatten().flat_map(|w| {
                            w.object_list
                                .iter()
                                .chain(&w.face_list)
                                .chain(&w.button_list)
                        }),
                    )
                })
                .collect::<Vec<_>>();
        let mut visited = 0;
        while let Some(object) = todo.pop() {
            visited += 1;
            ensure!(visited <= 4096, "native STRING retirement traversal budget");
            if object.object_type == 3 {
                if let Some(definition) = &object.runtime.native_text {
                    live.insert(definition.generation);
                }
            }
            todo.extend(&object.runtime.child_objects);
        }
        self.object_strings
            .entries
            .retain(|id, _| live.contains(id));
        self.state.io.text_sizes.retain(|id, _| live.contains(id));
        Ok(())
    }
    pub(super) fn reset_strings(&mut self) {
        self.object_strings = Default::default();
        self.state.io.pending_texts.clear();
        self.state.io.text_sizes.clear();
    }

    pub(super) fn poll_strings(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let requests = std::mem::take(&mut self.state.io.pending_texts);
        self.object_strings
            .prepare(ClientId(self.state.client), requests, host)?;
        self.object_strings
            .poll(ClientId(self.state.client), &mut self.state.io.text_sizes)
    }
    pub(super) fn hydrate_strings(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        let Some(vm) = self.state.vm.as_mut() else {
            return Ok(true);
        };
        let environment = native_string::Environment::new(&vm.ctx);
        let mut requests = BTreeMap::new();
        let revision = &mut vm.ctx.native_media_revision;
        let mut todo = vm
            .ctx
            .globals
            .stage_forms
            .values_mut()
            .map(|s| &mut **s)
            .flat_map(|s| {
                s.object_lists.values_mut().flatten().chain(
                    s.mwnd_lists.values_mut().flatten().flat_map(|w| {
                        w.object_list
                            .iter_mut()
                            .chain(&mut w.face_list)
                            .chain(&mut w.button_list)
                    }),
                )
            })
            .collect::<Vec<_>>();
        let mut visited = 0;
        while let Some(object) = todo.pop() {
            visited += 1;
            ensure!(visited <= 4096, "native STRING object traversal budget");
            if object.object_type == 3 {
                if object.runtime.native_text.is_none() {
                    *revision = revision
                        .checked_add(1)
                        .context("native STRING load generation overflow")?;
                    object.runtime.native_text = Some(native_string::Definition {
                        generation: *revision,
                        capture: environment.capture.clone(),
                    });
                }
                let generation = object.runtime.native_text.as_ref().unwrap().generation;
                if self.object_strings.frame(generation).is_some() {
                    object.runtime.native_text_size =
                        self.state.io.text_sizes.get(&generation).copied();
                } else {
                    let request = environment.request(object)?;
                    requests.insert(request.definition.generation, request);
                }
            }
            todo.extend(&mut object.runtime.child_objects);
        }
        self.object_strings
            .prepare(ClientId(self.state.client), requests, host)?;
        self.object_strings
            .poll(ClientId(self.state.client), &mut self.state.io.text_sizes)
    }
}
