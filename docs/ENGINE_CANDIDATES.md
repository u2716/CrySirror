# Planned engine support

Candidates are not registered or supported engines. See
[current compatibility](NATIVE_ADAPTERS.md) for implemented adapters and
[engine onboarding](ENGINE_ONBOARDING.md) for acceptance requirements.

| Engine | Intended mode | State | Native data |
| --- | --- | --- | --- |
| codeX RScript (Liar-soft / raiL-soft) | `adopt`, the default under ADR 0102 | Reference research; no VM adapter implemented. | GSC scripts, XFL archives, LWG composition and WCG images. |

## codeX RScript references

Read-only source review, 2026-11-16. No external runtime, conversion tool or
game was executed, and no external implementation was imported.

| Project | Inspected revision | Actual role | Scope and limitation |
| --- | --- | --- | --- |
| [vn-web-engine](https://github.com/SirOlaf/vn-web-engine) | [`ded89356`](https://github.com/SirOlaf/vn-web-engine/tree/ded893567114df9af37e26a7c4ef00bfba1b8ef0/src/engines/rscript) | TypeScript GSC interpreter and game runtime with browser output; MIT. | Documents RScript 1.9.0.0 and 1.11.0.3 profiles, native saves and four exercised titles. Its documented missing opcode handlers are not complete engine coverage. |
| [rscript2renpy](https://github.com/Love-in-idleness/rscript2renpy) | [`201560ad`](https://github.com/Love-in-idleness/rscript2renpy/tree/201560ad36b3b963bbca882fc81b9921cf8383fe) | Ren'Py 8 compatibility runtime and game-specific builders; MIT code. | Builders consume converted TSC and prepared resources, not original GSC directly. Requires Ren'Py and title adaptation. |
| [LiarsoftTool](https://github.com/Love-in-idleness/LiarsoftTool) | [`619b6807`](https://github.com/Love-in-idleness/LiarsoftTool/tree/619b680741e2baedb7409ed4efb21aa95dd483b7) | C++ resource/script conversion tools; GPL-3.0. | XFL/LWG, WCG/LIM, GSC/TSC and wrapped audio. Its experimental Windows runtime extends the original game through DLL hooks; it is not a standalone portable VM replacement. |
| [gscScriptCompAndDecompiler](https://github.com/TesterTesterov/gscScriptCompAndDecompiler) | [`b9e29c97`](https://github.com/TesterTesterov/gscScriptCompAndDecompiler/tree/b9e29c97429838b9b6ca29861e7a50b4a10c370e) | Python GSC decompiler/compiler; GPL-3.0. | Useful operand/control-flow research, not a game runtime. Its command reference explicitly retains unknown operands. |

The browser implementation is the most direct execution reference among these
sources: [interpreter](https://github.com/SirOlaf/vn-web-engine/blob/ded893567114df9af37e26a7c4ef00bfba1b8ef0/src/engines/rscript/vm/interpreter.ts),
[native handlers](https://github.com/SirOlaf/vn-web-engine/blob/ded893567114df9af37e26a7c4ef00bfba1b8ef0/src/engines/rscript/runtime/opcodes.ts)
and [profiles/known gaps](https://github.com/SirOlaf/vn-web-engine/blob/ded893567114df9af37e26a7c4ef00bfba1b8ef0/src/engines/rscript/README.md).
Its missing-handler branch reports a diagnostic and yields a frame. That is a
compatibility policy, not evidence of an original no-op, and must not be copied
as successful execution of unsupported CrySirror operations.

These are unofficial implementations. Their playthrough and parity claims are
author reports, not CrySirror validation. Without matching official material,
recover behavior from the original program and resolve discrepancies against
that evidence. Shared ancestry between tools is not independent confirmation.

Before implementation, identify the target build and document separate native
ABI/opcode/syscall tables. Keep parser layouts distinct from executed handlers.
Then map VM waits, text, images, effects, audio and saves to shared CrySirror
services. A browser or Ren'Py host does not establish PSV/Switch, original-save
or mobile lifecycle support here. No integration-mode migration is decided by
this reference list.
