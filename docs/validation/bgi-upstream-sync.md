# BGI upstream synchronization — 2026-11-06

## Source implementation

Integrated BGI is synchronized to ethornell-rs
`3ee43dab43854e2f9c614dd1d26213e17911d877` (2026-11-05).
[The source record](../../crates/crysirror-bgi-integrated/SOURCE.json) stores the
upstream and local SHA-256 for 140 imported source/shader files. The previous
import lacked a pinned record; `0cfff1d274ecc4554d0af8af1c7b1c7908595e96`
matched its normalized source apart from existing host changes and was used as
a reconciliation base, without claiming it was the original import commit.

Twenty source files changed, including the new native `stream_io.rs` module:

- Profile-selected 64/128 SE channel banks and larger verified main-thread regions.
- Native sprite bitmap-property and BackML source-motion curve handling.
- Resource stream write modes, transfers, seek/close results and shared state.
- Composition against current bitmap backing, empty-text styling, path/encoding
  handling, empty history records and font fallback candidates.

The complete native runtime remains in workspace modules. Existing host-owned
frame/PCM, bootstrap/font/input/dialog bridges and installed-resource view were
retained. The existing private writable overlay already covers the upstream
`BGI.gdb` alias. Its broader primary-root overlay policy was preserved; the
upstream narrow-whitelist-only test addition was not imported. Actual resources
remain read-only. No Siglus source/configuration was changed by this update.

## Local compilation

`cargo check --release -p crysirror-bgi-integrated` passed in 27.05 seconds.
Private compiler output is `target/verification/bgi-upstream-check.log`.

`cargo build --release -p crysirror-desktop --bin crysirror` then passed in
2m57s. The final full-player output is
`target/verification/bgi-upstream-player-build2.log`.

The full-player build initially found an existing Sugarpot caller missing the
new shared `AudioRequest::SetGain.fade_ms` field. The caller now supplies `0.`,
preserving its immediate menu-volume change. This is a compatibility repair,
not a change to Sugarpot fades or the shared API.

The integrated adapter Config and reflected engine API were not changed by the
upstream merge; no new API fingerprint is introduced by this update.

## Behavior scope

No test suite, game program, original executable or user screen was run/read
for this synchronization. Compilation does not validate a game path, audio
output, rendering parity or Windows/Linux/mobile/browser/console behavior.
Prior game observations in other records remain prior-version evidence.

Final release player SHA-256: `e9c38d4ef90b58692bb02750a4088f0a4fe2adabc7a19acee97a9cc4eefcae11`.
