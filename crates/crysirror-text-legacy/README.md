# CrySirror legacy byte text

First-class shared CPU string operations for native byte-character profiles.
The package is independent of any VM, font, renderer or operating-system API.
Its GPL source boundary is separate from the base `crysirror-text` package.
See `SOURCE.json`, the retained copyright notices and `LICENSE`.

`ByteStringContext` performs bounded, cancellable character indexing, counting,
packed-character reads/writes, substring, search, push/pop/erase and numeric
conversion. A profile declares lead-byte detection and signed single bytes.
The original stored extent and NUL prefix remain distinct. Undefined native
out-of-buffer reads and integer overflow fail explicitly.

`NumericCompaction::PreserveStoredTail` preserves the reviewed in-place numeric
narrowing behavior; `Terminate` provides explicit terminated output semantics.
Adapters retain their heap lifetime, stack, alias and native opcode policy.
No game data or font is included. Tests use authored byte vectors only.
