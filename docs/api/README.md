# API reference maintenance

The public API reference is generated from runtime declarations. Edit the
source documentation and declarations, then regenerate the artifacts together.

| Artifact | Purpose |
| --- | --- |
| [REFERENCE.md](REFERENCE.md) | Classes, properties, methods, commands, data specifications and adapter configuration. |
| [crysirror.d.ts](crysirror.d.ts) | TypeScript declarations used by JS/TS callers. |
| [api.json](api.json) | Machine-readable reference for tools and MCP. |
| [CHANGES.md](CHANGES.md) | Signature fingerprints, breaking changes and migration instructions. |

## Regenerate

Run from the repository root:

```sh
cargo run --release -p crysirror-engine --example api_reference
```

The [generator](../../crates/crysirror-engine/examples/api_reference.rs) writes
the reference, TypeScript declarations and JSON. Review their changes together.
Documentation-only prose changes do not require a new signature fingerprint.

## Sources and ownership

| Source | Owns |
| --- | --- |
| [ClassRegistry](../../crates/crysirror-engine/src/object.rs) | Class/member names, types, writability, documentation and script globals. |
| [Command API map](../../crates/crysirror-engine/src/services.rs) | Typed command variants and the API members that issue them. |
| Rust struct/enum declarations and `///` comments | Specification fields, variants, types and descriptions. Defaults come from serde serialization. |
| Native adapter configuration declarations | Public adapter configuration fields; native opcode/syscall semantics remain in the [engine references](../native-api/README.md). |

Rust-only host methods that have no reflective signature belong in their Rust
documentation and the relevant [engine-system contracts](../ENGINE_SYSTEMS.md).
Use [asynchronous services](../ASYNC_SERVICES.md) for task/completion ownership,
[resource APIs](../RESOURCES.md) for provider and format contracts, and
[frame pipeline](../FRAME_PIPELINE.md) for rendering and publication.

## Changing the API

### Compatibility assessment

Before changing a shared API for native engine support, document the native
opcode/syscall contract and determine whether an existing capability already
matches it. Review callers across Rust, JS/TS, JSON commands, adapters and the
affected platform hosts. Classify both source/signature and behavioral impact:

| Change | Compatibility questions |
| --- | --- |
| Member, parameter, field or variant | Do names, types, required fields, writability or exhaustive Rust matches change? An added enum variant or struct field may break existing callers. |
| Defaults, errors or accepted values | Does the same existing input now behave differently, fail, or take an incompatible fallback? |
| Completion and ownership | Do clocks, waits, callbacks, cancellation, thread affinity or resource lifetime change for existing callers? |
| Serialized project/save/command data | Can existing data still be decoded with the same meaning? Does this need an explicit schema/profile migration? |
| Backend requirements | Does an existing host now require a new service, device feature or unsupported platform implementation? |

Prefer extensions that preserve the existing contract. For a necessary break,
record old/new behavior, affected consumers, migration and versioning before
applying it. Keep native ABI, public API and project/save schema versions
distinct. A signature fingerprint alone cannot detect a behavioral break;
Rust-only APIs also need compatibility review even without reflection. The
steps below govern generated artifacts; prose changes do not alter fingerprints.

### Updating declarations and generated artifacts

1. Document every public class, property, method and specification field at its
   declaration. Regenerate all three artifacts after changing declarations.
2. Record signature fingerprint changes in [CHANGES.md](CHANGES.md), with the
   affected members and caller migration. Names, types, writability, fields,
   defaults, command variants and adapter configuration are signature changes.
3. Treat renamed/removed members, changed variants and changed defaults as
   breaking changes. Project and save schemas are versioned separately in
   [save formats](../SAVE_FORMAT.md).
4. Keep implementation progress, build logs and native audit receipts out of
   this maintenance guide. Validation belongs in focused notes under
   [docs/validation](../validation/).
