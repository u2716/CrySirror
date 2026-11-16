# CrySirror documentation

## Play games

| Topic | Guide |
| --- | --- |
| Start a game, choose fonts and save locations, or use network storage | [Desktop player](DESKTOP_API.md) |
| Supported engines, resource formats and save-file compatibility | [Engine compatibility](NATIVE_ADAPTERS.md) |
| KAG scenarios, graphics extensions, movies and supported DLL interfaces | [KiriKiri plug-ins](KIRIKIRI.md#plug-in-compatibility) |
| BGI installed-game and installation-media directories | [BGI / Ethornell](BGI.md) |
| macOS, iOS and Android application packages | [Native applications](NATIVE_PACKAGING.md) |

## Engine APIs and authoring

| Topic | Guide |
| --- | --- |
| Engine classes, properties and commands | [API reference](api/REFERENCE.md) |
| JavaScript / TypeScript bindings | [TypeScript declarations](api/crysirror.d.ts) |
| Machine-readable API definitions | [api.json](api/api.json) |
| World, layers, UI, text and audio | [Engine systems](ENGINE_SYSTEMS.md) |
| Scene recipes and behaviours | [TS/JS authoring](AUTHORING.md) |
| MCP project tools | [MCP](MCP.md) |
| Images, archives, audio and video formats | [Resources](RESOURCES.md) |
| Image processing and composition | [Bitmap operations](BITMAPS.md) |
| Rigid bodies, collision and character control | [Physics](PHYSICS.md) |
| Rendering, streaming and profiling | [Frame pipeline](FRAME_PIPELINE.md) |
| Project imports | [Local imports](LOCAL_GAME_IMPORT.md) |

## Embedding and development

- [Architecture and data ownership](ARCHITECTURE.md)
- [Asynchronous services](ASYNC_SERVICES.md)
- [Memory budgets and resource ownership](MEMORY_MANAGEMENT.md)
- [NFS/SMB storage, readers and cache ownership](NETWORK_STORAGE.md)
- [PS Vita characteristics](../platform/psv/PLATFORM.md) / [Switch characteristics](../platform/switch/PLATFORM.md)
- [Engine onboarding and research/archive skills](ENGINE_ONBOARDING.md)
- [Native opcode and syscall implementation lists](native-api/README.md)
- [Integrated engines: upstream versions, local changes and rendering](INTEGRATED_ENGINES.md)
- [Framework saves](SAVE_FORMAT.md)
- [Original save versions and restoration workflow](../.agents/skills/crysirror-native-save/SKILL.md)
- [Platform lifecycle, packaging and release workflow](../.agents/skills/crysirror-platform-host/SKILL.md)
- [Game child-window visual and interaction design](design/IN_GAME_WINDOWS.md)
- [RealLive adapter APIs](native-api/reallive.md)
- [API reference generation](api/README.md)
- [Architecture decisions](adr/README.md)
- [Engine roadmap](adr/0041-engine-capability-roadmap.md)
- [Application icons](../icon/README.md)
