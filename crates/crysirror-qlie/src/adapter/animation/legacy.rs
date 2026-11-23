//! Native ABMP V15/V16/V17 record promotion. The original readers first expand
//! 92 -> 145 -> 162 bytes (5B3A28/5B3C64), then apply the same integer-key
//! upgrades as newer layers. Keep these resource-version rules out of drawing.
use super::*;

fn put_short(bytes: &mut [u8], at: usize, value: i32) {
    bytes[at..at + 2].copy_from_slice(&(value as i16).to_le_bytes());
}

pub(super) fn upgrade(tag: &str, raw: &[u8]) -> Result<[i32; 115]> {
    let width = match tag {
        "ablayer15" => 92,
        "ablayer16" => 145,
        "ablayer17" => 162,
        _ => bail!("unsupported legacy ABMP record {tag}"),
    };
    ensure!(raw.len() == width, "ABMP {tag} key width");
    let mut packed = [0_u8; 162];
    packed[..width].copy_from_slice(raw);
    if tag == "ablayer15" {
        // 5B3A28: replace the old 3x3 anchor selectors with percentage pairs,
        // widen four colour components, and initialise generator strengths.
        for (selector, x, y) in [(47, 129, 131), (50, 133, 135)] {
            let anchor = i32::from(packed[selector]);
            packed[selector] = 0;
            put_short(&mut packed, x, 50 * (anchor % 3));
            put_short(&mut packed, y, 50 * (anchor / 3));
        }
        for (source, target) in [(53, 137), (54, 139), (55, 141), (56, 143)] {
            let value = i32::from(packed[source]);
            put_short(&mut packed, target, value);
        }
        for offset in [84, 96, 108, 120] {
            packed[offset] = 100;
        }
    }
    // 5B3C64: these fields already exist in V17. Only the 92/145-byte
    // layouts introduce their defaults while expanding to 162 bytes.
    if tag != "ablayer17" {
        put_short(&mut packed, 150, 100);
        for offset in [152, 154, 156, 158] {
            put_short(&mut packed, offset, -1);
        }
    }

    // 5B4288/5B426C: legacy expression sentinels and byte alpha fields.
    let alpha = i32::from(packed[44]);
    let secondary_alpha = i32::from(packed[57]);
    put_short(&mut packed, 53, alpha);
    put_short(&mut packed, 55, secondary_alpha);
    let expression = |value: i32| {
        if (30000..30100).contains(&value) {
            value - 30000 + 32700
        } else {
            value
        }
    };
    for offset in [8, 10, 12, 14, 24, 26, 28, 30, 150, 72, 74, 34] {
        let value = i16::from_le_bytes(packed[offset..offset + 2].try_into()?) as i32;
        put_short(&mut packed, offset, expression(value));
    }
    for offset in [16, 20, 77] {
        let value = i32::from_le_bytes(packed[offset..offset + 4].try_into()?);
        packed[offset..offset + 4].copy_from_slice(&expression(value).to_le_bytes());
    }
    if packed[4] == 18 {
        packed[7] = 255;
        packed[62] = 0;
        packed[63] = 100;
    }
    let mut key = super::compact::compact_key(&packed)?;
    // 59D214 seeds a full key before 5B3EC0 writes the packed fields.
    key[103] = 50;
    key[105] = 100;
    key[106] = 255;
    // 5B441C: nested motion selectors moved out of the image selector field.
    if matches!(key[1] as u8, 3 | 4 | 7 | 13 | 18 | 19) {
        key[97] = key[10];
        key[10] = 0;
    }
    let flags = key[94];
    key[94] = if flags & 1 != 0 { 3 } else { 0 }
        | if flags & 2 != 0 { 4 } else { 0 }
        | if flags & 4 != 0 { 8 } else { 0 };
    Ok(key)
}

pub(super) fn finish(tag: &str, channel: u8, frames: &mut [[i32; 115]]) {
    // 5B39B0 applies this V15 compatibility default to the whole layer, not
    // individual zero-scale keys; zero can otherwise mean an intentional hide.
    if tag == "ablayer15" && channel == 1 && frames.iter().all(|key| key[36] == 0 && key[37] == 0) {
        for key in frames.iter_mut() {
            key[36] = 100;
            key[37] = 100;
        }
    }
    // V24 already stores the new fields and its reader does not call 5B4518.
    if !matches!(
        tag,
        "ablayer15"
            | "ablayer16"
            | "ablayer17"
            | "ablayer18"
            | "ablayer20"
            | "ablayer21"
            | "ablayer22"
            | "ablayer23"
    ) {
        return;
    }
    // 5B4518 migrates the generator and every following key in that sequence.
    let mut generator = false;
    for key in frames {
        let kind = key[1] as u8;
        if matches!(kind, 20 | 21) || generator {
            key[107] = key[20];
            key[108] = key[21];
            key[109] = key[22];
            key[110] = key[51];
            key[111] = key[24];
            key[112] = key[25];
            key[113] = key[26];
            key[114] = key[27];
            key[104] = key[18];
            key[105] = key[19];
            key[106] = 0;
            if key[94] & 4 != 0 {
                key[106] = 0x0f;
            }
            if key[94] & 8 != 0 {
                key[106] = 0xf0;
            }
            key[18..=27].fill(0);
            key[51] = 0;
            generator |= matches!(kind, 20 | 21);
        } else {
            key[104] = 0;
            key[105] = 100;
            key[106] = 255;
        }
    }
}
