use crate::native_asset::error::{AssetError, Result};
use crate::native_asset::nls::Nls;

pub type PacKey = [u8; 32];

pub fn normalize_pal_name_bytes(bytes: &mut [u8]) {
    for b in bytes.iter_mut() {
        if *b == b'/' {
            *b = b'\\';
        }
    }

    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if (b & 0x80) != 0 {
            i += 2;
            continue;
        }
        if b.is_ascii_lowercase() {
            bytes[i] = b.to_ascii_uppercase();
        }
        i += 1;
    }
}

pub fn make_pac_key(name: &str, nls: Nls) -> Result<PacKey> {
    if name.is_empty() {
        return Err(AssetError::EmptyAssetName);
    }

    let mut encoded = nls.encode(name)?;
    normalize_pal_name_bytes(&mut encoded);

    if encoded.is_empty() {
        return Err(AssetError::EmptyAssetName);
    }
    if encoded.len() > 32 {
        return Err(AssetError::NameTooLong {
            encoded_len: encoded.len(),
            max_len: 32,
            name: name.to_owned(),
        });
    }

    let mut key = [0u8; 32];
    key[..encoded.len()].copy_from_slice(&encoded);
    Ok(key)
}

pub(crate) fn first_bucket_byte(key: &PacKey) -> Result<u8> {
    let first = key[0];
    if first == 0xFF {
        return Err(AssetError::PacUnsupportedBucket { first_byte: first });
    }
    Ok(first)
}

pub fn key_display_lossy(key: &PacKey, nls: Nls) -> String {
    let end = key.iter().position(|&b| b == 0).unwrap_or(key.len());
    match nls.decode(&key[..end]) {
        Ok(s) => s,
        Err(_) => key[..end]
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" "),
    }
}
