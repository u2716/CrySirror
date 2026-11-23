# Image decoder technical debt

## SATD-image-exhibit-gyu

The [GYU decoder](src/exhibit_gyu.rs) handles stored color, modes 0x0200 and 0x0400
ring-LZSS color, mode 0x0800 interleaved color, inline shuffle seeds, and stored or compressed separate alpha
for indexed and BGR24 images. External seed tables, other
compression tags and BGRX with separate alpha remain unresolved.

To close, identify the matching original loader, verify each remaining layout
against locally authorized files, add pixel-level regressions, and exercise
the supported profiles through runtime asset loading. The separate ExHibit
script VM and game startup are outside this decoder's contract.

| Date | Change | Remaining scope |
| --- | --- | --- |
| 2026-11-22 | Added registry decoding for stored GYU images, opaque BGRX, the indexed palette and uncompressed alpha on indexed/BGR24 images. | Other profiles listed above and original-install verification. |
| 2026-11-23 | Connected inline shuffle and exact-size LZSS decoding to the shared registry. Confirmed header layouts in three Famisupi startup assets. | Retail player replay and pixel comparison pending, alongside the unresolved profiles above. |
| 2026-11-23 | Added mode 0x0200 and the independent 0x0800 decoder, with a synthetic literal and overlapping-reference regression. Scanned 7,328 installation headers. | New tests and pixel comparisons pending. |
