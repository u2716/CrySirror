// SPDX-License-Identifier: MPL-2.0
//! Content-recognized CPU image plugins, with scoped dependency reads.
use crate::{DecoderRegistry, ImageSurface, MAX_DECODED_BYTES};
use anyhow::{bail, ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    Error,
};
use crysirror_resource::{ArchiveEntry, DecodeContext};
use std::{cell::Cell, sync::Arc};

/// The host binds this callback to one client's capability storage.
pub type ImageSource<'a> = dyn Fn(&str) -> crysirror_core::Result<Vec<u8>> + 'a;
pub trait ImageFormat: Send + Sync {
    fn id(&self) -> &'static str;
    /// A specific native wrapper outranks its generic embedded image codec.
    fn priority(&self) -> u8 {
        50
    }
    fn priority_for(&self, _path: &str, _bytes: &[u8]) -> u8 {
        self.priority()
    }
    fn recognize(&self, path: &str, bytes: &[u8]) -> bool;
    fn decode(&self, bytes: &[u8], context: &ImageDecodeContext<'_>) -> Result<ImageSurface>;
    /// Header/range properties of the encoded source. A codec that cannot
    /// answer returns None; normalized RGBA must not stand in for this data.
    fn encoding_info(
        &self,
        _bytes: &[u8],
        _context: &ImageDecodeContext<'_>,
    ) -> Result<Option<crysirror_core::image_encoding::ImageEncodingInfo>> {
        Ok(None)
    }
}
struct Work {
    encoded: Cell<usize>,
    decoded: Cell<usize>,
}
pub struct ImageDecodeContext<'a> {
    pub path: &'a str,
    registry: &'a DecoderRegistry,
    source: Option<&'a ImageSource<'a>>,
    resource: Option<&'a DecodeContext>,
    depth: u8,
    work: Option<&'a Work>,
}
impl<'a> ImageDecodeContext<'a> {
    /// Observe the current resource worker's cancellation without copying input.
    /// Standalone decoding without a resource job has no cancellation token.
    pub fn check_cancelled(&self) -> Result<()> {
        if let Some(resource) = self.resource {
            resource.check_cancelled()?;
        }
        Ok(())
    }

