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

## Native engines

- [Siglus](crates/crysirror-siglus-integrated/README.md)
- [RealLive](docs/REALLIVE.md)
- [AVG32](docs/adr/0095-avg32-installed-games-shared-raster.md)
- [KiriKiri 2 / Z · KAG3](docs/adr/0088-kag3-semantic-adapter-and-shared-prerequisites.md)
- [NScripter](crates/crysirror-nscripter/README.md)
- [RFVP](docs/NATIVE_ADAPTERS.md)
- [BGI / Ethornell](docs/BGI.md)
- [SoftPAL](docs/NATIVE_ADAPTERS.md#bgi-and-softpal)
- [Circus](crates/crysirror-circus/README.md)
- [CatSystem2](crates/crysirror-catsystem2/README.md)
- [QLIE](crates/crysirror-qlie/README.md)
- [Majiro](crates/crysirror-majiro/README.md)
- [AliceSoft System 4](crates/crysirror-system4/README.md)
- [System-NNN](crates/crysirror-system-nnn/README.md)

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
