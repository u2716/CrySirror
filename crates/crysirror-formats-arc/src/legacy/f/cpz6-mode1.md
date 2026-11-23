# CPZ6 mode 1

The shared `purple-software/cpz6` format accepts `layout: "cmvs_mode1"`
and `key_id` in its decode parameters. The key provider supplies the 96-byte
table as a scoped capability. The layout uses the Amatsutsumi CPZ6 constants.
The existing parameterized CPZ5/CPZ6 route retains its defaults.

| Contract | Mode 1 behavior |
| --- | --- |
| Layout | Signature `CPZ6`, 64-byte header, encrypted directory and file indexes, then member data. Header word at offset 52 is XORed with `0xfb73a956` and must decode to 1. |
| Index | Outer transform starts at table slot 5. Final bytes use the forward nibble shift. Per-directory file-index tail adds 3. |
| Member | 64-bit data-relative offset, u32 stored size and encrypted-byte checksum at record offset 16. The rolling decrypt state starts at `0x2748c39e`. |
| Integrity | Member bytes are checked before decryption using the little-endian wrapping word and tail-byte sum from `0x5a902b7c`. Header and index authenticity remain unverified. |
| Lookup | Shared mounts expose canonical paths. The native VM must resolve script/resource names in registration order and handle any native basename ambiguity in its own adapter. |

## CPZ7 V7B

`purple-software/cpz7` accepts `layout: "cmvs_v7b"`, `key_id` and the four
`words` registered by the script. Words are ordered as native object fields
`+460`, `+464`, `+468` and `+472`. The table is the same 96-byte `.rdata` key.

| Contract | V7B behavior |
| --- | --- |
| Header | 72 bytes. The word at 64 is XORed with `0x65ef99f3` to give the mask block size. The word at 68 checks the first 68 bytes. |
| Index mask | Follows the index. MD5 of bytes 16 onward must match bytes 0..16. Bytes 24 onward are XORed with the key at 20..24, then expand through a bit tree. Index byte `i` is XORed with mask byte `(i + 3) % 0x3ff`. |
| Index | Directory and file seed steps are XORed with words 0 and 1. |
| Member | Records carry an unused word at 16, checksum at 20, key at 24 and name at 28. The key adds words 2 and 3 as in `sub_140014E60` and `sub_1400147D0` of the 2021-10-07 x64 build. |
