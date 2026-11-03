# ADR 0006: virtual filesystem providers and mount tables

Status: implemented (partial) on 2026-10-24.

## Requirement

R8: the filesystem is abstract. Native engines such as siglus_rs keep their
own filesystem semantics (archives, name rules, Gameexe/Scene.pck lookups) on
top of it.

## Decision

- `crysirror_core::vfs::VfsProvider` (no_std, read-only: `read`, `exists`,
  bounded `list`). It sees only canonical paths relative to its mount point and
  never sees a host path or a client identity. `MemoryProvider` is the first
  implementation.
- `DiskVfs::mount_provider(client, mount_point, provider)` overlays a provider
  on **one client's** asset namespace. Newer mounts shadow older mounts and the
  capability disk root. `NotFound` falls through, and any other error stops
  the lookup and is returned. Mount points are matched per component. Save and
  cache mounts stay disk-backed with atomic writes. The limit is 64 mounts per
  client.
- `Runtime::mount_provider` exposes this to hosts and plugins. Native VMs read
  through `HostServices::read`, so they see mounts without changing their own
  path or archive decoding. Engine scripts reach the same namespace through
  the `Fs` class.

The VFS contract itself does not decode native archives. Audited reusable
archive parsers and providers now live in shared resource/format crates (ADRs
0044/0065); native adapters retain archive selection and precedence policy.
A generic decoder must not be kept private to a VM merely to get it running.

## Evidence

- `vfs::tests::provider_mounts_shadow_disk_per_client_only` (runtime).
- `provider_tests::siglus_reads_its_scene_through_a_mounted_provider`: the
  original Siglus SceneVm loaded its `.scn` from a memory mount, with no
  file on disk.
- Later capability read-at and provider-backed Assets/resource services replaced
  the initial disk-only limitation. ADRs 0049/0052/0065 add bounded async reads,
  imports, streamed cells and native decoding; remaining cook/upload/lifetime
  work is tracked under ADR 0050. The original test receipts above remain scoped
  to the mount-table decision.
