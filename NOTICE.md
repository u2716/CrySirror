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

## Malie archive and cipher adaptations

The Malie archive parsers and CFI operations adapt GARbro code by morkt.
Copyright (C) 2015-2017 by morkt.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to
deal in the Software without restriction, including without limitation the
rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
IN THE SOFTWARE.

The shared Malie Camellia implementation adapts the NTT implementation
distributed with GARbro. Its notice follows:

Copyright (c) 2006
 NTT (Nippon Telegraph and Telephone Corporation) . All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:
1. Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer as
   the first lines of this file unmodified.
2. Redistributions in binary form must reproduce the above copyright
   notice, this list of conditions and the following disclaimer in the
   documentation and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY NTT ``AS IS'' AND ANY EXPRESS OR
IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
IN NO EVENT SHALL NTT BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## VP8 decoder (vp8-decoder 0.1.2)

WebM movies are decoded by the crates.io crate `vp8-decoder` =0.1.2
(MIT OR Apache-2.0, by Alakazam Labs), a port of the RFC 6386 reference decoder.
Its RFC 6386 Section 20 notice, reproduced from the crate's NOTICE file:

Copyright (c) 2010, 2011, Google Inc.  All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

o  Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer.

o  Redistributions in binary form must reproduce the above copyright
   notice, this list of conditions and the following disclaimer in
   the documentation and/or other materials provided with the
   distribution.

o  Neither the name of Google nor the names of its contributors may
   be used to endorse or promote products derived from this software
   without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE COPYRIGHT
HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY
WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.
