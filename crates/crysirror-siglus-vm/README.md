# Siglus capability reference VM

The default Siglus engine uses `integrated` in
[crysirror-siglus-integrated](../crysirror-siglus-integrated/README.md).
This crate preserves the previous extraction implementation for explicit
capability fixtures through `AdapterRegistry::with_reference_siglus()`; it is
not automatically registered or used by the default player.

## Decode material

`keys.rs` and `key_recovery.rs` adopt the upstream fixed format tables and
resource-pair recovery rules at the revision recorded in
[decode-source.json](decode-source.json). The per-game 16-byte key is not
bundled. The loader recovers it from caller-owned Gameexe.dat and Scene.pck,
then validates the result against both resources before accepting it.
Configured keys remain accepted under the upstream patched/easy-link policies;
non-converging or ambiguous recovery fails visibly. key.toml remains an optional
override rather than a required file for the standard resource profile.

`decode_material.rs` supplies bootstrap policy to this reference adapter.
The default integrated crate maintains its own upstream bootstrap modules. Direct
reference-adapter boot uses an owned preparation task and admits no script/input while
material is pending. A bounded content-identity cache avoids repeating recovery
between detection and VM startup. Recovered material is not printed in the
launch plan or written into the game directory.

Shared LZSS decompression retains the engine's 64 MiB output budget. Native
recovery uses at most four joined scoped workers; the browser compilation path
uses the serial solver. This does not establish browser/console game behavior.
No original EXE/DLL execution or external extraction tool is needed.

This bootstrap change does not implement the separate, currently rejected
append-pack resource path or certify every native operation. Native message,
render/audio and save contracts are described in
[the adapter guide](../../docs/NATIVE_ADAPTERS.md).
