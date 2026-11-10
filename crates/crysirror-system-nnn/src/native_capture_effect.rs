// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! effectGet/effectPut/effectBuffer/simplewipe native parameters and painter
//! policy. Captures are immutable engine resources; all copies/crops/wipes run
//! through Image jobs. No native screen allocation or pixel loop is retained.
use crate::native_picture_effect::{PictureEffectFrame, PictureEffectKind};
use anyhow::{Result, ensure};
use crysirror_core::image_processing::{
    ImageCompositeLayer, ImageCompositeMath, ImageInput, ImageResource,
};
use std::sync::Arc;

#[derive(Clone, Default)]
pub(crate) struct FrameBuffers {
    pub capture: Option<Arc<dyn ImageResource>>,
    pub wipe_tables: crate::native_simple_wipe::Tables,
    pub picture_updates:std::collections::BTreeMap<i32,crate::native_picture_effect::PictureEffectState>,
    pub once_printed: std::collections::BTreeSet<i32>,
    pub rainbow_palette:Option<Arc<crate::native_picture_rainbow::RainbowPalette>>,
}
pub(crate) fn parameters(command: i32, p: &[i32]) -> Result<[i32; 16]> {
    // The C++ GET/PUT/BUFFER setters copy uninitialised trailing stack words
    // into saves. They are never read by these classes: use deterministic zero
    // on first installation, while preserving supplied saved words on restore.
    let mut out = [0; 16];
    let count = p.len().min(match command {
        59 => 9,
        66 => 3,
        _ => 16,
    });
    out[..count].copy_from_slice(&p[..count]);
    if p.is_empty() {
        out[0] = 20;
    }
    match command {
        59 => {
            ensure!(p.len() >= 2, "NNN SIMPLEWIPE requires frame and selector");
            // Source loops past its nine-word array when given extra words.
            // Only the declared nine parameters are part of the native ABI.
            out[1] = out[1].max(0);
            if p.len() < 3 {
                out[2] = 13;
            }
            if p.len() < 6 {
                out[5] = 100;
            }
        }
        2 => {
            if p.len() < 3 {
                out[2] = out[1];
            }
            if p.len() < 4 {
                out[3] = out[2];
            }
        }
        75 => {
            if p.len() < 3 {
                out[2] = 1;
            }
        }
        66 | 73 => {}
        _ => anyhow::bail!("NNN unknown capture effect {command}"),
    }
    Ok(out)
}
impl PictureEffectFrame {
    pub(crate) fn buffered_parameters(&self) -> Option<(i32, [i32; 16])> {
        match self.state.kind {
            PictureEffectKind::Buffered {
                command,
                parameters,
            } => Some((command, parameters)),
            _ => None,
        }
    }
    pub(crate) fn needs_capture(&self) -> bool {
        match self.buffered_parameters() {
            Some((2, p)) => self.state.count != 0 || p[4] != 0,
            Some((59, p)) => p[7] != 0 || p[8] != 0,
            _ => false,
        }
    }
    pub(crate) fn wipe_can_print(&self) -> bool {
        let Some((59, p)) = self.buffered_parameters() else {
            return false;
        };
        let fits = |image: &Option<Arc<dyn ImageResource>>| {
            image
                .as_ref()
                .is_some_and(|s| s.size()[0] >= self.canvas[0] && s.size()[1] >= self.canvas[1])
        };
        (p[8] != 0 || fits(&self.source)) && (p[7] != 0 || fits(&self.second))
    }
    pub(crate) fn wipe_inputs(
        &self,
        capture: &Option<Arc<dyn ImageResource>>,
    ) -> Result<(ImageInput, ImageInput)> {
        let (_, p) = self.buffered_parameters().unwrap();
        let source = if p[8] != 0 {
            capture.as_ref()
        } else {
            self.source.as_ref()
        }
        .ok_or_else(|| anyhow::anyhow!("NNN SIMPLEWIPE start picture absent"))?;
        let target = if p[7] != 0 {
            capture.as_ref()
        } else {
            self.second.as_ref()
        }
        .ok_or_else(|| anyhow::anyhow!("NNN SIMPLEWIPE end picture absent"))?;
        let from = if p[8] != 0 {
            [0, 0]
        } else {
            self.first_destination
        };
        let to = if p[7] != 0 {
            [0, 0]
        } else {
            self.second_destination
        };
        Ok((
            self.crop(source.clone(), from)?,
            self.crop(target.clone(), to)?,
        ))
    }
    fn crop(&self, source: Arc<dyn ImageResource>, destination: [i32; 2]) -> Result<ImageInput> {
        let size = source.size();
        ensure!(
            size[0] >= self.canvas[0] && size[1] >= self.canvas[1],
            "NNN wipe source extent"
        );
        // CEffectSimpleWipe negates destination, ClipAndCheck clamps the source
        // window to the full image. Use i64 to define INT_MIN negation safely.
        let x = (-i64::from(destination[0])).clamp(0, i64::from(size[0] - self.canvas[0])) as i32;
        let y = (-i64::from(destination[1])).clamp(0, i64::from(size[1] - self.canvas[1])) as i32;
        Ok(ImageInput::Composite {
            base: Box::new(ImageInput::Solid {
                size: self.canvas,
                color: [0, 0, 0, 255],
            }),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Resource(source)),
                source: [x, y, self.canvas[0] as i32, self.canvas[1] as i32],
                destination: [0, 0],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::CopyRgb,
            }],
        })
    }
    pub(crate) fn first_put_input(&self, current: Arc<dyn ImageResource>) -> ImageInput {
        let (_, p) = self.buffered_parameters().unwrap();
        if let Some(source) = &self.source {
            let size = source.size();
            ImageInput::Composite {
                base: Box::new(ImageInput::Resource(current)),
                layers: vec![ImageCompositeLayer {
                    input: Box::new(ImageInput::Resource(source.clone())),
                    source: [0, 0, size[0] as i32, size[1] as i32],
                    destination: self.first_destination,
                    clip: None,
                    opacity: 255,
                    math: ImageCompositeMath::CopyRgb,
                }],
            }
        } else {
            // FillScreen packs the native integer channels into RGB bytes.
            let r = p[1];
            let g = if p[2] == -1 { r } else { p[2] };
            let b = if p[3] == -1 { g } else { p[3] };
            let rgb = r.wrapping_shl(16) | g.wrapping_shl(8) | b;
            ImageInput::Solid {
                size: self.canvas,
                color: [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        native_geometry::NativeRects,
        native_picture_effect::PictureEffectState,
        native_presentation::{NativeFrameEffect, NativeFrameWork},
    };
    use crysirror_core::{
        image_processing::ImageProcessState, service::HostServices, state::ClientId,
    };
    use crysirror_image::temporal_pattern::PatternRng;
    use std::time::{Duration, Instant};
    fn frame(command: i32, p: &[i32]) -> PictureEffectFrame {
        PictureEffectFrame {
            slot: 0,
            state: PictureEffectState::parse(command, p).unwrap(),
            source: None,
            rectangles: NativeRects::natural([2, 1]),
            canvas: [2, 1],
            has_mask: false,
            source_clip:None,
            rainbow_palette:None,
            second_clip:None,
            motion_background:false,
            visible: true,
            geometry_assigned: false,
            first_slot: 0,
            second_slot: -1,
            first_destination: [0, 0],
            second: None,
            second_destination: [0, 0],
            normal: None,
        }
    }
    #[test]
    fn native_effect_translation_capture_put_and_buffer_wipe_keep_painter_order_and_native_counters()
     {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("assets")).unwrap();
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({"schema":2,"name":"capture","content_revision":"1","assets":"assets","canvas":[2,1],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            dir.path(),
            &dir.path().join("state"),
            Default::default(),
        )
        .unwrap();
        rt.tick(0, Default::default()).unwrap();
        let mut base = rt
            .host
            .image_metadata(ClientId(1))
            .unwrap()
            .begin_process(
                ImageInput::Solid {
                    size: [2, 1],
                    color: [10, 20, 30, 255],
                },
                b"[]".as_slice().into(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let image = loop {
            match base.poll().unwrap() {
                ImageProcessState::Ready(i) => break i,
                ImageProcessState::Failed(e) => panic!("{e}"),
                ImageProcessState::Pending => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
            }
        };
        let get = frame(73, &[20]);
        let fill = frame(2, &[20, 200, 100, 50]);
        let mut put = frame(2, &[20]);
        put.state.count = 1;
        let buffer = frame(66, &[20, 1]);
        let mut work = NativeFrameWork::with_context_base(
            ClientId(1),
            image.clone(),
            vec![
                NativeFrameEffect::Picture(get),
                NativeFrameEffect::Picture(fill),
                NativeFrameEffect::Picture(buffer),
                NativeFrameEffect::Picture(put),
            ],
            PatternRng::msvc(7),
            false,
            Default::default(),
            &mut rt.host,
        )
        .unwrap();
        let done = loop {
            if let Some(done) = work.poll(&mut rt.host).unwrap() {
                break done;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert_eq!(
            done.target.rgba().as_ref(),
            &[10, 20, 30, 255, 10, 20, 30, 255]
        );
        assert!(Arc::ptr_eq(done.buffers.capture.as_ref().unwrap(), &image));
        // Wipe start from the saved GET buffer, target is an ordinary picture;
        // countMax remains the authoritative saved counter, independent para0.
        let mut wipe = frame(59, &[999, 0, 13, 0, 0, 100, 0, 0, 1]);
        #[derive(Debug)]
        struct Target;
        impl ImageResource for Target {
            fn client(&self) -> ClientId {
                ClientId(1)
            }
            fn size(&self) -> [u32; 2] {
                [2, 1]
            }
            fn rgba(&self) -> Arc<[u8]> {
                [100, 100, 100, 255, 100, 100, 100, 255].as_slice().into()
            }
            fn opaque(&self) -> bool {
                true
            }
        }
        wipe.state.frames = 10;
        wipe.state.count = 5;
        wipe.second = Some(Arc::new(Target));
        let mut paint = frame(2, &[20, 100]);
        paint.source = None;
        let mut work = NativeFrameWork::with_context_base(
            ClientId(1),
            image,
            vec![
                NativeFrameEffect::Picture(paint),
                NativeFrameEffect::Picture(wipe.clone()),
            ],
            done.random,
            false,
            done.buffers,
            &mut rt.host,
        )
        .unwrap();
        let done = loop {
            if let Some(done) = work.poll(&mut rt.host).unwrap() {
                break done;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert_eq!(
            done.target.rgba().as_ref(),
            &[55, 60, 65, 255, 55, 60, 65, 255]
        );
        assert_eq!(
            (
                wipe.state.count,
                wipe.state.frames,
                wipe.state.parameters()[0]
            ),
            (5, 10, 999)
        );
        let mut once = frame(75, &[2, 1, 1]);
        once.normal = Some(crate::native_presentation::MoveFrame {
            source_clip:None,
            slot: 0,
            state: once.geometry(),
            source: Some(done.target.clone()),
            math: ImageCompositeMath::CopyRgb,
            visible: true,
            native_command: 0,
        });
        let mut buffers = done.buffers;
        for (count, expected) in [
            (0, [55, 60, 65, 255]),
            (0, [100, 100, 100, 255]),
            (1, [55, 60, 65, 255]),
        ] {
            once.state.count = count;
            let mut work = NativeFrameWork::with_context_base(
                ClientId(1),
                Arc::new(Target),
                vec![NativeFrameEffect::Picture(once.clone())],
                PatternRng::msvc(7),
                false,
                buffers,
                &mut rt.host,
            )
            .unwrap();
            let result = loop {
                if let Some(done) = work.poll(&mut rt.host).unwrap() {
                    break done;
                }
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            };
            assert_eq!(&result.target.rgba()[..4], &expected);
            buffers = result.buffers;
        }
        let mut add=frame(131,&[10,100]);add.source=Some(Arc::new(Target));
        let mut work=NativeFrameWork::with_context_base(ClientId(1),Arc::new(Target),vec![NativeFrameEffect::Picture(add)],PatternRng::msvc(7),false,buffers,&mut rt.host).unwrap();
        let result=loop{if let Some(done)=work.poll(&mut rt.host).unwrap(){break done;}assert!(Instant::now()<deadline);std::thread::yield_now();};
        assert_eq!(result.target.rgba().as_ref(),&[199,199,199,255,199,199,199,255]);
    }

    #[test]
    fn native_effect_translation_common_calcu_updates_hidden_target_before_wipe_lookup() {
        let mut target = frame(66, &[2]);
        target.slot = 13;
        target.visible = false;
        target.geometry_assigned = true;
        target.state.count = 1;
        target
            .rectangles
            .set_destination_pair([4, 0, 2, 1], [8, 0, 2, 1]);
        let mut wipe = frame(59, &[2, 0, 13]);
        wipe.second_slot = 13;
        let mut effects = std::collections::BTreeMap::from([
            (0, NativeFrameEffect::Picture(wipe)),
            (13, NativeFrameEffect::Picture(target)),
        ]);
        crate::native_presentation::calcu_effect_states(&mut effects, &mut PatternRng::msvc(7))
            .unwrap();
        let NativeFrameEffect::Picture(wipe) = &effects[&0] else {
            panic!()
        };
        assert_eq!(wipe.second_destination, [6, 0]);
        let visible = std::collections::BTreeSet::from([0]);
        assert_eq!(
            crate::native_presentation::ordered_frame_plan(&effects, &visible, &Default::default())
                .len(),
            2
        );
    }
    #[test]
    fn native_effect_translation_light_and_gradient_keep_native_word_and_row_bounds() {
        use crysirror_image::{ImageSurface, effects::ImageEffect};
        let mut light = frame(20, &[10, 20, -10, 0]);
        light.canvas = [1, 5];
        light.state.count = 10;
        let process = light
            .image_input(
                ImageInput::Solid {
                    size: [1, 5],
                    color: [40, 40, 40, 255],
                },
                &mut PatternRng::msvc(7),
            )
            .unwrap()
            .unwrap();
        let ImageInput::Process { effects, .. } = process else {
            panic!()
        };
        let effects: Vec<ImageEffect> = serde_json::from_slice(&effects).unwrap();
        let source = ImageSurface::filled(1, 5, [40, 40, 40, 255]).unwrap();
        let result = source.apply_effect(&effects[0]).unwrap();
        // Native CCalcuSpeed adds+0.5 before truncating signed values, so
        // the endpoint -10 is evaluated as -9 in this legacy ABI.
        assert_eq!(&result.rgba[..4], &[60, 31, 40, 255]);
        assert_eq!(&result.rgba[16..], &[40, 40, 40, 255]);
        let mut gradient = frame(73, &[]);
        gradient.canvas = [4, 2];
        gradient.state = PictureEffectState::parse_for_canvas(
            50,
            &[10, 10, 10, 10, 30, 30, 30],
            gradient.canvas,
        )
        .unwrap();
        gradient.state.count = 10;
        let ImageInput::Process { effects, .. } = gradient
            .image_input(
                ImageInput::Solid {
                    size: [4, 2],
                    color: [40, 40, 40, 255],
                },
                &mut PatternRng::msvc(7),
            )
            .unwrap()
            .unwrap()
        else {
            panic!()
        };
        let effects: Vec<ImageEffect> = serde_json::from_slice(&effects).unwrap();
        let result = ImageSurface::filled(4, 2, [40, 40, 40, 255])
            .unwrap()
            .apply_effect(&effects[0])
            .unwrap();
        assert_eq!(&result.rgba[..4], &[50, 50, 50, 255]);
        assert_eq!(&result.rgba[16..20], &[60, 60, 60, 255]);
        assert_eq!(gradient.state.parameters()[17], 0);
    }
    #[test]
    fn native_effect_translation_color_balance_retains_stored_band_discontinuity() {
        use crysirror_image::{
            ImageSurface,
            tonal_balance::{TonalBalance, TonalBand},
        };
        let state = PictureEffectState::parse(
            120,
            &[20, 128, 128, 128, 128, 128, 128, 1, 0, 0, 2, 2, 2, 0],
        )
        .unwrap();
        assert_eq!(state.command(), 120);
        assert_eq!(state.parameters().len(), 14);
        let lookup = TonalBalance {
            offsets: [128; 3],
            bands: [TonalBand::Midtones; 3],
            preserve_luminance: false,
        }
        .byte_lookup();
        let source = ImageSurface {
            width: 2,
            height: 1,
            rgba: vec![127, 127, 127, 128, 128, 128, 128, 255],
        };
        let result = lookup.apply(&source, &|| Ok(())).unwrap();
        assert_eq!(result.rgba, vec![254, 254, 254, 128, 192, 192, 192, 255]);
    }
}
