# Initial browser host validation — 2026-11-04

This receipt records the first 2D/cooperative slice. The later
[parallel 3D receipt](browser-parallel-3d.md) supersedes its current-profile limits.

## Implementation

`crysirror-web` is a workspace host over the existing Runtime, shared engine
services and wgpu renderer. It provides the bounded 2D data-project profile in
[ADR 0100](../adr/0100-browser-host-and-capability-runtime.md). The shared Runtime
accepts scoped storage capabilities; native constructors retain DiskVfs.
Browser assets are imported before startup. Worker jobs preserve deferred
completion and admission but are cooperative, not CPU-parallel.

The worker owns GPU/runtime objects. A bounded ImageBitmap transfer presents
completed shared-renderer frames on the page. Shared SceneAudio/Mixer/MixFrame
services produce PCM for an output-only AudioWorklet. Save receipts use
IndexedDB transaction completion; async JS writes own a byte copy before any
await so WebAssembly memory growth cannot detach their admitted input.

Core still declares `#![no_std]` and `#![forbid(unsafe_code)]`. No screen, window
or microphone from the user's active desktop was inspected. Browser automation
used a fresh, independent headless browser and original generated test assets.
No game data or fonts were packaged. No debug build, separate target directory,
CI workflow or remote build was used.

## Compilation and local tests

All Cargo validation used release and the existing target directory.

| Check | Actual result |
|---|---|
| `python3 tools/web_host.py build` with matching wasm-bindgen 0.2.129 | Passed; wasm + JS/TS bindings and local host staged under target/web |
| `cargo check --release --target wasm32-unknown-unknown -p crysirror-runtime -p crysirror-web` | Passed |
| `cargo check --release -p crysirror-desktop --bin crysirror` | Passed |
| `cargo test --release -p crysirror-tasks -p crysirror-web --lib` | 7 task tests + 3 imported-file/capability/runtime tests passed |
| `cargo test --release -p crysirror-runtime --lib` | 91 passed, 1 ignored |
| `python3 -m unittest discover -s tests -p test_web_host.py` | 2 passed |
| `CRYSIRROR_BLESS=1 cargo test --release -p crysirror-engine --test api_docs` | Final run: 8 passed |
| JavaScript syntax and scoped diff whitespace checks | Passed |

The ignored Runtime movie test requires an owner-supplied local MPEG. It is
not counted as a pass. Native test coverage includes scoped roots, provider
precedence, seekable readers, existing storage behavior, task backpressure,
cancellation, concurrency and dependency ordering. The portable host test
constructs a Runtime without disk roots, checks two separate client views,
performs a shared async read, and restores the original snapshot clock.

The first API regeneration exposed pre-existing stale metadata: an unmapped
Canvas Retire command and the missing PathCommand source after its move to
core. Mapping the existing operation to Canvases.apply and reading its actual
source repaired those reference checks. The final fingerprint is
`c1a04f58f36209a0`, logged in docs/api/CHANGES.md. The regeneration also records
concurrent image/type changes; it does not establish their behavioral coverage.

The validated wasm artifact was 25,144,206 bytes with SHA-256
`40b051be9301fd51bd77696862275f2f5d20b2a6c9dd0903fcc88653baf86cb6`.

## Observed browser behavior

Validated in an independent headless Chromium instance with WebGPU enabled,
served on localhost. Only the generated checker/tone project was used.

- The real shared renderer displayed the checker at the expected first position.
  A captured canvas image confirmed visible output. Canvas pixel reads matched
  background `[16,25,39,255]` and checker `[32,81,119,255]`. A configured sRGB
  texture view corrected the previously dark unorm-only output.
- Enter advanced the first input wait to the second position; a separate pointer
  click advanced the next wait. They did not both advance from one input edge.
- Scene audio reached the output AudioWorklet at 48 kHz. Its accumulated
  telemetry reported 11,978 nonzero output frames and peak 0.067138671875 from
  the generated quarter-second tone. This validates nonzero sample delivery,
  not a physical listening test.
- Save, advance, and load restored the saved position. After a page reload and
  new worker startup, load restored the previously saved second position from
  IndexedDB. Pixel reads distinguished the first and second sprite locations.
- Selecting the generated demo directory through the actual file input imported
  project.json and its asset tree and displayed the expected first frame.
- Changing the presentation to 800×500 CSS pixels retained a 640×360 backing
  canvas. Presentation and pointer code share the letterbox mapping.
- A write admitted from a WebAssembly memory view survived memory.grow before
  IndexedDB opened: the persisted bytes remained `[1,3,5,7]`.
- Aborting a replacement IndexedDB transaction rejected the write and retained
  the old `[1,3,5,7]` record. No request-success event was treated as committed.

Artifacts and command output remain under target/verification/wasm. The local
build/serve commands are documented in crates/crysirror-web/README.md.

This receipt does not claim installed-game compatibility, native adapter
coverage, PBR/3D support, CPU worker parallelism, OPFS streaming, physical audio
device latency, or validation on browsers other than this Chromium instance.
