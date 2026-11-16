# Notices

CrySirror, including the AVG32, RealLive and Siglus VM implementations maintained
by xmoezzz, uses MPL-2.0 except where component licenses state otherwise. Shared
image, text, UI, animation, resource and audio capabilities are maintained in the
engine workspace. The root LICENSE contains the MPL-2.0 text. MPL-2.0 workspace crates reference that shared text through Cargo metadata; source-level author and copyright notices retain their scope.

The integrated Siglus runtime maintains siglus_rs source under MPL-2.0 at
revision `bef39ea90c745a4888a92a09d18be1eecfdd10ca`. Original source paths and
digests are recorded in [its provenance file](crates/crysirror-siglus-integrated/SOURCE.json);
local host changes remain in the same maintained workspace crate.

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
See docs/RESOURCES.md.
The shared physics backend depends on Rapier3D 0.22.0; source, checksum and
reproducible behavior/performance evidence are recorded in ADR 0045.

The upstream fetch tool creates reference checkouts on the developer's machine;
use --source-root outside the deliverable tree. Those checkouts are not included
in this source export. No fonts may be added to an
assistant-generated delivery merely because they happen to be installed locally.

The KiriKiri integration consists of the first-class workspace crates
`crysirror-tjs2`, `crysirror-kag` and `crysirror-krkr`, distributed under
MPL-2.0 like the engine crates. License texts are supplied in the corresponding
crate LICENSE files; third-party notices beside their code retain their scope.
The desktop binary links them unconditionally; a distributed `crysirror`
binary must meet the source obligations of every linked licence
(GPL-3.0-or-later Circus and formats-arc, GPL-2.0-or-later NScripter and
System4). The MPL-2.0 engine crates remain usable without the binary.
The krkr-demo images and scripts are self-authored.
