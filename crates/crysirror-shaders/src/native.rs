// SPDX-License-Identifier: MPL-2.0
//! Native console source stages and a separately validated GXP program catalog.

pub const RAY_LITE_COMMON: &str = include_str!("../native/ray_lite.cgh");
pub const PSV_RAY_VERTEX: &str = include_str!("../native/psv/ray_lite_v.cg");
pub const PSV_RAY_FRAGMENT: &str = include_str!("../native/psv/ray_lite_f.cg");
pub const SWITCH_RAY_COMPUTE: &str = include_str!("../native/switch/ray_lite.comp.glsl");

pub struct Source {
    pub file: &'static str,
    pub entry: &'static str,
    pub stage: &'static str,
    pub source: String,
}

/// Expand only the engine-owned fixed include. Does not invoke a compiler.
pub fn ray_lite_sources(target: &str) -> Option<Vec<Source>> {
    let expand = |source: &str| source.replace("#include \"../ray_lite.cgh\"", RAY_LITE_COMMON);
    match target {
        "psv" => Some(vec![
            Source {
                file: "ray_lite_v.cg",
                entry: "main",
                stage: "vertex",
                source: PSV_RAY_VERTEX.into(),
            },
            Source {
                file: "ray_lite_f.cg",
                entry: "main",
                stage: "fragment",
                source: expand(PSV_RAY_FRAGMENT),
            },
        ]),
        "switch" => Some(vec![Source {
            file: "ray_lite.comp.glsl",
            entry: "main",
            stage: "compute",
            source: expand(SWITCH_RAY_COMPUTE),
        }]),
        _ => None,
    }
}

