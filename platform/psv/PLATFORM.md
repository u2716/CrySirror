# PS Vita characteristics and memory

Current source boundaries are reviewed here; successful compilation, emulator
pixels and hardware behavior require separate dated evidence. Read the
[shared memory contract](../../docs/MEMORY_MANAGEMENT.md) and
[GXM driver details](../../crates/crysirror-wgpu-gxm/README.md).

## Runtime and graphics boundary

| Area | Characteristic and integration rule |
| --- | --- |
| CPU/toolchain | 32-bit ARM/newlib target. Check integer narrowing, alignment, FFI layouts and thread stacks; host desktop layouts do not establish the Vita ABI. |
| Shared host | `crysirror-console-player` selects sessions. The Vita host owns controller/touch, display and audio devices. |
| Native rendering | GXM with compiled GXP programs and vita2d's GXM context; no OpenGL/vitaGL. Adopted engines use the shared renderer's GXM driver; integrated Siglus/BGI retain their native GXM paths. |
| Shader execution | No general compute stage or hardware ray-query path. Supported compute-like work needs an explicit native fragment/CPU lowering with its own contract. |
| Display | The current player configures 960x544. The original game canvas and pointer coordinates stay separate from presentation size. |
| Portability | Vita Rust std support is partial; a target/contract check does not validate every filesystem, thread, codec or engine dependency. |

Sources: [player](player/src/main.rs),
[session dispatch](../../crates/crysirror-console-player/src/session.rs),
[GXM implementation](../../crates/crysirror-wgpu-gxm/src/gxm.rs) and
[Rust target documentation](https://doc.rust-lang.org/rustc/platform-support/armv7-sony-vita-newlibeabihf.html).
VitaSDK is a community toolchain; its API declarations are not an official Sony
behavior specification. For memory queries see its
[sysmem reference](https://docs.vitasdk.org/group__SceSysmemUser.html).

## Memory domains and pressure

The player currently configures a **300 MiB newlib heap** through
`_newlib_heap_size_user`. This is a source setting for the selected launch setup,
not an advertised physical capacity or an approved budget for every game. The
heap arena is reserved at startup. Enlarging it can reduce user memory available
for mapped buffers, staging and textures that cannot fit in CDRAM.

Track user memory, CDRAM and physically contiguous memory independently using
the available native query. Keep allocator-used/free heap bytes separate from
those kernel-domain figures. Query failure must remain visible. Check the
actual launch memory mode and SDK/library revision before choosing limits;
emulator allocation success does not establish real-device availability.

The shared GXM driver currently:

- Maps user-uncached memblocks for buffers. Buffers up to 64 KiB use size classes
  from 1 MiB arenas; larger buffers own blocks. Count arena occupancy and block
  slots, not only useful buffer bytes.
- Tries CDRAM for render targets/large textures, then eligible user-uncached
  backing. This is a specific texture policy, not universal interchangeability
  of every allocation type.
- Allocates depth/stencil only when a pass needs it. Include stride/alignment,
  and avoid helpers that silently add depth to every 2D target.
- Handles 64-bit texture formats explicitly because the linked vita2d helper's
  size table otherwise underestimates them. Check both allocation size and GXM
  surface/output-register layout before adding a format.
- Retires destroyed resources for four presents for the current Vita3K path;
  allocation-pressure recovery waits for the GPU and reclaims retired resources
  before a bounded retry. This count is an implementation workaround, not a
  universal hardware synchronization rule.

Scenes/submission on this driver are synchronous. Upload/readback CPU copies
still require correct GPU ownership. Current readback includes emulator-specific
scene flushing; do not generalize that workaround to physical hardware or
assume it makes per-frame readback inexpensive.

## Feature and validation limits

For NFS/SMB, review the [network contract](../../docs/NETWORK_STORAGE.md).
The desktop clients' socket/thread/timer dependencies need actual target
backends; a GXM game result does not establish Vita network storage.

The driver documents supported color/data formats and explicit refusals, such
as unsupported compute, float-target blending and absent native programs.
Storage-format exposure for a fragment lowering does not create general compute
support. Review the live driver/library when extending mips, arrays, render
formats or blend modes; a compiled shader catalogue is not a connected pass.
Use [console shaders](../../docs/CONSOLE_SHADERS.md) and
[render tiers](../../crates/crysirror-render-tier/README.md) for reduced 3D/software
rays. They are not a full desktop hardware-RT implementation.

Keep visual-novel composition at its native contract, including text and effects.
Measure actual scene/character residency before claiming a large 3D workload
fits. Record CPU heap, free domains, live/retired texture/buffer counts and peaks
at named lifecycle points. Results from one integrated runtime do not validate
the shared GXM path, and emulator results do not certify hardware.
