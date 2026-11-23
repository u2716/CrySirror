use crate::native_asset::error::{AssetError, Result};

pub fn decrypt_dollar_payload(data: &mut [u8]) {
    let aligned_len = data.len() & !3;
    let mut rotate = 4u32;

    for chunk in data[..aligned_len].chunks_exact_mut(4) {
        chunk[0] = chunk[0].rotate_left(rotate & 7);
        rotate = rotate.wrapping_add(1);

        let mut value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        value ^= 0x084D_F873;
        value ^= 0xFF98_7DEE;
        chunk.copy_from_slice(&value.to_le_bytes());
    }
}

pub fn decrypt_pal_dollar_file(name: &str, data: &mut [u8]) -> Result<bool> {
    if data.first().copied() != Some(b'$') {
        return Ok(false);
    }
    if data.len() < 0x10 {
        return Err(AssetError::InvalidEncryptedFile {
            name: name.to_owned(),
            len: data.len(),
        });
    }
    decrypt_dollar_payload(&mut data[0x10..]);
    Ok(true)
}