/// PSV reduced 3D tier (crysirror-render-tier `Tier::Vita`): native Cg for GXM.
/// Includes are engine-owned and expanded here; no compiler is invoked.
pub const PSV_LITE_INCLUDES: &[(&str, &str)] = &[
    (
        "lite_common.cgh",
        include_str!("../native/psv/lite/lite_common.cgh"),
    ),
    (
        "lite_mesh_vertex.cgh",
        include_str!("../native/psv/lite/lite_mesh_vertex.cgh"),
    ),
    (
        "lite_shadow_vertex.cgh",
        include_str!("../native/psv/lite/lite_shadow_vertex.cgh"),
    ),
    (
        "lite_vfx_fragment.cgh",
        include_str!("../native/psv/lite/lite_vfx_fragment.cgh"),
    ),
    (
        "lite_vfx_vertex.cgh",
        include_str!("../native/psv/lite/lite_vfx_vertex.cgh"),
    ),
];
pub const PSV_LITE_PROGRAMS: &[(&str, &str)] = &[
    (
        "lite_bloom_down_f.cg",
        include_str!("../native/psv/lite/lite_bloom_down_f.cg"),
    ),
    (
        "lite_bloom_up_f.cg",
        include_str!("../native/psv/lite/lite_bloom_up_f.cg"),
    ),
    (
        "lite_copy_f.cg",
        include_str!("../native/psv/lite/lite_copy_f.cg"),
    ),
    (
        "lite_exposure_f.cg",
        include_str!("../native/psv/lite/lite_exposure_f.cg"),
    ),
    (
        "lite_fullscreen_v.cg",
        include_str!("../native/psv/lite/lite_fullscreen_v.cg"),
    ),
    (
        "lite_legacy_mesh_f.cg",
        include_str!("../native/psv/lite/lite_legacy_mesh_f.cg"),
    ),
    (
        "lite_legacy_mesh_v.cg",
        include_str!("../native/psv/lite/lite_legacy_mesh_v.cg"),
    ),
    (
        "lite_luminance_f.cg",
        include_str!("../native/psv/lite/lite_luminance_f.cg"),
    ),
    (
        "lite_mesh_f.cg",
        include_str!("../native/psv/lite/lite_mesh_f.cg"),
    ),
    (
        "lite_mesh_v.cg",
        include_str!("../native/psv/lite/lite_mesh_v.cg"),
    ),
    (
        "lite_particle_add_f.cg",
        include_str!("../native/psv/lite/lite_particle_add_f.cg"),
    ),
    (
        "lite_particle_alpha_f.cg",
        include_str!("../native/psv/lite/lite_particle_alpha_f.cg"),
    ),
    (
        "lite_particle_v.cg",
        include_str!("../native/psv/lite/lite_particle_v.cg"),
    ),
    (
        "lite_prefilter_f.cg",
        include_str!("../native/psv/lite/lite_prefilter_f.cg"),
    ),
    (
        "lite_probe_capture_f.cg",
        include_str!("../native/psv/lite/lite_probe_capture_f.cg"),
    ),
    (
        "lite_reduce_f.cg",
        include_str!("../native/psv/lite/lite_reduce_f.cg"),
    ),
    (
        "lite_shadow_f.cg",
        include_str!("../native/psv/lite/lite_shadow_f.cg"),
    ),
    (
        "lite_shadow_skinned_v.cg",
        include_str!("../native/psv/lite/lite_shadow_skinned_v.cg"),
    ),
    (
        "lite_shadow_v.cg",
        include_str!("../native/psv/lite/lite_shadow_v.cg"),
    ),
    (
        "lite_skinned_v.cg",
        include_str!("../native/psv/lite/lite_skinned_v.cg"),
    ),
    (
        "lite_sky_f.cg",
        include_str!("../native/psv/lite/lite_sky_f.cg"),
    ),
    (
        "lite_tonemap_f.cg",
        include_str!("../native/psv/lite/lite_tonemap_f.cg"),
    ),
    (
        "lite_vfx_additive_f.cg",
        include_str!("../native/psv/lite/lite_vfx_additive_f.cg"),
    ),
    (
        "lite_vfx_alpha_f.cg",
        include_str!("../native/psv/lite/lite_vfx_alpha_f.cg"),
    ),
    (
        "lite_vfx_mesh_v.cg",
        include_str!("../native/psv/lite/lite_vfx_mesh_v.cg"),
    ),
    (
        "lite_vfx_multiply_f.cg",
        include_str!("../native/psv/lite/lite_vfx_multiply_f.cg"),
    ),
    (
        "lite_vfx_premultiplied_f.cg",
        include_str!("../native/psv/lite/lite_vfx_premultiplied_f.cg"),
    ),
    (
        "lite_vfx_ribbon_v.cg",
        include_str!("../native/psv/lite/lite_vfx_ribbon_v.cg"),
    ),
    (
        "lite_vfx_sprite_v.cg",
        include_str!("../native/psv/lite/lite_vfx_sprite_v.cg"),
    ),
];

