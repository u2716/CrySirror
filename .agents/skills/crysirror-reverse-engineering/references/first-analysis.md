# First analysis: game directory to native contract

Start from the requested behavior and matching existing evidence. These are
analysis milestones, not a requirement to decode every file before implementing
the next reviewed slice.

## Identify inputs

Inspect directory names, configuration and bounded headers first. Identify the
original executable, engine DLLs and representative script/archive files. Record
the game/patch edition, sizes and SHA-256 digests of the actual analysis inputs.
For source, record the repository and commit. Do not run an EXE to identify its
architecture.

Examples for one chosen input, replacing the quoted placeholder:

```sh
# macOS
file "/path/to/Game.exe"
shasum -a 256 "/path/to/Game.exe"
```

```sh
# Linux
file "/path/to/Game.exe"
sha256sum "/path/to/Game.exe"
```

```powershell
# Windows PowerShell
Get-FileHash -Algorithm SHA256 -LiteralPath 'C:\Games\Example\Game.exe'
```

On Windows, inspect architecture through the import summary or an already
installed binary inspection tool. Keep host and target architectures separate:
a Windows x86 game can be inspected on a supported macOS host. Check CrySirror's
detection code, native API tables and source findings before declaring a new
engine.

Keep one ignored local analysis workspace for databases and small exports.
Reference installed read-only game files. If a tool needs a copied executable,
copy only that input and preserve its digest. Do not duplicate the whole game.
Check ignored status before placing derived game content in the checkout.

## Import and inspect

| Action | Ghidra | IDA | Expected result |
| --- | --- | --- | --- |
| Start a database | Create a local project and import the binary | Open the binary and privately save its analysis database | Persistent work tied to the input digest. |
| Confirm loading | Check format, language/compiler, bitness and endianness | Check loader, processor and image layout | Plausible sections/code. Raw imports need evidence for their language/base. |
| Analyze statically | Open in CodeBrowser and select suitable analysis options | Let autoanalysis complete | Functions and references without running the target. |
| Find clues | Search strings and inspect references | Inspect Strings, Imports, Functions and cross-references | A concrete caller to investigate. |
| Recover behavior | Compare Listing, Decompiler and references | Compare assembly, available local pseudocode and references | A traceable operation with checked branches. |

Use installed help for menu labels and shortcuts, which vary by release/keymap.
Official references include
[IDA subviews](https://docs.hex-rays.com/ida-9.2/user-guide/user-interface/subviews)
and [Ghidra class materials](https://github.com/NationalSecurityAgency/ghidra/tree/master/GhidraDocs/GhidraClass).

Explain terms when first needed: a *string reference* is code using that string;
a *caller* invokes a function; a *dispatcher* chooses a handler; a *decompiler*
infers source-like code. Labels such as `FUN_...` or `sub_...` are tool-generated,
not recovered original function names.

If code is absent or imports are implausible, check for a launcher, managed
assembly, data container, incorrect mapping or unsupported profile. Record the
observation. Failed analysis does not prove encryption and does not authorize
brute-force key searches or running an unknown unpacker.

## Locate an entry point

Use independent clues rather than a filename alone. Follow launchers into the
actual runtime module where needed.

| Clue | Follow toward | Recover |
| --- | --- | --- |
| Configuration names, archive extensions, magic | Open/read/seek callers and header checks | Lookup/mount order, path/case policy, versions and bounds. |
| Script errors or opcode names | Fetch/decode loop, switch/jump table, operand readers | Opcode identity/length, stack effects, branches and faults. |
| Exports or native-library names | Registration loops, receiver/module selectors, indirect calls | Qualified syscall identity, argument/result ABI, aliases and subselectors. |
| Graphics/text imports or layout strings | Surface creation, glyph layout, scene and presentation callers | Canvas, formats, alpha/blending, order and completion ownership. |
| Audio imports or sound-bank names | Decode, channels, mixing and clock users | Sample formats, seek/EOF, waits and native voice ownership. |
| Save names or serializers | State traversal, slot headers, restore/resource rebuilding | Save versions, logical state and pending-work reconstruction. |

Imports are leads, not proof of reachability. A single handler or table does not
establish complete syscall coverage.

## Trace a complete operation

Read callers, the complete handler and called helpers. Recover operand widths,
signedness, floating-point behavior, argument ownership, outputs, state writes,
normal/error returns and cleanup. Follow waits and retries: when does the PC
advance, when are arguments consumed, and could resumption repeat side effects?

Check uncertain decompiler types, table limits and indirect calls against
assembly/data. Inspect constructors, registration and teardown where relevant.
Keep address units explicit:

| Identity | Meaning |
| --- | --- |
| File offset | Position in the on-disk binary. |
| VA / RVA | Loaded address / address relative to the recorded image base. |
| Script PC | Native bytecode position, stating byte/word units. |
| Native ID | Logical opcode, resource, object or syscall selector. |

Do not interchange these identities. Separate facts, hypotheses and unresolved
branches; record tool version and input digest. Rename functions descriptively
while retaining their addresses and evidence for the names.

## Make findings implementable

Maintain a readable inventory in the adapter's native developer reference:

| Native identity/profile | Purpose | Operands/results and ownership | Implementation/shared API | Remaining gap |
| --- | --- | --- | --- | --- |
| Opcode or qualified syscall, including subselector | Recovered behavior | Widths, guards, stack effects, completion and errors | Concrete source route and capability, or not implemented | Specific unreviewed branch or missing service. |

Quantify coverage with the actual denominator and profile. Count opcodes,
syscall families and subselectors separately. Keep original explicit no-ops with
evidence. Unknown handlers, registered error callbacks and empty implementation
bodies cannot count as implemented behavior.

Continue with the [onboarding skill](../../crysirror-engine-onboarding/SKILL.md)
and [implementation guide](../../../../docs/ENGINE_ONBOARDING.md). Preserve the
mode: `adopt` extracts reusable mechanisms into shared services; `integrated`
retains a complete runtime behind the host seam. Do not reimplement a working
integrated upstream just because binary analysis tools are available.

Implement the next reviewed slice and use the smallest authorized release check
or real-game path that can verify it. Source coverage and observed behavior are
separate. Title, scenario, selections, audio/movie completion, save/load and exit
need their own observations. Put exact commands and actual errors in focused
validation notes rather than growing status/handoff dumps. Export the needed
functions and dependencies rather than the whole binary's pseudocode.
