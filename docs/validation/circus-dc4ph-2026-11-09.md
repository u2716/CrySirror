# Circus DC4PH playback and performance, 2026-11-09

Target: owner-authorized DC4PHDL install (0xabb6 MES profile), Apple M5,
release builds. Original game files were read in place and not copied.

## Implementation

- `crysirror-wgpu`: ordered 2D draws share one render pass per attachment
  until an ordering fence (coverage clear, backdrop copy, attachment change).
  Unfiltered transparent solid layers under identity blends
  (Normal/Add/Subtract with zero alpha, SourceOver/Screen with zero colour)
  are not rasterized.
- `crysirror-scene` snapshot isolation: an unclipped group with opacity 0
  freezes empty; an unclipped non-painting Normal container around exactly one
  painted Normal layer (no filter, or only `encoded_sampling`) folds the group
  alpha into that layer. All other unclipped groups still fail visibly.
- `crysirror-runtime`: generated `__crysirror/` asset bytes are dropped from
  the asset byte cache when their published-image lease is released and
  collected (previously they were retained for the whole session).
- `crysirror-circus`: a sequence command whose committed mask sample awaits an
  image worker is acknowledged once; the owner publishes that generation
  before any further native instruction or frame. Opcode 0x64 is classified
  as having no live save-body write: 4284B0 case 100 (0x42C4F7) copies an
  exact `"-"` into two dispatcher stack buffers and sets stack flags only.

## Compilation

`cargo build --release -p crysirror-desktop` succeeded.

## Behavior evidence

- Title screen, 2560x1440 offscreen target rendered every frame: 449 draws
  (435 transparent full-screen solids) took about 50 ms GPU per frame; after
  the change 14 draws, about 2.9 ms GPU per frame. A same-snapshot A/B
  capture (frames 400 and 3000) is pixel-identical with and without the
  pass merge and cull.
- `crysirror play DC4PHDL --headless --no-audio --frames 100000` with an input
  track pressing START and then clicking every 40 frames: before, it failed at
  frame 3020 (snapshot clip), then 18472 (snapshot clip), 34003 (mask
  dependency), 36533 (0x64 save gap), 77455 (asset cache 255 MiB of generated
  images). After all changes, the run completed 100000 frames without error
  in scene `DC4_KYO20190205A` (10843 native instructions).
- Cursor auto-hide: headless runtime run showed the cursor hidden after 5 s
  idle and visible again on the tick after a simulated pointer move. This
  matches 442FF0/442B70 (hide after 0x1388 ms without a GetCursorPos change
  when system DWORD 25 is set; native default 1).

## Not validated

Windowed frame rate on a visible window (the session's window was reported
occluded by macOS), the real OS cursor in a window, audio output, routes
after `DC4_KYO20190205A`, menus/save/load UI in this run, and
completion of the game.

## Tests

`crysirror-scene` snapshot_opacity (5 passed), `crysirror-runtime` lib
assets (5 passed), `crysirror-wgpu` lib (32 passed, 5 GPU tests ignored).
`crysirror-circus` lib: 75 passed, 39 failed, identical with and without
these changes (pre-existing test-host mock failures such as
`unexpected test system request Gesture(PrefixInput)`).
