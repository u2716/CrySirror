# CMVS

Onboarding mode is `adopt` under [ADR 0102](../../docs/adr/0102-engine-adoption-and-runtime-integration.md).
The crate is distributed under [MPL-2.0](../../LICENSE).

`Script::read` loads bounded PS2A and PSS0 scripts through a scoped `AssetReader`, including
compressed and uncompressed payloads. The script exposes code, labels, initial
mutable data and raw text. The VM profile owns the native text and value rules.

`vm::CmvsVm::prepare` takes an explicit `VmProfile`. The first implemented
profile is `Pc20160201X64`, based on the CMVS Windows build dated 2016-02-01.
The machine has bounded execution, value banks, expressions, control flow,
common-script calls, frame callbacks and explicit pending effects.
`Cpz6Resources` opens archives through the shared format registry.
Slot-header scans and CSS1 system data prepare through the client save
capability. Save thumbnails use shared frame readback and the native BMP
layout. The owner commits completed work at the requesting instruction.
See the [native ABI](NATIVE_ABI.md) and
[dispatch inventory](../../docs/native-api/cmvs.md).

The shared launcher registers the `cmvs` player adapter. The desktop
`play --headless` path runs the original logo and interactive title through
the scoped CPZ6 source. Input replay has activated New Game, played the
first story movie, and drawn the first line of dialogue in chapter one
through shared glyph layers. The shared Media path also plays the
idle-title OP. In-memory scene checkpoints cover supported main-script
transfers and explicit chapter capture. Further route coverage, original
gameplay save files and complete scene restoration remain open.
The [Amatsutsumi sample record](../../docs/validation/cmvs-amatsutsumi.md)
contains the first installation's behavior evidence for this profile.

`Pc20211007X64` identifies the CMVS Windows build dated 2021-10-07. Its CPZ7
archives open through the shared format registry, using the four words each
`0x2268..=0x226e` registration passes from `start.ps3`. KUNADO reaches its
title, load screen and story opening under this profile. Unconnected selectors
stop with a visible error, as tracked in
[SATD-cmvs-20211007-profile](SATD.md#satd-cmvs-20211007-profile).

## CVNS

`Cvns20040515X86` identifies the predecessor CVNS engine in the supplied i386
build. It has its own PSS0/CPZ1 admission, frame timing and syscall table.
The muted headless player runs voiced dialogue and a story choice back to the
title, opens the Settings pages and exits through the title menu. Voices use
the shared MV2X decoder.
The profile reads and writes CSS0 system data (`save000.dat`), scans numbered
CSP slot headers for the original save/load menus and plays CMV3 movies with
their indexed Vorbis soundtrack. Numbered CSP1 slots store the checkpoint,
optional thumbnail and embedded main script. A fresh player can rebuild the
saved scene and resume execution at the recorded instruction.
[CVNS validation](../../docs/validation/cvns-20040515.md) records the scope
of the current profile. [CVNS SATD](SATD.md#satd-cvns-20040515-profile)
tracks the remaining ABI and resource variants.
