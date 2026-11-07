//! Message glyph pictures (TImoMessage+DAC, 69B084): characters drawn from
//! picture sheets instead of the font.
//!
//! - `glpi` (69B264) resets offset 0,0 and design size 32,32 and empties the
//!   sheet and glyph tables.
//! - `glpo,x,y` (69D370) sets the design offset; `glps,w,h` (69D380) the
//!   design size, ignored unless both are >= 1.
//! - `glpf,file,id` (69B1A0) loads sheet `id`, replacing an existing one.
//! - `glpa,chars,id,x,y,w,h,ax,ay[,n]` (69B440/69B55C) maps `n` (>= 1)
//!   consecutive scalars from the first one to sheet cells, advancing x by
//!   w and wrapping to the next row of cells when x+w passes the sheet
//!   width. Each cell must lie inside a sheet of at least 2x2 pixels.
//!   A failed load or mapping reports the native script error.
//! - Drawing (69B7F4): the cell is stretched over the character box
//!   (font width x height) moved by trunc(offset * box / design + .5) and
//!   tinted by the text colour. Layout (69B77C) advances by
//!   trunc(ax * font width / design width + .5).
//! - `gt` (6A0F58) answers 1 when the text added since the page was cleared
//!   has a scalar outside `[...]` tags.
use super::*;
use crysirror_core::image_processing::ImageProcessState;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Sheet {
    pub id: i32,
    pub source: String,
    /// Resolved asset path of `source`, reloaded after a restore.
    #[serde(default)]
    pub path: String,
    /// Published resource path and pixel size. A restored entry keeps the
    /// saved path only to re-point its glyphs; `live` is false until the
    /// sheet has been published again.
    #[serde(default)]
    pub image: Option<(String, [u32; 2])>,
    #[serde(skip)]
    pub live: bool,
}
#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) struct Cell {
    pub sheet: i32,
    pub rect: [i16; 4],
    pub advance: [i16; 2],
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct GlyphPictures {
    pub offset: [i32; 2],
    pub design: [i32; 2],
    pub sheets: Vec<Sheet>,
    pub cells: BTreeMap<char, Cell>,
}
impl Default for GlyphPictures {
    fn default() -> Self {
        Self {
            offset: [0, 0],
            design: [32, 32],
            sheets: Vec::new(),
            cells: BTreeMap::new(),
        }
    }
}
impl GlyphPictures {
    fn sheet(&self, id: i32) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.id == id)
    }
    fn drawable(&self, c: char) -> Option<(Cell, &(String, [u32; 2]))> {
        let cell = *self.cells.get(&c)?;
        let sheet = self.sheet(cell.sheet).filter(|s| s.live)?;
        Some((cell, sheet.image.as_ref()?))
    }
    pub(super) fn all_live(&self) -> bool {
        self.sheets.iter().all(|s| s.live || s.path.is_empty())
    }
    /// Layout widths of mapped scalars for a font of `font_width` pixels.
    pub(super) fn cell_widths(&self, font_width: i32) -> Vec<(char, i32)> {
        let design = self.design[0].max(1) as f64;
        self.cells
            .iter()
            .filter(|(c, _)| self.drawable(**c).is_some())
            .map(|(c, cell)| {
                let w = (f64::from(cell.advance[0]) * f64::from(font_width) / design + 0.5).trunc();
                (*c, (w as i32).clamp(0, 4096))
            })
            .collect()
    }
    /// The picture cell for a character box at `top_left`.
    pub(super) fn image(
        &self,
        c: char,
        top_left: [i32; 2],
        font: [i32; 2],
        tint: [u8; 4],
    ) -> Option<crysirror_scene::layer::GlyphImage> {
        let (cell, (asset, size)) = self.drawable(c)?;
        let scale = |offset: i32, box_size: i32, design: i32| {
            (f64::from(offset) * f64::from(box_size) / f64::from(design.max(1)) + 0.5).trunc() as f32
        };
        let [x, y, w, h] = cell.rect.map(f32::from);
        let (sw, sh) = (size[0] as f32, size[1] as f32);
        Some(crysirror_scene::layer::GlyphImage {
            asset: asset.clone(),
            rect: [
                top_left[0] as f32 + scale(self.offset[0], font[0], self.design[0]),
                top_left[1] as f32 + scale(self.offset[1], font[1], self.design[1]),
                font[0].max(0) as f32,
                font[1].max(0) as f32,
            ],
            uv: [x / sw, y / sh, (x + w) / sw, (y + h) / sh],
            tint,
        })
    }
}
/// 6A0D68.
pub(super) fn has_text(text: &str) -> bool {
    let mut depth = 0i32;
    for c in text.chars() {
        if c == '[' {
            depth += 1;
        }
        if c == ']' {
            depth -= 1;
        }
        depth = depth.max(0);
        if depth == 0 && c != '[' && c != ']' {
            return true;
        }
    }
    false
}
pub(super) struct SheetJob {
    owner: (String, i32, String),
    load: Option<Task<resources::Loaded>>,
    decode: Option<crysirror_core::image_processing::ImageJob>,
}