    pub fn new(path: &'a str, registry: &'a DecoderRegistry) -> Self {
        Self {
            path,
            registry,
            source: None,
            resource: None,
            depth: 0,
            work: None,
        }
    }
    pub fn with_source(mut self, source: &'a ImageSource<'a>) -> Self {
        self.source = Some(source);
        self
    }
    pub fn with_resources(mut self, resources: &'a DecodeContext) -> Self {
        self.resource = Some(resources);
        self
    }
    pub fn key(&self, id: &str) -> Result<crysirror_resource::KeyMaterial> {
        let resources = self
            .resource
            .context("encrypted image needs a client-scoped key provider")?;
        resources.keys.key(resources.client, id)
    }
    fn relative(&self, name: &str) -> Result<String> {
        let native = name.replace('\\', "/");
        VfsPath::new(Mount::Asset, &native)
            .map_err(|e| anyhow::anyhow!("image dependency path: {e}"))?;
        let prefix = self.path.rsplit_once('/').map_or("", |(prefix, _)| prefix);
        let full = if prefix.is_empty() {
            native
        } else {
            format!("{prefix}/{native}")
        };
        Ok(VfsPath::new(Mount::Asset, &full)
            .map_err(|e| anyhow::anyhow!("image dependency path: {e}"))?
            .relative()
            .into())
    }
    pub fn try_read_relative(&self, name: &str) -> Result<Option<Vec<u8>>> {
        let path = self.relative(name)?;
        let source = self
            .source
            .context("image needs a client-scoped dependency resolver")?;
        match source(&path) {
            Ok(bytes) => {
                ensure!(
                    bytes.len() <= MAX_DECODED_BYTES,
                    "image dependency byte limit"
                );
                Ok(Some(bytes))
            }
            Err(Error::NotFound) => Ok(None),
            Err(e) => bail!("image dependency {path}: {e}"),
        }
    }
    pub fn read_relative(&self, name: &str) -> Result<Vec<u8>> {
        self.try_read_relative(name)?
            .context("image dependency not found")
    }
    pub fn decode_relative(&self, name: &str) -> Result<ImageSurface> {
        let path = self.relative(name)?;
        let bytes = self.read_relative(name)?;
        self.child(&path, &bytes)
    }
    pub fn decode_embedded(&self, path_hint: &str, bytes: &[u8]) -> Result<ImageSurface> {
        let path = self.relative(path_hint)?;
        self.child(&path, bytes)
    }
    fn child(&self, path: &str, bytes: &[u8]) -> Result<ImageSurface> {
        ensure!(self.depth < 16, "nested image decode depth limit");
        let context = ImageDecodeContext {
            path,
            registry: self.registry,
            source: self.source,
            resource: self.resource,
            depth: self.depth + 1,
            work: self.work,
        };
        self.registry.decode_context(bytes, None, &context)
    }
    /// Resolve an explicit format/path transform profile through the common
    /// resource service. A cipher name alone does not implement a native cipher.
    pub fn transform(
        &self,
        format: &str,
        bytes: &[u8],
        metadata: &serde_json::Value,
        required: bool,
    ) -> Result<Vec<u8>> {
        let Some(resource) = self.resource else {
            ensure!(
                !required,
                "encrypted image needs an explicit transform/key profile"
            );
            return Ok(bytes.to_vec());
        };
        let entry = ArchiveEntry {
            id: 0,
            path: self.path.into(),
            size: bytes.len() as u64,
            stored_size: bytes.len() as u64,
            metadata: metadata.clone(),
        };
        let source = resource.apply_pipeline(
            format,
            &entry,
            crysirror_resource::io::memory(bytes.to_vec()),
            required,
        )?;
        crysirror_resource::read_all(source.as_ref(), MAX_DECODED_BYTES as u64)
    }
}
impl DecoderRegistry {
    /// Encoded source metadata, selected by exactly the same native-provider
    /// priorities and ambiguity rules as pixel decoding. Unknown fields remain
    /// None and no standard-codec fallback fabricates original plane presence.
    pub fn encoding_info_with(
        &self,
        path: &str,
        bytes: &[u8],
        source: Option<&ImageSource<'_>>,
        resource: Option<&DecodeContext>,
        format: Option<&str>,
    ) -> Result<Option<crysirror_core::image_encoding::ImageEncodingInfo>> {
        ensure!(bytes.len() <= MAX_DECODED_BYTES, "encoded image byte limit");
        let context = ImageDecodeContext {
            path,
            registry: self,
            source,
            resource,
            depth: 0,
            work: None,
        };
        context.check_cancelled()?;
        let Some(decoder) = self.select_format(path, bytes, format)? else {
            return Ok(None);
        };
        let result = decoder.encoding_info(bytes, &context)?;
        if let Some(info) = &result {
            ensure!(
                info.format == decoder.id(),
                "encoded image metadata provider identity"
            );
            crate::rgba_len(info.size[0], info.size[1])?;
        }
        context.check_cancelled()?;
        Ok(result)
    }
    fn select_format(
        &self,
        path: &str,
        bytes: &[u8],
        explicit: Option<&str>,
    ) -> Result<Option<&Arc<dyn ImageFormat>>> {
        let decoder = if let Some(id) = explicit {
            Some(
                self.formats
                    .get(id)
                    .context("image format not registered")?,
            )
        } else {
            let mut matches = self.probe(path, bytes)?;
            if let Some(priority) = matches
                .iter()
                .map(|id| self.formats[id].priority_for(path, bytes))
                .max()
            {
                matches.retain(|id| self.formats[id].priority_for(path, bytes) == priority);
            }
            ensure!(
                matches.len() <= 1,
                "ambiguous image formats {matches:?}; specify a format ID"
            );
            matches.first().map(|id| &self.formats[id])
        };
        if let Some(decoder) = decoder {
            ensure!(
                decoder.recognize(path, bytes),
                "image does not match selected format {}",
                decoder.id()
            );
        }
        Ok(decoder)
    }
    pub fn register_format(&mut self, format: Arc<dyn ImageFormat>) -> Result<()> {
        let id = format.id();
        ensure!(
            !id.is_empty() && id.len() <= 128 && !self.formats.contains_key(id),
            "invalid or duplicate image format ID"
        );
        self.formats.insert(id, format);
        Ok(())
    }
    pub fn format_ids(&self) -> Vec<&'static str> {
        self.formats.keys().copied().collect()
    }
    pub fn extensions(&self) -> Vec<&str> {
        self.decoders.keys().map(String::as_str).collect()
    }
    pub fn probe(&self, path: &str, bytes: &[u8]) -> Result<Vec<&'static str>> {
        ensure!(bytes.len() <= MAX_DECODED_BYTES, "encoded image byte limit");
        Ok(self
            .formats
            .iter()
            .filter_map(|(&id, decoder)| decoder.recognize(path, bytes).then_some(id))
            .collect())
    }
    pub fn decode_with(
        &self,
        path: &str,
        bytes: &[u8],
        source: Option<&ImageSource<'_>>,
        resource: Option<&DecodeContext>,
        format: Option<&str>,
    ) -> Result<ImageSurface> {
        let work = Work {
            encoded: Cell::new(0),
            decoded: Cell::new(0),
        };
        let context = ImageDecodeContext {
            path,
            registry: self,
            source,
            resource,
            depth: 0,
            work: Some(&work),
        };
        self.decode_context(bytes, format, &context)
    }
    fn decode_context(
        &self,
        bytes: &[u8],
        format: Option<&str>,
        context: &ImageDecodeContext<'_>,
    ) -> Result<ImageSurface> {
        ensure!(bytes.len() <= MAX_DECODED_BYTES, "encoded image byte limit");
        if let Some(work) = context.work {
            let n = work
                .encoded
                .get()
                .checked_add(bytes.len())
                .context("image dependency byte overflow")?;
            ensure!(
                n <= 2 * MAX_DECODED_BYTES,
                "cumulative image dependency byte limit"
            );
            work.encoded.set(n);
        }
        let decoder = self.select_format(context.path, bytes, format)?;
        let surface = if let Some(decoder) = decoder {
            decoder
                .decode(bytes, context)
                .with_context(|| format!("decode {} as {}", context.path, decoder.id()))?
        } else {
            let extension = context
                .path
                .rsplit_once('.')
                .map(|(_, e)| e.to_ascii_lowercase())
                .unwrap_or_default();
            // Native memory resources need not carry a filename extension.
            // Standard codec magic is pure Rust and bounded by the encoded
            // byte limit above; proprietary format probes retain priority.
            let guessed = image::guess_format(bytes).ok();
            let content_decoder = guessed.as_ref().and_then(|format| {
                format
                    .extensions_str()
                    .iter()
                    .find_map(|extension| self.decoders.get(*extension))
            });
            content_decoder
                .or_else(|| self.decoders.get(&extension))
                .with_context(|| format!("unsupported image format: {extension}"))?
                .decode(bytes)
                .with_context(|| format!("decode {}", context.path))?
        };
        surface.validate()?;
        if let Some(work) = context.work {
            let n = work
                .decoded
                .get()
                .checked_add(surface.rgba.len())
                .context("decoded image work overflow")?;
            ensure!(
                n <= 2 * MAX_DECODED_BYTES,
                "cumulative decoded image byte limit"
            );
            work.decoded.set(n);
        }
        Ok(surface)
    }
}

