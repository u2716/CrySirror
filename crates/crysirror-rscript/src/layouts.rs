// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! Operand layouts of the RScript dispatchers, in the order each native
//! handler reads them. Opcodes absent from a table fall through to the native
//! default case, which yields one frame without reading operands.
use crate::gsc::Kind::{self, *};

const V: Kind = Value;
const V1: &[Kind] = &[V];
const V2: &[Kind] = &[V, V];
const V3: &[Kind] = &[V, V, V];
const V4: &[Kind] = &[V, V, V, V];
const V5: &[Kind] = &[V, V, V, V, V];
const NONE: &[Kind] = &[];

/// RScript 1.11.0.3 dispatcher (0x4358A0 in the Shapeshifter trial build;
/// every layout compared against its decompiled cases).
pub fn layout_1_11(op: u16) -> Option<&'static [Kind]> {
    Some(match op {
        0x03 | 0x04 | 0x05 => &[Target],
        0x08 | 0x0A | 0x0B | 0x11 | 0x1A | 0x1B | 0x32 | 0x33 | 0x34 | 0x37 | 0x39 | 0x3A | 0x44 | 0x45 | 0xDE => NONE,
        0x09 => &[U16],
        0x0C => &[V, Label],
        0x0D | 0x10 | 0x13 | 0x15 | 0x2C | 0x2E | 0x35 | 0x41 | 0x43 | 0x48 | 0x4A | 0x50 | 0x53 | 0x5E | 0x66 | 0x70 | 0xE6
        | 0xE7 => V1,
        0x0E => &[U16, Str, Target, Target, Target, Target, Target, Str, Str, Str, Str, Str, V, V, V],
        0x0F => &[V, Label, V, V, V, V, V, V, V, V, V, V],
        0x12 => &[V, Array],
        0x14 | 0x18 | 0x1D | 0x23 | 0x24 | 0x25 | 0x28 | 0x29 | 0x2A | 0x2B | 0x2D | 0x2F | 0x31 | 0x3D | 0x3E | 0x40 | 0x5C
        | 0x5D | 0x5F | 0x60 | 0x61 | 0x62 | 0x65 | 0x67 | 0x69 | 0x71..=0x75 | 0x78 | 0x84 | 0x8C | 0x8D | 0x8E | 0x98..=0x9B
        | 0x9E | 0x9F | 0xDD | 0xDF => V2,
        0x16 | 0x17 | 0x26 | 0x3B | 0x3F | 0x46 | 0x47 | 0x4D | 0x68 | 0x6C | 0x82 => V4,
        0x19 => &[Str, V],
        0x1C | 0x27 | 0x30 | 0x3C | 0x49 | 0x5A | 0x63 | 0x64 | 0x6B | 0x6E | 0x6F | 0x86 | 0x88 | 0x9C | 0xCA | 0xD5 | 0xDC => V3,
        0x1E => &[V, V, V, V, V, V],
        0x20 => &[V, V, V, V, V, Str],
        0x21 | 0x22 | 0x38 | 0x4B | 0x5B | 0x6A | 0x83 | 0x87 | 0x9D | 0xC9 | 0xE1 => V5,
        0x42 => &[U32, V, V, V],
        0x51 => &[V, U32, V, V, U32, U32, V],
        0x52 => &[V, V, V, V, U32, V],
        0x54 | 0x79 | 0x96 | 0x97 => &[V, Str],
        0xC8 => &[Target, V, V, V, V, V, V, V, V, V, V],
        0xD2 => &[Target, V],
        0xD3 => &[Target, V, V, V],
        0xD4 => &[Target],
        0xFF => V5,
        _ => return None,
    })
}

/// Every opcode of the 1.11 table, for coverage reports.
pub fn opcodes_1_11() -> impl Iterator<Item = u16> {
    (0u16..=0xFF).filter(|&op| layout_1_11(op).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn table_has_the_dispatcher_opcode_count() {
        // 138 opcodes with operand layouts in the 1.11.0.3 dispatcher.
        assert_eq!(opcodes_1_11().count(), 138);
        assert_eq!(layout_1_11(0x51).unwrap().len(), 7);
        assert_eq!(layout_1_11(0x0E).unwrap().len(), 15);
        assert!(layout_1_11(0x01).is_none());
    }
}
