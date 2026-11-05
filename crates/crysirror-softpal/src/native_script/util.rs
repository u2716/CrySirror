use crate::native_script::error::{PalError, Result};

pub fn read_u32_le(buf: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).ok_or(PalError::ArithmeticOverflow)?;
    if end > buf.len() {
        return Err(PalError::ReadOutOfBounds {
            offset,
            len: buf.len(),
            need: 4,
        });
    }
    let bytes = [
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ];
    Ok(u32::from_le_bytes(bytes))
}

pub fn parse_u32_number(value: &str) -> Result<u32> {
    let value = value.trim();
    if value.is_empty() {
        return Err(PalError::InvalidNumber {
            value: value.to_string(),
        });
    }

    let parsed = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u32::from_str_radix(hex, 16)
    } else {
        value.parse::<u32>()
    };

    parsed.map_err(|_| PalError::InvalidNumber {
        value: value.to_string(),
    })
}

pub fn parse_usize_number(value: &str) -> Result<usize> {
    let parsed = parse_u32_number(value)?;
    Ok(parsed as usize)
}
