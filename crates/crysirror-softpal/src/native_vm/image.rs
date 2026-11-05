#[derive(Clone, Debug)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub rgba: Vec<u8>,
}

pub fn decode_image(bytes: &[u8]) -> anyhow::Result<DecodedImage> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return png::decode(bytes);
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return jpeg::decode(bytes);
    }
    if bytes.starts_with(b"BM") {
        return bmp::decode(bytes);
    }
    if bytes.len() >= 0x20 && bytes.get(0..3) == Some(b"GE ") {
        return decode_pgd_ge(bytes);
    }
    if bytes.len() >= 0x38 && bytes.get(0..4) == Some(b"PGD3") {
        anyhow::bail!("PGD3 delta image requires a base-resource resolver");
    }
    if looks_like_tga(bytes) {
        return decode_tga(bytes);
    }
    anyhow::bail!("unsupported image signature");
}

mod bmp;
mod jpeg;
mod png;

/// Limit the new general-purpose image decoders before allocating pixel data.
/// Decoder work buffers have a separate limit; this bounds the final RGBA image.
const MAX_RASTER_BYTES: usize = 256 * 1024 * 1024;

fn bounded_rgba_len(width: u32, height: u32) -> anyhow::Result<usize> {
    let len = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|&len| len > 0 && len <= MAX_RASTER_BYTES as u64)
        .ok_or_else(|| anyhow::anyhow!("image dimensions exceed decoded size limit"))?;
    Ok(usize::try_from(len)?)
}

pub fn decode_image_with_resolver<F>(bytes: &[u8], resolver: &mut F) -> anyhow::Result<DecodedImage>
where
    F: FnMut(&str) -> anyhow::Result<Vec<u8>>,
{
    if bytes.len() >= 0x38 && bytes.get(0..4) == Some(b"PGD3") {
        return decode_pgd3_delta(bytes, resolver);
    }
    decode_image(bytes)
}

fn decode_pgd3_delta<F>(bytes: &[u8], resolver: &mut F) -> anyhow::Result<DecodedImage>
where
    F: FnMut(&str) -> anyhow::Result<Vec<u8>>,
{
    let offset_x = read_u16(bytes, 0x04)? as i16 as i32;
    let offset_y = read_u16(bytes, 0x06)? as i16 as i32;
    let width = read_u16(bytes, 0x08)? as usize;
    let height = read_u16(bytes, 0x0A)? as usize;
    let bpp = read_u16(bytes, 0x0C)?;
    let base_name = pgd3_base_name(bytes)?;
    let unpacked_size = read_u32(bytes, 0x30)? as usize;
    let packed_size = read_u32(bytes, 0x34)? as usize;
    let data_end = 0x38usize
        .checked_add(packed_size)
        .ok_or_else(|| anyhow::anyhow!("PGD3 packed stream size overflow"))?;
    if data_end > bytes.len() {
        anyhow::bail!(
            "PGD3 packed stream out of bounds: end {data_end}, len {}",
            bytes.len()
        );
    }

    let pixel_size = match bpp {
        24 => 3,
        32 => 4,
        _ => anyhow::bail!("unsupported PGD3 bpp {bpp}"),
    };
    let base_bytes = resolver(&base_name)?;
    let mut base = decode_image_with_resolver(&base_bytes, resolver)
        .map_err(|err| anyhow::anyhow!("PGD3 base {base_name:?} decode failed: {err}"))?;
    let mut stream = GeStream::new(Cursor::new(&bytes[..data_end], 0x38), unpacked_size);
    xor_pgd3_stream(&mut base, offset_x, offset_y, width, height, pixel_size, &mut stream)?;
    Ok(base)
}

fn pgd3_base_name(bytes: &[u8]) -> anyhow::Result<String> {
    let name_bytes = bytes
        .get(0x0E..0x30)
        .ok_or_else(|| anyhow::anyhow!("PGD3 base name out of bounds"))?;
    let end = name_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_bytes.len());
    if end == 0 {
        anyhow::bail!("PGD3 base name is empty");
    }
    Ok(String::from_utf8_lossy(&name_bytes[..end]).into_owned())
}

fn looks_like_tga(bytes: &[u8]) -> bool {
    if bytes.len() < 18 {
        return false;
    }
    let color_map_type = bytes[1];
    let image_type = bytes[2];
    let width = u16::from_le_bytes([bytes[12], bytes[13]]);
    let height = u16::from_le_bytes([bytes[14], bytes[15]]);
    let bpp = bytes[16];
    width != 0
        && height != 0
        && color_map_type <= 1
        && matches!(image_type, 2 | 3 | 10 | 11)
        && matches!(bpp, 8 | 24 | 32)
}

