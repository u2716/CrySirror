# Engine and MCP extension guides

The original static-scene-only overview has been retired. Dynamic World state,
PBR/assets, animation, physics, gameplay JS/TS and capability-gated hardware ray
queries now have implementations; their limits and validation are documented
with their owning systems.

- [Engine systems](ENGINE_SYSTEMS.md) and [architecture](ARCHITECTURE.md)
- [Generated classes, commands and script bindings](api/README.md)
- [Frame pipeline, profiling and streaming](FRAME_PIPELINE.md)
- [Native adapters and capability extraction](NATIVE_ADAPTERS.md)
- [MCP setup, available tools and fresh-session semantics](MCP.md)
- [Offline TS/JS project recipes](AUTHORING.md)

The legacy core `Require3d` instruction is a separate contract from the modern
World API. Its rejection must not be interpreted as absence of the engine's 3D
implementation. See the [documentation index](README.md) for other guides.