#[cfg(test)]
mod memory_codec_tests {
    use super::*;
    #[test]
    fn standard_image_magic_decodes_extensionless_and_misnamed_resources() {
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 1, image::Rgba([17, 31, 99, 255]))
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let registry = DecoderRegistry::default();
        for name in ["memory-octet", "wrong.jpg"] {
            let surface = registry
                .decode_with(name, encoded.get_ref(), None, None, None)
                .unwrap();
            assert_eq!([surface.width, surface.height], [2, 1]);
            assert_eq!(&surface.rgba[..4], &[17, 31, 99, 255]);
        }
        assert!(registry
            .decode_with("memory-octet", b"not an image", None, None, None)
            .is_err());
    }
    struct HeaderPlugin {
        id: &'static str,
        priority: u8,
    }
    impl ImageFormat for HeaderPlugin {
        fn id(&self) -> &'static str {
            self.id
        }
        fn priority_for(&self, _: &str, _: &[u8]) -> u8 {
            self.priority
        }
        fn recognize(&self, _: &str, bytes: &[u8]) -> bool {
            bytes == b"shared header"
        }
        fn decode(&self, _: &[u8], _: &ImageDecodeContext<'_>) -> Result<ImageSurface> {
            Ok(ImageSurface {
                width: 1,
                height: 1,
                rgba: vec![self.priority, 0, 0, 255],
            })
        }
        fn encoding_info(
            &self,
            _: &[u8],
            _: &ImageDecodeContext<'_>,
        ) -> Result<Option<crysirror_core::image_encoding::ImageEncodingInfo>> {
            Ok(Some(crysirror_core::image_encoding::ImageEncodingInfo {
                format: self.id.into(),
                size: [1, 1],
                source_origin: None,
                color_bits: None,
                has_color_plane: None,
                has_alpha_plane: None,
                color_row_bytes: None,
                alpha_sample_bytes: None,
            }))
        }
    }
    #[test]
    fn encoded_metadata_and_pixels_share_provider_priority_and_ambiguity_policy() {
        let mut registry = DecoderRegistry::default();
        registry
            .register_format(Arc::new(HeaderPlugin {
                id: "test/low",
                priority: 1,
            }))
            .unwrap();
        registry
            .register_format(Arc::new(HeaderPlugin {
                id: "test/high",
                priority: 2,
            }))
            .unwrap();
        let info = registry
            .encoding_info_with("x", b"shared header", None, None, None)
            .unwrap()
            .unwrap();
        assert_eq!(info.format, "test/high");
        assert_eq!(info.has_alpha_plane, None);
        assert_eq!(
            registry
                .decode_with("x", b"shared header", None, None, None)
                .unwrap()
                .rgba[0],
            2
        );
        assert!(registry
            .encoding_info_with("x", b"mismatch", None, None, Some("test/high"))
            .is_err());
        registry
            .register_format(Arc::new(HeaderPlugin {
                id: "test/tied",
                priority: 2,
            }))
            .unwrap();
        assert!(registry
            .encoding_info_with("x", b"shared header", None, None, None)
            .is_err());
        assert!(registry
            .decode_with("x", b"shared header", None, None, None)
            .is_err());
        assert!(registry
            .encoding_info_with("x", b"unrecognized", None, None, None)
            .unwrap()
            .is_none());
    }
}
