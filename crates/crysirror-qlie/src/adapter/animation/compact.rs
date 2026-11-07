//! Packed V18 records, promoted to the native V25 integer schema. Offsets are
//! from TABMPLayer10's compact-record conversion, not inferred from a picture.
use super::*;
pub(super) fn compact_key(b: &[u8]) -> Result<[i32; 115]> {
    ensure!(b.len() == 162, "ABMP compact key width");
    let short = |at| i16::from_le_bytes([b[at], b[at + 1]]) as i32;
    let wide = |at| i32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    let native = |v: i32| match v {
        32700..=32715 => v - 32700 + 0x10000000,
        32720..=32735 => v - 32720 + 0x10000100,
        _ => v,
    };
    let image = |v: u8| {
        if (200..216).contains(&v) {
            i32::from(v) - 200 + 0x10000000
        } else {
            i32::from(v)
        }
    };
    let alpha = |v: i32| match v {
        256..=271 => v - 256 + 0x10000000,
        276..=291 => v - 276 + 0x10000100,
        _ => v,
    };
    let color = |v: i32| match v {
        0x1000000..=0x100000f => v - 0x1000000 + 0x10000000,
        16777236..=16777251 => v - 16777236 + 0x10000100,
        _ => v,
    };
    let mut v = [0; 115];
    v[0] = wide(0);
    for (dst, src) in [
        (1, 4),
        (2, 5),
        (3, 145),
        (4, 148),
        (5, 146),
        (6, 149),
        (7, 62),
        (8, 63),
        (11, 60),
        (12, 61),
        (18, 45),
        (19, 46),
        (30, 161),
        (31, 48),
        (32, 49),
        (43, 76),
        (44, 147),
        (45, 51),
        (46, 59),
        (51, 58),
        (52, 52),
        (15, 47),
        (16, 50),
    ] {
        v[dst] = i32::from(b[src]);
    }
    v[9] = image(b[6]);
    v[10] = image(b[7]);
    for (dst, src) in [
        (20, 8),
        (21, 10),
        (22, 12),
        (23, 14),
        (24, 64),
        (25, 66),
        (26, 68),
        (27, 70),
        (33, 24),
        (34, 26),
        (36, 28),
        (37, 30),
        (38, 150),
        (39, 72),
        (40, 74),
        (41, 34),
        (42, 32),
        (53, 137),
        (54, 139),
        (55, 141),
        (56, 143),
    ] {
        v[dst] = native(short(src));
    }
    for (dst, src) in [(13, 16), (14, 20), (35, 77)] {
        v[dst] = native(wide(src));
    }
    v[28] = short(133);
    v[29] = short(135);
    v[47] = color(wide(36));
    v[48] = color(wide(40));
    v[49] = alpha(short(53));
    v[50] = alpha(short(55));
    for i in 0..4 {
        let src = 81 + 12 * i;
        for n in 0..4 {
            v[57 + 8 * i + n] = i32::from(b[src + n]);
            v[61 + 8 * i + n] = native(short(src + 4 + 2 * n));
        }
        v[89 + i] = short(152 + 2 * i);
    }
    v[94] = 6;
    v[95] = 62;
    v[96] = 3;
    // 59D214 seeds the native destination before 5B3EC0 writes packed
    // fields. These later fields have no counterpart in the compact record.
    v[103] = 50;
    v[105] = 100;
    v[106] = 255;
    Ok(v)
}