fn expand_lite(source: &str, depth: u32) -> Result<String, String> {
    if depth > 4 {
        return Err("include depth".into());
    }
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        if let Some(rest) = line.trim().strip_prefix("#include \"") {
            let name = rest
                .strip_suffix('"')
                .ok_or_else(|| format!("bad include: {line}"))?;
            let (_, body) = PSV_LITE_INCLUDES
                .iter()
                .find(|(n, _)| *n == name)
                .ok_or_else(|| format!("unknown include {name}"))?;
            if out.contains(&format!("// @lite-include {name}\n")) {
                continue;
            }
            out.push_str(&format!("// @lite-include {name}\n"));
            out.push_str(&expand_lite(body, depth + 1)?);
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

/// Expanded PSV tier programs. Stage follows the `_v`/`_f` suffix.
pub fn psv_lite_sources() -> Result<Vec<Source>, String> {
    PSV_LITE_PROGRAMS
        .iter()
        .map(|(file, source)| {
            let stage = if file.ends_with("_v.cg") {
                "vertex"
            } else {
                "fragment"
            };
            Ok(Source {
                file,
                entry: "main",
                stage,
                source: expand_lite(source, 0)?,
            })
        })
        .collect()
}

/// A compiled, immutable GXM program. Retain its patched objects until GPU completion.
pub struct Program {
    /// File stem used by the compiled manifest.
    pub name: &'static str,
    /// Native vertex or fragment stage.
    pub stage: &'static str,
    /// Aligned native program bytes retained for the process lifetime.
    pub gxp: &'static [u8],
}

// SceGxmProgram data must stay suitably aligned at its retained address.
#[repr(align(16))]
struct AlignedGxp<const N: usize>([u8; N]);

/// Native programs compiled from the recorded source set with full precision.
pub static PSV_PROGRAMS: &[Program] = include!("psv_programs.rs");
/// Compilation receipts, hashes, routes and native binding identities.
pub const PSV_MANIFEST: &str = include_str!("../native/psv/manifest.json");
include!("psv_shared.rs");
include!("psv_quad_index.rs");
include!("psv_wipe_index.rs");

/// Required radial / convolution rows / columns / explosion tap counts.
pub const PSV_LOOP_LIMITS: [i32; 4] = [10, 3, 3, 16];

/// Look up a compiled native program by its filename stem, without allocating.
pub fn psv_program(name: &str) -> Option<&'static Program> {
    PSV_PROGRAMS
        .binary_search_by(|p| p.name.cmp(name))
        .ok()
        .map(|i| &PSV_PROGRAMS[i])
}

/// Expanded shared 2D Cg compiler inputs. Does not invoke a compiler.
pub fn psv_shared_sources() -> Vec<Source> {
    PSV_SHARED_PROGRAMS
        .iter()
        .map(|&(file, source)| Source {
            file,
            entry: "main",
            stage: if file.ends_with("_v.cg") {
                "vertex"
            } else {
                "fragment"
            },
            source: source.to_owned(),
        })
        .collect()
}

/// Select a filter specialization from original LayerFilter.d.z and c.w flags.
/// Distortion must be an integer enum in 0..=10. Unknown inputs fail visibly
/// through None; the renderer must propagate that error, never choose a fallback.
pub fn psv_quad_filter_program(
    entry: &str,
    distortion: f32,
    flags: u32,
) -> Option<&'static Program> {
    if !distortion.is_finite() || !(0.0..=10.0).contains(&distortion) || distortion.fract() != 0.0 {
        return None;
    }
    let family = match entry {
        "fs_filtered" => 0,
        "fs_filtered_premultiply" => 1,
        "fs_stored" => 2,
        _ => return None,
    };
    let sampling = if flags & 128 != 0 {
        if flags & 64 != 0 {
            3
        } else {
            4
        }
    } else if flags & 32 != 0 {
        if flags & 64 != 0 {
            1
        } else {
            2
        }
    } else {
        0
    };
    PSV_PROGRAMS.get(PSV_QUAD_INDEX[family * 55 + distortion as usize * 5 + sampling])
}

/// Select the original transition branch. Mask-family aliases share kind 900;
/// unlisted kinds retain the canonical shader's default crossfade behavior.
pub fn psv_wipe_program(kind: i32) -> &'static Program {
    let kind = if (5..200).contains(&kind) && kind != 50 {
        900
    } else {
        kind
    };
    let slot = PSV_WIPE_INDEX
        .binary_search_by_key(&kind, |&(kind, _)| kind)
        .unwrap_or(0);
    &PSV_PROGRAMS[PSV_WIPE_INDEX[slot].1]
}

#[cfg(test)]
mod tests {
    #[test]
    fn psv_lite_programs_expand() {
        let sources = super::psv_lite_sources().unwrap();
        assert_eq!(sources.len(), super::PSV_LITE_PROGRAMS.len());
        for s in &sources {
            assert!(!s.source.contains("#include"), "{}", s.file);
            assert!(s.source.contains("main("), "{}", s.file);
            let code: String = s
                .source
                .lines()
                .map(|l| l.split("//").next().unwrap())
                .collect();
            assert_eq!(
                code.matches('{').count(),
                code.matches('}').count(),
                "{}",
                s.file
            );
            assert_eq!(
                code.matches('(').count(),
                code.matches(')').count(),
                "{}",
                s.file
            );
        }
        for (name, _) in super::PSV_LITE_INCLUDES {
            assert!(
                sources
                    .iter()
                    .any(|s| s.source.contains(&format!("// @lite-include {name}"))),
                "{name} unused"
            );
        }
    }
}
