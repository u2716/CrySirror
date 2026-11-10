// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Reviewed native resource precedence over shared archive/file decoders.
//! All routines run on admitted workers; no VM parser/codec or ambient path.
use crate::{
    native_audio::{NativeAudioLookup, NativeSource},
    native_resources::{ImageLookup, PackedMember, ResourceNames},
};
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    image_processing::ImageInput,
    service::{AssetReader, EncodedAudio},
    state::ClientId,
};
use crysirror_resource::{DecodeContext, FileRegistry, FormatRegistry, read_all};
use crysirror_tasks::Cancellation;
use std::sync::Arc;
pub type SourceOpener =
    Arc<dyn Fn(&str, &Cancellation) -> Result<Option<Arc<dyn AssetReader>>> + Send + Sync>;
#[derive(Clone, Debug)]
pub enum ResolvedAudio {
    Asset(crysirror_core::path::VfsPath),
    Encoded(EncodedAudio),
    Playlist(crysirror_core::service::AudioPlaylist),
}
#[derive(Clone, Debug)]
pub struct NativeImageSource {
    pub input: ImageInput,
    pub has_mask: bool,
    pub size: [u32; 2],
    pub encoded_len: usize,
    pub clip_insets: Option<[i32; 4]>,
}
#[derive(Clone)]
pub struct NativeResources {
    client: ClientId,
    open: SourceOpener,
    archives: Arc<FormatRegistry>,
    files: Arc<FileRegistry>,
}
impl NativeResources {
    pub(crate) fn client(&self) -> ClientId {
        self.client
    }
    pub fn blackcyc(client: ClientId, open: SourceOpener) -> Result<Self> {
        ensure!(client.0 != 0, "NNN resource client identity");
        let mut archives = FormatRegistry::default();
        crysirror_formats_blackcyc::register_archives(&mut archives)?;
        let mut files = FileRegistry::default();
        crysirror_formats_blackcyc::register_audio(&mut files)?;
        Ok(Self {
            client,
            open,
            archives: Arc::new(archives),
            files: Arc::new(files),
        })
    }
    fn context(&self, path: &str, cancel: &Cancellation) -> DecodeContext {
        let mut cx = DecodeContext::new(self.client).with_cancellation(cancel.clone());
        cx.source_name = Some(path.to_owned());
        let open = self.open.clone();
        let token = cancel.clone();
        cx.resolver = Some(Arc::new(move |path: &str| {
            token.check()?;
            open(path, &token)?.with_context(|| format!("NNN companion absent: {path}"))
        }));
        cx
    }
    fn packed(
        &self,
        member: &PackedMember,
        kind: &str,
        cancel: &Cancellation,
    ) -> Result<Option<Arc<dyn AssetReader>>> {
        cancel.check()?;
        let Some(body) = (self.open)(&member.archive, cancel)? else {
            return Ok(None);
        };
        let cx = self.context(&member.archive, cancel);
        let (_, archive) = self.archives.open(body, Some(kind), &cx)?;
        let Some(entry) = archive
            .entries()
            .iter()
            .find(|entry| entry.path.eq_ignore_ascii_case(&member.member))
        else {
            return Ok(None);
        };
        Ok(Some(archive.open(entry.id, &cx)?))
    }
    pub fn image(&self, lookup: &ImageLookup, cancel: &Cancellation) -> Result<ImageInput> {
        Ok(self.image_source(lookup, cancel)?.input)
    }
    pub fn image_source(
        &self,
        lookup: &ImageLookup,
        cancel: &Cancellation,
    ) -> Result<NativeImageSource> {
        self.optional_image_source(lookup, cancel)?
            .with_context(|| format!("NNN image resource absent: {}", lookup.loose))
    }
    pub(crate) fn optional_image_source(
        &self,
        lookup: &ImageLookup,
        cancel: &Cancellation,
    ) -> Result<Option<NativeImageSource>> {
        cancel.check()?;
        let mut selected = None;
        for member in &lookup.packed {
            let Some(table) = (self.open)(&member.table, cancel)? else {
                continue;
            };
            let cx = self.context(&member.archive, cancel);
            let table = read_all(table.as_ref(), cx.limits.max_index_size)
                .with_context(|| format!("NNN image directory read {}", member.table))?;
            let directory =
                crysirror_formats_blackcyc::GpkIndex::parse(&table, &cx).with_context(|| {
                    format!("NNN image directory {} bytes={}", member.table, table.len())
                })?;
            if directory.find(&member.member).is_none() {
                continue;
            }
            // Original highest matching directory commits precedence. If its
            // GPK is absent, fall back to loose, never search an older pack.
            selected = self
                .packed(member, "black-cyc/gpk", cancel)?
                .map(|reader| (member.member.clone(), reader));
            break;
        }
        let (name, reader) = match selected {
            Some(found) => found,
            None => {
                let Some(reader) = (self.open)(&lookup.loose, cancel)? else {
                    return Ok(None);
                };
                (lookup.loose.clone(), reader)
            }
        };
        let bytes = read_all(reader.as_ref(), 64 * 1024 * 1024)?;
        cancel.check()?;
        let info = crysirror_formats_blackcyc::Dwq::metadata(&bytes)?;
        let has_mask = matches!(info.pack, 2 | 3 | 7)
            || (info.pack == 8 && matches!(info.png_color_type, Some(4 | 6)));
        let encoded_len = bytes.len();
        let mut input = ImageInput::Encoded {
            name,
            bytes: bytes.into(),
            size: info.size,
        };
        if info.pack == 8 && !has_mask && info.png_transparency {
            // Native pngLoader only enables a mask for GA/RGBA. Its tRNS
            // condition cannot enable palette/RGB/grey transparency. Keep
            // the generic PNG decoder correct and express that native alpha
            // policy through the shared immutable image operation.
            input = ImageInput::Process {
                input: Box::new(input),
                effects: serde_json::to_vec(&[
                    crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 }
                ])?.into(),
            };
        }
        Ok(Some(NativeImageSource {
            has_mask,
            size: info.size,
            encoded_len,
            clip_insets: info.clip_insets,
            input,
        }))
    }

    pub fn audio(
        &self,
        source: &NativeSource,
        names: &ResourceNames,
        cancel: &Cancellation,
    ) -> Result<ResolvedAudio> {
        cancel.check()?;
        let (path, reader) = match source.lookup(names)? {
            NativeAudioLookup::Music(lookup) => {
                if let Some(control) = (self.open)(&lookup.control, cancel)? {
                    let mml = crate::native_mml::Mml::parse(&read_all(control.as_ref(), 65536)?)?;
                    let mut clips = Vec::new();
                    let mut total = 0usize;
                    for name in mml.names {
                        cancel.check()?;
                        let path = names.music(&name)?;
                        let reader = (self.open)(&path, cancel)?
                            .with_context(|| format!("NNN MML segment missing: {path}"))?;
                        let source = self.decode_audio(&path, reader, cancel)?;
                        total = total
                            .checked_add(source.bytes.len())
                            .context("NNN MML bytes overflow")?;
                        ensure!(
                            total <= 256 * 1024 * 1024,
                            "NNN MML encoded resource budget"
                        );
                        clips.push(source);
                    }
                    let source = crysirror_core::service::AudioPlaylist {
                        clips,
                        program: mml.program,
                    };
                    source.encoded_bytes()?;
                    return Ok(ResolvedAudio::Playlist(source));
                }
                let reader = (self.open)(&lookup.audio, cancel)?
                    .with_context(|| format!("NNN music resource absent: {}", lookup.audio))?;
                (lookup.audio, reader)
            }
            NativeAudioLookup::Sound { loose } => {
                let reader = (self.open)(&loose, cancel)?
                    .with_context(|| format!("NNN sound resource absent: {loose}"))?;
                (loose, reader)
            }
            NativeAudioLookup::Voice { packed, loose } => {
                if let Some(reader) = self.packed(&packed, "black-cyc/vpk", cancel)? {
                    (packed.member, reader)
                } else {
                    let reader = (self.open)(&loose, cancel)?
                        .with_context(|| format!("NNN voice resource absent: {loose}"))?;
                    (loose, reader)
                }
            }
        };
        Ok(ResolvedAudio::Encoded(
            self.decode_audio(&path, reader, cancel)?,
        ))
    }
    fn decode_audio(
        &self,
        path: &str,
        reader: Arc<dyn AssetReader>,
        cancel: &Cancellation,
    ) -> Result<EncodedAudio> {
        let cx = self.context(path, cancel);
        let (_, decoded) = self
            .files
            .decode(&path, reader, Some("black-cyc/vaw"), &cx)?;
        ensure!(
            matches!(decoded.extension.as_str(), "wav" | "ogg"),
            "NNN shared audio wrapper output profile"
        );
        let bytes = read_all(decoded.reader.as_ref(), 64 * 1024 * 1024)?;
        cancel.check()?;
        Ok(EncodedAudio {
            bytes: bytes.into(),
            frame_limit: None,
        })
    }
}
