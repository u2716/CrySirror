// SPDX-License-Identifier: MPL-2.0
//! KISS `.tex` textures of Custom Maid 3D 2 (`texture.arc`).
//!
//! Layout from `ImportCM.LoadTexture` (Assembly-CSharp, CM3D2 1.00): a .NET
//! `BinaryReader` string `CM3D2_TEX`, i32 version (1000), the source asset
//! path string, i32 payload length and the payload. The game hands the payload
//! to `Texture2D.LoadImage`, so it is a complete PNG or JPEG file; every
//! texture of the 1.00 data set is PNG. Rows stay top-down as encoded: Unity's
//! bottom-up texture origin is a sampling convention for the model importer.
//! COM3D2 v1010 adds image dimensions and a Unity texture format before the payload length.
use anyhow::{bail, ensure, Context, Result};
use crate::{decode_standard, ImageDecodeContext, ImageSurface};

pub const ID: &str = "kiss/cm3d2-tex";
const SIGNATURE: &[u8] = b"\x09CM3D2_TEX";

pub struct Format;
impl crate::ImageFormat for Format {
    fn id(&self) -> &'static str {
        ID
    }
    fn recognize(&self, _: &str, bytes: &[u8]) -> bool {
        bytes.starts_with(SIGNATURE)
    }
    fn decode(&self, bytes: &[u8], ctx: &ImageDecodeContext<'_>) -> Result<ImageSurface> {
        ctx.check_cancelled()?;
        let image = payload(bytes)?;
        ensure!(
            image.starts_with(b"\x89PNG") || image.starts_with(b"\xff\xd8\xff"),
            "CM3D2 texture payload is neither PNG nor JPEG"
        );
        decode_standard(image)
    }
}

/// The embedded encoded image of a `.tex` container.
pub fn payload(bytes: &[u8]) -> Result<&[u8]> {
    ensure!(bytes.starts_with(SIGNATURE), "not a CM3D2 texture");
    let mut at = SIGNATURE.len();
    let version = i32::from_le_bytes(bytes.get(at..at + 4).context("truncated CM3D2 texture")?.try_into()?);
    ensure!(version >= 1000, "unsupported CM3D2 texture version {version}");
    at += 4;
    // BinaryReader.ReadString: 7-bit variable byte count, then UTF-8.
    let mut length = 0usize;
    let mut shift = 0;
    loop {
        let byte = *bytes.get(at).context("truncated CM3D2 texture path")?;
        at += 1;
        length |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift > 28 {
            bail!("CM3D2 texture path length prefix is too long");
        }
    }
    at = at.checked_add(length).filter(|end| *end + 4 <= bytes.len()).context("truncated CM3D2 texture path")?;
    // COM3D2 ImportCM.LoadTexture adds width, height and Unity TextureFormat.
    // ARGB32 containers still carry a complete encoded image for LoadImage.
    if version >= 1010 {
        let header = bytes.get(at..at + 12).context("truncated COM3D2 texture descriptor")?;
        let width = i32::from_le_bytes(header[..4].try_into()?);
        let height = i32::from_le_bytes(header[4..8].try_into()?);
        let format = i32::from_le_bytes(header[8..].try_into()?);
        ensure!(width > 0 && height > 0, "invalid COM3D2 texture dimensions");
        ensure!(format == 5, "unsupported COM3D2 raw texture format {format}");
        at += 12;
    }
    let size = i32::from_le_bytes(bytes.get(at..at + 4).context("truncated texture length")?.try_into()?);
    ensure!(size >= 0, "negative CM3D2 texture payload size");
    at += 4;
    bytes.get(at..at + size as usize).context("CM3D2 texture payload outside the file")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(image: &[u8]) -> Vec<u8> {
        let mut out = SIGNATURE.to_vec();
        out.extend(1000i32.to_le_bytes());
        let path = b"assets/texture/a.png";
        out.push(path.len() as u8);
        out.extend(path);
        out.extend((image.len() as i32).to_le_bytes());
        out.extend(image);
        out
    }

    #[test]
    fn version_1010_reads_the_descriptor_before_the_encoded_image() {
        let image = b"fixture";
        let mut bytes = container(image);
        let version = SIGNATURE.len();
        bytes[version..version + 4].copy_from_slice(&1010i32.to_le_bytes());
        let descriptor = bytes.len() - image.len() - 4;
        bytes.splice(descriptor..descriptor, [16i32, 32, 5].map(i32::to_le_bytes).concat());
        assert_eq!(payload(&bytes).unwrap(), image);
        bytes[descriptor + 8..descriptor + 12].copy_from_slice(&10i32.to_le_bytes());
        assert!(payload(&bytes).is_err());
        assert!(payload(&bytes[..descriptor + 2]).is_err());
    }

    #[test]
    fn payload_follows_the_binary_reader_header() {
        let mut png = Vec::new();
        image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 128])
            .unwrap()
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let tex = container(&png);
        assert!(crate::ImageFormat::recognize(&Format, "a.tex", &tex));
        assert_eq!(payload(&tex).unwrap(), png.as_slice());
        let decoded = decode_standard(payload(&tex).unwrap()).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 1));
        assert_eq!(decoded.rgba, [255, 0, 0, 255, 0, 0, 255, 128]);
        let mut short = tex.clone();
        short.truncate(tex.len() - 1);
        assert!(payload(&short).is_err());
    }
}
