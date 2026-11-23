use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use crate::native_asset::{Nls, ResourceManager};
use crate::native_vm::image::decode_image_with_resolver;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(value_name = "GAME_ROOT")]
    root: PathBuf,

    #[arg(value_name = "RESOURCE")]
    resource: String,

    #[arg(long, value_enum, default_value_t = NlsArg::Sjis)]
    nls: NlsArg,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum NlsArg {
    Sjis,
    Gbk,
    Utf8,
}

impl From<NlsArg> for Nls {
    fn from(value: NlsArg) -> Self {
        match value {
            NlsArg::Sjis => Nls::ShiftJis,
            NlsArg::Gbk => Nls::Gbk,
            NlsArg::Utf8 => Nls::Utf8,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut manager = ResourceManager::bootstrap(&cli.root, cli.nls.into())
        .with_context(|| format!("failed to bootstrap {}", cli.root.display()))?;
    let asset = manager
        .open(&cli.resource)
        .or_else(|_| manager.open(&format!("{}.PGD", cli.resource)))
        .with_context(|| format!("failed to open resource {}", cli.resource))?;
    let mut resolver = |name: &str| -> anyhow::Result<Vec<u8>> { Ok(manager.open(name)?.bytes) };
    let image = decode_image_with_resolver(&asset.bytes, &mut resolver)
        .with_context(|| format!("failed to decode resource {}", asset.name))?;

    println!("resource: {}", asset.name);
    println!("file_size: 0x{:X}", asset.bytes.len());
    dump_header(&asset.bytes);
    println!("decoded_width: {}", image.width);
    println!("decoded_height: {}", image.height);
    println!("cell_width: {}", image.cell_width);
    println!("cell_height: {}", image.cell_height);
    println!("cell_columns: {}", image.width / image.cell_width.max(1));
    println!("cell_rows: {}", image.height / image.cell_height.max(1));
    println!("offset_x: {}", image.offset_x);
    println!("offset_y: {}", image.offset_y);
    println!(
        "decoded_canvas_rect: left=0 top=0 right={} bottom={}",
        image.width, image.height
    );
    println!(
        "default_source_rect: left=0 top=0 right={} bottom={}",
        image.cell_width, image.cell_height
    );
    println!(
        "frame_table_candidate: columns={} rows={} frames={} note=implicit_grid_from_cell_size",
        image.width / image.cell_width.max(1),
        image.height / image.cell_height.max(1),
        (image.width / image.cell_width.max(1)) * (image.height / image.cell_height.max(1))
    );
    if let Some(bounds) = alpha_bounds(&image.rgba, image.width, image.height) {
        let visible_width = bounds.right.saturating_sub(bounds.left);
        let visible_height = bounds.bottom.saturating_sub(bounds.top);
        println!(
            "alpha_bounds: left={} top={} right={} bottom={} width={} height={}",
            bounds.left, bounds.top, bounds.right, bounds.bottom, visible_width, visible_height
        );
        println!(
            "visible_bbox: left={} top={} right={} bottom={} width={} height={} source=alpha",
            bounds.left, bounds.top, bounds.right, bounds.bottom, visible_width, visible_height
        );
        println!(
            "alpha_bottom_margin: {}",
            image.height.saturating_sub(bounds.bottom)
        );
        println!("alpha_top_margin: {}", bounds.top);
        println!(
            "bottom_offset_from_cell: {}",
            image.cell_height.saturating_sub(bounds.bottom)
        );
        println!(
            "center_anchor_candidate: x={} y={}",
            bounds.left + visible_width / 2,
            bounds.bottom
        );
    } else {
        println!("alpha_bounds: none");
        println!("visible_bbox: none");
    }
    println!(
        "placement_metadata_explains_position: false note=PGD carries decode/cell/offset fields only; Game graphic.dat wrapper metadata and PalSpriteGetInfo must explain script placement"
    );
    Ok(())
}

fn dump_header(bytes: &[u8]) {
    let sig = bytes.get(0..4).unwrap_or(bytes);
    println!("signature: {}", String::from_utf8_lossy(sig));
    if bytes.len() >= 0x38 && bytes.get(0..4) == Some(b"PGD3") {
        println!("format: PGD3");
        println!("header.offset_x_i16: {}", read_i16(bytes, 0x04));
        println!("header.offset_y_i16: {}", read_i16(bytes, 0x06));
        println!("header.width_u16: {}", read_u16(bytes, 0x08));
        println!("header.height_u16: {}", read_u16(bytes, 0x0A));
        println!("header.bpp_u16: {}", read_u16(bytes, 0x0C));
        println!("header.unpacked_size_u32: {}", read_u32(bytes, 0x30));
        println!("header.packed_size_u32: {}", read_u32(bytes, 0x34));
    } else if bytes.len() >= 0x28 && bytes.get(0..3) == Some(b"GE ") {
        println!("format: GE");
        println!("header.offset_x_i32: {}", read_i32(bytes, 0x04));
        println!("header.offset_y_i32: {}", read_i32(bytes, 0x08));
        println!("header.width_u32: {}", read_u32(bytes, 0x0C));
        println!("header.height_u32: {}", read_u32(bytes, 0x10));
        println!("header.cell_width_u32: {}", read_u32(bytes, 0x14));
        println!("header.cell_height_u32: {}", read_u32(bytes, 0x18));
        println!("header.method_u16: {}", read_u16(bytes, 0x1C));
        println!("header.unpacked_size_u32: {}", read_u32(bytes, 0x20));
    } else {
        println!("format: unknown");
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

fn alpha_bounds(rgba: &[u8], width: u32, height: u32) -> Option<Bounds> {
    let mut left = width;
    let mut top = height;
    let mut right = 0u32;
    let mut bottom = 0u32;
    for y in 0..height {
        for x in 0..width {
            let idx = ((y as usize) * (width as usize) + (x as usize)) * 4 + 3;
            if rgba.get(idx).copied().unwrap_or(0) != 0 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x.saturating_add(1));
                bottom = bottom.max(y.saturating_add(1));
            }
        }
    }
    (right > left && bottom > top).then_some(Bounds {
        left,
        top,
        right,
        bottom,
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    bytes
        .get(offset..offset + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .unwrap_or(0)
}

fn read_i16(bytes: &[u8], offset: usize) -> i16 {
    read_u16(bytes, offset) as i16
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    bytes
        .get(offset..offset + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .unwrap_or(0)
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    read_u32(bytes, offset) as i32
}
