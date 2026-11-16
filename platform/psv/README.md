# PS Vita host

[Platform characteristics and memory](PLATFORM.md) describes user/CDRAM budgets,
allocation lifetimes, shader limits and the distinction between source and
device evidence.

`player/` contains the VitaSDK host entry, controller/touch polling, PCM output
and VPK packaging for the engine sessions selected by
`crysirror-console-player`. Siglus and BGI present through their integrated
GXM renderers. Every other engine (adopted VMs and authored projects) runs the
shared runtime and the unchanged shared renderer on
[`crysirror-wgpu-gxm`](../../crates/crysirror-wgpu-gxm/README.md), the GXM
driver under the wgpu custom backend.

The player reserves a 300 MiB newlib heap (`_newlib_heap_size_user`).
The rest of user memory backs GPU buffers, staging and the textures
that do not fit CDRAM. Allocation failures are written allocation-free to
`ux0:data/CrySirror/fatal.log`, together with the abort caller and the
text-segment return addresses found on the stack, for symbolisation with the
unstripped ELF (`cargo vita build elf --release`).

Adopted-engine results in Vita3K, with the defects found and fixed, are in
[psv-adopted-engines-2026-11-15](../../docs/validation/psv-adopted-engines-2026-11-15.md).

The shared shader library supplies native GXM Cg/GXP programs for shared 2D,
reduced 3D and stackless BVH software rays. No OpenGL/vitaGL path is used.
See the [native ABI](../../crates/crysirror-shaders/native/psv/README.md) and
[compiler utility](shaderc/README.md) for program selection and regeneration.
See [native shader sources and validation boundaries](../../docs/CONSOLE_SHADERS.md).
The reduced 3D feature tier for this platform is `crysirror-render-tier`.
VN composition is not reduced.

```sh
python3 tools/console_host.py psv
```

This performs a Release cross-check of `crysirror-console`, rebuilding
`core,alloc` with the installed nightly/rust-src. It does not execute a game,
link Vita devices, produce a VPK or test Vita3K/hardware.

`host.toml` records the target, package format and proposed capability-root
layout. The common coordinator in `crysirror-console` retains pending simulation
and presentation tickets, refuses another tick while pending, resets elapsed
time on resume, and requires real renderer/audio/storage/clock/worker support
before accepting an engine session. It contains no engine algorithms or SDK ABI.

Remaining shared-driver work:

1. Programs for remaining refused modules such as simple 3D `mesh.wgsl`, general
   compute beyond the box-filter lowering, mip chains and array layers. The
   current driver already has specific float/data targets and fragment-lowered
   box filtering; consult its README for their format/blending restrictions.
2. Review each 3D/BVH scene's admission, native data textures, targets and fences.
   A reduced shader/program path does not establish full PBR or hardware rays.
3. Validate shared PCM, scoped storage and bounded tasks per engine session,
   including suspend, cancellation, game switching and exit ownership.
4. Measure bindings, pixels, memory and performance separately on emulator and
   hardware. Integrated-runtime results do not validate shared rendering.

The local VitaSDK headers/toolchain are reference/build inputs only; they are
not copied into this repository. No game data, fonts or SDK binary is bundled.

[Rust Vita target requirements](https://doc.rust-lang.org/rustc/platform-support/armv7-sony-vita-newlibeabihf.html)
document partial std support and VitaSDK/build-std requirements. The target
contract compiling does not establish that the full engine is portable yet.
