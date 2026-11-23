# Naughty Dog native runtime

## SATD-nd-retail-runtime

**Scope.** Uncharted 2 PS3 BCAS20097 01.00. The resource profile lives in `src/lib.rs` and `src/package.rs`. CrySirror owns the independent PSARC format in `../crysirror-formats-arc/src/psarc.rs`.

**Remaining.** Determine native package variant/overlay and streaming priority. Recover typed geometry, VRAM textures and material bindings, scene object placement, animation, camera and physics from their matching data. Identify and implement native boot/control, interaction, script/data operations, waits, audio and persistence before registering a playable adapter. An indexed resource type, localized label or decoded controller image is separate from running a level.

**Closure.** Verify the profile through normal player boot and control of a rendered level, including the original resource bindings, input and save/restore behavior. Record real installation and renderer evidence for the covered profile.

| Date | Change | Remaining |
| --- | --- | --- |
| 2026-11-23 | Added the installed level manifest, PAK page item directories, localization catalogs and shared plain PSARC random access. | Native loading/selection and playable state above. |
| 2026-11-24 | Added VRAM_DESC texture decoding (DXT1/3/5, swizzled ARGB8) and shared XVAG PS-ADPCM decoding. Started EBOOT analysis from the owner's decrypted ELF. | Geometry (`BACKGROUND_SECTION`, actors), materials/shaders, Scream banks, DC script data and the native boot, menu, gameplay and save paths. |
| 2026-11-24 | Decoded compressed fragment indices (`src/geometry.rs`). | Vertex stream dequantization for every attribute kind, fragment/instance placement, materials, glTF output and rendering. |
| 2026-11-24 | Relocated package loading and background prototype mesh positions (`src/mesh.rs`): kind-4 bounds mapping; kind-6 integer grid offsets. Index variant with forward explicit references. | The kind-6 grid size is inferred (mesh kind-4 depth + 2, else 17) and fails on `con-1-start`/`war-1-market-coop`; recover the native rule from the mesh SPU/PPU code. Actor (foreground) index variant, UVs/normals/tangents, instance placement, materials and rendering. |
| 2026-11-24 | Kind-6 positions are `(q << 2) + offset` on the mesh's N-bit grid (seam vertices coincide only with that shift); position-less quads are skipped. All 334 installed packages with prototype meshes decode. | Grid N is inferred from data, not yet confirmed in native code. Actor index variant, UVs/normals/tangents, instance placement, materials and rendering. |
