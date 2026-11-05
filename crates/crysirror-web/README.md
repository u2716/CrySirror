# CrySirror Web

A browser host over the shared Runtime, 2D/PBR renderer and CPU resource
services. The default build runs CPU jobs on shared-memory Web Workers.
The VM, renderer, image processing and audio mixer are reused from the engine.

## Build and run

Requires an installed nightly Rust toolchain with rust-src, and a wasm-bindgen
CLI matching Cargo.lock. Stable remains the default for native builds.

```sh
python3 tools/web_host.py build
python3 tools/web_host.py serve
```

Open `http://127.0.0.1:8080`. **2D demo** exercises images, sequential input
waits and a generated tone. **3D demo** displays rotating PBR cubes and a ground
plane through the shared renderer. **Open project folder** imports a directory
containing project.json and its asset tree. **Save** and **Load** use the engine
snapshot format and transactional browser storage.

Builds use release and the existing target directory. The helper supplies
atomics, bulk memory, shared-memory linking and TLS exports, then generates the
JS/TypeScript bindings under target/web/pkg. Pass `--bindgen PATH` to select a
matching tool; matching tools under target/wasm-tools are also recognized.
No global tool or default toolchain is replaced.

The local server supplies COOP/COEP headers required for shared memory. Other
hosting must provide those headers, HTTPS (or localhost), and WebGPU,
OffscreenCanvas, ImageBitmap, AudioWorklet and IndexedDB support. A cooperative
fallback can be built explicitly with `build --single-threaded`; it does not
provide CPU parallelism.

## Ownership and supported profile

The presenting Worker owns GPU objects and ordered Runtime state. CPU jobs
receive immutable or exclusively owned engine data. Shared task pools dispatch
through Rayon-backed Workers, preserve per-pool concurrency limits, enforce
capacity backpressure and maintain FIFO order for one-worker pools. Startup
verifies two jobs overlap on distinct workers, FIFO, capacity rejection and
cancellation before publishing readiness. The default host initializes 2–4
CPU Workers according to hardwareConcurrency.

GPU pipeline validation is polled in browser frames. A pending pipeline retains
its prior displayed image; native pipeline validation keeps its existing path.
One transferable GPU ImageBitmap may be in flight to the page at a time.
The game canvas retains the project resolution when its CSS size changes.

The browser uses the shared data VM and accepts retained 2D and 3D/PBR project
state. Native game adapters are not yet registered here. This is not an
installed-game launcher for KRKR, Siglus or other original engines. There is no
WebGL or hardware-ray-tracing browser backend in this profile.

Imported files remain in the browser; the host has no upload endpoint. Limits
are 256 MiB total, 64 MiB per file and 65536 files. Fonts belong to the caller's
project. The demos contain only original generated assets.

Save/cache commits are reported Ready only after IndexedDB transaction
completion. Limits are 4 MiB per record (including snapshots), 64 MiB and 4096
records per namespace. Save/load retains existing engine resource limitations.
Synchronous storage mutation, original-save leases, dynamic archive mounting
on injected roots, synchronous GPU readback and GPU mesh-history capabilities
reject explicitly. Browser GPU readback needs a presenter-owned async actor;
GPU objects are never marked thread-safe to move them into CPU jobs.

CPU pool support does not mean every engine subsystem has completed the ADR
0050 migration. World commits and fixed-step ordering remain owner-controlled;
upload/cooking budgets and broader scheduling remain separate work. Rust panic
aborts require restarting the host. Clearing site data removes browser saves.

See [ADR 0100](../../docs/adr/0100-browser-host-and-capability-runtime.md) and
[the current validation receipt](../../docs/validation/browser-parallel-3d.md).
