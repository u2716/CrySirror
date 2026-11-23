//! Public Text facade for client-scoped asynchronous font registrations.
use crate::{EngineState, Value};
use anyhow::{Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        FontHandle, FontInfo, FontJob, FontLoadSource, FontLoadSpec, FontLoadState, HostServices,
    },
};
use serde_json::json;
fn info(f: FontInfo) -> serde_json::Value {
    json!({"handle":f.handle.id,"owner":f.handle.owner.0,"logical_alias":f.logical_alias,"alias":f.alias,"digest":f.digest,"revision":f.revision,"families":f.families,"face_index":f.face_index,"collection_faces":f.collection_faces,"byte_len":f.byte_len})
}
pub(crate) fn call(state: &mut EngineState, method: &str, args: &[Value]) -> Result<Option<Value>> {
    if !matches!(
        method,
        "beginFont"
            | "beginPrecomputedFont"
            | "fontStatus"
            | "resolveFont"
            | "resolveFontAlias"
            | "fontMetrics" | "glyphId" | "fontStyle"
            | "releaseFontJob"
            | "releaseFont"
    ) {
        return Ok(None);
    }
    let owner = state.client;
    let library = state.runtime.host.font_library(owner)?;
    let arg = args.first().context("font argument")?;
    let result = match method {
        "beginFont" => {
            let name = arg.as_str()?.to_owned();
            let path = args.get(1).context("font asset path")?.as_str()?;
            let index = args.get(2).map(|a| a.as_i64()).transpose()?.unwrap_or(0);
            let ticket = library.begin(FontLoadSpec {
                name,
                source: FontLoadSource::Storage(VfsPath::new(Mount::Asset, path)?),
                face_index: index.try_into()?,
            })?;
            Value::Int(ticket.id.try_into()?)
        }
        "beginPrecomputedFont" => {
            let name = arg.as_str()?.to_owned();
            let path = args
                .get(1)
                .context("precomputed font asset path")?
                .as_str()?;
            let pixel_size: u16 = args
                .get(2)
                .context("precomputed font pixel size")?
                .as_i64()?
                .try_into()?;
            anyhow::ensure!(
                (1..=512).contains(&pixel_size),
                "precomputed font pixel size must be 1..512"
            );
            let ticket = library.begin(FontLoadSpec {
                name,
                source: FontLoadSource::PrecomputedStorage {
                    path: VfsPath::new(Mount::Asset, path)?,
                    pixel_size,
                },
                face_index: 0,
            })?;
            Value::Int(ticket.id.try_into()?)
        }
        "fontStatus" => {
            let job = FontJob {
                owner,
                id: arg.as_i64()?.try_into()?,
            };
            Value::from_json(match library.status(job)? {
                FontLoadState::Pending => json!({"state":"pending"}),
                FontLoadState::Failed(error) => json!({"state":"failed","error":error}),
                FontLoadState::Ready(font) => json!({"state":"ready","font":info(font)}),
            })
        }
        "resolveFont" => Value::from_json(
            library
                .resolve(arg.as_str()?)?
                .map(info)
                .unwrap_or(serde_json::Value::Null),
        ),
        "resolveFontAlias" => library
            .resolve_alias(arg.as_str()?)?
            .map(Value::Str)
            .unwrap_or(Value::Null),
        "glyphId" => {
            let text=args.get(1).context("glyph character")?.as_str()?;let mut chars=text.chars();let ch=chars.next().context("empty glyph character")?;
            anyhow::ensure!(chars.next().is_none(),"glyph identifier requires one scalar");
            Value::Int(i64::from(library.glyph_id(arg.as_str()?,ch)?))
        }
        "fontStyle" => Value::from_json(library.face_style(arg.as_str()?)?.map(|s|json!({"weight":s.weight,"italic":s.italic})).unwrap_or(serde_json::Value::Null)),
        "fontMetrics" => {
            let alias = arg.as_str()?;
            let size = args.get(1).context("font metric size")?.as_f64()? as f32;
            let text = args.get(2).context("font metric character")?.as_str()?;
            let mut characters = text.chars();
            let character = characters
                .next()
                .context("font metric character is empty")?;
            anyhow::ensure!(
                characters.next().is_none(),
                "font metrics require one Unicode scalar"
            );
            let metrics = library.metrics(alias, size, character)?;
            Value::from_json(
                json!({"ascent":metrics.ascent,"descent":metrics.descent,"advance":metrics.advance}),
            )
        }
        "releaseFontJob" => {
            library.release_job(FontJob {
                owner,
                id: arg.as_i64()?.try_into()?,
            })?;
            Value::Null
        }
        "releaseFont" => {
            library.release_font(FontHandle {
                owner,
                id: arg.as_i64()?.try_into()?,
            })?;
            Value::Null
        }
        _ => unreachable!(),
    };
    Ok(Some(result))
}