fn decode_tga(bytes: &[u8]) -> anyhow::Result<DecodedImage> {
    if bytes.len() < 18 {
        anyhow::bail!("TGA header truncated");
    }
    let id_len = bytes[0] as usize;
    let color_map_type = bytes[1];
    let image_type = bytes[2];
    let color_map_len = u16::from_le_bytes([bytes[5], bytes[6]]) as usize;
    let color_map_bpp = bytes[7] as usize;
    let width = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
    let height = u16::from_le_bytes([bytes[14], bytes[15]]) as usize;
    let bpp = bytes[16] as usize;
    let descriptor = bytes[17];
    if width == 0 || height == 0 {
        anyhow::bail!("TGA has empty dimensions");
    }
    if color_map_type != 0 {
        anyhow::bail!("TGA color maps are not supported");
    }
    let pixel_bytes = match bpp {
        8 => 1,
        24 => 3,
        32 => 4,
        _ => anyhow::bail!("unsupported TGA bpp {bpp}"),
    };
    let color_map_bytes = color_map_len
        .checked_mul(color_map_bpp.saturating_add(7) / 8)
        .ok_or_else(|| anyhow::anyhow!("TGA color map size overflow"))?;
    let mut pos = 18usize
        .checked_add(id_len)
        .and_then(|value| value.checked_add(color_map_bytes))
        .ok_or_else(|| anyhow::anyhow!("TGA data offset overflow"))?;
    if pos > bytes.len() {
        anyhow::bail!("TGA data offset outside file");
    }
    let pixel_count = width
        .checked_mul(height)
        .ok_or_else(|| anyhow::anyhow!("TGA dimensions overflow"))?;
    let mut rgba = vec![0u8; pixel_count * 4];
    let origin_top = (descriptor & 0x20) != 0;
    let origin_right = (descriptor & 0x10) != 0;
    let mut write_pixel = |index: usize, px: &[u8]| -> anyhow::Result<()> {
        let x = index % width;
        let y = index / width;
        let dst_x = if origin_right { width - 1 - x } else { x };
        let dst_y = if origin_top { y } else { height - 1 - y };
        let dst = (dst_y * width + dst_x) * 4;
        match pixel_bytes {
            1 => {
                let g = px[0];
                rgba[dst..dst + 4].copy_from_slice(&[g, g, g, 255]);
            }
            3 => {
                rgba[dst..dst + 4].copy_from_slice(&[px[2], px[1], px[0], 255]);
            }
            4 => {
                rgba[dst..dst + 4].copy_from_slice(&[px[2], px[1], px[0], px[3]]);
            }
            _ => unreachable!(),
        }
        Ok(())
    };

    match image_type {
        2 | 3 => {
            let need = pixel_count
                .checked_mul(pixel_bytes)
                .ok_or_else(|| anyhow::anyhow!("TGA pixel data size overflow"))?;
            if pos.checked_add(need).is_none_or(|end| end > bytes.len()) {
                anyhow::bail!("TGA pixel data truncated");
            }
            for index in 0..pixel_count {
                let start = pos + index * pixel_bytes;
                write_pixel(index, &bytes[start..start + pixel_bytes])?;
            }
        }
        10 | 11 => {
            let mut index = 0usize;
            while index < pixel_count {
                let Some(&header) = bytes.get(pos) else {
                    anyhow::bail!("TGA RLE packet header truncated");
                };
                pos += 1;
                let count = (usize::from(header & 0x7F)).saturating_add(1);
                if (header & 0x80) != 0 {
                    if pos
                        .checked_add(pixel_bytes)
                        .is_none_or(|end| end > bytes.len())
                    {
                        anyhow::bail!("TGA RLE pixel truncated");
                    }
                    let px = &bytes[pos..pos + pixel_bytes];
                    pos += pixel_bytes;
                    for _ in 0..count {
                        if index >= pixel_count {
                            anyhow::bail!("TGA RLE packet overruns image");
                        }
                        write_pixel(index, px)?;
                        index += 1;
                    }
                } else {
                    for _ in 0..count {
                        if index >= pixel_count {
                            anyhow::bail!("TGA raw packet overruns image");
                        }
                        if pos
                            .checked_add(pixel_bytes)
                            .is_none_or(|end| end > bytes.len())
                        {
                            anyhow::bail!("TGA raw pixel truncated");
                        }
                        write_pixel(index, &bytes[pos..pos + pixel_bytes])?;
                        pos += pixel_bytes;
                        index += 1;
                    }
                }
            }
        }
        _ => anyhow::bail!("unsupported TGA image type {image_type}"),
    }

    Ok(DecodedImage {
        width: width as u32,
        height: height as u32,
        cell_width: width as u32,
        cell_height: height as u32,
        offset_x: 0,
        offset_y: 0,
        rgba,
    })
}

