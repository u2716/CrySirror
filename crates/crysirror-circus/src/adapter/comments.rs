//! Native timed comments and chat/log presentation. Text placement, coverage,
//! immutable images and pixel curtains are shared engine capabilities.
use super::*;
use crate::machine::comment_ops::CommentRequest;
use crysirror_text::reveal::pixel_curtain::PixelCurtain;
const ROOT: u32 = 0x3d00_0000;
const BASE: u32 = ROOT + 0x1000;
const BUILD_SHA: &str = "178ee9a5e34356b11942213e2e32a34afe4933c90d563bf5a3d6402250f54ab7";
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Row {
    text: String,
    icon: Option<u8>,
    stamp: Option<u16>,
    reveal: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ObjectCurtain {
    curtain: PixelCurtain,
    layers: Vec<(u32, u32)>,
    text: String,
}
#[derive(Default, Serialize, Deserialize)]
pub(super) struct NativeComments {
    backend: Option<i16>,
    enabled: bool,
    enabled_word: u32,
    published: bool,
    sources: Vec<String>,
    declarations: Vec<String>,
    raw_sources: Vec<u8>,
    selecting: bool,
    pending_selector: Option<(i16, [i16; 8])>,
    pending_sfx: bool,
    rows: Vec<Row>,
    tail: String,
    random: bool,
    #[serde(default)]
    random_word: i32,
    index: usize,
    interval: i32,
    clear_after: i32,
    clear_count: i32,
    last: u64,
    delay: u64,
    //456F00 stores the already multiplied signed DWORD duration in milliseconds.
    fade: Option<(u64, i32, i32, i32)>,
    #[serde(skip)]
    restore_owner: Option<Vec<u8>>,
    page: usize,
    force_finish: bool,
    pending_fill: bool,
    pending_choice: Option<usize>,
    pending_delay: Option<u64>,
    dirty: bool,
    curtains: Vec<ObjectCurtain>,
    names: Vec<String>,
    #[serde(skip)]
    assets: BTreeMap<String, UploadedImage>,
}
impl NativeComments {
    pub(super) fn animation_wait_busy(&self) -> bool {
        //457D50 returns six rich-text reveal states for backend1 (457BC0),
        //but only the active page timer for backend2 (458D10). Its sprite
        //decoration queues and the next queued comment do not enter v106.
        match self.backend {
            Some(1) => self.curtains.iter().any(|object| !object.curtain.complete()),
            Some(2) => self.interval != 0 && self.enabled,
            _ => false,
        }
    }
    pub(super) fn animating(&self) -> bool {
        self.backend.is_some()
            && (!self.sources.is_empty()
                || self.force_finish
                || self.curtains.iter().any(|c| !c.curtain.complete()))
    }
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            self.backend.is_none_or(|b| b == 1 || b == 2),
            "Circus saved comment backend"
        );
        ensure!(
            self.rows.len() <= 6
                && self.sources.len() <= 200
                && self.names.len() <= 11
                && (self.backend.is_none() || self.raw_sources.len() == 16400)
                && self.curtains.len() <= 6,
            "Circus saved comment budgets"
        );
        for object in &self.curtains {
            object.curtain.validate()?;
            ensure!(
                object.layers.len() == object.curtain.visible_widths().len(),
                "Circus saved comment curtain identity"
            );
        }
        for row in self.sources.iter().chain(self.rows.iter().map(|r| &r.text)) {
            let (bytes, _, error) = encoding_rs::SHIFT_JIS.encode(row);
            ensure!(
                !error && bytes.len() <= 260,
                "Circus saved comment text encoding/extent"
            );
        }
        Ok(())
    }
    pub(super) fn native_record(&self) -> Option<Vec<u8>> {
        if matches!(self.backend, Some(1 | 2)) {
            Some(self.raw_sources.get(..1668).unwrap_or(&[]).to_vec())
        } else {
            None
        }
    }
    fn write_raw_controls(&mut self) -> Result<()> {
        if self.backend.is_none() {
            return Ok(());
        }
        ensure!(
            self.raw_sources.len() == 16400,
            "Circus native log source arena"
        );
        self.raw_sources[1640..1644].copy_from_slice(&self.interval.to_le_bytes());
        if self.backend == Some(1) {
            self.raw_sources[1644..1648].copy_from_slice(&self.random_word.to_le_bytes());
            self.raw_sources[1648..1652].copy_from_slice(&self.clear_after.to_le_bytes());
            let (_, from, to, duration) = self.fade.unwrap_or((0, 0, 0, 0));
            for (at, value) in [(1652, from), (1656, to), (1660, duration)] {
                self.raw_sources[at..at + 4].copy_from_slice(&value.to_le_bytes());
            }
            //The final copied DWORD is unconsumed backend1 allocation padding.
            //Leave imported bytes intact; only new allocations normalize it.
            return Ok(());
        }
        self.raw_sources[1664..1668].copy_from_slice(&self.enabled_word.to_le_bytes());
        for (index, row) in self.sources.iter_mut().enumerate() {
            let raw = &self.raw_sources[index * 82..index * 82 + 82];
            let end = raw
                .iter()
                .position(|b| *b == 0)
                .context("Circus native log row terminator")?;
            let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw[..end]);
            ensure!(!error, "Circus native log control-overlap encoding");
            *row = text.into_owned();
        }
        Ok(())
    }
    fn reset_timing(&mut self) {
        self.interval = -1;
        self.random = true;
        self.random_word = 1;
        self.index = 0;
        self.clear_after = 0;
        self.clear_count = 0;
        self.delay = 0;
        self.fade = None;
        self.pending_choice = None;
        self.pending_delay = None;
    }
}
pub(super) fn load_comment_metadata(storage: &dyn ClientStorage) -> Result<Vec<String>> {
    let names = storage
        .list(Mount::Asset, "")?
        .into_iter()
        .filter(|n| n.to_ascii_lowercase().ends_with(".exe"))
        .collect::<Vec<_>>();
    ensure!(
        names.len() <= 32,
        "Circus comment metadata candidate budget"
    );
    let mut budget = crysirror_resource::pe::MAX_PE_BYTES as u64;
    for name in names {
        let path = VfsPath::new(Mount::Asset, &name)?;
        let metadata = storage
            .metadata(Mount::Asset, path.relative())?
            .context("Circus comment metadata source")?;
        if metadata.is_directory {
            continue;
        }
        let size = metadata
            .size
            .context("Circus comment metadata source extent")?;
        ensure!(size <= budget, "Circus comment metadata read budget");
        budget -= size;
        let bytes = crysirror_resource::read_all(storage.open_read(&path)?.as_ref(), size)?;
        let Ok(pe) = crysirror_resource::pe::PeData::parse(&bytes) else {
            continue;
        };
        let digest = pe
            .sha256()
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>();
        if digest != BUILD_SHA {
            continue;
        }
        let mut table = vec![];
        for address in [
            0x492c20, 0x492c18, 0x492c10, 0x492c08, 0x492c00, 0x492bf8, 0x492bf0, 0x492be8,
            0x48b92c, 0x492be0, 0x492bd8,
        ] {
            let raw = pe.va_slice(address, 8)?;
            let end = raw
                .iter()
                .position(|b| *b == 0)
                .context("Circus comment label terminator")?;
            let (value, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw[..end]);
            ensure!(!error, "Circus comment label encoding");
            table.push(value.into_owned());
        }
        return Ok(table);
    }
    bail!("Circus comment presentation requires pinned native metadata")
}
impl Bridge<'_> {
    pub(super) fn comment_layer_ids(&self) -> Vec<u32> {
        let c = &self.services.comments;
        if !c.published {
            return vec![];
        }
        let mut ids = vec![ROOT, ROOT + 1, ROOT + 2];
        for object in &c.curtains {
            ids.extend(object.layers.iter().map(|(id, _)| *id));
        }
        for (n, row) in c.rows.iter().enumerate() {
            if row.icon.is_some() {
                ids.push(BASE + n as u32 * 64 + 40);
            }
            if row.stamp.is_some() {
                ids.push(BASE + n as u32 * 64 + 41);
            }
        }
        ids
    }
    pub(super) fn restore_comment_record(&mut self, record: &[u8], source_arena: &[u8]) -> Result<bool> {
        ensure!(
            record.len() == 1668,
            "Circus restored comment record extent"
        );
        //457F30 ignores copied bytes when no backend is selected.
        if self.services.comments.backend.is_none() {
            return Ok(true);
        }
        let mut c = std::mem::take(&mut self.services.comments);
        let result = (|| -> Result<bool> {
            if !self.comment_assets(&mut c)? {
                return Ok(false);
            }
            let source=if c.backend==Some(1){&record[..1640]}else{
                ensure!(source_arena.len()==16400,"Circus restored log source body extent");
                source_arena
            };
            let owner_bytes=if c.backend==Some(1){record}else{source};
            if let Some(owner) = &c.restore_owner {
                ensure!(owner == owner_bytes, "Circus pending comment restore identity");
            } else {
                c.restore_owner = Some(owner_bytes.to_vec());
                if c.backend==Some(1){c.raw_sources=vec![0;16400];}
                else {
                    //458090 clears only20 source rows and the interval. The
                    //remaining180-row arena and enabled word belong to the
                    //existing player; they are not a newly zeroed buffer.
                    ensure!(c.raw_sources.len()==16400,"Circus retained log source arena");
                    c.raw_sources[..1644].fill(0);
                    c.interval=0;
                }
                let mut sources = Vec::new();
                for (index,raw) in source.chunks_exact(82).enumerate() {
                    if raw[0] == 0 { break; }
                    let n = raw.iter().position(|b| *b == 0)
                        .context("Circus restored comment source terminator")?;
                    let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&raw[..n]);
                    ensure!(!error, "Circus restored comment source encoding");
                    sources.push(text.into_owned());
                    if c.backend==Some(2){
                        let at=index*82;
                        c.raw_sources[at..at+n+1].copy_from_slice(&raw[..n+1]);
                    }
                }
                if c.backend==Some(1){c.raw_sources[..1668].copy_from_slice(record);}
                c.sources = sources.clone();
                c.declarations = sources;
                c.page = 0;
                c.pending_sfx = false;
                if c.backend == Some(1) {
                    c.rows.clear();c.tail.clear();c.curtains.clear();
                    //457F30 first resets456EB0, then restores six controls and
                    //fills rows only when a nonempty list terminates before20.
                    //Exactly20 entries deliberately skips that native branch.
                    c.reset_timing();
                    if !c.sources.is_empty() && c.sources.len() < 20 {
                        c.interval = i32::from_le_bytes(record[1640..1644].try_into()?);
                        c.random_word = i32::from_le_bytes(record[1644..1648].try_into()?);
                        c.random = c.random_word != 0;
                        c.clear_after = i32::from_le_bytes(record[1648..1652].try_into()?);
                        let duration = i32::from_le_bytes(record[1660..1664].try_into()?);
                        if duration != 0 {
                            //456F00 multiplies its argument by100 even on this
                            //restore call; retain the original repeated scaling.
                            c.fade = Some((self.host.now_ms(),
                                i32::from_le_bytes(record[1652..1656].try_into()?),
                                i32::from_le_bytes(record[1656..1660].try_into()?),
                                duration.wrapping_mul(100)));
                        }
                        c.pending_fill = true;
                    }
                } else {
                    //457F30 reads up to200 rows from the actual live Buf,
                    //including body fields after the copied1668-byte record.
                    //Full200 and empty lists skip458060/enabled/page entirely.
                    c.interval = i32::from_le_bytes(c.raw_sources[1640..1644].try_into()?);
                    c.enabled_word = u32::from_le_bytes(c.raw_sources[1664..1668].try_into()?);
                    if !c.sources.is_empty()&&c.sources.len()<200 {
                        c.enabled_word=u32::from_le_bytes(record[1664..1668].try_into()?);
                        c.rows.clear();c.tail.clear();c.curtains.clear();
                        self.comment_page(&mut c,self.host.now_ms(),false)?;
                    }
                    c.enabled=c.enabled_word!=0;
                }
            }
            if c.backend == Some(1) && c.pending_fill {
                while c.rows.len() < 6 && !c.sources.is_empty() {
                    if !self.comment_emit(&mut c, self.host.now_ms(), true)? { return Ok(false); }
                }
                for row in &mut c.rows { row.reveal = false; }
                c.pending_fill = false;
            }
            c.write_raw_controls()?;
            self.publish_comments(&mut c)?;
            c.restore_owner = None;
            Ok(true)
        })();
        self.services.comments = c;
        result
    }
    pub(super) fn native_comment(&mut self, request: &CommentRequest) -> Result<bool> {
        if let CommentRequest::Select { backend, now } = request {
            if !self.services.comments.selecting || self.services.comments.backend != *backend {
                let old = std::mem::take(&mut self.services.comments);
                if old.published {
                    self.submit(vec![json!({"op":"remove","id":ROOT})], vec![])?;
                }
                self.services.comments = NativeComments {
                    backend: *backend,
                    last: *now,
                    selecting: true,
                    raw_sources: if backend.is_some() {
                        vec![0; 16400]
                    } else {
                        vec![]
                    },
                    ..Default::default()
                };
                if *backend == Some(1) {
                    self.services.comments.reset_timing();
                    self.services.comments.write_raw_controls()?;
                }
            }
            let mut comments = std::mem::take(&mut self.services.comments);
            let ready = if backend.is_some() {
                self.comment_assets(&mut comments)?
            } else {
                true
            };
            if ready {
                comments.selecting = false;
            }
            self.services.comments = comments;
            return Ok(ready);
        }
        if let CommentRequest::Reset { backend, now } = request {
            ensure!(self.services.comments.backend == *backend, "Circus comment reset backend owner");
            let mut c = std::mem::take(&mut self.services.comments);
            if c.published { self.submit(vec![json!({"op":"remove","id":ROOT})], vec![])?; }
            c.published = false;
            c.sources.clear(); c.declarations.clear();
            if c.backend==Some(1){c.rows.clear();c.curtains.clear();c.tail.clear();}
            c.page = 0; c.last = *now;
            c.pending_selector = None; c.pending_fill = false;
            c.force_finish = false; c.pending_sfx = false; c.restore_owner = None;
            if c.backend == Some(1) {
                c.raw_sources[..1640].fill(0);
                c.reset_timing();
                c.write_raw_controls()?;
            } else if c.backend == Some(2) {
                //458090 clears the source rows and interval, retaining enabled
                //and the remainder of the original overlapping record tail.
                c.raw_sources[..1644].fill(0);
                c.interval = 0;
                self.publish_comments(&mut c)?;
            }
            self.services.comments = c;
            return Ok(true);
        }
        let mut comments = std::mem::take(&mut self.services.comments);
        let result = self.comment_request(&mut comments, request);
        self.services.comments = comments;
        result
    }
    fn comment_request(
        &mut self,
        c: &mut NativeComments,
        request: &CommentRequest,
    ) -> Result<bool> {
        let (backend, selector, args, sources, now, suppressed, skip) = match request {
            CommentRequest::Control {
                backend,
                selector,
                arguments,
                sources,
                now,
                suppressed,
                skip,
            } => (
                *backend,
                Some(*selector),
                *arguments,
                sources,
                *now,
                *suppressed,
                *skip,
            ),
            CommentRequest::Poll {
                sources,
                now,
                suppressed,
                skip,
            } => (
                c.backend.unwrap_or(0),
                None,
                [0; 8],
                sources,
                *now,
                *suppressed,
                *skip,
            ),
            CommentRequest::Select { .. } | CommentRequest::Reset { .. } => unreachable!(),
        };
        if !matches!(backend, 1 | 2) {
            return Ok(true);
        }
        ensure!(
            c.backend == Some(backend),
            "Circus comment backend selection receipt"
        );
        if !self.comment_assets(c)? {
            return Ok(false);
        }
        if c.declarations != *sources {
            let append = sources.starts_with(&c.declarations);
            let first = if append { c.declarations.len() } else { 0 };
            if append {
                c.sources.extend(sources[first..].iter().cloned());
            } else {
                c.sources = sources.clone();
            }
            {
                ensure!(
                    c.raw_sources.len() == 16400,
                    "Circus native log source arena"
                );
                for (index, row) in sources.iter().enumerate().skip(first) {
                    let (bytes, _, error) = encoding_rs::SHIFT_JIS.encode(row);
                    ensure!(
                        !error && bytes.len() < 82,
                        "Circus native log source record"
                    );
                    let at = 82 * index;
                    for (n, byte) in bytes.iter().copied().chain(std::iter::once(0)).enumerate() {
                        c.raw_sources[at + n] = byte;
                    }
                }
                if backend == 2 {
                    c.interval = i32::from_le_bytes(c.raw_sources[1640..1644].try_into()?);
                    c.enabled_word = u32::from_le_bytes(c.raw_sources[1664..1668].try_into()?);
                    c.enabled = c.enabled_word != 0;
                }
            }
            c.declarations = sources.clone();
            c.dirty=true;
        }
        let apply = if let Some(selector) = selector {
            match c.pending_selector {
                None => {
                    c.pending_selector = Some((selector, args));
                    true
                }
                Some(previous) => {
                    ensure!(
                        previous == (selector, args),
                        "Circus pending comment command identity"
                    );
                    false
                }
            }
        } else {
            false
        };
        if let Some(selector) = selector.filter(|_| apply) {
            match selector {
                0 => {
                    if backend == 1 {
                        c.reset_timing()
                    } else {
                        c.rows.clear();
                        self.comment_page(c, now, suppressed)?;
                    }
                    c.dirty = true;
                }
                1 => {
                    if backend == 1 {
                        c.reset_timing();
                        c.sources.clear();
                        c.declarations.clear();
                        c.raw_sources[..1640].fill(0)
                    } else {
                        c.rows.clear();
                        c.curtains.clear()
                    }
                    c.enabled = true;
                    c.enabled_word = 1;
                    c.dirty = true;
                }
                2 if backend == 1 => c.interval = i32::from(args[1]),
                2 => {
                    if args[1] == -1 {
                        if !c.force_finish && c.interval != 0 {
                            c.force_finish = true;
                            c.interval = 1;
                        }
                    } else {
                        c.interval = i32::from(args[1]);
                    }
                    if c.force_finish && c.interval != 0 {
                        c.write_raw_controls()?;
                        return Ok(false);
                    }
                }
                3 if backend == 1 => { c.random = args[1] != 0; c.random_word = i32::from(args[1]); },
                4 if backend == 1 => c.pending_fill = true,
                5 if backend == 1 && args[3] != 0 => {
                    c.fade = Some((now, i32::from(args[1]), i32::from(args[2]), i32::from(args[3]) * 100))
                }
                6 if backend == 1 => c.clear_after = i32::from(args[1]),
                7 => {
                    c.enabled = true;
                    c.enabled_word = 1;
                    c.dirty = true;
                }
                98 => {
                    c.enabled = false;
                    c.enabled_word = 0;
                    c.dirty = true;
                }
                _ => {} // Native switch has no default side effect.
            }
        }
        if backend == 2
            && selector == Some(2)
            && args[1] == -1
            && c.force_finish
            && now.saturating_sub(c.last) > 10
        {
            self.comment_page(c, now, suppressed)?;
        }
        if backend == 1 {
            if (1..=19).contains(&self.input.pointer_x)
                && (461..=564).contains(&self.input.pointer_y)
                && !c.enabled
            {
                c.enabled = true;
                c.dirty = true;
            }
            if c.pending_fill {
                while c.rows.len() < 6 && !c.sources.is_empty() {
                    if !self.comment_emit(c, now, true)? {
                        return Ok(false);
                    }
                }
                for row in &mut c.rows {
                    row.reveal = false;
                }
                c.pending_fill = false;
            } else if selector.is_none()
                && c.curtains.iter().all(|o| o.curtain.complete())
                && !c.sources.is_empty()
            {
                if !self.comment_emit(c, now, false)? {
                    return Ok(false);
                }
            }
        } else {
            if suppressed || !c.enabled {
                c.interval = 0;
            }
            if skip && c.interval != 0 {
                c.interval = 50;
            }
            if selector.is_none()
                && c.interval != 0
                && now.saturating_sub(c.last)
                    > u64::try_from(i64::from(c.interval) * 10).unwrap_or(0)
            {
                self.comment_page(c, now, suppressed)?;
                c.dirty = true;
            }
            if c.interval == 0 {
                c.force_finish = false;
            }
        }
        if c.dirty {
            self.publish_comments(c)?;
            c.dirty = false;
        }
        if c.pending_sfx {
            if !self.play_audio(2, "se\\473", false, false, 0, None)? {
                return Ok(false);
            }
            c.pending_sfx = false;
        }
        c.write_raw_controls()?;
        if !c.force_finish && selector.is_some() {
            c.pending_selector = None;
        }
        Ok(!c.force_finish)
    }
    fn comment_assets(&mut self, c: &mut NativeComments) -> Result<bool> {
        let names = if c.backend == Some(1) {
            vec!["system\\commentback"]
        } else {
            vec!["bg\\black", "system\\COMUCHIP", "system\\comustamp"]
        };
        for (n, name) in names.into_iter().enumerate() {
            if c.assets.contains_key(name) {
                continue;
            }
            let Some(Prepared::Image(image, _)) = self
                .services
                .prepare(Work::Image(name.into()), self.client)?
            else {
                return Ok(false);
            };
            let path = self.host.upload_rgba(
                self.client,
                ROOT + 0x800 + n as u32,
                image.width,
                image.height,
                &image.rgba,
            )?;
            c.assets.insert(
                name.into(),
                UploadedImage {
                    path,
                    size: [image.width, image.height],
                    opaque: image.rgba.chunks_exact(4).all(|p| p[3] == 255),
                },
            );
        }
        if c.backend == Some(2) && c.names.is_empty() {
            let Some(Prepared::CommentMetadata(names)) =
                self.services.prepare(Work::CommentMetadata, self.client)?
            else {
                return Ok(false);
            };
            c.names = names;
        }
        Ok(true)
    }
    fn comment_emit(&mut self, c: &mut NativeComments, now: u64, forced: bool) -> Result<bool> {
        if !c.tail.is_empty() {
            let tail = std::mem::take(&mut c.tail);
            append_comment(c, &tail)?;
            return Ok(true);
        }
        if !forced && now.saturating_sub(c.last) < c.delay {
            return Ok(true);
        }
        if c.pending_choice.is_none() {
            let Some(choice) = NativeHost::random(self, u16::try_from(c.sources.len())?)? else {
                return Ok(false);
            };
            c.pending_choice = Some(if c.random {
                usize::try_from(choice)?
            } else {
                c.index % c.sources.len()
            });
        }
        if c.pending_delay.is_none() {
            let range = if c.fade.is_some() {
                50
            } else if c.interval == -1 {
                2500
            } else {
                200
            };
            let Some(jitter) = NativeHost::random(self, range)? else {
                return Ok(false);
            };
            let delay = if let Some((began, from, to, duration)) = c.fade {
                let elapsed = now.saturating_sub(began);
                let total = i64::from(duration);
                let percentage = if (elapsed as i64) <= total {
                    let slope = (f64::from(to.wrapping_sub(from)) / total as f64) as f32;
                    (f64::from(slope) * elapsed as f64 + f64::from(from)) as i32
                } else {
                    i32::from(to)
                };
                let base = if c.interval == -1 { 2500 } else { c.interval };
                i64::from((base.wrapping_mul(percentage) / 100).wrapping_mul(100)) + i64::from(jitter)
            } else if c.interval == -1 {
                500 + i64::from(jitter)
            } else {
                i64::from(c.interval.wrapping_mul(100)) + i64::from(jitter)
            };
            c.pending_delay =
                Some(u64::from(delay as u32));
        }
        let index = c.pending_choice.take().unwrap();
        ensure!(index < c.sources.len(), "Circus comment source choice");
        if c.random {
            if c.clear_after != 0 {
                if c.index as i32 >= c.clear_after {
                    for (n, source) in c.sources.iter_mut().take(6).enumerate() {
                        *source = " ".into();
                        c.raw_sources[n * 82..n * 82 + 2].copy_from_slice(&[32, 0]);
                    }
                    c.index = 0;
                }
                c.index += 1;
            }
        } else {
            if c.index >= c.sources.len() {
                c.index = 0;
                if c.clear_after != 0 {
                    for (n, source) in c.sources.iter_mut().take(6).enumerate() {
                        *source = " ".into();
                        c.raw_sources[n * 82..n * 82 + 2].copy_from_slice(&[32, 0]);
                    }
                }
            }
            c.index = index + 1;
        }
        let text = c.sources[index].clone();
        c.last = now;
        c.delay = c.pending_delay.take().unwrap();
        append_comment(c, &text)?;
        Ok(true)
    }
    fn comment_page(&mut self, c: &mut NativeComments, now: u64, suppressed: bool) -> Result<()> {
        if suppressed {
            c.interval = 0;
        }
        let mut start = c.sources.len().saturating_sub(5);
        let mut end = c.sources.len();
        if c.interval != 0 {
            end = c.page + 1;
            if end > c.sources.len() {
                c.interval = 0;
                return Ok(());
            }
            start = if c.page < 2 { 0 } else { c.page - 2 };
            c.last = now;
            if end >= c.sources.len() {
                c.interval = 0;
            }
        }
        c.rows.clear();
        let mut last_unknown = true;
        for source in &c.sources[start..end] {
            last_unknown = source
                .strip_prefix('「')
                .and_then(|s| s.split_once('」'))
                .is_some_and(|(name, _)| !c.names.iter().any(|prefix| name.starts_with(prefix)));
            let (row, interval) = log_row(source, &c.names)?;
            if let Some(interval) = interval {
                c.interval = interval;
                if suppressed || !c.enabled {
                    c.interval = 0;
                }
            }
            c.rows.push(row);
        }
        c.page = end;
        if c.enabled && !c.sources.is_empty() && !last_unknown && !suppressed {
            c.pending_sfx = true;
        }
        c.dirty = true;
        Ok(())
    }
    fn publish_comments(&mut self, c: &mut NativeComments) -> Result<()> {
        let mut layers = vec![];
        if c.published {
            layers.push(json!({"op":"remove","id":ROOT}));
        }
        layers.push(json!({"op":"put","layer":{"id":ROOT,"parent":SCENE_ROOT,"rect":[0,0,1280,720],"visible":c.enabled&&!c.rows.is_empty()&&(c.backend!=Some(2)||!c.sources.is_empty()),"paint":{"kind":"solid","color":[0,0,0,0]},"order":[112,0]}}));
        if c.backend == Some(1) {
            comment_image(
                &mut layers,
                ROOT + 1,
                ROOT,
                &c.assets["system\\commentback"],
                [8, 390],
                None,
                255,
            )?;
        } else {
            comment_image(
                &mut layers,
                ROOT + 1,
                ROOT,
                &c.assets["bg\\black"],
                [0, 0],
                None,
                127,
            )?;
            comment_image(
                &mut layers,
                ROOT + 2,
                ROOT,
                &c.assets["system\\COMUCHIP"],
                [440, 16],
                Some([0, 0, 372, 518]),
                255,
            )?;
        }
        let previous = std::mem::take(&mut c.curtains);
        for (n, row) in c.rows.iter().enumerate() {
            let origin = if c.backend == Some(1) {
                [10, 374 + 12 * n as i16]
            } else {
                [540, 94 + 81 * n as i16]
            };
            let base = BASE + n as u32 * 64;
            let parts = self.comment_glyph_rows(
                &mut layers,
                ROOT,
                base,
                &row.text,
                origin,
                if c.backend == Some(1) { 7 } else { 135 },
            )?;
            let widths = parts.iter().map(|(_, width, _)| *width).collect();
            let mut curtain = PixelCurtain::new(widths, 28)?;
            if c.backend == Some(2) || !row.reveal {
                curtain.finish();
            }
            if let Some(old) = previous.get(n).filter(|old| old.text == row.text) {
                curtain = old.curtain.clone();
            }
            let object = ObjectCurtain {
                curtain,
                text: row.text.clone(),
                layers: parts.iter().map(|(id, _, height)| (*id, *height)).collect(),
            };
            for ((id, height), width) in object.layers.iter().zip(object.curtain.visible_widths()) {
                if let Some(layer) = layers.iter_mut().find(|v| v["layer"]["id"] == *id) {
                    layer["layer"]["source_crop"] = json!([0, 0, width, height]);
                }
            }
            c.curtains.push(object);
            if let Some(icon) = row.icon {
                comment_image(
                    &mut layers,
                    base + 40,
                    ROOT,
                    &c.assets["system\\COMUCHIP"],
                    [468, 80 + 81 * n as i16],
                    Some([420, 81 * i16::from(icon), 320, 81]),
                    255,
                )?;
            }
            if let Some(stamp) = row.stamp {
                let index = stamp - 1;
                comment_image(
                    &mut layers,
                    base + 41,
                    ROOT,
                    &c.assets["system\\comustamp"],
                    [480, 96 + 81 * n as i16],
                    Some([80 * (index / 8) as i16, 64 * (index % 8) as i16, 79, 63]),
                    255,
                )?;
            }
        }
        self.submit(layers, vec![])?;
        c.published = true;
        Ok(())
    }
    pub(super) fn advance_comments(&mut self) -> Result<()> {
        let mut commands = vec![];
        for object in &mut self.services.comments.curtains {
            if object.curtain.complete() {
                continue;
            }
            object.curtain.advance_ticks(1)?;
            for ((id, height), width) in object.layers.iter().zip(object.curtain.visible_widths()) {
                commands.push(json!({"op":"set_source_crop","id":id,"crop":[0,0,width,height]}));
            }
        }
        if !commands.is_empty() {
            self.submit(commands, vec![])?;
        }
        Ok(())
    }
}
fn split_encoded(text: &str, limit: usize, keep_pair: bool) -> Result<(String, String)> {
    let (bytes, _, error) = encoding_rs::SHIFT_JIS.encode(text);
    ensure!(!error, "Circus comment Shift-JIS");
    if bytes.len() <= limit {
        return Ok((text.into(), String::new()));
    }
    let mut boundary = limit;
    if keep_pair && boundary > 0 && is_lead(bytes[boundary - 1]) && valid_trail(bytes[boundary]) {
        boundary += 1;
    }
    let decode = |bytes: &[u8]| -> Result<String> {
        let (value, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(bytes);
        ensure!(!error, "Circus native comment split cuts multibyte text");
        Ok(value.into_owned())
    };
    Ok((decode(&bytes[..boundary])?, decode(&bytes[boundary..])?))
}
fn is_lead(byte: u8) -> bool {
    (0x81..=0x9f).contains(&byte) || (0xe0..=0xfc).contains(&byte)
}
fn valid_trail(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte) || (0x80..=0xfc).contains(&byte)
}
fn append_comment(c: &mut NativeComments, text: &str) -> Result<()> {
    let (text, tail) = split_encoded(&format!("@r{text}"), 40, true)?;
    c.tail = tail;
    if c.rows.len() >= 6 {
        c.rows.remove(0);
    }
    for row in &mut c.rows {
        row.reveal = false;
    }
    c.rows.push(Row {
        text,
        reveal: true,
        ..Default::default()
    });
    c.dirty = true;
    Ok(())
}
fn log_row(source: &str, names: &[String]) -> Result<(Row, Option<i32>)> {
    let mut row = Row {
        text: source.into(),
        ..Default::default()
    };
    if let Some(rest) = row.text.strip_prefix('「') {
        if let Some((name, text)) = rest.split_once('」') {
            row.icon = Some(
                names
                    .iter()
                    .position(|prefix| name.starts_with(prefix))
                    .unwrap_or(1) as u8,
            );
            row.text = text.into();
        }
    }
    if let Some(at) = row.text.find("@d") {
        let digits = row.text[at + 2..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        let number = if digits == 0 {
            0
        } else {
            row.text[at + 2..at + 2 + digits].parse::<u16>()?
        };
        if number > 0 {
            row.stamp = Some(number);
            row.icon = None;
            row.text.clear();
        }
    }
    let mut interval = None;
    while let Some(at) = row.text.find("@t") {
        let digits = row.text[at + 2..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        interval = Some(if digits == 0 {
            0
        } else {
            row.text[at + 2..at + 2 + digits].parse()?
        });
        row.text.replace_range(at..at + 2 + digits, "");
    }
    let (first, tail) = split_encoded(&row.text, 28, false)?;
    row.text = format!(
        "@y{first}{}",
        if tail.is_empty() {
            String::new()
        } else {
            format!("\n{tail}")
        }
    );
    Ok((row, interval))
}
fn comment_image(
    layers: &mut Vec<Value>,
    id: u32,
    parent: u32,
    image: &UploadedImage,
    position: [i16; 2],
    source: Option<[i16; 4]>,
    opacity: u8,
) -> Result<()> {
    let [sx, sy, w, h] = source.unwrap_or([
        0,
        0,
        i16::try_from(image.size[0])?,
        i16::try_from(image.size[1])?,
    ]);
    ensure!(w > 0 && h > 0, "Circus comment image rectangle");
    layers.push(json!({"op":"put","layer":{"id":id,"parent":parent,"rect":[position[0],position[1],w,h],"opacity":opacity,"paint":{"kind":"image","asset":image.path.relative()},"uv":[f32::from(sx)/image.size[0] as f32,f32::from(sy)/image.size[1] as f32,f32::from(sx+w)/image.size[0] as f32,f32::from(sy+h)/image.size[1] as f32],"order":[0,id&65535]}}));
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoded_comment_wrap_preserves_a_crossing_native_pair() {
        let (first, tail) = split_encoded(&format!("{}あZ", "a".repeat(39)), 40, true).unwrap();
        assert_eq!(first, format!("{}あ", "a".repeat(39)));
        assert_eq!(tail, "Z");
    }
    #[test]
    fn log_controls_keep_icon_stamp_and_last_interval() {
        let (row, interval) = log_row("@t4abcd@t12", &[]).unwrap();
        assert_eq!(row.text, "@yabcd");
        assert_eq!(interval, Some(12));
        let (row, _) = log_row("@d9", &[]).unwrap();
        assert_eq!(row.stamp, Some(9));
        assert_eq!(row.text, "@y");
    }
    #[test]
    fn source_row_twenty_retains_native_control_overlay() {
        let mut c = NativeComments {
            backend: Some(2),
            raw_sources: vec![0; 16400],
            sources: vec![String::new(); 21],
            interval: 0x41424344,
            enabled: true,
            enabled_word: 1,
            ..Default::default()
        };
        c.write_raw_controls().unwrap();
        assert_eq!(
            c.native_record().unwrap()[1640..1644],
            [0x44, 0x43, 0x42, 0x41]
        );
        assert_eq!(c.sources[20], "DCBA");
        assert_eq!(c.raw_sources[1664..1668], [1, 0, 0, 0]);
    }
    #[test]
    fn appending_comment_preserves_six_rows_and_only_newest_reveals() {
        let mut c = NativeComments::default();
        for n in 0..8 {
            append_comment(&mut c, &n.to_string()).unwrap();
        }
        assert_eq!(c.rows.len(), 6);
        assert_eq!(c.rows[0].text, "@r2");
        assert!(c.rows.last().unwrap().reveal);
        assert!(c.rows[..5].iter().all(|r| !r.reveal));
    }
}
