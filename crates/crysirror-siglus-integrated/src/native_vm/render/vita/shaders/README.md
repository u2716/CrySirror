# Siglus GXM shaders

These are the integrated Siglus renderer's existing shaders, copied from
`xmoezzz/siglus_rs`, `crates/siglus_scene_vm/src/render/vita/shaders`, at revision
`d34d98021bec19ce4c9fa533df507f37079454bd`. The compiler source is retained under
`platform/psv/siglus-shaderc`. The source project is MPL-2.0.

The set contains 15 Cg programs, three shared Cg includes and 15 compiled GXP
programs: sprite, plain copy, solid, wipe, page wipe, E-mote, mesh and shadow
stages. The original shader algorithms and existing binaries are retained.
`../gpu.rs` embeds the GXP files relative to its own module and registers them
with GXM. This is Siglus's internal renderer, not the shared Lite ray profile.

The imported GXP files are existing build outputs from the source project.
No new shader compilation, CrySirror Vita link or device run is claimed by this
import. The CrySirror console host still needs its device/output integration.
