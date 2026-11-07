// Copyright (c) 2026 CrySirror contributors. MIT.
//! 412E10 selects a logical font and resets its derived4x glyph cache. The
//! native .fcd cache is derived raster data, not a VM/game save. CrySirror's
//! font service owns face/glyph resources; no GDI or private font cache runs.
use super::*;
pub(super) struct NativeFont {
    pub requested: String,
    pub alias: String,
    pub height: i32,
    pub half_width: i32,
}
impl MajiroVm {
    pub(super) fn resolve_font_alias(
        &self,
        requested: &str,
        host: &mut dyn HostServices,
    ) -> Result<String> {
        let library = host.font_library(self.client).map_err(core_error)?;
        let alias = if let Some(alias) = library.resolve_alias(requested).map_err(core_error)? {
            alias
        } else if let Some(fallback) = &self.config.font_fallback {
            library
                .resolve_alias(fallback)
                .map_err(core_error)?
                .context("Majiro configured font fallback is unavailable")?
        } else {
            bail!(
                "Majiro font family {requested:?} is unavailable and no host fallback is configured"
            )
        };
        ensure!(
            library.is_loaded(&alias).map_err(core_error)?,
            "Majiro font selection has no prepared engine face"
        );
        Ok(alias)
    }
    pub(super) fn select_font(
        &mut self,
        args: &[Value],
        returns: bool,
        host: &mut dyn HostServices,
    ) -> Result<Option<Vec<Value>>> {
        ensure!(args.len() == 3, "Majiro font selection ABI");
        let bytes = args[0].bytes()?;
        let bytes = &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())];
        ensure!(bytes.len() <= 255, "Majiro native font name budget");
        let height = args[1].integer()?;
        let half_width = args[2].integer()?;
        let requested = encoding_rs::SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(bytes)
            .context("Majiro native font name CP932")?
            .into_owned();
        if requested.is_empty() {
            //412E10 clears the derived cache for an empty name even when
            //the supplied dimensions are zero; no font face is requested.
            self.font = None;
        } else {
            ensure!(
                (1..=512).contains(&height) && (1..=512).contains(&half_width),
                "Majiro native font cell geometry"
            );
            let alias = self.resolve_font_alias(&requested, host)?;
            self.font = Some(NativeFont {
                requested,
                alias,
                height,
                half_width,
            });
            self.api_calls += 1;
        }
        Ok(Some(if returns {
            vec![Value::Integer(0)]
        } else {
            vec![]
        }))
    }
}
