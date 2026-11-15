# Switch characteristics and memory

This describes the current source boundary, not a new build/game/hardware
validation. Read the [shared memory contract](../../docs/MEMORY_MANAGEMENT.md).
The shared console host is distinct from the native graphics/device driver.

## Runtime and graphics boundary

| Area | Characteristic and integration rule |
| --- | --- |
| CPU/toolchain | The player uses the repository's custom aarch64 Horizon/newlib target with rebuilt std and matching C ABI patches. The freestanding Rust contract target is a separate check. |
| Shared host | `crysirror-console-player` owns game sessions; the libnx shell owns process, input, display and audio devices. |
| Native rendering | deko3d device/queue with compiled DKSH programs. Adopted engines use the unchanged shared renderer through its custom backend; integrated routes retain their documented native renderer. |
| Compute/rays | Compute and reduced software-BVH passes require actual backend/shader support. Do not assume desktop hardware RT or acceleration-structure APIs. |
| Display | The current C driver reports 1280x720. This is presentation configuration, not the game's logical canvas or proof of automatic docked-resolution changes. |
| Launch mode | Process memory and available services depend on the actual homebrew/application launch environment. Record the mode; do not use total physical memory as the allocatable budget. |

Source entry points: [runtime Makefile](runtime/Makefile),
[target](rust/aarch64-switch.json), [shell](runtime/source/main.c),
[shared driver](runtime/source/csdk.c),
[device/presentation driver](runtime/source/gpu.c) and
[session dispatch](../../crates/crysirror-console-player/src/session.rs).
The [freestanding target documentation](https://doc.rust-lang.org/rustc/platform-support/aarch64-nintendo-switch-freestanding.html)
does not establish std/newlib compatibility for this custom target.

## Memory, layout and completion

CPU heaps and GPU-accessible blocks consume the available process backing;
CPU/GPU logical categories do not create independent physical budgets. Track
encoded/decoded asset copies, block-aligned GPU allocations, commands, display
buffers, descriptors, shader code, staging and retired resources. Host/emulator
memory is a separate measurement from the guest process budget.

The current shared C driver uses deko3d `DkMemBlock` storage, image layouts,
descriptor arrays and submission fences. Its source configures three command
slots and a 24 MiB shader-code block. Those are current implementation constants,
not general platform limits or measured workload budgets. Also review dynamic
command chunks, descriptor slots and resource-table capacity under pressure.

Preserve these boundaries when modifying allocation:

- Size images with the actual native image layout, alignment, levels, layers,
  samples and compression. Validate narrowing into the C driver's integer widths
  even when the Rust host is 64-bit.
- Distinguish CPU-cached/uncached and GPU-cached block flags. Perform the required
  visibility operations for uploads, descriptor updates and readback through the
  real driver API. A shared address does not eliminate cache/queue ordering.
- Keep owned textures separate from borrowed display images. The Rust custom
  backend defers memory/image/descriptor retirement; immediate C destroy
  functions rely on that caller-side completion discipline.
- Do not reuse a command slot, transfer region or descriptor while its recorded
  submission can still consume it. Keep fence sequence and timeout/failure
  behavior explicit, including on game switching and shutdown.
- Readback waits and temporary buffers contribute to both frame latency and
  peak residency. Retaining old/new frames and queued uploads can exceed a
  budget even when each individual resource fits.

[deko3d](https://github.com/devkitPro/deko3d) is the community native API used by
this source. Do not label it official NVN or copy proprietary SDK material into
the repository. An owner-supplied NVN backend would be a separate explicitly
implemented and validated path, not an automatic consequence of deko3d support.

## Feature and validation limits

For NFS/SMB, review the [network contract](../../docs/NETWORK_STORAGE.md).
Inspect libnx network setup and the selected transport's socket/thread/timer
support. A deko3d result or desktop feature build is not Switch network evidence.

Trace each session's actual backend and available programs. An integrated
engine's frame does not validate adopted-engine rendering, nor does the shader
exporter prove synchronization, format support or performance. Review
[console shaders](../../docs/CONSOLE_SHADERS.md) and
[render tiers](../../crates/crysirror-render-tier/README.md) for declared 3D
substitutions and their remaining connection/validation requirements.

Preserve native visual-novel canvas/text/effect behavior. Handheld/docked mode,
display changes, suspend and lifecycle ownership need their own observations;
do not infer them from one emulator startup. Record launch mode, build/toolchain,
source versions, guest allocation peaks, outstanding submissions and retirement
completion separately from physical hardware results.
