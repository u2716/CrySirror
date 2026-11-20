# crysirror-unity

Bounded readers for Unity 4.x player data (serialized file format 9 without
type trees), producing engine-owned data: object tables, GameObject/Transform
hierarchies, MonoScript names, Texture2D level 0 (Alpha8, ARGB4444, RGB24,
RGBA32, ARGB32, DXT1, DXT5), Material properties, Font TrueType data, Sprite
rects, uncompressed Mesh vertex/index data, renderer material lists, shader
names, `ResourceManager` paths and `BuildSettings` levels. `managed.rs`
decodes MonoBehaviour fields under a consumer-supplied schema (see
`tools/unity_managed_schema.py`); `gltf.rs` assembles hierarchies into GLB for
the world renderer. No Unity code, scripts or devices are hosted. Other Unity
versions and compressed meshes fail visibly.
