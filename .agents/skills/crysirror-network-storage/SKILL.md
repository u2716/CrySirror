---
name: crysirror-network-storage
description: Maintain NFS and SMB game-source support through CrySirror's scoped storage, random-access readers and integrated runtime I/O seams. Use for remote paths, archive reads, credentials, bounded caches, timeout behavior and per-platform network capability review.
---

# Network game sources through shared storage

Read `AGENTS.md`, [network storage](../../../docs/NETWORK_STORAGE.md) and the
affected provider/adapter source. Native `nfs://` and `smb://` paths belong to
ordinary installed-game launch. Inspect the existing implementation before
adding a new transport, whole-game download or OS mount requirement.

## Trace the complete source path

Follow input URL parsing, credential-free source identity, server/share/export,
selected game subdirectory, detection, directory metadata, archive indexing,
entry reads and the adapter's runtime I/O. Preserve the same scoped provider
through all stages. A network-aware detector with a later local-only resource
open is incomplete support.

Reuse `RemoteStorage`/`Subdirectory`, `SourcePath` and `AssetReader` for adopted
engines. For integrated engines inspect their injected provider/native I/O seam
and local writable overlay. A virtual path token is not an ordinary OS file;
passing it to an unadapted `std::fs`/native library is not remote support. Keep
the selected engine mode and original lookup/encoding/save semantics intact.

## Respect protocol and authority differences

- For SMB, distinguish host/share/game path, actual negotiated protocol and
  credentials. Use the existing supported credential inputs without printing
  passwords, full credential URLs, environment dumps or process command lines.
  Keep credentials out of logs, source IDs, profiles and portable saves.
- For NFS, distinguish the export from the selected directory and the server's
  namespace from its local filesystem path. Inspect version, UID/GID mapping,
  source-port and mount-port options; do not treat them as SMB password auth.
- Preserve percent decoding, canonical relative paths and client roots. Do not
  introduce traversal, cross-client access or unreviewed referral/symlink escapes.
- Remote game assets remain read-only; original native writes use the scoped
  local user/save overlay. Do not silently redirect saves onto the share or
  treat remote resources as disposable local cache files.

Use the user's chosen server/share and existing authorization. Do not discover
credentials by scanning unrelated files, change server exports/permissions,
start an elevated mount helper or scan other servers as an automatic repair.

## Maintain bounded random access and completion

Use existing `read_at`/seek facades for archive segments and native readers.
Handle short reads, EOF, large offsets and directory pagination. Keep compatible
filename and patch lookup behavior across local/remote storage; record provider
encoding restrictions rather than silently renaming files.

Run network I/O on bounded existing workers with defined owner completion.
A synchronous reader facade does not make an RPC safe on the render/audio
thread. Reuse connections/caches; do not create a connection per entry. Budget
block/directory caches, active replies, reader leases, decoder scratch and
prefetch together through [memory management](../crysirror-memory-management/SKILL.md).
Neither whole-game staging nor unlimited prefetch is an implicit fallback.

Keep not-found distinct from authentication, permission, timeout, disconnect and
corruption using the actual error model. A boolean existence query cannot prove
why a lookup failed; use error-preserving operations when native behavior needs
that distinction. Never convert transport failure into empty/successful data.
Document cache consistency and file-change guarantees/gaps. A retry must not
repeat native side effects or keep using stale resource identity.

Respect current timeout/connection-retirement behavior; do not promise automatic
reconnection unless implemented. Define cancellation, pending reads, late
results, close and session teardown before changing transport lifetime. Review
[API compatibility](../../../docs/api/README.md#compatibility-assessment) for
changes to URLs, credential inputs, errors, cache defaults and completion.

## Review availability and verify narrowly

Check actual Cargo feature propagation through the launcher and affected
integrated runtimes. Default desktop features do not establish console/mobile
support. For PSV/Switch inspect socket/thread/timer/allocator dependencies and
the platform's actual network setup. A browser transport/gateway is a separate
capability; native socket clients are not automatically browser-compatible.
HTTP requests are also separate from NFS/SMB game-directory providers.

Use scoped release checks and the smallest authorized source/read workload.
Separate connection/listing, remote detection, mounted archive reads, actual
runtime resource loading, local save overlay and fault behavior. A successful
listing alone does not validate gameplay. Use existing small fixtures/providers
where possible; do not copy a complete game or launch a broad regression run.

Record protocol/profile, credential-free origin, feature/backend, observed paths
and limitations. Keep private captures/logs bounded. A documentation/skill
change does not require a server connection, credentials, a build or a game run.
