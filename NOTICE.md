# Notices

CrySirror, including the AVG32, RealLive and Siglus VM implementations maintained
by xmoezzz, uses MPL-2.0 except where component licenses state otherwise. Shared
image, text, UI, animation, resource and audio capabilities are maintained in the
engine workspace. The root LICENSE contains the MPL-2.0 text.

The tiny
checkerboard/marker PNGs and tone.wav are self-authored test assets covered by
the repository's MPL-2.0 license. They are not extracted game assets.

No user game resource, game-specific key, credential,
proprietary SDK or font binary is included in this snapshot. The Siglus type-3
JPEG format's public fixed scrambling table is retained as algorithm code.

Cargo dependencies are external. Dependency resolution and local builds now
succeed; their complete resolved licenses/notices have not been audited here.
The packager includes CrySirror's own notice/license and current engine source,
not a certification of complete binary-distribution license compliance. Verify
source/binary correspondence and third-party notices before a public release.

The `crysirror-formats-arc` plugins translate selected archive decoders
from vn-tools/arc_unpacker at commit
456834ecf2e5686813802c37efd829310485c57d, Copyright (C) 2016 rr-.
They retain GPL-3.0-or-later attribution and the upstream license text in that
crate. The desktop binary always links them; the complete registration
inventory is a migration ledger, not a declaration that every decoder is ported.
See crates/crysirror-formats-arc/UPSTREAM.md and docs/RESOURCES.md.
The shared physics backend depends on Rapier3D 0.22.0; source, checksum and
reproducible behavior/performance evidence are recorded in ADR 0045.

The upstream fetch tool creates reference checkouts on the developer's machine;
use --source-root outside the deliverable tree. Those checkouts are not included
in this source export. No fonts may be added to an
assistant-generated delivery merely because they happen to be installed locally.

The KiriKiri integration consists of the first-class workspace crates
`crysirror-tjs2`, `crysirror-kag` and `crysirror-krkr`, each distributed under
AGPL-3.0-or-later. License texts are supplied in the corresponding crate
LICENSE files; existing source-level notices retain their applicable scope.
The desktop binary links them unconditionally, so a distributed `crysirror`
binary is covered by the AGPL, including its section 13 network clause, and
must meet the source obligations of every linked licence (AGPL-3.0-or-later
KiriKiri, GPL-3.0-or-later Circus and formats-arc, GPL-2.0-or-later NScripter
and System4). The MPL-2.0 engine crates remain usable without the binary.
The krkr-demo images and scripts are self-authored.
