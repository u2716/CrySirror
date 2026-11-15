# Integrated engines: versions, changes and rendering

Use this page when updating Siglus, BGI, SoftPAL or RFVP. These engines keep
their original runtimes inside workspace crates. CrySirror supplies the host.
Import revisions below describe recorded source baselines, not the latest
upstream release. Selected later imports are listed separately.

## Upstream versions

| Engine | Per-engine update record |
| --- | --- |
| Siglus | [UPSTREAM.md](../crates/crysirror-siglus-integrated/UPSTREAM.md) |
| BGI | [UPSTREAM.md](../crates/crysirror-bgi-integrated/UPSTREAM.md) |
| SoftPAL | [UPSTREAM.md](../crates/crysirror-softpal/UPSTREAM.md) |
| RFVP | [UPSTREAM.md](../crates/crysirror-rfvp-integrated/UPSTREAM.md) |


| Engine | Upstream | Recorded base | Later selected imports | Source record |
| --- | --- | --- | --- | --- |
| Siglus | [siglus_rs](https://github.com/xmoezzz/siglus_rs) | [`bef39ea9`](https://github.com/xmoezzz/siglus_rs/commit/bef39ea90c745a4888a92a09d18be1eecfdd10ca) | Included in the recorded baseline | [SOURCE.json](../crates/crysirror-siglus-integrated/SOURCE.json) |
| BGI | [ethornell-rs](https://github.com/xmoezzz/ethornell-rs) | [`2f8a9272`](https://github.com/xmoezzz/ethornell-rs/commit/2f8a9272a8f7ca075d82ac88fb802fa7247b3a99) | Included in the recorded baseline | [SOURCE.json](../crates/crysirror-bgi-integrated/SOURCE.json) |
| SoftPAL | [sena-rs](https://github.com/xmoezzz/sena-rs) | **Complete-runtime revision not recorded** | Vita driver: `7f633dbb89408297827d6c1857eb8e97dc363dff`. This is not a verified baseline for the complete runtime. | [Vita driver provenance](../crates/crysirror-softpal/native/vita/README.md) |
| RFVP | [rfvp](https://github.com/xmoezzz/rfvp) | [`9f07adef`](https://github.com/xmoezzz/rfvp/commit/9f07adef2eb5d559777f9bf59a67b756875cb24f) | Network readers: `91ef488` | [SOURCE.json](../crates/crysirror-rfvp-native/SOURCE.json) |

BGI's previous normalized source matched `0cfff1d2`; that revision is a merge
base, not the recorded original import. SoftPAL needs a verified complete-runtime
source revision before a reproducible upstream comparison can be made.

| Siglus console component | Recorded upstream revision | Record |
| --- | --- | --- |
| Vita Cg/GXP shader set | `d34d98021bec19ce4c9fa533df507f37079454bd` | [Shader source record](../crates/crysirror-siglus-integrated/src/native_vm/render/vita/shaders/README.md) |
| Vita player structure | `19066ea7f36b92fdecaea240a223c9bef14efd2c` | [Player source](../platform/psv/player/src/main.rs) |
| Switch build/shell structure | `19066ea7f36b92fdecaea240a223c9bef14efd2c` | [Build source](../platform/switch/runtime/Makefile) |

These component revisions do not replace the recorded complete-runtime base.

## Rendering in the default desktop integration

The default integrated player must use GPU composition. Uploading an already
composed CPU screen does not meet that requirement. The CPU composition and
full-frame readback paths below remain integration work. These are the current
code paths, not performance results.

| Engine | Composition | Transfer to CrySirror | Work remaining |
| --- | --- | --- | --- |
| Siglus | Original wgpu shaders on the GPU | Full-frame GPU readback to RGBA, then host texture upload | Remove the full-frame CPU round trip. |
| BGI | CPU composition through `snapshot::compose_runtime_frame` | Completed RGBA frame uploaded by the host | Connect the retained GPU renderer to the integrated host. |
| SoftPAL | CPU composition through `rasterize_scene_rgba` | Completed RGBA frame uploaded by the host | Connect the retained GPU renderer to the integrated host. |
| RFVP | CPU composition through `SoftRaster` | Completed RGBA frame uploaded by the host | Connect GPU composition to the integrated host. |

Siglus on Switch/Vita has a separate console session. It calls the native
deko3d/GXM renderer directly, rather than the desktop readback adapter. The
console path must be measured separately. See [console session dispatch](../crates/crysirror-console-player/src/session.rs)
and [Siglus console session](../crates/crysirror-siglus-integrated/src/console.rs).
Retained platform modules alone do not establish working console output.
The [Anemoi Switch startup comparison](validation/siglus-switch-anemoi-2026-11-14.md)
records one measured emulator run and its limits.

## Siglus: changes made during integration

| Area | CrySirror change | Inspect when updating |
| --- | --- | --- |
| Source layout | Upstream crates became internal `native_vm`, `native_assets` and `native_xfile` modules. | [Module exports](../crates/crysirror-siglus-integrated/src/lib.rs) |
| Lifecycle/input | A session worker drives the desktop runtime and forwards host input, focus and dialog results. | [Adapter](../crates/crysirror-siglus-integrated/src/adapter.rs) |
| Graphics | Desktop renders offscreen and reads back the final opaque frame. Console sessions use their native renderer directly. | [Desktop renderer](../crates/crysirror-siglus-integrated/src/native_vm/render/mod.rs), [console](../crates/crysirror-siglus-integrated/src/console.rs) |
| Audio | A host pull backend returns 48 kHz stereo PCM. The integrated runtime opens no separate audio device. | [Audio backend](../crates/crysirror-siglus-integrated/src/native_vm/audio/host_backend.rs) |
| Files/fonts/saves | Injected game/save/font roots, read-only game resources, local native saves and optional network capabilities. The upstream bundled font is not included. | [Host scope](../crates/crysirror-siglus-integrated/src/host_scope.rs), [material decoding](../crates/crysirror-siglus-integrated/src/decode_material.rs) |
| Snapshots | CrySirror framework snapshots are rejected. Native game saves remain runtime-owned. | [Adapter](../crates/crysirror-siglus-integrated/src/adapter.rs) |

Behavior evidence: [Siglus validation](validation/siglus-integrated.md).
Source synchronization and compilation: [2026-11-14 update](validation/siglus-upstream-sync-2026-11-14.md).

## BGI: changes made during integration

| Area | CrySirror change | Inspect when updating |
| --- | --- | --- |
| Source layout | Upstream crates became internal `native_*` modules. The GPU namespace uses `bgi_wgpu`. | [Module exports](../crates/crysirror-bgi-integrated/src/lib.rs), [source normalization](../crates/crysirror-bgi-integrated/SOURCE.json) |
| Lifecycle/input | A session worker drives the runtime. Window, input, dialogs and output devices belong to CrySirror. | [Adapter](../crates/crysirror-bgi-integrated/src/adapter.rs) |
| Graphics | The default `crysirror-host` feature selects completed CPU frames, rather than the retained GPU renderer. | [Host runtime](../crates/crysirror-bgi-integrated/src/native_app/host_runtime.rs) |
| Audio | Session-owned 48 kHz stereo PCM is drained between bounded clock steps. No standalone CPAL device is enabled. | [Audio session](../crates/crysirror-bgi-integrated/src/native_audio/session.rs) |
| Files/configuration | Host-selected bootstrap/font/GBK policy and `Install/` resource view. Native primary-root writes use a private writable overlay, read before installed resources. | [Host runtime](../crates/crysirror-bgi-integrated/src/native_app/host_runtime.rs), [capability I/O](../crates/crysirror-bgi-integrated/src/native_app/file_policy/cap_io.rs) |
| Saves | Native saves stay runtime-owned. CrySirror framework snapshots are rejected. | [Adapter](../crates/crysirror-bgi-integrated/src/adapter.rs) |

Behavior evidence: [BGI update validation](validation/bgi-upstream-sync.md).

## SoftPAL: changes made during integration

| Area | CrySirror change | Inspect when updating |
| --- | --- | --- |
| Source layout | Original asset, script and VM modules are internal workspace modules. The complete-runtime source pin is missing. | [Module exports](../crates/crysirror-softpal/src/lib.rs) |
| Lifecycle/input | A session worker owns PAL state and translates host input, window requests and aspect modes. | [Adapter](../crates/crysirror-softpal/src/adapter.rs) |
| Graphics | The worker rasterizes `FrameScene` on the CPU before publishing RGBA. The wgpu backend remains in the source tree. | [Adapter](../crates/crysirror-softpal/src/adapter.rs), [retained GPU backend](../crates/crysirror-softpal/src/native_vm/renderer/wgpu_backend.rs) |
| Audio | Native mixing publishes PCM through the host backend instead of opening its own output device. | [Audio backend](../crates/crysirror-softpal/src/native_vm/audio/host_backend.rs) |
| Files/NLS | Host resource/save/font roots and optional network resources. `nls`, native `lang` and four presentation aspect modes remain distinct settings. | [Configuration and worker](../crates/crysirror-softpal/src/adapter.rs) |
| Vita | Optional `vita-gxm` retains the original scene driver and borrowed texture/draw ABI. | [Native output contract](../crates/crysirror-softpal/native/vita/README.md) |

Network evidence for all four engines: [integrated network validation](validation/integrated-network-2026-11-09.md).

## RFVP: changes made during integration

| Area | CrySirror change | Inspect when updating |
| --- | --- | --- |
| Source layout | The complete runtime is maintained in `crysirror-rfvp-native`; `crysirror-rfvp-integrated` supplies its CrySirror host. | [Native source record](../crates/crysirror-rfvp-native/SOURCE.json), [adapter](../crates/crysirror-rfvp-integrated/src/lib.rs) |
| Lifecycle/input | One worker owns mutable runtime state, clock, input and shutdown. Previously global state is scoped to its session/thread. | [Adapter](../crates/crysirror-rfvp-integrated/src/lib.rs) |
| Graphics | The integrated host selects `SoftRaster` and publishes full straight RGBA8 frames. | [Renderer selection and frame output](../crates/crysirror-rfvp-integrated/src/lib.rs) |
| Audio/movies | Native mixing produces 48 kHz stereo PCM. Shared streaming codecs decode audio, and native BIN/loose readers feed host movie decoding. | [Audio bridge](../crates/crysirror-rfvp-integrated/src/audio.rs), [adapter](../crates/crysirror-rfvp-integrated/src/lib.rs) |
| Files/saves | Scoped read-at resources and optional network access. Native slots use the local `rfvp/` save namespace. Framework snapshots are rejected. | [Configuration and storage](../crates/crysirror-rfvp-integrated/src/lib.rs) |
| Console output | Optional Vita GXM and Switch software-framebuffer modules remain separate from the default desktop worker. The latter is CPU rendering. | [Platform contracts](../crates/crysirror-rfvp-integrated/README.md) |

## Updating an integrated engine

1. Compare the recorded base with the selected upstream commit. Account for
   later selected imports separately.
2. Check each local-change row above before merging. Preserve the host's
   window/device ownership, storage roots, input, output and save behavior.
3. Update the source record and this version table together. Record target
   builds and exercised game paths in the linked validation notes. A source
   import does not by itself prove frame rate, hardware or whole-game behavior.
