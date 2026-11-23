# Browser CPU workers and 3D/PBR — 2026-11-04

## Implementation

The default web build enables atomics/bulk memory, rebuilds std on the installed
nightly toolchain, and links imported shared memory with explicit TLS exports.
wasm-bindgen-rayon is pinned to 1.3.0 and Rayon to the existing locked 1.12.0.
Its Rust and bundlerless worker helper source were inspected before use.
Native/default toolchains and the existing target directory remain unchanged.
All builds and tests use release.

Shared TaskPool jobs use the global CPU pool with a bounded per-pool FIFO queue
and configured concurrency limit. Cancellation and completion use the same
engine receipts as native jobs. GPU objects remain on the presenting Worker.
Browser GPU-backed synchronous readback and mesh-history handles reject instead
of moving non-Send JS objects into shared CPU jobs. Native paths remain.

PBR material-pipeline validation is polled on browser frames. PreparationPending
retains the last displayed picture and does not replay simulation/input.
Native material validation preserves its prior behavior. The host admits 3D
project state and sends the shared extracted scenes to the existing renderer.

The 3D demo contains original cube/material/world data with angular velocity,
rendered through the shared engine.

## Compilation and checks

- `python3 tools/web_host.py build`: passed with threaded release wasm and
  generated JS/TS bindings. Build helper supplies shared-memory and TLS linking.
- Atomics-enabled nightly release check: passed.
- Native task tests: 7 passed.
- Native pipeline completion/error test: 1 passed.
- Native offscreen PBR sky/lit geometry/sun-shadow GPU test: explicitly executed
  with `--ignored`; 1 passed, zero ignored in that filtered run.
- Python web tooling tests: 2 passed. JS syntax and scoped whitespace checks passed.
- Existing release API reference generator rerun: 8 passed; reflective
  fingerprint stayed `c1a04f58f36209a0`. No reflective class signatures changed.

The first link attempt compiled atomics but did not import shared memory;
startup correctly failed. Explicit shared-memory linking then exposed missing
TLS exports. The final helper includes both; generated bindings instantiate
SharedArrayBuffer-backed memory with a 2 GiB maximum.

## Observed browser behavior

An independent headless Chromium profile was used; no user desktop/browser was
read. Cross-origin isolation was confirmed. The browser requested an Apple
adapter with `metal-3` architecture. Headless hardware GPU acceleration was
explicitly enabled; the software-GPU attempt was terminated after spending
excessive time preparing PBR pipelines and is not counted as a pass.

Actual browser startup reported:

```json
{"workers":4,"overlap":true,"worker_indices":[0,1],"fifo":true,"backpressure":true,"cancellation":true}
```

The two overlap jobs cannot complete until both have started. Single-worker
queue verification checked submission order and capacity rejection while its
first task was gated, then verified a cancelled queued job never executed.
Runtime task inspection reported real CPU workers and thousands of completed
frame-extraction jobs, with nonblocking assets enabled.

The 2D demo ran with `dedicated-worker-parallel`. Switching to 3D created a new
worker/pool, and the PBR scene presented thousands of frames. The captured
800×450 canvas visibly contained the teal and copper cubes, ground plane and
sky. World inspection confirmed nonzero changing cube rotation from the shared
angular-velocity simulation; both material specs were engine-owned PBR data.

Save and Load were also exercised on the live threaded 3D scene; the browser
reported `Save loaded.` and continued presenting without an error.

The initial 2D receipt separately records audio/file-import evidence.
This follow-up does not claim every 3D effect/format or native game adapter has
been browser-tested. Synchronous browser GPU readback remains unsupported.
It does not mark the engine-wide ADR 0050 subsystem migration complete.
