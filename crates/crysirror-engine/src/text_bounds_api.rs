//! Opaque, client-scoped glyph metadata worker tickets for the public Text API.
use crate::{EngineState, Value};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    service::{
        GlyphBoundsProgress, GlyphBoundsQuery, GlyphBoundsRequest, GlyphBoundsSpec, HostServices,
    },
    state::ClientId,
};
use std::collections::BTreeMap;
#[derive(Default)]
pub(crate) struct Jobs {
    next: u64,
    jobs: BTreeMap<u64, Job>,
}
struct Job {
    owner: ClientId,
    font: String,
    query: Box<dyn GlyphBoundsQuery>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    character: char,
    size: f32,
    #[serde(default)]
    baseline: [f32; 2],
}
pub(crate) fn call(state: &mut EngineState, method: &str, args: &[Value]) -> Result<Option<Value>> {
    if !matches!(
        method,
        "beginGlyphBounds" | "glyphBoundsStatus" | "releaseGlyphBounds"
    ) {
        return Ok(None);
    }
    let owner = state.client;
    if method == "beginGlyphBounds" {
        ensure!(args.len() == 2, "glyph bounds argument count");
        let font = args[0].as_str()?;
        let Value::List(requests) = &args[1] else {
            anyhow::bail!("glyph bounds requests must be a list");
        };
        ensure!(requests.len() <= 65536, "glyph bounds request count");
        let jobs = &state.rich_text.bounds;
        ensure!(
            jobs.jobs.len() < 64 && jobs.jobs.values().filter(|j| j.owner == owner).count() < 16,
            "glyph bounds ticket capacity"
        );
        let ticket = jobs
            .next
            .checked_add(1)
            .context("glyph bounds ticket overflow")?;
        ensure!(
            ticket <= i64::MAX as u64,
            "glyph bounds public ticket range"
        );
        let requests = serde_json::from_value::<Vec<Request>>(args[1].to_json())?;
        let fonts = state.runtime.host.font_library(owner)?;
        let font = fonts
            .resolve_alias(font)?
            .context("glyph bounds font is not installed for this client")?;
        let glyphs = requests
            .into_iter()
            .map(|r| GlyphBoundsRequest {
                character: r.character,
                size: r.size,
                baseline: r.baseline,
            })
            .collect();
        let query = fonts.begin_glyph_bounds(GlyphBoundsSpec {
            font: font.clone(),
            glyphs,
        })?;
        state
            .rich_text
            .bounds
            .jobs
            .insert(ticket, Job { owner, font, query });
        state.rich_text.bounds.next = ticket;
        return Ok(Some(Value::Int(ticket as i64)));
    }
    ensure!(args.len() == 1, "glyph bounds ticket argument count");
    let ticket: u64 = args[0].as_i64()?.try_into()?;
    let job = state
        .rich_text
        .bounds
        .jobs
        .get_mut(&ticket)
        .filter(|job| job.owner == owner)
        .context("glyph bounds ticket not owned by this client")?;
    if method == "releaseGlyphBounds" {
        job.query.cancel()?;
        state.rich_text.bounds.jobs.remove(&ticket);
        return Ok(Some(Value::Null));
    }
    let status = match job.query.poll()? {
        GlyphBoundsProgress::Pending => serde_json::json!({"state":"pending"}),
        GlyphBoundsProgress::Cancelled => serde_json::json!({"state":"cancelled"}),
        GlyphBoundsProgress::Failed(error) => {
            serde_json::json!({"state":"failed","error":error.to_string()})
        }
        GlyphBoundsProgress::Ready(resource) => {
            ensure!(
                resource.client() == owner && resource.font() == job.font,
                "glyph bounds worker returned foreign resource"
            );
            serde_json::json!({"state":"ready","font":resource.font(),"digest":resource.digest(),"glyphs":resource.glyphs().iter().map(|g|serde_json::json!({"glyph_id":g.glyph_id,"origin":g.origin,"size":g.size})).collect::<Vec<_>>()})
        }
    };
    Ok(Some(Value::from_json(status)))
}