impl QlieVm {
    /// Loads and publishes one sheet; None while pending.
    fn poll_sheet(
        &mut self,
        h: &mut dyn HostServices,
        id: &str,
        sheet: i32,
        path: &str,
    ) -> Result<Option<(String, [u32; 2])>> {
        let owner = (id.to_owned(), sheet, path.to_owned());
        if self.glyph_sheet.is_none() {
            let storage = h.storage(self.client)?;
            let client = self.client;
            let source = path.to_owned();
            self.glyph_sheet = Some(SheetJob {
                owner: owner.clone(),
                load: Some(self.pool.spawn(move |cancel| resources::load(storage, client, &source, &cancel))?),
                decode: None,
            });
        }
        let job = self.glyph_sheet.as_mut().unwrap();
        ensure!(job.owner == owner, "QLIE glyph picture completion owner");
        if let Some(load) = &mut job.load {
            let Some(loaded) = load.try_take()? else {
                return Ok(None);
            };
            job.load = None;
            let resources::Loaded::Blob(bytes) = loaded else {
                bail!("QLIE glyph picture sheet is an animation: {path}");
            };
            job.decode = Some(h.image_metadata(self.client)?.begin_encoded(path, bytes)?);
        }
        let decode = job.decode.as_mut().context("QLIE glyph picture decode")?;
        let image = match decode.poll()? {
            ImageProcessState::Pending => return Ok(None),
            ImageProcessState::Failed(e) => bail!("QLIE glyph picture sheet {path}: {e}"),
            ImageProcessState::Ready(image) => image,
        };
        self.glyph_sheet = None;
        self.next_image = self.next_image.checked_add(1).context("QLIE image id exhausted")?;
        let published = h.publish_image_resource(self.client, self.next_image, image)?;
        Ok(Some((published.path.relative().to_owned(), published.size)))
    }
    /// After a restore: republish every saved sheet and re-point the stashed
    /// picture glyphs. False while a sheet is loading.
    pub(super) fn rebuild_picture_sheets(&mut self, h: &mut dyn HostServices, id: &str) -> Result<bool> {
        loop {
            let Some(message) = self.objects[id].message.as_ref() else { return Ok(true) };
            let Some((index, sheet, path)) = message
                .pictures
                .sheets
                .iter()
                .enumerate()
                .find(|(_, s)| !s.live && !s.path.is_empty())
                .map(|(i, s)| (i, s.id, s.path.clone()))
            else {
                break;
            };
            let Some(image) = self.poll_sheet(h, id, sheet, &path)? else {
                return Ok(false);
            };
            let message = self.objects.get_mut(id).unwrap().message.as_mut().unwrap();
            let entry = &mut message.pictures.sheets[index];
            if let Some((old, _)) = entry.image.replace(image.clone()) {
                message.remap.insert(old, image.0);
            }
            entry.live = true;
        }
        let message = self.objects.get_mut(id).unwrap().message.as_mut().unwrap();
        for (index, mut image) in std::mem::take(&mut message.stashed_images) {
            if let Some(asset) = message.remap.get(&image.asset) {
                image.asset = asset.clone();
                if let Some(glyph) = message.glyphs_mut().get_mut(index) {
                    glyph.image = Some(image);
                }
            }
        }
        message.remap.clear();
        Ok(true)
    }
    fn pictures(&mut self, id: &str) -> Result<&mut GlyphPictures> {
        Ok(&mut self
            .objects
            .get_mut(id)
            .unwrap()
            .message
            .as_mut()
            .context("QLIE glyph picture message state")?
            .pictures)
    }
    /// None when `op` is not one of these msg methods; Some(false) while a
    /// sheet is still loading.
    pub(super) fn glyph_picture_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<Option<bool>> {
        if !matches!(op, "glpi" | "glpo" | "glps" | "glpf" | "glpa" | "gt") {
            return Ok(None);
        }
        if self.objects[id].message.is_none() {
            let layer = self.next_layer;
            self.next_layer += 1;
            self.objects.get_mut(id).unwrap().message =
                Some(message::Message::new(layer, self.config.canvas));
        }
        let n = |i: usize| -> Result<i32> {
            a.get(i).map(Value::integer).transpose().map(|v| v.unwrap_or(0))
        };
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        match op {
            "glpi" => {
                let client = self.client;
                let old = std::mem::take(self.pictures(id)?);
                for sheet in old.sheets.into_iter().filter(|s| s.live) {
                    if let Some((path, _)) = sheet.image {
                        h.release_image_resource(client, &path)?;
                    }
                }
            }
            "glpo" => {
                ensure!(a.len() == 2, "QLIE glyph picture offset ABI");
                self.pictures(id)?.offset = [n(0)?, n(1)?];
            }
            "glps" => {
                ensure!(a.len() == 2, "QLIE glyph picture size ABI");
                if n(0)? >= 1 && n(1)? >= 1 {
                    self.pictures(id)?.design = [n(0)?, n(1)?];
                }
            }
            "glpf" => {
                ensure!(a.len() == 2, "QLIE glyph picture sheet ABI");
                let sheet = n(1)?;
                let source = s(0);
                let client = self.client;
                let p = self.pictures(id)?;
                ensure!(
                    p.sheets.len() < 4096 || p.sheet(sheet).is_some(),
                    "QLIE glyph picture sheet budget"
                );
                let path = if source.is_empty() { String::new() } else { self.resolve(m, &source)? };
                let image = if path.is_empty() {
                    None
                } else {
                    match self.poll_sheet(h, id, sheet, &path)? {
                        Some(image) => Some(image),
                        None => return Ok(Some(false)),
                    }
                };
                let entry = Sheet { id: sheet, source, path, live: true, image };
                let p = self.pictures(id)?;
                let old = match p.sheets.iter().position(|s| s.id == sheet) {
                    Some(i) => {
                        let old = std::mem::replace(&mut p.sheets[i], entry);
                        old.image.filter(|_| old.live)
                    }
                    None => {
                        p.sheets.push(entry);
                        None
                    }
                };
                if let Some((path, _)) = old {
                    h.release_image_resource(client, &path)?;
                }
            }
            "glpa" => {
                ensure!(a.len() <= 9, "QLIE glyph picture mapping ABI");
                let first = s(0).chars().next().context("QLIE glyph picture scalar")?;
                let sheet = n(1)?;
                let (mut x, mut y, w, h_) = (n(2)?, n(3)?, n(4)?, n(5)?);
                let advance = [n(6)? as i16, n(7)? as i16];
                let count = n(8)?.max(1);
                let p = self.pictures(id)?;
                let size = p
                    .sheet(sheet)
                    .and_then(|s| s.image.as_ref())
                    .map(|(_, size)| [size[0] as i32, size[1] as i32])
                    .context("QLIE glyph picture sheet absent")?;
                ensure!(size[0] >= 2 && size[1] >= 2, "QLIE glyph picture sheet size");
                ensure!(p.cells.len() <= 65536, "QLIE glyph picture budget");
                let mut code = u32::from(first);
                for _ in 0..count {
                    ensure!(
                        x >= 0 && y >= 0 && x + w <= size[0] && y + h_ <= size[1],
                        "QLIE glyph picture cell outside sheet"
                    );
                    let c = char::from_u32(code).context("QLIE glyph picture scalar")?;
                    p.cells.insert(
                        c,
                        Cell {
                            sheet,
                            rect: [x as i16, y as i16, w as i16, h_ as i16],
                            advance,
                        },
                    );
                    x += w;
                    if x + w > size[0] {
                        y += h_;
                        x = 0;
                    }
                    code = code.checked_add(1).context("QLIE glyph picture scalar")?;
                }
            }
            "gt" => {
                let text = &self.objects[id].message.as_ref().unwrap().text;
                if has_text(text) {
                    m.result(0, Value::Int(1));
                }
            }
            _ => unreachable!(),
        }
        Ok(Some(true))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn tags_are_not_text() {
        assert!(!super::has_text("[n][f,1]"));
        assert!(super::has_text("[n]a"));
        assert!(!super::has_text("]"));
        assert!(!super::has_text(""));
    }
}