/// The LZ token stores a 12-bit distance. A 4 KiB window replaces the full
/// unpacked patch, which can otherwise exceed 64 MiB beside its decoded base.
struct GeStream<'a> {
    input: Cursor<'a>,
    history: [u8; 4096],
    size: usize,
    produced: usize,
    control: u16,
    run: usize,
    distance: usize,
}

impl<'a> GeStream<'a> {
    fn new(input: Cursor<'a>, size: usize) -> Self {
        Self { input, history: [0; 4096], size, produced: 0,
            control: 2, run: 0, distance: 0 }
    }

    #[inline]
    fn read(&mut self) -> anyhow::Result<u8> {
        if self.produced >= self.size {
            anyhow::bail!("PGD unpacked stream exhausted");
        }
        while self.run == 0 {
            self.control >>= 1;
            if self.control == 1 {
                self.control = u16::from(self.input.read_u8()?) | 0x100;
            }
            if self.control & 1 != 0 {
                let raw = usize::from(self.input.read_u16()?);
                let mut count = raw & 7;
                if raw & 8 == 0 {
                    count = (count << 8) | usize::from(self.input.read_u8()?);
                }
                self.run = count + 4;
                self.distance = raw >> 4;
                if self.distance == 0 || self.distance > self.produced {
                    anyhow::bail!("invalid PGD back-reference distance");
                }
            } else {
                self.run = usize::from(self.input.read_u8()?);
                self.distance = 0;
            }
            if self.run > self.size - self.produced {
                anyhow::bail!("PGD token exceeds unpacked stream");
            }
        }
        let byte = if self.distance == 0 {
            self.input.read_u8()?
        } else {
            self.history[(self.produced - self.distance) & 4095]
        };
        self.history[self.produced & 4095] = byte;
        self.produced += 1;
        self.run -= 1;
        Ok(byte)
    }
}

fn xor_pgd3_stream(
    base: &mut DecodedImage,
    offset_x: i32,
    offset_y: i32,
    width: usize,
    height: usize,
    pixel_size: usize,
    stream: &mut GeStream<'_>,
) -> anyhow::Result<()> {
    if offset_x < 0 || offset_y < 0 || width == 0 || height == 0 {
        anyhow::bail!("PGD3 invalid patch geometry");
    }
    let (offset_x, offset_y) = (offset_x as usize, offset_y as usize);
    if offset_x.saturating_add(width) > base.width as usize
        || offset_y.saturating_add(height) > base.height as usize {
        anyhow::bail!("PGD3 patch exceeds base image");
    }
    visit_pal_rows(stream, width, height, pixel_size, |y, row| {
        let start = ((offset_y + y) * base.width as usize + offset_x) * 4;
        for (dst, patch) in base.rgba[start..start + width * 4].chunks_exact_mut(4)
            .zip(row.chunks_exact(pixel_size)) {
            dst[0] ^= patch[2];
            dst[1] ^= patch[1];
            dst[2] ^= patch[0];
            if pixel_size == 4 { dst[3] ^= patch[3]; }
        }
        Ok(())
    })
}

fn visit_pal_rows(
    stream: &mut GeStream<'_>, width: usize, height: usize, pixel_size: usize,
    mut visit: impl FnMut(usize, &[u8]) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let stride = width.checked_mul(pixel_size)
        .ok_or_else(|| anyhow::anyhow!("PGD row size overflow"))?;
    let required = stride.checked_add(1).and_then(|row| row.checked_mul(height))
        .ok_or_else(|| anyhow::anyhow!("PGD image size overflow"))?;
    if width == 0 || height == 0 || required > stream.size - stream.produced {
        anyhow::bail!("PGD row data out of bounds");
    }
    let mut controls = vec![0; height];
    for control in &mut controls { *control = stream.read()?; }
    let mut previous = vec![0u8; stride];
    let mut row = vec![0u8; stride];
    for (y, control) in controls.into_iter().enumerate() {
        if control & 1 == 0 && control & 2 != 0 && y == 0 {
            anyhow::bail!("PGD row references previous row before row 0");
        }
        for x in 0..stride {
            let delta = stream.read()?;
            row[x] = if control & 1 != 0 {
                if x < pixel_size { delta } else { row[x - pixel_size].wrapping_sub(delta) }
            } else if control & 2 != 0 {
                previous[x].wrapping_sub(delta)
            } else if y == 0 || x < pixel_size {
                delta
            } else {
                let value = ((u16::from(previous[x]) + u16::from(row[x - pixel_size])) / 2) as u8;
                value.wrapping_sub(delta)
            };
        }
        visit(y, &row)?;
        std::mem::swap(&mut row, &mut previous);
    }
    while stream.produced < stream.size { stream.read()?; }
    Ok(())
}

