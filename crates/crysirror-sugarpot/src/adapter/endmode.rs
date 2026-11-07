// SPDX-License-Identifier: MPL-2.0
//!CEndMode float6 is the ending-information viewer, not process exit.
//!423080 loads endm_bk.bmp;423330 paints five integer-unlock ending rows and
//!one heading.423130/4231B0 close on mouse activation, restore the caller's
//!captured presentation, and leave VM variables/music/text untouched.
use super::*;
use crysirror_core::image_processing::ImageResource;
const OVERLAY: u32 = 63099;
const ROOT: u32 = 63000;
pub(super) struct EndMode {
    background: UploadedImage,
    old: Option<Arc<dyn ImageResource>>,
    new: Option<Arc<dyn ImageResource>>,
    capture: Option<ImageJob>,
    process: Option<ImageJob>,
    overlay: Option<UploadedImage>,
    stage: u8,
    weight: u32,
    deadline: u64,
    active: bool,
}
impl EndMode {
    pub(super) fn preparing(&self) -> bool {
        self.capture.is_some() || self.process.is_some()
    }
}
impl SugarPotVm {
    pub(super) fn end_mode(&mut self, h: &mut dyn HostServices) -> Result<bool> {
        if self.end_mode.is_none() {
            let Some(background) = self.image(h, "endm_bk.bmp")? else {
                return Ok(false);
            };
            self.end_mode = Some(EndMode {
                background,
                old: None,
                new: None,
                capture: Some(h.begin_layer_prefix_image_over(
                    self.client,
                    [1024, 600],
                    [9999, 0],
                    [0, 0, 0, 255],
                )?),
                process: None,
                overlay: None,
                stage: 0,
                weight: 224,
                deadline: 0,
                active: false,
            });
        }
        let mut mode = self.end_mode.take().unwrap();
        if mode.old.is_none() {
            match mode
                .capture
                .as_mut()
                .context("SugarPot end capture")?
                .poll()?
            {
                ImageProcessState::Pending => {
                    self.end_mode = Some(mode);
                    return Ok(false);
                }
                ImageProcessState::Failed(e) => bail!("SugarPot ending viewer capture: {e}"),
                ImageProcessState::Ready(resource) => {
                    mode.old = Some(resource);
                    mode.capture = None;
                }
            }
        }
        self.submit(h,vec![],vec![json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1024,600],"z":11000,
            "kind":"panel","color":[0,0,0,0],"modal":true,"key_event_scope":true,"keyboard_navigation":false,
            "key_events":[{"buttons":buttons::MOUSE_LEFT|buttons::MOUSE_RIGHT|buttons::ESCAPE|buttons::ENTER|buttons::SPACE,"event":"end_close"}]}})])?;
        self.wait = Some(Wait::EndMode);
        self.end_mode = Some(mode);
        Ok(true)
    }
    fn paint_end_mode(&mut self, h: &mut dyn HostServices, mode: &EndMode) -> Result<()> {
        let mut layers = vec![
            json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1024,600],"order":[9000,0],"paint":{"kind":"image","asset":mode.background.path.relative()}}}),
        ];
        let names = [
            " 穂香エンド",
            " 美琴エンド",
            " つくねエンド",
            " つみれエンド",
            " ノーマルエンド",
            "エンディング情報画面",
        ];
        let ys = [98, 126, 154, 196, 224, 52];
        for i in 0..6 {
            let (x, w) = if i < 5 { (64, 370) } else { (40, 320) };
            let opened = i == 5
                || self
                    .globals
                    .iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case(&format!("end{:02}", i + 1)))
                    .is_some_and(|(_, value)| matches!(value, Value::Integer(1)));
            let label = if opened { names[i] } else { "　Unknown" };
            //423330 draws a one-pixel black left and bottom edge, not a
            //replacement generic settings panel or an invented exit button.
            layers.push(json!({"op":"put","layer":{"id":ROOT+1+i as u32*3,"rect":[x,ys[i],1,24],"order":[9001,i*3],"paint":{"kind":"solid","color":[0,0,0,255]}}}));
            layers.push(json!({"op":"put","layer":{"id":ROOT+2+i as u32*3,"rect":[x,ys[i]+24,w,1],"order":[9001,i*3+1],"paint":{"kind":"solid","color":[0,0,0,255]}}}));
            let mut cursor = crysirror_text::grid::CellCursor::at_start();
            let layout = crysirror_text::grid::layout_cells(
                label,
                &crysirror_text::grid::CellLayoutOptions { cell_widths: Vec::new(),
                    bounds: [w - 4, 24],
                    size: 18,
                    spacing: [0, 0],
                    half_width: true,
                    half_spacing: false,
                    hanging: 0,
                    forbidden_line_start: String::new(),
                    indent_pairs: vec![],
                },
                &mut cursor,
            )?;
            let glyphs=layout.glyphs.iter().map(|g|json!({"character":g.character,"baseline":[x+4+g.position[0],ys[i]+4+g.position[1]+18]})).collect::<Vec<_>>();
            layers.push(json!({"op":"put","layer":{"id":ROOT+3+i as u32*3,"rect":[0,0,1024,600],"order":[9001,i*3+2],"paint":{"kind":"glyph_run","font":self.config.font,"size":18,"color":[0,0,0,255],"glyphs":glyphs}}}));
        }
        self.submit(h, layers, vec![])
    }
    pub(super) fn poll_end_mode(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
    ) -> Result<bool> {
        let mut mode = self
            .end_mode
            .take()
            .context("SugarPot ending viewer state")?;
        let result = self.end_mode_step(h, input, &mut mode);
        if !result.as_ref().is_ok_and(|done| *done) {
            self.end_mode = Some(mode);
        }
        result
    }
    fn end_overlay(
        &mut self,
        h: &mut dyn HostServices,
        mode: &mut EndMode,
        resource: Arc<dyn ImageResource>,
    ) -> Result<()> {
        let image = h.publish_image_resource(self.client, self.next_image, resource)?;
        self.next_image = self
            .next_image
            .checked_add(1)
            .context("SugarPot ending image ID")?;
        self.submit(h,vec![json!({"op":"put","layer":{"id":OVERLAY,"rect":[0,0,1024,600],"order":[10000,0],"paint":{"kind":"image","asset":image.path.relative()}}})],vec![])?;
        if let Some(old) = mode.overlay.replace(image) {
            h.release_image_resource(self.client, old.path.relative())?;
        }
        Ok(())
    }
    fn end_mode_step(
        &mut self,
        h: &mut dyn HostServices,
        input: &mut InputFrame,
        mode: &mut EndMode,
    ) -> Result<bool> {
        let events: Vec<Json> =
            serde_json::from_slice(&h.query_engine(self.client, EngineQuery::UiEvents)?)?;
        let owned_activate = events.iter().any(|event| {
            event["widget"]["owner"].as_u64() == Some(self.client.0 as u64)
                && event["widget"]["id"].as_u64() == Some(ROOT as u64)
                && event["kind"] == "end_close"
        });
        let activate = owned_activate
            || input.buttons_pressed
                & (buttons::MOUSE_LEFT
                    | buttons::MOUSE_RIGHT
                    | buttons::ESCAPE
                    | buttons::ENTER
                    | buttons::SPACE)
                != 0;
        if mode.active {
            if !activate {
                return Ok(false);
            }
            input.buttons_pressed = 0;
            mode.active = false;
            mode.stage = 2;
            mode.weight = 224;
            mode.deadline = 0;
        }
        if let Some(job) = &mut mode.process {
            match job.poll()? {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(e) => bail!("SugarPot ending palette: {e}"),
                ImageProcessState::Ready(resource) => {
                    mode.process = None;
                    self.end_overlay(h, mode, resource)?;
                    mode.deadline = h.now_ms().saturating_add(15);
                    mode.weight = if mode.stage % 2 == 0 {
                        mode.weight.saturating_sub(32)
                    } else {
                        mode.weight + 32
                    };
                    return Ok(false);
                }
            }
        }
        if activate {
            input.buttons_pressed = 0;
            mode.weight = if mode.stage % 2 == 0 { 0 } else { 256 };
        }
        if self.settings.header[4] != 0 {
            mode.weight = if mode.stage % 2 == 0 { 0 } else { 256 };
        }
        if h.now_ms() < mode.deadline {
            return Ok(false);
        }
        if mode.stage % 2 == 0 && mode.weight == 0 {
            //The original exact black terminal plane precedes new-scene paint.
            self.submit(h,vec![json!({"op":"put","layer":{"id":OVERLAY,"rect":[0,0,1024,600],"order":[10000,0],"paint":{"kind":"solid","color":[0,0,0,255]}}})],vec![])?;
            if mode.stage == 0 {
                self.paint_end_mode(h, mode)?;
                if mode.capture.is_none() {
                    mode.capture = Some(h.begin_layer_prefix_image_over(
                        self.client,
                        [1024, 600],
                        [9999, 0],
                        [0, 0, 0, 255],
                    )?);
                }
                match mode.capture.as_mut().unwrap().poll()? {
                    ImageProcessState::Pending => return Ok(false),
                    ImageProcessState::Failed(e) => bail!("SugarPot ending text capture: {e}"),
                    ImageProcessState::Ready(resource) => {
                        mode.new = Some(resource);
                        mode.capture = None;
                    }
                }
            } else {
                self.submit(
                    h,
                    (ROOT..=ROOT + 18)
                        .map(|id| json!({"op":"remove","id":id}))
                        .collect(),
                    vec![],
                )?;
            }
            mode.stage += 1;
            mode.weight = 32;
        }
        if mode.stage % 2 == 1 && mode.weight >= 256 {
            //Restore the unmodified, fully bright endpoint before returning.
            let resource = if mode.stage == 1 {
                mode.new.as_ref()
            } else {
                mode.old.as_ref()
            }
            .unwrap()
            .clone();
            self.end_overlay(h, mode, resource)?;
            if mode.stage == 1 {
                mode.active = true;
                return Ok(false);
            }
            self.submit(
                h,
                vec![json!({"op":"remove","id":OVERLAY})],
                vec![json!({"op":"remove","id":ROOT})],
            )?;
            if let Some(image) = mode.overlay.take() {
                h.release_image_resource(self.client, image.path.relative())?;
            }
            return Ok(true);
        }
        let resource = if mode.stage == 0 || mode.stage == 3 {
            mode.old.as_ref()
        } else {
            mode.new.as_ref()
        }
        .unwrap()
        .clone();
        let lut =
            crysirror_image::rgb_lut::RgbLookup::weighted_color_fixed([0; 3], mode.weight, 0, 8)?;
        mode.process = Some(
            h.image_metadata(self.client)?.begin_process(
                ImageInput::Resource(resource),
                serde_json::to_vec(&vec![crysirror_image::effects::ImageEffect::RgbLookup {
                    spec: lut,
                }])?
                .into(),
            )?,
        );
        Ok(false)
    }
}
