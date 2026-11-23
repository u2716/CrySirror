//! Native font numbers and text operands; engine jobs own fonts and rasterization.
use super::*;
use crysirror_core::{
    image_processing::*,
    service::{FontJob, FontLoadSource, FontLoadSpec, FontLoadState},
};
use crysirror_text::{
    bitmap_layout::{LayoutParams, layout_text},
    message::{parse_message_markup_with, ruby_registry::RubyRegistry},
};
#[derive(Clone)]
struct NativeFont {
    id: i32,
    option: i32,
    charset: Option<u8>,
}
pub(crate) struct TextState {
    fonts: BTreeMap<String, NativeFont>,
    aliases: BTreeMap<String, String>,
    next_font: i32,
    resources: BTreeMap<String, String>,
    pub ruby: RubyRegistry,
    pub layout: crate::text_policy::TextLayoutDefaults,
    pub style: crate::text_policy::TextStyleDefaults,
    pub spacing: i32,
    pub indent: bool,
    pub override_color: i32,
    pub output_pair: [i32; 2],
    pub output_color: i32,
    pub fragments: Vec<crysirror_bgi_vm::System92TextFragmentRecord>,
}
impl Default for TextState {
    fn default() -> Self {
        Self {
            fonts: [
                (
                    "ＭＳ ゴシック".into(),
                    NativeFont {
                        id: 0,
                        option: 0,
                        charset: Some(128),
                    },
                ),
                (
                    "ＭＳ 明朝".into(),
                    NativeFont {
                        id: 1,
                        option: 0,
                        charset: Some(128),
                    },
                ),
            ]
            .into(),
            aliases: BTreeMap::new(),
            next_font: 2,
            resources: BTreeMap::new(),
            ruby: Default::default(),
            layout: Default::default(),
            style: Default::default(),
            spacing: 0,
            indent: false,
            override_color: -1,
            output_pair: [0; 2],
            output_color: 0,
            fragments: Vec::new(),
        }
    }
}
pub(crate) enum TextWork {
    FontRead {
        key: String,
        name: String,
        task: Task<Vec<u8>>,
    },
    FontLoad {
        key: String,
        job: FontJob,
    },
    Capture {
        bitmap: u32,
        format: i32,
        aux: [i32; 2],
        spec: BitmapTextSpec,
        output: i32,
        job: ImageJob,
    },
    Draw {
        bitmap: u32,
        format: i32,
        aux: [i32; 2],
        output: i32,
        job: ImageJob,
    },
    Display {
        bitmap: u32,
        format: i32,
        aux: [i32; 2],
        output: i32,
        raw: Arc<dyn ImageResource>,
        job: ImageJob,
    },
}
pub(crate) fn handles(group: u8, id: u16) -> bool {
    matches!(
        (group, id),
        (0xb0, 0xc0..=0xc3 | 0xc6 | 0xc7)
            | (0x92, 0x9b | 0x9c | 0x9e)
            | (0x91, 0x94..=0x99 | 0x9c | 0x9d)
    )
}
fn rgb(packed: u32) -> [u8; 3] {
    [(packed >> 16) as u8, (packed >> 8) as u8, packed as u8]
}
impl BgiVm {
    pub(crate) fn named_font_alias(
        &self,
        host: &mut dyn HostServices,
        name: &str,
    ) -> Result<String> {
        if let Some(font) = self.text.fonts.get(name) {
            return self.font_alias(host, font.id);
        }
        let resolved = self
            .text
            .aliases
            .get(name)
            .map(String::as_str)
            .unwrap_or(name);
        host.font_library(self.client)?
            .resolve_alias(resolved)?
            .with_context(|| format!("BGI font face {name:?} has no engine font binding"))
    }
    pub(crate) fn native_font_known(&self, id: i32) -> bool {
        self.text.fonts.values().any(|font| font.id == id)
    }
    pub(crate) fn font_alias(&self, host: &mut dyn HostServices, id: i32) -> Result<String> {
        let name = self
            .text
            .fonts
            .iter()
            .find_map(|(name, font)| (font.id == id).then_some(name.as_str()))
            .context("BGI unregistered font number")?;
        let name = self
            .text
            .aliases
            .get(name)
            .map(String::as_str)
            .unwrap_or(name);
        let library = host.font_library(self.client)?;
        if let Some(alias) = library.resolve_alias(name)? {
            return Ok(alias);
        }
        if matches!(id, 0 | 1) {
            if let Some(alias) = self.config.font.as_deref() {
                return library
                    .resolve_alias(alias)?
                    .context("BGI configured font unavailable");
            }
        }
        bail!("BGI font face {name:?} has no engine font binding")
    }
    fn text_plan(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
    ) -> Result<Option<TextWork>> {
        let a = &call.args;
        if call.group == 0x91 && matches!(call.id, 0x94..=0x99) {
            match call.id {
                0x94 => {
                    let source = if a[0] == 0 {
                        None
                    } else {
                        Some(vm.string(a[0])?)
                    };
                    let replacement = if a[1] == 0 {
                        None
                    } else {
                        Some(vm.string(a[1])?)
                    };
                    match (source.filter(|s| !s.is_empty()), replacement) {
                        (None, _) => self.text.ruby.clear(),
                        (Some(source), Some(replacement)) if !replacement.is_empty() => {
                            self.text.ruby.register(&source, &replacement, false)
                        }
                        (Some(source), _) => {
                            self.text.ruby.remove(&source);
                        }
                    }
                    vm.commit_call(&[])?;
                }
                0x95 => {
                    let source = vm.string(a[1])?;
                    let (records, count) = self.text.ruby.collect_records(&source);
                    let writes = if count > 0 {
                        let mut bytes = encoding_rs::SHIFT_JIS.encode(&records).0.into_owned();
                        bytes.push(0);
                        vec![crysirror_bgi_vm::HostMemoryWrite {
                            address: a[0] as u32,
                            bytes,
                        }]
                    } else {
                        Vec::new()
                    };
                    vm.commit_call_writes(&[count], writes)?;
                }
                0x96 => {
                    let records = vm.string(a[0])?;
                    let accepted = self.text.ruby.register_records(&records);
                    vm.commit_call(&[i32::from(accepted)])?;
                }
                0x97 => {
                    let name = self
                        .text
                        .fonts
                        .iter()
                        .find_map(|(name, font)| (font.id == a[0]).then_some(name.clone()));
                    self.text.style.font_name = name.filter(|s| !s.is_empty());
                    self.text.style.set_fields([a[1], a[2], a[3], a[4], -1, -1]);
                    vm.commit_call(&[])?;
                }
                0x98 => {
                    self.text
                        .layout
                        .configure(a.as_slice().try_into()?)
                        .map_err(|e| anyhow::anyhow!("BGI text layout {e:?}"))?;
                    self.text.spacing = self.text.layout.character_spacing;
                    self.text.indent = self.text.layout.mode_flag != 0;
                    vm.commit_call(&[])?;
                }
                0x99 => {
                    let accepted = self.text.layout.set_scale_divisor(a[0]);
                    vm.commit_call(&[i32::from(accepted)])?;
                }
                _ => unreachable!(),
            }
            return Ok(None);
        }
        if call.group == 0xb0 {
            match call.id {
                0xc0 | 0xc1 => {
                    let name = vm.string(a[0])?;
                    let option = if call.id == 0xc0 { -1 } else { a[1] };
                    let charset = match option {
                        0 => Some(128),
                        1 => Some(0),
                        _ => None,
                    };
                    let id = if let Some(font) = self.text.fonts.get_mut(&name) {
                        if option != -1 {
                            font.option = option;
                            font.charset = charset;
                        }
                        font.id
                    } else {
                        let id = self.text.next_font;
                        self.text.next_font = self.text.next_font.saturating_add(1).max(1);
                        self.text.fonts.insert(
                            name,
                            NativeFont {
                                id,
                                option,
                                charset,
                            },
                        );
                        id
                    };
                    vm.commit_call(&[id])?;
                    return Ok(None);
                }
                0xc2 | 0xc3 => {
                    let archive = if call.id == 0xc3 {
                        vm.string(a[0])?
                    } else {
                        String::new()
                    };
                    let name = vm.string(a[usize::from(call.id == 0xc3)])?;
                    if name.is_empty() {
                        vm.commit_call(&[0])?;
                        return Ok(None);
                    }
                    if self
                        .text
                        .resources
                        .keys()
                        .any(|key| key.rsplit(':').next() == Some(name.as_str()))
                    {
                        vm.commit_call(&[1])?;
                        return Ok(None);
                    }
                    let key = format!("{archive}:{name}");
                    let storage = host.storage(self.client)?;
                    let config = self.config.clone();
                    let client = self.client;
                    let file = name.clone();
                    let task = self.pool.spawn(move |cancel| {
                        cancel.check()?;
                        let bytes = program_io::read_raw(
                            storage.as_ref(),
                            client,
                            &config,
                            &archive,
                            &file,
                        )?
                        .ok_or(Error::NotFound)?;
                        cancel.check()?;
                        Ok(bytes)
                    })?;
                    return Ok(Some(TextWork::FontRead { key, name, task }));
                }
                0xc6 => {
                    let first = vm.string(a[0])?;
                    let second = vm.string(a[1])?;
                    let available = !first.is_empty()
                        && !second.is_empty()
                        && (first == second
                            || self.text.fonts.contains_key(&first)
                            || self.text.fonts.contains_key(&second)
                            || self.text.aliases.get(&first) == Some(&second));
                    vm.commit_call(&[i32::from(available)])?;
                    return Ok(None);
                }
                0xc7 => {
                    let alias = vm.string(a[0])?;
                    let face = vm.string(a[1])?;
                    if !alias.is_empty() {
                        self.text.aliases.insert(alias, face);
                    }
                    vm.commit_call(&[])?;
                    return Ok(None);
                }
                _ => unreachable!(),
            }
        }
        if call.group == 0x92 && call.id == 0x9e {
            let count = self.text.fragments.len().min(16);
            let mut bytes = vec![0; count * 128];
            for (index, record) in self.text.fragments.iter().take(count).enumerate() {
                let encoded = encoding_rs::SHIFT_JIS.encode(&record.text).0;
                let n = encoded.len().min(95);
                let at = index * 128;
                bytes[at..at + n].copy_from_slice(&encoded[..n]);
                bytes[at + 120..at + 124].copy_from_slice(&record.x.to_le_bytes());
                bytes[at + 124..at + 128].copy_from_slice(&record.y.to_le_bytes());
            }
            let writes = if bytes.is_empty() {
                Vec::new()
            } else {
                vec![crysirror_bgi_vm::HostMemoryWrite {
                    address: a[0] as u32,
                    bytes,
                }]
            };
            vm.commit_call_writes(&[count as i32], writes)?;
            self.text.fragments.clear();
            return Ok(None);
        }
        if call.group == 0x92 && call.id == 0x9b {
            let bytes = match a[1] {
                256 => self
                    .text
                    .output_pair
                    .into_iter()
                    .flat_map(i32::to_le_bytes)
                    .collect(),
                257 => self.text.output_color.to_le_bytes().to_vec(),
                _ => bail!("BGI text output parameter {}", a[1]),
            };
            vm.commit_call_writes(
                &[],
                vec![crysirror_bgi_vm::HostMemoryWrite {
                    address: a[0] as u32,
                    bytes,
                }],
            )?;
            return Ok(None);
        }
        let (bitmap, x, y, text, color, font, size, width_percent, kinsoku, pitch, shadow) =
            if call.group == 0x92 {
                let shadow = if matches!(a[15], 1 | 2)
                    && (0..=100).contains(&a[16])
                    && (0..=100).contains(&a[17])
                    && (0..=256).contains(&a[19])
                {
                    Some(BitmapTextShadow {
                        mode: a[15] as u8,
                        offset: [
                            (a[9] * a[16] / 100).max(1) as u16,
                            (a[9] * a[17] / 100).max(1) as u16,
                        ],
                        color: rgb(a[18] as u32),
                        concentration: a[19] as u16,
                    })
                } else {
                    None
                };
                (
                    a[0],
                    a[1],
                    a[2],
                    vm.string(a[3])?,
                    a[4] as u32,
                    a[8],
                    a[9],
                    a[10],
                    a[13] != 0,
                    a[14],
                    shadow,
                )
            } else {
                (
                    a[0],
                    a[1],
                    a[2],
                    vm.string(a[3])?,
                    a[13] as u32,
                    a[6],
                    a[7],
                    a[8],
                    a[11] != 0,
                    a[12],
                    None,
                )
            };
        ensure!(
            (4..=200).contains(&size) && (25..=200).contains(&width_percent),
            "BGI bitmap text height/width"
        );
        let (bitmap, info) = self.bitmap(bitmap)?;
        let info = info.clone();
        let font = self.font_alias(host, font)?;
        let library = host.font_library(self.client)?;
        ensure!(text.len() <= 1024 * 1024, "BGI bitmap text byte limit");
        let parsed = parse_message_markup_with(
            &text,
            &mut self.text.ruby,
            (self.text.override_color != -1).then_some(self.text.override_color as u32 & 0xffffff),
        );
        ensure!(
            parsed.text.chars().count() <= 65536,
            "BGI bitmap text glyph limit"
        );
        let mut advances = BTreeMap::new();
        for ch in parsed.text.chars() {
            if !advances.contains_key(&ch) {
                advances.insert(
                    ch,
                    library
                        .metrics(&font, size as f32, ch)?
                        .advance
                        .max(size as f32 * 0.5)
                        * width_percent as f32
                        / 100.,
                );
            }
        }
        let layout = layout_text(
            &parsed.text,
            &LayoutParams {
                x: x as f32,
                y: y as f32,
                size: size as f32,
                bitmap_width: info.size[0] as f32,
                spacing: self.text.spacing as f32,
                pitch_percent: pitch,
                kinsoku,
                indent_brackets: self.text.indent,
            },
            &|ch| advances.get(&ch).copied().unwrap_or(0.),
        );
        self.text.fragments = parsed
            .link_spans
            .iter()
            .filter_map(|link| {
                let first = layout.glyphs.iter().find(|g| g.index >= link.start_char)?;
                Some(crysirror_bgi_vm::System92TextFragmentRecord {
                    text: link.text.clone(),
                    x: first.x.round() as i32,
                    y: first.y.round() as i32,
                })
            })
            .collect();
        let glyphs = layout
            .glyphs
            .iter()
            .map(|g| BitmapGlyphRecord {
                character: g.ch,
                position: [g.x, g.y],
                color: rgb(parsed
                    .style_spans
                    .iter()
                    .find(|span| span.start_char <= g.index && g.index < span.end_char)
                    .and_then(|span| span.style.packed_rgb)
                    .unwrap_or(color)),
            })
            .collect();
        let output = if call.group == 0x92 {
            layout.lines
        } else {
            layout.end_x.round() as i32
        };
        let spec = BitmapTextSpec {
            font,
            size: size as u16,
            destination_format: info.format as u8,
            glyphs,
            shadow,
        };
        let job = if let Some(image) = info.image {
            host.image_metadata(self.client)?
                .begin_bitmap_text(ImageInput::Resource(image), spec)?
        } else {
            return Ok(Some(TextWork::Capture {
                bitmap,
                format: info.format,
                aux: info.auxiliary,
                spec,
                output,
                job: host.begin_canvas_image(self.client, bitmap)?,
            }));
        };
        Ok(Some(TextWork::Draw {
            bitmap,
            format: info.format,
            aux: info.auxiliary,
            output,
            job,
        }))
    }
    pub(crate) fn text_call(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
        request: u64,
    ) -> Result<bool> {
        let mut work = if let Some(work) = self.text_jobs.remove(&request) {
            work
        } else {
            match self.text_plan(host, vm, call)? {
                Some(work) => work,
                None => {
                    self.calls += 1;
                    return Ok(true);
                }
            }
        };
        loop {
            work = match work {
                TextWork::FontRead {
                    key,
                    name,
                    mut task,
                } => match task.try_take() {
                    Ok(None) => {
                        self.text_jobs
                            .insert(request, TextWork::FontRead { key, name, task });
                        return Ok(false);
                    }
                    Err(error) => {
                        if matches!(error.downcast_ref::<Error>(), Some(Error::NotFound)) {
                            vm.commit_call(&[0])?;
                            self.calls += 1;
                            return Ok(true);
                        }
                        return Err(error);
                    }
                    Ok(Some(bytes)) => {
                        let job = host.font_library(self.client)?.begin(FontLoadSpec {
                            name: format!("bgi/font/{}", self.text.resources.len()),
                            source: FontLoadSource::Bytes(Arc::from(bytes)),
                            face_index: 0,
                        })?;
                        let _ = name;
                        TextWork::FontLoad { key, job }
                    }
                },
                TextWork::FontLoad { key, job } => {
                    let library = host.font_library(self.client)?;
                    match library.status(job)? {
                        FontLoadState::Pending => {
                            self.text_jobs
                                .insert(request, TextWork::FontLoad { key, job });
                            return Ok(false);
                        }
                        FontLoadState::Failed(error) => {
                            library.release_job(job)?;
                            bail!("BGI font registration: {error}");
                        }
                        FontLoadState::Ready(info) => {
                            library.release_job(job)?;
                            for family in &info.families {
                                self.text.aliases.insert(family.clone(), info.alias.clone());
                            }
                            self.text.resources.insert(key, info.alias);
                            vm.commit_call(&[1])?;
                            self.calls += 1;
                            return Ok(true);
                        }
                    }
                }
                TextWork::Capture {
                    bitmap,
                    format,
                    aux,
                    spec,
                    output,
                    mut job,
                } => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.text_jobs.insert(
                            request,
                            TextWork::Capture {
                                bitmap,
                                format,
                                aux,
                                spec,
                                output,
                                job,
                            },
                        );
                        return Ok(false);
                    }
                    ImageProcessState::Failed(error) => bail!("BGI bitmap text capture: {error}"),
                    ImageProcessState::Ready(image) => TextWork::Draw {
                        bitmap,
                        format,
                        aux,
                        output,
                        job: host
                            .image_metadata(self.client)?
                            .begin_bitmap_text(ImageInput::Resource(image), spec)?,
                    },
                },
                TextWork::Draw {
                    bitmap,
                    format,
                    aux,
                    output,
                    mut job,
                } => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.text_jobs.insert(
                            request,
                            TextWork::Draw {
                                bitmap,
                                format,
                                aux,
                                output,
                                job,
                            },
                        );
                        return Ok(false);
                    }
                    ImageProcessState::Failed(error) => bail!("BGI bitmap text raster: {error}"),
                    ImageProcessState::Ready(raw) => {
                        if format == 1 && !raw.opaque() {
                            let effects = Arc::from(serde_json::to_vec(&vec![
                                crysirror_image::effects::ImageEffect::OverrideAlpha { alpha: 255 },
                            ])?);
                            let job = host
                                .image_metadata(self.client)?
                                .begin_process(ImageInput::Resource(raw.clone()), effects)?;
                            TextWork::Display {
                                bitmap,
                                format,
                                aux,
                                output,
                                raw,
                                job,
                            }
                        } else {
                            self.publish_bitmap(host, bitmap, format, aux, raw.clone(), raw)?;
                            vm.commit_call(&[output])?;
                            self.calls += 1;
                            return Ok(true);
                        }
                    }
                },
                TextWork::Display {
                    bitmap,
                    format,
                    aux,
                    output,
                    raw,
                    mut job,
                } => match job.poll()? {
                    ImageProcessState::Pending => {
                        self.text_jobs.insert(
                            request,
                            TextWork::Display {
                                bitmap,
                                format,
                                aux,
                                output,
                                raw,
                                job,
                            },
                        );
                        return Ok(false);
                    }
                    ImageProcessState::Failed(error) => bail!("BGI bitmap text display: {error}"),
                    ImageProcessState::Ready(display) => {
                        self.publish_bitmap(host, bitmap, format, aux, raw, display)?;
                        vm.commit_call(&[output])?;
                        self.calls += 1;
                        return Ok(true);
                    }
                },
            };
        }
    }
}
