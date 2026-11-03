# 0053: KiriKiri (TJS2) adapter over engine Layer/Canvas services

Date: 2026-10-26. Status: implemented (first slice). Implementation,
compilation and behaviour evidence are recorded separately in STATUS.md.

## Context

The KiriKiri (KRKR2/KRKRZ) integration consists of the `crysirror-tjs2` VM,
`crysirror-kag` scenario parser and `crysirror-krkr` semantic adapter.
CrySirror's shared puppet capability (ADR 0029) supplies E-mote presentation.

AGENTS.md and NATIVE_ADAPTERS.md require a thin semantic/syscall adapter:
original VM semantics, engine-owned resources, no second renderer.

## Decision

1. **Maintain the VM/parser as first-class crates.** `crysirror-tjs2` owns TJS2
   parsing, lowering, bytecode verification and execution. `crysirror-kag` owns
   KAG scenario parsing. `crysirror-krkr` maps native objects and operations onto
   shared engine services.
2. **Link the native integration in the desktop host.** TJS2, KAG and the native
   adapter are workspace dependencies; no per-engine feature flag is required.
3. **Native classes translate to engine services** (`crysirror-krkr`):
   - `Layer` → one engine Layer node (position, size, clip to bounds,
     visibility, opacity) plus one engine Canvas surface for its image,
     presented beneath the node. Painter order is a depth-first walk of the
     KiriKiri tree, sorted by `absolute`, written as engine layer order.
   - `loadImages` → a Canvas `set` of an image paint decoded by the shared
     decoder registry (TLG through `arc-formats`); `fillRect`, `colorRect`,
     `copyRect`, `operateRect`, `assignImages`, `drawText` → Canvas
     `clear`/`draw`/`copy` where the engine operation is the same operation.
   - `Timer` → engine clock, KiriKiri defaults (interval 1000, capacity 6,
     overridable `onTimer`, action owner/name); `System` continuous handlers.
   - `Window` → the project canvas; mouse/key input → `onMouseDown/Up/Move`,
     `onClick`, `onKeyDown` on the window and the front-most hit layer.
   - `Storages` → KiriKiri auto-path, case-insensitive naming over the
     client's capability storage; `Scripts` → the TJS2 compiler/VM.
4. **Two engine capabilities were added first**, usable without KiriKiri:
   - `CanvasCommand::Present { parent, position }`: a canvas presentation
     can live inside a layer tree and inherit its transform, visibility,
     opacity and clip. The per-client surface budget is 1024.
   - `HostServices::image_metadata`: a thread-safe image metadata handle,
     because KiriKiri scripts read `imageWidth` synchronously after
     `loadImages`.
5. **Unsupported means visible failure.** Unknown native profiles must report
   their missing capability. The initial slice's unsupported font, canvas-copy,
   draw-face, audio, archive and plugin families were extended by later work;
   use current adapter tests/docs for their exact supported profiles.

## Rejected alternatives

- **Separate native renderer/mixer.** Layer composition, video and audio belong
  to shared CrySirror services, so native scripts operate on engine-owned state.
- **Unsafe borrowed host pointer inside TJS natives.** Replaced by the
  long-lived `image_metadata` handle, the same pattern as `storage()`.

## Follow-up implementation

Shared XP3 mounts/protection are covered by ADRs 0091/0096, engine font metrics by
0063, KAG3/audio/storage mappings by 0088, menus/awaitable services by 0066, vector
and worker effects by 0090/0093, and isolated subtree transitions by 0094.
These retire the original five-item bootstrap checklist. Complete plugin/native
save profiles, compatibility errors and game regressions remain tracked in
NATIVE_ADAPTERS.md and dated STATUS.md entries. A successful boot is not full
capability extraction or all-game acceptance.

## Acceptance evidence (this slice)

- `crysirror-scene` `presentation_attaches_under_parent_layer_and_follows_it`.
- `crysirror-krkr` tests: a self-authored script with a script subclass of
  `Layer`, a Timer on the engine clock and a click produce engine Layer and
  Canvas state inspectable without the VM; unsupported members fail with
  their capability names; Shift_JIS scripts and case-insensitive storage.
- `examples/krkr-demo` (self-authored script, generated images) renders
  through the desktop renderer; the current build needs no engine-specific flag.
- This initial slice did not run a commercial KiriKiri title. Later KAG3 and
  installed-game evidence is recorded separately in STATUS.md; it does not turn
  the original slice's tests into complete game acceptance.
