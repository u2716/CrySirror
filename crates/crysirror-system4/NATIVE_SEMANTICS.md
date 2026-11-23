# System 4 native semantics

Current API and parameter coverage: [developer reference](../../docs/native-api/system4.md).
Native binary/source identities and complete-file digests remain in [SOURCE.json](SOURCE.json).
This file records semantic findings, not compilation or game-run results.

| Area | Native contract | Implementation |
| --- | --- | --- |
| Calls and frames | Arguments enter local pages in native order; returns stay on the operand stack. AIN v5 methods do not add the later v6.1 `this` reference. | `src/vm/functions.rs`, `src/vm/mod.rs` |
| References | Preserve page/index identity and ownership. | `src/vm/heap.rs`, `src/vm/external.rs` |
| Cleanup | Destructors run before frame removal; members release in reverse order. A pending external call suspends cleanup rather than reordering it. | `src/vm/functions.rs` |
| Structure assignment | Call destination destructor, release destination members, then copy the source; destructors can mutate that source. Reference members alias; owned values receive native fresh slots. | `src/vm/` |
| Arrays | Realloc shrinks elements in ascending order; complete-page deletion releases in reverse. Outer growth retains existing nested dimensions. | `src/vm/arrays.rs` |
| Delegates | Keep native target/object identity, typed arguments and resumable invocation. | `src/vm/delegates.rs` |
| External calls | Validate native argument ownership before consuming words. Commit typed returns/output references against the retained call. | `src/vm/external.rs` |
| ASSERT | False is fatal; it is not a skipped debug instruction. | `src/vm/mod.rs` |
| SACT constructor | Show=1, draw method=0, blend rate=255, brightness=255. WindowSurface is separate from the ordinary sprite map. | `src/sact.rs`, sprite/native modules |
| Metadata-only images | Native dimensions can remain 0×0 while a transparent retained container owns text children. | sprite/native modules |
| Sprite drawing | Native geometry, visibility, opacity and draw-method state map to shared layers. Native IDs are not GPU pointers. | `src/sprites.rs`, `src/native_*` |
| Screen/multiply | RGB-only operations preserve destination alpha; general alpha blending is not equivalent. | shared Image profiles |
| Stretch and area reduction | Preserve separately quantized nearest 16.16 steps and admitted rectangle domains. | shared Image workers |
| Rotation and projection | Keep native rounding/order and explicit alias/crop/finite-scale restrictions. Only the polygon path admits its documented same-surface feedback. | shared Image workers |
| Math | Preserve native random/table mode policy. Portable f32/f64 does not imply x87 or MMX/SSE byte parity. | `src/native_math.rs` and shared kernels |
| Bezier / RDFT | At most 13 control points; RDFT finite power-of-two length 2..65536, positive sine, packed DC/Nyquist/Re/Im layout. | native Math and shared Audio |
| Sample analysis | Original PCM8/16 mono/stereo samples become actual arrays; aliases and untouched tails remain significant. | shared Audio/source workers |
| Source sample reads | RIFF integer PCM8/16 mono/stereo only; no playback-cursor mutation. Compressed formats, other depths/layouts and reads beyond EOF fail. | native music/sample queries |
| Civil clock | Include milliseconds and commit outputs after the shared clock receipt. | native SACT/platform queries |
| Disabled joypad | Return native zero without changing output stick references. | native input queries |
| OutputLog | SHA/profile admission selects the reviewed absent-debugger branch. Disconnected output/clear do not dispatch; persistence/auto-save controls return false. No DLL executes. | `src/adapter.rs` |
| CALLSYS6 | Append original CP932 bytes through scoped logging; the following native S_POP retains string ownership. | `src/adapter.rs` |
| Glyph images | Shared preparation owns immutable glyph products and receipts; native layout remains in the adapter. | native text and shared Text/Image |
| INI/resources | Preserve System40.ini policy and one-based AIN resource IDs; ALD physical/link parsing belongs to shared archive readers. | installed bootstrap/resources |
| Optional music interface | A reviewed unavailable-device/interface branch is distinct from unknown ABI and from successful audio playback. | native music/system modules |
| Saves | Native frame/value ownership and staged restoration are separate from recognizing a save container; full resource reconstruction is not implied. | `src/native_save/`, `src/vm/resume.rs` |

Detailed restrictions and every current source route are in the developer
reference. Historical compilation and observed behavior remain in
[focused validation](../../docs/validation/system4-shared-native-2026-11-09.md).
