//! Native keyed-atlas weather presentation (DC2 417580, D.C.III 407300 /
//! 4087A0): each atlas's FF00FF key becomes transparent once (shared image
//! composition), then each sprite is an engine image Layer with an atlas
//! crop and the native /16 weight as opacity, grouped per painter pass.
use super::*;
use crate::dc2::WeatherFrame;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageJob, ImageProcessState};
const ROOT: u32 = 0x3c03_0000;
///Group g occupies ROOT + 1 + g * GROUP_SPAN; its sprites follow it.
const GROUP_SPAN: u32 = 0x1000;
const MAX_GROUPS: usize = 4;
#[derive(Default)]
pub(super) struct Weather {
    ///Source path -> (keyed atlas, size).
    keyed: BTreeMap<String, (UploadedImage, [u32; 2])>,
    jobs: BTreeMap<String, ImageJob>,
    ///Sprite layer count of each published pass.
    layers: Vec<usize>,
}
impl Bridge<'_> {
    ///Keyed copy of `source`, or None while its composition job runs.
    fn weather_atlas(&mut self, source: &str, publish_id: u32) -> Result<Option<(UploadedImage, [u32; 2])>> {
        if let Some(found) = self.services.dc2_weather.keyed.get(source) {
            return Ok(Some(found.clone()));
        }
        if !self.services.dc2_weather.jobs.contains_key(source) {
            let image = self
                .host
                .published_image_resource(self.client, source)?
                .context("Circus weather atlas lease")?;
            let size = image.size();
            use crysirror_core::image_blend::{RgbaBlendArithmetic as A, RgbaBlendState, RgbaByteRounding as R};
            let input = ImageInput::Composite {
                base: Box::new(ImageInput::Solid { size, color: [0, 0, 0, 0] }),
                layers: vec![ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(image)),
                    source: [0, 0, i32::try_from(size[0])?, i32::try_from(size[1])?],
                    destination: [0, 0],
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::Normalized(RgbaBlendState {
                        arithmetic: A::Byte255 { source: R::Floor, destination: R::Floor },
                        source_color_key: Some([255, 0, 255]),
                        ..Default::default()
                    }),
                }],
            };
            let job = self.host.image_metadata(self.client)?.begin_process(input, Arc::from(b"[]".as_slice()))?;
            self.services.dc2_weather.jobs.insert(source.to_owned(), job);
        }
        let job = self.services.dc2_weather.jobs.get_mut(source).unwrap();
        let image = match job.poll()? {
            ImageProcessState::Pending => return Ok(None),
            ImageProcessState::Failed(error) => bail!("Circus weather atlas keying failed: {error}"),
            ImageProcessState::Ready(image) => image,
        };
        self.services.dc2_weather.jobs.remove(source);
        let size = image.size();
        let published = self.host.publish_image_resource(self.client, publish_id, image)?;
        self.services.dc2_weather.keyed.insert(source.to_owned(), (published.clone(), size));
        Ok(Some((published, size)))
    }
    fn release_weather(&mut self) -> Result<()> {
        let passes: Vec<Value> = (0..self.services.dc2_weather.layers.len())
            .map(|g| json!({"op":"remove","id":ROOT + 1 + g as u32 * GROUP_SPAN}))
            .collect();
        if !passes.is_empty() {
            self.submit(passes, vec![])?;
        }
        for (_, (image, _)) in std::mem::take(&mut self.services.dc2_weather.keyed) {
            self.host.release_image_resource(self.client, image.path.relative())?;
        }
        self.services.dc2_weather = Default::default();
        Ok(())
    }
    pub(super) fn dc2_weather_frame(&mut self, frame: Option<&WeatherFrame>) -> Result<bool> {
        let Some(frame) = frame else {
            self.release_weather()?;
            return Ok(true);
        };
        ensure!(frame.groups.len() <= MAX_GROUPS, "Circus weather pass count");
        let mut atlases = Vec::new();
        for (index, group) in frame.groups.iter().enumerate() {
            let source = self
                .services
                .images
                .get(&group.texture_slot)
                .context("Circus weather atlas slot")?
                .path
                .relative()
                .to_owned();
            let Some(atlas) = self.weather_atlas(&source, ROOT + 0x8000 + index as u32)? else {
                return Ok(false);
            };
            atlases.push(atlas);
        }
        //Drop keyed atlases whose source is no longer referenced.
        let used: BTreeSet<String> = atlases.iter().map(|(image, _)| image.path.relative().to_owned()).collect();
        let stale: Vec<String> = self
            .services
            .dc2_weather
            .keyed
            .iter()
            .filter(|(_, (image, _))| !used.contains(image.path.relative()))
            .map(|(source, _)| source.clone())
            .collect();
        for source in stale {
            if let Some((image, _)) = self.services.dc2_weather.keyed.remove(&source) {
                self.host.release_image_resource(self.client, image.path.relative())?;
            }
        }
        //The passes draw into the native framebuffer before the window
        //records, so they live in the captured scene root below them.
        self.root()?;
        let [cw, ch] = frame.canvas;
        let mut layers = Vec::new();
        let previous = std::mem::take(&mut self.services.dc2_weather.layers);
        for (g, (group, (atlas, size))) in frame.groups.iter().zip(&atlases).enumerate() {
            let group_id = ROOT + 1 + g as u32 * GROUP_SPAN;
            let opacity = (u32::from(group.weight.min(16)) * 255 / 16) as u8;
            layers.push(json!({"op":"put","layer":{"id":group_id,"parent":SCENE_ROOT,"rect":[0,0,cw,ch],"order":group.order,"paint":{"kind":"group"}}}));
            let [w, h] = size.map(|v| v as f32);
            for (index, (src, dst)) in group.sprites.iter().enumerate() {
                let uv = [src[0] as f32 / w, src[1] as f32 / h, (src[0] + src[2]) as f32 / w, (src[1] + src[3]) as f32 / h];
                layers.push(json!({"op":"put","layer":{
                    "id":group_id+1+index as u32,"parent":group_id,"rect":[dst[0],dst[1],src[2],src[3]],
                    "order":[2,index as i32],"opacity":opacity,
                    "paint":{"kind":"image","asset":atlas.path.relative()},"uv":uv,
                    "sampling":{"linear":false},"filter":{"encoded_sampling":true}}}));
            }
            ensure!(group.sprites.len() < GROUP_SPAN as usize - 1, "Circus weather pass size");
            for index in group.sprites.len()..previous.get(g).copied().unwrap_or(0) {
                layers.push(json!({"op":"remove","id":group_id+1+index as u32}));
            }
            self.services.dc2_weather.layers.push(group.sprites.len());
        }
        for g in frame.groups.len()..previous.len() {
            layers.push(json!({"op":"remove","id":ROOT + 1 + g as u32 * GROUP_SPAN}));
        }
        self.submit(layers, vec![])?;
        Ok(true)
    }
}
