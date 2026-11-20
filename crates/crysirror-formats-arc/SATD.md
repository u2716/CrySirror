# Shared resource format self-admitted technical debt

## SATD-formats-arc-pb2-selection

**Scope and current location.** [PB2A decoder](src/images_a/pb2.rs) and its
named and animated native entry points. The registry decodes unnamed memory
images. This native path selects frame zero for type 5 and rejects type 6
because the caller supplies no name.

**Remaining work.** Expose named type 6 entries through scoped resource selection. Type 6 reads a name at payload
offset 36, decodes base planes and applies a matching entry's tile overlay
when the requested name differs from the base. It rejects an unrecognized
name. Verify native stream and tile handling for those entry points.

**Closure evidence.** Exercise frame zero, later frames, frame bounds, named
base and overlay selection, and missing or invalid names through the shared
registry and native CMVS resource callers.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-19 | Implemented PB2A unnamed memory-image types 1–5 and rejection of type 6 without a name. Moved the PB3 split-stream LZSS primitive from `src/images_a/pb3.rs::lz` to `src/images_a/common.rs::purple_lz` for reuse by PB2A. PB3 retains final clipped matches. | Named type 6 selection and subsequent type 5 frames. |
| 2026-11-20 | Routed CVNS numbered type 5 frames through the registered PB2 decoder. Each selected record overlays the original base. The 2004 decoder at `0x41e280` dispatches types 1–5. | Later CMVS named type 6 selection and acceptance. |

## SATD-formats-arc-pb3-selection

**Scope and current location.** [PB3 decoder](src/images_a/pb3.rs), native
named type 4 and original dispatcher variants. The standalone reader reports
that type 4 requires a name. The unnamed CMVS memory loader rejects it.

**Remaining work.** Implement scoped type 4 named base and overlay selection.
Complete the original layout audit for types 1 through 3 and type 5 and verify
the images under the original adapter's resource search and upload rules.

**Closure evidence.** Exercise named type 4, its missing or invalid variant
names, all named and unnamed variants, and CMVS-scoped base resolution through
the normal player. Keep the existing registry's standalone profile tests.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-19 | Recovered type 1 block sizes, type 4's name guard, type 7 interleaved LZSS and the type 6 and 8 base overlay path with IDA. Added type 7 and 8, variable block sizes and the strict native base option for type 6. | Named selection and remaining original layout and player validation above. |

## SATD-formats-arc-krkr-madocrypt-nana-installed

**Location.** MadoCrypt detection in `src/xp3/adler_xor/madocrypt.rs`, Nana
profile and name recovery in `src/xp3/cxdec/nana.rs`, and installed-game
archive selection in `../crysirror-launch/src/detect.rs`.

**Remaining work.** Determine which initializer layout the retail Nana game uses
and demonstrate automatic recovery from that layout.
The published PR #3 accepts a supplied profile. Its extraction results do
not validate playback through CrySirror.

**Closure evidence.** Run the archive and launch regression tests. Launch both
original installs headlessly with audio muted, reach the first scene and load
foreground images using native script paths. For Nana, demonstrate the same
result using only the original installation files.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-21 | Connected archive-inferred MadoCrypt and profile-driven Nana to XP3 opening and installed-game detection. Added authored fixtures for companion selection, offsets, filename authentication and rejection of corrupt streams. | Automatic Nana initialization, execution of regression tests and original-game playback. |
| 2026-11-21 | Added module-based selection for a referenced six-word Nana state, full name-index authentication and recovery of Cabbage content through the existing module analyzer. Added a no-sidecar regression fixture. Static inspection of the [official trial](http://frontwing.jp/product/loca-love/jp/story3_trial_01.html) found 1,374 inline `hnfn` records in `fgimage.xp3` and no `dls:` descriptor. | The PR #3 retail initializer layout and headless retail playback still need evidence. The new fixture has not been run. |