fn decode_pgd_ge(bytes: &[u8]) -> anyhow::Result<DecodedImage> {
    let offset_x = read_u32(bytes, 0x04)? as i32;
    let offset_y = read_u32(bytes, 0x08)? as i32;
    let mut width = read_u32(bytes, 0x0C)? as usize;
    let mut height = read_u32(bytes, 0x10)? as usize;
    let cell_width = read_u32(bytes, 0x14)?.max(1);
    let cell_height = read_u32(bytes, 0x18)?.max(1);
    let method = read_u16(bytes, 0x1C)?;
    let unpacked_size = read_u32(bytes, 0x20)? as usize;
    let rgba = if method == 3 {
        let mut stream = GeStream::new(Cursor::new(bytes, 0x28), unpacked_size);
        let mut header = [0u8; 8];
        for byte in &mut header { *byte = stream.read()?; }
        let bpp = u16::from_le_bytes([header[2], header[3]]);
        width = usize::from(u16::from_le_bytes([header[4], header[5]]));
        height = usize::from(u16::from_le_bytes([header[6], header[7]]));
        let pixel_size = match bpp {
            24 => 3,
            32 => 4,
            _ => anyhow::bail!("unsupported PGD/GE bpp {bpp}"),
        };
        let length = bounded_rgba_len(width as u32, height as u32)?;
        image_allocation_begin(length);
        let mut pixels = vec![0u8; length];
        visit_pal_rows(&mut stream, width, height, pixel_size, |y, row| {
            let start = y * width * 4;
            for (dst, src) in pixels[start..start + width * 4].chunks_exact_mut(4)
                .zip(row.chunks_exact(pixel_size)) {
                dst.copy_from_slice(&[src[2], src[1], src[0], if pixel_size == 4 { src[3] } else { 255 }]);
            }
            Ok(())
        })?;
        pixels
    } else {
        let mut input = Cursor::new(bytes, 0x28);
        let unpacked = unpack_ge_pre(&mut input, unpacked_size)?;
        match method {
            1 => post_process_planes_bgra_to_rgba(&unpacked)?,
            2 => post_process_yuv_like_to_rgba(&unpacked, width, height)?,
            _ => anyhow::bail!("unsupported PGD/GE method {method}"),
        }
    };
    let expected = width
        .checked_mul(height)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| anyhow::anyhow!("PGD/GE dimensions overflow"))?;
    if rgba.len() != expected {
        anyhow::bail!(
            "PGD/GE decoded size mismatch: got {}, expected {}",
            rgba.len(),
            expected
        );
    }
    Ok(DecodedImage {
        width: width as u32,
        height: height as u32,
        cell_width: cell_width.min(width as u32).max(1),
        cell_height: cell_height.min(height as u32).max(1),
        offset_x,
        offset_y,
        rgba,
    })
}

fn image_allocation_begin(bytes: usize) {
    #[cfg(target_os = "vita")]
    if bytes >= 8 * 1024 * 1024 {
        unsafe extern "C" { fn sena_vita_image_allocation(bytes: usize); }
        unsafe { sena_vita_image_allocation(bytes); }
    }
    #[cfg(not(target_os = "vita"))]
    let _ = bytes;
}

