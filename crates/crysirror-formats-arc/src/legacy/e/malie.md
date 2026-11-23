# Malie resource formats

These plugins expose archive members through CrySirror Resources and scoped
random-access readers. They do not execute Malie scripts or register a game VM.

- `malie/lib`: classic `LIB\0`, CP932 names and embedded directories, with
  offsets relative to the containing LIB.
- `malie/libu`: Unicode names, nested LIBU directories and signed 64-bit
  relative offsets. Directory-record sizes do not bound nested indexes;
  file ranges remain bounded by the source.
- `malie/libp`: recursive indexed directories, file flags `0x10000` and
  `0x20000`, 1024-byte offset units and configurable data alignment (4096 by
  default).
- `malie/mgf`: existing shared PNG-signature restoration and image decoding.
- `malie/dzi`: shared bitmap composition of scoped `tex/` WebP, PNG or MGF
  companions. Only a typed not-found error permits trying the next extension.

LIBP/LIBU accept whole-source encryption profiles:

```ts
resources.defineProfile("malie", {
  parameters: {
    "malie/libp": {
      encryption: "camellia", // alternatively "cfi"
      key_id: "host-provided",
      data_alignment: 4096
    }
  }
});
resources.mount("data.dat", "story", { format: "malie/libp", profile: "malie" });
```

Use `malie/libu` in both places for an encrypted LIBU archive and omit
`data_alignment`. Plain signatures can be probed; encrypted archives require
explicit format and cipher selection. The host supplies keys through the
client's `KeyProvider`, never as profile JSON or game data bundled with the
engine. No executable key extraction or title-key catalogue is provided.

The shared transforms are `malie-camellia` and `malie-cfi`. Camellia accepts a
16-byte Camellia-128 key, or 52/56 expanded little-endian words in the referenced
Malie layout (56-word inputs use the first 52 words). CFI accepts 32 key bytes
followed by four little-endian rotation words, for 48 bytes total. Transform
parameter `offset` defaults to zero and denotes the physical source position.
The reader and this offset must contain/start on complete 16-byte blocks.
Archive profiles decrypt the whole source view, preserving physical positions
for arbitrary member offsets and concurrent reads; no whole-archive buffer,
shared seek cursor or native filesystem path is required.

With these whole-source profiles, `.txtz` members are zlib-decoded. `.psbz`
members with a `PSBZ` header use its declared decoded size followed by zlib;
members without that header remain raw. Listing sizes retain the original
index sizes; the opened reader reports its actual decoded length. Decoded data
uses the existing entry/resident limits. Do not also apply a caller pipeline
for the same cipher or compression layer. Existing explicit per-index and
per-entry pipelines remain available when no whole-source profile is selected.

Malformed indexes, traversal, cycles, missing keys, partial cipher blocks and
budget failures return errors. DZI texture-set and native mip/layout variants
beyond the described grids are not inferred.

The archive layouts and Malie-specific cipher operations follow the third-party
[GARbro Malie implementation](https://github.com/morkt/GARbro/tree/b09ee4570ccb1daf6ac56710ee8934dc0b8baeb0/ArcFormats/Malie),
with its notices retained beside the Rust code. These references do not establish
original-program parity. The Camellia primitive is checked against
[RFC 3713 Appendix A](https://www.rfc-editor.org/rfc/rfc3713#appendix-A).
Focused tests are in `tests/malie.rs` and the shared transform module; they use
authored bytes, not commercial game archives. No real-title execution or
hardware behavior is claimed by this resource integration.
