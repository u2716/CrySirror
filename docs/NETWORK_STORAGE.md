# Network game storage

NFS and SMB are ordinary native game-source paths backed by scoped providers.
Use the [network-storage skill](../.agents/skills/crysirror-network-storage/SKILL.md)
for changes and the [desktop guide](DESKTOP_API.md#games-on-an-smb-share) for
launch commands and credential/options syntax. This document describes source
contracts; it adds no transport implementation or execution evidence.

## Source ownership

| Source | Responsibility |
| --- | --- |
| [network contracts](../crates/crysirror-network/src/lib.rs) | `RemoteStorage`, metadata, credential-free origin and client subdirectory authority. |
| [SMB](../crates/crysirror-network/src/smb.rs) | Read-only share, bounded worker/read cache and native protocol errors. |
| [NFS](../crates/crysirror-network/src/nfs.rs) | Export/version/options, transport owner, bounded RPCs and connection retirement. |
| [launch source](../crates/crysirror-launch/src/source_path.rs) | Local/remote input selection, detection metadata and scoped readers. |
| [native readers](../crates/crysirror-network/src/native_io.rs) / [native filesystem seam](../crates/crysirror-network/src/native_fs.rs) | Integrated-runtime read/seek/filesystem adaptation and local user overlays. |
| [integrated roots](../crates/crysirror-launch/src/integrated_roots.rs) | Feature checks, remote resource bindings and separate local save roots. |

Do not require OS mounting, a helper process or downloading a whole game. Reuse
the provider from detection through runtime archive/resource access. Shared
parsers use `AssetReader`; integrated runtime readers use their retained native
I/O seam. Any remaining unadapted local-path/mmap/plugin requirement is a
specific gap, not permission to substitute a different renderer/runtime.

The native filesystem seam can expose a private `.game-network-*` path token.
It is an in-process provider identity, not a directory of staged game data.
Do not populate it with a full game or pass it to arbitrary OS file APIs. Its
mount/provider lease must outlive native readers using that identity.

## Protocol and identity

| Topic | SMB | NFS |
| --- | --- | --- |
| Input | `smb://server/share/game` | `nfs://server/export/game`, with supported export/version options. |
| Authority | Selected server/share, then game/client subdirectory | Selected export and game/client subdirectory within the server namespace. |
| Authentication | Existing URL/environment credential inputs | Supported AUTH_SYS UID/GID mapping, not SMB-style password authentication. |
| Options | Inspect the provider's accepted URL fields | Inspect version/export, UID/GID, source-port and mount-port parsing. |
| Failure/reconnect | Disconnects surface errors; no general automatic reconnect contract | Timed-out operations retire the connection; reopen is required for the documented path. |

Check the current provider/version before adding options or extending these
contracts. Do not automatically change server export permissions, disable
security, or request privileged mounting to repair a client failure. Source
identity and diagnostics omit credentials. Avoid URL passwords in command lines
and do not print complete environment/process arguments during diagnosis.

Remote assets are read-only. Writable native user files/saves remain in their
local scoped overlay. Preserve overlay-before-asset lookup and adapter-specific
NLS, patch ordering and original-save policy. URL origin, game identity and
native resource names are distinct; a change to one may affect local save
lookup and needs compatibility review.

## I/O, caches and failure semantics

The current providers use 256 KiB blocks with 128 cached blocks, a nominal
32 MiB block-cache capacity per connection. They also retain directory metadata,
reader/connection state and active replies; that block cap is not a complete
process-memory limit. External reader leases and decoded resources may outlive
cache eviction. Review [memory accounting](MEMORY_MANAGEMENT.md) across queues,
archive decoders and GPU consumers before increasing concurrency or prefetch.

Preserve short reads, EOF and random-access offsets, and bound index/directory
work. Network work belongs on the existing bounded transport/resource workers;
do not block the realtime output thread because a facade offers synchronous
`Read`/`Seek`. Close/cancellation must preserve active owners and retire stale
results. Existing cached metadata is not a universal live-update guarantee:
document which changes can be detected and whether a session must reopen.

The providers retain typed not-found where available; many other failures are
I/O errors with protocol context. Do not turn them into missing optional files
or fabricate finer error categories that the implementation cannot expose.
Transport failure, resource corruption and an unsupported archive profile need
different diagnoses. HTTP/WebDAV/SFTP game URLs are not implied by these
providers; the separate granted HTTP request service is not a mounted directory.

## Availability and validation

The current desktop manifest enables `smb` and `nfs` by default and forwards the
features to the integrated runtimes. Custom feature selections still need the
matching providers. Inspect the actual build, not only a library's default
features. See [desktop NFS options](DESKTOP_API.md#games-on-an-nfs-export).

PSV/Switch need an actual compatible socket, thread, timer and transport backend;
their renderer or package source does not establish network storage. Mobile
lifecycle/network access and browser provider/gateway support also need separate
implementation and evidence. No console/browser network acceptance follows
from desktop feature compilation.

Validate connection/listing, launch detection, random archive-entry reads and
native runtime loading separately. Check the local save overlay and the changed
fault/timeout path when in scope. Record credential-free source, native profile,
build features and observed behavior. Keep raw logs/private data local and
bounded. A directory listing or feature declaration is not full-game acceptance.
