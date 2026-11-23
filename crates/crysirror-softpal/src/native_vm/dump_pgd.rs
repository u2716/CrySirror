use std::fs::File;
use std::io::BufWriter;
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

    #[arg(value_name = "OUTPUT")]
    output: PathBuf,

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
    if let Some(parent) = cli.output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let file = File::create(&cli.output)
        .with_context(|| format!("failed to create {}", cli.output.display()))?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.rgba)?;
    eprintln!(
        "wrote {} from {} size={}x{} offset=({}, {}) cell={}x{}",
        cli.output.display(),
        asset.name,
        image.width,
        image.height,
        image.offset_x,
        image.offset_y,
        image.cell_width,
        image.cell_height
    );
    Ok(())
}
