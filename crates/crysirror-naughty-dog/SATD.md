# Naughty Dog native runtime

## SATD-nd-retail-runtime

**Scope.** Uncharted 2 PS3 BCAS20097 01.00. The resource profile lives in `src/lib.rs` and `src/package.rs`. CrySirror owns the independent PSARC format in `../crysirror-formats-arc/src/psarc.rs`.

**Remaining.** Determine native package variant/overlay and streaming priority. Recover typed geometry, VRAM textures and material bindings, scene object placement, animation, camera and physics from their matching data. Identify and implement native boot/control, interaction, script/data operations, waits, audio and persistence before registering a playable adapter. An indexed resource type, localized label or decoded controller image is separate from running a level.

**Closure.** Verify the profile through normal player boot and control of a rendered level, including the original resource bindings, input and save/restore behavior. Record real installation and renderer evidence for the covered profile.

| Date | Change | Remaining |
| --- | --- | --- |
| 2026-11-23 | Added the installed level manifest, PAK page item directories, localization catalogs and shared plain PSARC random access. | Native loading/selection and playable state above. |
