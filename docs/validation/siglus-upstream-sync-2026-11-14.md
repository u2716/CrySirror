# Siglus upstream synchronization — 2026-11-14

## Implementation

- Mode remains `integrated` in `crysirror-siglus-integrated`.
- Previous complete-runtime base: `5f358ef1360b0439e92aed2b749e8f25d8721954`.
- New complete-runtime base: `bef39ea90c745a4888a92a09d18be1eecfdd10ca`.
- Three-way comparison used the original upstream base, the existing CrySirror
  integration including local changes, and the new upstream source. All previous
  source-inventory hashes matched the old base.
- Updated 26 internal source files. The baseline contains the Unicode layout,
  font lookup, save/menu/window and mask inheritance fixes. Added the three
  upstream Unicode dependencies.
- Preserved offscreen presentation, host native UI/PCM, capability roots,
  writable save roots and injected file/byte fonts. Vita shader include and
  Switch PCM adaptations remained byte-identical to their pre-sync local state.
- Standalone upstream app entry points and GitHub release workflows are not
  CrySirror host implementations and were not copied. No font/game data was imported.
- [Source records](../../crates/crysirror-siglus-integrated/SOURCE.json) contain
  the new source mapping and hashes for all 192 mapped source/shader files.

## Compilation

All three commands completed successfully using the existing release target:

```sh
cargo check --release -p crysirror-siglus-integrated
cargo check --release -p crysirror-siglus-integrated --features nfs,smb
cargo build --release -p crysirror-desktop --bin crysirror
```

The commands emitted warnings and did not execute tests. The desktop build
finished in 7m 00s and replaced `target/release/crysirror` (151,203,584 bytes).
Its SHA-256 is
`4e44e4ecae1113d74634d61d017bfe3a7009cb6b0fc66652629a13f05a3bd2a9`.
The sync changes no public CrySirror API or adapter configuration schema.

## Behavior scope

Source and compilation validation only. No game/emulator/window was started,
no screen was read, and no pixel, audio-device, mobile or console behavior is
certified by these checks. Earlier behavior receipts remain scoped to their
recorded versions.
