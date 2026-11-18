# Software heritage

Visual novels and other legacy games are stored in engine-specific archives,
images, audio and protection schemes. When the original tools and runtimes
are no longer maintained, these formats become the main obstacle to keeping a
game playable. CrySirror reads them directly, so a game can run from the
player's own installed copy on current desktops, mobile devices and consoles.

CrySirror reads every format in place. It does not extract archives or
convert games, and it does not ship game data, keys, fonts or proprietary
SDK files. The player supplies the game, and CrySirror reads it on demand
from local disks, NFS exports or SMB shares.

## Archive and resource formats

The `crysirror-formats-arc` crate contains the archive library. It registers
**289 decoders in 92 engine and publisher families**:

| Kind | Decoders |
| --- | --- |
| Archives | 152 |
| Images | 96 |
| Audio | 16 |
| Encoded files | 25 |

258 decoders are complete. The other 31 are partial and support only some
versions or variants. When a variant is unsupported, the decoder reports an
error instead of returning incomplete data. Decoded archives mount as virtual
directories through the shared `Resources` API. Engine adapters, the
JS/TS API and the MCP tools all use those mounts.

Covered families include Abogado, ActiveSoft, AliceSoft, Amuse Craft, BGI,
Bishop, CatSystem, CRI, CyberWorks, DxLib, Entis, Escude, GLib, Innocent Grey,
Kaguya, KID, KiriKiri, Leaf, Liar-soft, Lilim, Lucifen, Mages, Majiro, Malie,
Minato Soft, NekoPack, Nitroplus, NScripter, N-System, Purple Software, QLIE,
RealLive, Ren'Py, RPG Maker, Shiina Rio, Silky's, Tactics, Triangle, Unity,
Will, YU-RIS and Yuzusoft.

The per-variant status is in [arc-resource-support.json](arc-resource-support.json)
and [arc-image-support.json](arc-image-support.json). The API is described in
[Resource formats](RESOURCES.md).

## KiriKiri automatic decryption

KiriKiri games often protect their XP3 archives with a filter that is
compiled into the game executable or a `.tpm` plug-in. CrySirror's KiriKiri
decryption component, **xp3-brute**, handles these games automatically. The
player does not need to provide a key or choose a profile.

1. `crysirror play` opens `data.xp3` with each supported protection profile.
2. The component reads the game's EXE and TPM files as data and recognizes the
   CXDEC-family and HXV4 filter parameters in their code and data. Game code
   is never run.
3. If more than one parameter set is found, each is tested against the
   archive's own Adler-32 checksums. Only an exact single match is accepted.
   Otherwise, the archive fails to open and the error states what was found.
4. Each entry is decrypted when it is read. The archives on disk stay
   encrypted, and nothing is unpacked.

The same component decodes TJS `ns0`/`4s0` data packs (PBD). There are no
per-title key tables or key-space searches, and no emulator is used. The
design is in [ADR 0096](adr/0096-cxdec-xp3-content-filter-and-special-index.md)
and [ADR 0091](adr/0091-authenticated-hxv4-resources-and-bootstrap.md).
[ADR 0097](adr/0097-krkr-protection-interpreted-and-lifted-filters.md) plans a
no-JIT interpreter and lifter for other filter callbacks. That work is not
implemented yet.

## Engines

Formats alone do not make a game playable. CrySirror also runs the original
scenario bytecode and save files for KiriKiri, Siglus, RealLive, AVG32, BGI,
SoftPAL, RFVP, NScripter, Circus, CatSystem2, QLIE, Majiro, AliceSoft System 4,
System-NNN, SugarPot, N-System and WillPlus. See
[Supported game engines](NATIVE_ADAPTERS.md) and
[Native opcode and syscall lists](native-api/README.md).

Licence terms for the distributed binary are listed in [NOTICE.md](../NOTICE.md).
