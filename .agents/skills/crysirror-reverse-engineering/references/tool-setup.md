# Setting up a local reverse-engineering tool

Use this reference when a contributor needs a disassembler or installation help.
Reuse a suitable existing tool. Check host architecture, free space and the
selected release's dependencies before downloading. Install one tool first and
keep its projects outside the game directory.

## Choose the route

| Situation | Route | Verify |
| --- | --- | --- |
| Matching maintained source exists | Inspect that pinned source first | Same native profile, complete helpers and conditional branches. |
| Suitable IDA is installed | Reuse its installation and database | Target support, local decompiler entitlement and scripting availability. |
| The user requests IDA | Official installer for the selected OS/edition | Licence/account steps and whether the edition meets the task. |
| No preference or existing tool | Official Ghidra release | Host requirements, target loader/language and matching JDK. |
| Managed assembly or data-only script | Matching managed/source workflow | A native-code decompiler may not decode the relevant program. |

Ghidra supports local disassembly/decompilation. IDA Free's current product page
lists x86/x64 disassembly and cloud decompilation; cloud processing is unsuitable
for this workflow's private game material. An installed IDA GUI does not prove
IDAPython, plugins or batch automation are available. Verify the actual edition
and use local Ghidra decompilation or available local IDA capabilities as needed.

Official references:

- [Ghidra releases](https://github.com/NationalSecurityAgency/ghidra/releases)
- [Ghidra requirements](https://github.com/NationalSecurityAgency/ghidra#install)
- [Ghidra Getting Started](https://github.com/NationalSecurityAgency/ghidra/blob/master/GhidraDocs/GettingStarted.md)
- [IDA installation](https://docs.hex-rays.com/getting-started/install-ida)
- [IDA Free capabilities](https://hex-rays.com/ida-free)

Recheck these sources during setup. The downloaded release's bundled instructions
govern its dependencies; development-branch requirements may differ.

## Ghidra

1. Download the packaged asset from the official release page, rather than a
   source-code archive. Record its release identifier and installation path.
2. Read that release's requirements and install the matching 64-bit JDK from an
   official distribution. A JRE alone is insufficient. Verify which Java the
   launcher uses before changing environment variables; scope overrides to the
   tool if other projects need a different JDK.
3. Extract into a new tool directory rather than overlaying an old installation.
   Start `ghidraRun` on macOS/Linux or `ghidraRun.bat` on Windows. Give the user
   the exact path and quote paths containing spaces.
4. If launch fails, inspect the actual launcher error and bundled troubleshooting
   instructions. Resolve the indicated dependency or Java path. Do not disable
   system security globally or install unrelated SDKs.

Create a local non-shared project for private game analysis. A source build or
Ghidra Server is unnecessary for the first ordinary import.

## IDA

1. Follow the official installer instructions for the user's OS and edition.
   Have the user complete required account, licence and purchase interactions
   directly. Never ask for passwords or licence contents in chat.
2. Select the matching host installer. Follow its wizard and documented licence
   activation/location. Do not copy another user's install or bundle licences
   with the repository. A file's presence does not establish entitlement.
3. Start IDA and record its actual version/edition. Check processor support and
   local decompiler availability before importing private game code. Save the
   database in a private working directory.
4. Set up Python or plugins only when supported and needed, using documentation
   matching the installed release. Verify scripting with a small local query;
   do not assume a paid-edition API is available in Free.

Keep private game material out of cloud decompilation, public submissions and
remote database sharing. Use an existing trusted local connector within its
actual capabilities and authorization. Installing a new connector is separate
setup; a first analysis need not depend on an MCP plugin.

## Verify readiness without starting the game

| Check | Evidence |
| --- | --- |
| Tool starts | Actual version and a successful local launch or supported headless invocation. |
| Target loads | Correct format, processor, bitness and endianness in the import summary. |
| Analysis works | A real code function, its assembly and a reference can be inspected. Record decompiler failure separately. |
| Work persists | The project/database reopens with an annotation intact. |
| Automation works, when needed | A supported local API lists one real function/address from this database. |

For GUI-only setup, give one small action at a time and describe the expected
result. Use a local export or textual error when sufficient; screen access is
not a prerequisite for the workflow.

Continue with [first analysis](first-analysis.md).
