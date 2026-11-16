# Circus developer reference

[All engines](README.md)

Mode: `adopt`.

## ABI and host contract

| Profile | Execution boundary |
| --- | --- |
| DC1 `#ADV_TXT` | Dedicated record/label/read-history interpreter; not MES bytecode. |
| DC2PC `0x5769` | Dedicated offset1 MES dispatcher. |
| DC3 `0x8A71` | Opcode remapping plus original-profile helpers; remaps are listed separately. |
| DC3WY `0xA09F` | Dedicated offset2 MES dispatcher and shared DC3 handlers. |
| DC4 `0xAAB6` | Parser recognition only; no execution dispatcher. |
| DC4PH `0xABB6` | MES dispatcher, 139 native operations, 42 proven original empty slots and continuation `0x77`. |

PCs are byte offsets. MES operands retain signed words, shifted CP932 strings,
version-specific prefixes and continuation records. A subselector belongs to its
parent opcode and profile; equal numbers in other profiles are different APIs.
`Machine::step` returns `Outcome`; service retry retains the instruction and
native pending state. Branch/scene changes commit through their native paths.

| Native area | CrySirror service / implementation | Completion contract |
| --- | --- | --- |
| Images, sprites and effects | Image jobs, retained Layer/Canvas and VFX through `NativeHost` | Preparation/publication can retry; native state commits after acknowledgement. |
| Text, menus and controls | Glyph/text and retained UI events | Reveal, choice and menu continuation remain native. |
| Sound and movies | Shared Audio/Media | Native channels, group controls and waits retain their own IDs. |
| Slots and autosave | Scoped Storage plus native save projection | Capture and live-state projection are distinct; missing layouts fail. |

Limits include parse-only DC4, profile-specific unsupported selectors, and save,
movie or effect domains rejected in the tables. `Original no-op` is sourced from
`src/opcodes.rs`; an empty Rust branch alone is insufficient evidence.

Source: [crate and provenance](../../crates/crysirror-circus/),
[step/Outcome](../../crates/crysirror-circus/src/machine.rs).
Behavior receipts: [DC1](../validation/circus-dc1-2026-11-12.md),
[DC4PH](../validation/circus-dc4ph-2026-11-09.md). These exercise named paths only.

<!-- BEGIN GENERATED DISPATCH TABLES -->

## Opcode and API implementation tables

`Handled` means an executable source branch exists. It is not a claim that every argument/subselector or a complete game has been audited. Explicit gaps are shown per row.

| Status | Entries |
| --- | ---: |
| Continuation | 1 |
| Empty handler | 23 |
| Handled | 661 |
| Original no-op | 42 |
| Partial | 11 |
| Unsupported | 15 |

DC4 0xAAB6 is recognized by the parser but has no execution dispatcher. DC3 0x8A71 remaps are listed separately from shared DC3WY handlers.

These tables cover the checked-in catalogues and explicit dispatch branches. An engine-wide upstream denominator is not inferred from these rows.

## DC1 ADV_TXT instructions

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Substitute | Handled | — | [dc1_story.rs:1459](../../crates/crysirror-circus/src/dc1_story.rs#L1459) |
| `0x02` | Update effects enabled | Handled | — | [dc1_story.rs:1477](../../crates/crysirror-circus/src/dc1_story.rs#L1477) |
| `0x03` | Update effects enabled | Handled | — | [dc1_story.rs:1478](../../crates/crysirror-circus/src/dc1_story.rs#L1478) |
| `0x04` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x05` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x06` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x07` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x08` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x09` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x0A` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x0B` | Execute execute branch | Handled | — | [dc1_story.rs:1482](../../crates/crysirror-circus/src/dc1_story.rs#L1482) |
| `0x0C` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x0D` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x0E` | Execute execute branch | Handled | — | [dc1_story.rs:1485](../../crates/crysirror-circus/src/dc1_story.rs#L1485) |
| `0x0F` | Execute execute branch | Unsupported | DC1 404600 opcode 15/16 (4030D0 forced redraw) is not ported | [dc1_story.rs:1486](../../crates/crysirror-circus/src/dc1_story.rs#L1486) |
| `0x10` | Execute execute branch | Unsupported | DC1 404600 opcode 15/16 (4030D0 forced redraw) is not ported | [dc1_story.rs:1486](../../crates/crysirror-circus/src/dc1_story.rs#L1486) |
| `0x11` | Update auto text | Handled | — | [dc1_story.rs:1487](../../crates/crysirror-circus/src/dc1_story.rs#L1487) |
| `0x12` | Update auto text | Handled | — | [dc1_story.rs:1488](../../crates/crysirror-circus/src/dc1_story.rs#L1488) |
| `0x13` | Execute execute branch | Unsupported | DC1 416AE0 particle owner is not ported | [dc1_story.rs:1489](../../crates/crysirror-circus/src/dc1_story.rs#L1489) |
| `0x14` | Execute execute branch | Unsupported | DC1 416AE0 particle owner is not ported | [dc1_story.rs:1489](../../crates/crysirror-circus/src/dc1_story.rs#L1489) |
| `0x15` | Execute execute branch | Unsupported | DC1 416AE0 particle owner is not ported | [dc1_story.rs:1489](../../crates/crysirror-circus/src/dc1_story.rs#L1489) |
| `0x16` | Execute execute branch | Handled | — | [dc1_story.rs:1490](../../crates/crysirror-circus/src/dc1_story.rs#L1490) |
| `0x17` | Jump | Handled | — | [dc1_story.rs:1493](../../crates/crysirror-circus/src/dc1_story.rs#L1493) |
| `0x18` | Load | Handled | — | [dc1_story.rs:1497](../../crates/crysirror-circus/src/dc1_story.rs#L1497) |
| `0x19` | Load | Handled | — | [dc1_story.rs:1497](../../crates/crysirror-circus/src/dc1_story.rs#L1497) |
| `0x1A` | Update name, name shown, name drawn | Handled | — | [dc1_story.rs:1517](../../crates/crysirror-circus/src/dc1_story.rs#L1517) |
| `0x1B` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x1C` | Update transition | Handled | — | [dc1_story.rs:1530](../../crates/crysirror-circus/src/dc1_story.rs#L1530) |
| `0x1D` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x1E` | Execute execute branch | Handled | — | [dc1_story.rs:1531](../../crates/crysirror-circus/src/dc1_story.rs#L1531) |
| `0x1F` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x20` | Update grade | Handled | — | [dc1_story.rs:1535](../../crates/crysirror-circus/src/dc1_story.rs#L1535) |
| `0x21` | Execute execute branch | Handled | — | [dc1_story.rs:1542](../../crates/crysirror-circus/src/dc1_story.rs#L1542) |
| `0x22` | Execute execute branch | Handled | — | [dc1_story.rs:1545](../../crates/crysirror-circus/src/dc1_story.rs#L1545) |
| `0x23` | Execute execute branch | Handled | — | [dc1_story.rs:1545](../../crates/crysirror-circus/src/dc1_story.rs#L1545) |
| `0x24` | Execute execute branch | Handled | — | [dc1_story.rs:1545](../../crates/crysirror-circus/src/dc1_story.rs#L1545) |
| `0x25` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x26` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x27` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x28` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x29` | Execute execute branch | Handled | — | [dc1_story.rs:1545](../../crates/crysirror-circus/src/dc1_story.rs#L1545) |
| `0x2A` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x2B` | Execute execute branch | Handled | — | [dc1_story.rs:1550](../../crates/crysirror-circus/src/dc1_story.rs#L1550) |
| `0x2C` | Update text color | Handled | — | [dc1_story.rs:1553](../../crates/crysirror-circus/src/dc1_story.rs#L1553) |
| `0x2D` | Update text speed | Handled | — | [dc1_story.rs:1558](../../crates/crysirror-circus/src/dc1_story.rs#L1558) |
| `0x2E` | Execute execute branch | Handled | — | [dc1_story.rs:1531](../../crates/crysirror-circus/src/dc1_story.rs#L1531) |
| `0x30` | Update yield sprite | Handled | — | [dc1_story.rs:1565](../../crates/crysirror-circus/src/dc1_story.rs#L1565) |
| `0x31` | Execute execute branch | Handled | — | [dc1_story.rs:1589](../../crates/crysirror-circus/src/dc1_story.rs#L1589) |
| `0x32` | Execute execute branch | Handled | — | [dc1_story.rs:1597](../../crates/crysirror-circus/src/dc1_story.rs#L1597) |
| `0x33` | Execute execute branch | Handled | — | [dc1_story.rs:1603](../../crates/crysirror-circus/src/dc1_story.rs#L1603) |
| `0x34` | Update effect pending, yield sprite | Handled | — | [dc1_story.rs:1581](../../crates/crysirror-circus/src/dc1_story.rs#L1581) |
| `0x35` | Execute execute branch | Handled | — | [dc1_story.rs:1612](../../crates/crysirror-circus/src/dc1_story.rs#L1612) |
| `0x36` | Update visual / effect | Handled | — | [dc1_story.rs:1618](../../crates/crysirror-circus/src/dc1_story.rs#L1618) |
| `0x37` | Execute execute branch | Unsupported | DC1 404600 sprite effect/pose opcode {} is not ported | [dc1_story.rs:1623](../../crates/crysirror-circus/src/dc1_story.rs#L1623) |
| `0x38` | Execute execute branch | Unsupported | DC1 404600 sprite effect/pose opcode {} is not ported | [dc1_story.rs:1623](../../crates/crysirror-circus/src/dc1_story.rs#L1623) |
| `0x39` | Update visual / background | Handled | — | [dc1_story.rs:1624](../../crates/crysirror-circus/src/dc1_story.rs#L1624) |
| `0x3A` | Update visual / effect | Handled | — | [dc1_story.rs:1628](../../crates/crysirror-circus/src/dc1_story.rs#L1628) |
| `0x3B` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x3C` | Music | Handled | — | [dc1_story.rs:1640](../../crates/crysirror-circus/src/dc1_story.rs#L1640) |
| `0x3D` | Music Fade | Handled | — | [dc1_story.rs:1651](../../crates/crysirror-circus/src/dc1_story.rs#L1651) |
| `0x3E` | Execute execute branch | Unsupported | DC1 404600 opcode 62 (42A4E0) is not ported | [dc1_story.rs:1655](../../crates/crysirror-circus/src/dc1_story.rs#L1655) |
| `0x3F` | Sound | Handled | — | [dc1_story.rs:1656](../../crates/crysirror-circus/src/dc1_story.rs#L1656) |
| `0x40` | Sound Stop | Handled | — | [dc1_story.rs:1661](../../crates/crysirror-circus/src/dc1_story.rs#L1661) |
| `0x41` | Voice | Handled | — | [dc1_story.rs:1662](../../crates/crysirror-circus/src/dc1_story.rs#L1662) |
| `0x42` | Voice Stop | Handled | — | [dc1_story.rs:1672](../../crates/crysirror-circus/src/dc1_story.rs#L1672) |
| `0x43` | Place reset | Handled | — | [dc1_story.rs:1673](../../crates/crysirror-circus/src/dc1_story.rs#L1673) |
| `0x44` | Execute execute branch | Unsupported | DC1 404600 opcode 68 (42A800 voice variant) is not ported | [dc1_story.rs:1686](../../crates/crysirror-circus/src/dc1_story.rs#L1686) |
| `0x45` | Update overlay name | Handled | — | [dc1_story.rs:1687](../../crates/crysirror-circus/src/dc1_story.rs#L1687) |
| `0x46` | Execute execute branch | Handled | — | [dc1_story.rs:1531](../../crates/crysirror-circus/src/dc1_story.rs#L1531) |
| `0x47` | Execute execute branch | Handled | — | [dc1_story.rs:1531](../../crates/crysirror-circus/src/dc1_story.rs#L1531) |
| `0x48` | Update overlay name | Handled | — | [dc1_story.rs:1687](../../crates/crysirror-circus/src/dc1_story.rs#L1687) |
| `0x49` | Execute execute branch | Handled | — | [dc1_story.rs:1531](../../crates/crysirror-circus/src/dc1_story.rs#L1531) |
| `0x4A` | Execute execute branch | Handled | — | [dc1_story.rs:1531](../../crates/crysirror-circus/src/dc1_story.rs#L1531) |
| `0x4C` | Update choice building, choice capture title, choice capture item | Partial | DC1 404600 opcode 77 (positioned choice) is not ported | [dc1_story.rs:1691](../../crates/crysirror-circus/src/dc1_story.rs#L1691) |
| `0x4D` | Update choice building, choice capture title, choice capture item | Partial | DC1 404600 opcode 77 (positioned choice) is not ported | [dc1_story.rs:1691](../../crates/crysirror-circus/src/dc1_story.rs#L1691) |
| `0x4E` | Update choice capture item, choice capture title | Handled | — | [dc1_story.rs:1699](../../crates/crysirror-circus/src/dc1_story.rs#L1699) |
| `0x4F` | Execute execute branch | Handled | — | [dc1_story.rs:1707](../../crates/crysirror-circus/src/dc1_story.rs#L1707) |
| `0x50` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x51` | Execute execute branch | Handled | — | [dc1_story.rs:1711](../../crates/crysirror-circus/src/dc1_story.rs#L1711) |
| `0x52` | Execute execute branch | Handled | — | [dc1_story.rs:1711](../../crates/crysirror-circus/src/dc1_story.rs#L1711) |
| `0x53` | Update choice enabled | Handled | — | [dc1_story.rs:1717](../../crates/crysirror-circus/src/dc1_story.rs#L1717) |
| `0x54` | Execute execute branch | Handled | — | [dc1_story.rs:1550](../../crates/crysirror-circus/src/dc1_story.rs#L1550) |
| `0x55` | Execute execute branch | Handled | — | [dc1_story.rs:1550](../../crates/crysirror-circus/src/dc1_story.rs#L1550) |
| `0x56` | Execute execute branch | Handled | — | [dc1_story.rs:1718](../../crates/crysirror-circus/src/dc1_story.rs#L1718) |
| `0x57` | Execute execute branch | Handled | — | [dc1_story.rs:1722](../../crates/crysirror-circus/src/dc1_story.rs#L1722) |
| `0x58` | Jump | Handled | — | [dc1_story.rs:1728](../../crates/crysirror-circus/src/dc1_story.rs#L1728) |
| `0x59` | Jump | Handled | — | [dc1_story.rs:1728](../../crates/crysirror-circus/src/dc1_story.rs#L1728) |
| `0x5A` | Execute execute branch | Handled | — | [dc1_story.rs:1718](../../crates/crysirror-circus/src/dc1_story.rs#L1718) |
| `0x5B` | Update date | Handled | — | [dc1_story.rs:1737](../../crates/crysirror-circus/src/dc1_story.rs#L1737) |
| `0x5C` | Update window, name shown, name drawn | Handled | — | [dc1_story.rs:1740](../../crates/crysirror-circus/src/dc1_story.rs#L1740) |
| `0x5D` | Update time of day | Handled | — | [dc1_story.rs:1747](../../crates/crysirror-circus/src/dc1_story.rs#L1747) |
| `0x5E` | Execute execute branch | Handled | — | [dc1_story.rs:1545](../../crates/crysirror-circus/src/dc1_story.rs#L1545) |
| `0x5F` | Execute execute branch | Handled | — | [dc1_story.rs:1531](../../crates/crysirror-circus/src/dc1_story.rs#L1531) |
| `0x60` | Execute execute branch | Handled | — | [dc1_story.rs:1531](../../crates/crysirror-circus/src/dc1_story.rs#L1531) |
| `0x61` | Update flags / text pacing | Handled | — | [dc1_story.rs:1752](../../crates/crysirror-circus/src/dc1_story.rs#L1752) |
| `0x62` | Execute execute branch | Unsupported | DC1 406DA0(50/51) flash transition is not ported | [dc1_story.rs:1757](../../crates/crysirror-circus/src/dc1_story.rs#L1757) |
| `0x63` | Execute execute branch | Unsupported | DC1 406DA0(50/51) flash transition is not ported | [dc1_story.rs:1757](../../crates/crysirror-circus/src/dc1_story.rs#L1757) |
| `0x64` | Execute execute branch | Handled | — | [dc1_story.rs:1550](../../crates/crysirror-circus/src/dc1_story.rs#L1550) |
| `0x65` | Execute execute branch | Handled | — | [dc1_story.rs:1758](../../crates/crysirror-circus/src/dc1_story.rs#L1758) |
| `0x68` | Execute execute branch | Handled | — | [dc1_story.rs:1763](../../crates/crysirror-circus/src/dc1_story.rs#L1763) |
| `0x69` | Execute execute branch | Handled | — | [dc1_story.rs:1763](../../crates/crysirror-circus/src/dc1_story.rs#L1763) |
| `0x6A` | Jump | Handled | — | [dc1_story.rs:1770](../../crates/crysirror-circus/src/dc1_story.rs#L1770) |
| `0x6B` | Execute execute branch | Handled | — | [dc1_story.rs:1780](../../crates/crysirror-circus/src/dc1_story.rs#L1780) |
| `0x6C` | Execute execute branch | Handled | — | [dc1_story.rs:1780](../../crates/crysirror-circus/src/dc1_story.rs#L1780) |
| `0x6D` | Jump | Handled | — | [dc1_story.rs:1792](../../crates/crysirror-circus/src/dc1_story.rs#L1792) |
| `0x6E` | Jump | Handled | — | [dc1_story.rs:1792](../../crates/crysirror-circus/src/dc1_story.rs#L1792) |
| `0x6F` | Update flags / phase | Handled | — | [dc1_story.rs:1810](../../crates/crysirror-circus/src/dc1_story.rs#L1810) |
| `0x70` | Execute execute branch | Unsupported | DC1 404600 opcode 112 (configuration toggles) is not ported | [dc1_story.rs:1811](../../crates/crysirror-circus/src/dc1_story.rs#L1811) |
| `0x71` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x72` | Flags read marks | Handled | — | [dc1_story.rs:1812](../../crates/crysirror-circus/src/dc1_story.rs#L1812) |
| `0x73` | Flags read marks | Handled | — | [dc1_story.rs:1812](../../crates/crysirror-circus/src/dc1_story.rs#L1812) |
| `0x74` | Music Attenuation | Handled | — | [dc1_story.rs:1816](../../crates/crysirror-circus/src/dc1_story.rs#L1816) |
| `0x75` | Voice Attenuation | Handled | — | [dc1_story.rs:1817](../../crates/crysirror-circus/src/dc1_story.rs#L1817) |
| `0x76` | Sound Attenuation | Handled | — | [dc1_story.rs:1818](../../crates/crysirror-circus/src/dc1_story.rs#L1818) |
| `0x77` | Execute execute branch | Unsupported | DC1 42ACE0 pan opcode {} is not ported | [dc1_story.rs:1819](../../crates/crysirror-circus/src/dc1_story.rs#L1819) |
| `0x78` | Execute execute branch | Unsupported | DC1 42ACE0 pan opcode {} is not ported | [dc1_story.rs:1819](../../crates/crysirror-circus/src/dc1_story.rs#L1819) |
| `0x79` | Execute execute branch | Unsupported | DC1 42ACE0 pan opcode {} is not ported | [dc1_story.rs:1819](../../crates/crysirror-circus/src/dc1_story.rs#L1819) |
| `0x7A` | Execute execute branch | Handled | — | [dc1_story.rs:1820](../../crates/crysirror-circus/src/dc1_story.rs#L1820) |
| `0x7B` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x7C` | Execute execute branch | Handled | — | [dc1_story.rs:1479](../../crates/crysirror-circus/src/dc1_story.rs#L1479) |
| `0x7D` | Update overlay name, overlay mode, overlay offset | Handled | — | [dc1_story.rs:1834](../../crates/crysirror-circus/src/dc1_story.rs#L1834) |
| `0x7E` | Update overlay name, overlay mode, overlay offset | Handled | — | [dc1_story.rs:1834](../../crates/crysirror-circus/src/dc1_story.rs#L1834) |
| `0x7F` | Update reset scroll on wipe | Handled | — | [dc1_story.rs:1842](../../crates/crysirror-circus/src/dc1_story.rs#L1842) |

## DC2PC 0x5769 opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x03` | Update state / history count, state / current label | Handled | — | [dc2.rs:534](../../crates/crysirror-circus/src/dc2.rs#L534) |
| `0x04` | Update state / current label | Handled | — | [dc2.rs:553](../../crates/crysirror-circus/src/dc2.rs#L553) |
| `0x07` | Text and message windows (execute_text) | Partial | DC2 automatic text mode {mode} (422E60 process7) is not connected | [dc2_text.rs:248](../../crates/crysirror-circus/src/dc2_text.rs#L248) |
| `0x08` | Menus (execute_menu) | Handled | — | [dc2_menu.rs:40](../../crates/crysirror-circus/src/dc2_menu.rs#L40) |
| `0x09` | Menus (execute_menu) | Handled | — | [dc2_menu.rs:42](../../crates/crysirror-circus/src/dc2_menu.rs#L42) |
| `0x0A` | Menus (execute_menu) | Empty handler | Original no-op status not established | [dc2_menu.rs:43](../../crates/crysirror-circus/src/dc2_menu.rs#L43) |
| `0x0F` | Execute execute branch | Handled | — | [dc2.rs:554](../../crates/crysirror-circus/src/dc2.rs#L554) |
| `0x11` | case17: =-1, then426EE0 starts the timer fade | Handled | — | [dc2.rs:476](../../crates/crysirror-circus/src/dc2.rs#L476) |
| `0x14` | Execute execute ui branch | Handled | — | [dc2_ui.rs:67](../../crates/crysirror-circus/src/dc2_ui.rs#L67) |
| `0x16` | Execute execute branch | Handled | Guard: a[0] == 5 | [dc2.rs:480](../../crates/crysirror-circus/src/dc2.rs#L480) |
| `0x17` | Update state / builder / count | Handled | — | [dc2_ui.rs:77](../../crates/crysirror-circus/src/dc2_ui.rs#L77) |
| `0x18` | Execute execute ui branch | Handled | — | [dc2_ui.rs:112](../../crates/crysirror-circus/src/dc2_ui.rs#L112) |
| `0x19` | Update state / image flag | Handled | — | [dc2_ui.rs:125](../../crates/crysirror-circus/src/dc2_ui.rs#L125) |
| `0x1E` | Execute execute branch | Handled | Guard: a[0] == 0 | [dc2.rs:555](../../crates/crysirror-circus/src/dc2.rs#L555) |
| `0x22` | Execute execute branch | Handled | — | [dc2.rs:557](../../crates/crysirror-circus/src/dc2.rs#L557) |
| `0x25` | Update state / pending check | Handled | — | [dc2_ui.rs:126](../../crates/crysirror-circus/src/dc2_ui.rs#L126) |
| `0x26` | case38: 255 pauses/resumes timer5 (); 7 is fog | Handled | Guard: a[0] == 255 | [dc2.rs:449](../../crates/crysirror-circus/src/dc2.rs#L449) |
| `0x2A` | Load Image | Handled | — | [dc2_resources.rs:68](../../crates/crysirror-circus/src/dc2_resources.rs#L68) |
| `0x30` | Clear Prepared | Handled | — | [dc2_resources.rs:110](../../crates/crysirror-circus/src/dc2_resources.rs#L110) |
| `0x31` | Load Image | Handled | — | [dc2_resources.rs:68](../../crates/crysirror-circus/src/dc2_resources.rs#L68) |
| `0x32` | Load Image | Handled | — | [dc2_resources.rs:68](../../crates/crysirror-circus/src/dc2_resources.rs#L68) |
| `0x33` | Load Image | Handled | — | [dc2_resources.rs:68](../../crates/crysirror-circus/src/dc2_resources.rs#L68) |
| `0x37` | Execute execute branch | Handled | — | [dc2.rs:568](../../crates/crysirror-circus/src/dc2.rs#L568) |
| `0x3A` | Execute execute branch | Handled | — | [dc2.rs:579](../../crates/crysirror-circus/src/dc2.rs#L579) |
| `0x3D` | Execute execute ui branch | Handled | — | [dc2_ui.rs:56](../../crates/crysirror-circus/src/dc2_ui.rs#L56) |
| `0x3E` | Execute execute ui branch | Handled | — | [dc2_ui.rs:56](../../crates/crysirror-circus/src/dc2_ui.rs#L56) |
| `0x44` | Menus (execute_menu) | Handled | — | [dc2_menu.rs:41](../../crates/crysirror-circus/src/dc2_menu.rs#L41) |
| `0x47` | Execute execute branch | Handled | — | [dc2.rs:568](../../crates/crysirror-circus/src/dc2.rs#L568) |
| `0x49` | Execute execute branch | Handled | — | [dc2.rs:587](../../crates/crysirror-circus/src/dc2.rs#L587) |
| `0x4B` | Text and message windows (execute_text) | Partial | DC2 41B710 variable expansion in names is not connected | [dc2_text.rs:242](../../crates/crysirror-circus/src/dc2_text.rs#L242) |
| `0x4C` | Text and message windows (execute_text) | Handled | — | [dc2_text.rs:247](../../crates/crysirror-circus/src/dc2_text.rs#L247) |
| `0x4F` | Update state / builder / current, state / builder / count | Handled | — | [dc2_ui.rs:127](../../crates/crysirror-circus/src/dc2_ui.rs#L127) |
| `0x50` | Update state / image flag | Handled | — | [dc2_ui.rs:174](../../crates/crysirror-circus/src/dc2_ui.rs#L174) |
| `0x51` | Execute execute ui branch | Handled | — | [dc2_ui.rs:222](../../crates/crysirror-circus/src/dc2_ui.rs#L222) |
| `0x5C` | Assign | Handled | — | [dc2.rs:595](../../crates/crysirror-circus/src/dc2.rs#L595) |
| `0x5D` | Assign | Handled | — | [dc2.rs:614](../../crates/crysirror-circus/src/dc2.rs#L614) |
| `0x5E` | Assign | Handled | — | [dc2.rs:614](../../crates/crysirror-circus/src/dc2.rs#L614) |
| `0x5F` | Execute execute branch | Handled | — | [dc2.rs:630](../../crates/crysirror-circus/src/dc2.rs#L630) |
| `0x64` | Execute execute branch | Partial | DC2 0x64 native1/2 bitmap swap with loaded portraits is not connected | [dc2.rs:667](../../crates/crysirror-circus/src/dc2.rs#L667) |
| `0x65` | Case101: [13*v5] aliases pose (v5-16)'s percent/centre for v5 16..19; 255 sets pose0/pose1 percent (-1 keeps) | Handled | — | [dc2.rs:687](../../crates/crysirror-circus/src/dc2.rs#L687) |
| `0x66` | Palette | Handled | — | [dc2.rs:705](../../crates/crysirror-circus/src/dc2.rs#L705) |

## DC3 0x8A71 opcode remapping

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x02` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x03` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x04` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x07` | Execute map branch | Handled | — | [dc3_original.rs:61](../../crates/crysirror-circus/src/dc3_original.rs#L61) |
| `0x08` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x09` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x0A` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x0E` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x10` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x12` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x13` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x14` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x15` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x16` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x18` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x1A` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x1B` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x1C` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x1E` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x20` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x22` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x26` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x27` | Execute map branch | Handled | — | [dc3_original.rs:59](../../crates/crysirror-circus/src/dc3_original.rs#L59) |
| `0x2C` | Execute map branch | Handled | — | [dc3_original.rs:62](../../crates/crysirror-circus/src/dc3_original.rs#L62) |
| `0x2D` | Execute map branch | Handled | — | [dc3_original.rs:63](../../crates/crysirror-circus/src/dc3_original.rs#L63) |
| `0x2E` | Execute map branch | Handled | — | [dc3_original.rs:64](../../crates/crysirror-circus/src/dc3_original.rs#L64) |
| `0x2F` | Execute map branch | Handled | — | [dc3_original.rs:65](../../crates/crysirror-circus/src/dc3_original.rs#L65) |
| `0x30` | Execute map branch | Handled | — | [dc3_original.rs:66](../../crates/crysirror-circus/src/dc3_original.rs#L66) |
| `0x32` | Execute map branch | Handled | — | [dc3_original.rs:67](../../crates/crysirror-circus/src/dc3_original.rs#L67) |
| `0x34` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x35` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x36` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x37` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x38` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x39` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x3A` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x3B` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x3C` | Execute map branch | Handled | — | [dc3_original.rs:68](../../crates/crysirror-circus/src/dc3_original.rs#L68) |
| `0x3E` | Execute map branch | Handled | — | [dc3_original.rs:69](../../crates/crysirror-circus/src/dc3_original.rs#L69) |
| `0x42` | Execute map branch | Handled | — | [dc3_original.rs:70](../../crates/crysirror-circus/src/dc3_original.rs#L70) |
| `0x43` | Execute map branch | Handled | — | [dc3_original.rs:71](../../crates/crysirror-circus/src/dc3_original.rs#L71) |
| `0x44` | Execute map branch | Handled | — | [dc3_original.rs:72](../../crates/crysirror-circus/src/dc3_original.rs#L72) |
| `0x45` | Execute map branch | Handled | — | [dc3_original.rs:73](../../crates/crysirror-circus/src/dc3_original.rs#L73) |
| `0x4A` | Execute map branch | Handled | — | [dc3_original.rs:74](../../crates/crysirror-circus/src/dc3_original.rs#L74) |
| `0x4D` | Execute map branch | Handled | — | [dc3_original.rs:75](../../crates/crysirror-circus/src/dc3_original.rs#L75) |
| `0x4F` | Execute map branch | Handled | — | [dc3_original.rs:76](../../crates/crysirror-circus/src/dc3_original.rs#L76) |
| `0x50` | Execute map branch | Handled | — | [dc3_original.rs:77](../../crates/crysirror-circus/src/dc3_original.rs#L77) |
| `0x51` | Execute map branch | Handled | — | [dc3_original.rs:78](../../crates/crysirror-circus/src/dc3_original.rs#L78) |
| `0x53` | Execute map branch | Handled | — | [dc3_original.rs:79](../../crates/crysirror-circus/src/dc3_original.rs#L79) |
| `0x54` | Execute map branch | Handled | — | [dc3_original.rs:79](../../crates/crysirror-circus/src/dc3_original.rs#L79) |
| `0x55` | Execute map branch | Handled | — | [dc3_original.rs:79](../../crates/crysirror-circus/src/dc3_original.rs#L79) |
| `0x56` | Execute map branch | Handled | — | [dc3_original.rs:80](../../crates/crysirror-circus/src/dc3_original.rs#L80) |
| `0x59` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x5A` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x5B` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x5C` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x5D` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x5E` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x5F` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x60` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x61` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x62` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x63` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x64` | Execute map branch | Handled | — | [dc3_original.rs:81](../../crates/crysirror-circus/src/dc3_original.rs#L81) |
| `0x66` | Execute map branch | Handled | — | [dc3_original.rs:82](../../crates/crysirror-circus/src/dc3_original.rs#L82) |
| `0x67` | Execute map branch | Handled | — | [dc3_original.rs:82](../../crates/crysirror-circus/src/dc3_original.rs#L82) |
| `0x68` | Execute map branch | Handled | — | [dc3_original.rs:82](../../crates/crysirror-circus/src/dc3_original.rs#L82) |
| `0x69` | Execute map branch | Handled | — | [dc3_original.rs:82](../../crates/crysirror-circus/src/dc3_original.rs#L82) |
| `0x6B` | Execute map branch | Handled | — | [dc3_original.rs:83](../../crates/crysirror-circus/src/dc3_original.rs#L83) |
| `0x6C` | Execute map branch | Handled | — | [dc3_original.rs:84](../../crates/crysirror-circus/src/dc3_original.rs#L84) |
| `0x6E` | Execute map branch | Handled | — | [dc3_original.rs:85](../../crates/crysirror-circus/src/dc3_original.rs#L85) |
| `0x6F` | Execute map branch | Handled | — | [dc3_original.rs:86](../../crates/crysirror-circus/src/dc3_original.rs#L86) |

## DC3WY 0xA09F shared handlers

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Native case0 does not advance the decoder. It yields to its process loop; it is not an exit or an invented scene end | Handled | — | [dc3.rs:641](../../crates/crysirror-circus/src/dc3.rs#L641) |
| `0x03` | Update state / history, state / system data, state / savepoint | Handled | — | [dc3.rs:652](../../crates/crysirror-circus/src/dc3.rs#L652) |
| `0x04` | Read Label | Handled | — | [dc3.rs:690](../../crates/crysirror-circus/src/dc3.rs#L690) |
| `0x05` | Audio playback/control (execute_audio) | Handled | Guard: a[0] == 254 | [dc3_audio.rs:75](../../crates/crysirror-circus/src/dc3_audio.rs#L75) |
| `0x06` | Text and message windows (execute_text) | Handled | — | [dc3_text.rs:231](../../crates/crysirror-circus/src/dc3_text.rs#L231) |
| `0x08` | 0x08: Buf42792/23576 = 1; copies template this+10292 (Rust slot 11, from 0x18 3) into the choice builder this+10236 and… | Handled | — | [dc3_menu.rs:63](../../crates/crysirror-circus/src/dc3_menu.rs#L63) |
| `0x09` | Update state / system data | Handled | Guard: a[0] != 0 | [dc3.rs:710](../../crates/crysirror-circus/src/dc3.rs#L710) |
| `0x0A` | Begin Menu | Handled | — | [dc3_menu.rs:49](../../crates/crysirror-circus/src/dc3_menu.rs#L49) |
| `0x0E` | Actual effect-selector fall-throughs of4233A0, including the bootstrap's14/32. Other selectors retain their native fence | Empty handler | Original no-op status not established; Guard: matches!(a[0], 7 \\| 10 \\| 11 \\| 13 \\| 14 \\| 23 \\| 24 \\| 25 \\| 30 \\| 31 \\| 32) | [dc3.rs:734](../../crates/crysirror-circus/src/dc3.rs#L734) |
| `0x0F` | Update state / halted | Handled | — | [dc3.rs:647](../../crates/crysirror-circus/src/dc3.rs#L647) |
| `0x12` | Release Audio Group | Handled | — | [dc3_audio.rs:152](../../crates/crysirror-circus/src/dc3_audio.rs#L152) |
| `0x13` | Update state / wait skippable, state / wait until, state / auxiliary flag | Handled | Guard: a[0] < 254 | [dc3.rs:1134](../../crates/crysirror-circus/src/dc3.rs#L1134) |
| `0x14` | Resource loading (execute_resources) | Handled | — | [dc3_resources.rs:199](../../crates/crysirror-circus/src/dc3_resources.rs#L199) |
| `0x15` | Execute execute branch | Handled | — | [dc3.rs:1040](../../crates/crysirror-circus/src/dc3.rs#L1040) |
| `0x18` | Define Group | Handled | Guard: a[0] > 0 | [dc3.rs:981](../../crates/crysirror-circus/src/dc3.rs#L981) |
| `0x19` | Update state / auxiliary flag | Handled | — | [dc3.rs:1112](../../crates/crysirror-circus/src/dc3.rs#L1112) |
| `0x1C` | Assign | Handled | Guard: a[0] == 1 | [dc3.rs:739](../../crates/crysirror-circus/src/dc3.rs#L739) |
| `0x1E` | Execute execute branch | Empty handler | Original no-op status not established; Guard: a[0] != 0 | [dc3.rs:735](../../crates/crysirror-circus/src/dc3.rs#L735) |
| `0x20` | Menus (execute_menu) | Partial | DC3WY case 0x20 activation (42B6C0/4112B0) is not connected | [dc3_menu.rs:34](../../crates/crysirror-circus/src/dc3_menu.rs#L34) |
| `0x22` | Execute execute branch | Handled | — | [dc3.rs:972](../../crates/crysirror-circus/src/dc3.rs#L972) |
| `0x23` | 0x23: var(99,2) = rand() % operand (the shared CRT sequence) | Handled | — | [dc3.rs:768](../../crates/crysirror-circus/src/dc3.rs#L768) |
| `0x31` | Menus (execute_menu) | Handled | — | [dc3_menu.rs:44](../../crates/crysirror-circus/src/dc3_menu.rs#L44) |
| `0x32` | Execute execute ui branch | Handled | — | [dc3_ui.rs:15](../../crates/crysirror-circus/src/dc3_ui.rs#L15) |
| `0x33` | Execute execute ui branch | Handled | — | [dc3_ui.rs:15](../../crates/crysirror-circus/src/dc3_ui.rs#L15) |
| `0x34` | Execute execute ui branch | Handled | — | [dc3_ui.rs:15](../../crates/crysirror-circus/src/dc3_ui.rs#L15) |
| `0x36` | Resource loading (execute_resources) | Handled | — | [dc3_resources.rs:198](../../crates/crysirror-circus/src/dc3_resources.rs#L198) |
| `0x3A` | Resource loading (execute_resources) | Partial | DC3WY 0x3A object slot {slot} (record copy) is not connected; Guard: !matches!(a[0], 0 \\| 1 \\| 14..=18 \\| 20 \\| 21) | [dc3_resources.rs:99](../../crates/crysirror-circus/src/dc3_resources.rs#L99) |
| `0x3B` | Load Image | Handled | — | [dc3.rs:922](../../crates/crysirror-circus/src/dc3.rs#L922) |
| `0x3D` | Portrait operations | Partial | DC3WY 0x3D rollback (0x1B copy) is not connected | [dc3_portrait.rs:185](../../crates/crysirror-circus/src/dc3_portrait.rs#L185) |
| `0x3F` | Menus (execute_rollback) | Partial | DC3WY 0x3F companion record restore (slots 6..9) is not connected | [dc3_advmenu.rs:218](../../crates/crysirror-circus/src/dc3_advmenu.rs#L218) |
| `0x41` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:32](../../crates/crysirror-circus/src/dc3_callbacks.rs#L32) |
| `0x43` | , then slot 1 reloads and object 0 takes its record | Handled | — | [dc3_advmenu.rs:176](../../crates/crysirror-circus/src/dc3_advmenu.rs#L176) |
| `0x45` | Release Audio Group | Handled | — | [dc3_audio.rs:104](../../crates/crysirror-circus/src/dc3_audio.rs#L104) |
| `0x46` | Release Audio Group | Handled | — | [dc3_audio.rs:104](../../crates/crysirror-circus/src/dc3_audio.rs#L104) |
| `0x48` | actual source early return | Empty handler | Original no-op status not established | [dc3_scene.rs:18](../../crates/crysirror-circus/src/dc3_scene.rs#L18) |
| `0x49` | actual source early return | Empty handler | Original no-op status not established | [dc3_scene.rs:18](../../crates/crysirror-circus/src/dc3_scene.rs#L18) |
| `0x4A` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:50](../../crates/crysirror-circus/src/dc3_callbacks.rs#L50) |
| `0x4D` | Execute execute branch | Handled | — | [dc3.rs:1117](../../crates/crysirror-circus/src/dc3.rs#L1117) |
| `0x54` | Menus (execute_menu) | Handled | — | [dc3_menu.rs:72](../../crates/crysirror-circus/src/dc3_menu.rs#L72) |
| `0x5A` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:51](../../crates/crysirror-circus/src/dc3_callbacks.rs#L51) |
| `0x5C` | Execute execute branch | Handled | — | [dc3.rs:908](../../crates/crysirror-circus/src/dc3.rs#L908) |
| `0x5D` | Resource loading (execute_resources) | Handled | — | [dc3_resources.rs:132](../../crates/crysirror-circus/src/dc3_resources.rs#L132) |
| `0x5E` | Update state / source count | Handled | — | [dc3.rs:895](../../crates/crysirror-circus/src/dc3.rs#L895) |
| `0x61` | Text and message windows (execute_text) | Handled | — | [dc3_text.rs:215](../../crates/crysirror-circus/src/dc3_text.rs#L215) |
| `0x62` | Text and message windows (execute_text) | Handled | — | [dc3_text.rs:219](../../crates/crysirror-circus/src/dc3_text.rs#L219) |
| `0x65` | Audio playback/control (execute_audio) | Handled | — | [dc3_audio.rs:76](../../crates/crysirror-circus/src/dc3_audio.rs#L76) |
| `0x67` | Stop Audio | Handled | Guard: a[0] == 255 | [dc3_audio.rs:88](../../crates/crysirror-circus/src/dc3_audio.rs#L88) |
| `0x68` | Audio playback/control (execute_audio) | Handled | Guard: a[0] >= 0 | [dc3_audio.rs:126](../../crates/crysirror-circus/src/dc3_audio.rs#L126) |
| `0x69` | Update state / builder / pending button, state / builder / pending sounds, state / builder / pending flags | Handled | — | [dc3.rs:1006](../../crates/crysirror-circus/src/dc3.rs#L1006) |
| `0x6A` | Image part | Handled | — | [dc3.rs:1052](../../crates/crysirror-circus/src/dc3.rs#L1052) |
| `0x6C` | Execute execute ui branch | Handled | — | [dc3_ui.rs:32](../../crates/crysirror-circus/src/dc3_ui.rs#L32) |
| `0x6E` | Update state / builder / group / position, state / builder / group / motion | Handled | — | [dc3_ui.rs:91](../../crates/crysirror-circus/src/dc3_ui.rs#L91) |
| `0x70` | Image part | Handled | — | [dc3.rs:1098](../../crates/crysirror-circus/src/dc3.rs#L1098) |
| `0x71` | Execute execute ui branch | Handled | — | [dc3_ui.rs:110](../../crates/crysirror-circus/src/dc3_ui.rs#L110) |
| `0x78` | Assign | Handled | — | [dc3.rs:839](../../crates/crysirror-circus/src/dc3.rs#L839) |
| `0x79` | Assign | Handled | — | [dc3.rs:850](../../crates/crysirror-circus/src/dc3.rs#L850) |
| `0x7A` | Assign | Handled | — | [dc3.rs:856](../../crates/crysirror-circus/src/dc3.rs#L856) |
| `0x7B` | Assign | Handled | — | [dc3.rs:887](../../crates/crysirror-circus/src/dc3.rs#L887) |
| `0x7C` | Execute execute branch | Handled | — | [dc3.rs:812](../../crates/crysirror-circus/src/dc3.rs#L812) |
| `0x7D` | Execute execute ui branch | Empty handler | Original no-op status not established; Guard: a[2] == 0 | [dc3_ui.rs:65](../../crates/crysirror-circus/src/dc3_ui.rs#L65) |
| `0x7E` | Update state / builder / offset | Handled | — | [dc3_ui.rs:109](../../crates/crysirror-circus/src/dc3_ui.rs#L109) |
| `0x81` | Execute execute portrait branch | Empty handler | Original no-op status not established; Guard: a[3] == 255 | [dc3_portrait.rs:51](../../crates/crysirror-circus/src/dc3_portrait.rs#L51) |
| `0x83` | Place object | Handled | — | [dc3.rs:958](../../crates/crysirror-circus/src/dc3.rs#L958) |
| `0x84` | 0x84 x,y,k,t: k&0x10 is a no-op; otherwise then moves portrait k to (x, y) over 6t ticks (60 when t is 0) | Empty handler | Original no-op status not established; Guard: (a[2] as u8) & 0x10 != 0 | [dc3_portrait.rs:137](../../crates/crysirror-circus/src/dc3_portrait.rs#L137) |
| `0x86` | Execute execute branch | Empty handler | Original no-op status not established; Guard: a[0] >= 64 | [dc3.rs:946](../../crates/crysirror-circus/src/dc3.rs#L946) |
| `0x94` | no 0x10/0x20/0x40 flag: native does nothing 0x94: Buf23569, the tint mode applies at publish | Handled | — | [dc3_portrait.rs:184](../../crates/crysirror-circus/src/dc3_portrait.rs#L184) |
| `0x9A` | Visual effects (execute_spawn_window) | Handled | — | [dc3_effects.rs:272](../../crates/crysirror-circus/src/dc3_effects.rs#L272) |
| `0x9B` | Visual effects (execute_spawn_window) | Handled | — | [dc3_effects.rs:273](../../crates/crysirror-circus/src/dc3_effects.rs#L273) |

## DC3WY 0xA09F shared handlers / instruction.opcode=0x32..=0x34

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x32` | Execute execute ui branch | Handled | — | [dc3_ui.rs:17](../../crates/crysirror-circus/src/dc3_ui.rs#L17) |
| `0x33` | Execute execute ui branch | Handled | — | [dc3_ui.rs:24](../../crates/crysirror-circus/src/dc3_ui.rs#L24) |

## DC4PH 0xABB6 opcodes

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Parser leaves PC on 0; native main only yields (LABEL_989). An end marker is an idle menu point, not a process-exit opc… | Handled | — | [machine.rs:1199](../../crates/crysirror-circus/src/machine.rs#L1199) |
| `0x01` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x02` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x03` | Native skip notice | Handled | — | [machine.rs:1222](../../crates/crysirror-circus/src/machine.rs#L1222) |
| `0x04` | Read Label | Handled | — | [machine.rs:1206](../../crates/crysirror-circus/src/machine.rs#L1206) |
| `0x05` | Audio/video control (execute_media) | Handled | — | [media_ops.rs:178](../../crates/crysirror-circus/src/machine/media_ops.rs#L178) |
| `0x06` | Begin native dialogue wait | Handled | — | [machine.rs:1290](../../crates/crysirror-circus/src/machine.rs#L1290) |
| `0x07` | Audio/video control (execute_media) | Handled | — | [media_ops.rs:176](../../crates/crysirror-circus/src/machine/media_ops.rs#L176) |
| `0x08` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x09` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:283](../../crates/crysirror-circus/src/machine/text_ops.rs#L283) |
| `0x0A` | Refresh dialogue controls | Handled | — | [machine.rs:1318](../../crates/crysirror-circus/src/machine.rs#L1318) |
| `0x0B` | Activate Panel | Handled | — | [machine.rs:1300](../../crates/crysirror-circus/src/machine.rs#L1300) |
| `0x0C` | Commit UI builder to the message panel | Handled | — | [text_ops.rs:248](../../crates/crysirror-circus/src/machine/text_ops.rs#L248) |
| `0x0D` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x0E` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x0F` | Framebuffer operation dispatcher | Handled | — | [effect_ops.rs:215](../../crates/crysirror-circus/src/machine/effect_ops.rs#L215) |
| `0x10` | Audio/video control (execute_media) | Handled | — | [media_ops.rs:177](../../crates/crysirror-circus/src/machine/media_ops.rs#L177) |
| `0x11` | Update state / protagonist names, state / savepoint, state / read run | Handled | — | [machine.rs:1392](../../crates/crysirror-circus/src/machine.rs#L1392) |
| `0x12` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x13` | Release Music | Handled | — | [machine.rs:1420](../../crates/crysirror-circus/src/machine.rs#L1420) |
| `0x14` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x15` | Update state / wait process, state / wait menu motion | Handled | Guard: a[0] == 255 | [machine.rs:1437](../../crates/crysirror-circus/src/machine.rs#L1437) |
| `0x16` | Update state / wait audio mask, state / wait audio started | Handled | — | [machine.rs:1461](../../crates/crysirror-circus/src/machine.rs#L1461) |
| `0x17` | Execute execute branch | Handled | — | [machine.rs:1478](../../crates/crysirror-circus/src/machine.rs#L1478) |
| `0x18` | Save/load (execute_save) | Handled | Guard: matches!(a[0], 4 \\| 24) | [save_ops.rs:174](../../crates/crysirror-circus/src/machine/save_ops.rs#L174) |
| `0x19` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x1A` | Group Reverse Order | Handled | — | [machine.rs:1586](../../crates/crysirror-circus/src/machine.rs#L1586) |
| `0x1B` | Graphics and composition (execute_visual) | Handled | — | [visual_ops.rs:61](../../crates/crysirror-circus/src/machine/visual_ops.rs#L61) |
| `0x1C` | Save/load (execute_save) | Handled | — | [save_ops.rs:136](../../crates/crysirror-circus/src/machine/save_ops.rs#L136) |
| `0x1D` | Save/load (execute_save) | Handled | — | [save_ops.rs:196](../../crates/crysirror-circus/src/machine/save_ops.rs#L196) |
| `0x1E` | System commands (execute_system) | Handled | — | [system_ops.rs:955](../../crates/crysirror-circus/src/machine/system_ops.rs#L955) |
| `0x1F` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x20` | Execute execute branch | Handled | — | [machine.rs:1626](../../crates/crysirror-circus/src/machine.rs#L1626) |
| `0x21` | Native21 skips its own already-decoded string, not a following instruction (LABEL1094 is strlen(current string)+1) | Empty handler | Original no-op status not established | [text_ops.rs:329](../../crates/crysirror-circus/src/machine/text_ops.rs#L329) |
| `0x22` | Copy Ui To Panel | Handled | — | [text_ops.rs:304](../../crates/crysirror-circus/src/machine/text_ops.rs#L304) |
| `0x23` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x24` | Group Reverse Order | Handled | — | [text_ops.rs:377](../../crates/crysirror-circus/src/machine/text_ops.rs#L377) |
| `0x25` | Execute execute branch | Handled | — | [machine.rs:1639](../../crates/crysirror-circus/src/machine.rs#L1639) |
| `0x26` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:180](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L180) |
| `0x27` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x28` | Falling Effect | Handled | — | [machine.rs:1650](../../crates/crysirror-circus/src/machine.rs#L1650) |
| `0x29` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x2A` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x2B` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x2C` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x2D` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x2E` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x2F` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x30` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x31` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x32` | Execute execute branch | Handled | — | [machine.rs:1632](../../crates/crysirror-circus/src/machine.rs#L1632) |
| `0x33` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:330](../../crates/crysirror-circus/src/machine/text_ops.rs#L330) |
| `0x34` | Execute execute branch | Handled | Guard: (1..=4).contains(&a[0]) | [machine.rs:1683](../../crates/crysirror-circus/src/machine.rs#L1683) |
| `0x35` | Execute execute branch | Handled | — | [machine.rs:1715](../../crates/crysirror-circus/src/machine.rs#L1715) |
| `0x36` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:149](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L149) |
| `0x37` | Resource loading (execute_resource) | Handled | — | [resource_ops.rs:651](../../crates/crysirror-circus/src/machine/resource_ops.rs#L651) |
| `0x38` | Decoration/sprite operation dispatcher | Handled | — | [sprite_ops.rs:142](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L142) |
| `0x39` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:219](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L219) |
| `0x3A` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x3B` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x3C` | Packed image commands (execute_packed) | Handled | Guard: (33..=36).contains(&a[0]) | [packed_ops.rs:90](../../crates/crysirror-circus/src/machine/packed_ops.rs#L90) |
| `0x3D` | Resource loading (execute_resource) | Handled | — | [resource_ops.rs:655](../../crates/crysirror-circus/src/machine/resource_ops.rs#L655) |
| `0x3E` | Enter a native menu | Handled | — | [text_ops.rs:200](../../crates/crysirror-circus/src/machine/text_ops.rs#L200) |
| `0x3F` | Resource loading (execute_resource) | Handled | — | [resource_ops.rs:767](../../crates/crysirror-circus/src/machine/resource_ops.rs#L767) |
| `0x40` | Resource loading (execute_resource) | Handled | — | [resource_ops.rs:823](../../crates/crysirror-circus/src/machine/resource_ops.rs#L823) |
| `0x41` | Resource loading (execute_resource) | Handled | — | [resource_ops.rs:855](../../crates/crysirror-circus/src/machine/resource_ops.rs#L855) |
| `0x42` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x43` | System commands (execute_system) | Handled | — | [system_ops.rs:716](../../crates/crysirror-circus/src/machine/system_ops.rs#L716) |
| `0x44` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:342](../../crates/crysirror-circus/src/machine/text_ops.rs#L342) |
| `0x45` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:364](../../crates/crysirror-circus/src/machine/text_ops.rs#L364) |
| `0x46` | Update state / voice timing from sprite | Handled | — | [machine.rs:1817](../../crates/crysirror-circus/src/machine.rs#L1817) |
| `0x47` | Execute execute branch | Handled | Guard: self.state.placement_declaration.is_none() | [machine.rs:1879](../../crates/crysirror-circus/src/machine.rs#L1879) |
| `0x48` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x49` | Resource loading (execute_resource) | Handled | — | [resource_ops.rs:866](../../crates/crysirror-circus/src/machine/resource_ops.rs#L866) |
| `0x4A` | Resource loading (execute_resource) | Handled | — | [resource_ops.rs:866](../../crates/crysirror-circus/src/machine/resource_ops.rs#L866) |
| `0x4B` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x4C` | Release Audio Group | Handled | — | [media_ops.rs:144](../../crates/crysirror-circus/src/machine/media_ops.rs#L144) |
| `0x4D` | Release Audio Group | Handled | — | [media_ops.rs:144](../../crates/crysirror-circus/src/machine/media_ops.rs#L144) |
| `0x4E` | Release Audio Group | Handled | — | [media_ops.rs:144](../../crates/crysirror-circus/src/machine/media_ops.rs#L144) |
| `0x4F` | Sync sprite states | Handled | — | [machine.rs:1897](../../crates/crysirror-circus/src/machine.rs#L1897) |
| `0x50` | Sync sprite states | Handled | — | [machine.rs:1897](../../crates/crysirror-circus/src/machine.rs#L1897) |
| `0x51` | Update state / animation prefix | Handled | — | [machine.rs:1549](../../crates/crysirror-circus/src/machine.rs#L1549) |
| `0x52` | Movie playback and control | Handled | — | [movie_ops.rs:36](../../crates/crysirror-circus/src/machine/movie_ops.rs#L36) |
| `0x53` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x54` | Execute execute branch | Handled | — | [machine.rs:1813](../../crates/crysirror-circus/src/machine.rs#L1813) |
| `0x55` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:437](../../crates/crysirror-circus/src/machine/text_ops.rs#L437) |
| `0x56` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:437](../../crates/crysirror-circus/src/machine/text_ops.rs#L437) |
| `0x57` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x58` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:445](../../crates/crysirror-circus/src/machine/text_ops.rs#L445) |
| `0x59` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:449](../../crates/crysirror-circus/src/machine/text_ops.rs#L449) |
| `0x5A` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:449](../../crates/crysirror-circus/src/machine/text_ops.rs#L449) |
| `0x5B` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:449](../../crates/crysirror-circus/src/machine/text_ops.rs#L449) |
| `0x5C` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:488](../../crates/crysirror-circus/src/machine/text_ops.rs#L488) |
| `0x5D` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x5E` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:517](../../crates/crysirror-circus/src/machine/text_ops.rs#L517) |
| `0x5F` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:532](../../crates/crysirror-circus/src/machine/text_ops.rs#L532) |
| `0x60` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x61` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:535](../../crates/crysirror-circus/src/machine/text_ops.rs#L535) |
| `0x62` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x63` | Execute execute branch | Handled | — | [machine.rs:1943](../../crates/crysirror-circus/src/machine.rs#L1943) |
| `0x64` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:543](../../crates/crysirror-circus/src/machine/text_ops.rs#L543) |
| `0x65` | Update state / system data | Handled | — | [machine.rs:1949](../../crates/crysirror-circus/src/machine.rs#L1949) |
| `0x66` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x67` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:548](../../crates/crysirror-circus/src/machine/text_ops.rs#L548) |
| `0x68` | Cursor | Handled | — | [machine.rs:1975](../../crates/crysirror-circus/src/machine.rs#L1975) |
| `0x69` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x6A` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:517](../../crates/crysirror-circus/src/machine/text_ops.rs#L517) |
| `0x6B` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:556](../../crates/crysirror-circus/src/machine/text_ops.rs#L556) |
| `0x6C` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x6D` | Audio/video control (execute_media) | Handled | — | [media_ops.rs:221](../../crates/crysirror-circus/src/machine/media_ops.rs#L221) |
| `0x6E` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x6F` | Audio/video control (execute_media) | Handled | — | [media_ops.rs:242](../../crates/crysirror-circus/src/machine/media_ops.rs#L242) |
| `0x70` | Release Music | Handled | — | [media_ops.rs:199](../../crates/crysirror-circus/src/machine/media_ops.rs#L199) |
| `0x71` | Audio/video control (execute_media) | Handled | Guard: a[0]>=0 | [media_ops.rs:154](../../crates/crysirror-circus/src/machine/media_ops.rs#L154) |
| `0x72` | Update state / visual / frame toggle, state / current button | Handled | — | [machine.rs:1976](../../crates/crysirror-circus/src/machine.rs#L1976) |
| `0x73` | Declare button image | Handled | — | [machine.rs:2000](../../crates/crysirror-circus/src/machine.rs#L2000) |
| `0x74` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:639](../../crates/crysirror-circus/src/machine/text_ops.rs#L639) |
| `0x75` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:648](../../crates/crysirror-circus/src/machine/text_ops.rs#L648) |
| `0x76` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:657](../../crates/crysirror-circus/src/machine/text_ops.rs#L657) |
| `0x77 Continuation` | Continuation operand record | Continuation | Not an independent operation | [opcodes.rs:14](../../crates/crysirror-circus/src/opcodes.rs#L14) |
| `0x78` | Update state / builder / position, state / builder / motion | Handled | — | [machine.rs:1670](../../crates/crysirror-circus/src/machine.rs#L1670) |
| `0x79` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:677](../../crates/crysirror-circus/src/machine/text_ops.rs#L677) |
| `0x7A` | Execute execute branch | Handled | — | [machine.rs:2004](../../crates/crysirror-circus/src/machine.rs#L2004) |
| `0x7B` | Update state / builder / image animations, state / builder / image animation targets, state / builder / image animation owners | Handled | — | [machine.rs:2023](../../crates/crysirror-circus/src/machine.rs#L2023) |
| `0x7C` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:681](../../crates/crysirror-circus/src/machine/text_ops.rs#L681) |
| `0x7D` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:687](../../crates/crysirror-circus/src/machine/text_ops.rs#L687) |
| `0x7E` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:691](../../crates/crysirror-circus/src/machine/text_ops.rs#L691) |
| `0x7F` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:696](../../crates/crysirror-circus/src/machine/text_ops.rs#L696) |
| `0x80` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:701](../../crates/crysirror-circus/src/machine/text_ops.rs#L701) |
| `0x81` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:729](../../crates/crysirror-circus/src/machine/text_ops.rs#L729) |
| `0x82` | case130 has no default assignment or getter side effect | Handled | Guard: !matches!(a[3],0..=2) | [machine.rs:2067](../../crates/crysirror-circus/src/machine.rs#L2067) |
| `0x83` | Execute execute branch | Handled | — | [machine.rs:2083](../../crates/crysirror-circus/src/machine.rs#L2083) |
| `0x84` | System commands (execute_system) | Handled | — | [system_ops.rs:800](../../crates/crysirror-circus/src/machine/system_ops.rs#L800) |
| `0x85` | Execute execute branch | Handled | — | [machine.rs:2088](../../crates/crysirror-circus/src/machine.rs#L2088) |
| `0x86` | Execute execute branch | Handled | — | [machine.rs:2095](../../crates/crysirror-circus/src/machine.rs#L2095) |
| `0x87` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:739](../../crates/crysirror-circus/src/machine/text_ops.rs#L739) |
| `0x88` | Update state / builder offset | Handled | — | [machine.rs:2145](../../crates/crysirror-circus/src/machine.rs#L2145) |
| `0x89` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:257](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L257) |
| `0x8A` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x8B` | sub_412910 Native object 10/20 maps to resource 4/5 (sub_41E6A0) | Handled | Guard: (16..=20).contains(&a[3]) | [machine.rs:2151](../../crates/crysirror-circus/src/machine.rs#L2151) |
| `0x8C` | explicitly selects [this+0x60cc] => native object75, which sub_41E6A0 maps from resource31 (message-window object) | Handled | Guard: a[3] == 0 | [machine.rs:2168](../../crates/crysirror-circus/src/machine.rs#L2168) |
| `0x8D` | Sprite Position | Handled | — | [machine.rs:2172](../../crates/crysirror-circus/src/machine.rs#L2172) |
| `0x8E` | Sprite state (execute_sprite) | Handled | Guard: a[3] & 0x20 != 0 | [sprite_ops.rs:319](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L319) |
| `0x8F` | Packed image commands (execute_packed) | Handled | Guard: a[3] & 0x30 != 0 | [packed_ops.rs:100](../../crates/crysirror-circus/src/machine/packed_ops.rs#L100) |
| `0x90` | Palette | Handled | — | [machine.rs:2176](../../crates/crysirror-circus/src/machine.rs#L2176) |
| `0x91` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x92` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x93` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x94` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:257](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L257) |
| `0x95` | Packed image commands (execute_packed) | Handled | Guard: a[3] == 5 | [packed_ops.rs:127](../../crates/crysirror-circus/src/machine/packed_ops.rs#L127) |
| `0x96` | Graphics and composition (execute_visual) | Handled | — | [visual_ops.rs:127](../../crates/crysirror-circus/src/machine/visual_ops.rs#L127) |
| `0x97` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x98` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0x99` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:252](../../crates/crysirror-circus/src/machine/effect_ops.rs#L252) |
| `0x9A` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:256](../../crates/crysirror-circus/src/machine/effect_ops.rs#L256) |
| `0x9B` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:281](../../crates/crysirror-circus/src/machine/effect_ops.rs#L281) |
| `0x9C` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:256](../../crates/crysirror-circus/src/machine/effect_ops.rs#L256) |
| `0x9D` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:299](../../crates/crysirror-circus/src/machine/effect_ops.rs#L299) |
| `0x9E` | System commands (execute_system) | Handled | — | [system_ops.rs:827](../../crates/crysirror-circus/src/machine/system_ops.rs#L827) |
| `0x9F` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:434](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L434) |
| `0xA0` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:434](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L434) |
| `0xA1` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:257](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L257) |
| `0xA2` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:348](../../crates/crysirror-circus/src/machine/effect_ops.rs#L348) |
| `0xA3` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0xA4` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:340](../../crates/crysirror-circus/src/machine/effect_ops.rs#L340) |
| `0xA5` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:344](../../crates/crysirror-circus/src/machine/effect_ops.rs#L344) |
| `0xA6` | Original empty dispatcher entry | Original no-op | — | [opcodes.rs:11](../../crates/crysirror-circus/src/opcodes.rs#L11) |
| `0xA7` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:180](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L180) |
| `0xA8` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:180](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L180) |
| `0xA9` | System commands (execute_system) | Handled | — | [system_ops.rs:763](../../crates/crysirror-circus/src/machine/system_ops.rs#L763) |
| `0xAA` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:365](../../crates/crysirror-circus/src/machine/effect_ops.rs#L365) |
| `0xAB` | Audio/video control (execute_media) | Handled | Guard: a[0] == 99 | [media_ops.rs:269](../../crates/crysirror-circus/src/machine/media_ops.rs#L269) |
| `0xAC` | Visual effects (execute_effects) | Handled | — | [effect_ops.rs:375](../../crates/crysirror-circus/src/machine/effect_ops.rs#L375) |
| `0xAD` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:735](../../crates/crysirror-circus/src/machine/text_ops.rs#L735) |
| `0xAE` | original early-return identity when no queued stage effect | Handled | Guard: a[3] == 255 | [machine.rs:1874](../../crates/crysirror-circus/src/machine.rs#L1874) |
| `0xAF` | Audio/video control (execute_media) | Handled | — | [media_ops.rs:263](../../crates/crysirror-circus/src/machine/media_ops.rs#L263) |
| `0xB0` | Record native command macros and select a bank | Handled | — | [macro_ops.rs:117](../../crates/crysirror-circus/src/machine/macro_ops.rs#L117) |
| `0xB1` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:538](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L538) |
| `0xB2` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:561](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L561) |
| `0xB3` | Graphics and composition (execute_visual) | Handled | — | [visual_ops.rs:135](../../crates/crysirror-circus/src/machine/visual_ops.rs#L135) |
| `0xB4` | embedded Tech pass p0: SRCALPHA / INVSRCALPHA, ordinary shared normal layers | Handled | — | [machine.rs:1864](../../crates/crysirror-circus/src/machine.rs#L1864) |
| `0xB5` | System commands (execute_system) | Handled | — | [system_ops.rs:831](../../crates/crysirror-circus/src/machine/system_ops.rs#L831) |

## DC4PH 0xABB6 opcodes / i.opcode=0x59 \| 0x5a \| 0x5b

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x59` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:465](../../crates/crysirror-circus/src/machine/text_ops.rs#L465) |
| `0x5A` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:466](../../crates/crysirror-circus/src/machine/text_ops.rs#L466) |

## DC4PH 0xABB6 opcodes / i.opcode=0x89 \| 0x94 \| 0xa1

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x89` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:259](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L259) |
| `0x94` | Sprite state (execute_sprite) | Handled | — | [sprite_ops.rs:270](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L270) |

## Selectors / dc2_map.rs / execute_map / a[0]

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x03` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x04` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x05` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x0C` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x0D` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x0E` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x0F` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x10` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x11` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x12` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x13` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x14` | Execute execute map branch | Empty handler | Original no-op status not established | [dc2_map.rs:40](../../crates/crysirror-circus/src/dc2_map.rs#L40) |
| `0x15` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x16` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x17` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x18` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x19` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x1A` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x1B` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x1C` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x1D` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x1E` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x1F` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x20` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x21` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x22` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x23` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x24` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x25` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x26` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x27` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x28` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x29` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x2A` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x2B` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x2C` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x2D` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x2E` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x2F` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x30` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x31` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x32` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x33` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x34` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x35` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x36` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x37` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x38` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x39` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x3A` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x3B` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x3C` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x3D` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x3E` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x3F` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x40` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x41` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x42` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x43` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x44` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x45` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x46` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x47` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x48` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x49` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x4A` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x4B` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x4C` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x4D` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x4E` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x4F` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x50` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x51` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x52` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x53` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x54` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x55` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x56` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x57` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x58` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x59` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x5A` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x5B` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x5C` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x5D` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x5E` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x5F` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x60` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x61` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x62` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |
| `0x63` | 's switch has no case for these selectors | Handled | — | [dc2_map.rs:42](../../crates/crysirror-circus/src/dc2_map.rs#L42) |

## Selectors / dc2_scene.rs / execute_scene / selector

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x03` | Scene transfer/state (execute_scene) | Handled | — | [dc2_scene.rs:34](../../crates/crysirror-circus/src/dc2_scene.rs#L34) |
| `0x04` | Scene transfer/state (execute_scene) | Handled | — | [dc2_scene.rs:33](../../crates/crysirror-circus/src/dc2_scene.rs#L33) |
| `0x05` | Scene transfer/state (execute_scene) | Handled | — | [dc2_scene.rs:34](../../crates/crysirror-circus/src/dc2_scene.rs#L34) |

## Selectors / dc2_ui.rs / execute_ui / selector / instruction.opcode=0x17

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Execute execute ui branch | Handled | — | [dc2_ui.rs:80](../../crates/crysirror-circus/src/dc2_ui.rs#L80) |
| `0x02` | Execute execute ui branch | Handled | — | [dc2_ui.rs:81](../../crates/crysirror-circus/src/dc2_ui.rs#L81) |
| `0x03` | Execute execute ui branch | Handled | — | [dc2_ui.rs:82](../../crates/crysirror-circus/src/dc2_ui.rs#L82) |
| `0x04` | Execute execute ui branch | Handled | — | [dc2_ui.rs:80](../../crates/crysirror-circus/src/dc2_ui.rs#L80) |

## Selectors / dc3_audio.rs / execute_audio / a[0] / instruction.opcode=0x65

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Audio playback/control (execute_audio) | Handled | — | [dc3_audio.rs:82](../../crates/crysirror-circus/src/dc3_audio.rs#L82) |
| `0x01` | Audio playback/control (execute_audio) | Handled | — | [dc3_audio.rs:83](../../crates/crysirror-circus/src/dc3_audio.rs#L83) |

## Selectors / dc3_callbacks.rs / execute_handlers / a[0] / instruction.opcode=0x41

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:33](../../crates/crysirror-circus/src/dc3_callbacks.rs#L33) |
| `0x01` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:34](../../crates/crysirror-circus/src/dc3_callbacks.rs#L34) |
| `0x02` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:35](../../crates/crysirror-circus/src/dc3_callbacks.rs#L35) |
| `0x03` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:36](../../crates/crysirror-circus/src/dc3_callbacks.rs#L36) |
| `0x04` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:37](../../crates/crysirror-circus/src/dc3_callbacks.rs#L37) |
| `0x05` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:38](../../crates/crysirror-circus/src/dc3_callbacks.rs#L38) |
| `0x06` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:39](../../crates/crysirror-circus/src/dc3_callbacks.rs#L39) |
| `0x0A` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:40](../../crates/crysirror-circus/src/dc3_callbacks.rs#L40) |
| `0x0B` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:41](../../crates/crysirror-circus/src/dc3_callbacks.rs#L41) |
| `0x0C` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:42](../../crates/crysirror-circus/src/dc3_callbacks.rs#L42) |
| `0x0D` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:43](../../crates/crysirror-circus/src/dc3_callbacks.rs#L43) |
| `0x0E` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:44](../../crates/crysirror-circus/src/dc3_callbacks.rs#L44) |
| `0x0F` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:45](../../crates/crysirror-circus/src/dc3_callbacks.rs#L45) |
| `0x10` | Execute execute handlers branch | Handled | — | [dc3_callbacks.rs:46](../../crates/crysirror-circus/src/dc3_callbacks.rs#L46) |

## Selectors / dc3_portrait.rs / execute_portrait / a[0] / instruction.opcode=0x81 if (a[3] as u8) & 0x40 != 0

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Portrait operations | Handled | — | [dc3_portrait.rs:109](../../crates/crysirror-circus/src/dc3_portrait.rs#L109) |
| `0x01` | Portrait operations | Handled | — | [dc3_portrait.rs:109](../../crates/crysirror-circus/src/dc3_portrait.rs#L109) |
| `0x02` | Portrait operations | Handled | — | [dc3_portrait.rs:109](../../crates/crysirror-circus/src/dc3_portrait.rs#L109) |
| `0x03` | Portrait mask | Handled | — | [dc3_portrait.rs:126](../../crates/crysirror-circus/src/dc3_portrait.rs#L126) |

## Selectors / dc3_system.rs / execute_system / a[0]

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x02` | System commands (execute_system) | Handled | — | [dc3_system.rs:25](../../crates/crysirror-circus/src/dc3_system.rs#L25) |
| `0x03` | System commands (execute_system) | Handled | — | [dc3_system.rs:25](../../crates/crysirror-circus/src/dc3_system.rs#L25) |
| `0x04` | operand nonzero writes slot 0 (only outside replay), zero writes slot var(99,254) | Handled | — | [dc3_system.rs:45](../../crates/crysirror-circus/src/dc3_system.rs#L45) |
| `0x05` | System commands (execute_system) | Partial | DC3WY case16 selector 6/1 (402130) is not connected | [dc3_system.rs:88](../../crates/crysirror-circus/src/dc3_system.rs#L88) |
| `0x06` | System commands (execute_system) | Partial | DC3WY case16 selector 6/1 (402130) is not connected | [dc3_system.rs:88](../../crates/crysirror-circus/src/dc3_system.rs#L88) |
| `0x07` | System commands (execute_system) | Handled | — | [dc3_system.rs:31](../../crates/crysirror-circus/src/dc3_system.rs#L31) |
| `0x08` | 8: (a1, 0) with renderer+24 forced to 1 for the call (D.C.III (a1)): the native window/fullscreen switch | Handled | — | [dc3_system.rs:177](../../crates/crysirror-circus/src/dc3_system.rs#L177) |
| `0x0A` | System commands (execute_system) | Handled | — | [dc3_system.rs:108](../../crates/crysirror-circus/src/dc3_system.rs#L108) |
| `0x0F` | System commands (execute_system) | Handled | — | [dc3_system.rs:77](../../crates/crysirror-circus/src/dc3_system.rs#L77) |
| `0x12` | case 18: 0 backs the 0x4528 preference bytes up, 2 restores them, 1 sets var(3,0) = 1 when they differ from the backup,… | Handled | — | [dc3_system.rs:54](../../crates/crysirror-circus/src/dc3_system.rs#L54) |
| `0x1A` | Native Mixer Controls | Handled | — | [dc3_system.rs:114](../../crates/crysirror-circus/src/dc3_system.rs#L114) |

## Selectors / dc3_system.rs / execute_system / a[1] / a[0]=18

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | System commands (execute_system) | Handled | — | [dc3_system.rs:57](../../crates/crysirror-circus/src/dc3_system.rs#L57) |
| `0x01` | System commands (execute_system) | Handled | — | [dc3_system.rs:58](../../crates/crysirror-circus/src/dc3_system.rs#L58) |

## Selectors / machine.rs / execute / a[0] / i.opcode=0x1a \| 0x24

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Execute execute branch | Handled | — | [machine.rs:1592](../../crates/crysirror-circus/src/machine.rs#L1592) |
| `0x02` | Execute execute branch | Handled | — | [machine.rs:1593](../../crates/crysirror-circus/src/machine.rs#L1593) |
| `0x03` | Execute execute branch | Handled | — | [machine.rs:1594](../../crates/crysirror-circus/src/machine.rs#L1594) |
| `0x04` | Execute execute branch | Handled | — | [machine.rs:1602](../../crates/crysirror-circus/src/machine.rs#L1602) |
| `0x05` | Execute execute branch | Handled | — | [machine.rs:1602](../../crates/crysirror-circus/src/machine.rs#L1602) |
| `0x06` | Execute execute branch | Handled | — | [machine.rs:1595](../../crates/crysirror-circus/src/machine.rs#L1595) |
| `0x07` | Execute execute branch | Handled | — | [machine.rs:1596](../../crates/crysirror-circus/src/machine.rs#L1596) |
| `0x0D` | Execute execute branch | Handled | — | [machine.rs:1597](../../crates/crysirror-circus/src/machine.rs#L1597) |
| `0x0E` | Execute execute branch | Handled | — | [machine.rs:1597](../../crates/crysirror-circus/src/machine.rs#L1597) |
| `0x0F` | Execute execute branch | Handled | — | [machine.rs:1597](../../crates/crysirror-circus/src/machine.rs#L1597) |
| `0x10` | Execute execute branch | Handled | — | [machine.rs:1597](../../crates/crysirror-circus/src/machine.rs#L1597) |
| `0x11` | Execute execute branch | Handled | — | [machine.rs:1597](../../crates/crysirror-circus/src/machine.rs#L1597) |
| `0xFE` | Group Reverse Order | Handled | — | [machine.rs:1605](../../crates/crysirror-circus/src/machine.rs#L1605) |
| `0xFF` | Group Reverse Order | Handled | — | [machine.rs:1605](../../crates/crysirror-circus/src/machine.rs#L1605) |

## Selectors / machine.rs / execute / a[0] / i.opcode=0x34 if matches!(a[0],5..=7)

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x05` | Execute execute branch | Handled | — | [machine.rs:1702](../../crates/crysirror-circus/src/machine.rs#L1702) |
| `0x06` | Execute execute branch | Handled | — | [machine.rs:1702](../../crates/crysirror-circus/src/machine.rs#L1702) |

## Selectors / machine.rs / execute / a[0] / i.opcode=0x43

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute execute branch | Handled | — | [machine.rs:1555](../../crates/crysirror-circus/src/machine.rs#L1555) |
| `0x01` | Execute execute branch | Handled | — | [machine.rs:1559](../../crates/crysirror-circus/src/machine.rs#L1559) |
| `0x02` | Execute execute branch | Handled | — | [machine.rs:1563](../../crates/crysirror-circus/src/machine.rs#L1563) |
| `0x03` | Execute execute branch | Handled | — | [machine.rs:1564](../../crates/crysirror-circus/src/machine.rs#L1564) |
| `0x04` | Execute execute branch | Handled | — | [machine.rs:1565](../../crates/crysirror-circus/src/machine.rs#L1565) |
| `0x05` | Execute execute branch | Handled | — | [machine.rs:1566](../../crates/crysirror-circus/src/machine.rs#L1566) |
| `0x0A` | Execute execute branch | Handled | — | [machine.rs:1567](../../crates/crysirror-circus/src/machine.rs#L1567) |
| `0x0B` | Execute execute branch | Handled | — | [machine.rs:1568](../../crates/crysirror-circus/src/machine.rs#L1568) |
| `0x0C` | Execute execute branch | Handled | — | [machine.rs:1569](../../crates/crysirror-circus/src/machine.rs#L1569) |
| `0x0D` | Execute execute branch | Handled | — | [machine.rs:1570](../../crates/crysirror-circus/src/machine.rs#L1570) |
| `0x0E` | Execute execute branch | Handled | — | [machine.rs:1571](../../crates/crysirror-circus/src/machine.rs#L1571) |
| `0x0F` | Execute execute branch | Handled | — | [machine.rs:1572](../../crates/crysirror-circus/src/machine.rs#L1572) |

## Selectors / machine.rs / execute / a[1] / i.opcode=0x34 if a[0]==99

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Execute execute branch | Handled | — | [machine.rs:1693](../../crates/crysirror-circus/src/machine.rs#L1693) |
| `0x02` | Execute execute branch | Handled | — | [machine.rs:1693](../../crates/crysirror-circus/src/machine.rs#L1693) |

## Selectors / machine/comment_ops.rs / execute_comment / a[0]

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute execute comment branch | Handled | — | [comment_ops.rs:81](../../crates/crysirror-circus/src/machine/comment_ops.rs#L81) |
| `0x01` | Execute execute comment branch | Handled | — | [comment_ops.rs:82](../../crates/crysirror-circus/src/machine/comment_ops.rs#L82) |
| `0x07` | Execute execute comment branch | Handled | — | [comment_ops.rs:82](../../crates/crysirror-circus/src/machine/comment_ops.rs#L82) |
| `0x62` | Execute execute comment branch | Handled | — | [comment_ops.rs:81](../../crates/crysirror-circus/src/machine/comment_ops.rs#L81) |

## Selectors / machine/media_ops.rs / execute_media / a[0] / i.opcode=0x05

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0xFE` | Audio/video control (execute_media) | Handled | — | [media_ops.rs:179](../../crates/crysirror-circus/src/machine/media_ops.rs#L179) |
| `0xFF` | Audio/video control (execute_media) | Handled | — | [media_ops.rs:185](../../crates/crysirror-circus/src/machine/media_ops.rs#L185) |

## Selectors / machine/portrait_ops.rs / execute_portrait / a[0]

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Execute execute portrait branch | Handled | — | [portrait_ops.rs:151](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L151) |
| `0x01` | Execute execute portrait branch | Handled | — | [portrait_ops.rs:145](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L145) |
| `0x02` | Execute execute portrait branch | Handled | — | [portrait_ops.rs:146](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L146) |

## Selectors / machine/sprite_ops.rs / execute_decoration / a[0]

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | Sprite state (execute_decoration) | Handled | — | [sprite_ops.rs:798](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L798) |
| `0x01` | Sprite state (execute_decoration) | Handled | — | [sprite_ops.rs:799](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L799) |
| `0x02` | Sprite state (execute_decoration) | Handled | — | [sprite_ops.rs:881](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L881) |
| `0x03` | Sprite state (execute_decoration) | Handled | — | [sprite_ops.rs:800](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L800) |
| `0x04` | Sprite state (execute_decoration) | Handled | — | [sprite_ops.rs:800](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L800) |
| `0x05` | Sprite state (execute_decoration) | Handled | — | [sprite_ops.rs:801](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L801) |

## Selectors / machine/system_ops.rs / execute_system / a[0] / i.opcode=0x18

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | System commands (execute_system) | Handled | — | [system_ops.rs:872](../../crates/crysirror-circus/src/machine/system_ops.rs#L872) |
| `0x01` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x02` | System commands (execute_system) | Handled | — | [system_ops.rs:874](../../crates/crysirror-circus/src/machine/system_ops.rs#L874) |
| `0x03` | System commands (execute_system) | Handled | — | [system_ops.rs:874](../../crates/crysirror-circus/src/machine/system_ops.rs#L874) |
| `0x04` | System commands (execute_system) | Handled | — | [system_ops.rs:872](../../crates/crysirror-circus/src/machine/system_ops.rs#L872) |
| `0x05` | System commands (execute_system) | Handled | — | [system_ops.rs:872](../../crates/crysirror-circus/src/machine/system_ops.rs#L872) |
| `0x06` | System commands (execute_system) | Handled | — | [system_ops.rs:875](../../crates/crysirror-circus/src/machine/system_ops.rs#L875) |
| `0x07` | System commands (execute_system) | Handled | — | [system_ops.rs:872](../../crates/crysirror-circus/src/machine/system_ops.rs#L872) |
| `0x08` | System commands (execute_system) | Handled | — | [system_ops.rs:894](../../crates/crysirror-circus/src/machine/system_ops.rs#L894) |
| `0x09` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x0A` | System commands (execute_system) | Handled | — | [system_ops.rs:895](../../crates/crysirror-circus/src/machine/system_ops.rs#L895) |
| `0x0B` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x0C` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x0D` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x0E` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x0F` | System commands (execute_system) | Handled | — | [system_ops.rs:872](../../crates/crysirror-circus/src/machine/system_ops.rs#L872) |
| `0x10` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x11` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x12` | System commands (execute_system) | Handled | — | [system_ops.rs:896](../../crates/crysirror-circus/src/machine/system_ops.rs#L896) |
| `0x13` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x14` | System commands (execute_system) | Handled | — | [system_ops.rs:935](../../crates/crysirror-circus/src/machine/system_ops.rs#L935) |
| `0x15` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |
| `0x16` | System commands (execute_system) | Handled | — | [system_ops.rs:944](../../crates/crysirror-circus/src/machine/system_ops.rs#L944) |
| `0x17` | System commands (execute_system) | Handled | — | [system_ops.rs:944](../../crates/crysirror-circus/src/machine/system_ops.rs#L944) |
| `0x18` | System commands (execute_system) | Handled | — | [system_ops.rs:872](../../crates/crysirror-circus/src/machine/system_ops.rs#L872) |
| `0x19` | System commands (execute_system) | Handled | — | [system_ops.rs:944](../../crates/crysirror-circus/src/machine/system_ops.rs#L944) |
| `0x1A` | System commands (execute_system) | Handled | — | [system_ops.rs:872](../../crates/crysirror-circus/src/machine/system_ops.rs#L872) |
| `0x1B` | System commands (execute_system) | Handled | — | [system_ops.rs:951](../../crates/crysirror-circus/src/machine/system_ops.rs#L951) |
| `0x1C` | System commands (execute_system) | Handled | — | [system_ops.rs:952](../../crates/crysirror-circus/src/machine/system_ops.rs#L952) |
| `0x1D` | System commands (execute_system) | Handled | — | [system_ops.rs:935](../../crates/crysirror-circus/src/machine/system_ops.rs#L935) |
| `0x63` | System commands (execute_system) | Empty handler | Original no-op status not established | [system_ops.rs:873](../../crates/crysirror-circus/src/machine/system_ops.rs#L873) |

## Selectors / machine/system_ops.rs / execute_system / a[0] / i.opcode=0x18 / a[0]=22 \| 23 \| 25

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x16` | System commands (execute_system) | Handled | — | [system_ops.rs:946](../../crates/crysirror-circus/src/machine/system_ops.rs#L946) |
| `0x17` | System commands (execute_system) | Handled | — | [system_ops.rs:947](../../crates/crysirror-circus/src/machine/system_ops.rs#L947) |

## Selectors / machine/system_ops.rs / execute_system / a[0] / i.opcode=0x1e

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | System commands (execute_system) | Handled | — | [system_ops.rs:956](../../crates/crysirror-circus/src/machine/system_ops.rs#L956) |
| `0x02` | System commands (execute_system) | Handled | — | [system_ops.rs:957](../../crates/crysirror-circus/src/machine/system_ops.rs#L957) |
| `0x06` | System commands (execute_system) | Handled | — | [system_ops.rs:969](../../crates/crysirror-circus/src/machine/system_ops.rs#L969) |
| `0x07` | System commands (execute_system) | Empty handler | Original no-op status not established; Guard: a[1] == 0 | [system_ops.rs:973](../../crates/crysirror-circus/src/machine/system_ops.rs#L973) |
| `0x09` | System commands (execute_system) | Handled | — | [system_ops.rs:986](../../crates/crysirror-circus/src/machine/system_ops.rs#L986) |
| `0x12` | System commands (execute_system) | Handled | — | [system_ops.rs:997](../../crates/crysirror-circus/src/machine/system_ops.rs#L997) |
| `0x13` | System commands (execute_system) | Handled | — | [system_ops.rs:1049](../../crates/crysirror-circus/src/machine/system_ops.rs#L1049) |
| `0x14` | System commands (execute_system) | Handled | — | [system_ops.rs:1050](../../crates/crysirror-circus/src/machine/system_ops.rs#L1050) |
| `0x33` | System commands (execute_system) | Handled | Guard: a[1]==1 | [system_ops.rs:1036](../../crates/crysirror-circus/src/machine/system_ops.rs#L1036) |
| `0x62` | System commands (execute_system) | Handled | — | [system_ops.rs:1056](../../crates/crysirror-circus/src/machine/system_ops.rs#L1056) |
| `0x63` | System commands (execute_system) | Handled | — | [system_ops.rs:1081](../../crates/crysirror-circus/src/machine/system_ops.rs#L1081) |
| `0x69` | System commands (execute_system) | Handled | — | [system_ops.rs:1087](../../crates/crysirror-circus/src/machine/system_ops.rs#L1087) |
| `0xC8` | System commands (execute_system) | Handled | — | [system_ops.rs:1089](../../crates/crysirror-circus/src/machine/system_ops.rs#L1089) |
| `0xC9` | System commands (execute_system) | Handled | — | [system_ops.rs:1094](../../crates/crysirror-circus/src/machine/system_ops.rs#L1094) |
| `0xCA` | System commands (execute_system) | Handled | — | [system_ops.rs:1094](../../crates/crysirror-circus/src/machine/system_ops.rs#L1094) |
| `0xCB` | System commands (execute_system) | Handled | — | [system_ops.rs:1118](../../crates/crysirror-circus/src/machine/system_ops.rs#L1118) |
| `0xCC` | System commands (execute_system) | Handled | — | [system_ops.rs:1219](../../crates/crysirror-circus/src/machine/system_ops.rs#L1219) |
| `0xCE` | System commands (execute_system) | Handled | — | [system_ops.rs:1263](../../crates/crysirror-circus/src/machine/system_ops.rs#L1263) |
| `0xD3` | System commands (execute_system) | Handled | — | [system_ops.rs:1288](../../crates/crysirror-circus/src/machine/system_ops.rs#L1288) |

## Selectors / machine/system_ops.rs / execute_system / a[0] / i.opcode=0x43

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | System commands (execute_system) | Handled | — | [system_ops.rs:718](../../crates/crysirror-circus/src/machine/system_ops.rs#L718) |
| `0x01` | System commands (execute_system) | Handled | — | [system_ops.rs:719](../../crates/crysirror-circus/src/machine/system_ops.rs#L719) |
| `0x02` | System commands (execute_system) | Handled | — | [system_ops.rs:720](../../crates/crysirror-circus/src/machine/system_ops.rs#L720) |
| `0x03` | System commands (execute_system) | Handled | — | [system_ops.rs:721](../../crates/crysirror-circus/src/machine/system_ops.rs#L721) |
| `0x04` | System commands (execute_system) | Handled | — | [system_ops.rs:722](../../crates/crysirror-circus/src/machine/system_ops.rs#L722) |
| `0x05` | System commands (execute_system) | Handled | — | [system_ops.rs:723](../../crates/crysirror-circus/src/machine/system_ops.rs#L723) |
| `0x06` | System commands (execute_system) | Handled | — | [system_ops.rs:724](../../crates/crysirror-circus/src/machine/system_ops.rs#L724) |
| `0x0A` | System commands (execute_system) | Handled | — | [system_ops.rs:725](../../crates/crysirror-circus/src/machine/system_ops.rs#L725) |
| `0x0B` | System commands (execute_system) | Handled | — | [system_ops.rs:726](../../crates/crysirror-circus/src/machine/system_ops.rs#L726) |
| `0x0C` | System commands (execute_system) | Handled | — | [system_ops.rs:727](../../crates/crysirror-circus/src/machine/system_ops.rs#L727) |
| `0x0D` | System commands (execute_system) | Handled | — | [system_ops.rs:728](../../crates/crysirror-circus/src/machine/system_ops.rs#L728) |
| `0x0E` | System commands (execute_system) | Handled | — | [system_ops.rs:729](../../crates/crysirror-circus/src/machine/system_ops.rs#L729) |
| `0x0F` | System commands (execute_system) | Handled | — | [system_ops.rs:730](../../crates/crysirror-circus/src/machine/system_ops.rs#L730) |
| `0x10` | System commands (execute_system) | Handled | — | [system_ops.rs:731](../../crates/crysirror-circus/src/machine/system_ops.rs#L731) |

## Selectors / machine/system_ops.rs / execute_system / a[1] / i.opcode=0x1e / a[0]=18

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | System commands (execute_system) | Handled | — | [system_ops.rs:999](../../crates/crysirror-circus/src/machine/system_ops.rs#L999) |
| `0x01` | System commands (execute_system) | Handled | — | [system_ops.rs:1004](../../crates/crysirror-circus/src/machine/system_ops.rs#L1004) |

## Selectors / machine/system_ops.rs / execute_system / a[1] / i.opcode=0x1e / a[0]=204

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x00` | System commands (execute_system) | Handled | — | [system_ops.rs:1234](../../crates/crysirror-circus/src/machine/system_ops.rs#L1234) |
| `0x01` | System commands (execute_system) | Handled | — | [system_ops.rs:1235](../../crates/crysirror-circus/src/machine/system_ops.rs#L1235) |
| `0x02` | System commands (execute_system) | Handled | — | [system_ops.rs:1236](../../crates/crysirror-circus/src/machine/system_ops.rs#L1236) |
| `0x03` | System commands (execute_system) | Handled | — | [system_ops.rs:1236](../../crates/crysirror-circus/src/machine/system_ops.rs#L1236) |

## Selectors / machine/text_ops.rs / execute_text / a[0] / i.opcode=0x24

| ID / API | Purpose | Status | Gap / condition | Source |
| --- | --- | --- | --- | --- |
| `0x01` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:386](../../crates/crysirror-circus/src/machine/text_ops.rs#L386) |
| `0x02` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:386](../../crates/crysirror-circus/src/machine/text_ops.rs#L386) |
| `0x03` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:386](../../crates/crysirror-circus/src/machine/text_ops.rs#L386) |
| `0x04` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:386](../../crates/crysirror-circus/src/machine/text_ops.rs#L386) |
| `0x05` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:386](../../crates/crysirror-circus/src/machine/text_ops.rs#L386) |
| `0x06` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:386](../../crates/crysirror-circus/src/machine/text_ops.rs#L386) |
| `0x07` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:386](../../crates/crysirror-circus/src/machine/text_ops.rs#L386) |
| `0x0D` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:387](../../crates/crysirror-circus/src/machine/text_ops.rs#L387) |
| `0x0E` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:387](../../crates/crysirror-circus/src/machine/text_ops.rs#L387) |
| `0x0F` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:388](../../crates/crysirror-circus/src/machine/text_ops.rs#L388) |
| `0x10` | Text and message windows (execute_text) | Handled | — | [text_ops.rs:388](../../crates/crysirror-circus/src/machine/text_ops.rs#L388) |

## Subselectors and conditional dispatch

Patterns retain their source namespace and guards. `_` is the fallback of that table. Boolean/optional results follow the function's return contract. Call-site names identify the implementation to inspect, including indirect service calls.

### adapter.rs / Bridge<'_>::define_group

```rust
fn define_group(&mut self, slot: u8, group: &Group) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(layer) = layers.iter_mut().rev().find(\|v\| v["layer"]["id"] == id)` | Local state/value operations | Handled | [809](../../crates/crysirror-circus/src/adapter.rs#L809) |

### adapter.rs / Bridge<'_>::emit

```rust
fn emit(&mut self, event: &Event) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind; under event=Event::CenterAudioPan { kind }` | `0` | Local state/value operations | Handled | [1403](../../crates/crysirror-circus/src/adapter.rs#L1403) |
| `kind; under event=Event::CenterAudioPan { kind }` | `1` | Local state/value operations | Handled | [1404](../../crates/crysirror-circus/src/adapter.rs#L1404) |
| `kind; under event=Event::CenterAudioPan { kind }` | `2` | Local state/value operations | Handled | [1405](../../crates/crysirror-circus/src/adapter.rs#L1405) |
| `kind; under event=Event::CenterAudioPan { kind }` | `_` | Local state/value operations | Unsupported: Circus native pan category | [1406](../../crates/crysirror-circus/src/adapter.rs#L1406) |

### adapter/audio.rs / Bridge<'_>::native_balance

```rust
fn native_balance(&mut self,kind:u8,from:i32,to:i32,steps:u32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0` | Local state/value operations | Handled | [512](../../crates/crysirror-circus/src/adapter/audio.rs#L512) |
| `kind` | `1` | Local state/value operations | Handled | [512](../../crates/crysirror-circus/src/adapter/audio.rs#L512) |
| `kind` | `2` | Local state/value operations | Handled | [512](../../crates/crysirror-circus/src/adapter/audio.rs#L512) |
| `kind` | `_` | Local state/value operations | Unsupported: Circus balance category | [512](../../crates/crysirror-circus/src/adapter/audio.rs#L512) |

### adapter/audio.rs / Bridge<'_>::native_rate_group_ready

```rust
fn native_rate_group_ready(&mut self,kind:u8,percent:i32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0` | Local state/value operations | Handled | [586](../../crates/crysirror-circus/src/adapter/audio.rs#L586) |
| `kind` | `1` | Local state/value operations | Handled | [586](../../crates/crysirror-circus/src/adapter/audio.rs#L586) |
| `kind` | `2` | Local state/value operations | Handled | [586](../../crates/crysirror-circus/src/adapter/audio.rs#L586) |
| `kind` | `_` | Local state/value operations | Unsupported: Circus native frequency category | [586](../../crates/crysirror-circus/src/adapter/audio.rs#L586) |

### adapter/audio.rs / Bridge<'_>::play_audio_policy

```rust
fn play_audio_policy(
        &mut self,
        channel: u32,
        name: &str,
        voice: bool,
        looping: bool,
        loop_start: u64,
        loop_end: Option<u64>,
        resident:bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self .services .mixer_controls .context("Circus native mixer controls missing")? .enabled[kind]` | Local state/value operations | Handled | [445](../../crates/crysirror-circus/src/adapter/audio.rs#L445) |

### adapter/audio.rs / Bridge<'_>::play_native_music

```rust
fn play_native_music(&mut self,asset:&crate::machine::MusicAsset,looping:bool,resident:bool) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.play_audio_policy(0,&asset.name,false,looping,if resident{0}else{asset.loop_start},if resident{None}else{asset.loop_end},resident)?` | Local state/value operations | Handled | [554](../../crates/crysirror-circus/src/adapter/audio.rs#L554) |

### adapter/audio.rs / Bridge<'_>::publish_native_sound

```rust
fn publish_native_sound(&mut self,declaration:&crate::machine::media_ops::AudioDeclaration) -> Result<Option<crate::machine::media_ops::AudioPublication>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `declaration.name.is_empty()\|\|!self.services.mixer_controls.context("Circus mixer controls missing")?.enabled[kind]` | Local state/value operations | Handled | [145](../../crates/crysirror-circus/src/adapter/audio.rs#L145) |
| `condition` | `declaration.name.contains("silent")` | Local state/value operations | Handled | [154](../../crates/crysirror-circus/src/adapter/audio.rs#L154) |
| `condition` | `let Some(index)=self.services.sound_prefixes.as_ref().context("Circus native SFX prefixes")?.iter().position(\|prefix\|declaration.name.get(..prefix.len()).is_some_and(\|name\|name.eq_ignore_ascii_case(prefix)))` | Local state/value operations | Handled | [181](../../crates/crysirror-circus/src/adapter/audio.rs#L181) |
| `condition` | `!self.play_audio(channel,&declaration.name,declaration.voice,declaration.looping,0,None)?` | Local state/value operations | Handled | [190](../../crates/crysirror-circus/src/adapter/audio.rs#L190) |

### adapter/audio.rs / Bridge<'_>::release_native_group

```rust
fn release_native_group(&mut self,kind:u8,forced:bool,steps:u32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0` | Local state/value operations | Handled | [98](../../crates/crysirror-circus/src/adapter/audio.rs#L98) |
| `kind` | `1` | Local state/value operations | Handled | [98](../../crates/crysirror-circus/src/adapter/audio.rs#L98) |
| `kind` | `2` | Local state/value operations | Handled | [98](../../crates/crysirror-circus/src/adapter/audio.rs#L98) |
| `kind` | `_` | Local state/value operations | Unsupported: Circus audio stop category | [98](../../crates/crysirror-circus/src/adapter/audio.rs#L98) |
| `condition` | `kind==0&&!forced&&self.services.sound_stop_group.is_none()&&self.services.native_profile==Some(crate::Profile::Dc3WithYou)` | self.stop_dc3_music | Handled | [99](../../crates/crysirror-circus/src/adapter/audio.rs#L99) |

### adapter/comments.rs / Bridge<'_>::comment_request

```rust
fn comment_request(
        &mut self,
        c: &mut NativeComments,
        request: &CommentRequest,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector` | `0` | self.comment_page | Handled | [499](../../crates/crysirror-circus/src/adapter/comments.rs#L499) |
| `selector` | `1` | Local state/value operations | Handled | [508](../../crates/crysirror-circus/src/adapter/comments.rs#L508) |
| `selector` | `2 if backend == 1` | i32::from | Handled | [522](../../crates/crysirror-circus/src/adapter/comments.rs#L522) |
| `selector` | `2` | i32::from | Handled | [523](../../crates/crysirror-circus/src/adapter/comments.rs#L523) |
| `selector` | `3 if backend == 1` | i32::from | Handled | [537](../../crates/crysirror-circus/src/adapter/comments.rs#L537) |
| `selector` | `4 if backend == 1` | Local state/value operations | Handled | [538](../../crates/crysirror-circus/src/adapter/comments.rs#L538) |
| `selector` | `5 if backend == 1 && args[3] != 0` | i32::from | Handled | [539](../../crates/crysirror-circus/src/adapter/comments.rs#L539) |
| `selector` | `6 if backend == 1` | i32::from | Handled | [542](../../crates/crysirror-circus/src/adapter/comments.rs#L542) |
| `selector` | `7` | Local state/value operations | Handled | [543](../../crates/crysirror-circus/src/adapter/comments.rs#L543) |
| `selector` | `98` | Local state/value operations | Handled | [548](../../crates/crysirror-circus/src/adapter/comments.rs#L548) |
| `selector` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [553](../../crates/crysirror-circus/src/adapter/comments.rs#L553) |
| `condition` | `backend == 2 && selector == Some(2) && args[1] == -1 && c.force_finish && now.saturating_sub(c.last) > 10` | self.comment_page | Handled | [556](../../crates/crysirror-circus/src/adapter/comments.rs#L556) |
| `condition` | `selector.is_none() && c.interval != 0 && now.saturating_sub(c.last) > u64::try_from(i64::from(c.interval) * 10).unwrap_or(0)` | self.comment_page | Handled | [597](../../crates/crysirror-circus/src/adapter/comments.rs#L597) |

### adapter/comments.rs / Bridge<'_>::publish_comments

```rust
fn publish_comments(&mut self, c: &mut NativeComments) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(layer) = layers.iter_mut().find(\|v\| v["layer"]["id"] == *id)` | Local state/value operations | Handled | [854](../../crates/crysirror-circus/src/adapter/comments.rs#L854) |

### adapter/control_records.rs / declaration_attributes

```rust
fn declaration_attributes(group:&Group,layers:&mut Vec<Value>,base:u32) -> Result<BTreeSet<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(layer)=layers.iter().find(\|command\|command["layer"]["id"]==id).map(\|command\|command["layer"].clone())` | Local state/value operations | Handled | [59](../../crates/crysirror-circus/src/adapter/control_records.rs#L59) |

### adapter/dc1_story.rs / Dc1Vm::system_layers

```rust
fn system_layers(&mut self, host: &mut dyn HostServices, state: &mut StoryHost) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `story.mode` | `26` | Local state/value operations | Handled | [886](../../crates/crysirror-circus/src/adapter/dc1_story.rs#L886) |
| `story.mode` | `27 \| 31 \| 32` | crate::dc1_story::SystemMenu::default | Handled | [887](../../crates/crysirror-circus/src/adapter/dc1_story.rs#L887) |
| `story.mode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [888](../../crates/crysirror-circus/src/adapter/dc1_story.rs#L888) |

### adapter/dc1_story.rs / Dc1Vm::visual_layers

```rust
fn visual_layers(&self, visual: &Visual, base: u32, shake: [i32; 2], tables: &crate::dc1_frontend::Story) -> Result<Vec<Value>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode; under &visual.overlay=Some((name, mode, offset))` | `1` | Local state/value operations | Handled | [643](../../crates/crysirror-circus/src/adapter/dc1_story.rs#L643) |
| `mode; under &visual.overlay=Some((name, mode, offset))` | `2` | Local state/value operations | Handled | [644](../../crates/crysirror-circus/src/adapter/dc1_story.rs#L644) |
| `mode; under &visual.overlay=Some((name, mode, offset))` | `3` | Local state/value operations | Handled | [645](../../crates/crysirror-circus/src/adapter/dc1_story.rs#L645) |
| `mode; under &visual.overlay=Some((name, mode, offset))` | `_` | Local state/value operations | Handled | [646](../../crates/crysirror-circus/src/adapter/dc1_story.rs#L646) |

### adapter/dc2_menu_view.rs / View::tick

```rust
fn tick(
        &mut self,
        bridge: &mut Bridge<'_>,
        menu: &MenuWait,
        settings: &crate::dc2_settings::SystemData,
    ) -> Result<Frame>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == 255` | self.sounds.pop_front | Handled | [128](../../crates/crysirror-circus/src/adapter/dc2_menu_view.rs#L128) |
| `condition` | `let Some(name) = &button.sounds[0]` | self.sounds.push_back, Sound::Voice | Handled | [166](../../crates/crysirror-circus/src/adapter/dc2_menu_view.rs#L166) |
| `condition` | `events.iter().any(\|event\| { event["widget"]["id"].as_u64() == Some(u64::from(id)) && event["kind"] == "activate" })` | self.sounds.push_back, Sound::Id, Sound::Voice | Handled | [170](../../crates/crysirror-circus/src/adapter/dc2_menu_view.rs#L170) |
| `condition` | `let Some(name) = &button.sounds[1]` | self.sounds.push_back, Sound::Voice | Handled | [181](../../crates/crysirror-circus/src/adapter/dc2_menu_view.rs#L181) |

### adapter/dc3.rs / finish_frame

```rust
fn finish_frame(
    machine: &mut crate::dc3::Machine,
    bridge: &mut Bridge<'_>,
    pending: &mut Option<Outcome>,
    now: u64,
    cursor_deadline: Option<u64>,
) -> Result<VmYield>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!machine.publish_audio(bridge).with_context(\|\| { format!( "Circus DC3WY {} dispatch fence pc={:#x} opcode={:#04x}", machine.state.scene, machine.state.last_offset, machine.state.last_opcode.unwrap_or(0) ) })?` | Local state/value operations | Handled | [258](../../crates/crysirror-circus/src/adapter/dc3.rs#L258) |

### adapter/dc3_music.rs / Bridge<'_>::play_dc3_music

```rust
fn play_dc3_music(&mut self, asset: &MusicAsset, looping: bool) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `asset.name.is_empty() \|\| !self.services.mixer_controls.context("DC3 music mixer owner")?.enabled[0]` | Local state/value operations | Handled | [35](../../crates/crysirror-circus/src/adapter/dc3_music.rs#L35) |

### adapter/effect_stage.rs / Bridge<'_>::stage_batch

```rust
fn stage_batch(
        &mut self,
        operations: &[StageOperation],
        ticks: i32,
        references: &[StageReference],
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `operation.mode == 3` | Local state/value operations | Handled | [434](../../crates/crysirror-circus/src/adapter/effect_stage.rs#L434) |
| `condition` | `operation.mode >= 2` | self.visual_root, serde_json::from_slice, EngineQuery::Layer, self.submit | Handled | [437](../../crates/crysirror-circus/src/adapter/effect_stage.rs#L437) |
| `condition` | `operation.restore_position && ((0..=5).contains(&operation.selector) \|\| operation.selector == 255)` | serde_json::from_slice, EngineQuery::Layer, self.submit | Handled | [440](../../crates/crysirror-circus/src/adapter/effect_stage.rs#L440) |
| `condition` | `operation.apply_view && ((0..=5).contains(&operation.selector) \|\| operation.selector == 255)` | i64::from, i32::try_from, self.visual_root, serde_json::from_slice, EngineQuery::Layer | Handled | [473](../../crates/crysirror-circus/src/adapter/effect_stage.rs#L473) |
| `condition` | `i64::from(height) * i64::from(operation.zoom) / 100 < 720 && height != 0` | i32::try_from | Handled | [487](../../crates/crysirror-circus/src/adapter/effect_stage.rs#L487) |

### adapter/effects.rs / drifting_spec

```rust
fn drifting_spec(
    kind: i16,
    parameter: i16,
    texture: &str,
    extent: [u32; 2],
    rng: SequenceRng,
    now: u64,
    dc3: bool,
) -> Result<SystemSpec>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `2` | Local state/value operations | Handled | [512](../../crates/crysirror-circus/src/adapter/effects.rs#L512) |
| `kind` | `3` | Local state/value operations | Handled | [513](../../crates/crysirror-circus/src/adapter/effects.rs#L513) |
| `kind` | `14` | Local state/value operations | Handled | [514](../../crates/crysirror-circus/src/adapter/effects.rs#L514) |
| `kind` | `_` | Local state/value operations | Handled | [515](../../crates/crysirror-circus/src/adapter/effects.rs#L515) |

### adapter/effects.rs / Bridge<'_>::falling_effect

```rust
fn falling_effect(&mut self, kind: i16, parameter: i16) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 7 \| 16)` | self.falling_scene_special | Handled | [74](../../crates/crysirror-circus/src/adapter/effects.rs#L74) |
| `condition` | `matches!(kind, 7 \| 16)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [74](../../crates/crysirror-circus/src/adapter/effects.rs#L74) |
| `condition` | `kind==15` | Local state/value operations | Handled | [89](../../crates/crysirror-circus/src/adapter/effects.rs#L89) |
| `condition` | `kind == 0` | self.services.effect_control.clear_pool_controls, self.world | Handled | [90](../../crates/crysirror-circus/src/adapter/effects.rs#L90) |
| `condition; under receiver=kind == 0` | `previous.kind == 0` | self.services.effect_control.clear_pool_controls | Handled | [97](../../crates/crysirror-circus/src/adapter/effects.rs#L97) |
| `kind` | `13` | Local state/value operations | Handled | [163](../../crates/crysirror-circus/src/adapter/effects.rs#L163) |
| `kind` | `15` | Local state/value operations | Handled | [164](../../crates/crysirror-circus/src/adapter/effects.rs#L164) |
| `kind` | `17` | Local state/value operations | Handled | [165](../../crates/crysirror-circus/src/adapter/effects.rs#L165) |
| `kind` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [166](../../crates/crysirror-circus/src/adapter/effects.rs#L166) |
| `condition` | `kind == 2` | Local state/value operations | Handled | [182](../../crates/crysirror-circus/src/adapter/effects.rs#L182) |
| `condition` | `kind == 15` | Local state/value operations | Handled | [183](../../crates/crysirror-circus/src/adapter/effects.rs#L183) |
| `condition; under receiver=else: kind == 15` | `kind == 2` | Local state/value operations | Handled | [185](../../crates/crysirror-circus/src/adapter/effects.rs#L185) |
| `condition` | `kind == 15` | Local state/value operations | Handled | [208](../../crates/crysirror-circus/src/adapter/effects.rs#L208) |
| `condition` | `kind == 15` | self.services.effect_control.orbit_control, self.host.now_ms | Handled | [220](../../crates/crysirror-circus/src/adapter/effects.rs#L220) |
| `condition; under receiver=else: kind == 15` | `matches!(kind, 2 \| 3 \| 13 \| 14 \| 17)` | self.host.now_ms | Handled | [235](../../crates/crysirror-circus/src/adapter/effects.rs#L235) |
| `condition; under receiver=else: kind == 15` | `matches!(kind, 2 \| 3 \| 13 \| 14 \| 17)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [235](../../crates/crysirror-circus/src/adapter/effects.rs#L235) |
| `condition` | `kind != 15 \|\| index == 0` | Local state/value operations | Handled | [280](../../crates/crysirror-circus/src/adapter/effects.rs#L280) |

### adapter/history.rs / Bridge<'_>::native_history

```rust
fn native_history(
        &mut self,
        request: &HistoryRequest,
    ) -> Result<Option<HistoryReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under request=HistoryRequest::PreviewShell` | `!self.services.images.contains_key(&CHIP) && !self.emit(&Event::LoadImage{slot:CHIP,name:"system\\bklogchip".into()})?` | Local state/value operations | Handled | [259](../../crates/crysirror-circus/src/adapter/history.rs#L259) |
| `condition; under request=HistoryRequest::Poll { active, can_open, right_opens, }` | `events .iter() .any(\|event\| event["kind"] == "activate" && event["widget"]["id"] == id)` | Local state/value operations | Handled | [483](../../crates/crysirror-circus/src/adapter/history.rs#L483) |
| `condition; under request=HistoryRequest::Poll { active, can_open, right_opens, }` | `events.iter().any(\|event\| { event["kind"] == "activate" && event["widget"]["id"] == BACK_BUTTON })` | Local state/value operations | Handled | [501](../../crates/crysirror-circus/src/adapter/history.rs#L501) |
| `condition; under request=HistoryRequest::Poll { active, can_open, right_opens, }` | `events .iter() .any(\|event\| event["kind"] == "activate" && event["widget"]["id"] == TOGGLE)` | Local state/value operations | Handled | [506](../../crates/crysirror-circus/src/adapter/history.rs#L506) |
| `condition; under request=HistoryRequest::Poll { active, can_open, right_opens, }` | `let Some(event) = events .iter() .find(\|event\| event["kind"] == "change" && event["widget"]["id"] == SLIDER)` | f64::from | Handled | [512](../../crates/crysirror-circus/src/adapter/history.rs#L512) |
| `condition; under request=HistoryRequest::Poll { active, can_open, right_opens, }` | `events.iter().any(\|value\| value["kind"] == event)` | Local state/value operations | Handled | [532](../../crates/crysirror-circus/src/adapter/history.rs#L532) |
| `condition; under request=HistoryRequest::Poll { active, can_open, right_opens, }` | `action.is_none() && events.iter().any(\|event\| event["kind"] == "history_select")` | HistoryAction::Row | Handled | [537](../../crates/crysirror-circus/src/adapter/history.rs#L537) |

### adapter/images.rs / load_profile_image_origin

```rust
fn load_profile_image_origin(
    storage: &dyn ClientStorage,
    client: ClientId,
    name: &str,
    profile: Option<crate::Profile>,
) -> Result<(ImageSurface, [i32; 2])>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(id) = archive .resolve(basename) .or_else(\|\| archive.resolve(&format!("{basename}.crx")))` | Local state/value operations | Handled | [77](../../crates/crysirror-circus/src/adapter/images.rs#L77) |

### adapter/inline.rs / Bridge<'_>::native_inline_event

```rust
fn native_inline_event(&mut self, event: &InlineEvent) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under event.kind=1..=10` | `event.kind <= 5` | Local state/value operations | Handled | [10](../../crates/crysirror-circus/src/adapter/inline.rs#L10) |
| `condition; under event.kind=1..=10` | `event.kind <= 5` | Local state/value operations | Handled | [15](../../crates/crysirror-circus/src/adapter/inline.rs#L15) |
| `event.kind` | `1..=10` | u8::try_from, self.services.images.get, self.services.image_extents.get, self.services.images.insert, self.services.images.remove | Handled | [8](../../crates/crysirror-circus/src/adapter/inline.rs#L8) |
| `event.kind` | `11` | self.services.inline_music_stopped.as_ref, self.release_music, self.native_music, u16::try_from, self.play_native_music | Handled | [68](../../crates/crysirror-circus/src/adapter/inline.rs#L68) |
| `event.kind` | `12` | self.release_music, u32::try_from | Handled | [84](../../crates/crysirror-circus/src/adapter/inline.rs#L84) |
| `event.kind` | `13` | self.sound_name, u16::try_from | Handled | [88](../../crates/crysirror-circus/src/adapter/inline.rs#L88) |
| `event.kind` | `kind` | Local state/value operations | Unsupported: Circus scheduled native inline effect{kind} requires its extracted engine operation | [103](../../crates/crysirror-circus/src/adapter/inline.rs#L103) |

### adapter/menu.rs / menu_image_animations

```rust
fn menu_image_animations(
    group: &Group,
    images: &BTreeMap<u8, UploadedImage>,
    layers: &mut Vec<Value>,
    base:u32,
    references:&BTreeMap<String,[f32;2]>,
) -> Result<BTreeSet<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(command) = layers .iter_mut() .find(\|v\| v["op"] == "begin_sequence" && v["id"] == id)` | u8::try_from | Handled | [830](../../crates/crysirror-circus/src/adapter/menu.rs#L830) |

### adapter/menu.rs / Bridge<'_>::begin_menu

```rust
fn begin_menu(&mut self, group: &Group) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(layer) = layers.iter_mut().rev().find(\|v\| v["layer"]["id"] == id)` | Local state/value operations | Handled | [298](../../crates/crysirror-circus/src/adapter/menu.rs#L298) |
| `condition` | `let Some(last) = layers.iter_mut().rev().find(\|v\| v["layer"]["id"] == id)` | Local state/value operations | Handled | [302](../../crates/crysirror-circus/src/adapter/menu.rs#L302) |
| `condition` | `let Some(command)=layers.iter_mut().rev().find(\|command\|command["layer"]["id"]==id)` | Local state/value operations | Handled | [355](../../crates/crysirror-circus/src/adapter/menu.rs#L355) |
| `condition` | `let Some(last) = layers.iter_mut().rev().find(\|v\| v["layer"]["id"] == id)` | Local state/value operations | Handled | [362](../../crates/crysirror-circus/src/adapter/menu.rs#L362) |

### adapter/menu.rs / Bridge<'_>::tick_menu_inner

```rust
fn tick_menu_inner(
        &mut self,
        menu: &mut NativeMenu,
        events: &[Value],
        _panel: bool,
    ) -> Result<Option<String>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.play_audio(2, &name, false, false, 0, None)?` | Local state/value operations | Handled | [683](../../crates/crysirror-circus/src/adapter/menu.rs#L683) |
| `condition` | `!self.play_audio(8, name, true, false, 0, None)?` | Local state/value operations | Handled | [699](../../crates/crysirror-circus/src/adapter/menu.rs#L699) |

### adapter/menu_values.rs / Bridge<'_>::poll_native_slider

```rust
fn poll_native_slider(&mut self,menu:&mut NativeMenu,ordinal:usize,status:&Value,events:&[Value]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `event["kind"].as_str()` | `Some("native_decrease"\|"native_decrease_held")` | Local state/value operations | Handled | [95](../../crates/crysirror-circus/src/adapter/menu_values.rs#L95) |
| `event["kind"].as_str()` | `Some("native_increase"\|"native_increase_held")` | i32::from | Handled | [96](../../crates/crysirror-circus/src/adapter/menu_values.rs#L96) |
| `event["kind"].as_str()` | `Some("native_commit")` | Local state/value operations | Handled | [97](../../crates/crysirror-circus/src/adapter/menu_values.rs#L97) |
| `event["kind"].as_str()` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [98](../../crates/crysirror-circus/src/adapter/menu_values.rs#L98) |

### adapter/movie.rs / movie_asset

```rust
fn movie_asset(name: &str) -> Result<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!name.to_ascii_lowercase().ends_with(".mpg")` | Local state/value operations | Handled | [194](../../crates/crysirror-circus/src/adapter/movie.rs#L194) |

### adapter/movie.rs / Bridge<'_>::native_movie

```rust
fn native_movie(&mut self, request: &MovieRequest) -> Result<Option<TextReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under request=MovieRequest::Poll { now: _ }` | `events.iter().any(\|event\| event.kind == "movie_skip") \|\| matches!(status.phase, MediaPhase::Failed \| MediaPhase::Ended \| MediaPhase::Stopped)` | self.submit, NativeMovie::default | Handled | [146](../../crates/crysirror-circus/src/adapter/movie.rs#L146) |

### adapter/packed.rs / open_pack_with_context

```rust
fn open_pack_with_context(storage:&dyn ClientStorage,context:&DecodeContext,name:&str) -> Result<Arc<dyn Archive>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!name.to_ascii_lowercase().ends_with(".pck")` | Local state/value operations | Handled | [36](../../crates/crysirror-circus/src/adapter/packed.rs#L36) |

### adapter/packed.rs / Bridge<'_>::prepare_packed_tick

```rust
fn prepare_packed_tick(&mut self,fast:bool) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `next==playback.shown&&name==playback.name\|\|playback.pending.as_ref().is_some_and(\|pending\|pending.0==next&&pending.1==name)` | Local state/value operations | Handled | [258](../../crates/crysirror-circus/src/adapter/packed.rs#L258) |

### adapter/panel.rs / Bridge<'_>::poll_panel_inner

```rust
fn poll_panel_inner(&mut self, panel: &mut NativePanel) -> Result<Option<PanelReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `key(&["panel_scroll"])` | serde_json::from_slice, EngineQuery::UiInteraction, f64::from, crate::machine::menu_ops::native_ftol, i32::from | Handled | [609](../../crates/crysirror-circus/src/adapter/panel.rs#L609) |
| `condition` | `key(&["panel_page_down"])` | Local state/value operations | Handled | [639](../../crates/crysirror-circus/src/adapter/panel.rs#L639) |
| `condition` | `key(&["panel_page_up"])` | Local state/value operations | Handled | [651](../../crates/crysirror-circus/src/adapter/panel.rs#L651) |
| `condition` | `panel.focus == panel.row_boundary as i32 && key(&["menu_next", "menu_right", "menu_right_held"])` | Local state/value operations | Handled | [668](../../crates/crysirror-circus/src/adapter/panel.rs#L668) |
| `condition` | `old_focus == 0 && key(&["menu_previous", "menu_left", "menu_left_held"])` | Local state/value operations | Handled | [685](../../crates/crysirror-circus/src/adapter/panel.rs#L685) |
| `condition` | `!self.play_audio(2, name, false, false, 0, None)?` | Local state/value operations | Handled | [810](../../crates/crysirror-circus/src/adapter/panel.rs#L810) |
| `condition` | `!self.play_audio(2, &name, false, false, 0, None)?` | Local state/value operations | Handled | [822](../../crates/crysirror-circus/src/adapter/panel.rs#L822) |
| `condition` | `!self.play_audio(8, name, true, false, 0, None)?` | Local state/value operations | Handled | [838](../../crates/crysirror-circus/src/adapter/panel.rs#L838) |

### adapter/panel.rs / Bridge<'_>::publish_panel_bounds

```rust
fn publish_panel_bounds(&mut self, build: &Build) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(command) = layers .last_mut() .filter(\|command\| command["layer"]["id"] == THUMB)` | Local state/value operations | Handled | [337](../../crates/crysirror-circus/src/adapter/panel.rs#L337) |

### adapter/profile.rs / Bridge<'_>::tick_player_input

```rust
fn tick_player_input(&mut self, state: &mut ProfileInput) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.emit(&Event::PlayAudio { channel: if name.eq_ignore_ascii_case("se\\201.pcm") { 7 } else { 6 }, name, voice: false, })?` | Local state/value operations | Handled | [141](../../crates/crysirror-circus/src/adapter/profile.rs#L141) |
| `condition` | `name.eq_ignore_ascii_case("se\\201.pcm")` | Local state/value operations | Handled | [142](../../crates/crysirror-circus/src/adapter/profile.rs#L142) |
| `condition` | `event["kind"] == "dialog_result" && event["widget"]["id"].as_u64() == Some(u64::from(DIALOG))` | Local state/value operations | Handled | [176](../../crates/crysirror-circus/src/adapter/profile.rs#L176) |
| `condition` | `event["kind"] == "activate" && event["widget"]["id"].as_u64() == Some(u64::from(SUBMIT))` | serde_json::from_slice, EngineQuery::UiWidget | Handled | [188](../../crates/crysirror-circus/src/adapter/profile.rs#L188) |

### adapter/resource.rs / Bridge<'_>::native_resource

```rust
fn native_resource(
        &mut self,
        request: &ResourceRequest,
    ) -> Result<Option<ResourceReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under request=ResourceRequest::PreserveTextureGeometry {object,crop,allocation,reference}` | `layer["paint"]["kind"]!="image"` | u32::from, serde_json::from_slice, self.host.query_engine, EngineQuery::Layer | Handled | [184](../../crates/crysirror-circus/src/adapter/resource.rs#L184) |
| `condition; under request=ResourceRequest::PreserveTextureGeometry {object,crop,allocation,reference}` | `layer["paint"]["kind"]=="image"` | crysirror_scene::layer::crop::texel_center_quad, crate::machine::visual_ops::object_resource, super::sprite_resources::alias, self.dc3_image_origin, self.submit | Handled | [188](../../crates/crysirror-circus/src/adapter/resource.rs#L188) |
| `condition; under request=ResourceRequest::PreserveTextureGeometry {object,crop,allocation,reference}` | `id==0x3e00_0000+u32::from(*object)` | self.set_packed_source_crop | Handled | [203](../../crates/crysirror-circus/src/adapter/resource.rs#L203) |

### adapter/save_resources.rs / Bridge<'_>::falling_scene_special

```rust
fn falling_scene_special(&mut self, kind: i16, parameter: i16) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 7` | Local state/value operations | Handled | [458](../../crates/crysirror-circus/src/adapter/save_resources.rs#L458) |
| `condition` | `kind == 16` | self.native_scrolling_pair | Handled | [461](../../crates/crysirror-circus/src/adapter/save_resources.rs#L461) |

### adapter/save_resources.rs / Bridge<'_>::finish_native_resource_restore

```rust
fn finish_native_resource_restore(&mut self, plan: &RestorePlan) -> Result<Option<SaveReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.native_packed(&PackedRequest::Load { slot: 33, name: name.to_owned(), }).with_context(\|\|format!("Circus425640 packed reload slot={} path=Advdata/GRP/{}",image.slot,name.replace('\\',"/")))?` | Local state/value operations | Handled | [590](../../crates/crysirror-circus/src/adapter/save_resources.rs#L590) |
| `condition` | `audio.kind == 0` | self.services.music_loops.is_none, self.services.prepare, self.play_audio_policy, self.services.saves.restore.as_mut | Handled | [609](../../crates/crysirror-circus/src/adapter/save_resources.rs#L609) |
| `condition; under receiver=audio.kind == 0` | `!name.ends_with(".PCM")` | Local state/value operations | Handled | [625](../../crates/crysirror-circus/src/adapter/save_resources.rs#L625) |
| `condition; under receiver=audio.kind == 0` | `!self.play_audio_policy( 0, &audio.name, false, true, bounds[0], (bounds[1] != 0).then_some(bounds[1]), false, ).with_context(\|\|format!("Circus425640 music reload path=Advdata/{}.PCM",audio.name.replace('\\',"/")))?` | Local state/value operations | Handled | [636](../../crates/crysirror-circus/src/adapter/save_resources.rs#L636) |
| `condition; under receiver=else: audio.kind == 0` | `audio.kind == 1` | Local state/value operations | Handled | [661](../../crates/crysirror-circus/src/adapter/save_resources.rs#L661) |

### adapter/save_resources.rs / Bridge<'_>::restore_native_resources

```rust
fn restore_native_resources(
        &mut self,
        plan: &RestorePlan,
    ) -> Result<Option<SaveReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 4` | Local state/value operations | Handled | [334](../../crates/crysirror-circus/src/adapter/save_resources.rs#L334) |
| `condition` | `mode == 4` | Local state/value operations | Handled | [432](../../crates/crysirror-circus/src/adapter/save_resources.rs#L432) |

### adapter/sprite.rs / Bridge<'_>::native_sprite

```rust
fn native_sprite(&mut self, request: &SpriteRequest) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `object<150&&command.operations.iter().any(\|operation\|matches!(operation,SequenceOperation::Reset{..}))` | self.services.visual.filters.insert, self.services.primary.resources.remove, sprite_resources::alias, serde_json::from_slice, self.host.query_engine | Handled | [151](../../crates/crysirror-circus/src/adapter/sprite.rs#L151) |
| `condition` | `object<150&&matches!(operation,SequenceOperation::Reset{..})` | Local state/value operations | Handled | [164](../../crates/crysirror-circus/src/adapter/sprite.rs#L164) |

### adapter/sprite_masks.rs / Bridge<'_>::mask_target

```rust
fn mask_target(&mut self, object: u16) -> Result<Option<UploadedImage>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `layer["paint"]["kind"] != "image"` | serde_json::from_slice, self.host.query_engine, EngineQuery::Layer, u32::from | Handled | [233](../../crates/crysirror-circus/src/adapter/sprite_masks.rs#L233) |
| `condition` | `layer["paint"]["kind"] != "image"` | Local state/value operations | Handled | [241](../../crates/crysirror-circus/src/adapter/sprite_masks.rs#L241) |

### adapter/sprite_masks.rs / Bridge<'_>::mirror_sprite_mask_operations

```rust
fn mirror_sprite_mask_operations(&mut self, request: &SpriteRequest) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `operation` | `SequenceOperation::Capture { enabled: true, .. }` | Local state/value operations | Handled | [167](../../crates/crysirror-circus/src/adapter/sprite_masks.rs#L167) |
| `operation` | `SequenceOperation::Finish { lanes, .. } if lanes.as_ref().is_none_or(\|lanes\| lanes.contains(&7))` | Local state/value operations | Handled | [168](../../crates/crysirror-circus/src/adapter/sprite_masks.rs#L168) |
| `operation` | `SequenceOperation::Reset { .. }` | Local state/value operations | Handled | [173](../../crates/crysirror-circus/src/adapter/sprite_masks.rs#L173) |
| `operation` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [177](../../crates/crysirror-circus/src/adapter/sprite_masks.rs#L177) |
| `condition` | `matches!(op, SequenceOperation::Replay)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [183](../../crates/crysirror-circus/src/adapter/sprite_masks.rs#L183) |

### adapter/sprite_masks.rs / Bridge<'_>::prepare_forced_sprite_masks

```rust
fn prepare_forced_sprite_masks(
        &mut self,
        request: &SpriteRequest,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!command.operations.iter().any(\|operation\|matches!(operation,SequenceOperation::Finish{cancel:false,lanes} if lanes.as_ref().is_none_or(\|lanes\|lanes.contains(&7))))` | Local state/value operations | Handled | [605](../../crates/crysirror-circus/src/adapter/sprite_masks.rs#L605) |

### adapter/system.rs / Bridge<'_>::native_system

```rust
fn native_system(&mut self, request: SystemRequest) -> Result<Option<SystemReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under request=SystemRequest::NativeFullscreenMode(mode)` | `mode != 0 && mode != self.services.system.native_fullscreen_mode` | Local state/value operations | Handled | [1001](../../crates/crysirror-circus/src/adapter/system.rs#L1001) |
| `condition; under request=SystemRequest::Panel { mode, .. } if mode != 1 && mode != 6` | `now != started && events.iter().any(\|event\| event.kind == "blank_advance")` | self.submit | Handled | [1268](../../crates/crysirror-circus/src/adapter/system.rs#L1268) |
| `condition; under request=SystemRequest::CharacterPreview { name, position, auxiliary, scale, }` | `!self.native_sprite(&crate::machine::sprite_ops::SpriteRequest{commands:vec![crate::machine::sprite_ops::SpriteCommand{ seed:crate::native_live::NativeLiveState::sprite_reset_seed(149)?, operations:vec![crysirror_scene::layer::queued::SequenceOperation::Set{ channels:vec![0,1,3,4,12,13,14,15],values:vec![position[0] as f32,position[1] as f32,f32::from(scale),f32::from(scale),auxiliary[0] as f32,auxiliary[1] as f32,logical[0] as f32,logical[1] as f32],retained:false, }], }]})?` | Local state/value operations | Handled | [1306](../../crates/crysirror-circus/src/adapter/system.rs#L1306) |

### adapter/system.rs / Bridge<'_>::system_metadata

```rust
fn system_metadata(&mut self) -> Result<Option<NativeMetadata>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == -1` | Local state/value operations | Handled | [734](../../crates/crysirror-circus/src/adapter/system.rs#L734) |
| `condition` | `key == 0` | Local state/value operations | Handled | [780](../../crates/crysirror-circus/src/adapter/system.rs#L780) |

### adapter/system_cloud.rs / Bridge<'_>::tick_account

```rust
fn tick_account(&mut self, job: &mut AccountJob) -> Result<Option<SystemReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under job.phase=Phase::Ready` | `job.operation == AccountOperation::ClearReport && ["CLEARNAME", "CLEARMAIL"] .iter() .any(\|key\| job.profile[*key].as_str().is_none_or(str::is_empty))` | Local state/value operations | Handled | [392](../../crates/crysirror-circus/src/adapter/system_cloud.rs#L392) |
| `condition; under job.phase=Phase::Credentials` | `event["kind"] == "cancel"` | Local state/value operations | Handled | [577](../../crates/crysirror-circus/src/adapter/system_cloud.rs#L577) |
| `condition; under job.phase=Phase::Credentials` | `event["kind"] == "submit"` | Local state/value operations | Handled | [579](../../crates/crysirror-circus/src/adapter/system_cloud.rs#L579) |
| `condition; under job.phase=Phase::Credentials` | `event["kind"] == "activate"` | u32::try_from | Handled | [581](../../crates/crysirror-circus/src/adapter/system_cloud.rs#L581) |

### adapter/system_gestures.rs / Bridge<'_>::gesture_overlay

```rust
fn gesture_overlay(&mut self,group:Option<&Group>,position:[i32;2]) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(command)=layers.iter_mut().rev().find(\|command\|command["layer"]["id"]==id)` | Local state/value operations | Handled | [52](../../crates/crysirror-circus/src/adapter/system_gestures.rs#L52) |
| `condition` | `let Some(command)=layers.iter_mut().rev().find(\|command\|command["layer"]["id"]==id)` | Local state/value operations | Handled | [59](../../crates/crysirror-circus/src/adapter/system_gestures.rs#L59) |

### adapter/system_gestures.rs / NativeGestures::observe_stock

```rust
fn observe_stock(&mut self,down:u64,pressed:u64,time_ms:u64,enabled:bool) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!enabled\|\|down&key==0` | Local state/value operations | Handled | [26](../../crates/crysirror-circus/src/adapter/system_gestures.rs#L26) |
| `condition` | `since.is_none()\|\|pressed&key!=0` | Local state/value operations | Handled | [27](../../crates/crysirror-circus/src/adapter/system_gestures.rs#L27) |

### adapter/system_license.rs / Bridge<'_>::tick_native_license

```rust
fn tick_native_license(&mut self, state: &mut LicenseDialog) -> Result<Option<SystemReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `event["widget"]["id"].as_u64() == Some(u64::from(FIELD))` | u64::from | Handled | [181](../../crates/crysirror-circus/src/adapter/system_license.rs#L181) |
| `event["kind"].as_str()` | `Some("submit")` | u64::from | Handled | [183](../../crates/crysirror-circus/src/adapter/system_license.rs#L183) |
| `event["kind"].as_str()` | `Some("cancel")` | u64::from | Handled | [184](../../crates/crysirror-circus/src/adapter/system_license.rs#L184) |
| `event["kind"].as_str()` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [185](../../crates/crysirror-circus/src/adapter/system_license.rs#L185) |
| `condition` | `event["kind"] == "activate"` | Local state/value operations | Handled | [187](../../crates/crysirror-circus/src/adapter/system_license.rs#L187) |

### adapter/system_selection.rs / Bridge<'_>::rebuild_native_selection

```rust
fn rebuild_native_selection(
        &mut self,
        mode: u8,
        page: i32,
        declarations: &PanelDeclarations,
    ) -> Result<Option<SystemReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(mode, 3 \| 4)` | SystemReply::Selection | Handled | [60](../../crates/crysirror-circus/src/adapter/system_selection.rs#L60) |
| `condition` | `mode == 3 \|\| body.is_some()` | Local state/value operations | Handled | [145](../../crates/crysirror-circus/src/adapter/system_selection.rs#L145) |
| `condition` | `mode == 4 && missing` | Local state/value operations | Handled | [160](../../crates/crysirror-circus/src/adapter/system_selection.rs#L160) |
| `condition` | `state == 99 \|\| mode == 4 && missing` | Local state/value operations | Handled | [172](../../crates/crysirror-circus/src/adapter/system_selection.rs#L172) |

### adapter/text.rs / scheduled_control

```rust
fn scheduled_control(text:&str) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `["@t","@cmd","@d","@muf","@mu","@lse","@se","@ht","@hy","@re1"].iter().any(\|prefix\|text.starts_with(prefix))` | Local state/value operations | Handled | [114](../../crates/crysirror-circus/src/adapter/text.rs#L114) |
| `condition` | `matches!(selector,b'c'\|b'f')` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [118](../../crates/crysirror-circus/src/adapter/text.rs#L118) |

### adapter/text.rs / Bridge<'_>::finish_text_motion

```rust
fn finish_text_motion(&mut self, release44: bool) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `layer["paint"]["kind"] == "image" && layer["paint"]["asset"] == image.path.relative()` | Local state/value operations | Handled | [719](../../crates/crysirror-circus/src/adapter/text.rs#L719) |

### adapter/text.rs / Bridge<'_>::native_text

```rust
fn native_text(&mut self, request: &TextRequest) -> Result<Option<TextReply>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under request=TextRequest::ImmediateImage(part)` | `let Some(layer) = layers.iter_mut().rev().find(\|command\|command["layer"]["id"]==WINDOW)` | Local state/value operations | Handled | [461](../../crates/crysirror-circus/src/adapter/text.rs#L461) |
| `condition; under request=TextRequest::Dialogue { speaker, body, body_template, name_template, body_group, name_group, speed, }` | `let Some(layer) = layers.iter_mut().rev().find(\|value\| value["layer"]["id"] == id)` | Local state/value operations | Handled | [575](../../crates/crysirror-circus/src/adapter/text.rs#L575) |

### adapter/text_stream.rs / Bridge<'_>::advance_text_streams

```rust
fn advance_text_streams(&mut self) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `paused.iter().any(\|base\|id>=*base&&id<base.saturating_add(0x1_0000))` | Local state/value operations | Handled | [111](../../crates/crysirror-circus/src/adapter/text_stream.rs#L111) |

### dc1_frontend.rs / image_path

```rust
fn image_path(name: String) -> Result<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.to_ascii_lowercase().ends_with(".crx")` | Local state/value operations | Handled | [164](../../crates/crysirror-circus/src/dc1_frontend.rs#L164) |

### dc1_story.rs / Story::after_dispatch

```rust
fn after_dispatch(&mut self, op: u8) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under op=4 \| 6 \| 8 \| 9 \| 10` | `op == 9` | Local state/value operations | Handled | [1885](../../crates/crysirror-circus/src/dc1_story.rs#L1885) |
| `condition; under op=4 \| 6 \| 8 \| 9 \| 10` | `op == 10` | Local state/value operations | Handled | [1896](../../crates/crysirror-circus/src/dc1_story.rs#L1896) |
| `condition; under op=4 \| 6 \| 8 \| 9 \| 10` | `op == 6` | Local state/value operations | Handled | [1900](../../crates/crysirror-circus/src/dc1_story.rs#L1900) |
| `condition; under op=69 \| 72` | `op == 69` | Local state/value operations | Handled | [2038](../../crates/crysirror-circus/src/dc1_story.rs#L2038) |
| `condition; under op=70 \| 71 \| 73 \| 74` | `op >= 73` | Local state/value operations | Handled | [2042](../../crates/crysirror-circus/src/dc1_story.rs#L2042) |
| `op` | `4 \| 6 \| 8 \| 9 \| 10` | std::mem::take, self.substitute, self.history.len, self.name.clone, self.pending.iter | Handled | [1884](../../crates/crysirror-circus/src/dc1_story.rs#L1884) |
| `op` | `5` | self.new_page | Handled | [1920](../../crates/crysirror-circus/src/dc1_story.rs#L1920) |
| `op` | `7` | Local state/value operations | Handled | [1923](../../crates/crysirror-circus/src/dc1_story.rs#L1923) |
| `op` | `12` | Local state/value operations | Handled | [1928](../../crates/crysirror-circus/src/dc1_story.rs#L1928) |
| `op` | `13` | Local state/value operations | Handled | [1929](../../crates/crysirror-circus/src/dc1_story.rs#L1929) |
| `op` | `27` | self.name_glyphs.clear | Handled | [1930](../../crates/crysirror-circus/src/dc1_story.rs#L1930) |
| `op` | `37` | Local state/value operations | Handled | [1936](../../crates/crysirror-circus/src/dc1_story.rs#L1936) |
| `op` | `38` | Local state/value operations | Handled | [1940](../../crates/crysirror-circus/src/dc1_story.rs#L1940) |
| `op` | `39` | Local state/value operations | Handled | [1941](../../crates/crysirror-circus/src/dc1_story.rs#L1941) |
| `op` | `34` | Local state/value operations | Handled | [1945](../../crates/crysirror-circus/src/dc1_story.rs#L1945) |
| `op` | `35` | Local state/value operations | Handled | [1946](../../crates/crysirror-circus/src/dc1_story.rs#L1946) |
| `op` | `36` | Local state/value operations | Handled | [1947](../../crates/crysirror-circus/src/dc1_story.rs#L1947) |
| `op` | `40` | Local state/value operations | Handled | [1950](../../crates/crysirror-circus/src/dc1_story.rs#L1950) |
| `op` | `41` | Local state/value operations | Handled | [1955](../../crates/crysirror-circus/src/dc1_story.rs#L1955) |
| `op` | `42` | Local state/value operations | Handled | [1962](../../crates/crysirror-circus/src/dc1_story.rs#L1962) |
| `op` | `46` | Local state/value operations | Handled | [1972](../../crates/crysirror-circus/src/dc1_story.rs#L1972) |
| `op` | `48 \| 52` | self.yield_sprite.take, self.hide_for_sprite, self.effect_pending.take | Handled | [1978](../../crates/crysirror-circus/src/dc1_story.rs#L1978) |
| `op` | `49` | self.hide_for_sprite | Handled | [1995](../../crates/crysirror-circus/src/dc1_story.rs#L1995) |
| `op` | `50` | Local state/value operations | Handled | [2008](../../crates/crysirror-circus/src/dc1_story.rs#L2008) |
| `op` | `51` | self.hide_for_sprite | Handled | [2015](../../crates/crysirror-circus/src/dc1_story.rs#L2015) |
| `op` | `53 \| 59` | Local state/value operations | Empty handler: Original no-op status not established | [2029](../../crates/crysirror-circus/src/dc1_story.rs#L2029) |
| `op` | `67` | self.begin_transition | Handled | [2030](../../crates/crysirror-circus/src/dc1_story.rs#L2030) |
| `op` | `69 \| 72` | self.overlay_name.clone | Handled | [2036](../../crates/crysirror-circus/src/dc1_story.rs#L2036) |
| `op` | `70 \| 71 \| 73 \| 74` | i32::from | Handled | [2041](../../crates/crysirror-circus/src/dc1_story.rs#L2041) |
| `op` | `80` | self.show_window, std::mem::take, self.draw_line | Handled | [2045](../../crates/crysirror-circus/src/dc1_story.rs#L2045) |
| `op` | `92` | self.timer | Handled | [2062](../../crates/crysirror-circus/src/dc1_story.rs#L2062) |
| `op` | `93` | Local state/value operations | Empty handler: Original no-op status not established | [2092](../../crates/crysirror-circus/src/dc1_story.rs#L2092) |
| `op` | `115 \| 114` | Local state/value operations | Handled | [2093](../../crates/crysirror-circus/src/dc1_story.rs#L2093) |
| `op` | `123` | Local state/value operations | Handled | [2096](../../crates/crysirror-circus/src/dc1_story.rs#L2096) |
| `op` | `124` | Local state/value operations | Handled | [2101](../../crates/crysirror-circus/src/dc1_story.rs#L2101) |
| `op` | `other` | Local state/value operations | Unsupported: DC1 408F70 yielded opcode {other} is not ported | [2106](../../crates/crysirror-circus/src/dc1_story.rs#L2106) |

### dc1_story.rs / Story::execute

```rust
fn execute(&mut self, record: &Record) -> Result<Exec>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under record.opcode=24 \| 25` | `record.opcode == 24` | Local state/value operations | Handled | [1499](../../crates/crysirror-circus/src/dc1_story.rs#L1499) |
| `condition; under record.opcode=76 \| 77` | `record.opcode == 77` | Local state/value operations | Unsupported: DC1 404600 opcode 77 (positioned choice) is not ported | [1695](../../crates/crysirror-circus/src/dc1_story.rs#L1695) |
| `condition; under record.opcode=107 \| 108` | `record.opcode == 107` | Local state/value operations | Handled | [1786](../../crates/crysirror-circus/src/dc1_story.rs#L1786) |
| `condition; under record.opcode=109 \| 110` | `record.opcode == 109` | Local state/value operations | Handled | [1800](../../crates/crysirror-circus/src/dc1_story.rs#L1800) |
| `condition; under record.opcode=125 \| 126` | `record.opcode == 125` | Local state/value operations | Handled | [1838](../../crates/crysirror-circus/src/dc1_story.rs#L1838) |
| `record.opcode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [1843](../../crates/crysirror-circus/src/dc1_story.rs#L1843) |

### dc1_story.rs / Story::frame

```rust
fn frame(&mut self, input: Input) -> Result<Step>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under self.day_state=2 \| 4` | `self.mode != -1` | Local state/value operations | Handled | [555](../../crates/crysirror-circus/src/dc1_story.rs#L555) |
| `condition; under self.day_state=14` | `self.mode == -1` | Local state/value operations | Handled | [601](../../crates/crysirror-circus/src/dc1_story.rs#L601) |

### dc1_story.rs / Story::overlay_slide

```rust
fn overlay_slide(&mut self, click: bool) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode == 8` | Local state/value operations | Handled | [2199](../../crates/crysirror-circus/src/dc1_story.rs#L2199) |
| `condition` | `self.mode == 8` | Local state/value operations | Handled | [2206](../../crates/crysirror-circus/src/dc1_story.rs#L2206) |

### dc1_story.rs / Story::reveal

```rust
fn reveal(&mut self) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.mode != 18` | Local state/value operations | Handled | [1217](../../crates/crysirror-circus/src/dc1_story.rs#L1217) |

### dc1_story.rs / Story::scroll_step

```rust
fn scroll_step(&mut self, click: bool) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.mode` | `4` | Local state/value operations | Handled | [2177](../../crates/crysirror-circus/src/dc1_story.rs#L2177) |
| `self.mode` | `5` | Local state/value operations | Handled | [2178](../../crates/crysirror-circus/src/dc1_story.rs#L2178) |
| `self.mode` | `6` | Local state/value operations | Handled | [2179](../../crates/crysirror-circus/src/dc1_story.rs#L2179) |
| `self.mode` | `_` | Local state/value operations | Handled | [2180](../../crates/crysirror-circus/src/dc1_story.rs#L2180) |

### dc1_story.rs / Story::tick

```rust
fn tick(&mut self, input: Input) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `input.cancel && (matches!(self.mode, 1 \| 15 \| 25) \|\| (self.mode == 17 && self.eyecatch.is_none()))` | SystemMenu::default | Handled | [733](../../crates/crysirror-circus/src/dc1_story.rs#L733) |
| `condition` | `matches!(self.mode, 1 \| 15 \| 17 \| 18) && self.window && self.eyecatch.is_none()` | self.tables.window_buttons.iter | Handled | [746](../../crates/crysirror-circus/src/dc1_story.rs#L746) |
| `condition` | `hide && matches!(self.mode, 1 \| 15 \| 16 \| 17 \| 25) && self.wipe.is_none()` | Local state/value operations | Handled | [793](../../crates/crysirror-circus/src/dc1_story.rs#L793) |
| `condition; under self.mode=24` | `self.mode != 24` | Local state/value operations | Handled | [978](../../crates/crysirror-circus/src/dc1_story.rs#L978) |
| `self.mode` | `0` | self.dispatch, self.after_dispatch | Handled | [802](../../crates/crysirror-circus/src/dc1_story.rs#L802) |
| `self.mode` | `1` | self.choose | Handled | [808](../../crates/crysirror-circus/src/dc1_story.rs#L808) |
| `self.mode` | `4..=7` | self.scroll_step | Handled | [809](../../crates/crysirror-circus/src/dc1_story.rs#L809) |
| `self.mode` | `8 \| 9` | self.overlay_slide | Handled | [810](../../crates/crysirror-circus/src/dc1_story.rs#L810) |
| `self.mode` | `10` | self.wipe_step | Handled | [811](../../crates/crysirror-circus/src/dc1_story.rs#L811) |
| `self.mode` | `11` | Local state/value operations | Handled | [812](../../crates/crysirror-circus/src/dc1_story.rs#L812) |
| `self.mode` | `19` | self.open_backlog | Handled | [818](../../crates/crysirror-circus/src/dc1_story.rs#L818) |
| `self.mode` | `20` | self.backlog_step | Handled | [819](../../crates/crysirror-circus/src/dc1_story.rs#L819) |
| `self.mode` | `26` | self.menu_step | Handled | [820](../../crates/crysirror-circus/src/dc1_story.rs#L820) |
| `self.mode` | `27` | self.option.take, SystemMenu::default, self.events.push | Partial: DC1 41E180 player-name entry is not ported | [821](../../crates/crysirror-circus/src/dc1_story.rs#L821) |
| `self.mode` | `31 \| 32` | self.dialog.as_mut, SystemMenu::default | Partial: DC1 408F70 state31 window-size change is not ported | [857](../../crates/crysirror-circus/src/dc1_story.rs#L857) |
| `self.mode` | `-2` | Local state/value operations | Empty handler: Original no-op status not established | [874](../../crates/crysirror-circus/src/dc1_story.rs#L874) |
| `self.mode` | `14` | Local state/value operations | Handled | [875](../../crates/crysirror-circus/src/dc1_story.rs#L875) |
| `self.mode` | `15` | self.show_window, self.draw_name, self.now.saturating_sub, self.reveal | Handled | [882](../../crates/crysirror-circus/src/dc1_story.rs#L882) |
| `self.mode` | `16` | self.animate_cursor | Handled | [908](../../crates/crysirror-circus/src/dc1_story.rs#L908) |
| `self.mode` | `17` | self.now.saturating_sub, self.eyecatch.is_some, self.timer, self.new_page, self.animate_cursor | Handled | [927](../../crates/crysirror-circus/src/dc1_story.rs#L927) |
| `self.mode` | `18` | self.now.saturating_sub | Handled | [960](../../crates/crysirror-circus/src/dc1_story.rs#L960) |
| `self.mode` | `21 \| 22` | Local state/value operations | Handled | [966](../../crates/crysirror-circus/src/dc1_story.rs#L966) |
| `self.mode` | `23` | self.shake_step | Handled | [971](../../crates/crysirror-circus/src/dc1_story.rs#L971) |
| `self.mode` | `24` | self.reveal | Handled | [972](../../crates/crysirror-circus/src/dc1_story.rs#L972) |
| `self.mode` | `25` | self.animate_cursor | Handled | [983](../../crates/crysirror-circus/src/dc1_story.rs#L983) |
| `self.mode` | `-1` | Local state/value operations | Empty handler: Original no-op status not established | [992](../../crates/crysirror-circus/src/dc1_story.rs#L992) |
| `self.mode` | `other` | Local state/value operations | Unsupported: DC1 408F70 presentation mode {other} is not ported | [993](../../crates/crysirror-circus/src/dc1_story.rs#L993) |
| `condition` | `matches!(self.mode, 0 \| 11 \| 15 \| 17 \| 18)` | self.place_step | Handled | [996](../../crates/crysirror-circus/src/dc1_story.rs#L996) |

### dc1_system.rs / SystemData::parse

```rust
fn parse(bytes: &[u8]) -> Result<Self>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `1` | Local state/value operations | Handled | [58](../../crates/crysirror-circus/src/dc1_system.rs#L58) |
| `opcode` | `2` | Local state/value operations | Handled | [72](../../crates/crysirror-circus/src/dc1_system.rs#L72) |
| `opcode` | `3` | Local state/value operations | Handled | [82](../../crates/crysirror-circus/src/dc1_system.rs#L82) |
| `opcode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [83](../../crates/crysirror-circus/src/dc1_system.rs#L83) |

### dc2.rs / named_label

```rust
fn named_label(program: &Program, name: &[u8]) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode == 0x35 && program.literal_bytes(pc) == Some(name)` | Local state/value operations | Handled | [744](../../crates/crysirror-circus/src/dc2.rs#L744) |

### dc2.rs / native_noop

```rust
fn native_noop(opcode: u8) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( opcode, 1 \| 2 \| 0x1f \| 0x24 \| 0x28 \| 0x29 \| 0x2f \| 0x35 \| 0x45 \| 0x4a \| 0x4d \| 0x4e \| 0x52 )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [223](../../crates/crysirror-circus/src/dc2.rs#L223) |

### dc2.rs / skip_marker

```rust
fn skip_marker(program: &Program, mut pc: u32, tag: u8) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode < 41` | Local state/value operations | Handled | [760](../../crates/crysirror-circus/src/dc2.rs#L760) |
| `condition` | `opcode >= 78` | Local state/value operations | Handled | [762](../../crates/crysirror-circus/src/dc2.rs#L762) |
| `condition` | `opcode < 47` | Local state/value operations | Handled | [766](../../crates/crysirror-circus/src/dc2.rs#L766) |
| `condition` | `opcode == 0x1e && program.native_byte(next - 1) == Some(tag)` | Local state/value operations | Handled | [788](../../crates/crysirror-circus/src/dc2.rs#L788) |

### dc2.rs / Machine::execute

```rust
fn execute(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(outcome) = self.execute_scene(instruction, a[0], text, host)?` | Local state/value operations | Handled | [340](../../crates/crysirror-circus/src/dc2.rs#L340) |
| `condition` | `matches!(instruction.opcode, 0x21 \| 0x53 \| 0x60 \| 0x61)` | self.state.draw.clone, Vec::with_capacity, self.value, self.state.settings.prefix, self.advance | Handled | [400](../../crates/crysirror-circus/src/dc2.rs#L400) |
| `condition` | `instruction.opcode == 0x21 && a[0] == 1` | Vec::with_capacity, self.value, self.state.settings.prefix | Handled | [406](../../crates/crysirror-circus/src/dc2.rs#L406) |
| `condition; under instruction.opcode=0x26` | `a[0] == 0 && a[1] == 0 && !self.load_native5("-", host, control)?` | Local state/value operations | Handled | [465](../../crates/crysirror-circus/src/dc2.rs#L465) |
| `condition; under instruction.opcode=0x26` | `!self.start_weather(i32::from(a[0]), i32::from(a[1]), host, now)?` | Local state/value operations | Handled | [468](../../crates/crysirror-circus/src/dc2.rs#L468) |
| `condition; under instruction.opcode=0x16 if a[0] == 8` | `(previous != 0) == (a[1] != 0)` | self.advance | Handled | [505](../../crates/crysirror-circus/src/dc2.rs#L505) |
| `condition; under instruction.opcode=0x22` | `a[0] == 0` | Local state/value operations | Handled | [558](../../crates/crysirror-circus/src/dc2.rs#L558) |
| `condition; under instruction.opcode=0x37 \| 0x47` | `instruction.opcode == 0x37` | Local state/value operations | Handled | [573](../../crates/crysirror-circus/src/dc2.rs#L573) |
| `condition; under instruction.opcode=0x5c / a[3]=1 \| 2` | `a[3] == 1` | Local state/value operations | Handled | [602](../../crates/crysirror-circus/src/dc2.rs#L602) |
| `a[3]; under instruction.opcode=0x5c` | `0` | i32::from | Handled | [597](../../crates/crysirror-circus/src/dc2.rs#L597) |
| `a[3]; under instruction.opcode=0x5c` | `1 \| 2` | self.value | Handled | [598](../../crates/crysirror-circus/src/dc2.rs#L598) |
| `a[3]; under instruction.opcode=0x5c` | `_` | self.advance | Handled | [608](../../crates/crysirror-circus/src/dc2.rs#L608) |
| `condition; under instruction.opcode=0x5c` | `!self.assign(a[0], a[1], value, host)?` | Local state/value operations | Handled | [610](../../crates/crysirror-circus/src/dc2.rs#L610) |
| `condition; under instruction.opcode=0x5d \| 0x5e` | `instruction.opcode == 0x5e` | self.value | Handled | [618](../../crates/crysirror-circus/src/dc2.rs#L618) |
| `condition; under instruction.opcode=0x5d \| 0x5e` | `!self.assign(a[0], a[1], value, host)?` | Local state/value operations | Handled | [626](../../crates/crysirror-circus/src/dc2.rs#L626) |
| `condition; under instruction.opcode=0x5f` | `(a[3] as u16) & 0x80 != 0` | i32::from | Handled | [634](../../crates/crysirror-circus/src/dc2.rs#L634) |
| `condition; under instruction.opcode=0x5f` | `(a[3] as u16) & 0x80 != 0` | self.value, i16::from | Handled | [639](../../crates/crysirror-circus/src/dc2.rs#L639) |
| `mode; under instruction.opcode=0x5f` | `0` | i32::from | Handled | [649](../../crates/crysirror-circus/src/dc2.rs#L649) |
| `mode; under instruction.opcode=0x5f` | `1` | i32::from | Handled | [650](../../crates/crysirror-circus/src/dc2.rs#L650) |
| `mode; under instruction.opcode=0x5f` | `2` | i32::from | Handled | [651](../../crates/crysirror-circus/src/dc2.rs#L651) |
| `mode; under instruction.opcode=0x5f` | `3` | i32::from | Handled | [652](../../crates/crysirror-circus/src/dc2.rs#L652) |
| `mode; under instruction.opcode=0x5f` | `4` | i32::from | Handled | [653](../../crates/crysirror-circus/src/dc2.rs#L653) |
| `mode; under instruction.opcode=0x5f` | `5` | i32::from | Handled | [654](../../crates/crysirror-circus/src/dc2.rs#L654) |
| `mode; under instruction.opcode=0x5f` | `_` | Local state/value operations | Handled | [655](../../crates/crysirror-circus/src/dc2.rs#L655) |
| `condition; under instruction.opcode=0x64` | `(0..5).contains(&a[2])` | Local state/value operations | Handled | [668](../../crates/crysirror-circus/src/dc2.rs#L668) |
| `condition; under instruction.opcode=0x64` | `a[2] == 255` | self.state.primary_names.swap, self.state.primary_ids.swap | Partial: DC2 0x64 native1/2 bitmap swap with loaded portraits is not connected | [673](../../crates/crysirror-circus/src/dc2.rs#L673) |
| `condition; under instruction.opcode=0x65 / a[3]=255` | `a[0] != -1` | Local state/value operations | Handled | [689](../../crates/crysirror-circus/src/dc2.rs#L689) |
| `condition; under instruction.opcode=0x65 / a[3]=255` | `a[1] != -1` | Local state/value operations | Handled | [692](../../crates/crysirror-circus/src/dc2.rs#L692) |
| `a[3]; under instruction.opcode=0x65` | `255` | Local state/value operations | Handled | [688](../../crates/crysirror-circus/src/dc2.rs#L688) |
| `a[3]; under instruction.opcode=0x65` | `16..=19` | Local state/value operations | Handled | [696](../../crates/crysirror-circus/src/dc2.rs#L696) |
| `a[3]; under instruction.opcode=0x65` | `20..` | Local state/value operations | Empty handler: Original no-op status not established | [701](../../crates/crysirror-circus/src/dc2.rs#L701) |
| `a[3]; under instruction.opcode=0x65` | `5..=15` | Local state/value operations | Unsupported: DC2 0x65 selector {} writes outside the pose table (B0B4BC alias) | [702](../../crates/crysirror-circus/src/dc2.rs#L702) |
| `a[3]; under instruction.opcode=0x65` | `_` | Local state/value operations | Unsupported: DC2 0x65 negative selector {} | [703](../../crates/crysirror-circus/src/dc2.rs#L703) |
| `condition; under instruction.opcode=0x66` | `(0..64).contains(&a[0])` | host.emit, self.state.palette.insert | Handled | [706](../../crates/crysirror-circus/src/dc2.rs#L706) |
| `instruction.opcode` | `0x26 if a[0] == 255` | Local state/value operations | Handled | [449](../../crates/crysirror-circus/src/dc2.rs#L449) |
| `instruction.opcode` | `0x26 if a[0] == 7` | self.load_native5 | Handled | [455](../../crates/crysirror-circus/src/dc2.rs#L455) |
| `instruction.opcode` | `0x16 if a[0] == 5` | host.write_profile_system, self.state.settings.bytes | Handled | [480](../../crates/crysirror-circus/src/dc2.rs#L480) |
| `instruction.opcode` | `0x16 if a[0] == 6` | Local state/value operations | Handled | [488](../../crates/crysirror-circus/src/dc2.rs#L488) |
| `instruction.opcode` | `0x16 if matches!(a[0], 7 \| 12 \| 13)` | self.state.runtime.insert, i32::from | Handled | [492](../../crates/crysirror-circus/src/dc2.rs#L492) |
| `instruction.opcode` | `0x16 if a[0] == 8` | self.state.runtime.get, self.value, self.advance, host.system, SystemRequest::NativeFullscreenMode | Handled | [497](../../crates/crysirror-circus/src/dc2.rs#L497) |
| `instruction.opcode` | `0x16 if a[0] == 10` | host.system, self.advance | Handled | [520](../../crates/crysirror-circus/src/dc2.rs#L520) |
| `instruction.opcode` | `0x16 if !matches!(a[0], 0..=13 \| 17)` | Local state/value operations | Empty handler: Original no-op status not established | [533](../../crates/crysirror-circus/src/dc2.rs#L533) |
| `instruction.opcode` | `0x1e if a[0] == 0` | Local state/value operations | Handled | [555](../../crates/crysirror-circus/src/dc2.rs#L555) |
| `instruction.opcode` | `op if native_noop(op)` | Local state/value operations | Empty handler: Original no-op status not established | [715](../../crates/crysirror-circus/src/dc2.rs#L715) |
| `instruction.opcode` | `_` | Local state/value operations | Unsupported: unsupported native DC2 instruction {:#04x} operands {a:?} text {text:?}; execution stopped without advancing | [716](../../crates/crysirror-circus/src/dc2.rs#L716) |

### dc2_audio.rs / voice_request

```rust
fn voice_request(settings: &crate::dc2_settings::SystemData, name: &str, balance_db100: i32) -> Result<Option<Request>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `dword(56)? == 0 \|\| dword(64)? == 0 \|\| name.is_empty()` | Local state/value operations | Handled | [166](../../crates/crysirror-circus/src/dc2_audio.rs#L166) |

### dc2_audio.rs / Machine::execute_audio

```rust
fn execute_audio(
        &mut self,
        instruction: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x36` | Local state/value operations | Handled | [121](../../crates/crysirror-circus/src/dc2_audio.rs#L121) |

### dc2_crossfade.rs / Machine::execute_crossfade

```rust
fn execute_crossfade(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(instruction.opcode == 0x0d && matches!(a[0], 20..=22))` | Local state/value operations | Handled | [167](../../crates/crysirror-circus/src/dc2_crossfade.rs#L167) |

### dc2_crossfade.rs / Machine::execute_fade_in

```rust
fn execute_fade_in(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(instruction.opcode == 0x0d && a[0] == 1)` | Local state/value operations | Handled | [329](../../crates/crysirror-circus/src/dc2_crossfade.rs#L329) |

### dc2_crossfade.rs / Machine::execute_flash

```rust
fn execute_flash(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(instruction.opcode == 0x0d && a[0] == 3)` | Local state/value operations | Handled | [103](../../crates/crysirror-circus/src/dc2_crossfade.rs#L103) |

### dc2_crossfade.rs / Machine::execute_redraw

```rust
fn execute_redraw(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(instruction.opcode == 0x0d && matches!(a[0], 5 \| 12 \| 15))` | Local state/value operations | Handled | [42](../../crates/crysirror-circus/src/dc2_crossfade.rs#L42) |

### dc2_crossfade.rs / Machine::execute_shake

```rust
fn execute_shake(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(instruction.opcode == 0x0d && matches!(a[0], 4 \| 6))` | Local state/value operations | Handled | [255](../../crates/crysirror-circus/src/dc2_crossfade.rs#L255) |

### dc2_draw.rs / DrawState::apply_window_geometry

```rust
fn apply_window_geometry(
        &mut self,
        visible: &[bool],
        alternate: bool,
        native_font_style: bool,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!visible \|\| record.kind != 20` | Local state/value operations | Handled | [155](../../crates/crysirror-circus/src/dc2_draw.rs#L155) |

### dc2_draw.rs / DrawState::declare

```rust
fn declare(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        check: &mut i32,
    ) -> Result<Option<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector; under instruction.opcode=0x21` | `1 \| 4` | Local state/value operations | Handled | [43](../../crates/crysirror-circus/src/dc2_draw.rs#L43) |
| `selector; under instruction.opcode=0x21` | `3 \| 6 \| 7` | Local state/value operations | Handled | [44](../../crates/crysirror-circus/src/dc2_draw.rs#L44) |
| `selector; under instruction.opcode=0x21` | `2 \| 5` | Local state/value operations | Handled | [45](../../crates/crysirror-circus/src/dc2_draw.rs#L45) |
| `selector; under instruction.opcode=0x21` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [46](../../crates/crysirror-circus/src/dc2_draw.rs#L46) |
| `condition; under instruction.opcode=0x21` | `selector == 255` | Local state/value operations | Handled | [57](../../crates/crysirror-circus/src/dc2_draw.rs#L57) |
| `condition; under instruction.opcode=0x61` | `a[2] == 0` | Local state/value operations | Handled | [128](../../crates/crysirror-circus/src/dc2_draw.rs#L128) |
| `instruction.opcode` | `0x21` | self.snapshots.insert | Handled | [39](../../crates/crysirror-circus/src/dc2_draw.rs#L39) |
| `instruction.opcode` | `0x53` | i32::from, Default::default | Handled | [65](../../crates/crysirror-circus/src/dc2_draw.rs#L65) |
| `instruction.opcode` | `0x60` | Default::default | Handled | [105](../../crates/crysirror-circus/src/dc2_draw.rs#L105) |
| `instruction.opcode` | `0x61` | Local state/value operations | Handled | [127](../../crates/crysirror-circus/src/dc2_draw.rs#L127) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [135](../../crates/crysirror-circus/src/dc2_draw.rs#L135) |

### dc2_fade.rs / Machine::execute_fade

```rust
fn execute_fade(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x0d \|\| a[0] != 2` | Local state/value operations | Handled | [25](../../crates/crysirror-circus/src/dc2_fade.rs#L25) |

### dc2_map.rs / Machine::execute_map

```rust
fn execute_map(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x1c` | Local state/value operations | Handled | [36](../../crates/crysirror-circus/src/dc2_map.rs#L36) |
| `a[0]` | `selector` | Local state/value operations | Unsupported: DC2 408560 selector {selector} is not connected | [45](../../crates/crysirror-circus/src/dc2_map.rs#L45) |

### dc2_menu.rs / Machine::admit_rows

```rust
fn admit_rows(
        &mut self,
        buttons: impl Iterator<Item = (usize, ui::Button)>,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Vec<(usize, ui::Button)>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `button.kind == 131` | Local state/value operations | Handled | [106](../../crates/crysirror-circus/src/dc2_menu.rs#L106) |

### dc2_menu.rs / Machine::execute_menu

```rust
fn execute_menu(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [44](../../crates/crysirror-circus/src/dc2_menu.rs#L44) |

### dc2_movie.rs / Machine::execute_movie

```rust
fn execute_movie(
        &mut self,
        instruction: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x38` | Local state/value operations | Handled | [17](../../crates/crysirror-circus/src/dc2_movie.rs#L17) |

### dc2_music.rs / Machine::execute_music

```rust
fn execute_music(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(instruction.opcode, 5 \| 6)` | Local state/value operations | Handled | [16](../../crates/crysirror-circus/src/dc2_music.rs#L16) |
| `condition` | `instruction.opcode == 6` | self.advance | Handled | [19](../../crates/crysirror-circus/src/dc2_music.rs#L19) |
| `condition` | `extension.opcode == 6` | i32::from | Handled | [30](../../crates/crysirror-circus/src/dc2_music.rs#L30) |
| `condition` | `a[0] == 255` | host.emit, self.state.runtime.insert, self.advance | Handled | [38](../../crates/crysirror-circus/src/dc2_music.rs#L38) |
| `condition` | `a[0] == 254` | self.state.runtime.get | Handled | [47](../../crates/crysirror-circus/src/dc2_music.rs#L47) |
| `condition` | `!(0..=2).contains(&a[1]) \|\| id < 0` | self.state.runtime.insert, self.advance | Handled | [52](../../crates/crysirror-circus/src/dc2_music.rs#L52) |
| `condition` | `a[1] == 1 && self.state.runtime.get(&244).copied().unwrap_or(-1) == id` | self.advance | Handled | [56](../../crates/crysirror-circus/src/dc2_music.rs#L56) |
| `condition` | `id >= 80` | Local state/value operations | Handled | [94](../../crates/crysirror-circus/src/dc2_music.rs#L94) |
| `condition` | `!host.direct_audio(&Request { name, channel: 0, voice: false, gain_db100, balance_db100: 0, looping: a[1] != 2, streamed: true, })?` | Local state/value operations | Handled | [108](../../crates/crysirror-circus/src/dc2_music.rs#L108) |
| `condition` | `id < 100` | self.state.settings.set_byte | Handled | [120](../../crates/crysirror-circus/src/dc2_music.rs#L120) |

### dc2_pan.rs / Machine::execute_pan

```rust
fn execute_pan(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x62` | Local state/value operations | Handled | [27](../../crates/crysirror-circus/src/dc2_pan.rs#L27) |
| `a[2]` | `1` | self.advance | Handled | [31](../../crates/crysirror-circus/src/dc2_pan.rs#L31) |
| `a[2]` | `2` | self.advance | Handled | [35](../../crates/crysirror-circus/src/dc2_pan.rs#L35) |
| `a[2]` | `0 \| 3 \| 8..=135` | Local state/value operations | Empty handler: Original no-op status not established | [39](../../crates/crysirror-circus/src/dc2_pan.rs#L39) |
| `a[2]` | `4..=7` | self.advance | Handled | [41](../../crates/crysirror-circus/src/dc2_pan.rs#L41) |
| `a[2]` | `_` | Local state/value operations | Unsupported: DC2 native pan speed selector {} is not mapped | [42](../../crates/crysirror-circus/src/dc2_pan.rs#L42) |
| `condition` | `a[2] == 3` | Local state/value operations | Handled | [55](../../crates/crysirror-circus/src/dc2_pan.rs#L55) |
| `condition` | `a[2] == 3` | Local state/value operations | Handled | [79](../../crates/crysirror-circus/src/dc2_pan.rs#L79) |

### dc2_portrait.rs / Machine::execute_expression

```rust
fn execute_expression(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x2c` | Local state/value operations | Handled | [435](../../crates/crysirror-circus/src/dc2_portrait.rs#L435) |

### dc2_portrait.rs / Machine::execute_motion

```rust
fn execute_motion(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(instruction.opcode == 0x65 && (0..5).contains(&a[3]))` | Local state/value operations | Handled | [150](../../crates/crysirror-circus/src/dc2_portrait.rs#L150) |

### dc2_portrait.rs / Machine::execute_portrait

```rust
fn execute_portrait(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x2b` | Local state/value operations | Handled | [217](../../crates/crysirror-circus/src/dc2_portrait.rs#L217) |

### dc2_portrait.rs / Machine::load_native5

```rust
fn load_native5(&mut self, name: &str, host: &mut dyn NativeHost, control: bool) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.load_portrait(4, name, None, host, control)?` | Local state/value operations | Handled | [352](../../crates/crysirror-circus/src/dc2_portrait.rs#L352) |

### dc2_resources.rs / Machine::execute_resources

```rust
fn execute_resources(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under instruction.opcode=0x2a \| 0x31..=0x33` | `instruction.opcode == 0x2a` | self.advance | Handled | [69](../../crates/crysirror-circus/src/dc2_resources.rs#L69) |
| `condition; under instruction.opcode=0x2a \| 0x31..=0x33` | `!(0..=3).contains(&a[0])` | self.advance | Handled | [70](../../crates/crysirror-circus/src/dc2_resources.rs#L70) |
| `condition; under instruction.opcode=0x30` | `!host.emit(&Event::LoadImage { slot: 1, name: text.into(), })?` | Local state/value operations | Handled | [207](../../crates/crysirror-circus/src/dc2_resources.rs#L207) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [252](../../crates/crysirror-circus/src/dc2_resources.rs#L252) |

### dc2_scene.rs / Machine::execute_scene

```rust
fn execute_scene(
        &mut self,
        instruction: &Instruction,
        selector: i16,
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x2e` | Local state/value operations | Handled | [18](../../crates/crysirror-circus/src/dc2_scene.rs#L18) |
| `selector` | `_` | Local state/value operations | Handled | [35](../../crates/crysirror-circus/src/dc2_scene.rs#L35) |
| `condition` | `selector != 4 && previous >= 0` | Local state/value operations | Handled | [54](../../crates/crysirror-circus/src/dc2_scene.rs#L54) |
| `condition` | `mode == 1 \|\| selector == 5` | host.resource | Handled | [75](../../crates/crysirror-circus/src/dc2_scene.rs#L75) |
| `condition` | `mode != 2` | self.state.runtime.insert | Handled | [111](../../crates/crysirror-circus/src/dc2_scene.rs#L111) |
| `condition` | `mode == 1 \|\| selector == 5` | Default::default, self.state.draw.planes.iter_mut, self.state.resources.display_base.clear | Handled | [118](../../crates/crysirror-circus/src/dc2_scene.rs#L118) |

### dc2_shatter.rs / initialize

```rust
fn initialize(kind: i16, rand: &mut dyn FnMut() -> Result<i32>) -> Result<TileScatter>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `4` | Local state/value operations | Handled | [51](../../crates/crysirror-circus/src/dc2_shatter.rs#L51) |
| `kind` | `0` | Local state/value operations | Handled | [62](../../crates/crysirror-circus/src/dc2_shatter.rs#L62) |
| `kind` | `1` | Local state/value operations | Handled | [72](../../crates/crysirror-circus/src/dc2_shatter.rs#L72) |
| `kind` | `2` | Local state/value operations | Handled | [81](../../crates/crysirror-circus/src/dc2_shatter.rs#L81) |
| `kind` | `3` | Local state/value operations | Handled | [90](../../crates/crysirror-circus/src/dc2_shatter.rs#L90) |
| `kind` | `5` | Local state/value operations | Handled | [100](../../crates/crysirror-circus/src/dc2_shatter.rs#L100) |
| `kind` | `6` | Local state/value operations | Handled | [110](../../crates/crysirror-circus/src/dc2_shatter.rs#L110) |
| `kind` | `9` | Local state/value operations | Handled | [120](../../crates/crysirror-circus/src/dc2_shatter.rs#L120) |
| `kind` | `10` | Local state/value operations | Handled | [130](../../crates/crysirror-circus/src/dc2_shatter.rs#L130) |
| `kind` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [136](../../crates/crysirror-circus/src/dc2_shatter.rs#L136) |

### dc2_shatter.rs / Machine::execute_shatter

```rust
fn execute_shatter(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(instruction.opcode == 0x0d && a[0] == 14)` | Local state/value operations | Handled | [149](../../crates/crysirror-circus/src/dc2_shatter.rs#L149) |
| `condition` | `!host.emit(&Event::LoadImage { slot: OVERLAY_SLOT, name: "system\\eff".into() })?` | Local state/value operations | Handled | [203](../../crates/crysirror-circus/src/dc2_shatter.rs#L203) |

### dc2_sound.rs / Machine::execute_sound

```rust
fn execute_sound(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x5b` | Local state/value operations | Handled | [39](../../crates/crysirror-circus/src/dc2_sound.rs#L39) |
| `condition` | `id == -1` | self.state.audio.sound_fade.clone, host.audio_playing, usize::from, self.sound_level, host.emit | Handled | [43](../../crates/crysirror-circus/src/dc2_sound.rs#L43) |
| `condition` | `mode == 1` | self.state.audio.sound_fade.clone, host.audio_playing, usize::from, self.sound_level | Handled | [44](../../crates/crysirror-circus/src/dc2_sound.rs#L44) |
| `condition` | `id >= 0 && enabled` | host.dc2_sound, host.audio_playing, usize::from, self.sound_level, host.direct_audio | Handled | [77](../../crates/crysirror-circus/src/dc2_sound.rs#L77) |
| `condition` | `mode != 0` | Local state/value operations | Handled | [85](../../crates/crysirror-circus/src/dc2_sound.rs#L85) |
| `condition` | `!host.direct_audio(&Request { name, channel: host_channel(channel), voice: false, gain_db100, balance_db100: 0, looping: mode == 1, streamed: false, })?` | Local state/value operations | Handled | [100](../../crates/crysirror-circus/src/dc2_sound.rs#L100) |
| `condition` | `mode != 0` | Local state/value operations | Handled | [113](../../crates/crysirror-circus/src/dc2_sound.rs#L113) |

### dc2_stage.rs / Machine::default_composition

```rust
fn default_composition(
        &mut self,
        host: &mut dyn NativeHost,
        include_buttons: bool,
    ) -> Result<Option<(Vec<crate::dc2_bitmap::Draw>, Vec<crate::dc2_bitmap::Draw>)>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `button.kind == 131` | Local state/value operations | Handled | [37](../../crates/crysirror-circus/src/dc2_stage.rs#L37) |

### dc2_stage.rs / Machine::execute_stage

```rust
fn execute_stage(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode == 0x20 \|\| bake` | self.state.scene.clone, self.default_composition, host.bitmap_stage, self.state.draw.planes.iter_mut, Vec::new | Handled | [150](../../crates/crysirror-circus/src/dc2_stage.rs#L150) |
| `condition` | `instruction.opcode == 0x4c && !self.clear_text(host)?` | Local state/value operations | Handled | [229](../../crates/crysirror-circus/src/dc2_stage.rs#L229) |

### dc2_stage.rs / Machine::record_draw

```rust
fn record_draw(&self, record: &draw::Record) -> Result<Option<crate::dc2_bitmap::Draw>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind & 128 != 0 && self.state.settings.prefix(10)?.unwrap_or(0) != 0` | self.state.settings.prefix | Handled | [91](../../crates/crysirror-circus/src/dc2_stage.rs#L91) |
| `condition` | `kind & 64 != 0` | Local state/value operations | Handled | [99](../../crates/crysirror-circus/src/dc2_stage.rs#L99) |

### dc2_text.rs / Machine::execute_message

```rust
fn execute_message(
        &mut self,
        instruction: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
        skipping: bool,
    ) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.state.last_opcode != Some(0x4b)` | self.state.text.name.clear | Handled | [288](../../crates/crysirror-circus/src/dc2_text.rs#L288) |

### dc2_text.rs / Machine::execute_text

```rust
fn execute_text(
        &mut self,
        instruction: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under instruction.opcode=0x07` | `self.state.last_opcode != Some(0x4c)` | self.advance | Handled | [249](../../crates/crysirror-circus/src/dc2_text.rs#L249) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [273](../../crates/crysirror-circus/src/dc2_text.rs#L273) |

### dc2_text.rs / Machine::window_draws

```rust
fn window_draws(&mut self, host: &mut dyn NativeHost) -> Result<Option<Vec<Draw>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `record.kind == 20` | Local state/value operations | Handled | [196](../../crates/crysirror-circus/src/dc2_text.rs#L196) |

### dc2_ui.rs / Machine::execute_ui

```rust
fn execute_ui(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
    ) -> Result<Option<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under instruction.opcode=0x3d \| 0x3e` | `instruction.opcode == 0x3d` | Local state/value operations | Handled | [62](../../crates/crysirror-circus/src/dc2_ui.rs#L62) |
| `selector; under instruction.opcode=0x17` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [83](../../crates/crysirror-circus/src/dc2_ui.rs#L83) |
| `condition; under instruction.opcode=0x17` | `matches!(selector, 1 \| 4)` | Local state/value operations | Handled | [89](../../crates/crysirror-circus/src/dc2_ui.rs#L89) |
| `condition; under instruction.opcode=0x17` | `matches!(selector, 1 \| 4)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [89](../../crates/crysirror-circus/src/dc2_ui.rs#L89) |
| `condition; under instruction.opcode=0x17` | `selector == 3` | Local state/value operations | Handled | [94](../../crates/crysirror-circus/src/dc2_ui.rs#L94) |
| `condition; under instruction.opcode=0x17` | `selector == 1` | Local state/value operations | Handled | [103](../../crates/crysirror-circus/src/dc2_ui.rs#L103) |
| `condition; under instruction.opcode=0x18` | `a[0] == 1 && self.state.runtime.get(&3).copied().unwrap_or(0) == 0` | Local state/value operations | Handled | [113](../../crates/crysirror-circus/src/dc2_ui.rs#L113) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [261](../../crates/crysirror-circus/src/dc2_ui.rs#L261) |

### dc2_wait.rs / Machine::execute_wait

```rust
fn execute_wait(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(instruction.opcode, 0x3b \| 0x3c)` | usize::from, self.advance | Handled | [33](../../crates/crysirror-circus/src/dc2_wait.rs#L33) |
| `condition` | `instruction.opcode != 0x12` | Local state/value operations | Handled | [39](../../crates/crysirror-circus/src/dc2_wait.rs#L39) |
| `condition` | `a[1] & 8 != 0` | Local state/value operations | Handled | [43](../../crates/crysirror-circus/src/dc2_wait.rs#L43) |
| `condition` | `a[0] == 0` | Local state/value operations | Handled | [44](../../crates/crysirror-circus/src/dc2_wait.rs#L44) |

### dc2_weather.rs / texture_name

```rust
fn texture_name(kind: i32) -> &'static str
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `11` | Local state/value operations | Handled | [85](../../crates/crysirror-circus/src/dc2_weather.rs#L85) |
| `kind` | `12` | Local state/value operations | Handled | [86](../../crates/crysirror-circus/src/dc2_weather.rs#L86) |
| `kind` | `3` | Local state/value operations | Handled | [87](../../crates/crysirror-circus/src/dc2_weather.rs#L87) |
| `kind` | `1` | Local state/value operations | Handled | [88](../../crates/crysirror-circus/src/dc2_weather.rs#L88) |
| `kind` | `_` | Local state/value operations | Handled | [89](../../crates/crysirror-circus/src/dc2_weather.rs#L89) |

### dc2_weather.rs / Machine::start_weather

```rust
fn start_weather(&mut self, kind: i32, density: i32, host: &mut dyn NativeHost, now: u64) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `current != kind && current != 0` | host.dc2_weather | Handled | [220](../../crates/crysirror-circus/src/dc2_weather.rs#L220) |
| `condition` | `kind == 0 \|\| !enabled` | Default::default | Handled | [228](../../crates/crysirror-circus/src/dc2_weather.rs#L228) |
| `kind` | `1 \| 4 \| 11 \| 12` | Local state/value operations | Handled | [233](../../crates/crysirror-circus/src/dc2_weather.rs#L233) |
| `kind` | `3` | Local state/value operations | Handled | [234](../../crates/crysirror-circus/src/dc2_weather.rs#L234) |
| `kind` | `_` | Local state/value operations | Unsupported: DC2 weather profile {kind} (417CE0/4198C0/4199B0) is not connected | [235](../../crates/crysirror-circus/src/dc2_weather.rs#L235) |

### dc2_wipe.rs / field

```rust
fn field(mode: u8, x: i32, y: i32) -> Option<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `0` | Local state/value operations | Handled | [29](../../crates/crysirror-circus/src/dc2_wipe.rs#L29) |
| `mode` | `1` | Local state/value operations | Handled | [30](../../crates/crysirror-circus/src/dc2_wipe.rs#L30) |
| `mode` | `2` | Local state/value operations | Handled | [32](../../crates/crysirror-circus/src/dc2_wipe.rs#L32) |
| `mode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [33](../../crates/crysirror-circus/src/dc2_wipe.rs#L33) |

### dc2_wipe.rs / Machine::execute_wipe

```rust
fn execute_wipe(
        &mut self,
        instruction: &Instruction,
        a: [i16; 4],
        host: &mut dyn NativeHost,
        now: u64,
        control: bool,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(instruction.opcode == 0x0d && a[0] == 13)` | Local state/value operations | Handled | [49](../../crates/crysirror-circus/src/dc2_wipe.rs#L49) |
| `condition` | `!host.bitmap_stage(&Request::Wipe { owner: wipe.owner.clone(), slot: 1, mode: wipe.mode, offset: wipe.offset, })?` | Local state/value operations | Handled | [143](../../crates/crysirror-circus/src/dc2_wipe.rs#L143) |

### dc3.rs / native_noop

```rust
fn native_noop(opcode: u8) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( opcode, 0x01 \| 0x02 \| 0x0c \| 0x0d \| 0x11 \| 0x17 \| 0x1d \| 0x21 \| 0x24 \| 0x26 \| 0x28 \| 0x29 \| 0x2a \| 0x2b \| 0x2c \| 0x2d \| 0x2e \| 0x2f \| 0x30 \| 0x38 \| 0x39 \| 0x40 \| 0x42 \| 0x44 \| 0x47 \| 0x4c \| 0x50 \| 0x55 \| 0x56 \| 0x59 \| 0x5b \| 0x5f \| 0x60 \| 0x63 \| 0x64 \| 0x6d \| 0x72 \| 0x80 \| 0x87 \| 0x88 \| 0x89 \| 0x8d \| 0x8e \| 0x99 )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [364](../../crates/crysirror-circus/src/dc3.rs#L364) |

### dc3.rs / skip_marker

```rust
fn skip_marker(program: &Program, mut pc: u32, tag: i16) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x1e && i16::from( program .native_byte(pc + 2) .context("DC3WY truncated conditional marker")? as i8, ) == tag` | Local state/value operations | Handled | [1208](../../crates/crysirror-circus/src/dc3.rs#L1208) |
| `condition` | `opcode < 0x39` | Local state/value operations | Handled | [1219](../../crates/crysirror-circus/src/dc3.rs#L1219) |
| `condition` | `opcode >= 0x64` | Local state/value operations | Handled | [1221](../../crates/crysirror-circus/src/dc3.rs#L1221) |
| `condition` | `opcode < 0x42` | Local state/value operations | Handled | [1224](../../crates/crysirror-circus/src/dc3.rs#L1224) |

### dc3.rs / template_slot

```rust
fn template_slot(selector: i16, finalizing: bool) -> Option<u8>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector` | `1` | Local state/value operations | Handled | [1252](../../crates/crysirror-circus/src/dc3.rs#L1252) |
| `selector` | `2` | Local state/value operations | Handled | [1253](../../crates/crysirror-circus/src/dc3.rs#L1253) |
| `selector` | `3` | Local state/value operations | Handled | [1254](../../crates/crysirror-circus/src/dc3.rs#L1254) |
| `selector` | `6` | Local state/value operations | Handled | [1255](../../crates/crysirror-circus/src/dc3.rs#L1255) |
| `selector` | `7` | Local state/value operations | Handled | [1256](../../crates/crysirror-circus/src/dc3.rs#L1256) |
| `selector` | `13` | Local state/value operations | Handled | [1257](../../crates/crysirror-circus/src/dc3.rs#L1257) |
| `selector` | `14` | Local state/value operations | Handled | [1258](../../crates/crysirror-circus/src/dc3.rs#L1258) |
| `selector` | `15` | Local state/value operations | Handled | [1259](../../crates/crysirror-circus/src/dc3.rs#L1259) |
| `selector` | `16` | Local state/value operations | Handled | [1260](../../crates/crysirror-circus/src/dc3.rs#L1260) |
| `selector` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1261](../../crates/crysirror-circus/src/dc3.rs#L1261) |
| `selector` | `1..=7` | Local state/value operations | Handled | [1265](../../crates/crysirror-circus/src/dc3.rs#L1265) |
| `selector` | `13` | Local state/value operations | Handled | [1266](../../crates/crysirror-circus/src/dc3.rs#L1266) |
| `selector` | `14` | Local state/value operations | Handled | [1267](../../crates/crysirror-circus/src/dc3.rs#L1267) |
| `selector` | `15` | Local state/value operations | Handled | [1268](../../crates/crysirror-circus/src/dc3.rs#L1268) |
| `selector` | `16` | Local state/value operations | Handled | [1269](../../crates/crysirror-circus/src/dc3.rs#L1269) |
| `selector` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1270](../../crates/crysirror-circus/src/dc3.rs#L1270) |

### dc3.rs / Machine::execute

```rust
fn execute(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `a[3] == 0` | Local state/value operations | Handled | [544](../../crates/crysirror-circus/src/dc3.rs#L544) |
| `condition` | `instruction.opcode == 0x16 && a[0] == 0` | self.execute_load | Handled | [624](../../crates/crysirror-circus/src/dc3.rs#L624) |
| `condition; under instruction.opcode=0x03` | `program .instruction(next) .map_or(true, \|following\| following.opcode != 0x45)` | self.state.runtime_values.get | Handled | [667](../../crates/crysirror-circus/src/dc3.rs#L667) |
| `condition; under instruction.opcode=0x7c` | `(a[3] as u8 as i8) < 0` | self.value, i16::from | Handled | [814](../../crates/crysirror-circus/src/dc3.rs#L814) |
| `mode; under instruction.opcode=0x7c` | `0` | Local state/value operations | Handled | [820](../../crates/crysirror-circus/src/dc3.rs#L820) |
| `mode; under instruction.opcode=0x7c` | `1` | Local state/value operations | Handled | [821](../../crates/crysirror-circus/src/dc3.rs#L821) |
| `mode; under instruction.opcode=0x7c` | `2` | Local state/value operations | Handled | [822](../../crates/crysirror-circus/src/dc3.rs#L822) |
| `mode; under instruction.opcode=0x7c` | `3` | Local state/value operations | Handled | [823](../../crates/crysirror-circus/src/dc3.rs#L823) |
| `mode; under instruction.opcode=0x7c` | `4` | Local state/value operations | Handled | [824](../../crates/crysirror-circus/src/dc3.rs#L824) |
| `mode; under instruction.opcode=0x7c` | `5` | Local state/value operations | Handled | [825](../../crates/crysirror-circus/src/dc3.rs#L825) |
| `mode; under instruction.opcode=0x7c` | `_` | Local state/value operations | Handled | [826](../../crates/crysirror-circus/src/dc3.rs#L826) |
| `a[3]; under instruction.opcode=0x78` | `0` | i32::from | Handled | [841](../../crates/crysirror-circus/src/dc3.rs#L841) |
| `a[3]; under instruction.opcode=0x78` | `1` | self.value, i32::from | Handled | [842](../../crates/crysirror-circus/src/dc3.rs#L842) |
| `a[3]; under instruction.opcode=0x78` | `2` | self.value, i32::from | Handled | [843](../../crates/crysirror-circus/src/dc3.rs#L843) |
| `a[3]; under instruction.opcode=0x78` | `_` | self.advance | Handled | [844](../../crates/crysirror-circus/src/dc3.rs#L844) |
| `condition; under instruction.opcode=0x78` | `!self.assign(host, a[0], a[1], value)?` | Local state/value operations | Handled | [846](../../crates/crysirror-circus/src/dc3.rs#L846) |
| `condition; under instruction.opcode=0x79` | `!self.assign(host, a[0], a[1], value)?` | Local state/value operations | Handled | [852](../../crates/crysirror-circus/src/dc3.rs#L852) |
| `condition; under instruction.opcode=0x7a` | `!self.assign(host, a[0], a[1], total)?` | Local state/value operations | Handled | [883](../../crates/crysirror-circus/src/dc3.rs#L883) |
| `condition; under instruction.opcode=0x7b` | `!self.assign(host, a[0], a[1], total)?` | Local state/value operations | Handled | [891](../../crates/crysirror-circus/src/dc3.rs#L891) |
| `condition; under instruction.opcode=0x86` | `!host.emit(&Event::Palette { index: a[0] as u8, rgb: logical_color(rgb), })?` | Local state/value operations | Handled | [950](../../crates/crysirror-circus/src/dc3.rs#L950) |
| `condition; under instruction.opcode=0x83` | `!self.place_object(host, 65, [i32::from(a[0]), i32::from(a[1])])?` | Local state/value operations | Handled | [965](../../crates/crysirror-circus/src/dc3.rs#L965) |
| `condition; under instruction.opcode=0x22` | `let Some(slot) = template_slot(a[0], false)` | std::mem::take | Handled | [975](../../crates/crysirror-circus/src/dc3.rs#L975) |
| `condition; under instruction.opcode=0x18 if a[0] > 0` | `let Some(slot) = template_slot(a[0], true)` | host.emit, self.state.templates.insert, Builder::default | Handled | [985](../../crates/crysirror-circus/src/dc3.rs#L985) |
| `condition; under instruction.opcode=0x15` | `a[0] != 255` | Local state/value operations | Handled | [1042](../../crates/crysirror-circus/src/dc3.rs#L1042) |
| `condition; under instruction.opcode=0x15` | `a[1] != 255` | Local state/value operations | Handled | [1045](../../crates/crysirror-circus/src/dc3.rs#L1045) |
| `condition; under instruction.opcode=0x6a` | `a[7] == 5` | i32::from | Handled | [1053](../../crates/crysirror-circus/src/dc3.rs#L1053) |
| `a[7]; under instruction.opcode=0x6a` | `6` | Local state/value operations | Handled | [1070](../../crates/crysirror-circus/src/dc3.rs#L1070) |
| `a[7]; under instruction.opcode=0x6a` | `8` | Local state/value operations | Handled | [1071](../../crates/crysirror-circus/src/dc3.rs#L1071) |
| `a[7]; under instruction.opcode=0x6a` | `10` | Local state/value operations | Handled | [1072](../../crates/crysirror-circus/src/dc3.rs#L1072) |
| `a[7]; under instruction.opcode=0x6a` | `11` | Local state/value operations | Handled | [1073](../../crates/crysirror-circus/src/dc3.rs#L1073) |
| `a[7]; under instruction.opcode=0x6a` | `value` | Local state/value operations | Handled | [1074](../../crates/crysirror-circus/src/dc3.rs#L1074) |
| `condition; under instruction.opcode=0x70` | `let Some(part) = self.image_part(operands, base, 0, (a[6] as u8 as i8) < 0, host)?` | self.state.builder.group.images.push | Handled | [1102](../../crates/crysirror-circus/src/dc3.rs#L1102) |
| `condition; under instruction.opcode=0x19` | `a[0] == 0` | Local state/value operations | Handled | [1113](../../crates/crysirror-circus/src/dc3.rs#L1113) |
| `condition; under instruction.opcode=0x4d` | `let Some(target) = program .instructions .iter() .take_while(\|item\| item.opcode != 0) .find(\|item\| { item.opcode == 0x44 && !text.is_empty() && program.literal_bytes(item.offset) == program.literal_bytes(instruction.offset) })` | Local state/value operations | Handled | [1120](../../crates/crysirror-circus/src/dc3.rs#L1120) |
| `instruction.opcode` | `0x09 if a[0] != 0` | self.state.system_data.clone, self.state.runtime_values.get, self.state.skip_latches.iter, self.state.runtime_values.insert | Handled | [710](../../crates/crysirror-circus/src/dc3.rs#L710) |
| `instruction.opcode` | `0x0e if matches!(a[0], 7 \| 10 \| 11 \| 13 \| 14 \| 23 \| 24 \| 25 \| 30 \| 31 \| 32)` | Local state/value operations | Empty handler: Original no-op status not established | [734](../../crates/crysirror-circus/src/dc3.rs#L734) |
| `instruction.opcode` | `0x1e if a[0] != 0` | Local state/value operations | Empty handler: Original no-op status not established | [735](../../crates/crysirror-circus/src/dc3.rs#L735) |
| `instruction.opcode` | `0x1c if a[0] == 1` | host.system, self.assign, i32::from | Handled | [739](../../crates/crysirror-circus/src/dc3.rs#L739) |
| `instruction.opcode` | `0x1c if a[0] == 19` | u16::try_from, self.value, host.resource, self.assign, i32::from | Handled | [751](../../crates/crysirror-circus/src/dc3.rs#L751) |
| `instruction.opcode` | `0x1c if a[0] == 9` | self.begin_credits, Outcome::WaitUntil | Handled | [777](../../crates/crysirror-circus/src/dc3.rs#L777) |
| `instruction.opcode` | `0x1c if a[0] == 207` | self.state.scene.to_ascii_lowercase, self.assign | Handled | [781](../../crates/crysirror-circus/src/dc3.rs#L781) |
| `instruction.opcode` | `0x1c if !matches!( a[0], 1 \| 6 \| 7 \| 9 \| 18 \| 19 \| 50 \| 51 \| 98 \| 99 \| 200 \| 201 \| 202 \| 203 \| 204 \| 206 \| 207 )` | Local state/value operations | Empty handler: Original no-op status not established | [793](../../crates/crysirror-circus/src/dc3.rs#L793) |
| `instruction.opcode` | `0x86 if a[0] >= 64` | Local state/value operations | Empty handler: Original no-op status not established | [946](../../crates/crysirror-circus/src/dc3.rs#L946) |
| `instruction.opcode` | `0x18 if a[0] > 0` | self.state.builder.clone, host.emit, self.state.templates.insert, Builder::default | Handled | [981](../../crates/crysirror-circus/src/dc3.rs#L981) |
| `instruction.opcode` | `0x13 if a[0] < 254` | u64::try_from, Outcome::WaitUntil | Handled | [1134](../../crates/crysirror-circus/src/dc3.rs#L1134) |
| `instruction.opcode` | `opcode if native_noop(opcode)` | Local state/value operations | Empty handler: Original no-op status not established | [1146](../../crates/crysirror-circus/src/dc3.rs#L1146) |
| `instruction.opcode` | `opcode` | Local state/value operations | Unsupported: unsupported original DC3WY instruction {opcode:#04x}; execution stopped without advancing | [1147](../../crates/crysirror-circus/src/dc3.rs#L1147) |

### dc3_advmenu.rs / Machine::execute_return

```rust
fn execute_return(&mut self, instruction: &Instruction, a: [i16; 8], host: &mut dyn NativeHost) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x1b` | Local state/value operations | Handled | [110](../../crates/crysirror-circus/src/dc3_advmenu.rs#L110) |
| `condition` | `a[1] == 0` | Local state/value operations | Handled | [120](../../crates/crysirror-circus/src/dc3_advmenu.rs#L120) |

### dc3_advmenu.rs / Machine::execute_rollback

```rust
fn execute_rollback(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `text != "--" \|\| !matches!(instruction.opcode, 0x3a \| 0x43 \| 0x3d \| 0x3f)` | Local state/value operations | Handled | [165](../../crates/crysirror-circus/src/dc3_advmenu.rs#L165) |
| `condition; under instruction.opcode=0x43` | `!self.publish_object_with(host, 1, 0, &name, false)?` | Local state/value operations | Handled | [195](../../crates/crysirror-circus/src/dc3_advmenu.rs#L195) |
| `instruction.opcode` | `0x3a if (1..=22).contains(&a[0])` | self.return_name, self.execute_resources | Handled | [170](../../crates/crysirror-circus/src/dc3_advmenu.rs#L170) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [226](../../crates/crysirror-circus/src/dc3_advmenu.rs#L226) |

### dc3_audio.rs / Machine::execute_audio

```rust
fn execute_audio(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<bool>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `a[0]; under instruction.opcode=0x65` | `value` | i32::from | Handled | [84](../../crates/crysirror-circus/src/dc3_audio.rs#L84) |
| `condition; under instruction.opcode=0x45 \| 0x46` | `!host.emit(&Event::ReleaseAudioGroup { kind: 2, forced: true, steps: 0, })?` | Local state/value operations | Handled | [109](../../crates/crysirror-circus/src/dc3_audio.rs#L109) |
| `condition; under instruction.opcode=0x12` | `!host.emit(&Event::ReleaseAudioGroup { kind: 0, forced: false, steps: 60 })?` | Local state/value operations | Handled | [153](../../crates/crysirror-circus/src/dc3_audio.rs#L153) |
| `condition; under instruction.opcode=0x68 if a[0] == -2` | `!host.emit(&Event::ReleaseAudioGroup { kind: 2, forced: true, steps: 0, })?` | Local state/value operations | Handled | [162](../../crates/crysirror-circus/src/dc3_audio.rs#L162) |
| `condition; under instruction.opcode=0x68 if a[0] == -1` | `!host.emit(&Event::ReleaseAudioGroup { kind: 1, forced: false, steps: 60, })?` | Local state/value operations | Handled | [176](../../crates/crysirror-circus/src/dc3_audio.rs#L176) |
| `instruction.opcode` | `0x05 if a[0] == 254` | self.publish_audio | Handled | [75](../../crates/crysirror-circus/src/dc3_audio.rs#L75) |
| `instruction.opcode` | `0x67 if a[0] == 255` | host.emit, self.state.audio.pending.remove, self.state.audio.music_name.clear | Handled | [88](../../crates/crysirror-circus/src/dc3_audio.rs#L88) |
| `instruction.opcode` | `0x68 if a[0] >= 0` | self.state.audio.pending.contains_key, host.sound_name, self.state.audio.pending.insert | Handled | [126](../../crates/crysirror-circus/src/dc3_audio.rs#L126) |
| `instruction.opcode` | `0x68 if a[0] == -2` | host.emit, self.state.audio.pending.retain, self.state.audio.last_voice.clear | Handled | [161](../../crates/crysirror-circus/src/dc3_audio.rs#L161) |
| `instruction.opcode` | `0x68 if a[0] == -1` | host.emit, Default::default | Handled | [172](../../crates/crysirror-circus/src/dc3_audio.rs#L172) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [192](../../crates/crysirror-circus/src/dc3_audio.rs#L192) |

### dc3_audio.rs / Machine::publish_audio

```rust
fn publish_audio(&mut self, host: &mut dyn NativeHost) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `controls.enabled[0] && !sound.name.is_empty()` | Local state/value operations | Handled | [215](../../crates/crysirror-circus/src/dc3_audio.rs#L215) |

### dc3_body.rs / Machine::execute_snapshot

```rust
fn execute_snapshot(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x1a` | Local state/value operations | Handled | [286](../../crates/crysirror-circus/src/dc3_body.rs#L286) |

### dc3_callbacks.rs / Machine::execute_handlers

```rust
fn execute_handlers(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        now: u64,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `a[0]; under instruction.opcode=0x41` | `_` | self.advance | Handled | [48](../../crates/crysirror-circus/src/dc3_callbacks.rs#L48) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [52](../../crates/crysirror-circus/src/dc3_callbacks.rs#L52) |
| `condition` | `instruction.opcode == 0x41 && a[0] == 16` | host.system, crate::machine::system_ops::SystemRequest::CallbackTimer | Handled | [62](../../crates/crysirror-circus/src/dc3_callbacks.rs#L62) |
| `condition` | `instruction.opcode == 0x41 && a[0] == 6` | Local state/value operations | Handled | [75](../../crates/crysirror-circus/src/dc3_callbacks.rs#L75) |
| `condition` | `instruction.opcode == 0x41 && a[0] == 16` | Local state/value operations | Handled | [79](../../crates/crysirror-circus/src/dc3_callbacks.rs#L79) |

### dc3_controls.rs / Machine::dialogue_action

```rust
fn dialogue_action(&mut self, host: &mut dyn NativeHost, action: &str, now: u64) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `name` | `"n" \| "n<" \| "n>"` | Local state/value operations | Handled | [156](../../crates/crysirror-circus/src/dc3_controls.rs#L156) |
| `name` | `_ if name.starts_with('c')` | self.set_window_hidden | Handled | [160](../../crates/crysirror-circus/src/dc3_controls.rs#L160) |
| `name` | `_ if name.starts_with('r')` | self.toggle_skip | Handled | [161](../../crates/crysirror-circus/src/dc3_controls.rs#L161) |
| `name` | `_ if name.starts_with('a') && !matches!(name.as_bytes().get(1), Some(b'<' \| b'>' \| b'+' \| b'-' \| b'='))` | self.toggle_auto | Handled | [165](../../crates/crysirror-circus/src/dc3_controls.rs#L165) |
| `name` | `_ if name.starts_with('s')` | Local state/value operations | Handled | [169](../../crates/crysirror-circus/src/dc3_controls.rs#L169) |
| `name` | `_ if name.starts_with('l')` | Local state/value operations | Handled | [170](../../crates/crysirror-circus/src/dc3_controls.rs#L170) |
| `name` | `_ if name.starts_with('h')` | Local state/value operations | Handled | [171](../../crates/crysirror-circus/src/dc3_controls.rs#L171) |
| `name` | `_ if name.starts_with('o')` | Local state/value operations | Handled | [172](../../crates/crysirror-circus/src/dc3_controls.rs#L172) |
| `name` | `_ if name.starts_with('b')` | Local state/value operations | Handled | [173](../../crates/crysirror-circus/src/dc3_controls.rs#L173) |
| `name` | `_ if name.starts_with('t')` | Local state/value operations | Handled | [174](../../crates/crysirror-circus/src/dc3_controls.rs#L174) |
| `name` | `_` | Local state/value operations | Unsupported: D.C.III 429B80 dialogue control action {action:?} is not connected | [175](../../crates/crysirror-circus/src/dc3_controls.rs#L175) |
| `name.as_bytes()` | `[b'n', ..]` | self.value, self.state.system_data.clone | Handled | [179](../../crates/crysirror-circus/src/dc3_controls.rs#L179) |
| `name.as_bytes()` | `[b'c', ..]` | self.set_window_hidden | Handled | [186](../../crates/crysirror-circus/src/dc3_controls.rs#L186) |
| `name.as_bytes()` | `[b'r', ..]` | self.toggle_skip | Handled | [187](../../crates/crysirror-circus/src/dc3_controls.rs#L187) |
| `name.as_bytes()` | `[b'o', ..]` | Local state/value operations | Handled | [191](../../crates/crysirror-circus/src/dc3_controls.rs#L191) |
| `name.as_bytes()` | `[b'd', b's', ..] if self.value(9, 7)? == 0` | Local state/value operations | Handled | [192](../../crates/crysirror-circus/src/dc3_controls.rs#L192) |
| `name.as_bytes()` | `[b'd', b'l', ..] if self.value(9, 7)? == 0` | Local state/value operations | Handled | [193](../../crates/crysirror-circus/src/dc3_controls.rs#L193) |
| `name.as_bytes()` | `[b'd', b's' \| b'l', ..]` | Local state/value operations | Handled | [194](../../crates/crysirror-circus/src/dc3_controls.rs#L194) |
| `name.as_bytes()` | `[b'a', b'a', ..]` | Local state/value operations | Unsupported: DC3WY 421AD0 action {action:?} is not connected | [195](../../crates/crysirror-circus/src/dc3_controls.rs#L195) |
| `name.as_bytes()` | `[b'a', ..]` | self.toggle_auto | Handled | [196](../../crates/crysirror-circus/src/dc3_controls.rs#L196) |
| `name.as_bytes()` | `_` | Local state/value operations | Unsupported: DC3WY 421AD0 dialogue control action {action:?} is not connected | [200](../../crates/crysirror-circus/src/dc3_controls.rs#L200) |

### dc3_effects.rs / Machine::execute_effects

```rust
fn execute_effects(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode == 0x0e && matches!(a[0], 0..=2)` | self.flat_fade | Handled | [152](../../crates/crysirror-circus/src/dc3_effects.rs#L152) |
| `condition` | `instruction.opcode == 0x0e && matches!(a[0], 4 \| 6 \| 51 \| 52)` | self.skip_for, self.shake_objects, self.advance | Handled | [159](../../crates/crysirror-circus/src/dc3_effects.rs#L159) |
| `condition` | `matches!(a[0], 4 \| 6)` | Local state/value operations | Handled | [161](../../crates/crysirror-circus/src/dc3_effects.rs#L161) |
| `condition` | `matches!(a[0], 4 \| 51)` | Local state/value operations | Handled | [171](../../crates/crysirror-circus/src/dc3_effects.rs#L171) |
| `condition` | `a[0] == 51` | Local state/value operations | Handled | [172](../../crates/crysirror-circus/src/dc3_effects.rs#L172) |
| `condition` | `instruction.opcode == 0x0e && matches!(a[0], 17 \| 20 \| 21)` | self.mask_fade | Handled | [179](../../crates/crysirror-circus/src/dc3_effects.rs#L179) |
| `condition` | `instruction.opcode == 0x0e && a[0] == 5` | self.advance | Handled | [184](../../crates/crysirror-circus/src/dc3_effects.rs#L184) |
| `condition` | `instruction.opcode == 0x0e && a[0] == 3` | self.value, host.effect_control, self.advance, Outcome::WaitUntil | Handled | [190](../../crates/crysirror-circus/src/dc3_effects.rs#L190) |
| `condition` | `self.value(99, 10)? != 0 && !host.effect_control(&EffectRequest::Flash { repeats: a[1].max(0) as u16 })?` | Local state/value operations | Handled | [192](../../crates/crysirror-circus/src/dc3_effects.rs#L192) |
| `condition` | `instruction.opcode == 0x0e && !matches!(a[0], 0..=7 \| 9..=11 \| 13..=17 \| 20 \| 21 \| 23..=25 \| 30..=32 \| 51 \| 52)` | self.advance, self.native_skipping, self.value, Outcome::WaitUntil | Handled | [207](../../crates/crysirror-circus/src/dc3_effects.rs#L207) |
| `condition` | `instruction.opcode != 0x0e \|\| !matches!(a[0], 15 \| 16)` | Local state/value operations | Handled | [216](../../crates/crysirror-circus/src/dc3_effects.rs#L216) |
| `condition` | `!host.emit(&Event::LoadImage { slot: 35, name: name.into() })?` | Local state/value operations | Handled | [224](../../crates/crysirror-circus/src/dc3_effects.rs#L224) |
| `condition` | `self.value(99, 10)? != 0 && !host.effect_control(&EffectRequest::TileMask { pattern: a[1], color: self.state.resources.transition_color, })?` | Local state/value operations | Handled | [228](../../crates/crysirror-circus/src/dc3_effects.rs#L228) |
| `condition` | `a[0] == 16` | host.emit, self.stop_fog, self.assign | Handled | [238](../../crates/crysirror-circus/src/dc3_effects.rs#L238) |

### dc3_effects.rs / Machine::execute_spawn_window

```rust
fn execute_spawn_window(&mut self, instruction: &Instruction, a: [i16; 8], host: &mut dyn NativeHost) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [274](../../crates/crysirror-circus/src/dc3_effects.rs#L274) |

### dc3_effects.rs / Machine::execute_weather

```rust
fn execute_weather(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x27` | Local state/value operations | Handled | [357](../../crates/crysirror-circus/src/dc3_effects.rs#L357) |
| `condition` | `a[0] == 7` | self.state.image_names.contains_key, self.publish_texture, self.advance | Handled | [362](../../crates/crysirror-circus/src/dc3_effects.rs#L362) |
| `condition` | `a[1] != 0` | Local state/value operations | Handled | [363](../../crates/crysirror-circus/src/dc3_effects.rs#L363) |
| `condition` | `a[0] == 16` | self.start_clouds, self.state.image_names.contains_key, self.publish_texture, self.advance | Handled | [377](../../crates/crysirror-circus/src/dc3_effects.rs#L377) |
| `condition` | `a[1] != 0` | self.start_clouds | Handled | [378](../../crates/crysirror-circus/src/dc3_effects.rs#L378) |
| `condition` | `a[0] == 0 && !self.stop_fog(host)?` | Local state/value operations | Handled | [411](../../crates/crysirror-circus/src/dc3_effects.rs#L411) |
| `condition` | `a[0] == 0` | Local state/value operations | Handled | [414](../../crates/crysirror-circus/src/dc3_effects.rs#L414) |
| `condition` | `a[0] == 2` | Local state/value operations | Handled | [420](../../crates/crysirror-circus/src/dc3_effects.rs#L420) |
| `condition` | `a[0] == 0` | Local state/value operations | Handled | [423](../../crates/crysirror-circus/src/dc3_effects.rs#L423) |

### dc3_effects.rs / Machine::flat_fade

```rust
fn flat_fade(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.skip_for(instruction.offset) && self.value(9, 99)? == 0` | self.advance | Handled | [69](../../crates/crysirror-circus/src/dc3_effects.rs#L69) |

### dc3_effects.rs / Machine::mask_fade

```rust
fn mask_fade(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `a[0] == 17` | self.publish_texture | Handled | [118](../../crates/crysirror-circus/src/dc3_effects.rs#L118) |
| `condition` | `a[1] == 1` | Local state/value operations | Handled | [119](../../crates/crysirror-circus/src/dc3_effects.rs#L119) |
| `condition` | `!self.publish_texture(host, 16, name)?` | Local state/value operations | Handled | [120](../../crates/crysirror-circus/src/dc3_effects.rs#L120) |
| `condition` | `a[0] == 17` | Local state/value operations | Handled | [126](../../crates/crysirror-circus/src/dc3_effects.rs#L126) |
| `condition` | `!host.effect_control(&EffectRequest::MaskFade { reverse: a[0] != 20, ticks, color })?` | Local state/value operations | Handled | [128](../../crates/crysirror-circus/src/dc3_effects.rs#L128) |

### dc3_load.rs / Machine::execute_load

```rust
fn execute_load(
        &mut self,
        instruction: &Instruction,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `progress.mode < 2 && !self.load_teardown(host)?` | Local state/value operations | Handled | [94](../../crates/crysirror-circus/src/dc3_load.rs#L94) |
| `condition` | `let Some([kind, density]) = self.state.rain.map(\|density\| [2, density]).or(self.state.weather).filter(\|_\| progress.mode < 2)` | host.emit | Handled | [126](../../crates/crysirror-circus/src/dc3_load.rs#L126) |
| `condition` | `self.state.clouds && progress.mode < 2 && !self.start_clouds(host)?` | Local state/value operations | Handled | [136](../../crates/crysirror-circus/src/dc3_load.rs#L136) |
| `condition` | `progress.mode == 0 && self.state.audio.music_name.is_empty() && !host.emit(&Event::StopAudio { channel: 0 })?` | Local state/value operations | Handled | [143](../../crates/crysirror-circus/src/dc3_load.rs#L143) |
| `condition` | `index == 0 && audio.channel_one_loop && !name.is_empty()` | Local state/value operations | Handled | [152](../../crates/crysirror-circus/src/dc3_load.rs#L152) |
| `condition` | `progress.mode == 1` | self.adjusted_ticks, host.effect_control | Handled | [170](../../crates/crysirror-circus/src/dc3_load.rs#L170) |
| `condition` | `progress.mode == 0` | Outcome::WaitUntil | Handled | [185](../../crates/crysirror-circus/src/dc3_load.rs#L185) |

### dc3_menu.rs / Machine::execute_menu

```rust
fn execute_menu(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `instruction.opcode` | `0x09 if a[0] == 0` | self.state.choice.as_ref, host.emit, Event::BeginMenu, self.advance | Handled | [94](../../crates/crysirror-circus/src/dc3_menu.rs#L94) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [112](../../crates/crysirror-circus/src/dc3_menu.rs#L112) |

### dc3_menu.rs / Machine::poll_menu

```rust
fn poll_menu(&mut self, program: &Program, host: &mut dyn NativeHost) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(target) = program .instructions .iter() .take_while(\|item\| item.opcode != 0) .find(\|item\| item.opcode == 0x44 && matches!(&item.arguments, Arguments::String(label) if *label == action))` | Local state/value operations | Handled | [124](../../crates/crysirror-circus/src/dc3_menu.rs#L124) |

### dc3_motion.rs / Machine::execute_motion

```rust
fn execute_motion(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode == 0x13 && a[0] == 254` | self.layer_fence | Handled | [53](../../crates/crysirror-circus/src/dc3_motion.rs#L53) |
| `condition` | `instruction.opcode == 0x35` | self.state.sprites.iter, self.object_seed, host.sprite, self.advance | Handled | [57](../../crates/crysirror-circus/src/dc3_motion.rs#L57) |
| `condition` | `matches!(instruction.opcode, 0x9d \| 0x9e)` | u16::try_from, self.object_seed, host.sprite, self.state.sprites.entry, self.advance | Handled | [74](../../crates/crysirror-circus/src/dc3_motion.rs#L74) |
| `condition` | `instruction.opcode == 0x93` | self.portrait_transition | Handled | [94](../../crates/crysirror-circus/src/dc3_motion.rs#L94) |
| `condition` | `instruction.opcode != 0x7f` | Local state/value operations | Handled | [97](../../crates/crysirror-circus/src/dc3_motion.rs#L97) |
| `a[2]` | `1` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [103](../../crates/crysirror-circus/src/dc3_motion.rs#L103) |
| `a[2]` | `0` | Local state/value operations | Handled | [104](../../crates/crysirror-circus/src/dc3_motion.rs#L104) |
| `a[2]` | `0x63` | i32::from | Handled | [105](../../crates/crysirror-circus/src/dc3_motion.rs#L105) |
| `a[2]` | `_` | i32::from | Handled | [106](../../crates/crysirror-circus/src/dc3_motion.rs#L106) |

### dc3_motion.rs / Machine::portrait_transition

```rust
fn portrait_transition(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.camera_transformed() && a[1] == 100` | self.advance | Handled | [206](../../crates/crysirror-circus/src/dc3_motion.rs#L206) |
| `condition` | `a[2] != -1 \|\| a[3] != -1` | i32::from, self.camera_zoom | Handled | [224](../../crates/crysirror-circus/src/dc3_motion.rs#L224) |
| `condition` | `!self.camera_zoom(host, i32::from(a[1]), x, y)?` | Local state/value operations | Handled | [227](../../crates/crysirror-circus/src/dc3_motion.rs#L227) |
| `condition` | `a[1] == 100` | Local state/value operations | Handled | [231](../../crates/crysirror-circus/src/dc3_motion.rs#L231) |

### dc3_movie.rs / Machine::execute_movie

```rust
fn execute_movie(
        &mut self,
        i: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode != 0x4b` | Local state/value operations | Handled | [24](../../crates/crysirror-circus/src/dc3_movie.rs#L24) |
| `condition` | `!host.emit(&Event::ReleaseAudioGroup { kind: pending.phase - 1, forced: true, steps: 0, })?` | Local state/value operations | Handled | [72](../../crates/crysirror-circus/src/dc3_movie.rs#L72) |

### dc3_music_policy.rs / Machine::execute_music

```rust
fn execute_music(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<bool>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x67 && !(instruction.opcode == 0x05 && a[0] != 254)` | Local state/value operations | Handled | [12](../../crates/crysirror-circus/src/dc3_music_policy.rs#L12) |
| `condition` | `instruction.opcode == 0x67 \|\| a[0] == 255` | self.state.system_data.clone, self.publish_music_volume | Handled | [15](../../crates/crysirror-circus/src/dc3_music_policy.rs#L15) |
| `condition` | `instruction.opcode == 0x05` | Local state/value operations | Handled | [25](../../crates/crysirror-circus/src/dc3_music_policy.rs#L25) |
| `condition` | `a[0] == 255` | host.emit, self.state.audio.pending.remove, self.state.audio.music_name.clear | Handled | [48](../../crates/crysirror-circus/src/dc3_music_policy.rs#L48) |
| `condition` | `a[1] != 1 \|\| self.state.audio.music_index != i32::from(a[0])` | self.state.audio.pending.insert | Handled | [54](../../crates/crysirror-circus/src/dc3_music_policy.rs#L54) |

### dc3_original.rs / map

```rust
fn map(opcode: u8) -> Option<u8>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [87](../../crates/crysirror-circus/src/dc3_original.rs#L87) |

### dc3_original_effects.rs / Machine::begin_original_effect

```rust
fn begin_original_effect(&mut self, e: &mut OriginalEffect, host: &mut dyn NativeHost) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.emit(&Event::ReleaseAudioGroup { kind, forced: true, steps: 0 })?` | Local state/value operations | Handled | [234](../../crates/crysirror-circus/src/dc3_original_effects.rs#L234) |
| `e.selector` | `16 if !effects` | Local state/value operations | Handled | [242](../../crates/crysirror-circus/src/dc3_original_effects.rs#L242) |
| `e.selector` | `17 if skipping` | Local state/value operations | Handled | [243](../../crates/crysirror-circus/src/dc3_original_effects.rs#L243) |
| `e.selector` | `17` | Local state/value operations | Handled | [244](../../crates/crysirror-circus/src/dc3_original_effects.rs#L244) |
| `e.selector` | `21 if skipping \|\| !effects \|\| !self.state.image_names.contains_key(&MASK_SLOT)` | Local state/value operations | Handled | [245](../../crates/crysirror-circus/src/dc3_original_effects.rs#L245) |
| `e.selector` | `21` | self.original_speed, usize::try_from | Handled | [246](../../crates/crysirror-circus/src/dc3_original_effects.rs#L246) |
| `e.selector` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [255](../../crates/crysirror-circus/src/dc3_original_effects.rs#L255) |
| `condition` | `e.selector == 16 && !self.publish_texture(host, CRACK_SLOT, "SYSTEM\\EFF")?` | Local state/value operations | Handled | [258](../../crates/crysirror-circus/src/dc3_original_effects.rs#L258) |
| `condition` | `e.selector == 17` | Local state/value operations | Handled | [261](../../crates/crysirror-circus/src/dc3_original_effects.rs#L261) |

### dc3_original_effects.rs / Machine::execute_original_effects

```rust
fn execute_original_effects(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.state.original \|\| instruction.opcode != 0x0e` | Local state/value operations | Handled | [137](../../crates/crysirror-circus/src/dc3_original_effects.rs#L137) |
| `condition` | `matches!(a[0], 5 \| 7 \| 9 \| 10 \| 11 \| 13 \| 14 \| 30 \| 31 \| 32 \| 51 \| 52)` | Local state/value operations | Handled | [141](../../crates/crysirror-circus/src/dc3_original_effects.rs#L141) |

### dc3_original_effects.rs / Machine::original_call

```rust
fn original_call(&mut self, e: &mut OriginalEffect) -> Result<After>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under e.selector=1 \| 2` | `e.selector == 1` | Local state/value operations | Handled | [330](../../crates/crysirror-circus/src/dc3_original_effects.rs#L330) |
| `condition; under e.selector=1 \| 2` | `e.selector == 1` | Local state/value operations | Handled | [334](../../crates/crysirror-circus/src/dc3_original_effects.rs#L334) |
| `condition; under e.selector=4 \| 6` | `e.selector == 4` | Local state/value operations | Handled | [363](../../crates/crysirror-circus/src/dc3_original_effects.rs#L363) |
| `e.selector` | `0` | self.original_speed, After::Wait | Handled | [300](../../crates/crysirror-circus/src/dc3_original_effects.rs#L300) |
| `e.selector` | `1 \| 2` | After::ResetWait, After::Wait | Handled | [323](../../crates/crysirror-circus/src/dc3_original_effects.rs#L323) |
| `e.selector` | `3` | self.original_palette, After::Wait | Handled | [338](../../crates/crysirror-circus/src/dc3_original_effects.rs#L338) |
| `e.selector` | `4 \| 6` | self.original_speed, After::Wait | Handled | [351](../../crates/crysirror-circus/src/dc3_original_effects.rs#L351) |
| `e.selector` | `16` | self.original_tile_field, Vec::new, After::Wait | Handled | [367](../../crates/crysirror-circus/src/dc3_original_effects.rs#L367) |
| `e.selector` | `17` | self.original_speed, i64::from, After::Wait | Handled | [395](../../crates/crysirror-circus/src/dc3_original_effects.rs#L395) |
| `e.selector` | `21` | After::Wait | Handled | [417](../../crates/crysirror-circus/src/dc3_original_effects.rs#L417) |
| `e.selector` | `_` | Local state/value operations | Handled | [438](../../crates/crysirror-circus/src/dc3_original_effects.rs#L438) |

### dc3_original_effects.rs / Machine::original_effect_ahead

```rust
fn original_effect_ahead(&self, program: &Program) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(program.original_opcode(instruction), &instruction.arguments)` | `(Some(0x0e), Arguments::Bytes([selector, _]))` | Self::original_reads_presented, i16::from | Handled | [102](../../crates/crysirror-circus/src/dc3_original_effects.rs#L102) |
| `(program.original_opcode(instruction), &instruction.arguments)` | `(Some(0x00 \| 0x07 \| 0x0f), _) \| (None, _)` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [108](../../crates/crysirror-circus/src/dc3_original_effects.rs#L108) |
| `(program.original_opcode(instruction), &instruction.arguments)` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [109](../../crates/crysirror-circus/src/dc3_original_effects.rs#L109) |

### dc3_original_effects.rs / Machine::original_reads_presented

```rust
fn original_reads_presented(selector: i16) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(selector, 0 \| 2 \| 3 \| 4 \| 6 \| 16 \| 17 \| 20 \| 21)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [90](../../crates/crysirror-circus/src/dc3_original_effects.rs#L90) |

### dc3_original_effects.rs / Machine::original_tile_field

```rust
fn original_tile_field(&mut self, kind: i32) -> Result<TileScatter>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0` | Local state/value operations | Handled | [464](../../crates/crysirror-circus/src/dc3_original_effects.rs#L464) |
| `kind` | `1` | Local state/value operations | Handled | [473](../../crates/crysirror-circus/src/dc3_original_effects.rs#L473) |
| `kind` | `2` | Local state/value operations | Handled | [482](../../crates/crysirror-circus/src/dc3_original_effects.rs#L482) |
| `kind` | `3` | Local state/value operations | Handled | [492](../../crates/crysirror-circus/src/dc3_original_effects.rs#L492) |
| `kind` | `4` | Local state/value operations | Handled | [502](../../crates/crysirror-circus/src/dc3_original_effects.rs#L502) |
| `kind` | `5` | Local state/value operations | Handled | [511](../../crates/crysirror-circus/src/dc3_original_effects.rs#L511) |
| `kind` | `6` | Local state/value operations | Handled | [521](../../crates/crysirror-circus/src/dc3_original_effects.rs#L521) |
| `kind` | `9` | Local state/value operations | Handled | [531](../../crates/crysirror-circus/src/dc3_original_effects.rs#L531) |
| `kind` | `10` | Local state/value operations | Handled | [541](../../crates/crysirror-circus/src/dc3_original_effects.rs#L541) |
| `kind` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [547](../../crates/crysirror-circus/src/dc3_original_effects.rs#L547) |

### dc3_original_effects.rs / Machine::run_original_effect

```rust
fn run_original_effect(
        &mut self,
        e: &mut OriginalEffect,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `e.selector` | `16` | Local state/value operations | Handled | [195](../../crates/crysirror-circus/src/dc3_original_effects.rs#L195) |
| `e.selector` | `21` | Local state/value operations | Handled | [196](../../crates/crysirror-circus/src/dc3_original_effects.rs#L196) |
| `e.selector` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [197](../../crates/crysirror-circus/src/dc3_original_effects.rs#L197) |

### dc3_original_weather.rs / Machine::execute_original_pan_target

```rust
fn execute_original_pan_target(&mut self, program: &Program, instruction: &Instruction, a: [i16; 8]) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.state.original \|\| instruction.opcode != 0x7f \|\| program.original_opcode(instruction) != Some(0x69) \|\| a[2] != 2` | Local state/value operations | Handled | [354](../../crates/crysirror-circus/src/dc3_original_weather.rs#L354) |

### dc3_original_weather.rs / Machine::execute_original_weather

```rust
fn execute_original_weather(&mut self, instruction: &Instruction, a: [i16; 8], host: &mut dyn NativeHost) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.state.original \|\| instruction.opcode != 0x27` | Local state/value operations | Handled | [362](../../crates/crysirror-circus/src/dc3_original_weather.rs#L362) |
| `kind` | `0` | self.original_fog, self.finish_original_pan, Default::default | Handled | [387](../../crates/crysirror-circus/src/dc3_original_weather.rs#L387) |
| `kind` | `1 \| 3 \| 4 \| 9 \| 11` | self.publish_pending_slot | Handled | [396](../../crates/crysirror-circus/src/dc3_original_weather.rs#L396) |
| `kind` | `2` | self.publish_pending_slot | Handled | [415](../../crates/crysirror-circus/src/dc3_original_weather.rs#L415) |
| `kind` | `7` | self.original_fog | Handled | [433](../../crates/crysirror-circus/src/dc3_original_weather.rs#L433) |
| `kind` | `6` | self.state.original_pan.context, u32::try_from, i64::from, host.sprite, self.state.sprites.entry | Handled | [439](../../crates/crysirror-circus/src/dc3_original_weather.rs#L439) |
| `kind` | `_` | Local state/value operations | Unsupported: D.C.III 0x27 {kind} ({parameter}) is not connected (40AB30 default branch) | [466](../../crates/crysirror-circus/src/dc3_original_weather.rs#L466) |

### dc3_original_weather.rs / Machine::finish_original_pan

```rust
fn finish_original_pan(&mut self, host: &mut dyn NativeHost) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.sprite(&SpriteRequest { commands: vec![SpriteCommand { seed: self.object_seed(0), operations: vec![SequenceOperation::Finish { cancel: false, lanes: None }] }], })?` | Local state/value operations | Handled | [482](../../crates/crysirror-circus/src/dc3_original_weather.rs#L482) |

### dc3_original_weather.rs / Machine::original_fog

```rust
fn original_fog(&mut self, host: &mut dyn NativeHost, mode: i32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | self.publish_texture, self.stop_fog | Handled | [502](../../crates/crysirror-circus/src/dc3_original_weather.rs#L502) |
| `condition` | `mode == 3` | u32::try_from, self.start_cloud_scroll | Handled | [513](../../crates/crysirror-circus/src/dc3_original_weather.rs#L513) |

### dc3_original_weather.rs / OriginalWeather::start_petals

```rust
fn start_petals(&mut self, kind: i32, parameter: i32, rng: &mut crysirror_vfx::xorshift::Xorshift128) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `3` | Local state/value operations | Handled | [154](../../crates/crysirror-circus/src/dc3_original_weather.rs#L154) |
| `kind` | `4` | Local state/value operations | Handled | [159](../../crates/crysirror-circus/src/dc3_original_weather.rs#L159) |
| `kind` | `11` | Local state/value operations | Handled | [160](../../crates/crysirror-circus/src/dc3_original_weather.rs#L160) |
| `kind` | `9` | Local state/value operations | Handled | [161](../../crates/crysirror-circus/src/dc3_original_weather.rs#L161) |
| `kind` | `_` | Local state/value operations | Handled | [162](../../crates/crysirror-circus/src/dc3_original_weather.rs#L162) |
| `kind` | `4` | Local state/value operations | Handled | [168](../../crates/crysirror-circus/src/dc3_original_weather.rs#L168) |
| `kind` | `11` | Local state/value operations | Handled | [169](../../crates/crysirror-circus/src/dc3_original_weather.rs#L169) |
| `kind` | `_` | Local state/value operations | Handled | [170](../../crates/crysirror-circus/src/dc3_original_weather.rs#L170) |
| `mode` | `1` | Local state/value operations | Handled | [183](../../crates/crysirror-circus/src/dc3_original_weather.rs#L183) |
| `mode` | `2` | Local state/value operations | Handled | [184](../../crates/crysirror-circus/src/dc3_original_weather.rs#L184) |
| `mode` | `_` | Local state/value operations | Handled | [185](../../crates/crysirror-circus/src/dc3_original_weather.rs#L185) |

### dc3_original_weather.rs / OriginalWeather::start_rain

```rust
fn start_rain(&mut self, mode: i32, rng: &mut crysirror_vfx::xorshift::Xorshift128) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `1` | Local state/value operations | Handled | [258](../../crates/crysirror-circus/src/dc3_original_weather.rs#L258) |
| `mode` | `2` | Local state/value operations | Handled | [259](../../crates/crysirror-circus/src/dc3_original_weather.rs#L259) |
| `mode` | `3` | Local state/value operations | Handled | [260](../../crates/crysirror-circus/src/dc3_original_weather.rs#L260) |
| `mode` | `_` | Local state/value operations | Handled | [261](../../crates/crysirror-circus/src/dc3_original_weather.rs#L261) |
| `condition; under mode=_` | `mode == 2` | Local state/value operations | Handled | [270](../../crates/crysirror-circus/src/dc3_original_weather.rs#L270) |
| `mode` | `1` | Local state/value operations | Handled | [268](../../crates/crysirror-circus/src/dc3_original_weather.rs#L268) |
| `mode` | `_` | Local state/value operations | Handled | [269](../../crates/crysirror-circus/src/dc3_original_weather.rs#L269) |

### dc3_original_weather.rs / OriginalWeather::step_rain

```rust
fn step_rain(&mut self, rng: &mut crysirror_vfx::xorshift::Xorshift128) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(mode, d.variant)` | `(1, _) \| (_, 0)` | Local state/value operations | Handled | [284](../../crates/crysirror-circus/src/dc3_original_weather.rs#L284) |
| `(mode, d.variant)` | `(_, 1)` | Local state/value operations | Handled | [285](../../crates/crysirror-circus/src/dc3_original_weather.rs#L285) |
| `(mode, d.variant)` | `(3, 2)` | Local state/value operations | Handled | [286](../../crates/crysirror-circus/src/dc3_original_weather.rs#L286) |
| `(mode, d.variant)` | `_` | Local state/value operations | Handled | [287](../../crates/crysirror-circus/src/dc3_original_weather.rs#L287) |
| `condition` | `mode == 1` | Local state/value operations | Handled | [292](../../crates/crysirror-circus/src/dc3_original_weather.rs#L292) |
| `condition` | `mode == 2` | Local state/value operations | Handled | [299](../../crates/crysirror-circus/src/dc3_original_weather.rs#L299) |

### dc3_portrait.rs / tint_rewrite

```rust
fn tint_rewrite(name: &str, mode: i32) -> String
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 0` | Local state/value operations | Handled | [238](../../crates/crysirror-circus/src/dc3_portrait.rs#L238) |
| `(name.find(','), mode)` | `(Some(comma), 1)` | Local state/value operations | Handled | [252](../../crates/crysirror-circus/src/dc3_portrait.rs#L252) |
| `(name.find(','), mode)` | `(Some(comma), 2)` | Local state/value operations | Handled | [253](../../crates/crysirror-circus/src/dc3_portrait.rs#L253) |
| `(name.find(','), mode)` | `_` | Local state/value operations | Handled | [254](../../crates/crysirror-circus/src/dc3_portrait.rs#L254) |

### dc3_portrait.rs / Machine::execute_portrait

```rust
fn execute_portrait(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under instruction.opcode=0x81 if (a[3] as u8) & 0x80 != 0` | `!self.portrait_mask(host, object, false, 12 * i32::from(a[1]))?` | Local state/value operations | Handled | [91](../../crates/crysirror-circus/src/dc3_portrait.rs#L91) |
| `condition; under instruction.opcode=0x81 if (a[3] as u8) & 0x40 != 0 / a[0]=0..=2` | `a[0] < 2` | Local state/value operations | Handled | [117](../../crates/crysirror-circus/src/dc3_portrait.rs#L117) |
| `condition; under instruction.opcode=0x81 if (a[3] as u8) & 0x40 != 0 / a[0]=0..=2` | `a[0] == 0` | Local state/value operations | Handled | [118](../../crates/crysirror-circus/src/dc3_portrait.rs#L118) |
| `condition; under instruction.opcode=0x81 if (a[3] as u8) & 0x40 != 0 / a[0]=3` | `!self.portrait_mask(host, object, true, 12 * i32::from(a[1]))?` | Local state/value operations | Handled | [127](../../crates/crysirror-circus/src/dc3_portrait.rs#L127) |
| `a[0]; under instruction.opcode=0x81 if (a[3] as u8) & 0x40 != 0` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [131](../../crates/crysirror-circus/src/dc3_portrait.rs#L131) |
| `condition; under instruction.opcode=0x84` | `a[3] != 0` | i32::from | Handled | [140](../../crates/crysirror-circus/src/dc3_portrait.rs#L140) |
| `instruction.opcode` | `0x81 if a[3] == 255` | Local state/value operations | Empty handler: Original no-op status not established | [51](../../crates/crysirror-circus/src/dc3_portrait.rs#L51) |
| `instruction.opcode` | `0x81 if a[3] == 0` | u16::try_from, i32::from, self.state.sprites.get, self.portrait_operations, self.state.sprites.entry | Handled | [52](../../crates/crysirror-circus/src/dc3_portrait.rs#L52) |
| `instruction.opcode` | `0x81 if (a[3] as u8) & 0x80 != 0` | u16::from, self.skip_for, self.state.image_names.contains_key, self.publish_texture, self.state.resources.pending.insert | Handled | [80](../../crates/crysirror-circus/src/dc3_portrait.rs#L80) |
| `instruction.opcode` | `0x81 if (a[3] as u8) & 0x40 != 0` | u16::from, self.skip_for, self.state.image_names.contains_key, self.publish_texture, self.state.sprites.get | Handled | [100](../../crates/crysirror-circus/src/dc3_portrait.rs#L100) |
| `instruction.opcode` | `0x84 if (a[2] as u8) & 0x10 != 0` | Local state/value operations | Empty handler: Original no-op status not established | [137](../../crates/crysirror-circus/src/dc3_portrait.rs#L137) |
| `instruction.opcode` | `0x81 if a[3] & 0x20 != 0` | u16::try_from, host.visual, self.state.sprites.entry | Handled | [155](../../crates/crysirror-circus/src/dc3_portrait.rs#L155) |
| `instruction.opcode` | `0x81 if a[3] & 0x10 != 0 && self.state.camera` | Local state/value operations | Empty handler: Original no-op status not established | [164](../../crates/crysirror-circus/src/dc3_portrait.rs#L164) |
| `instruction.opcode` | `0x81 if a[3] & 0x10 != 0` | u16::try_from, f32::from, self.state.sprites.get, self.portrait_operations, self.state.sprites.entry | Handled | [165](../../crates/crysirror-circus/src/dc3_portrait.rs#L165) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [203](../../crates/crysirror-circus/src/dc3_portrait.rs#L203) |

### dc3_portrait.rs / Machine::settle_portrait

```rust
fn settle_portrait(&mut self, host: &mut dyn NativeHost, slot: u8, object: u16, name: &str, logical: [u32; 2]) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `upper.starts_with("BST\\MRK") && !name.contains(',')` | i32::try_from | Handled | [225](../../crates/crysirror-circus/src/dc3_portrait.rs#L225) |

### dc3_reset.rs / Machine::execute_reset

```rust
fn execute_reset(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x10` | Local state/value operations | Handled | [10](../../crates/crysirror-circus/src/dc3_reset.rs#L10) |
| `condition` | `a[0] == 0` | self.state.script_bytes.fill | Handled | [24](../../crates/crysirror-circus/src/dc3_reset.rs#L24) |

### dc3_resources.rs / b_rewrite

```rust
fn b_rewrite(name: &str) -> String
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `name.contains("_B,")` | Local state/value operations | Handled | [443](../../crates/crysirror-circus/src/dc3_resources.rs#L443) |

### dc3_resources.rs / Machine::execute_resources

```rust
fn execute_resources(
        &mut self,
        program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under i.opcode=0x3a if matches!(a[0], 20 \| 21)` | `!self.publish_texture(host, a[0] as u8, text)?` | Local state/value operations | Handled | [128](../../crates/crysirror-circus/src/dc3_resources.rs#L128) |
| `condition; under i.opcode=0x14` | `following.opcode != 0x14` | Local state/value operations | Handled | [203](../../crates/crysirror-circus/src/dc3_resources.rs#L203) |
| `i.opcode` | `0x3a if a[0] == 1` | self.state.sprites.get, self.camera_zoom, self.reset_background_actor, self.state.sprites.insert, Sprite::default | Handled | [33](../../crates/crysirror-circus/src/dc3_resources.rs#L33) |
| `i.opcode` | `0x3a if (14..=18).contains(&a[0])` | self.value, char::from, host.emit, self.state.image_names.insert | Handled | [79](../../crates/crysirror-circus/src/dc3_resources.rs#L79) |
| `i.opcode` | `0x3a if !matches!(a[0], 0 \| 1 \| 14..=18 \| 20 \| 21)` | self.value, char::from, self.state.resources.pending.insert | Partial: DC3WY 0x3A object slot {slot} (record copy) is not connected | [99](../../crates/crysirror-circus/src/dc3_resources.rs#L99) |
| `i.opcode` | `0x3a if matches!(a[0], 20 \| 21)` | self.publish_texture | Handled | [127](../../crates/crysirror-circus/src/dc3_resources.rs#L127) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [216](../../crates/crysirror-circus/src/dc3_resources.rs#L216) |

### dc3_resources.rs / Machine::release_scene_resources

```rust
fn release_scene_resources(&mut self, host: &mut dyn NativeHost) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.emit(&Event::FallingEffect { kind: 0, parameter: 0 })?` | Local state/value operations | Handled | [417](../../crates/crysirror-circus/src/dc3_resources.rs#L417) |

### dc3_scene.rs / Machine::execute_scene

```rust
fn execute_scene(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `instruction.opcode` | `0x3a if a[0] == 0` | self.advance | Handled | [15](../../crates/crysirror-circus/src/dc3_scene.rs#L15) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [19](../../crates/crysirror-circus/src/dc3_scene.rs#L19) |
| `condition` | `instruction.opcode == 0x49` | Local state/value operations | Handled | [40](../../crates/crysirror-circus/src/dc3_scene.rs#L40) |
| `condition` | `instruction.opcode == 0x49 \|\| scene >= 0` | Local state/value operations | Handled | [78](../../crates/crysirror-circus/src/dc3_scene.rs#L78) |
| `condition` | `instruction.opcode == 0x48` | Local state/value operations | Handled | [81](../../crates/crysirror-circus/src/dc3_scene.rs#L81) |
| `condition` | `let Some(label) = target .instructions .iter() .take_while(\|item\| item.opcode != 0) .find(\|item\| { item.opcode == 0x44 && target.literal_bytes(item.offset) == Some(deferred_jump.as_slice()) })` | Local state/value operations | Handled | [92](../../crates/crysirror-circus/src/dc3_scene.rs#L92) |
| `condition` | `instruction.opcode == 0x48 && !self.release_scene_resources(host)?` | Local state/value operations | Handled | [110](../../crates/crysirror-circus/src/dc3_scene.rs#L110) |
| `condition` | `instruction.opcode == 0x48 && !host.emit(&Event::ResetScene)?` | Local state/value operations | Handled | [113](../../crates/crysirror-circus/src/dc3_scene.rs#L113) |
| `condition` | `instruction.opcode == 0x48` | self.state.audio.last_voice.clear, Builder::default, self.state.sprites.values_mut | Handled | [119](../../crates/crysirror-circus/src/dc3_scene.rs#L119) |
| `condition` | `instruction.opcode == 0x49` | Local state/value operations | Handled | [129](../../crates/crysirror-circus/src/dc3_scene.rs#L129) |

### dc3_settings.rs / prefix_index

```rust
fn prefix_index(selector: i16) -> Option<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector` | `5` | Local state/value operations | Handled | [370](../../crates/crysirror-circus/src/dc3_settings.rs#L370) |
| `selector` | `7` | Local state/value operations | Handled | [371](../../crates/crysirror-circus/src/dc3_settings.rs#L371) |
| `selector` | `8` | Local state/value operations | Handled | [372](../../crates/crysirror-circus/src/dc3_settings.rs#L372) |
| `selector` | `9` | Local state/value operations | Handled | [373](../../crates/crysirror-circus/src/dc3_settings.rs#L373) |
| `selector` | `10` | Local state/value operations | Handled | [374](../../crates/crysirror-circus/src/dc3_settings.rs#L374) |
| `selector` | `12` | Local state/value operations | Handled | [375](../../crates/crysirror-circus/src/dc3_settings.rs#L375) |
| `selector` | `13` | Local state/value operations | Handled | [376](../../crates/crysirror-circus/src/dc3_settings.rs#L376) |
| `selector` | `14` | Local state/value operations | Handled | [377](../../crates/crysirror-circus/src/dc3_settings.rs#L377) |
| `selector` | `15` | Local state/value operations | Handled | [378](../../crates/crysirror-circus/src/dc3_settings.rs#L378) |
| `selector` | `16` | Local state/value operations | Handled | [379](../../crates/crysirror-circus/src/dc3_settings.rs#L379) |
| `selector` | `17` | Local state/value operations | Handled | [380](../../crates/crysirror-circus/src/dc3_settings.rs#L380) |
| `selector` | `18` | Local state/value operations | Handled | [381](../../crates/crysirror-circus/src/dc3_settings.rs#L381) |
| `selector` | `26` | Local state/value operations | Handled | [382](../../crates/crysirror-circus/src/dc3_settings.rs#L382) |
| `selector` | `27` | Local state/value operations | Handled | [383](../../crates/crysirror-circus/src/dc3_settings.rs#L383) |
| `selector` | `28` | Local state/value operations | Handled | [384](../../crates/crysirror-circus/src/dc3_settings.rs#L384) |
| `selector` | `29` | Local state/value operations | Handled | [385](../../crates/crysirror-circus/src/dc3_settings.rs#L385) |
| `selector` | `30` | Local state/value operations | Handled | [386](../../crates/crysirror-circus/src/dc3_settings.rs#L386) |
| `selector` | `190` | Local state/value operations | Handled | [387](../../crates/crysirror-circus/src/dc3_settings.rs#L387) |
| `selector` | `191` | Local state/value operations | Handled | [388](../../crates/crysirror-circus/src/dc3_settings.rs#L388) |
| `selector` | `194` | Local state/value operations | Handled | [389](../../crates/crysirror-circus/src/dc3_settings.rs#L389) |
| `selector` | `196` | Local state/value operations | Handled | [390](../../crates/crysirror-circus/src/dc3_settings.rs#L390) |
| `selector` | `197` | Local state/value operations | Handled | [391](../../crates/crysirror-circus/src/dc3_settings.rs#L391) |
| `selector` | `198` | Local state/value operations | Handled | [392](../../crates/crysirror-circus/src/dc3_settings.rs#L392) |
| `selector` | `199` | Local state/value operations | Handled | [393](../../crates/crysirror-circus/src/dc3_settings.rs#L393) |
| `selector` | `200` | Local state/value operations | Handled | [394](../../crates/crysirror-circus/src/dc3_settings.rs#L394) |
| `selector` | `201` | Local state/value operations | Handled | [395](../../crates/crysirror-circus/src/dc3_settings.rs#L395) |
| `selector` | `202` | Local state/value operations | Handled | [396](../../crates/crysirror-circus/src/dc3_settings.rs#L396) |
| `selector` | `203` | Local state/value operations | Handled | [397](../../crates/crysirror-circus/src/dc3_settings.rs#L397) |
| `selector` | `205` | Local state/value operations | Handled | [398](../../crates/crysirror-circus/src/dc3_settings.rs#L398) |
| `selector` | `206` | Local state/value operations | Handled | [399](../../crates/crysirror-circus/src/dc3_settings.rs#L399) |
| `selector` | `207` | Local state/value operations | Handled | [400](../../crates/crysirror-circus/src/dc3_settings.rs#L400) |
| `selector` | `208` | Local state/value operations | Handled | [401](../../crates/crysirror-circus/src/dc3_settings.rs#L401) |
| `selector` | `209` | Local state/value operations | Handled | [402](../../crates/crysirror-circus/src/dc3_settings.rs#L402) |
| `selector` | `210` | Local state/value operations | Handled | [403](../../crates/crysirror-circus/src/dc3_settings.rs#L403) |
| `selector` | `211` | Local state/value operations | Handled | [404](../../crates/crysirror-circus/src/dc3_settings.rs#L404) |
| `selector` | `212` | Local state/value operations | Handled | [405](../../crates/crysirror-circus/src/dc3_settings.rs#L405) |
| `selector` | `213` | Local state/value operations | Handled | [406](../../crates/crysirror-circus/src/dc3_settings.rs#L406) |
| `selector` | `214` | Local state/value operations | Handled | [407](../../crates/crysirror-circus/src/dc3_settings.rs#L407) |
| `selector` | `218` | Local state/value operations | Handled | [408](../../crates/crysirror-circus/src/dc3_settings.rs#L408) |
| `selector` | `221` | Local state/value operations | Handled | [409](../../crates/crysirror-circus/src/dc3_settings.rs#L409) |
| `selector` | `225` | Local state/value operations | Handled | [410](../../crates/crysirror-circus/src/dc3_settings.rs#L410) |
| `selector` | `227` | Local state/value operations | Handled | [411](../../crates/crysirror-circus/src/dc3_settings.rs#L411) |
| `selector` | `237` | Local state/value operations | Handled | [412](../../crates/crysirror-circus/src/dc3_settings.rs#L412) |
| `selector` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [413](../../crates/crysirror-circus/src/dc3_settings.rs#L413) |

### dc3_settings.rs / SystemData::prefix_value

```rust
fn prefix_value(&self, selector: i16) -> Result<Option<i32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 217` | self.checkbox, self.dword | Handled | [64](../../crates/crysirror-circus/src/dc3_settings.rs#L64) |
| `condition` | `selector == 11` | self.dword | Handled | [67](../../crates/crysirror-circus/src/dc3_settings.rs#L67) |

### dc3_settings.rs / SystemData::register_source

```rust
fn register_source(&mut self, name: &[u8], count: &mut u8) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.0[at..at + name.len()].eq_ignore_ascii_case(name)` | Local state/value operations | Handled | [135](../../crates/crysirror-circus/src/dc3_settings.rs#L135) |

### dc3_settings.rs / SystemData::set_prefix_value

```rust
fn set_prefix_value(&mut self, selector: i16, value: i32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 217` | self.set_checkbox, self.dword | Handled | [75](../../crates/crysirror-circus/src/dc3_settings.rs#L75) |
| `condition` | `selector == 11` | self.put_dword | Handled | [79](../../crates/crysirror-circus/src/dc3_settings.rs#L79) |

### dc3_system.rs / Machine::execute_system

```rust
fn execute_system(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<bool>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `instruction.opcode != 0x16` | Local state/value operations | Handled | [21](../../crates/crysirror-circus/src/dc3_system.rs#L21) |
| `condition; under a[0]=4` | `a[1] != 0` | Local state/value operations | Handled | [46](../../crates/crysirror-circus/src/dc3_system.rs#L46) |
| `a[1]; under a[0]=18` | `_` | self.state.preference_backup.clone, self.state.system_data.bytes, crate::dc3_settings::SystemData::try_from | Handled | [70](../../crates/crysirror-circus/src/dc3_system.rs#L70) |
| `condition; under a[0]=5 \| 6` | `a[0] == 5 && !host.write_profile_system(Profile::Dc3WithYou, data.bytes())?` | Local state/value operations | Handled | [103](../../crates/crysirror-circus/src/dc3_system.rs#L103) |
| `a[0]` | `selector if !matches!( selector, 0 \| 2 \| 3 \| 4 \| 5 \| 6 \| 7 \| 8 \| 10 \| 15 \| 18 \| 20 \| 22 \| 23 \| 24 \| 25 \| 26 \| 27 )` | Local state/value operations | Empty handler: Original no-op status not established | [183](../../crates/crysirror-circus/src/dc3_system.rs#L183) |
| `a[0]` | `selector` | Local state/value operations | Unsupported: unsupported original DC3WY system selector {selector}; native call remains unacknowledged | [188](../../crates/crysirror-circus/src/dc3_system.rs#L188) |

### dc3_text.rs / parse_markup

```rust
fn parse_markup(text: &str, names: &[String; 2], speed: i32) -> Result<(String, Vec<TextEvent>)>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `events[index - 1].at_ms == events[index].at_ms && events[index - 1].kind == events[index].kind` | Local state/value operations | Handled | [155](../../crates/crysirror-circus/src/dc3_text.rs#L155) |

### dc3_text.rs / Machine::execute_text

```rust
fn execute_text(
        &mut self,
        instruction: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [240](../../crates/crysirror-circus/src/dc3_text.rs#L240) |

### dc3_text.rs / Machine::run_text_event

```rust
fn run_text_event(&mut self, host: &mut dyn NativeHost, event: &TextEvent) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under event.kind=13..=20` | `event.kind <= 16` | Local state/value operations | Handled | [341](../../crates/crysirror-circus/src/dc3_text.rs#L341) |
| `condition; under event.kind=1..=5` | `!self.swap_object_texture(host, 1, 0, &name)?` | Local state/value operations | Handled | [362](../../crates/crysirror-circus/src/dc3_text.rs#L362) |
| `condition; under event.kind=29 \| 30` | `event.kind == 30` | Local state/value operations | Handled | [377](../../crates/crysirror-circus/src/dc3_text.rs#L377) |
| `event.kind` | `12` | host.sound_name, u16::try_from, host.publish_audio | Handled | [330](../../crates/crysirror-circus/src/dc3_text.rs#L330) |
| `event.kind` | `13..=20` | self.native_skipping, u16::from, self.shake_objects | Handled | [336](../../crates/crysirror-circus/src/dc3_text.rs#L336) |
| `event.kind` | `6..=10` | crate::dc3::slot::resource_object, self.state.image_names.get, self.swap_object_texture | Handled | [346](../../crates/crysirror-circus/src/dc3_text.rs#L346) |
| `event.kind` | `1..=5` | self.state.image_names.get, host.resource, self.swap_object_texture | Handled | [354](../../crates/crysirror-circus/src/dc3_text.rs#L354) |
| `event.kind` | `29 \| 30` | self.native_skipping, self.shake_objects | Handled | [372](../../crates/crysirror-circus/src/dc3_text.rs#L372) |
| `event.kind` | `kind` | Local state/value operations | Unsupported: DC3WY 41F1D0 inline event kind {kind} is not connected | [379](../../crates/crysirror-circus/src/dc3_text.rs#L379) |

### dc3_ui.rs / Machine::execute_ui

```rust
fn execute_ui(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
    ) -> Result<Option<u32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under instruction.opcode=0x32..=0x34 / instruction.opcode=0x32` | `a[0] == 0` | Local state/value operations | Handled | [18](../../crates/crysirror-circus/src/dc3_ui.rs#L18) |
| `instruction.opcode; under instruction.opcode=0x32..=0x34` | `_` | Local state/value operations | Handled | [25](../../crates/crysirror-circus/src/dc3_ui.rs#L25) |
| `condition; under instruction.opcode=0x6e` | `a[..2] != a[2..4]` | Local state/value operations | Handled | [99](../../crates/crysirror-circus/src/dc3_ui.rs#L99) |
| `condition; under instruction.opcode=0x71` | `a[1] <= 256` | Local state/value operations | Handled | [111](../../crates/crysirror-circus/src/dc3_ui.rs#L111) |
| `condition; under instruction.opcode=0x71` | `!matches!(a[0] & 15, 0 \| 1)` | Local state/value operations | Handled | [118](../../crates/crysirror-circus/src/dc3_ui.rs#L118) |
| `condition; under instruction.opcode=0x71` | `a[1] != 768` | Local state/value operations | Handled | [121](../../crates/crysirror-circus/src/dc3_ui.rs#L121) |
| `instruction.opcode` | `0x7d if a[2] == 0` | Local state/value operations | Empty handler: Original no-op status not established | [65](../../crates/crysirror-circus/src/dc3_ui.rs#L65) |
| `instruction.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [141](../../crates/crysirror-circus/src/dc3_ui.rs#L141) |

### dc3_value_reads.rs / Machine::prepare_value_reads

```rust
fn prepare_value_reads(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under instruction.opcode=0x7c` | `(a[3] as u8 as i8) < 0` | i16::from | Handled | [31](../../crates/crysirror-circus/src/dc3_value_reads.rs#L31) |
| `instruction.opcode` | `0x78 if matches!(a[3], 1 \| 2)` | Local state/value operations | Handled | [23](../../crates/crysirror-circus/src/dc3_value_reads.rs#L23) |
| `instruction.opcode` | `_` | Local state/value operations | Handled | [65](../../crates/crysirror-circus/src/dc3_value_reads.rs#L65) |
| `selector` | `4` | host.system, i32::from | Handled | [97](../../crates/crysirror-circus/src/dc3_value_reads.rs#L97) |
| `selector` | `21` | host.resource, i32::from | Handled | [102](../../crates/crysirror-circus/src/dc3_value_reads.rs#L102) |
| `selector` | `_` | Local state/value operations | Handled | [110](../../crates/crysirror-circus/src/dc3_value_reads.rs#L110) |

### lib.rs / Program::native_byte

```rust
fn native_byte(&self, pc:u32) -> Option<u8>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `&instruction.arguments` | `Arguments::Bytes(values)` | Local state/value operations | Handled | [231](../../crates/crysirror-circus/src/lib.rs#L231) |
| `&instruction.arguments` | `Arguments::Words(values)` | Local state/value operations | Handled | [232](../../crates/crysirror-circus/src/lib.rs#L232) |
| `&instruction.arguments` | `Arguments::SlotString{slot,..} if relative==1` | Local state/value operations | Handled | [233](../../crates/crysirror-circus/src/lib.rs#L233) |
| `&instruction.arguments` | `Arguments::SlotString{..}\|Arguments::String(_)` | self.literal_bytes | Handled | [234](../../crates/crysirror-circus/src/lib.rs#L234) |

### machine.rs / skip

```rust
fn skip(program: &Program, mut pc: u32, marker: u8) -> Result<u32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode == 0` | Local state/value operations | Unsupported: Circus branch marker {marker} not found before end | [2363](../../crates/crysirror-circus/src/machine.rs#L2363) |
| `condition` | `i.opcode == 0x20 && matches!(i.arguments,Arguments::Bytes([_,id]) if id==marker)` | Local state/value operations | Handled | [2366](../../crates/crysirror-circus/src/machine.rs#L2366) |

### machine.rs / Machine::execute

```rust
fn execute(
        &mut self,
        program: &Program,
        i: &Instruction,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode==0x1e && (self.state.dispatch_a2!=0 \|\| self.state.native_flags.get(&25104).copied().unwrap_or(0)!=0)` | self.state.native_live.ignored_system_dispatch | Handled | [1152](../../crates/crysirror-circus/src/machine.rs#L1152) |
| `condition; under result=CommandResult::Complete { next, outcome }` | `i.opcode==0x18` | self.state.native_live.syscall_completed | Handled | [1183](../../crates/crysirror-circus/src/machine.rs#L1183) |
| `condition; under result=CommandResult::Complete { next, outcome }` | `i.opcode==0x1e` | self.state.native_live.system_function_completed | Handled | [1184](../../crates/crysirror-circus/src/machine.rs#L1184) |
| `condition; under i.opcode=0x03` | `program .instruction(next) .map_or(true, \|next\| next.opcode != 0x4c)` | self.state.read_run.as_ref, u32::from, self.state.read_run.as_mut, host.scene_index, self.state.scene.clone | Handled | [1240](../../crates/crysirror-circus/src/machine.rs#L1240) |
| `condition; under i.opcode=0x03` | `id >= 0 && *start != 0` | Local state/value operations | Handled | [1254](../../crates/crysirror-circus/src/machine.rs#L1254) |
| `condition; under i.opcode=0x0a` | `let Some(jump) = program.instructions.iter().find(\|instruction\| { instruction.opcode == 3 && matches!(instruction.arguments, Arguments::Bytes([lo, hi]) if (u16::from(lo) \| (u16::from(hi) << 8)) == label) })` | Local state/value operations | Handled | [1327](../../crates/crysirror-circus/src/machine.rs#L1327) |
| `condition; under i.opcode=0x0a` | `let Some(jump) = program.instructions.iter().find(\|instruction\| { instruction.opcode == 0x4b && matches!(&instruction.arguments, Arguments::String(label) if label == &action) })` | Local state/value operations | Handled | [1345](../../crates/crysirror-circus/src/machine.rs#L1345) |
| `condition; under i.opcode=0x0a` | `a[0] == 0` | self.state.native_live.dialogue_refresh_buffers, self.state.scene.clone, self.state.dialogue.refresh_text_owner.is_none, self.state.groups.contains_key, host.text | Handled | [1350](../../crates/crysirror-circus/src/machine.rs#L1350) |
| `condition; under i.opcode=0x11` | `a[0] == 0` | self.state.bytes.retain | Handled | [1393](../../crates/crysirror-circus/src/machine.rs#L1393) |
| `condition; under i.opcode=0x13` | `a[1] == 255 && a[4] != 0` | u32::try_from | Handled | [1423](../../crates/crysirror-circus/src/machine.rs#L1423) |
| `condition; under i.opcode=0x15` | `a[0] == 0 && a[1] != 0` | Local state/value operations | Handled | [1450](../../crates/crysirror-circus/src/machine.rs#L1450) |
| `condition; under i.opcode=0x16` | `data.opcode != 0x16` | Local state/value operations | Handled | [1464](../../crates/crysirror-circus/src/machine.rs#L1464) |
| `a[0]; under i.opcode=0x43` | `_` | Local state/value operations | Unsupported: Circus native callback slot {} needs verified activation/timer policy | [1573](../../crates/crysirror-circus/src/machine.rs#L1573) |
| `condition; under i.opcode=0x1a \| 0x24` | `i.opcode==0x1a` | self.state.finish_button | Handled | [1589](../../crates/crysirror-circus/src/machine.rs#L1589) |
| `condition; under i.opcode=0x1a \| 0x24` | `i.opcode == 0x1a` | Local state/value operations | Handled | [1590](../../crates/crysirror-circus/src/machine.rs#L1590) |
| `a[0]; under i.opcode=0x1a \| 0x24` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1598](../../crates/crysirror-circus/src/machine.rs#L1598) |
| `a[0]; under i.opcode=0x1a \| 0x24` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1612](../../crates/crysirror-circus/src/machine.rs#L1612) |
| `condition; under i.opcode=0x20` | `a[0] == 0` | Local state/value operations | Handled | [1627](../../crates/crysirror-circus/src/machine.rs#L1627) |
| `condition; under i.opcode=0x28` | `!matches!(a[0], 7 \| 16) && !self.flush_falling_resources(host)?` | Local state/value operations | Handled | [1651](../../crates/crysirror-circus/src/machine.rs#L1651) |
| `condition; under i.opcode=0x28` | `a[0] == 0` | self.state.native_live.special_effects, host.resource, self.state.native_live.unload_image, self.state.image_names.remove, self.state.resources.clear_pending | Handled | [1654](../../crates/crysirror-circus/src/machine.rs#L1654) |
| `condition; under i.opcode=0x78` | `a[..2] != a[2..4]` | Local state/value operations | Handled | [1673](../../crates/crysirror-circus/src/machine.rs#L1673) |
| `condition; under i.opcode=0x34 if (1..=4).contains(&a[0])` | `a[0] == 4` | self.state.builder.behavior_flags.insert | Handled | [1687](../../crates/crysirror-circus/src/machine.rs#L1687) |
| `a[1]; under i.opcode=0x34 if a[0]==99` | `_` | Local state/value operations | Handled | [1693](../../crates/crysirror-circus/src/machine.rs#L1693) |
| `condition; under i.opcode=0x34 if a[0]==8` | `a[1]!=0` | Local state/value operations | Handled | [1698](../../crates/crysirror-circus/src/machine.rs#L1698) |
| `a[0]; under i.opcode=0x34 if matches!(a[0],5..=7)` | `_` | Local state/value operations | Handled | [1702](../../crates/crysirror-circus/src/machine.rs#L1702) |
| `condition; under i.opcode=0x34 if matches!(a[0],5..=7)` | `!self.set_control_attribute(host,a[1]!=0,attribute)?` | Local state/value operations | Handled | [1703](../../crates/crysirror-circus/src/machine.rs#L1703) |
| `condition; under i.opcode=0x46` | `a[0] == 255` | self.state.voice_timing.clear | Handled | [1820](../../crates/crysirror-circus/src/machine.rs#L1820) |
| `condition; under i.opcode=0x46` | `!self .state .voice_timing .iter() .any(\|row\| row.0.starts_with(&key))` | self.state.voice_timing.push, i32::from | Handled | [1831](../../crates/crysirror-circus/src/machine.rs#L1831) |
| `condition; under i.opcode=0x46` | `a[0] > 100` | i32::from | Handled | [1843](../../crates/crysirror-circus/src/machine.rs#L1843) |
| `condition; under i.opcode=0x47` | `let Some(row) = table.iter_mut().find(\|row\| row.0 == key)` | Local state/value operations | Handled | [1889](../../crates/crysirror-circus/src/machine.rs#L1889) |
| `condition; under i.opcode=0x4f \| 0x50` | `i.opcode==0x4f` | Vec::new, self.state.native_live.object_record, u32::from_le_bytes, self.state.native_live.sprite_seed, host.sprite | Handled | [1905](../../crates/crysirror-circus/src/machine.rs#L1905) |
| `condition; under i.opcode=0x4f \| 0x50` | `i.opcode==0x4f` | self.state.dialogue.voice_source.clear | Handled | [1937](../../crates/crysirror-circus/src/machine.rs#L1937) |
| `condition; under i.opcode=0x7b` | `a[1]<=256\|\|a[1]==768` | self.state.builder.images.len, self.state.builder.control_operations.retain, self.state.builder.control_operations.push, UiRecordAttribute::Pivot, Vec::new | Handled | [2024](../../crates/crysirror-circus/src/machine.rs#L2024) |
| `condition; under i.opcode=0x82` | `a[3] == 0` | Local state/value operations | Handled | [2069](../../crates/crysirror-circus/src/machine.rs#L2069) |
| `a[3]; under i.opcode=0x82` | `0` | i32::from | Handled | [2075](../../crates/crysirror-circus/src/machine.rs#L2075) |
| `a[3]; under i.opcode=0x82` | `1` | Local state/value operations | Handled | [2076](../../crates/crysirror-circus/src/machine.rs#L2076) |
| `a[3]; under i.opcode=0x82` | `2` | Local state/value operations | Handled | [2077](../../crates/crysirror-circus/src/machine.rs#L2077) |
| `a[3]; under i.opcode=0x82` | `_` | Local state/value operations | Unsupported: Circus arithmetic assignment mode {} | [2078](../../crates/crysirror-circus/src/machine.rs#L2078) |
| `i.opcode` | `0x15 if a[0] == 255` | Local state/value operations | Handled | [1437](../../crates/crysirror-circus/src/machine.rs#L1437) |
| `i.opcode` | `0x15 if a[0] == 254` | Local state/value operations | Handled | [1443](../../crates/crysirror-circus/src/machine.rs#L1443) |
| `i.opcode` | `0x18 if a[0] == 15` | self.state.native_flags.insert, self.state.assign | Handled | [1487](../../crates/crysirror-circus/src/machine.rs#L1487) |
| `i.opcode` | `0x18 if a[0] == 7` | host.system, system_ops::SystemRequest::WindowTransitionFlag, i32::from, self.state.system.insert | Handled | [1494](../../crates/crysirror-circus/src/machine.rs#L1494) |
| `i.opcode` | `0x18 if a[0] == 26` | self.state.pending_system_reload.is_none, host.reload_system, self.state.pending_system_reload.as_ref, self.state.system_data.clone, self.state.value | Handled | [1500](../../crates/crysirror-circus/src/machine.rs#L1500) |
| `i.opcode` | `0x18 if a[0] == 5` | host.scene_index, host.write_system, self.state.system_data.bytes | Handled | [1522](../../crates/crysirror-circus/src/machine.rs#L1522) |
| `i.opcode` | `0x1e if a[0] == 19` | u16::try_from, self.state.value, host.save_readable, self.state.assign, i32::from | Handled | [1540](../../crates/crysirror-circus/src/machine.rs#L1540) |
| `i.opcode` | `0x1e if a[0] == 51` | Local state/value operations | Handled | [1548](../../crates/crysirror-circus/src/machine.rs#L1548) |
| `i.opcode` | `0x34 if (1..=4).contains(&a[0])` | self.state.current_button.as_mut, self.state.builder.behavior_flags.insert | Handled | [1683](../../crates/crysirror-circus/src/machine.rs#L1683) |
| `i.opcode` | `0x34 if a[0]==99` | self.state.select_builder | Handled | [1692](../../crates/crysirror-circus/src/machine.rs#L1692) |
| `i.opcode` | `0x34 if a[0]==8` | self.state.current_button.as_mut | Handled | [1696](../../crates/crysirror-circus/src/machine.rs#L1696) |
| `i.opcode` | `0x34 if matches!(a[0],5..=7)` | self.set_control_attribute | Handled | [1701](../../crates/crysirror-circus/src/machine.rs#L1701) |
| `i.opcode` | `0x34 if a[0] == 0` | Local state/value operations | Handled | [1706](../../crates/crysirror-circus/src/machine.rs#L1706) |
| `i.opcode` | `0x3c if a[0] == 0` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1747](../../crates/crysirror-circus/src/machine.rs#L1747) |
| `i.opcode` | `0x49 if text == ":r" \|\| text == ":n"` | self.state.native_live.background_protection, self.state.callbacks.clear, i32::from | Handled | [1767](../../crates/crysirror-circus/src/machine.rs#L1767) |
| `i.opcode` | `opcode if crate::opcodes::native_opcode_kind(opcode)==Some(crate::opcodes::NativeOpcodeKind::NoOp)` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1812](../../crates/crysirror-circus/src/machine.rs#L1812) |
| `i.opcode` | `0xb3 if a[0] == 0` | Local state/value operations | Handled | [1860](../../crates/crysirror-circus/src/machine.rs#L1860) |
| `i.opcode` | `0x9d if a[..4] == [0, 100, 0, 0] && self.state.native_flags.get(&66513).copied().unwrap_or(0) == 0` | Local state/value operations | Handled | [1869](../../crates/crysirror-circus/src/machine.rs#L1869) |
| `i.opcode` | `0xae if a[3] == 255` | Local state/value operations | Handled | [1874](../../crates/crysirror-circus/src/machine.rs#L1874) |
| `i.opcode` | `0x47 if self.state.placement_declaration.is_none()` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1879](../../crates/crysirror-circus/src/machine.rs#L1879) |
| `i.opcode` | `0x82 if !matches!(a[3],0..=2)` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [2067](../../crates/crysirror-circus/src/machine.rs#L2067) |
| `i.opcode` | `0x87 if a[2] == 0` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [2127](../../crates/crysirror-circus/src/machine.rs#L2127) |
| `i.opcode` | `0x8b if (16..=20).contains(&a[3])` | Local state/value operations | Handled | [2151](../../crates/crysirror-circus/src/machine.rs#L2151) |
| `i.opcode` | `0x8b if a[3] == 0 && (0..=4).contains(&a[2])` | Local state/value operations | Handled | [2162](../../crates/crysirror-circus/src/machine.rs#L2162) |
| `i.opcode` | `0x8c if a[3] == 0` | Local state/value operations | Handled | [2168](../../crates/crysirror-circus/src/machine.rs#L2168) |
| `i.opcode` | `0xab if a[0] == -1` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [2183](../../crates/crysirror-circus/src/machine.rs#L2183) |
| `i.opcode` | `0xab if a[0] == 99` | Local state/value operations | Handled | [2184](../../crates/crysirror-circus/src/machine.rs#L2184) |
| `i.opcode` | `_` | Local state/value operations | Unsupported: unsupported native Circus instruction {:#04x}; execution stopped without advancing | [2189](../../crates/crysirror-circus/src/machine.rs#L2189) |
| `condition` | `i.opcode==0x18` | self.state.native_live.syscall_completed | Handled | [2201](../../crates/crysirror-circus/src/machine.rs#L2201) |
| `condition` | `i.opcode==0x1e` | self.state.native_live.system_function_completed | Handled | [2202](../../crates/crysirror-circus/src/machine.rs#L2202) |

### machine.rs / Machine::step_without_history_preview

```rust
fn step_without_history_preview(&mut self,host:&mut dyn NativeHost,now:u64,pressed:bool) -> Result<Outcome>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:0})?\|\|!self.poll_text_events_mode(host,u64::MAX,2)?\|\|host.text(&text_ops::TextRequest::ClearDialogue)?.is_none()` | Local state/value operations | Handled | [885](../../crates/crysirror-circus/src/machine.rs#L885) |

### machine.rs / Machine::sync_native_event

```rust
fn sync_native_event(&mut self, host: &mut dyn NativeHost, event: &Event) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `*kind==0` | self.clear_effect_pools | Handled | [2257](../../crates/crysirror-circus/src/machine.rs#L2257) |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `matches!(*kind,7\|16) && *parameter==0` | self.state.native_live.unload_image, self.state.image_names.remove, self.state.resources.clear_pending | Handled | [2258](../../crates/crysirror-circus/src/machine.rs#L2258) |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `*kind==7` | Local state/value operations | Handled | [2259](../../crates/crysirror-circus/src/machine.rs#L2259) |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `*kind==15` | self.state.image_names.insert, self.state.native_live.load_image | Handled | [2264](../../crates/crysirror-circus/src/machine.rs#L2264) |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `matches!(*kind,13\|17)` | self.state.image_names.insert, self.state.native_live.load_image | Handled | [2268](../../crates/crysirror-circus/src/machine.rs#L2268) |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `*kind==13` | Local state/value operations | Handled | [2269](../../crates/crysirror-circus/src/machine.rs#L2269) |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `matches!(*kind,13\|15\|17)` | host.image_dimensions, self.state.native_live.attach_image, self.state.native_live.gap | Handled | [2273](../../crates/crysirror-circus/src/machine.rs#L2273) |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `*kind==15` | Local state/value operations | Handled | [2274](../../crates/crysirror-circus/src/machine.rs#L2274) |
| `condition; under event=Event::FallingEffect { kind, parameter }` | `*kind==16 && *parameter!=0` | host.image_dimensions, self.state.native_live.reset_object, self.state.native_live.bind_resource, self.sync_sprite_states, self.sync_effect_frame | Handled | [2281](../../crates/crysirror-circus/src/machine.rs#L2281) |

### machine/builder_ops.rs / Machine::finalize_panel_owner

```rust
fn finalize_panel_owner(
        &mut self,
        host: &mut dyn NativeHost,
        phase: u8,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.emit(&Event::FinalizePanel { declarations: declarations.clone(), scroll: self.state.value(99, 246)?, selected: self.state.value(99, 254)?, focus: self.state.value(99, 245)?, ticket: (self.state.instructions, phase), })?` | Local state/value operations | Handled | [110](../../crates/crysirror-circus/src/machine/builder_ops.rs#L110) |

### machine/comment_ops.rs / Machine::execute_comment

```rust
fn execute_comment(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode != 0xab \|\| matches!(a[0], 99 \| -1)` | Local state/value operations | Handled | [57](../../crates/crysirror-circus/src/machine/comment_ops.rs#L57) |
| `a[0]` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [83](../../crates/crysirror-circus/src/machine/comment_ops.rs#L83) |
| `condition` | `backend == 1 && a[0] == 1` | self.state.media.clear_playlist | Handled | [86](../../crates/crysirror-circus/src/machine/comment_ops.rs#L86) |

### machine/effect_ops.rs / Machine::execute_effects

```rust
fn execute_effects(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode == 0x0f` | self.execute_framebuffer | Handled | [215](../../crates/crysirror-circus/src/machine/effect_ops.rs#L215) |
| `condition` | `i.opcode == 0x9d && self.state.effect_ops.immediate_stage_pc == Some(i.offset)` | self.apply_history_stage, self.state.native_live.stage_opcode_completed | Handled | [218](../../crates/crysirror-circus/src/machine/effect_ops.rs#L218) |
| `condition` | `i.opcode == 0x9d && a[1] == 100 && self.state.dispatch_a2 == 0 && self.state.effect_ops.zoom_count == 0 && self.state.native_live.stage_zoom_idle()?` | self.state.native_live.stage_opcode_completed | Handled | [228](../../crates/crysirror-circus/src/machine/effect_ops.rs#L228) |
| `condition` | `matches!(i.opcode, 0x9a..=0x9d)` | self.finish_framebuffer_actors, host.effect_control, self.sync_effect_frame, host.effect_states | Handled | [240](../../crates/crysirror-circus/src/machine/effect_ops.rs#L240) |
| `condition; under i.opcode=0x9a \| 0x9c` | `i.opcode == 0x9c` | i32::from | Handled | [257](../../crates/crysirror-circus/src/machine/effect_ops.rs#L257) |
| `condition; under i.opcode=0x9a \| 0x9c` | `i.opcode == 0x9c` | Local state/value operations | Handled | [268](../../crates/crysirror-circus/src/machine/effect_ops.rs#L268) |
| `condition; under i.opcode=0x9a \| 0x9c` | `i.opcode == 0x9c` | self.state.native_flags.insert, self.state.native_live.stage_pending | Handled | [275](../../crates/crysirror-circus/src/machine/effect_ops.rs#L275) |
| `condition; under i.opcode=0x9b` | `a[1] == 100` | self.state.native_flags.insert | Handled | [294](../../crates/crysirror-circus/src/machine/effect_ops.rs#L294) |
| `condition; under i.opcode=0x9d` | `a[1] == 100` | self.state.native_flags.insert | Handled | [308](../../crates/crysirror-circus/src/machine/effect_ops.rs#L308) |
| `condition; under i.opcode=0x9d` | `history && a[1] == 100 \|\| self.state.effect_ops.zoom_count < 10` | i32::from, self.state.effect_ops.pending.push | Handled | [312](../../crates/crysirror-circus/src/machine/effect_ops.rs#L312) |
| `condition; under i.opcode=0x9d` | `history && a[1] == 100 \|\| self.state.effect_ops.zoom_count == 0` | Local state/value operations | Handled | [323](../../crates/crysirror-circus/src/machine/effect_ops.rs#L323) |
| `condition; under i.opcode=0x9d` | `!(history && a[1] == 100)` | Local state/value operations | Handled | [328](../../crates/crysirror-circus/src/machine/effect_ops.rs#L328) |
| `condition; under i.opcode=0xa2` | `a[0] == 0` | Local state/value operations | Handled | [349](../../crates/crysirror-circus/src/machine/effect_ops.rs#L349) |
| `condition; under i.opcode=0xa2` | `a[0] != 0` | Local state/value operations | Handled | [354](../../crates/crysirror-circus/src/machine/effect_ops.rs#L354) |
| `condition; under i.opcode=0xa2` | `a[0] != 0` | i32::from | Handled | [359](../../crates/crysirror-circus/src/machine/effect_ops.rs#L359) |
| `a[3]; under i.opcode=0xac` | `0` | i32::from | Handled | [376](../../crates/crysirror-circus/src/machine/effect_ops.rs#L376) |
| `a[3]; under i.opcode=0xac` | `1` | self.state.native_live.sprite_seed, host.sprite, self.sync_sprite_states | Handled | [388](../../crates/crysirror-circus/src/machine/effect_ops.rs#L388) |
| `a[3]; under i.opcode=0xac` | `2` | Local state/value operations | Handled | [408](../../crates/crysirror-circus/src/machine/effect_ops.rs#L408) |
| `a[3]; under i.opcode=0xac` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [412](../../crates/crysirror-circus/src/machine/effect_ops.rs#L412) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [414](../../crates/crysirror-circus/src/machine/effect_ops.rs#L414) |
| `condition` | `matches!(i.opcode, 0x9a..=0x9d)` | Local state/value operations | Handled | [454](../../crates/crysirror-circus/src/machine/effect_ops.rs#L454) |

### machine/effect_ops.rs / Machine::prepare_effects_frame

```rust
fn prepare_effects_frame(&mut self, host: &mut dyn NativeHost) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `operation.mode == 3` | Vec::new, i32::from, self.state.native_live.sprite_seed | Handled | [562](../../crates/crysirror-circus/src/machine/effect_ops.rs#L562) |
| `condition` | `operation.restore_position && ((0..=5).contains(&operation.selector) \|\| operation.selector == 255)` | i32::from | Handled | [565](../../crates/crysirror-circus/src/machine/effect_ops.rs#L565) |
| `condition` | `operation.mode >= 2` | self.state.native_live.stage_reference_cache, self.state.native_live.stage_reference_commit | Handled | [598](../../crates/crysirror-circus/src/machine/effect_ops.rs#L598) |
| `condition` | `operation.mode == 3` | self.state.native_live.stage_reference_cache | Handled | [604](../../crates/crysirror-circus/src/machine/effect_ops.rs#L604) |

### machine/framebuffer_ops.rs / Machine::execute_framebuffer

```rust
fn execute_framebuffer(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `capture \|\| (layout && !matches!( kind, 3 \| 4 \| 5 \| 6 \| 7 \| 9 \| 10 \| 11 \| 13 \| 14 \| 15 \| 16 \| 17 \| 20 \| 21 \| 23 \| 24 \| 25 \| 30 \| 31 \| 32 \| 51 \| 52 ))` | host.emit, self.state.native_live.framebuffer_falling_clear, self.clear_effect_pools, self.state.native_flags.insert, self.state.native_live.framebuffer_dispatch_skipped | Handled | [47](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L47) |
| `condition` | `matches!( kind, 3 \| 4 \| 5 \| 6 \| 7 \| 9 \| 10 \| 11 \| 13 \| 14 \| 15 \| 16 \| 17 \| 20 \| 21 \| 23 \| 24 \| 25 \| 30 \| 31 \| 32 \| 51 \| 52 )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [49](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L49) |
| `condition` | `kind == 2` | host.emit, self.state.native_live.framebuffer_falling_clear, self.clear_effect_pools, self.state.native_flags.insert | Handled | [75](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L75) |
| `condition; under receiver=kind == 2` | `!host.emit(&Event::FallingEffect { kind: 0, parameter: 0, })?` | Local state/value operations | Handled | [76](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L76) |
| `condition` | `(matches!(kind, 15 \| 16 \| 17 \| 20) && (layout \|\| fast)) \|\| (kind == 21 && (history \|\| layout \|\| (fast && self.state.value(9, 99)? == 0 && self.state.value(99, 192)? == 0)))` | self.state.native_live.framebuffer_dispatch_skipped | Handled | [95](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L95) |
| `condition; under kind=3` | `!host.effect_control(&EffectRequest::Flash { repeats: a[1].max(0) as u16, })?` | Local state/value operations | Handled | [137](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L137) |
| `condition; under kind=4 \| 6 \| 51 \| 52` | `matches!(kind, 4 \| 6)` | Local state/value operations | Handled | [154](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L154) |
| `condition; under kind=4 \| 6 \| 51 \| 52` | `matches!(kind, 4 \| 6)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [154](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L154) |
| `condition; under kind=4 \| 6 \| 51 \| 52` | `kind == 51` | Local state/value operations | Handled | [157](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L157) |
| `condition; under kind=4 \| 6 \| 51 \| 52` | `matches!(kind, 4 \| 51)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [170](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L170) |
| `condition; under kind=4 \| 6 \| 51 \| 52` | `kind == 51` | Local state/value operations | Handled | [172](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L172) |
| `kind` | `7 \| 10 \| 11 \| 13 \| 14 \| 23 \| 24 \| 25 \| 30 \| 31 \| 32` | self.state.native_live.ignored_framebuffer_effect | Handled | [105](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L105) |
| `kind` | `9` | self.flush_framebuffer_background, self.state.native_live.object_record, f32::from_le_bytes, host.effect_control, self.state.native_live.capture_background | Handled | [109](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L109) |
| `kind` | `3` | self.state.value, host.effect_control, self.state.native_flags.insert, self.state.native_live.framebuffer_color_effect | Handled | [135](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L135) |
| `kind` | `4 \| 6 \| 51 \| 52` | i32::from, self.finish_framebuffer_actors, Vec::new, self.state.native_live.sprite_seed, host.sprite | Handled | [152](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L152) |
| `kind` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [191](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L191) |
| `condition` | `kind == 2 && fast && (history \|\| self.state.value(9, 99)? == 0)` | self.state.native_flags.get, host.effect_control, self.commit_framebuffer_clear | Handled | [201](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L201) |
| `condition; under kind=0 \| 1 \| 2` | `kind <= 1 && self.state.transition_color.is_none()` | self.state.native_live.framebuffer_without_color | Handled | [214](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L214) |
| `condition; under kind=0 \| 1 \| 2` | `kind == 2` | host.effect_control, self.state.native_live.framebuffer_partial_clear, self.clear_effect_pools, self.state.native_flags.insert, i32::from | Handled | [218](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L218) |
| `condition; under kind=0 \| 1 \| 2` | `kind <= 1 && fast && (history \|\| self.state.value(9, 99)? == 0)` | self.state.native_live.framebuffer_dispatch_skipped | Handled | [230](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L230) |
| `condition; under kind=17 \| 20 \| 21` | `kind == 17` | host.emit, self.state.image_names.insert, self.state.native_live.load_image | Handled | [275](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L275) |
| `condition; under kind=17 \| 20 \| 21 / receiver=kind == 17` | `a[1] == 1` | Local state/value operations | Handled | [276](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L276) |
| `condition; under kind=17 \| 20 \| 21 / receiver=kind == 17` | `!host.emit(&Event::LoadImage { slot: 21, name: name.into(), })?` | Local state/value operations | Handled | [281](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L281) |
| `condition; under kind=17 \| 20 \| 21` | `(self.state.value(99, 10)? != 0 \|\| self.state.value(99, 192)? != 0) && !host.effect_control(&EffectRequest::MaskFade { reverse: kind != 20, ticks: if kind == 17 { 60 } else { 12 * i32::from(a[1]) }, color, })?` | Local state/value operations | Handled | [295](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L295) |
| `condition; under kind=15 \| 16` | `!host.emit(&Event::LoadImage { slot: 45, name: "system\\effbreak".into(), })?` | Local state/value operations | Handled | [321](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L321) |
| `condition; under kind=15 \| 16` | `self.state.effect_ops.framebuffer_started != Some(i.offset) && self.state.value(99, 10)? != 0 && !host.effect_control(&EffectRequest::TileMask { pattern: a[1], color: self.state.transition_color, })?` | Local state/value operations | Handled | [329](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L329) |
| `condition; under kind=15 \| 16` | `kind == 16` | self.state.assign, host.emit, self.state.native_live.framebuffer_falling_clear, self.state.native_live.falling_effect, self.clear_effect_pools | Handled | [345](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L345) |
| `condition; under kind=15 \| 16 / receiver=kind == 16` | `!host.emit(&Event::FallingEffect { kind: 0, parameter: 0, })?` | Local state/value operations | Handled | [353](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L353) |
| `kind` | `5` | self.state.native_live.framebuffer_queue_effect | Handled | [210](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L210) |
| `kind` | `0 \| 1 \| 2` | self.state.transition_color.is_none, self.state.native_live.framebuffer_without_color, host.effect_control, self.state.native_live.framebuffer_partial_clear, self.clear_effect_pools | Handled | [213](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L213) |
| `kind` | `17 \| 20 \| 21` | host.emit, self.state.image_names.insert, self.state.native_live.load_image, self.state.image_names.contains_key, self.state.value | Handled | [274](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L274) |
| `kind` | `15 \| 16` | host.emit, self.state.image_names.insert, self.state.native_live.load_image, self.state.value, host.effect_control | Handled | [320](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L320) |
| `kind` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [365](../../crates/crysirror-circus/src/machine/framebuffer_ops.rs#L365) |

### machine/gesture_ops.rs / Machine::poll_native_gestures

```rust
fn poll_native_gestures(&mut self,host:&mut dyn NativeHost,now:u64) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under action=7\|8\|9\|11 / host.system(SystemRequest::NativeWindowMode)?=Some(SystemReply::NativeWindowMode(mode))` | `mode!=0` | Local state/value operations | Handled | [136](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L136) |
| `condition; under action=7\|8\|9\|11` | `self.state.value(93,kind)?!=0` | Local state/value operations | Handled | [144](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L144) |
| `condition; under action=7\|8\|9\|11` | `host.publish_audio(&media_ops::AudioDeclaration{name:"se\\201".into(),voice:false,looping:false,voice_volume:None})?.is_none()` | Local state/value operations | Handled | [148](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L148) |
| `condition; under action=7\|8\|9\|11` | `!matches!(self.gesture_request(host,GestureRequest::Confirm{kind,template,warp_pointer:self.state.value(99,205)?!=0})?,Some(GestureReply::Done))` | Local state/value operations | Handled | [153](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L153) |
| `condition; under action=7\|8\|9\|11` | `matches!(kind,2\|3)` | host.system, SystemRequest::WindowTransitionFlag | Handled | [154](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L154) |
| `condition; under action=7\|8\|9\|11` | `matches!(kind,2\|3)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [154](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L154) |
| `condition; under action=7\|8\|9\|11 / action=9` | `kind==2` | Local state/value operations | Handled | [173](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L173) |
| `condition; under action=7\|8\|9\|11` | `complete&&matches!(kind,2\|3)&&host.system(SystemRequest::WindowTransitionFlag(Some(self.state.system_ops.gestures.saved_window_gate)))?.is_none()` | Local state/value operations | Handled | [180](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L180) |
| `condition; under action=7\|8\|9\|11` | `matches!(kind,2\|3)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [180](../../crates/crysirror-circus/src/machine/gesture_ops.rs#L180) |

### machine/history_ops.rs / Machine::begin_history_replay

```rust
fn begin_history_replay(&mut self,host:&mut dyn NativeHost,flat:i32,_now:u64,preview:bool) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.emit(&Event::ReleaseAudioGroup{kind,forced:true,steps:0})?` | Local state/value operations | Handled | [368](../../crates/crysirror-circus/src/machine/history_ops.rs#L368) |

### machine/history_ops.rs / Machine::history_row

```rust
fn history_row(&mut self,host:&mut dyn NativeHost,flat:i32) -> Result<Option<Option<HistoryRow>>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `(&instruction.arguments,instruction.opcode)` | `(Arguments::String(text),0x6a)` | Local state/value operations | Handled | [289](../../crates/crysirror-circus/src/machine/history_ops.rs#L289) |
| `(&instruction.arguments,instruction.opcode)` | `(Arguments::String(text),0x4d)` | Local state/value operations | Handled | [290](../../crates/crysirror-circus/src/machine/history_ops.rs#L290) |
| `(&instruction.arguments,instruction.opcode)` | `(Arguments::String(text),0x6b)` | Local state/value operations | Handled | [291](../../crates/crysirror-circus/src/machine/history_ops.rs#L291) |
| `(&instruction.arguments,instruction.opcode)` | `(_,0\|0x16)` | Local state/value operations | Handled | [292](../../crates/crysirror-circus/src/machine/history_ops.rs#L292) |
| `(&instruction.arguments,instruction.opcode)` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [293](../../crates/crysirror-circus/src/machine/history_ops.rs#L293) |

### machine/history_ops.rs / Machine::open_history

```rust
fn open_history(&mut self,host:&mut dyn NativeHost,now:u64) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.state.history.opening_sound&&host.publish_audio(&media_ops::AudioDeclaration{name:"se\\201".into(),voice:false,looping:false,voice_volume:None})?.is_none()` | Local state/value operations | Handled | [112](../../crates/crysirror-circus/src/machine/history_ops.rs#L112) |
| `condition` | `!host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:0})?` | Local state/value operations | Handled | [116](../../crates/crysirror-circus/src/machine/history_ops.rs#L116) |

### machine/history_ops.rs / Machine::poll_history

```rust
fn poll_history(&mut self,host:&mut dyn NativeHost,now:u64) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under action=HistoryAction::Row(row_index)` | `self.history_reply(host,HistoryRequest::Confirm{kind:1,row,template,warp_pointer:self.state.system_data.dword(33)!=0})?.is_none()` | Local state/value operations | Handled | [259](../../crates/crysirror-circus/src/machine/history_ops.rs#L259) |

### machine/input_prefix.rs / Machine::poll_native_prefix

```rust
fn poll_native_prefix(
        &mut self,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 1` | self.state.native_flags.insert, self.prefix_finish | Handled | [170](../../crates/crysirror-circus/src/machine/input_prefix.rs#L170) |
| `condition` | `host .publish_audio(&media_ops::AudioDeclaration { name: "se\\201".into(), voice: false, looping: false, voice_volume: None, })? .is_none()` | Local state/value operations | Handled | [221](../../crates/crysirror-circus/src/machine/input_prefix.rs#L221) |
| `condition` | `input.released & key != 0` | Local state/value operations | Handled | [261](../../crates/crysirror-circus/src/machine/input_prefix.rs#L261) |

### machine/macro_ops.rs / Machine::record_macro

```rust
fn record_macro(
        &mut self,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
    ) -> Result<Option<Outcome>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode == 0xb0` | Local state/value operations | Handled | [106](../../crates/crysirror-circus/src/machine/macro_ops.rs#L106) |
| `condition` | `i.opcode == 0xb0` | i16::from | Handled | [117](../../crates/crysirror-circus/src/machine/macro_ops.rs#L117) |
| `condition` | `a[0] == 255` | i16::from | Handled | [118](../../crates/crysirror-circus/src/machine/macro_ops.rs#L118) |
| `condition` | `i16::from(self.state.macros.group) != a[1]` | Local state/value operations | Handled | [121](../../crates/crysirror-circus/src/machine/macro_ops.rs#L121) |
| `condition` | `matches!(i.opcode, 0x6a \| 0x6b)` | Local state/value operations | Handled | [130](../../crates/crysirror-circus/src/machine/macro_ops.rs#L130) |

### machine/macro_ops.rs / Machine::replay_macro

```rust
fn replay_macro(
        &mut self,
        id: i32,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == 0` | Vec::new | Handled | [202](../../crates/crysirror-circus/src/machine/macro_ops.rs#L202) |
| `condition` | `id == 0` | Local state/value operations | Handled | [218](../../crates/crysirror-circus/src/machine/macro_ops.rs#L218) |
| `condition` | `id == 0` | Local state/value operations | Handled | [240](../../crates/crysirror-circus/src/machine/macro_ops.rs#L240) |
| `condition` | `command.opcode == 0` | Local state/value operations | Handled | [247](../../crates/crysirror-circus/src/machine/macro_ops.rs#L247) |
| `condition` | `id != 0` | Local state/value operations | Handled | [262](../../crates/crysirror-circus/src/machine/macro_ops.rs#L262) |

### machine/macro_ops.rs / NativeCommand::instruction

```rust
fn instruction(&self, pc: u32) -> Instruction
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `self.opcode` | `0x00..=0x3a` | Arguments::Bytes | Handled | [288](../../crates/crysirror-circus/src/machine/macro_ops.rs#L288) |
| `self.opcode` | `0x3b..=0x47` | Local state/value operations | Handled | [289](../../crates/crysirror-circus/src/machine/macro_ops.rs#L289) |
| `self.opcode` | `0x48..=0x6d` | Arguments::String | Handled | [293](../../crates/crysirror-circus/src/machine/macro_ops.rs#L293) |
| `self.opcode` | `_` | Arguments::Words | Handled | [294](../../crates/crysirror-circus/src/machine/macro_ops.rs#L294) |

### machine/media_ops.rs / Machine::execute_media

```rust
fn execute_media(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under i.opcode=0x4c\|0x4d\|0x4e` | `!host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:30})?` | Local state/value operations | Handled | [146](../../crates/crysirror-circus/src/machine/media_ops.rs#L146) |
| `condition; under i.opcode=0x4c\|0x4d\|0x4e` | `i.opcode==0x4e` | Local state/value operations | Handled | [150](../../crates/crysirror-circus/src/machine/media_ops.rs#L150) |
| `condition; under i.opcode=0x71 if a[0]==-2` | `!host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:30})?` | Local state/value operations | Handled | [166](../../crates/crysirror-circus/src/machine/media_ops.rs#L166) |
| `condition; under i.opcode=0x71 if a[0]==-1` | `!host.emit(&Event::ReleaseAudioGroup{kind:1,forced:false,steps:30})?` | Local state/value operations | Handled | [173](../../crates/crysirror-circus/src/machine/media_ops.rs#L173) |
| `a[0]; under i.opcode=0x05` | `percent` | self.state.value, i32::from | Handled | [192](../../crates/crysirror-circus/src/machine/media_ops.rs#L192) |
| `condition; under i.opcode=0x70` | `a[0]==255` | host.emit, self.state.native_live.music | Handled | [207](../../crates/crysirror-circus/src/machine/media_ops.rs#L207) |
| `condition; under i.opcode=0x70` | `!(a[1]==1 && self.state.current_music==a[0])` | Local state/value operations | Handled | [215](../../crates/crysirror-circus/src/machine/media_ops.rs#L215) |
| `condition; under i.opcode=0x6f` | `a[0] != 255` | Local state/value operations | Handled | [250](../../crates/crysirror-circus/src/machine/media_ops.rs#L250) |
| `condition; under i.opcode=0xaf` | `a[3]<3` | self.state.media.pending_balance.insert, i32::from, u32::try_from | Handled | [264](../../crates/crysirror-circus/src/machine/media_ops.rs#L264) |
| `condition; under i.opcode=0xab if a[0] == 99` | `!self.reset_comments(host,matches!(a[1],1\|2).then_some(a[1]),now,true)?` | Local state/value operations | Handled | [270](../../crates/crysirror-circus/src/machine/media_ops.rs#L270) |
| `i.opcode` | `0x71 if a[0]>=0` | self.state.pending_audio.iter, host.sound_name, self.state.pending_audio.push | Handled | [154](../../crates/crysirror-circus/src/machine/media_ops.rs#L154) |
| `i.opcode` | `0x71 if a[0]==-2` | self.state.pending_audio.retain, self.state.dialogue.voice_source.clear, self.state.native_live.voice_source, self.state.native_flags.get, host.emit | Handled | [161](../../crates/crysirror-circus/src/machine/media_ops.rs#L161) |
| `i.opcode` | `0x71 if a[0]==-1` | self.state.native_live.sound_stop_declared, self.state.native_flags.get, self.state.pending_audio.retain, host.emit | Handled | [169](../../crates/crysirror-circus/src/machine/media_ops.rs#L169) |
| `i.opcode` | `0xab if a[0] == 99` | self.reset_comments, self.state.media.playlists.clear, self.state.native_live.movie_backend_selected | Handled | [269](../../crates/crysirror-circus/src/machine/media_ops.rs#L269) |
| `i.opcode` | `0xab if a[0]==-1` | self.reset_comments, self.state.native_live.movie_stopped, self.state.media.playlists.clear, self.state.assign, self.state.native_flags.insert | Handled | [277](../../crates/crysirror-circus/src/machine/media_ops.rs#L277) |
| `i.opcode` | `0xab if self.state.video_backend.is_none()` | Local state/value operations | Empty handler: Original no-op status not established | [278](../../crates/crysirror-circus/src/machine/media_ops.rs#L278) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [279](../../crates/crysirror-circus/src/machine/media_ops.rs#L279) |
| `condition` | `i.opcode==0x05` | self.state.native_live.music_volume_backup | Handled | [281](../../crates/crysirror-circus/src/machine/media_ops.rs#L281) |

### machine/media_ops.rs / Machine::prepare_restore_audio

```rust
fn prepare_restore_audio(&self,plan:&mut crate::native_save::RestorePlan) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `audio.kind==2` | self.state.native_voice_volume | Handled | [313](../../crates/crysirror-circus/src/machine/media_ops.rs#L313) |

### machine/menu_ops.rs / image_visible

```rust
fn image_visible(mode:i16,state:i16) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `mode` | `100` | Local state/value operations | Handled | [24](../../crates/crysirror-circus/src/machine/menu_ops.rs#L24) |
| `mode` | `101\|102\|104` | Local state/value operations | Handled | [24](../../crates/crysirror-circus/src/machine/menu_ops.rs#L24) |
| `mode` | `3` | Local state/value operations | Handled | [24](../../crates/crysirror-circus/src/machine/menu_ops.rs#L24) |
| `mode` | `100..` | Local state/value operations | Handled | [24](../../crates/crysirror-circus/src/machine/menu_ops.rs#L24) |
| `mode` | `_` | Local state/value operations | Handled | [24](../../crates/crysirror-circus/src/machine/menu_ops.rs#L24) |

### machine/menu_ops.rs / Machine::declare_button_image

```rust
fn declare_button_image(&mut self,a:[i16;8],host:&mut dyn NativeHost) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `a[7]==5` | self.state.current_button.as_mut, i32::from | Handled | [28](../../crates/crysirror-circus/src/machine/menu_ops.rs#L28) |
| `condition` | `a[7]==12` | self.set_control_attribute, UiRecordAttribute::Pivot | Handled | [33](../../crates/crysirror-circus/src/machine/menu_ops.rs#L33) |
| `a[7]` | `6` | Local state/value operations | Handled | [40](../../crates/crysirror-circus/src/machine/menu_ops.rs#L40) |
| `a[7]` | `8` | Local state/value operations | Handled | [40](../../crates/crysirror-circus/src/machine/menu_ops.rs#L40) |
| `a[7]` | `10` | Local state/value operations | Handled | [40](../../crates/crysirror-circus/src/machine/menu_ops.rs#L40) |
| `a[7]` | `11` | Local state/value operations | Handled | [40](../../crates/crysirror-circus/src/machine/menu_ops.rs#L40) |
| `a[7]` | `value` | Local state/value operations | Handled | [40](../../crates/crysirror-circus/src/machine/menu_ops.rs#L40) |

### machine/movie_ops.rs / Machine::execute_movie

```rust
fn execute_movie(
        &mut self,
        _program: &Program,
        i: &Instruction,
        _a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode != 0x52` | Local state/value operations | Handled | [36](../../crates/crysirror-circus/src/machine/movie_ops.rs#L36) |
| `condition` | `!protected && !host.emit(&Event::ReleaseAudioGroup { kind, forced: true, steps: 0, })?` | Local state/value operations | Handled | [79](../../crates/crysirror-circus/src/machine/movie_ops.rs#L79) |

### machine/packed_ops.rs / Machine::execute_packed

```rust
fn execute_packed(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode==0x95 && (self.state.dispatch_a2!=0 \|\| self.state.native_flags.get(&25104).copied().unwrap_or(0)!=0)` | Local state/value operations | Handled | [70](../../crates/crysirror-circus/src/machine/packed_ops.rs#L70) |
| `condition` | `i.opcode == 0x95 && matches!(a[3], 5..=7)` | u16::try_from, self.state.native_live.sprite_seed, host.sprite, self.sync_sprite_states | Handled | [73](../../crates/crysirror-circus/src/machine/packed_ops.rs#L73) |
| `condition; under i.opcode=0x3c if (33..=36).contains(&a[0])` | `!host.packed(&PackedRequest::Load { slot: a[0] as u8, name: text.to_owned(), })?` | Local state/value operations | Handled | [91](../../crates/crysirror-circus/src/machine/packed_ops.rs#L91) |
| `condition; under i.opcode=0x8f if a[3] & 0x30 != 0` | `self .state .image_names .get(&33) .is_some_and(\|name\| !name.is_empty())` | self.state.native_live.sprite_seed, host.packed, self.state.native_live.reset_object, self.packed_publication_receipt, self.sync_sprite_states | Handled | [101](../../crates/crysirror-circus/src/machine/packed_ops.rs#L101) |
| `condition; under i.opcode=0x95 if matches!(a[3], 4 \| 6)` | `a[3] == 4` | Local state/value operations | Handled | [133](../../crates/crysirror-circus/src/machine/packed_ops.rs#L133) |
| `i.opcode` | `0x3c if (33..=36).contains(&a[0])` | host.packed, self.state.image_names.insert, self.state.native_live.packed_resource_loaded | Handled | [90](../../crates/crysirror-circus/src/machine/packed_ops.rs#L90) |
| `i.opcode` | `0x8f if a[3] & 0x30 != 0` | self.state.native_live.sprite_seed, host.packed, self.state.native_live.reset_object, self.packed_publication_receipt, self.sync_sprite_states | Handled | [100](../../crates/crysirror-circus/src/machine/packed_ops.rs#L100) |
| `i.opcode` | `0x95 if a[3] == 5` | u16::try_from | Handled | [127](../../crates/crysirror-circus/src/machine/packed_ops.rs#L127) |
| `i.opcode` | `0x95 if a[3] == 7` | Local state/value operations | Handled | [131](../../crates/crysirror-circus/src/machine/packed_ops.rs#L131) |
| `i.opcode` | `0x95 if matches!(a[3], 4 \| 6)` | u8::try_from, self.state.native_live.sprite_seed, host.packed, self.state.native_live.reset_object, self.packed_publication_receipt | Handled | [132](../../crates/crysirror-circus/src/machine/packed_ops.rs#L132) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [171](../../crates/crysirror-circus/src/machine/packed_ops.rs#L171) |

### machine/portrait_ops.rs / Machine::execute_portrait

```rust
fn execute_portrait(
        &mut self,
        i: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!(i.opcode, 0x8b \| 0x8c)` | Local state/value operations | Handled | [10](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L10) |
| `condition` | `a[3] == 255` | self.state.native_live.sprite_opcode_completed | Handled | [19](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L19) |
| `condition` | `a[3] != 0 && !negative && a[3] & 0x70 == 0` | self.state.native_live.sprite_opcode_completed | Handled | [24](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L24) |
| `condition` | `i.opcode == 0x8b && !negative && a[3] & 0x70 == 0x10 && self.state.native_flags.get(&66513).copied().unwrap_or(0) != 0` | self.state.native_live.sprite_opcode_completed | Handled | [28](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L28) |
| `condition` | `i.opcode == 0x8c` | Local state/value operations | Handled | [36](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L36) |
| `condition` | `a[3] == 0` | u16::try_from, i32::from | Handled | [38](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L38) |
| `condition` | `i.opcode == 0x8c` | Local state/value operations | Handled | [47](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L47) |
| `condition` | `a[3] == 0` | self.state.native_live.sprite_seed | Handled | [55](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L55) |
| `condition` | `(a[3] as u8 as i8) < 0` | self.state.resources.clear_pending, self.state.pending_images.remove, self.state.image_names.remove, self.state.native_live.image_name, self.state.native_live.unload_image | Handled | [71](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L71) |
| `condition` | `!host.effect_control(&super::effect_ops::EffectRequest::SpriteMask { seed: self.state.native_live.sprite_seed(object)?, mode: false, ticks: 12 * i32::from(a[1]), })?` | Local state/value operations | Handled | [88](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L88) |
| `condition` | `a[3] & 0x40 != 0` | self.state.native_flags.get, self.state.native_live.sprite_opcode_completed, self.state.native_live.unload_image, self.state.image_names.remove, host.sprite | Handled | [97](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L97) |
| `condition` | `i.opcode == 0x8c && self.state.native_flags.get(&25104).copied().unwrap_or(0) != 0` | self.state.native_live.sprite_opcode_completed | Handled | [98](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L98) |
| `condition` | `a[0] == 3` | host.effect_control, self.state.native_live.sprite_seed, i32::from, self.sync_sprite_states | Handled | [124](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L124) |
| `condition` | `!host.effect_control(&super::effect_ops::EffectRequest::SpriteMask { seed: self.state.native_live.sprite_seed(object)?, mode: true, ticks: 12 * i32::from(a[1]), })?` | Local state/value operations | Handled | [125](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L125) |
| `condition` | `(0..=2).contains(&a[0])` | self.state.native_live.sprite_seed | Handled | [133](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L133) |
| `condition` | `i.opcode == 0x8b` | Local state/value operations | Handled | [136](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L136) |
| `condition` | `a[0] == 2` | Local state/value operations | Handled | [138](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L138) |
| `condition` | `i.opcode == 0x8b` | Local state/value operations | Handled | [143](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L143) |
| `a[0]` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [147](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L147) |
| `a[0]` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [153](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L153) |
| `condition` | `a[3] & 0x20 != 0` | host.visual, self.state.native_live.object_priority | Handled | [201](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L201) |
| `condition` | `!host.visual(&super::visual_ops::VisualRequest::Priority { object, priority: a[0], })?` | Local state/value operations | Handled | [202](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L202) |
| `condition` | `a[3] & 0x10 != 0` | self.state.native_flags.get, self.state.native_live.sprite_seed | Handled | [209](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L209) |
| `condition` | `i.opcode == 0x8c \|\| self.state.native_flags.get(&66513).copied().unwrap_or(0) == 0` | self.state.native_live.sprite_seed | Handled | [210](../../crates/crysirror-circus/src/machine/portrait_ops.rs#L210) |

### machine/resource_ops.rs / retarget_inline_name

```rust
fn retarget_inline_name(old:&str,staged:&str) -> Result<String>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode==1` | Local state/value operations | Handled | [1122](../../crates/crysirror-circus/src/machine/resource_ops.rs#L1122) |
| `condition` | `mode==2` | Local state/value operations | Handled | [1122](../../crates/crysirror-circus/src/machine/resource_ops.rs#L1122) |
| `condition` | `mode==1` | Local state/value operations | Handled | [1126](../../crates/crysirror-circus/src/machine/resource_ops.rs#L1126) |
| `condition` | `mode==2` | Local state/value operations | Handled | [1126](../../crates/crysirror-circus/src/machine/resource_ops.rs#L1126) |

### machine/resource_ops.rs / Machine::execute_resource

```rust
fn execute_resource(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!matches!( i.opcode, 0x37 \| 0x3c \| 0x3d \| 0x3f \| 0x40 \| 0x41 \| 0x49 \| 0x4a )` | Local state/value operations | Handled | [632](../../crates/crysirror-circus/src/machine/resource_ops.rs#L632) |
| `condition` | `matches!(i.opcode,0x49\|0x4a) && !(text.starts_with(":n")\|\|text.starts_with(":r"))` | self.state.saves.rollback.as_ref, self.native_cg_index | Handled | [640](../../crates/crysirror-circus/src/machine/resource_ops.rs#L640) |
| `condition` | `i.opcode==0x49` | Local state/value operations | Handled | [642](../../crates/crysirror-circus/src/machine/resource_ops.rs#L642) |
| `condition; under i.opcode=0x3c` | `a[0] == 0` | self.state.native_live.stage_image | Handled | [673](../../crates/crysirror-circus/src/machine/resource_ops.rs#L673) |
| `condition; under i.opcode=0x3c` | `(33..=36).contains(&a[0])` | Local state/value operations | Handled | [680](../../crates/crysirror-circus/src/machine/resource_ops.rs#L680) |
| `condition; under i.opcode=0x3c` | `name.starts_with("--") && slot <= 29` | usize::from | Handled | [696](../../crates/crysirror-circus/src/machine/resource_ops.rs#L696) |
| `condition; under i.opcode=0x3c` | `!self.resource_load(host, slot, &name, matches!(slot, 26 \| 27))?` | Local state/value operations | Handled | [721](../../crates/crysirror-circus/src/machine/resource_ops.rs#L721) |
| `condition; under i.opcode=0x3c` | `name.contains("BG\\BLACK") \|\| name.contains("BG\\WHITE")` | self.state.native_flags.insert | Handled | [729](../../crates/crysirror-circus/src/machine/resource_ops.rs#L729) |
| `condition; under i.opcode=0x40` | `!host.sprite(&sprite_ops::SpriteRequest{commands:vec![sprite_ops::SpriteCommand{ seed:self.state.native_live.sprite_seed(63)?,operations:vec![operation], }]})?` | Local state/value operations | Handled | [841](../../crates/crysirror-circus/src/machine/resource_ops.rs#L841) |
| `condition; under i.opcode=0x49 \| 0x4a` | `i.opcode == 0x49` | Local state/value operations | Handled | [906](../../crates/crysirror-circus/src/machine/resource_ops.rs#L906) |
| `i.opcode` | `_` | Local state/value operations | Handled | [913](../../crates/crysirror-circus/src/machine/resource_ops.rs#L913) |
| `condition` | `matches!(i.opcode, 0x3f \| 0x40 \| 0x41)` | self.state.native_live.resource_opcode_completed | Handled | [916](../../crates/crysirror-circus/src/machine/resource_ops.rs#L916) |

### machine/resource_ops.rs / Machine::flush_framebuffer_background

```rust
fn flush_framebuffer_background(&mut self,host:&mut dyn NativeHost) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!self.resource_load(host,1,&name,true)?` | Local state/value operations | Handled | [927](../../crates/crysirror-circus/src/machine/resource_ops.rs#L927) |

### machine/resource_ops.rs / Machine::publish_resources

```rust
fn publish_resources(
        &mut self,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.sprite(&sprite_ops::SpriteRequest { commands: vec![sprite_ops::SpriteCommand { seed, operations: vec![SequenceOperation::Set { channels: vec![3, 4], values: vec![sx, sy], retained: true, }], }], })?` | Local state/value operations | Handled | [974](../../crates/crysirror-circus/src/machine/resource_ops.rs#L974) |
| `condition` | `!host.sprite(&sprite_ops::SpriteRequest { commands: vec![sprite_ops::SpriteCommand { seed, operations: vec![SequenceOperation::Set { channels: vec![0, 1], values: position.iter().map(\|v\| *v as f32).collect(), retained: true, }], }], })?` | Local state/value operations | Handled | [1017](../../crates/crysirror-circus/src/machine/resource_ops.rs#L1017) |

### machine/resource_ops.rs / Machine::resource_copy_object

```rust
fn resource_copy_object(
        &mut self,
        host: &mut dyn NativeHost,
        source: u16,
        target: u16,
        slot: u8,
        advance: bool,
        preserve_texture: bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.sprite(&sprite_ops::SpriteRequest { commands: vec![sprite_ops::SpriteCommand { seed, operations: vec![SequenceOperation::RestoreControl { state, seed_missing: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 10], }], }], })?` | Local state/value operations | Handled | [437](../../crates/crysirror-circus/src/machine/resource_ops.rs#L437) |

### machine/resource_ops.rs / Machine::resource_rollback

```rust
fn resource_rollback(
        &mut self,
        host: &mut dyn NativeHost,
        slot: u8,
        load_slot: u8,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.packed(&packed_ops::PackedRequest::Load { slot: load_slot, name: name[3..].to_owned(), })?` | Local state/value operations | Handled | [302](../../crates/crysirror-circus/src/machine/resource_ops.rs#L302) |
| `condition` | `!host.sprite(&sprite_ops::SpriteRequest { commands: vec![sprite_ops::SpriteCommand { seed, operations: vec![SequenceOperation::RestoreControl { state: control, seed_missing: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 10], }], }], })?` | Local state/value operations | Handled | [342](../../crates/crysirror-circus/src/machine/resource_ops.rs#L342) |

### machine/resource_ops.rs / Machine::resource_variant

```rust
fn resource_variant(&self,slot:u8,name:&str) -> String
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode!=0&&((19..=23).contains(&slot)&&name.get(..4).is_some_and(\|prefix\|prefix.eq_ignore_ascii_case("bst\\"))\|\|(4..=13).contains(&slot))` | Local state/value operations | Handled | [189](../../crates/crysirror-circus/src/machine/resource_ops.rs#L189) |
| `condition` | `matches!(mode,1\|2)` | Local state/value operations | Handled | [194](../../crates/crysirror-circus/src/machine/resource_ops.rs#L194) |
| `condition` | `mode==1` | Local state/value operations | Handled | [194](../../crates/crysirror-circus/src/machine/resource_ops.rs#L194) |
| `condition` | `let Some(at)=name.find(if variant==1{'*'}else{'@'})` | Local state/value operations | Handled | [214](../../crates/crysirror-circus/src/machine/resource_ops.rs#L214) |

### machine/restore_ops.rs / Machine::execute_native_restore

```rust
fn execute_native_restore(
        &mut self,
        i: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<CommandResult>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode == 0x1d` | Local state/value operations | Handled | [21](../../crates/crysirror-circus/src/machine/restore_ops.rs#L21) |
| `condition` | `i.opcode == 0x1d` | Local state/value operations | Handled | [28](../../crates/crysirror-circus/src/machine/restore_ops.rs#L28) |
| `condition` | `!host.emit(&Event::ReleaseAudioGroup { kind, forced: true, steps: 0 })?` | Local state/value operations | Handled | [110](../../crates/crysirror-circus/src/machine/restore_ops.rs#L110) |

### machine/save_ops.rs / Machine::execute_save

```rust
fn execute_save(
        &mut self,
        program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(i.opcode, 0x1c \| 0x1d) \|\| (i.opcode == 0x18 && matches!(a[0], 0 \| 4 \| 24))` | Local state/value operations | Handled | [123](../../crates/crysirror-circus/src/machine/save_ops.rs#L123) |
| `condition; under i.opcode=0x18 if matches!(a[0], 4 \| 24)` | `a[0] == 4 && a[1] != 0 && self.state.dispatch_a2 != 0` | self.state.native_live.syscall_completed | Handled | [177](../../crates/crysirror-circus/src/machine/save_ops.rs#L177) |
| `condition; under i.opcode=0x18 if matches!(a[0], 4 \| 24)` | `a[0] == 24` | Local state/value operations | Handled | [181](../../crates/crysirror-circus/src/machine/save_ops.rs#L181) |
| `condition; under i.opcode=0x18 if matches!(a[0], 4 \| 24)` | `a[0] == 4 && a[1] != 0` | Local state/value operations | Handled | [186](../../crates/crysirror-circus/src/machine/save_ops.rs#L186) |
| `i.opcode` | `0x18 if matches!(a[0], 4 \| 24)` | self.state.native_live.syscall_completed, self.state.value, self.write_native_slot | Handled | [174](../../crates/crysirror-circus/src/machine/save_ops.rs#L174) |
| `i.opcode` | `0x18 if a[0] == 0` | self.execute_native_restore | Handled | [195](../../crates/crysirror-circus/src/machine/save_ops.rs#L195) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [197](../../crates/crysirror-circus/src/machine/save_ops.rs#L197) |

### machine/save_ops.rs / Machine::native_quick_slot

```rust
fn native_quick_slot(&mut self,host:&mut dyn NativeHost,write:bool) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.sprite(&sprite_ops::SpriteRequest{commands:vec![sprite_ops::SpriteCommand{seed:sprite_ops::SpriteSeed{object:150,values:seed.clone(),retained:seed},operations:vec![SequenceOperation::Finish{cancel:false,lanes:None}]}]})?` | Local state/value operations | Handled | [321](../../crates/crysirror-circus/src/machine/save_ops.rs#L321) |

### machine/sprite_ops.rs / Machine::execute_decoration

```rust
fn execute_decoration(
        &mut self,
        i: &Instruction,
        a: [i16; 8],
        host: &mut dyn NativeHost,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `a[0]` | `_` | Vec::new | Handled | [802](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L802) |
| `condition` | `(0..=5).contains(&a[0])` | Local state/value operations | Handled | [817](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L817) |
| `condition; under a[0]=0 \| 1` | `a[0] == 0` | Local state/value operations | Handled | [841](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L841) |
| `condition; under a[0]=0 \| 1` | `a[0] == 1` | Local state/value operations | Handled | [844](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L844) |
| `condition; under a[0]=0 \| 1` | `a[0] == 0` | Local state/value operations | Handled | [850](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L850) |
| `condition; under a[0]=0 \| 1` | `a[0] == 0` | Local state/value operations | Handled | [869](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L869) |
| `condition; under a[0]=3 \| 4` | `a[0] == 3` | Local state/value operations | Handled | [894](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L894) |
| `condition; under a[0]=3 \| 4` | `a[0] == 3` | Local state/value operations | Handled | [896](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L896) |
| `condition; under a[0]=3 \| 4` | `a[0] == 3` | Local state/value operations | Handled | [898](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L898) |
| `condition; under a[0]=3 \| 4` | `a[0] == 3` | Local state/value operations | Handled | [899](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L899) |
| `condition; under a[0]=3 \| 4` | `a[0] == 3` | Local state/value operations | Handled | [901](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L901) |
| `a[0]` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [926](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L926) |

### machine/sprite_ops.rs / Machine::execute_sprite

```rust
fn execute_sprite(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode == 0x38` | self.execute_decoration | Handled | [142](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L142) |
| `condition; under i.opcode=0x26 \| 0xa7 \| 0xa8` | `i.opcode == 0x26` | host.system | Handled | [181](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L181) |
| `condition; under i.opcode=0x26 \| 0xa7 \| 0xa8` | `i.opcode == 0xa8` | u16::try_from | Handled | [190](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L190) |
| `condition; under i.opcode=0x26 \| 0xa7 \| 0xa8` | `a[0] == 255` | Local state/value operations | Handled | [193](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L193) |
| `condition; under i.opcode=0x26 \| 0xa7 \| 0xa8` | `i.opcode == 0xa8` | Local state/value operations | Handled | [209](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L209) |
| `condition; under i.opcode=0x39` | `a[0] > 100` | self.state.native_live.sprite_seed | Handled | [220](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L220) |
| `a[2]; under i.opcode=0x89 \| 0x94 \| 0xa1 / i.opcode=0x89` | `0` | Local state/value operations | Handled | [265](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L265) |
| `a[2]; under i.opcode=0x89 \| 0x94 \| 0xa1 / i.opcode=0x89` | `99` | i32::from | Handled | [266](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L266) |
| `a[2]; under i.opcode=0x89 \| 0x94 \| 0xa1 / i.opcode=0x89` | `_` | i32::from | Handled | [267](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L267) |
| `i.opcode; under i.opcode=0x89 \| 0x94 \| 0xa1` | `_` | u16::try_from, i32::from | Handled | [277](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L277) |
| `condition; under i.opcode=0x8e if a[3] & 0x20 != 0` | `a[0] == 0` | Local state/value operations | Handled | [320](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L320) |
| `condition; under i.opcode=0x8e if a[3] & 0x10 != 0` | `i.opcode == 0x8e && cosine` | Local state/value operations | Handled | [331](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L331) |
| `condition; under i.opcode=0x8e` | `a[3] == 0` | Local state/value operations | Handled | [374](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L374) |
| `condition; under i.opcode=0x8f if a[3] & 0x30 == 0` | `a[3] == 0` | Local state/value operations | Handled | [404](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L404) |
| `condition; under i.opcode=0x9f \| 0xa0` | `i.opcode == 0x9f` | host.sprite, self.sync_sprite_states | Handled | [443](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L443) |
| `condition; under i.opcode=0x9f \| 0xa0` | `i.opcode == 0x9f` | Local state/value operations | Handled | [504](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L504) |
| `condition; under i.opcode=0xb1` | `(0..=100).contains(&a[0])` | host.visual, self.state.native_live.sprite_seed | Handled | [539](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L539) |
| `condition; under i.opcode=0xb1` | `!host.visual(&visual_ops::VisualRequest::Blur { from: a[0] * 10, to: a[0] * 10, ticks: 0, })?` | Local state/value operations | Handled | [540](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L540) |
| `condition; under i.opcode=0xb1` | `(1..100).contains(&a[0])` | self.state.native_live.sprite_seed | Handled | [547](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L547) |
| `condition; under i.opcode=0xb2` | `a[..2].iter().all(\|v\| (0..=100).contains(v))` | u32::from, self.state.native_live.sprite_seed | Handled | [562](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L562) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `0` | Local state/value operations | Handled | [615](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L615) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `1` | Local state/value operations | Handled | [628](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L628) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `2` | Local state/value operations | Handled | [641](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L641) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `3` | Local state/value operations | Handled | [654](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L654) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `_` | Local state/value operations | Handled | [667](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L667) |
| `i.opcode` | `0x41 if !text.starts_with('-') \|\| (text != "-" && text != "--")` | Local state/value operations | Empty handler: Original no-op status not established | [247](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L247) |
| `i.opcode` | `0x4a if matches!(text, ":n" \| ":r")` | self.state.native_flags.insert, i32::from, self.state.callbacks.clear | Handled | [248](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L248) |
| `i.opcode` | `0x8e if a[3] & 0x20 != 0` | u32::try_from | Handled | [319](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L319) |
| `i.opcode` | `0x8e if a[3] & 0x10 != 0` | u16::try_from, i32::from, self.state.native_live.sprite_seed | Handled | [327](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L327) |
| `i.opcode` | `0x8f if a[3] & 0x30 == 0` | u32::try_from, i32::from, self.state.native_live.sprite_seed | Handled | [403](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L403) |
| `i.opcode` | `0x95 if !matches!(a[3], 0..=7)` | self.state.native_live.sprite_seed, u16::try_from | Handled | [593](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L593) |
| `i.opcode` | `0x95 if matches!(a[3], 0..=3)` | u16::try_from, self.state.native_live.sprite_seed | Handled | [611](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L611) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [689](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L689) |
| `i.opcode` | `0x89 if a[2] == 1` | Local state/value operations | Handled | [707](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L707) |
| `i.opcode` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [712](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L712) |
| `condition` | `i.opcode == 0x36` | host.effect_control, self.sync_effect_frame, host.effect_states, host.text | Handled | [715](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L715) |
| `condition` | `i.opcode == 0xb1 && (0..=100).contains(&a[0])` | self.state.native_live.blur_immediate | Handled | [730](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L730) |
| `condition` | `i.opcode == 0x39 && a[0] > 100` | Local state/value operations | Handled | [733](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L733) |

### machine/sprite_ops.rs / Machine::native_skip_notice

```rust
fn native_skip_notice(&mut self, host: &mut dyn NativeHost, mode: i16) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!(0..=2).contains(&mode)` | Local state/value operations | Handled | [107](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L107) |
| `condition` | `mode == 2` | Local state/value operations | Handled | [108](../../crates/crysirror-circus/src/machine/sprite_ops.rs#L108) |

### machine/system_ops.rs / Machine::execute_system

```rust
fn execute_system(
        &mut self,
        program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `a[0]; under i.opcode=0x43` | `_` | Local state/value operations | Handled | [732](../../crates/crysirror-circus/src/machine/system_ops.rs#L732) |
| `condition; under i.opcode=0x43` | `a[0] == 6` | host.system | Handled | [739](../../crates/crysirror-circus/src/machine/system_ops.rs#L739) |
| `condition; under i.opcode=0x43` | `a[0] == 16` | host.system, SystemRequest::CallbackTimer | Handled | [746](../../crates/crysirror-circus/src/machine/system_ops.rs#L746) |
| `condition; under i.opcode=0x43` | `a[0] == 0` | self.state.callbacks.insert, String::new | Handled | [755](../../crates/crysirror-circus/src/machine/system_ops.rs#L755) |
| `condition; under i.opcode=0x43` | `a[0] == 1` | self.state.callbacks.insert, String::new | Handled | [758](../../crates/crysirror-circus/src/machine/system_ops.rs#L758) |
| `condition; under i.opcode=0xa9 / a[3]=0 \| 1 \| 2` | `a[3] == 0` | self.state.value | Handled | [765](../../crates/crysirror-circus/src/machine/system_ops.rs#L765) |
| `a[3]; under i.opcode=0xa9 / a[3]=0 \| 1 \| 2` | `0` | Local state/value operations | Handled | [784](../../crates/crysirror-circus/src/machine/system_ops.rs#L784) |
| `a[3]; under i.opcode=0xa9 / a[3]=0 \| 1 \| 2` | `1` | Local state/value operations | Handled | [785](../../crates/crysirror-circus/src/machine/system_ops.rs#L785) |
| `a[3]; under i.opcode=0xa9 / a[3]=0 \| 1 \| 2` | `_` | Local state/value operations | Handled | [786](../../crates/crysirror-circus/src/machine/system_ops.rs#L786) |
| `a[3]; under i.opcode=0xa9` | `0 \| 1 \| 2` | self.state.value | Handled | [764](../../crates/crysirror-circus/src/machine/system_ops.rs#L764) |
| `a[3]; under i.opcode=0xa9` | `10 \| 11` | Local state/value operations | Handled | [791](../../crates/crysirror-circus/src/machine/system_ops.rs#L791) |
| `a[3]; under i.opcode=0xa9` | `99` | Local state/value operations | Handled | [797](../../crates/crysirror-circus/src/machine/system_ops.rs#L797) |
| `a[3]; under i.opcode=0xa9` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [798](../../crates/crysirror-circus/src/machine/system_ops.rs#L798) |
| `condition; under i.opcode=0x84` | `row.opcode == 2` | Local state/value operations | Handled | [806](../../crates/crysirror-circus/src/machine/system_ops.rs#L806) |
| `condition; under i.opcode=0xb5` | `a[3] == 99` | Local state/value operations | Handled | [832](../../crates/crysirror-circus/src/machine/system_ops.rs#L832) |
| `condition; under i.opcode=0x18 / a[0]=6` | `id >= 0 && *start != 0` | Local state/value operations | Handled | [880](../../crates/crysirror-circus/src/machine/system_ops.rs#L880) |
| `condition; under i.opcode=0x18 / a[0]=6` | `a[1] == 1` | host.scene_index, self.state.native_live.discard_scene_read_history | Handled | [886](../../crates/crysirror-circus/src/machine/system_ops.rs#L886) |
| `condition; under i.opcode=0x18 / a[0]=20 \| 29` | `a[0] == 29` | Local state/value operations | Handled | [937](../../crates/crysirror-circus/src/machine/system_ops.rs#L937) |
| `condition; under i.opcode=0x18 / a[0]=20 \| 29` | `!self.poll_preference_reset(host, i, a[0], a[1], replacement, replay, None)?` | Local state/value operations | Handled | [940](../../crates/crysirror-circus/src/machine/system_ops.rs#L940) |
| `a[0]; under i.opcode=0x18 / a[0]=22 \| 23 \| 25` | `_` | Local state/value operations | Handled | [948](../../crates/crysirror-circus/src/machine/system_ops.rs#L948) |
| `a[0]; under i.opcode=0x18` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [953](../../crates/crysirror-circus/src/machine/system_ops.rs#L953) |
| `condition; under i.opcode=0x1e / a[0]=6` | `a[1] != 0` | Local state/value operations | Handled | [970](../../crates/crysirror-circus/src/machine/system_ops.rs#L970) |
| `condition; under i.opcode=0x1e / a[0]=9` | `a[1] == 1` | Local state/value operations | Handled | [993](../../crates/crysirror-circus/src/machine/system_ops.rs#L993) |
| `a[1]; under i.opcode=0x1e / a[0]=18` | `_` | Local state/value operations | Handled | [1018](../../crates/crysirror-circus/src/machine/system_ops.rs#L1018) |
| `condition; under i.opcode=0x1e / a[0]=98` | `!self.poll_preference_reset(host,i,98,a[1],replacement,replay,None)?` | Local state/value operations | Handled | [1069](../../crates/crysirror-circus/src/machine/system_ops.rs#L1069) |
| `condition; under i.opcode=0x1e / a[0]=200` | `!self.music_gallery(a[1], host)?` | Local state/value operations | Handled | [1090](../../crates/crysirror-circus/src/machine/system_ops.rs#L1090) |
| `condition; under i.opcode=0x1e / a[0]=201 \| 202` | `a[0] == 201 && (a[1] == 254 \|\| a[1] == 255)` | self.state.system_data.bytes, i32::from, SystemData::try_from | Handled | [1095](../../crates/crysirror-circus/src/machine/system_ops.rs#L1095) |
| `condition; under i.opcode=0x1e / a[0]=201 \| 202` | `a[0] == 201` | self.set_music_unlocked, self.music_unlocked | Handled | [1110](../../crates/crysirror-circus/src/machine/system_ops.rs#L1110) |
| `condition; under i.opcode=0x1e / a[0]=203` | `name.contains("io_")` | Local state/value operations | Handled | [1190](../../crates/crysirror-circus/src/machine/system_ops.rs#L1190) |
| `condition; under i.opcode=0x1e / a[0]=203` | `name.contains("1_a") \|\| name.contains("1_b")` | Local state/value operations | Handled | [1191](../../crates/crysirror-circus/src/machine/system_ops.rs#L1191) |
| `condition; under i.opcode=0x1e / a[0]=203` | `name.contains("io_e")` | Local state/value operations | Handled | [1194](../../crates/crysirror-circus/src/machine/system_ops.rs#L1194) |
| `condition; under i.opcode=0x1e / a[0]=203` | `!host.sprite(&sprite_ops::SpriteRequest{commands:vec![sprite_ops::SpriteCommand{ seed:self.state.native_live.sprite_seed(149)?, operations:vec![crysirror_scene::layer::queued::SequenceOperation::Reset{values:reset.values,retained:reset.retained}], }]})?` | Local state/value operations | Handled | [1202](../../crates/crysirror-circus/src/machine/system_ops.rs#L1202) |
| `condition; under i.opcode=0x1e / a[0]=203` | `!self.resource_load(host,4,&name,false)?` | Local state/value operations | Handled | [1211](../../crates/crysirror-circus/src/machine/system_ops.rs#L1211) |
| `condition; under i.opcode=0x1e / a[0]=204 / a[1]=2 \| 3` | `a[1] == 2` | Local state/value operations | Handled | [1237](../../crates/crysirror-circus/src/machine/system_ops.rs#L1237) |
| `a[1]; under i.opcode=0x1e / a[0]=204` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1253](../../crates/crysirror-circus/src/machine/system_ops.rs#L1253) |
| `condition; under i.opcode=0x1e / a[0]=204` | `limit.is_some_and(\|limit\| value >= limit) \|\| (a[1] == 4 && row[8] == 0) \|\| a[1] == 5` | self.state.assign | Handled | [1256](../../crates/crysirror-circus/src/machine/system_ops.rs#L1256) |
| `a[0]; under i.opcode=0x1e` | `7 if a[1] == 0` | Local state/value operations | Empty handler: Original no-op status not established | [973](../../crates/crysirror-circus/src/machine/system_ops.rs#L973) |
| `a[0]; under i.opcode=0x1e` | `7 if matches!(self.state.system_ops.panel_mode,1\|6)` | Local state/value operations | Handled | [974](../../crates/crysirror-circus/src/machine/system_ops.rs#L974) |
| `a[0]; under i.opcode=0x1e` | `51 if a[1]==1` | host.system, self.state.protagonist_names.iter, encoding_rs::SHIFT_JIS.encode, self.state.native_flags.insert, i32::from | Handled | [1036](../../crates/crysirror-circus/src/machine/system_ops.rs#L1036) |
| `a[0]; under i.opcode=0x1e` | `_` | Local state/value operations | Empty handler: Original no-op status not established | [1294](../../crates/crysirror-circus/src/machine/system_ops.rs#L1294) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [1296](../../crates/crysirror-circus/src/machine/system_ops.rs#L1296) |
| `condition; under reply=SystemReply::Done` | `i.opcode == 0xa9 && a[3] == 0` | self.state.assign | Handled | [1310](../../crates/crysirror-circus/src/machine/system_ops.rs#L1310) |
| `condition; under reply=SystemReply::Account { status, system, .. }` | `i.opcode == 0x18` | i32::from | Handled | [1332](../../crates/crysirror-circus/src/machine/system_ops.rs#L1332) |
| `condition` | `i.opcode==0x1e && a[0]==203` | self.sync_sprite_states, host.image_dimensions, self.state.native_live.bind_resource, self.state.native_live.character_preview_completed | Handled | [1358](../../crates/crysirror-circus/src/machine/system_ops.rs#L1358) |
| `condition` | `i.opcode==0x1e && self.state.native_flags.get(&272075).copied().unwrap_or(0)!=0` | self.state.dialogue.panel.finalized, host.system, self.state.value, self.state.native_flags.insert | Handled | [1366](../../crates/crysirror-circus/src/machine/system_ops.rs#L1366) |
| `condition` | `i.opcode==0xa9` | self.state.native_live.audio_transport_completed | Handled | [1374](../../crates/crysirror-circus/src/machine/system_ops.rs#L1374) |

### machine/system_ops.rs / Machine::poll_preference_reset

```rust
fn poll_preference_reset(
        &mut self, host: &mut dyn NativeHost, i: &Instruction, selector: i16,
        parameter: i16, replacement: Option<SystemData>, replay: bool,
        names: Option<[String; 2]>,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 29 && parameter == 1` | self.state.assign, self.state.system_data.dword | Handled | [278](../../crates/crysirror-circus/src/machine/system_ops.rs#L278) |

### machine/system_ops.rs / Machine::poll_system_callbacks

```rust
fn poll_system_callbacks(
        &mut self,
        host: &mut dyn NativeHost,
        now: u64,
        _pressed: bool,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!music_enabled&&!pending_music&&!self.state.callbacks.iter().any(\|(_,name)\|!name.is_empty()&&name!="-")` | Local state/value operations | Handled | [564](../../crates/crysirror-circus/src/machine/system_ops.rs#L564) |
| `condition` | `trigger.is_none()&&pressed & mask != 0 && self .state .callbacks .get(&offset) .is_some_and(\|name\| !name.is_empty() && name != "-")` | Local state/value operations | Handled | [592](../../crates/crysirror-circus/src/machine/system_ops.rs#L592) |
| `condition` | `trigger.is_none()&&panel_scroll&&self.state.callbacks.get(&11288).is_some_and(\|name\|!name.is_empty())` | Local state/value operations | Handled | [605](../../crates/crysirror-circus/src/machine/system_ops.rs#L605) |
| `condition` | `trigger.is_none()&&!music_enabled && self .state .callbacks .get(&10968) .is_some_and(\|name\| !name.is_empty() && name != "-")` | Local state/value operations | Handled | [619](../../crates/crysirror-circus/src/machine/system_ops.rs#L619) |
| `condition` | `trigger.is_none()&&!music_enabled && idle && self .state .callbacks .get(&11352) .is_some_and(\|name\| !name.is_empty() && name != "-")` | host.system, SystemRequest::CallbackTimer | Handled | [633](../../crates/crysirror-circus/src/machine/system_ops.rs#L633) |
| `condition` | `let Some(target) = program.instructions.iter().find(\|i\| { i.opcode == 0x4b && matches!(&i.arguments,Arguments::String(label)if label==name) })` | Local state/value operations | Handled | [673](../../crates/crysirror-circus/src/machine/system_ops.rs#L673) |

### machine/text_ops.rs / Machine::execute_text

```rust
fn execute_text(
        &mut self,
        program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `i.opcode == 0x3e` | Local state/value operations | Handled | [200](../../crates/crysirror-circus/src/machine/text_ops.rs#L200) |
| `condition` | `i.opcode==0x7b&&a[1]<=256&&self.state.native_last_opcode==Some(0x87)` | self.state.builder.texts.len, self.state.value, encoding_rs::SHIFT_JIS.encode, i32::try_from, self.state.system_data.dword | Handled | [207](../../crates/crysirror-circus/src/machine/text_ops.rs#L207) |
| `condition` | `i.opcode==0x1a&&a[0]<=0` | host.emit, Box::new, host.text, self.state.callbacks.retain, self.state.selected_builder | Handled | [225](../../crates/crysirror-circus/src/machine/text_ops.rs#L225) |
| `condition` | `i.opcode == 0x0c` | self.builder_stage, self.state.builder.clone, self.resolve_group_values, host.emit, Event::CopyUiToPanel | Handled | [248](../../crates/crysirror-circus/src/machine/text_ops.rs#L248) |
| `condition` | `i.opcode == 0x0c` | Local state/value operations | Handled | [255](../../crates/crysirror-circus/src/machine/text_ops.rs#L255) |
| `condition; under i.opcode=0x22` | `a[0]!=0` | self.builder_stage, self.state.builder.clone, self.resolve_group_values, host.emit, Event::CopyUiToPanel | Handled | [309](../../crates/crysirror-circus/src/machine/text_ops.rs#L309) |
| `condition; under i.opcode=0x33` | `a[0] == 0` | Local state/value operations | Handled | [331](../../crates/crysirror-circus/src/machine/text_ops.rs#L331) |
| `condition; under i.opcode=0x45` | `matches!(a[0], 0 \| 1)` | encoding_rs::SHIFT_JIS.encode | Handled | [365](../../crates/crysirror-circus/src/machine/text_ops.rs#L365) |
| `condition; under i.opcode=0x24` | `matches!(a[0], 254 \| 255)` | host.emit, self.scoped_ui, Event::GroupReverseOrder, self.state.builder.behavior_flags.insert, self.state.builder.behavior_flags.remove | Handled | [378](../../crates/crysirror-circus/src/machine/text_ops.rs#L378) |
| `condition; under i.opcode=0x24` | `!host.emit(&self.scoped_ui(Event::GroupReverseOrder(a[0]==254)))?` | Local state/value operations | Handled | [379](../../crates/crysirror-circus/src/machine/text_ops.rs#L379) |
| `condition; under i.opcode=0x24` | `a[0] == 254` | self.state.builder.behavior_flags.insert | Handled | [380](../../crates/crysirror-circus/src/machine/text_ops.rs#L380) |
| `condition; under i.opcode=0x24` | `let Some(slot) = match a[0] { 1..=7 => Some(a[0] as u8 - 1), 13 \| 14 => Some(a[0] as u8 + 4), 15 \| 16 => Some(a[0] as u8 - 8), _ => None, }` | self.state.select_builder, self.state.builder.clone, host.emit, self.scoped_ui, self.state.groups.insert | Handled | [385](../../crates/crysirror-circus/src/machine/text_ops.rs#L385) |
| `a[0]; under i.opcode=0x24` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [389](../../crates/crysirror-circus/src/machine/text_ops.rs#L389) |
| `condition; under i.opcode=0x59 \| 0x5a \| 0x5b` | `i.opcode == 0x59 && self.state.system_data.dword(4410 + selector as usize) != 0` | Local state/value operations | Handled | [455](../../crates/crysirror-circus/src/machine/text_ops.rs#L455) |
| `i.opcode; under i.opcode=0x59 \| 0x5a \| 0x5b` | `_` | Local state/value operations | Handled | [467](../../crates/crysirror-circus/src/machine/text_ops.rs#L467) |
| `condition; under i.opcode=0x59 \| 0x5a \| 0x5b` | `i.opcode==0x5a` | Local state/value operations | Handled | [474](../../crates/crysirror-circus/src/machine/text_ops.rs#L474) |
| `condition; under i.opcode=0x59 \| 0x5a \| 0x5b` | `i.opcode==0x5a` | Local state/value operations | Handled | [474](../../crates/crysirror-circus/src/machine/text_ops.rs#L474) |
| `condition; under i.opcode=0x74` | `a[7] == -1` | Local state/value operations | Handled | [641](../../crates/crysirror-circus/src/machine/text_ops.rs#L641) |
| `condition; under i.opcode=0x7c` | `a[3] == 0` | Local state/value operations | Handled | [682](../../crates/crysirror-circus/src/machine/text_ops.rs#L682) |
| `condition; under i.opcode=0x80` | `a[3] == 99` | Local state/value operations | Handled | [702](../../crates/crysirror-circus/src/machine/text_ops.rs#L702) |
| `condition; under i.opcode=0x80` | `(0..3).contains(&a[3]) && self.state.dialogue.panel_image_declarations.len() < 3` | self.state.dialogue.panel_texts.push | Handled | [716](../../crates/crysirror-circus/src/machine/text_ops.rs#L716) |
| `condition; under i.opcode=0x87` | `a[2] != 0` | self.state.builder.texts.push, self.state.builder.pushed_text, self.state.builder.texts.len | Handled | [740](../../crates/crysirror-circus/src/machine/text_ops.rs#L740) |
| `condition; under i.opcode=0x87` | `a[2] < 99 && a[2] != 0` | Local state/value operations | Handled | [748](../../crates/crysirror-circus/src/machine/text_ops.rs#L748) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [758](../../crates/crysirror-circus/src/machine/text_ops.rs#L758) |

### machine/text_ops.rs / Machine::finish_native_dialogue_wait

```rust
fn finish_native_dialogue_wait(&mut self,host:&mut dyn NativeHost) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `self.state.value(99,208)?!=0&&!host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:0})?` | Local state/value operations | Handled | [178](../../crates/crysirror-circus/src/machine/text_ops.rs#L178) |

### machine/text_ops.rs / Machine::poll_text_events_mode

```rust
fn poll_text_events_mode(
        &mut self,
        host: &mut dyn NativeHost,
        now: u64,
        mode: u8,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `let Some(linked) = match selector { 2 => Some(3), 3 => Some(2), 9 => Some(10), 10 => Some(9), 1 => Some(11), 11 => Some(1), _ => None, }` | self.toggle_gallery | Handled | [824](../../crates/crysirror-circus/src/machine/text_ops.rs#L824) |
| `selector` | `2` | Local state/value operations | Handled | [825](../../crates/crysirror-circus/src/machine/text_ops.rs#L825) |
| `selector` | `3` | Local state/value operations | Handled | [826](../../crates/crysirror-circus/src/machine/text_ops.rs#L826) |
| `selector` | `9` | Local state/value operations | Handled | [827](../../crates/crysirror-circus/src/machine/text_ops.rs#L827) |
| `selector` | `10` | Local state/value operations | Handled | [828](../../crates/crysirror-circus/src/machine/text_ops.rs#L828) |
| `selector` | `1` | Local state/value operations | Handled | [829](../../crates/crysirror-circus/src/machine/text_ops.rs#L829) |
| `selector` | `11` | Local state/value operations | Handled | [830](../../crates/crysirror-circus/src/machine/text_ops.rs#L830) |
| `selector` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [831](../../crates/crysirror-circus/src/machine/text_ops.rs#L831) |
| `condition` | `event.kind == 36` | self.replay_macro, self.state.dialogue.pending_inline_events.remove | Handled | [882](../../crates/crysirror-circus/src/machine/text_ops.rs#L882) |
| `condition` | `(1..=5).contains(&event.kind)` | self.state.image_names.get, self.native_cg_index | Handled | [899](../../crates/crysirror-circus/src/machine/text_ops.rs#L899) |
| `condition` | `event.kind == 13` | host.sound_name, u16::try_from, host.publish_audio | Handled | [905](../../crates/crysirror-circus/src/machine/text_ops.rs#L905) |
| `condition; under receiver=else: event.kind == 13` | `(14..=35).contains(&event.kind)` | self.execute_inline_effect | Handled | [927](../../crates/crysirror-circus/src/machine/text_ops.rs#L927) |
| `condition; under receiver=else: event.kind == 13` | `(6..=10).contains(&event.kind)` | u8::try_from, super::visual_ops::resource_object, self.state.dialogue.pending_inline_events.remove, self.prepare_inline_transfer | Handled | [932](../../crates/crysirror-circus/src/machine/text_ops.rs#L932) |
| `condition; under receiver=else: event.kind == 13` | `!self.prepare_inline_transfer(host,19+event.kind-6,target)?` | Local state/value operations | Handled | [935](../../crates/crysirror-circus/src/machine/text_ops.rs#L935) |
| `condition` | `(1..=10).contains(&event.kind)` | u8::try_from, self.state.image_names.insert, host.image_dimensions | Handled | [940](../../crates/crysirror-circus/src/machine/text_ops.rs#L940) |
| `condition` | `event.kind <= 5` | Local state/value operations | Handled | [942](../../crates/crysirror-circus/src/machine/text_ops.rs#L942) |
| `condition` | `event.kind == 11` | u16::try_from, self.state.native_live.music | Handled | [958](../../crates/crysirror-circus/src/machine/text_ops.rs#L958) |
| `condition; under receiver=else: event.kind == 11` | `event.kind == 12` | self.state.native_live.music | Handled | [967](../../crates/crysirror-circus/src/machine/text_ops.rs#L967) |

### machine/text_ops/inline.rs / deferred_in_mode_two

```rust
fn deferred_in_mode_two(kind: u8) -> bool
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind,14..=17\|19..=22\|34\|35)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [7](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L7) |

### machine/text_ops/inline.rs / Machine::execute_inline_effect

```rust
fn execute_inline_effect(
        &mut self,
        event: &InlineEvent,
        host: &mut dyn NativeHost,
    ) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 18 \| 23 \| 28 \| 33)` | Local state/value operations | Handled | [18](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L18) |
| `condition` | `matches!(kind, 18 \| 23 \| 28 \| 33)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [18](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L18) |
| `condition` | `matches!(kind, 29..=32)` | self.state.image_names.insert, String::new, self.state.native_live.image_name, self.state.native_skipping, self.state.native_live.unload_image | Handled | [21](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L21) |
| `condition` | `matches!(kind, 29..=32)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [21](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L21) |
| `kind` | `14..=17` | Local state/value operations | Handled | [43](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L43) |
| `kind` | `19..=22` | Local state/value operations | Handled | [44](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L44) |
| `kind` | `24..=27` | Local state/value operations | Handled | [45](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L45) |
| `kind` | `29..=32` | Local state/value operations | Handled | [46](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L46) |
| `kind` | `34 \| 35` | Local state/value operations | Handled | [47](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L47) |
| `kind` | `_` | Local state/value operations | Unsupported: Circus inline effect kind{kind} | [48](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L48) |
| `condition` | `matches!(kind,24..=27\|29..=32)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [50](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L50) |
| `condition` | `!host.effect_control(&crate::machine::effect_ops::EffectRequest::SpriteMask { seed: self.state.native_live.sprite_seed(object)?, mode: kind < 29, ticks: 60, })?` | Local state/value operations | Handled | [84](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L84) |
| `condition` | `matches!(kind, 14..=17 \| 35)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [93](../../crates/crysirror-circus/src/machine/text_ops/inline.rs#L93) |

### machine/text_ops/menu_entry.rs / Machine::execute_menu_entry

```rust
fn execute_menu_entry(
        &mut self,
        program: &Program,
        i: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<CommandResult>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!host.emit(&Event::ReleaseAudioGroup { kind: 1, forced: false, steps: 30, })?` | Local state/value operations | Handled | [117](../../crates/crysirror-circus/src/machine/text_ops/menu_entry.rs#L117) |

### machine/visual_ops.rs / Machine::execute_visual

```rust
fn execute_visual(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition; under i.opcode=0x1b` | `a[0] == 0` | Local state/value operations | Handled | [62](../../crates/crysirror-circus/src/machine/visual_ops.rs#L62) |
| `condition; under i.opcode=0x89` | `a[2] == 1` | Local state/value operations | Handled | [71](../../crates/crysirror-circus/src/machine/visual_ops.rs#L71) |
| `a[2]; under i.opcode=0x89` | `0` | Local state/value operations | Handled | [79](../../crates/crysirror-circus/src/machine/visual_ops.rs#L79) |
| `a[2]; under i.opcode=0x89` | `99` | i32::from | Handled | [80](../../crates/crysirror-circus/src/machine/visual_ops.rs#L80) |
| `a[2]; under i.opcode=0x89` | `_` | i32::from | Handled | [81](../../crates/crysirror-circus/src/machine/visual_ops.rs#L81) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `0` | Local state/value operations | Handled | [100](../../crates/crysirror-circus/src/machine/visual_ops.rs#L100) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `1` | f32::from | Handled | [106](../../crates/crysirror-circus/src/machine/visual_ops.rs#L106) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `2` | f32::from | Handled | [112](../../crates/crysirror-circus/src/machine/visual_ops.rs#L112) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `3` | f32::from | Handled | [118](../../crates/crysirror-circus/src/machine/visual_ops.rs#L118) |
| `a[3]; under i.opcode=0x95 if matches!(a[3], 0..=3)` | `_` | Local state/value operations | Handled | [124](../../crates/crysirror-circus/src/machine/visual_ops.rs#L124) |
| `condition; under i.opcode=0xb1` | `(0..=100).contains(&a[0])` | Local state/value operations | Handled | [131](../../crates/crysirror-circus/src/machine/visual_ops.rs#L131) |
| `condition; under i.opcode=0xb2` | `a[..2].iter().all(\|value\|(0..=100).contains(value))` | u32::try_from | Handled | [132](../../crates/crysirror-circus/src/machine/visual_ops.rs#L132) |
| `condition; under i.opcode=0xb3` | `(0..4).contains(&a[0])` | Local state/value operations | Handled | [135](../../crates/crysirror-circus/src/machine/visual_ops.rs#L135) |
| `i.opcode` | `0x95 if matches!(a[3], 0..=3)` | f32::from | Handled | [97](../../crates/crysirror-circus/src/machine/visual_ops.rs#L97) |
| `i.opcode` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [145](../../crates/crysirror-circus/src/machine/visual_ops.rs#L145) |

### native_control.rs / alignment_maps

```rust
fn alignment_maps(record:&[u8]) -> Result<[crysirror_scene::layer::queued::ValueMap;2]>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode==0` | Local state/value operations | Handled | [311](../../crates/crysirror-circus/src/native_control.rs#L311) |
| `condition` | `mode==2` | Local state/value operations | Handled | [313](../../crates/crysirror-circus/src/native_control.rs#L313) |

### native_control.rs / from_record

```rust
fn from_record(record: &[u8], current: &SequenceStatus) -> Result<SequenceStatus>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `id == 7` | Local state/value operations | Handled | [216](../../crates/crysirror-circus/src/native_control.rs#L216) |

### native_live.rs / NativeLiveState::audit_completed

```rust
fn audit_completed(&mut self, opcode: u8, scene: &str, pc: u32) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `!projected && matches!( opcode, 0x0f \| 0x36 \| 0x38 \| 0x3f \| 0x40 \| 0x41 \| 0x4a \| 0x52 \| 0x89 \| 0x8e \| 0x8f \| 0x95 \| 0x9a \| 0x9b \| 0x9c \| 0x9d \| 0xa8 )` | Local state/value operations | Handled | [2398](../../crates/crysirror-circus/src/native_live.rs#L2398) |
| `condition` | `matches!( opcode, 0x0f \| 0x36 \| 0x38 \| 0x3f \| 0x40 \| 0x41 \| 0x4a \| 0x52 \| 0x89 \| 0x8e \| 0x8f \| 0x95 \| 0x9a \| 0x9b \| 0x9c \| 0x9d \| 0xa8 )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [2399](../../crates/crysirror-circus/src/native_live.rs#L2399) |
| `condition` | `!projected && !matches!(opcode,0x00..=0x04\|0x06\|0x08\|0x0b\|0x0d\|0x0e\|0x11\|0x12\|0x13\|0x14\|0x15\|0x16\|0x17\|0x19\|0x1a\|0x1c\|0x1f\|0x20\|0x23\|0x24\|0x25\|0x27\| 0x29..=0x31\|0x3a\|0x3b\|0x42\|0x48\|0x4b\|0x53\|0x54\|0x57\|0x5d\|0x60\|0x61\|0x62\|0x64\|0x66\|0x69\|0x6c\|0x6e\|0x8b..=0x8d\| 0x32\|0x34\|0x35\|0x43\|0x46\|0x47\|0x51\|0x63\|0x65\|0x68\|0x70\|0x72\|0x73\|0x76\|0x78\|0x7a\|0x7b\|0x82\|0x83\|0x85\|0x86\|0x87\|0x90\|0x91\|0x92\|0x93\|0x97\|0x98\|0x99\|0x9e\|0xa3\|0xa4\|0xa5\|0xa6\|0xaa\|0xae\|0xaf\|0xb0)` | self.gap | Handled | [2430](../../crates/crysirror-circus/src/native_live.rs#L2430) |
| `condition` | `matches!(opcode,0x00..=0x04\|0x06\|0x08\|0x0b\|0x0d\|0x0e\|0x11\|0x12\|0x13\|0x14\|0x15\|0x16\|0x17\|0x19\|0x1a\|0x1c\|0x1f\|0x20\|0x23\|0x24\|0x25\|0x27\| 0x29..=0x31\|0x3a\|0x3b\|0x42\|0x48\|0x4b\|0x53\|0x54\|0x57\|0x5d\|0x60\|0x61\|0x62\|0x64\|0x66\|0x69\|0x6c\|0x6e\|0x8b..=0x8d\| 0x32\|0x34\|0x35\|0x43\|0x46\|0x47\|0x51\|0x63\|0x65\|0x68\|0x70\|0x72\|0x73\|0x76\|0x78\|0x7a\|0x7b\|0x82\|0x83\|0x85\|0x86\|0x87\|0x90\|0x91\|0x92\|0x93\|0x97\|0x98\|0x99\|0x9e\|0xa3\|0xa4\|0xa5\|0xa6\|0xaa\|0xae\|0xaf\|0xb0)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [2431](../../crates/crysirror-circus/src/native_live.rs#L2431) |

### native_live.rs / NativeLiveState::dialogue_nonzero_completed

```rust
fn dialogue_nonzero_completed(&mut self, selector: i16, dispatch: i32, forced: bool, label_replay: bool) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector != 0 && dispatch == 0 && !forced && !label_replay` | self.projection_receipts.insert | Handled | [660](../../crates/crysirror-circus/src/native_live.rs#L660) |

### native_live.rs / NativeLiveState::falling_effect

```rust
fn falling_effect(&mut self, kind: i16, parameter: i16) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `kind == 0` | self.write32 | Handled | [2366](../../crates/crysirror-circus/src/native_live.rs#L2366) |
| `condition; under receiver=else: kind == 0` | `matches!(kind, 2 \| 7 \| 13 \| 15 \| 16 \| 17)` | self.write32, u32::from, i32::from | Handled | [2370](../../crates/crysirror-circus/src/native_live.rs#L2370) |
| `condition; under receiver=else: kind == 0` | `matches!(kind, 2 \| 7 \| 13 \| 15 \| 16 \| 17)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [2370](../../crates/crysirror-circus/src/native_live.rs#L2370) |
| `kind; under receiver=else: kind == 0` | `2` | Local state/value operations | Handled | [2371](../../crates/crysirror-circus/src/native_live.rs#L2371) |
| `kind; under receiver=else: kind == 0` | `7` | Local state/value operations | Handled | [2371](../../crates/crysirror-circus/src/native_live.rs#L2371) |
| `kind; under receiver=else: kind == 0` | `13 \| 17` | Local state/value operations | Handled | [2371](../../crates/crysirror-circus/src/native_live.rs#L2371) |
| `kind; under receiver=else: kind == 0` | `15` | Local state/value operations | Handled | [2371](../../crates/crysirror-circus/src/native_live.rs#L2371) |
| `kind; under receiver=else: kind == 0` | `16` | Local state/value operations | Handled | [2371](../../crates/crysirror-circus/src/native_live.rs#L2371) |
| `kind; under receiver=else: kind == 0` | `_` | Local state/value operations | Handled | [2371](../../crates/crysirror-circus/src/native_live.rs#L2371) |
| `condition; under receiver=else: kind == 0` | `matches!(kind, 7 \| 16)` | u32::from | Handled | [2372](../../crates/crysirror-circus/src/native_live.rs#L2372) |
| `condition; under receiver=else: kind == 0` | `matches!(kind, 7 \| 16)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [2372](../../crates/crysirror-circus/src/native_live.rs#L2372) |

### native_live.rs / NativeLiveState::framebuffer_color_effect

```rust
fn framebuffer_color_effect(&mut self, kind: i16) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 0 \| 1 \| 2 \| 3 \| 9 \| 15 \| 16 \| 17 \| 20 \| 21)` | self.projection_receipts.insert | Handled | [1277](../../crates/crysirror-circus/src/native_live.rs#L1277) |
| `condition` | `matches!(kind, 0 \| 1 \| 2 \| 3 \| 9 \| 15 \| 16 \| 17 \| 20 \| 21)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1277](../../crates/crysirror-circus/src/native_live.rs#L1277) |

### native_live.rs / NativeLiveState::framebuffer_queue_effect

```rust
fn framebuffer_queue_effect(&mut self, kind: i16) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 4 \| 5 \| 6 \| 51 \| 52) && self.known_queues.is_some()` | self.projection_receipts.insert | Handled | [1313](../../crates/crysirror-circus/src/native_live.rs#L1313) |
| `condition` | `matches!(kind, 4 \| 5 \| 6 \| 51 \| 52)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1313](../../crates/crysirror-circus/src/native_live.rs#L1313) |

### native_live.rs / NativeLiveState::ignored_framebuffer_effect

```rust
fn ignored_framebuffer_effect(&mut self, kind: i16) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(kind, 7 \| 10 \| 11 \| 13 \| 14 \| 23 \| 24 \| 25 \| 30 \| 31 \| 32)` | self.projection_receipts.insert | Handled | [1165](../../crates/crysirror-circus/src/native_live.rs#L1165) |
| `condition` | `matches!(kind, 7 \| 10 \| 11 \| 13 \| 14 \| 23 \| 24 \| 25 \| 30 \| 31 \| 32)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1165](../../crates/crysirror-circus/src/native_live.rs#L1165) |

### native_live.rs / NativeLiveState::media_completed

```rust
fn media_completed(&mut self, opcode: u8, selector: i16) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x6f \|\| opcode == 0x71 \|\| opcode == 0x05 && selector != 254` | self.projection_receipts.insert | Handled | [746](../../crates/crysirror-circus/src/native_live.rs#L746) |

### native_live.rs / NativeLiveState::object_position

```rust
fn object_position(
        &mut self,
        opcode: u8,
        object: u16,
        position: [i32; 2],
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode != 0xa1` | self.remember_position, usize::from | Handled | [1663](../../crates/crysirror-circus/src/native_live.rs#L1663) |

### native_live.rs / NativeLiveState::object_track_target

```rust
fn object_track_target(
        &mut self,
        opcode: u8,
        object: u16,
        kind: u8,
        values: [f32; 2],
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `kind` | `0` | Local state/value operations | Handled | [1732](../../crates/crysirror-circus/src/native_live.rs#L1732) |
| `kind` | `1` | Local state/value operations | Handled | [1736](../../crates/crysirror-circus/src/native_live.rs#L1736) |
| `kind` | `2` | Local state/value operations | Handled | [1737](../../crates/crysirror-circus/src/native_live.rs#L1737) |
| `kind` | `3` | Local state/value operations | Handled | [1738](../../crates/crysirror-circus/src/native_live.rs#L1738) |
| `kind` | `_` | Local state/value operations | Handled | [1739](../../crates/crysirror-circus/src/native_live.rs#L1739) |

### native_live.rs / NativeLiveState::runtime_system_value

```rust
fn runtime_system_value(&self,selector:i16) -> Option<i32>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector` | `183` | Local state/value operations | Handled | [546](../../crates/crysirror-circus/src/native_live.rs#L546) |
| `selector` | `233` | Local state/value operations | Handled | [546](../../crates/crysirror-circus/src/native_live.rs#L546) |
| `selector` | `234` | Local state/value operations | Handled | [546](../../crates/crysirror-circus/src/native_live.rs#L546) |
| `selector` | `241` | Local state/value operations | Handled | [546](../../crates/crysirror-circus/src/native_live.rs#L546) |
| `selector` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [546](../../crates/crysirror-circus/src/native_live.rs#L546) |

### native_live.rs / NativeLiveState::stage_opcode_completed

```rust
fn stage_opcode_completed(&mut self, opcode: u8, noop: bool) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x88 \|\| opcode == 0x28 \|\| noop \|\| self.known_queues.is_some()` | self.projection_receipts.insert | Handled | [1880](../../crates/crysirror-circus/src/native_live.rs#L1880) |

### native_live.rs / NativeLiveState::syscall_completed

```rust
fn syscall_completed(&mut self, selector: i16) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!(selector,1..=29\|99) \|\| !matches!(selector,0..=29\|99)` | self.projection_receipts.insert | Handled | [1128](../../crates/crysirror-circus/src/native_live.rs#L1128) |
| `condition` | `matches!(selector,1..=29\|99)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1128](../../crates/crysirror-circus/src/native_live.rs#L1128) |
| `condition` | `matches!(selector,0..=29\|99)` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1128](../../crates/crysirror-circus/src/native_live.rs#L1128) |

### native_live.rs / NativeLiveState::system_function_completed

```rust
fn system_function_completed(&mut self, selector: i16, parameter: i16) -> ()
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `matches!( selector, 1 \| 2 \| 6 \| 7 \| 9 \| 18 \| 19 \| 20 \| 98 \| 99 \| 105 \| 200 \| 201 \| 202 \| 204 \| 206 \| 211 ) \|\| selector == 51 && parameter != 1` | self.projection_receipts.insert | Handled | [1156](../../crates/crysirror-circus/src/native_live.rs#L1156) |
| `condition` | `matches!( selector, 1 \| 2 \| 6 \| 7 \| 9 \| 18 \| 19 \| 20 \| 98 \| 99 \| 105 \| 200 \| 201 \| 202 \| 204 \| 206 \| 211 )` | Local state/value operations | Guard / allowlist: Predicate only; execution is defined by its caller | [1156](../../crates/crysirror-circus/src/native_live.rs#L1156) |

### native_live.rs / NativeLiveState::transfer_scene

```rust
fn transfer_scene(
        &mut self,
        opcode: u8,
        target: &str,
        previous_index: i32,
        ui_exists: [bool; 2],
        protected: bool,
        animations_finalized: bool,
    ) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `opcode == 0x4f` | Local state/value operations | Handled | [1546](../../crates/crysirror-circus/src/native_live.rs#L1546) |
| `condition` | `opcode == 0x50 \|\| previous_index >= 0` | self.record_scene_bank_changes, self.write32, self.read32 | Handled | [1550](../../crates/crysirror-circus/src/native_live.rs#L1550) |
| `condition` | `opcode == 0x4f` | self.write32, self.read32 | Handled | [1552](../../crates/crysirror-circus/src/native_live.rs#L1552) |
| `condition` | `opcode == 0x4f` | self.known_queues.as_ref, self.gap | Handled | [1564](../../crates/crysirror-circus/src/native_live.rs#L1564) |
| `condition` | `opcode == 0x4f` | self.sprites.iter_mut, self.unload_image, self.read32, self.write32 | Handled | [1582](../../crates/crysirror-circus/src/native_live.rs#L1582) |

### native_live.rs / NativeLiveState::voice_timing

```rust
fn voice_timing(&mut self, mode: i16, name: &str) -> Result<()>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `mode == 255 \|\| name.starts_with('-')` | self.write32 | Handled | [872](../../crates/crysirror-circus/src/native_live.rs#L872) |
| `condition` | `mode == 255` | self.write32 | Handled | [875](../../crates/crysirror-circus/src/native_live.rs#L875) |
| `condition` | `mode > 100` | i32::from | Handled | [891](../../crates/crysirror-circus/src/native_live.rs#L891) |

### settings.rs / prefix_index

```rust
fn prefix_index(selector: u8) -> Option<usize>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `selector` | `5` | Local state/value operations | Handled | [235](../../crates/crysirror-circus/src/settings.rs#L235) |
| `selector` | `7` | Local state/value operations | Handled | [236](../../crates/crysirror-circus/src/settings.rs#L236) |
| `selector` | `8` | Local state/value operations | Handled | [237](../../crates/crysirror-circus/src/settings.rs#L237) |
| `selector` | `9` | Local state/value operations | Handled | [238](../../crates/crysirror-circus/src/settings.rs#L238) |
| `selector` | `10` | Local state/value operations | Handled | [239](../../crates/crysirror-circus/src/settings.rs#L239) |
| `selector` | `12` | Local state/value operations | Handled | [240](../../crates/crysirror-circus/src/settings.rs#L240) |
| `selector` | `13` | Local state/value operations | Handled | [241](../../crates/crysirror-circus/src/settings.rs#L241) |
| `selector` | `14` | Local state/value operations | Handled | [242](../../crates/crysirror-circus/src/settings.rs#L242) |
| `selector` | `15` | Local state/value operations | Handled | [243](../../crates/crysirror-circus/src/settings.rs#L243) |
| `selector` | `16` | Local state/value operations | Handled | [244](../../crates/crysirror-circus/src/settings.rs#L244) |
| `selector` | `17` | Local state/value operations | Handled | [245](../../crates/crysirror-circus/src/settings.rs#L245) |
| `selector` | `18` | Local state/value operations | Handled | [246](../../crates/crysirror-circus/src/settings.rs#L246) |
| `selector` | `26` | Local state/value operations | Handled | [247](../../crates/crysirror-circus/src/settings.rs#L247) |
| `selector` | `27` | Local state/value operations | Handled | [248](../../crates/crysirror-circus/src/settings.rs#L248) |
| `selector` | `28` | Local state/value operations | Handled | [249](../../crates/crysirror-circus/src/settings.rs#L249) |
| `selector` | `29` | Local state/value operations | Handled | [250](../../crates/crysirror-circus/src/settings.rs#L250) |
| `selector` | `30` | Local state/value operations | Handled | [251](../../crates/crysirror-circus/src/settings.rs#L251) |
| `selector` | `180` | Local state/value operations | Handled | [252](../../crates/crysirror-circus/src/settings.rs#L252) |
| `selector` | `181` | Local state/value operations | Handled | [253](../../crates/crysirror-circus/src/settings.rs#L253) |
| `selector` | `182` | Local state/value operations | Handled | [254](../../crates/crysirror-circus/src/settings.rs#L254) |
| `selector` | `184` | Local state/value operations | Handled | [255](../../crates/crysirror-circus/src/settings.rs#L255) |
| `selector` | `185` | Local state/value operations | Handled | [256](../../crates/crysirror-circus/src/settings.rs#L256) |
| `selector` | `188` | Local state/value operations | Handled | [257](../../crates/crysirror-circus/src/settings.rs#L257) |
| `selector` | `190` | Local state/value operations | Handled | [258](../../crates/crysirror-circus/src/settings.rs#L258) |
| `selector` | `191` | Local state/value operations | Handled | [259](../../crates/crysirror-circus/src/settings.rs#L259) |
| `selector` | `194` | Local state/value operations | Handled | [260](../../crates/crysirror-circus/src/settings.rs#L260) |
| `selector` | `196` | Local state/value operations | Handled | [261](../../crates/crysirror-circus/src/settings.rs#L261) |
| `selector` | `197` | Local state/value operations | Handled | [262](../../crates/crysirror-circus/src/settings.rs#L262) |
| `selector` | `198` | Local state/value operations | Handled | [263](../../crates/crysirror-circus/src/settings.rs#L263) |
| `selector` | `199` | Local state/value operations | Handled | [264](../../crates/crysirror-circus/src/settings.rs#L264) |
| `selector` | `200` | Local state/value operations | Handled | [265](../../crates/crysirror-circus/src/settings.rs#L265) |
| `selector` | `201` | Local state/value operations | Handled | [266](../../crates/crysirror-circus/src/settings.rs#L266) |
| `selector` | `202` | Local state/value operations | Handled | [267](../../crates/crysirror-circus/src/settings.rs#L267) |
| `selector` | `203` | Local state/value operations | Handled | [268](../../crates/crysirror-circus/src/settings.rs#L268) |
| `selector` | `205` | Local state/value operations | Handled | [269](../../crates/crysirror-circus/src/settings.rs#L269) |
| `selector` | `206` | Local state/value operations | Handled | [270](../../crates/crysirror-circus/src/settings.rs#L270) |
| `selector` | `207` | Local state/value operations | Handled | [271](../../crates/crysirror-circus/src/settings.rs#L271) |
| `selector` | `208` | Local state/value operations | Handled | [272](../../crates/crysirror-circus/src/settings.rs#L272) |
| `selector` | `209` | Local state/value operations | Handled | [273](../../crates/crysirror-circus/src/settings.rs#L273) |
| `selector` | `210` | Local state/value operations | Handled | [274](../../crates/crysirror-circus/src/settings.rs#L274) |
| `selector` | `211` | Local state/value operations | Handled | [275](../../crates/crysirror-circus/src/settings.rs#L275) |
| `selector` | `212` | Local state/value operations | Handled | [276](../../crates/crysirror-circus/src/settings.rs#L276) |
| `selector` | `213` | Local state/value operations | Handled | [277](../../crates/crysirror-circus/src/settings.rs#L277) |
| `selector` | `214` | Local state/value operations | Handled | [278](../../crates/crysirror-circus/src/settings.rs#L278) |
| `selector` | `218` | Local state/value operations | Handled | [279](../../crates/crysirror-circus/src/settings.rs#L279) |
| `selector` | `221` | Local state/value operations | Handled | [280](../../crates/crysirror-circus/src/settings.rs#L280) |
| `selector` | `225` | Local state/value operations | Handled | [281](../../crates/crysirror-circus/src/settings.rs#L281) |
| `selector` | `227` | Local state/value operations | Handled | [282](../../crates/crysirror-circus/src/settings.rs#L282) |
| `selector` | `237` | Local state/value operations | Handled | [283](../../crates/crysirror-circus/src/settings.rs#L283) |
| `selector` | `_` | Local state/value operations | Return only: Boolean/optional result interpreted by the caller | [284](../../crates/crysirror-circus/src/settings.rs#L284) |

### settings.rs / SystemData::assign

```rust
fn assign(&mut self, selector: u8, value: i32) -> Result<bool>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 11` | self.set_dword | Handled | [212](../../crates/crysirror-circus/src/settings.rs#L212) |
| `condition` | `selector == 217` | self.dword, self.set_dword | Handled | [216](../../crates/crysirror-circus/src/settings.rs#L216) |

### settings.rs / SystemData::value

```rust
fn value(&self, selector: u8) -> Result<Option<i32>>
```

| Selector / caller branch | Pattern / condition | Implementation calls | State / restriction | Source |
| --- | --- | --- | --- | --- |
| `condition` | `selector == 11` | self.dword | Handled | [198](../../crates/crysirror-circus/src/settings.rs#L198) |
| `condition` | `selector == 217` | self.dword | Handled | [201](../../crates/crysirror-circus/src/settings.rs#L201) |
