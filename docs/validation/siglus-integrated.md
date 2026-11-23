# Siglus integrated runtime — 2026-11-06

## Implementation

The default desktop player registers `crysirror-siglus-integrated` as `siglus`.
The complete maintained VM, native resources, renderer, effects, mixer, proc flow
and original-save implementation are internal modules, imported from siglus_rs
revision `5f358ef1360b0439e92aed2b749e8f25d8721954` with source digests in the
[provenance record](../../crates/crysirror-siglus-integrated/SOURCE.json).

One owner worker creates, steps and destroys the native runtime. The renderer
produces complete RGBA backbuffers offscreen; the session-owned Kira backend
produces stereo float PCM at 48 kHz. CrySirror owns presentation, input and output
devices. The [crate contract](../../crates/crysirror-siglus-integrated/README.md)
records the local-folder profile and remaining platform/host work.

The old extraction VM is no longer automatically registered. Its explicit
reference registry is retained for capability fixtures. Project validation no
longer forces the extraction VM's config schema before the selected factory
runs. Native keyboard handling retains Esc/function-key ownership; quick input
pulses retain a subsequent release. Native global saves are fingerprinted and
written through the host-selected writable root.

## Compilation and documentation

`cargo check --release -p crysirror-desktop` passed. A release player build passed
with `cargo build --release -p crysirror-desktop --bin crysirror`; the initial
successful launch evidence below used build 4; the final input-release fix was
compiled successfully in build 6 (3m53s) and exercised in the 6000-tick run.
One shared legacy-mesh early return needed its existing `WorldFrame` wrapper
to compile the current workspace. Private command output is under
`target/verification/siglus-integrated-*`.

The API reference was regenerated with the release `api_reference` example,
without executing tests; fingerprint `20f7dc585b1b21b5` documents the default
integrated configuration. No new test case or regression suite was run.

## Observed game path

Private installed directory: `~/Downloads/game-anemoi`. No original EXE/DLL,
game window or audio output device was opened. No user screen was read.
Captures are generated offscreen render targets, retained only under ignored
`target/verification`.

With `CRYSIRROR_OFFSCREEN_PACING_MS=4`, `play <directory> --headless --no-audio`
completed these observed paths:

| Host ticks | Input | Observed native frontier | Generated picture |
| --- | --- | --- | --- |
| 1200 | None | `_start`, line 38, 945 native redraws | 1920×1080 disclaimer, Japanese text, background and feather effects |
| 3000 | Enter pulses at ticks 400, 1000, 1600 | `_menu`, line 268, 2372 native redraws | KEY brand logo over animated background |
| 6000 (final build) | Same Enter pulses | `__sys_menu`, line 79, 3368 native redraws | 1920×1080 anemoi trial title menu, character background, start/continue/load/config/exit items |

All three reports identify `runtime: siglus-integrated`, `ready: true`, and contain
no VM error. The previous `SET_JOYPAD_MODE_OVERRIDE` capability-consumer failure
did not recur on these paths. `_menu` is a script scene name; the brand-logo
capture alone is not evidence of interactive title-menu coverage. The final
6000-tick capture establishes title-menu presentation, without asserting
untested menu operations or whole-game compatibility.

The game requested `Noto Sans CJK JP DemiLight`, absent from this machine. Native
font lookup used the host-supplied fallback; this does not establish exact font
appearance. No font was bundled in the new crate. The original source's fallback
log still calls this an embedded default, although this integration supplies a
host font and bundles none.

Native `config.sav`, `global.sav`, `read.sav` and `twitter_state.dat` were written
beneath the selected private state directory, not the installed game directory.
The game directory still has no `key.toml`. PCM processing was connected during
the run, but muted offscreen execution does not validate audible playback,
latency or physical-device behavior. No whole-game, mobile, browser or console
compatibility claim follows from these observations.

The final tested player and `target/release/crysirror` matched SHA-256
`a17fd8c07e0ee5c447d34de5d0785176f669a7557183e7eefb2966941ebdad26`.
Final private evidence: `siglus-integrated-final.json`,
`siglus-integrated-final.png`, and `siglus-integrated-player-build6.log` under
`target/verification`. The reports preserve the real native frontier; host tick
counts are not native refresh-rate or playback-performance measurements.
