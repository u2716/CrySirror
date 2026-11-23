# Siglus Gameexe bootstrap material — 2026-11-06

## Implementation

The desktop detector previously supplied empty decode material when key.toml
was absent and described every Gameexe decode error as a missing key file.
The native adapter had the same missing defaults and recovery path.

The fixed Gameexe/scene format tables and production resource-pair recovery
rules now come from siglus_rs at
`5f358ef1360b0439e92aed2b749e8f25d8721954`. Exact source hashes and local bounds
are recorded in the VM crate's
[decode-source.json](../../crates/crysirror-siglus-vm/decode-source.json).
Detection and adapter bootstrap share one material resolver. The adapter uses
an owned preparation task; an immutable, bounded content cache avoids duplicate
recovery. Original files are read only and recovered keys are not logged or
serialized into the detection plan.

## Compilation

- `cargo check --release -p crysirror-siglus-vm -p crysirror-desktop`: passed.
- `cargo build --release -p crysirror-desktop --bin crysirror`: passed.
- Existing workspace warnings remain. No tests were added or executed.
- No reflective class/command schema or native snapshot layout changed.

## Observed behavior

The newly built release player ran `play ~/Downloads/game-anemoi --detect-only`.
It completed with exit code zero, identified Siglus and parsed the native
1920x1080 canvas. The Gameexe encryption-mode word was 1, and the game directory
contained no key.toml before or after detection. Automatic recovery admitted
the key only after its Gameexe and Scene resource validations passed.

This run exercised actual resource decoding and detection. It did not start
the game VM, create a window/audio device, read the user's screen or execute
the original EXE/DLL. Native gameplay, append packs and save behavior are not
established by this result. Private reports are under
`target/verification/siglus-anemoi-*`.
