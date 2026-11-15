---
name: crysirror-archive-onboarding
description: Add or extend archive formats in CrySirror's shared resource and VFS pipeline, including native layout analysis, bounded random-access readers, transforms, keys, registration and mounted-resource validation. Use for archive support rather than a complete VM port.
---

# Archive formats through shared Resources

Read the repository's `AGENTS.md` and [resource contracts](../../../docs/RESOURCES.md).
Reuse current shared parsers and matching native findings. Use
[online research](../crysirror-engine-research/SKILL.md) for missing references and
[original analysis](../crysirror-reverse-engineering/SKILL.md) for unresolved
native layouts. Without matching official material, original-program reverse
evidence governs; unofficial unpackers are references, not guaranteed parity.

## Locate the integration boundary

| Source entry point | Responsibility |
| --- | --- |
| [resource traits](../../../crates/crysirror-resource/src/lib.rs) | `ArchiveFormat`, `Archive`, `ArchiveEntry` over scoped `AssetReader` inputs. |
| [format registry and mount](../../../crates/crysirror-resource/src/registry.rs) | `FormatRegistry`, bounded probing, explicit format selection and `ArchiveMount` VFS publication. |
| [decode context](../../../crates/crysirror-resource/src/context.rs) | Client authority, limits, cancellation, companion resolution and profile parameters. |
| [transforms](../../../crates/crysirror-resource/src/transforms.rs) / [keys](../../../crates/crysirror-resource/src/keys.rs) | Reusable byte transformations and client-scoped secret lookup. |
| [format plugin registration](../../../crates/crysirror-formats-arc/src/lib.rs) | Existing shared native format registrations and their implementation modules. |
| [public Resources implementation](../../../crates/crysirror-engine/src/resource_api.rs) | Engine-facing probing, metadata, async decode and mounted outputs. |

Put format-specific layout/resource policy in an appropriate maintained workspace
format module. Add reusable I/O, transforms or decoding mechanisms to shared
services. Do not shell out to an unpacker or require a copied/extracted game
directory. An integrated runtime may retain its native parser under its existing
host seam; this skill does not mandate migrating that runtime to an adopt
boundary or replacing its complete resource implementation.

## Recover the format/profile

Identify the actual version and record source revision or original binary/input
digest. Follow header/index parsing, name decoding, lookup, entry reads,
decompression/transforms and failures. Recover offset bases and widths,
alignment, split-volume/companion rules, stored versus decoded sizes, hashes,
checksums, filename encodings and title-specific parameters. Preserve duplicate
lookup and patch priority as explicit native policy where the shared mount
contract cannot represent it directly.

A shared extension, successful header match or a plausible output is insufficient
evidence for all variants or a correct key. Preserve unsupported cases visibly;
do not silently choose an unrelated parser or return empty content.

## Implement readers and publication

Use [memory management](../crysirror-memory-management/SKILL.md) for reservations,
decoded residency, cancellation and overlap with GPU consumers. Format byte
limits alone do not establish a process-wide memory budget.

For NFS/SMB inputs, follow [network storage](../crysirror-network-storage/SKILL.md).
Use the same scoped random-access reader for archive indexes, segments and
companions without a full download or native OS-path requirement.

- Implement bounded `ArchiveFormat::probe` and full `open` validation. Match
  content evidence; ambiguous registry matches need explicit selection. Do not
  assume the registry chooses the highest confidence score.
- Keep native offsets and segment plans typed inside the parser. Validate
  checked arithmetic, source ranges, entry/index counts and decoded sizes before
  reads or allocations. Validate actual decompressor completion and integrity.
- Expose entries through `Archive::open` as `AssetReader` streams. Reuse shared
  subrange/concatenation and transform helpers; support short reads, EOF and
  unaligned/random access correctly. Materialize compressed entries only within
  the existing byte limits, not whole archives by default.
- Keep client capability roots, normalized names and entry IDs valid at the
  mount boundary. Traversal, duplicate canonical paths and cross-client access
  cannot be silently accepted. Request an explicit shared capability when a
  native lookup rule needs more than the existing contract supports.
- Use `DecodeContext`, `KeyProvider` and companion resolution. Profile parameters
  carry identifiers/options, not secret bytes, ambient paths or device pointers.
  Do not invent title keys or serialize keys in project/save data.
- Preserve async ownership: workers receive immutable inputs, cancellation and
  bounded reservations; the owner publishes completed mounts/resources. Use
  existing Resources jobs for live hosts and release late cancelled results.
- Route extracted image/audio/file payloads to existing shared decoder paths.
  Opening an archive is not VM execution, pixel parity or audio completion.

Prefer mount-in-place and representative entry reads. Do not duplicate a full
game or create another target directory for routine validation. Resource parsing
must not require a window, GPU, native OS file path or game process; assess
browser/mobile/console storage and task limitations separately.

## Register, verify and document

Register the implemented format through the existing plugin/host route. Expose
it through normal Resources operations and installed-game mounts where required.
Keep native launch ordering in the engine adapter/launch plan; a parser's
registration alone does not wire a game into the player.

Use the smallest authorized release check and representative existing fixtures
or game entries. Verify recognition, index metadata, mounted reads, random reads
and the changed truncation/overflow/transform paths as appropriate. Add tests
only where needed to expose a new semantic or lifetime risk. Do not run a whole
game/regression suite merely because a parser changed.

Document the format/profile, source identity, registration route and gaps beside
the parser and in the relevant resource support ledger. Separate implementation,
compilation and exercised behavior. Preserve provenance and applicable notices;
`UPSTREAM.md` remains reserved for Siglus, BGI, SoftPAL and RFVP. If public APIs
change, follow [API generation](../../../docs/api/README.md). Keep raw game data,
keys and large traces private. A documentation-only skill change needs no Cargo
build or game execution.
