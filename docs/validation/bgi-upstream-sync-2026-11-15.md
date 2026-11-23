# BGI upstream synchronization, 2026-11-15

## Source implementation

The integrated runtime is synchronized from
`3ee43dab43854e2f9c614dd1d26213e17911d877` to
[`2f8a9272a8f7ca075d82ac88fb802fa7247b3a99`](https://github.com/xmoezzz/ethornell-rs/commit/2f8a9272a8f7ca075d82ac88fb802fa7247b3a99).
The latter includes PR #127's newer native operations and PR #129's substitute
font sizing. The original upstream checkout and its branch were not changed;
the recorded remote revision was read directly.

| Area | Imported behavior |
| --- | --- |
| VM | Newer ABI profiles, compact/modern/fused BP decoding, native buffers, pointer tags, extension/version dispatch and memory checks. |
| Graphics/input | Linked sprite masks, animated bitmap lifetime, raw image formats, knob routing, projected sources and native graph operations. |
| Fonts | Game-directory font candidates, native named transforms, coverage reduction and substitute-font metrics. |
| Storage | Publisher language archive views, UserData queries and raw archive path/read handling. |
| Lifecycle/presentation | Normal root completion and independent scene/global-video sample scheduling. |

The [source record](../../crates/crysirror-bgi-integrated/SOURCE.json) contains
163 upstream file digests and their current local digests. Source changes were
reconciled against the previous pin and a snapshot of the dirty local tree.
Existing host-only files, capability I/O and native GXM modules were retained.
All 163 resulting local digests were checked against the final record.

## CrySirror boundary

Mode remains `integrated`. The host continues to own the session worker,
window/events, output device, frame queue and 48 kHz PCM queue. The current CPU
frame-output path is preserved; this synchronization does not migrate it to GPU
composition. Its bridge now uses the upstream CPU presenter's schedule and
publishes only changed completed samples, so global movie samples do not depend
on an unrelated scene presentation deadline. The retained direct-renderer path
uses the updated upstream GPU presenter.

The following local policies remain intentional:

- Custom/loose bootstrap names and the installed-resource view remain valid.
- Canonical system bootstrap selects the original manager's coherent language
  view. Loose `ipl._bp` does not require a system archive.
- Launch detection delegates declared image bounds and supported archive tags
  to the retained runtime. BP tail bytes no longer cause premature rejection.
- Windows/Unix scoped storage, local user overlays, network overrides and media
  sentinels remain CrySirror-owned. Upstream tests requiring its narrower write
  whitelist remain excluded rather than redefining this integration's policy.
- Native console drivers and host event/PCM bridges remain in place. Upstream
  standalone launchers, signing workflows and lockfiles are not substituted for
  CrySirror packaging.

`getrandom` is connected on upstream-supported hosts, including its browser
feature. The upstream console error for an unavailable secure random source is
preserved; compilation does not establish that operation on those devices.

## Compilation

| Command | Observed result |
| --- | --- |
| `cargo check --release -p crysirror-bgi-integrated` | Passed. |
| `cargo check --release -p crysirror-bgi-integrated --features nfs,smb` | Passed after reconciling duplicate network constructors introduced by the three-way merge. |
| `cargo build --release -p crysirror-desktop --bin crysirror` | Passed, 3m21s. Updated `target/release/crysirror`. |
| `VITASDK=/usr/local/vitasdk cargo +nightly vita build vpk -- --release` in `platform/psv/player` | Passed, 1m57s plus packaging. Updated the release VPK. |

The existing native adapter configuration and reflective engine signatures did
not change. The two Rust-only bootstrap inspection helpers have no reflective
signature, so no `docs/api` fingerprint change is introduced.

## Behavior scope

No test suite, regression run, BGI game, original executable or desktop capture
was executed for this synchronization. These are source and compilation
results. Previous BGI game observations remain evidence for their own source
versions. Windows/Linux/mobile/browser/Switch builds and real-device behavior
were not validated here. PSV VPK compilation is not PSV gameplay acceptance.
