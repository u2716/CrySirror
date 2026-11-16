# BGI upstream integration

Mode: `integrated`.

| Item | Recorded value |
| --- | --- |
| Repository | [https://github.com/xmoezzz/ethornell-rs](https://github.com/xmoezzz/ethornell-rs) |
| Complete-runtime base | `0bc6d1c0d5b662acc9006be94896324600ad05f1` |
| Source record | [SOURCE.json](SOURCE.json) |

Previous synchronized revision: `2f8a9272a8f7ca075d82ac88fb802fa7247b3a99`.

## Changes retained in CrySirror

- CrySirror-owned session worker, immutable completed frames and session-owned 48 kHz stereo PCM. The host output queue uses the upstream independent scene/movie schedule.
- Keep host-selected bootstrap, font, input/dialog handling and installed-resource view. Launch detection uses the retained BP image bounds and language-tag policy.
- Primary-root native writes use the existing private user overlay; this subsumes the upstream BGI.gdb alias while keeping the resource directory read-only. Upstream test cases that require its narrower writable-name whitelist are not imported.
- Upstream mobile audio route recovery remains behind the optional non-host backend; integrated sessions retain CrySirror-owned devices and PCM.
- Keep existing GPU namespace, file-policy/platform seams and MPEG codec boundary; no standalone upstream launcher or audio device is enabled.

When updating, compare the new upstream revision with the source record and
retain the host changes above. Update this record and SOURCE.json together.
Later selected imports, rendering/output paths and platform boundaries are in
[the integration reference](../../docs/INTEGRATED_ENGINES.md). Compilation and
observed behavior belong in the linked validation notes, separately from the
imported source revision.

Latest synchronization: [2026-11-16 validation](../../docs/validation/bgi-upstream-sync-2026-11-16.md).
