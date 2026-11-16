# N-System native adapter

[Command list and ABI profile](../../docs/native-api/nsystem.md).

Onboarding mode: **adopt** (ADR 0102). This crate owns the original MSD
(`MSCENARIO FILE`) reader and the `CDIBSnrScreen` command semantics of the
N-System engine, plus the native message window, choice, action and title
screen policies. Pixels, glyphs, layers, transitions, audio, movie and storage
go through shared CrySirror services. No original EXE/DLL is executed.
Provenance: `SOURCE.json` (HANANYU.EXE, static IDA analysis; every handler
comment cites the original method address).

## Layout

- `script.rs` — MSD decryption (per-build window-title MD5 block XOR,
  0x442B60), header/label/load-point tables, operand decoding, setup records.
- `message.rs` — `MESRUN` items to placed characters: native widths, line
  pitch, kinsoku backoff (table 0x5CD138), click/page tokens.
- `adapter.rs` — `NSystemVm` (`VmAdapter`): layer pages (2×16, commit at
  `OVERLAP`), OVERLAP effects, audio, message window from MESWND.MSD and
  sprite regions, SELMAKE/SELRUN, LYCOLOR, ACTMAKE scroll actions.
- `adapter/title.rs` — `CDIBTitleScreen`: logos, opening movie, name fades,
  logo zoom, menu, idle movie replay; `END` returns to the menu.

## Shared capabilities added for this engine

- `crysirror-formats-arc` MGD: the compression field is a u16 (0x43C2F0);
  `images_c::mgd::regions` exposes the sprite region table.
- `crysirror-scene` transitions: `Mask.vague` (continuous soft edge, Alpha
  mode 2), `SpinZoom` (Rotate), `Bounce` (Bound), `DoorFlip` (RotDoor).
- `crysirror-formats-arc` legacy archives: the entry-size budget no longer
  rejects Raw (stored) members at index time.

## Native screens and files

- `adapter/panel.rs` — native buttons/sliders (`CDIBButtonCtrl`, `CDIBScrollCtrl`).
- `adapter/screens.rs` — save/load and confirmation screens; `adapter/option.rs`,
  `adapter/omake.rs` — option and omake (CG/scene/music) screens.
- `adapter/slots.rs`, `save.rs`, `system.rs` — original GAMEnn.SAV, CONFIG.SAV
  and Novel.ini formats, LPT snapshot and transactional load.

## Not adopted (fail visibly)

OVERLAP types other than Blt/Fade/Alpha/Rotate/Bound/RotDoor, ACTMAKE types
other than linear Scroll, camera depth, movie layers in slots, call stacks in
slots. Not drawn: page-turn wipe, MWO/MWC slide. See the native-api page for
inferred details.
