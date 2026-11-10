<p align="center">
  <img src="icon/crysirror.svg" width="160" height="160" alt="CrySirror logo">
</p>

<h1 align="center">CrySirror</h1>

<p align="center">A data-driven Rust 3D engine with JS/TS scripting, native VN compatibility and MCP tools.</p>

CrySirror provides shared rendering, physics, animation, audio, text, UI and asset
systems for building games. Scenes and behaviours can be described as data or
controlled through Rust and JavaScript/TypeScript APIs. The desktop host uses
**wgpu + winit**, targeting Windows, Linux and macOS.

## Capabilities

| System | Features |
| --- | --- |
| 3D rendering | PBR/HDR, image-based lighting, local lights and shadows, fog, SSAO, TAA, bloom and tone mapping. |
| Ray tracing | Hardware ray queries, dynamic BLAS/TLAS, alpha-tested shadows, soft shadows, glossy reflections and diffuse GI on supported devices. |
| World and animation | Entity hierarchies, transforms, skeletal animation, blending, animation graphs and level sequences. |
| Physics | Rapier 3D rigid bodies, colliders, CCD, joints, character control, queries and collision events. |
| Visual effects | Modular CPU/GPU particles, sprite/mesh/ribbon rendering, sub-emitters and flipbooks. |
| 2D and UI | Retained layers and canvases, transforms, clipping, blending, filters, transitions, widgets and menus. |
| Text | Font resources/fallback, shaping, bidi, rich text, ruby, vertical layout, outlines/shadows and typewriter reveal. |
| Audio | SFX pools, music streaming, buses/ducking, fades, 3D spatial processing, acoustic BVHs, HRTF, reverb and early reflections. |
| Assets | glTF/GLB, FBX, PMX and VRM import; archive mounts, native image/audio/video codecs and asynchronous preparation. |
| Scripting and tools | Boa + Oxc JS/TS lifecycle scripts, data-driven projects, profiling and MCP project operations. |

[Engine systems](docs/ENGINE_SYSTEMS.md) · [Rendering and frame pipeline](docs/FRAME_PIPELINE.md) · [Resource formats](docs/RESOURCES.md)

## APIs

Rust callers, JS/TS behaviours and native adapters use the same engine services
and engine-owned resources.

| Entry point | Documentation |
| --- | --- |
| Engine classes and commands — World/Entity, Layers/Canvases, Text, Audio, Physics and Vfx | [API reference](docs/api/REFERENCE.md) |
| JavaScript / TypeScript bindings | [TypeScript declarations](docs/api/crysirror.d.ts) |
| Rust application lifecycle and public engine facade | [crysirror-engine](crates/crysirror-engine/src/lib.rs) |
| Machine-readable API schema | [api.json](docs/api/api.json) |
| MCP inspection, validation, editing, shader checks, trace, capture and packaging | [MCP tools](docs/MCP.md) |
| Offline TS/JS scene recipes | [Authoring guide](docs/AUTHORING.md) |

## Supported game engines

Open original game data with `crysirror play /path/to/game`.
See the [compatibility guide](docs/NATIVE_ADAPTERS.md) for features and save-file compatibility.

| Engine | Game data |
| --- | --- |
| [KiriKiri 2 / Z · KAG3](docs/KIRIKIRI.md) | XP3, TJS/KAG and [supported plug-ins](docs/KIRIKIRI.md#plug-in-compatibility) |
| [Siglus](docs/NATIVE_ADAPTERS.md#siglus) | Gameexe, Scene, G00 and voice banks |
| [RealLive](docs/NATIVE_ADAPTERS.md#reallive) | Gameexe.ini, Seen, PDT/G00 |
| [AVG32](docs/NATIVE_ADAPTERS.md#avg32) | Scenario data, PDT10/11, SAVE.INI |
| [BGI / Ethornell](docs/BGI.md) | BP, ARC, CBG |
| [SoftPAL](docs/NATIVE_ADAPTERS.md#bgi-and-softpal) | PAL resources and scenarios |
| [RFVP](docs/NATIVE_ADAPTERS.md#rfvp) | HCB, BIN |
| [NScripter](docs/NATIVE_ADAPTERS.md#nscripter) | Text scenarios, SAR/NSA |
| [Circus](docs/NATIVE_ADAPTERS.md#circus) | DC4 Plus Harmony MES, PCK/DAT, CRXG/XPCM |
| [CatSystem2](docs/NATIVE_ADAPTERS.md#catsystem2) | KCS/FES/CST, INT/FWP, HG3 |
| [QLIE](docs/NATIVE_ADAPTERS.md#qlie) | FilePack, .s, ABMP/DPNG |
| [Majiro](docs/NATIVE_ADAPTERS.md#majiro) | MJO, ARC, RCT/RC8 |
| [AliceSoft System 4](docs/NATIVE_ADAPTERS.md#alicesoft-system-4) | AIN v5, ALD/ALK, QNT |
| [System-NNN](docs/NATIVE_ADAPTERS.md#system-nnn) | SPT, Black Cyc archives, DWQ/VAW |
| [SugarPot](docs/NATIVE_ADAPTERS.md#sugarpot) | Tsukumo no Tsuki AOS2/SCR, ABM/BMP |

## Try CrySirror

Requires Rust 1.95 or newer and the platform's graphics/input/audio libraries.

```sh
cargo build --release --locked
./target/release/crysirror run examples/desktop/systems.json --no-audio
./target/release/crysirror play /path/to/game
```

The desktop build includes the native adapters and format providers. Installed
games are detected and mounted in place. See the [desktop guide](docs/DESKTOP_API.md)
for launch options, headless rendering and project setup.

Games can also be opened from `smb://server/share/game` or `nfs://server/export/game`
with on-demand network reads and local saves; see [SMB](docs/DESKTOP_API.md#games-on-an-smb-share)
and [NFS launch options](docs/DESKTOP_API.md#games-on-an-nfs-export).

[Native application packages](docs/NATIVE_PACKAGING.md) describe the macOS app, iOS and Android frontends and their local Release build commands.

[Documentation](docs/README.md) · [Architecture](docs/ARCHITECTURE.md) · [Roadmap](docs/adr/0041-engine-capability-roadmap.md)

## License

CrySirror-authored code uses MPL-2.0. Adopted and third-party components retain
their applicable licenses; see [LICENSE](LICENSE) and [NOTICE.md](NOTICE.md).
