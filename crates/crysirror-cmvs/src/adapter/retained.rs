// SPDX-License-Identifier: MPL-2.0
//! Command aliases between native target families and consumed input edges.

/// How the first operand of an aliased command selects its target.
#[derive(Clone, Copy)]
pub(super) enum AliasTarget {
    Window,
    /// Text fields follow the 12 text objects.
    Field,
    /// List objects follow the 6 button groups.
    List,
}

/// Commands sharing the operands of an implemented command on another target family.
pub(super) fn aliased_command(opcode: u16) -> Option<(u16, AliasTarget)> {
    let field = |command| Some((command, AliasTarget::Field));
    let list = |command| Some((command, AliasTarget::List));
    match opcode {
        0x2378 => list(0x2218),
        0x2379 => list(0x2219),
        0x237a => list(0x221a),
        0x2370 => list(0x2210),
        0x2371 => list(0x2211),
        0x2372 => list(0x2212),
        0x2373 => list(0x2213),
        0x2374 => list(0x2214),
        0x2375 => list(0x2215),
        0x2376 => list(0x2216),
        0x2377 => list(0x2217),
        0x237b => list(0x221b),
        0x2380 | 0x2381 => field(0x2100),
        0x2382 => field(0x2101),
        0x2383 => field(0x2102),
        0x2385 => field(0x2103),
        0x2386 => field(0x2104),
        0x2387 => field(0x2105),
        0x2388 => field(0x2106),
        0x2389 => field(0x2107),
        0x238a => field(0x211a),
        0x238b => field(0x210b),
        0x238c => field(0x210c),
        0x238e => field(0x210f),
        0x238f => field(0x2110),
        0x2391 => field(0x2119),
        0x2392 => field(0x2113),
        0x2393 => field(0x210e),
        0x2394 => field(0x2118),
        0x23b9 => field(0x211d),
        _ => window_child_command(opcode).map(|command| (command, AliasTarget::Window)),
    }
}

/// Message window child commands and the object command with the same operands.
pub(super) fn window_child_command(opcode: u16) -> Option<u16> {
    Some(match opcode {
        0x2176 => 0x2038,
        0x2177 => 0x2039,
        0x217e => 0x202c,
        0x2181 => 0x2051,
        0x2187 => 0x204b,
        0x2189 => 0x2043,
        0x218b => 0x2049,
        0x21c0..=0x21c3 | 0x21c5..=0x21c8 | 0x21ca..=0x21cd => opcode + 0x20,
        0x21d4..=0x21d7 => opcode + 0x20,
        _ => return None,
    })
}

/// The native clear of an input action's press and release edges.
pub(super) fn cleared_action(opcode: u16) -> Option<usize> {
    Some(match opcode {
        0x219d => 15,
        0x219f => 16,
        0x21a1 => 0,
        0x21a3 => 1,
        0x21a9 => 6,
        0x21ab => 9,
        0x21ad => 10,
        0x21af => 11,
        0x21b1 => 12,
        0x21b3 => 13,
        0x21b5 => 14,
        0x21b7 => 2,
        0x21b9 => 3,
        0x21bb => 4,
        0x21bd => 5,
        0x22f1 => 17,
        0x22f3 => 18,
        0x22f5 => 19,
        0x2345 => 22,
        0x2347 => 23,
        0x2349 => 21,
        0x234b => 20,
        _ => return None,
    })
}
