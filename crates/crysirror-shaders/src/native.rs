// SPDX-License-Identifier: MPL-2.0
//! Native source stages, deliberately distinct from compiled GXP/DKSH programs.

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
            Source { file: "ray_lite_v.cg", entry: "main", stage: "vertex", source: PSV_RAY_VERTEX.into() },
            Source { file: "ray_lite_f.cg", entry: "main", stage: "fragment", source: expand(PSV_RAY_FRAGMENT) },
        ]),
        "switch" => Some(vec![Source {
            file: "ray_lite.comp.glsl", entry: "main", stage: "compute", source: expand(SWITCH_RAY_COMPUTE),
        }]),
        _ => None,
    }
}
