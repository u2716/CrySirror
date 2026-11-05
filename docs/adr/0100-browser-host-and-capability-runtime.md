# ADR 0100: Browser host over shared Runtime capabilities

Date: 2026-11-04
Status: implemented 2D/PBR browser host and shared CPU workers; wider engine migration remains partial

## Decision

`crysirror-web` is an additional presenting host, beside `crysirror-desktop`.
It owns browser lifecycle, file import, dedicated-worker messages, WebGPU
surface creation, output-only Web Audio and IndexedDB commits. It reuses
`crysirror-runtime`, the data VM, Scene/Canvas/Layer/UI state, image/text/audio
services, immutable frame extraction, and `crysirror-wgpu`. There is no browser
copy of the VM, compositor or renderer. Desktop stays the default workspace
host. Core and compat retain their safe `no_std + alloc` boundary.

`Runtime::with_storage` accepts a project capability plus explicitly scoped
client capabilities. `Assets::from_storage` resolves project assets without
ambient filesystem calls. `RuntimeStorage` preserves the native DiskVfs route
on native hosts, including provider precedence and existing opened handles.
Injected roots never manufacture OS paths. Physics consumes `FileSystem`
instead of depending on a disk implementation. Native asset registrations still
open a restricted directory capability; concatenating paths below the broader
project root must not permit symlinks to cross a client's asset boundary.

The browser audio coordinator calls shared `SceneAudio`, `Mixer` and `MixFrame`
services for both scene voices and native channel PCM. There is no private audio
decoder or VM mixer in the browser host.

The initial browser host imports bounded immutable file bytes before admitting
a Runtime. Async asset/CPU jobs keep explicit admission, ownership, cancellation,
dependencies and completion. No synchronous fetch or filesystem impersonation
is introduced. Imported assets are not uploaded to a server. Game data and fonts
are not included in the distribution. The generated checker/tone demo is owned
synthetic test data.

## Execution boundary

One dedicated browser worker owns the Runtime, GPU and imported bytes. Browser
input remains on the main thread; mixed PCM goes to an output AudioWorklet. The worker transfers one completed
ImageBitmap at a time to the page bitmap renderer; a consumption acknowledgement
bounds presentation ownership. It does not depend on automatic HTML placeholder
updates for a transferred GPU canvas.
Only serializable data and transferable buffers cross these boundaries.

The default browser build enables atomics and bulk memory, rebuilds std with
nightly, imports shared memory, and exports the TLS initialization symbols.
wasm-bindgen-rayon 1.3.0 starts a host-owned global CPU pool. Shared TaskPool
jobs run on those workers; each pool preserves its own concurrency limit,
bounded admission, cancellation and FIFO order when configured for one worker.
Startup verifies actual overlap on distinct workers and these queue contracts.
The host initializes 2–4 workers by default. GPU objects stay on the presenter.

The supplied local HTTP server enables cross-origin isolation with COOP/COEP.
An explicit single-threaded build retains deferred cooperative jobs and reports
zero CPU workers. Blocking waits reject on both browser profiles. Profiler and
asset clocks use web-time, including cross-worker time origins for atomics.

Browser mesh-history and GPU-backed synchronous readers reject admission:
they need a presenter actor and async readback receipts. Native implementations
remain. No unsafe Send/Sync wrapper is introduced for browser GPU objects.
wgpu's existing non-atomic compatibility feature has no effect in the atomics
build. The browser presenter polls pipeline validation and retries encoding
without repeating input or simulation.

This provides a parallel browser executor; it does not complete every row of
ADR 0050. Owner-controlled World/fixed-step commits, general cooking/upload
budgets and broader scheduling remain separate work. Rust panic aborts require
restarting the browser host.

## Current browser profile and visible limits

- WebGPU, OffscreenCanvas, ImageBitmap presentation, module Workers,
  AudioWorklet and IndexedDB are needed.
  Serve through localhost or HTTPS. No WebGL fallback is promised.
- The shared `crysirror-script-v1` adapter is registered. Native adapters are not
  substituted with it: absent factories fail by engine name. Native Siglus is
  still registered by the native Runtime; its old standalone wasm entry is not
  the browser host boundary.
- Retained 2D and 3D/PBR project state use the same Runtime and renderer as
  native hosts. Browser pipeline validation is an asynchronous frame dependency.
  The rotating-cube PBR demo is verified; this is not a claim that every 3D
  material/effect/format has been browser-tested.
- Initial import limits: 256 MiB total, 64 MiB per file, 65536 files. Larger
  installed games need a random-read Blob/OPFS provider and resource readiness
  design rather than raising a whole-game memory copy limit.
- Save/cache capabilities are distinct per client and namespace. Atomic writes
  use IndexedDB transactions; `Ready` is published only on transaction complete.
  Failed commits do not publish new in-memory bytes. One pending commit per
  capability bounds admission. Synchronous writes/removes, native storage leases,
  metadata-preserving copies and archive mounting on injected roots reject
  explicitly. They are not silently emulated.
- The browser host's snapshot save/load uses the existing Runtime snapshot and
  restore format. It does not add original-save support to an unported VM or
  bypass existing resource snapshot limitations.
- The logical canvas remains the project resolution. CSS resizing changes only
  presentation size; pointer mapping accounts for the letterboxed viewport.
  Browser devicePixelRatio does not implicitly enlarge the game resolution.
- WebGPU canvas textures use a configured sRGB view for the shared linear
  renderer. The surface's base unorm format alone would darken the picture.

Implementation, builds and observed behavior are recorded separately in
[the current validation receipt](../validation/browser-parallel-3d.md). The
[initial receipt](../validation/browser-host.md) records the earlier 2D slice.
