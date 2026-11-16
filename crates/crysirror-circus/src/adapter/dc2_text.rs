//! DC2 message window and cells as shared retained layers inside the scene
//! root, above the weather group (layer order is global: weather uses major
//! 2, the window major 5), so feedback captures include them as in the
//! native framebuffer. Window parts of ARGB32 atlases are source-over image
//! layers whose opacity carries the 40C0E0 /16 weight; cells (421160/421290)
//! are one glyph run with each shadow drawn before its text.
use super::*;
use crate::dc2::TextFrame;
use crate::dc2_bitmap::Draw;
const ROOT: u32 = 0x3c04_0000;
const GLYPHS: u32 = ROOT + 0x100;
#[derive(Default)]
pub(super) struct Text {
    shown: bool,
    window: Vec<Draw>,
    ascent: BTreeMap<u8, f32>,
    /// Image key -> ARGB32 (40B5F0/40C0E0) display format.
    formats: BTreeMap<String, bool>,
}
impl Bridge<'_> {
    fn dc2_window_layers(&mut self, window: &[Draw]) -> Result<Vec<Value>> {
        let mut layers = Vec::new();
        for (index, part) in window.iter().enumerate() {
            let id = ROOT + 1 + index as u32;
            let (slot, source, destination, weight) = match part {
                Draw::Image { slot, source, destination } => (*slot, *source, *destination, None),
                Draw::Blend { slot, source, destination, weight } => (*slot, *source, *destination, Some(*weight)),
                Draw::Solid { rect, color } => {
                    layers.push(json!({"op":"put","layer":{"id":id,"parent":ROOT,"rect":rect,"order":[5,index as i32],"paint":{"kind":"solid","color":color}}}));
                    continue;
                }
                Draw::SolidBlend { rect, color, weight } => {
                    let opacity = (u32::from(*weight) * 255 / 16) as u8;
                    layers.push(json!({"op":"put","layer":{"id":id,"parent":ROOT,"rect":rect,"order":[5,index as i32],"opacity":opacity,"paint":{"kind":"solid","color":[color[0],color[1],color[2],255]}}}));
                    continue;
                }
            };
            let uploaded = self.services.images.get(&slot).context("DC2 window template image slot")?;
            let key = uploaded.path.relative().to_owned();
            let name = self.services.image_names.get(&slot).context("DC2 window template image name")?.clone();
            let argb = match self.services.dc2_text.formats.get(&key) {
                Some(argb) => *argb,
                None => {
                    let storage = self.services.storage.as_ref().context("DC2 window format storage")?.clone();
                    let argb = super::dc2_background::validate(storage.as_ref(), &name)?;
                    self.services.dc2_text.formats.insert(key.clone(), argb);
                    argb
                }
            };
            ensure!(argb, "DC2 FF00FF-keyed window template parts are not connected");
            let image = self
                .host
                .published_image_resource(self.client, &key)?
                .context("DC2 window template image lease")?;
            let [w, h] = image.size().map(|v| v as f32);
            let uv = [
                source[0] as f32 / w,
                source[1] as f32 / h,
                (source[0] + source[2]) as f32 / w,
                (source[1] + source[3]) as f32 / h,
            ];
            let opacity = weight.map_or(255, |weight| (u32::from(weight) * 255 / 16) as u8);
            layers.push(json!({"op":"put","layer":{
                "id":id,"parent":ROOT,"rect":[destination[0],destination[1],source[2],source[3]],
                "order":[5,index as i32],"opacity":opacity,
                "paint":{"kind":"image","asset":key},"uv":uv,
                "sampling":{"linear":false},"filter":{"encoded_sampling":true}}}));
        }
        Ok(layers)
    }
    pub(super) fn dc2_text_frame(&mut self, frame: Option<&TextFrame>) -> Result<bool> {
        let Some(frame) = frame else {
            if self.services.dc2_text.shown {
                self.submit(vec![json!({"op":"remove","id":ROOT})], vec![])?;
                self.services.dc2_text.shown = false;
                self.services.dc2_text.window.clear();
            }
            return Ok(true);
        };
        self.root()?;
        let mut layers = Vec::new();
        if !self.services.dc2_text.shown || self.services.dc2_text.window != frame.window {
            if self.services.dc2_text.shown {
                layers.push(json!({"op":"remove","id":ROOT}));
            }
            layers.push(json!({"op":"put","layer":{"id":ROOT,"parent":SCENE_ROOT,"rect":[0,0,800,600],"order":[5,0],"paint":{"kind":"group"}}}));
            layers.extend(self.dc2_window_layers(&frame.window)?);
            self.services.dc2_text.window = frame.window.clone();
        }
        let font = self.font.to_owned();
        let metrics = self.host.text_measurement(self.client)?;
        let mut shadows = Vec::new();
        let mut cells = Vec::new();
        for glyph in &frame.glyphs {
            let ascent = match self.services.dc2_text.ascent.get(&glyph.size) {
                Some(value) => *value,
                None => {
                    let value = metrics.measure_simple(&font, f32::from(glyph.size), " ")?.ascent;
                    self.services.dc2_text.ascent.insert(glyph.size, value);
                    value
                }
            };
            for character in glyph.text.chars() {
                let cell = if character.is_ascii() || ('\u{ff61}'..='\u{ff9f}').contains(&character) {
                    u16::from(glyph.size) / 2
                } else {
                    u16::from(glyph.size)
                };
                let [x, y] = glyph.position.map(|v| v as f32);
                if let Some(rgb) = glyph.shadow {
                    shadows.push(json!({"character":character.to_string(),"baseline":[x + 1., y + 1. + ascent],"size":glyph.size,"cell_width":cell,"color":[rgb[0],rgb[1],rgb[2],255]}));
                }
                let rgb = glyph.color;
                cells.push(json!({"character":character.to_string(),"baseline":[x, y + ascent],"size":glyph.size,"cell_width":cell,"color":[rgb[0],rgb[1],rgb[2],255]}));
            }
        }
        shadows.extend(cells);
        layers.push(json!({"op":"put","layer":{"id":GLYPHS,"parent":ROOT,"rect":[0,0,800,600],"paint":{"kind":"glyph_run","font":font,"size":28,"color":[255,255,255,255],"glyphs":shadows},"order":[5,1000]}}));
        self.submit(layers, vec![])?;
        self.services.dc2_text.shown = true;
        Ok(true)
    }
}
