//! Original packed 16-byte bitmap descriptors, usable as ordinary image assets.
pub struct Format;
impl crate::ImageFormat for Format {
    fn id(&self) -> &'static str {
        "engine/packed-bitmap"
    }
    fn recognize(&self, _: &str, bytes: &[u8]) -> bool {
        matches!(
            crate::cbg::detect_image_format(bytes),
            crate::cbg::ImageFormat::RawBgiImage
        )
    }
    fn decode(
        &self,
        bytes: &[u8],
        context: &crate::ImageDecodeContext<'_>,
    ) -> anyhow::Result<crate::ImageSurface> {
        context.check_cancelled()?;
        let image = crate::cbg::decode_image(bytes)?;
        context.check_cancelled()?;
        image.validate()?;
        Ok(image)
    }
}
