// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native static SystemMenu backdrop plan. Pixels stay in shared image jobs.
use super::*;
use crysirror_core::image_processing::{ImageCompositeLayer, ImageCompositeMath, ImageResource};
use crysirror_image::{effects::ImageEffect, integer_color::ColorMatrixFixed};
/// CGameCallBack::CheckExpModeByVar: later duplicate declarations win and
/// variable index zero (including a missing name) never disables a mode.
pub(super) fn mode_enabled(
    config: &crate::native_bootstrap::NativeConfiguration,
    variables: &crate::expression::Variables,
    mode: i32,
) -> Result<bool> {
    ensure!(
        (1..256).contains(&mode),
        "NNN system extended mode table bounds"
    );
    let count = config
        .setup
        .integer(b"expModeCheckByVarNumber")?
        .unwrap_or(0);
    ensure!(
        (0..=256).contains(&count),
        "NNN system extended mode gate budget"
    );
    let mut variable = None;
    for index in 1..=count {
        if config
            .setup
            .integer(format!("expModeCheckByVarMode{index}").as_bytes())?
            .unwrap_or(0)
            == mode
        {
            if let Some(name) = config
                .setup
                .parameter(format!("expModeCheckByVarName{index}").as_bytes())
            {
                variable = config.variable_names.index(name);
            }
        }
    }
    Ok(variable
        .filter(|id| *id > 0)
        .is_none_or(|id| variables.read_stored(id) != 0))
}
pub(super) fn has_save(
    config: &crate::native_bootstrap::NativeConfiguration,
    storage: &dyn ClientStorage,
    cancel: &Cancellation,
) -> Result<bool> {
    let load = native_load::table(config, storage, "load", cancel)?;
    let pages = load.integer(b"page")?.unwrap_or(10);
    let x = load.integer(b"blockNumberX")?.unwrap_or(4);
    let y = load.integer(b"blockNumberY")?.unwrap_or(3);
    let count = pages
        .checked_mul(x)
        .and_then(|n| n.checked_mul(y))
        .context("NNN load catalogue count overflow")?;
    ensure!(
        pages > 0 && x > 0 && y > 0 && (1..=10000).contains(&count),
        "NNN load catalogue count budget"
    );
    let trial = config.setup.integer(b"taikenFlag")?.unwrap_or(0) != 0;
    let prefix = if trial { "tdata" } else { "data" };
    // CCommonLoad::CheckExistSaveData only tests LoadHeaderAndPic's open-file
    // flag, not the separate CheckLoadOkVersion predicate. No thumbnails need
    // decoding merely to decide whether the Load control is enabled.
    // One directory snapshot avoids a remote open/roundtrip for every absent
    // slot. Still open candidate files: a directory with a .sav name is not a
    // successfully opened original save, and deletion can race the snapshot.
    let entries = match storage.list(Mount::Save, "") {
        Ok(entries) => entries
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        Err(Error::NotFound) => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    for slot in 0..count {
        cancel.check()?;
        let name = format!("{prefix}{slot}.sav");
        if !entries.contains(&name) {
            continue;
        }
        match storage.open_read(&VfsPath::new(Mount::Save, &name)?) {
            Ok(_) => return Ok(true),
            Err(Error::NotFound) => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(false)
}
pub(super) fn backdrop(
    table: &crate::config::Names,
    config: &crate::native_bootstrap::NativeConfiguration,
    resources: &NativeResources,
    captured: Arc<dyn ImageResource>,
    cancel: &Cancellation,
) -> Result<ImageInput> {
    let n = |key: &[u8], default| -> Result<i32> { Ok(table.integer(key)?.unwrap_or(default)) };
    let levels = u32::try_from(config.setup.integer(b"dwqPackLevel")?.unwrap_or(0))?;
    let mut input = if n(b"backScreenType", 2)? == 1 {
        let name = table
            .parameter(b"backScreenFileName")
            .unwrap_or(b"bg_sysmenu");
        resources.image(
            &config
                .resources
                .picture(&[b"sys/".as_slice(), name].concat(), levels)?,
            cancel,
        )?
    } else {
        ImageInput::Resource(captured)
    };
    if n(b"backScreenType", 2)? == 2 {
        let mut effects = vec![];
        if n(b"backScreenEffectNega", 0)? != 0 {
            effects.push(ImageEffect::Invert);
        }
        if n(b"backScreenEffectGrey", 0)? != 0 {
            effects.push(ImageEffect::FixedLuma {
                spec: crysirror_image::fixed_luma::FixedLumaSpec {
                    coefficients: [76, 150, 29],
                    groups: [1, 0, 0],
                    fractional_bits: 8,
                    amount256: 256,
                    tint: None,
                },
            });
        }
        if n(b"backScreenEffectMulti", 0)? != 0 {
            let r = n(b"backScreenMultiR", 100)?;
            let g = n(b"backScreenMultiG", 100)?;
            let b = n(b"backScreenMultiB", 100)?;
            let g = if g == -1 { r } else { g };
            let b = if b == -1 { g } else { b };
            let weights = [r, g, b].map(|v| v.clamp(0, 100) * 256 / 100);
            effects.push(ImageEffect::IntegerColorMatrix {
                spec: ColorMatrixFixed {
                    matrix: [[weights[0], 0, 0], [0, weights[1], 0], [0, 0, weights[2]]],
                    offset: [0; 3],
                    divisor: 256,
                },
            });
        }
        if n(b"backScreenEffectAddSub", 0)? != 0 {
            let r = n(b"backScreenColorR", 0)?;
            let g = n(b"backScreenColorG", 0)?;
            let b = n(b"backScreenColorB", 0)?;
            let g = if g == -9999 { r } else { g };
            let b = if b == -9999 { g } else { b };
            effects.push(ImageEffect::IntegerColorMatrix {
                spec: ColorMatrixFixed {
                    matrix: [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
                    offset: [r, g, b].map(|v| v.clamp(-255, 255)),
                    divisor: 1,
                },
            });
        }
        input = ImageInput::Process {
            input: Box::new(input),
            effects: serde_json::to_vec(&effects)?.into(),
        };
    }
    let name = table
        .parameter(b"filenameMenuBase")
        .unwrap_or(b"ta_menulistbase");
    if let Some(source) = resources.optional_image_source(
        &config
            .resources
            .picture(&[b"sys/".as_slice(), name].concat(), levels)?,
        cancel,
    )? {
        let size = source.size;
        let alpha = if source.has_mask {
            crysirror_image::sprite_alpha::SpriteAlpha::Preserve
        } else {
            crysirror_image::sprite_alpha::SpriteAlpha::CornerKey { right: false }
        };
        input = ImageInput::Composite {
            base: Box::new(input),
            layers: vec![ImageCompositeLayer {
                input: Box::new(ImageInput::Process {
                    input: Box::new(source.input),
                    effects: serde_json::to_vec(&vec![ImageEffect::SpriteAlpha { spec: alpha }])?
                        .into(),
                }),
                source: [0, 0, size[0] as i32, size[1] as i32],
                destination: [n(b"basePrintX", 536)?, n(b"basePrintY", 10)?],
                clip: None,
                opacity: 255,
                math: ImageCompositeMath::MaskSeparate256,
            }],
        };
    }
    Ok(input)
}
