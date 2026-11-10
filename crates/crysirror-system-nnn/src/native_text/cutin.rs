//! CCommonPrintMessage / CCutinNameData policy. The table, signed mode and
//! per-row selectors are native state; shared positioned glyphs, glyph views
//! and stored-byte solid blends own the actual drawing.
use super::*;

#[derive(Clone, Debug)]
struct Entry {
    name: Vec<u8>,
    origin: [i32; 2],
    next: [i32; 2],
    window: i32,
    padding: [i32; 2],
    color: [i32; 4],
    font_delta: i32,
}
impl Default for Entry {
    fn default() -> Self {
        Self {
            name: vec![0x83, 0x47, 0x83, 0x89, 0x81, 0x5b], // エラー
            origin: [0, 0],
            next: [0, 30],
            window: 1,
            padding: [0, 0],
            color: [128, 255, 192, 50],
            font_delta: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct CutinPolicy {
    entries: Vec<Entry>,
    print_name: bool,
    windows: BTreeMap<i32, Option<(String, [u32; 2])>>,
}
impl CutinPolicy {
    pub fn parse(names: &Names, print_name: bool) -> Result<Self> {
        ensure!(
            names.tokens.len() % 16 == 0,
            "NNN Cutin character table requires sixteen tokens per row"
        );
        let mut entries = Vec::new();
        for (index, tokens) in names.tokens.chunks_exact(16).enumerate() {
            // The source allocates row zero but deliberately parses only 1+.
            let mut entry = Entry::default();
            if index > 0 {
                entry.name = tokens[0].clone();
                entry.origin = [integer(&tokens[2])?, integer(&tokens[3])?];
                entry.next = [integer(&tokens[4])?, integer(&tokens[5])?];
                entry.window = integer(&tokens[6])?;
                entry.padding = [integer(&tokens[7])?, integer(&tokens[8])?];
                entry.color = [
                    integer(&tokens[9])?,
                    integer(&tokens[10])?,
                    integer(&tokens[11])?,
                    integer(&tokens[12])?,
                ];
                entry.font_delta = integer(&tokens[13])?;
            }
            entries.push(entry);
        }
        Ok(Self {
            entries,
            print_name,
            windows: BTreeMap::new(),
        })
    }
    pub(crate) fn window_numbers(&self) -> Result<std::collections::BTreeSet<i32>> {
        self.entries
            .iter()
            .filter(|e| e.window < 0)
            .map(|e| {
                e.window
                    .checked_neg()
                    .context("NNN Cutin window number overflow")
            })
            .collect()
    }
    pub(crate) fn bind_window(&mut self, number: i32, image: Option<(String, [u32; 2])>) {
        self.windows.insert(number, image);
    }
    pub(crate) fn font_sizes(&self, base: i32) -> Result<std::collections::BTreeSet<u16>> {
        self.entries
            .iter()
            .map(|e| {
                let size = base
                    .checked_add(e.font_delta)
                    .context("NNN Cutin font overflow")?;
                ensure!((1..=512).contains(&size), "NNN Cutin font size bounds");
                Ok(size as u16)
            })
            .collect()
    }
    fn entry(&self, selector: i32) -> Entry {
        usize::try_from(selector)
            .ok()
            .filter(|i| *i > 0)
            .and_then(|i| self.entries.get(i))
            .cloned()
            .unwrap_or_default()
    }
    pub(super) fn suppress_name(&self, selector: i32, row: &[u8]) -> bool {
        let entry = self.entry(selector);
        !self.print_name && (entry.name.first() == Some(&b'*') || entry.name == row)
    }
    pub(super) fn face_origin(
        &self,
        selector: i32,
        row: usize,
        body: &TextFace,
    ) -> Result<(TextFace, [i32; 2])> {
        let entry = self.entry(selector);
        let mut face = body.clone();
        face.size = u16::try_from(
            i32::from(body.size)
                .checked_add(entry.font_delta)
                .context("NNN Cutin font overflow")?,
        )?;
        ensure!((1..=512).contains(&face.size), "NNN Cutin font size bounds");
        // GetZahyo + GetNextZahyo * absolute message row, not group-local row.
        let mut point = entry.origin;
        for axis in 0..2 {
            point[axis] = i32::try_from(
                i64::from(point[axis]) + i64::from(entry.next[axis]) * i64::try_from(row)?,
            )?;
        }
        Ok((face, point))
    }
    pub(super) fn window(
        &self,
        selector: i32,
        rows: i32,
        cells: usize,
        profile: &MessageProfile,
        row: usize,
    ) -> Result<Option<Value>> {
        let entry = self.entry(selector);
        if entry.window < 0 {
            let number = entry
                .window
                .checked_neg()
                .context("NNN Cutin window number overflow")?;
            let image = self
                .windows
                .get(&number)
                .context("NNN Cutin image window has not been prepared")?;
            // LoadDWQ returning FALSE selects buffer-1: original Print has no pixels.
            let Some((path, size)) = image else {
                return Ok(None);
            };
            let percent = profile.cutin_window_percent;
            if percent == 0 {
                return Ok(None);
            }
            let mode = if percent == 100 {
                json!("mask_separate256")
            } else {
                json!({"mask_piecewise256":{"weight":percent*256/100}})
            };
            return Ok(Some(
                json!({"op":"put","layer":{"id":node(profile,row,true)?,"parent":profile.root,
                "rect":[i64::from(entry.origin[0])-i64::from(entry.padding[0]),i64::from(entry.origin[1])-i64::from(entry.padding[1]),size[0],size[1]],
                "order":[profile.order,10+row*2],"blend":{"stored_byte":{"mode":mode,"force_output_opaque":true}},
                "paint":{"kind":"image","asset":path}}}),
            ));
        }
        if entry.window == 0 {
            return Ok(None);
        }
        ensure!(
            (1..=64).contains(&rows),
            "NNN Cutin window row count bounds"
        );
        let font = i64::from(profile.body.size) + i64::from(entry.font_delta) + 1;
        let width = if entry.window > 1 {
            i64::from(entry.window)
        } else {
            font * i64::try_from(cells)?
                + i64::from(entry.padding[0]) * 2
                + i64::from(entry.next[0]) * i64::from(rows - 1)
        };
        let height =
            i64::from(entry.next[1]) * i64::from(rows - 1) + font + i64::from(entry.padding[1]) * 2;
        // Original clipped drawing has no pixels for nonpositive rectangles.
        if width <= 0 || height <= 0 {
            return Ok(None);
        }
        ensure!(
            width <= 32767 && height <= 32767,
            "NNN Cutin window dimensions budget"
        );
        let point = [
            i64::from(entry.origin[0]) - i64::from(entry.padding[0]),
            i64::from(entry.origin[1]) - i64::from(entry.padding[1]),
        ];
        let percent = entry.color[3].clamp(0, 100);
        if percent == 0 {
            return Ok(None);
        }
        let alpha = if percent == 100 {
            255
        } else {
            (percent * 256 / 100) as u8
        };
        let color = [
            entry.color[0] as u8,
            entry.color[1] as u8,
            entry.color[2] as u8,
            255,
        ];
        Ok(Some(
            json!({"op":"put","layer":{"id":node(profile,row,true)?,"parent":profile.root,
            "rect":[point[0],point[1],width,height],"opacity":alpha,"order":[profile.order,10+row*2],
            "blend":{"stored_byte":{"mode":"opaque_scalar256","force_output_opaque":true}},
            "paint":{"kind":"solid","color":color}}}),
        ))
    }
}
pub(super) fn node(profile: &MessageProfile, row: usize, window: bool) -> Result<u32> {
    ensure!(row < 64, "NNN Cutin row identity bounds");
    profile
        .body_layer
        .checked_add(128 + row as u32 * 2 + u32::from(!window))
        .context("NNN Cutin layer identity overflow")
}
pub(super) fn node_ids(profile: &MessageProfile) -> Result<Vec<u32>> {
    (0..64)
        .flat_map(|row| [node(profile, row, true), node(profile, row, false)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Metrics;
    impl TextMeasurement for Metrics {
        fn measure_simple(
            &self,
            _: &str,
            size: f32,
            _: &str,
        ) -> crysirror_core::Result<crysirror_core::service::TextExtent> {
            Ok(crysirror_core::service::TextExtent {
                advance: 0.,
                line_height: size,
                ascent: size * 0.75,
                descent: -size * 0.25,
            })
        }
    }
    fn fixture_profile() -> MessageProfile {
        let mut profile = MessageProfile::from_legacy_setup(
            &Names::parse(
                b"nameFlag 1 nameColorChangeFlag 0 autoNameWindowFlag 0 windowMessageNumber 8",
                false,
            )
            .unwrap(),
            MessageBindings {
                client: ClientId(1),
                root: 1,
                body_layer: 5000,
                name_layer: 5001,
                order: 1000,
                canvas: [800, 600],
                window: None,
            },
            "fixture",
            2,
            None,
            Some(Arc::from(
                (0x20u8..=0xdf).map(char::from).collect::<Vec<_>>(),
            )),
            1,
        )
        .unwrap();
        let table=Names::parse(b"zero tag 9 9 9 9 9 9 9 9 9 9 9 9 0 0 Alice tag 100 50 2 30 1 3 4 128 255 192 50 2 0 0 * tag 20 40 0 24 80 2 2 255 0 0 100 0 0 0",false).unwrap();
        profile.cutin = Some(Arc::new(CutinPolicy::parse(&table, false).unwrap()));
        profile.cutin_mode = 1;
        profile
    }
    #[test]
    fn native_cutin_message_stamps_retained_rows_uses_absolute_coordinates_and_shared_window_blend()
    {
        let profile = fixture_profile();
        let call = NativeCall {
            serial: 1,
            pc: 10,
            next: 20,
            identify: 0x66660006,
            code: 0x22220001,
            payload: Payload::Message {
                kind: 0x22220001,
                serial: 7,
                cutin: 1,
                text: b"Alice\r\nAB\r\nC".to_vec(),
            },
        };
        let plan = NativeMessageState::default()
            .prepare(&call, &profile, &Metrics)
            .unwrap();
        assert_eq!(plan.next.rows, vec![b"AB".to_vec(), b"C".to_vec()]);
        assert_eq!(&plan.next.cutin_rows[..2], &[[1, 2], [1, 1]]);
        assert!(plan.next.cutin_rows[2..].iter().all(|row| *row == [0; 2]));
        let nodes = plan.batch["layers"].as_array().unwrap();
        let get = |id| nodes.iter().find(|n| n["layer"]["id"] == id).unwrap()["layer"].clone();
        let master: LayerSpec = serde_json::from_value(get(5000)).unwrap();
        assert!(!master.visible);
        let crysirror_scene::layer::Paint::GlyphRun { glyphs, .. } = master.paint else {
            panic!()
        };
        assert_eq!(glyphs[0].baseline, [100., 69.5]);
        assert_eq!(glyphs[2].baseline, [102., 99.5]);
        assert_eq!(glyphs[0].size, Some(26.));
        let window: LayerSpec =
            serde_json::from_value(get(node(&profile, 0, true).unwrap())).unwrap();
        assert_eq!(window.rect, [97., 46., 62., 65.]);
        assert!(matches!(
            window.blend,
            crysirror_scene::layer::Blend::StoredByte {
                mode: crysirror_image::rgb_lut::ConstantByteBlend::OpaqueScalar256,
                ..
            }
        ));
        let view: LayerSpec =
            serde_json::from_value(get(node(&profile, 1, false).unwrap())).unwrap();
        assert!(matches!(
            view.paint,
            crysirror_scene::layer::Paint::GlyphView {
                source: 5000,
                range: Some([2, 3]),
                ..
            }
        ));
    }
    #[test]
    fn native_cutin_negative_mode_keeps_old_layout_without_stamping_new_rows_and_round_trips_save_seed()
     {
        let mut profile = fixture_profile();
        profile.cutin_mode = -1;
        let state = NativeMessageState {
            rows: vec![b"A".to_vec()],
            cutin_rows: vec![[1, 1]],
            ..Default::default()
        };
        let call = NativeCall {
            serial: 1,
            pc: 10,
            next: 20,
            identify: 0x66660006,
            code: 0x22220003,
            payload: Payload::Message {
                kind: 0x22220003,
                serial: 8,
                cutin: 2,
                text: b"B".to_vec(),
            },
        };
        let plan = state.prepare(&call, &profile, &Metrics).unwrap();
        assert_eq!(&plan.next.cutin_rows[..2], &[[1, 1], [0, 0]]);
        let saved = NativeMessageState::prepare_saved(
            plan.next.rows.clone(),
            0x22220001,
            8,
            true,
            &profile,
            &Metrics,
            plan.next.cutin_rows.clone(),
            vec![],
        )
        .unwrap();
        assert_eq!(saved.next.cutin_rows, plan.next.cutin_rows);
        let mut image = fixture_profile();
        image.cutin = Some(Arc::new(
            CutinPolicy::parse(
                &Names::parse(
                    b"zero t 0 0 0 0 0 0 0 0 0 0 0 0 0 0 Alice t 0 0 0 30 -1 0 0 0 0 0 0 0 0 0",
                    false,
                )
                .unwrap(),
                false,
            )
            .unwrap(),
        ));
        assert!(state.prepare(&call, &image, &Metrics).is_err());
    }
    #[test]
    fn native_cutin_hidden_ram_affects_live_window_but_native_save_only_writes_visible_rows() {
        let mut profile = fixture_profile();
        profile.window_rows = 1;
        let call = NativeCall {
            serial: 1,
            pc: 10,
            next: 20,
            identify: 0x66660006,
            code: 0x22220001,
            payload: Payload::Message {
                kind: 0x22220001,
                serial: 7,
                cutin: 1,
                text: b"Alice\r\nAB\r\nLONG".to_vec(),
            },
        };
        let plan = NativeMessageState::default()
            .prepare(&call, &profile, &Metrics)
            .unwrap();
        assert_eq!(plan.next.rows, vec![b"AB".to_vec()]);
        assert_eq!(plan.next.row_storage[1], b"LONG");
        assert_eq!(&plan.next.cutin_rows[..2], &[[1, 2], [1, 1]]);
        let window = |plan: &MessagePlan| {
            plan.batch["layers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["layer"]["id"] == node(&profile, 0, true).unwrap())
                .unwrap()["layer"]["rect"]
                .clone()
        };
        assert_eq!(window(&plan), json!([97, 46, 116, 65]));
        let native = crate::native_save::LegacySaveProfile {
            header_name: b"cutin".to_vec(),
            version: 1,
            extension_sizes: vec![],
            omake: false,
            cutin: true,
        };
        let mut file = crate::native_save::NativeSaveFile::for_live(
            &native,
            crate::native_system_file::SystemStamp([2026, 11, 10, 2, 0, 0, 0, 0]),
            crate::native_thumbnail::NativeThumbnail {
                size: [1, 1],
                bgrx: vec![0; 4],
            },
        )
        .unwrap();
        file.replace_live_wait(&crate::native_save_wait::SavedWait::Message {
            rows: plan.next.rows.clone(),
            mode: plan.next.mode,
            serial: 7,
            window_number: 0,
            window_visible: true,
            cannot_click: false,
        })
        .unwrap();
        assert!(file.messages(64).unwrap()[1].is_empty());
        let restored = NativeMessageState::prepare_saved(
            plan.next.rows.clone(),
            plan.next.mode,
            7,
            true,
            &profile,
            &Metrics,
            plan.next.cutin_rows.clone(),
            file.messages(64).unwrap(),
        )
        .unwrap();
        assert_eq!(window(&restored), json!([97, 46, 62, 65]));
        let mut bridge = NativeTextBridge::new(profile).unwrap();
        bridge.state = plan.next;
        let before = bridge.state.clone();
        bridge.accept_clear().unwrap();
        assert!(bridge.state.rows.is_empty());
        assert_eq!(bridge.state.row_storage, before.row_storage);
        assert_eq!(bridge.state.cutin_rows, before.cutin_rows);
    }
    #[test]
    fn native_cutin_lprint_scrolls_full_ram_and_metadata_before_clipping_visible_rows() {
        let mut profile = fixture_profile();
        profile.full_rows = 1;
        let call = NativeCall {
            serial: 1,
            pc: 10,
            next: 20,
            identify: 0x66660006,
            code: 0x22220002,
            payload: Payload::Message {
                kind: 0x22220002,
                serial: 7,
                cutin: 1,
                text: b"A\r\nB\r\nC".to_vec(),
            },
        };
        let plan = NativeMessageState::default()
            .prepare(&call, &profile, &Metrics)
            .unwrap();
        assert_eq!(plan.next.rows, vec![b"C".to_vec()]);
        assert_eq!(plan.next.row_storage[0], b"C");
        assert!(plan.next.row_storage[1..].iter().all(Vec::is_empty));
        assert_eq!(plan.next.cutin_rows[0], [1, 1]);
        assert!(plan.next.cutin_rows[1..].iter().all(|r| *r == [0; 2]));
    }
    #[test]
    fn native_cutin_image_window_uses_full_image_and_native_transparency_and_missing_file_is_inert()
    {
        let mut profile = fixture_profile();
        let names = Names::parse(
            b"zero t 0 0 0 0 0 0 0 0 0 0 0 0 0 0 Alice t 100 50 0 30 -2 3 4 0 0 0 0 0 0 0",
            false,
        )
        .unwrap();
        let mut policy = CutinPolicy::parse(&names, false).unwrap();
        assert_eq!(
            policy.window_numbers().unwrap(),
            std::collections::BTreeSet::from([2])
        );
        assert!(policy.window(1, 1, 1, &profile, 0).is_err());
        policy.bind_window(2, None);
        assert!(policy.window(1, 1, 1, &profile, 0).unwrap().is_none());
        policy.bind_window(2, Some(("fixture/window.png".into(), [80, 40])));
        profile.cutin_window_percent = 50;
        let node = policy.window(1, 1, 1, &profile, 0).unwrap().unwrap();
        let layer: LayerSpec = serde_json::from_value(node["layer"].clone()).unwrap();
        assert_eq!(layer.rect, [97., 46., 80., 40.]);
        assert!(matches!(
            layer.blend,
            crysirror_scene::layer::Blend::StoredByte {
                mode: crysirror_image::rgb_lut::ConstantByteBlend::MaskPiecewise256 { weight: 128 },
                ..
            }
        ));
        profile.cutin_window_percent = 100;
        let layer: LayerSpec = serde_json::from_value(
            policy.window(1, 1, 1, &profile, 0).unwrap().unwrap()["layer"].clone(),
        )
        .unwrap();
        assert!(matches!(
            layer.blend,
            crysirror_scene::layer::Blend::StoredByte {
                mode: crysirror_image::rgb_lut::ConstantByteBlend::MaskSeparate256,
                ..
            }
        ));
    }
}