fn unpack_ge_pre(input: &mut Cursor<'_>, unpacked_size: usize) -> anyhow::Result<Vec<u8>> {
    image_allocation_begin(unpacked_size);
    let mut output = vec![0u8; unpacked_size];
    let mut dst = 0usize;
    let mut ctl = 2u16;
    while dst < output.len() {
        ctl >>= 1;
        if ctl == 1 {
            ctl = u16::from(input.read_u8()?) | 0x100;
        }
        let count = if (ctl & 1) != 0 {
            let raw = input.read_u16()? as usize;
            let mut count = raw & 7;
            if (raw & 8) == 0 {
                count = (count << 8) | usize::from(input.read_u8()?);
            }
            count += 4;
            let offset = raw >> 4;
            if offset == 0 || offset > dst {
                anyhow::bail!("invalid PGD/GE back-reference offset {offset} at {dst}");
            }
            copy_overlapped(&mut output, dst - offset, dst, count)?;
            count
        } else {
            let count = usize::from(input.read_u8()?);
            input.read_exact(&mut output[dst..dst + count])?;
            count
        };
        dst = dst.saturating_add(count);
    }
    Ok(output)
}

fn post_process_planes_bgra_to_rgba(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    if input.len() % 4 != 0 {
        anyhow::bail!("PGD/GE method 1 payload is not four planes");
    }
    let plane = input.len() / 4;
    let (a, rest) = input.split_at(plane);
    let (r, rest) = rest.split_at(plane);
    let (g, b) = rest.split_at(plane);
    let mut output = Vec::with_capacity(input.len());
    for i in 0..plane {
        output.extend_from_slice(&[r[i], g[i], b[i], a[i]]);
    }
    Ok(output)
}

fn post_process_yuv_like_to_rgba(
    input: &[u8],
    width: usize,
    height: usize,
) -> anyhow::Result<Vec<u8>> {
    if width % 2 != 0 || height % 2 != 0 {
        anyhow::bail!("PGD/GE method 2 dimensions must be even");
    }
    let segment = width
        .checked_mul(height)
        .ok_or_else(|| anyhow::anyhow!("PGD/GE dimensions overflow"))?
        / 4;
    if input.len() < segment * 3 {
        anyhow::bail!("PGD/GE method 2 payload too short");
    }
    let mut src0 = 0usize;
    let mut src1 = segment;
    let mut src2 = segment * 2;
    let stride = width * 4;
    let mut output = vec![0u8; stride * height];
    let points = [0usize, 1, width, width + 1];
    for y in 0..height / 2 {
        for x in 0..width / 2 {
            let i0 = input[src0] as i8 as i32;
            let i1 = input[src1] as i8 as i32;
            let b_delta = 226 * i0;
            let g_delta = -43 * i0 - 89 * i1;
            let r_delta = 179 * i1;
            src0 += 1;
            src1 += 1;
            for point in points {
                let base = i32::from(input[src2 + point]) << 7;
                let px = ((y * 2) * width + (x * 2) + point) * 4;
                output[px] = clamp_u8((base + r_delta) >> 7);
                output[px + 1] = clamp_u8((base + g_delta) >> 7);
                output[px + 2] = clamp_u8((base + b_delta) >> 7);
                output[px + 3] = 255;
            }
            src2 += 2;
        }
        src2 += width;
    }
    Ok(output)
}


fn copy_overlapped(output: &mut [u8], src: usize, dst: usize, count: usize) -> anyhow::Result<()> {
    let end = dst
        .checked_add(count)
        .ok_or_else(|| anyhow::anyhow!("PGD/GE overlap copy overflow"))?;
    if end > output.len() {
        anyhow::bail!("PGD/GE overlap copy out of bounds");
    }
    for i in 0..count {
        output[dst + i] = output[src + i];
    }
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> anyhow::Result<u16> {
    let raw = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| anyhow::anyhow!("u16 read out of bounds at 0x{offset:X}"))?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> anyhow::Result<u32> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow::anyhow!("u32 read out of bounds at 0x{offset:X}"))?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8], pos: usize) -> Self {
        Self { bytes, pos }
    }

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        let byte = *self
            .bytes
            .get(self.pos)
            .ok_or_else(|| anyhow::anyhow!("PGD/GE input out of bounds"))?;
        self.pos += 1;
        Ok(byte)
    }

    fn read_u16(&mut self) -> anyhow::Result<u16> {
        let value = read_u16(self.bytes, self.pos)?;
        self.pos += 2;
        Ok(value)
    }

    fn read_exact(&mut self, dst: &mut [u8]) -> anyhow::Result<()> {
        let end = self
            .pos
            .checked_add(dst.len())
            .ok_or_else(|| anyhow::anyhow!("PGD/GE input range overflow"))?;
        let src = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| anyhow::anyhow!("PGD/GE input range out of bounds"))?;
        dst.copy_from_slice(src);
        self.pos = end;
        Ok(())
    }
}
