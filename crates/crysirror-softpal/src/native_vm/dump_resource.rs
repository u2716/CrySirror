use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use crate::native_asset::{Nls, ResourceManager};

#[derive(Parser, Debug)]
struct Cli {
    #[arg(value_name = "GAME_ROOT")]
    root: PathBuf,

    #[arg(value_name = "RESOURCE")]
    resource: String,

    #[arg(long, value_enum, default_value_t = NlsArg::Sjis)]
    nls: NlsArg,

    #[arg(long, default_value_t = 256)]
    bytes: usize,

    #[arg(long, default_value_t = 0)]
    offset: usize,

    #[arg(long)]
    raw: bool,
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
    let mut asset = manager
        .open_decrypting(&cli.resource)
        .or_else(|_| manager.open_decrypting(&format!("{}.ANI", cli.resource)))
        .or_else(|_| manager.open_decrypting(&format!("{}.PGD", cli.resource)))
        .with_context(|| format!("failed to open resource {}", cli.resource))?;
    if cli.raw {
        asset = manager
            .open(&cli.resource)
            .or_else(|_| manager.open(&format!("{}.ANI", cli.resource)))
            .or_else(|_| manager.open(&format!("{}.PGD", cli.resource)))
            .with_context(|| format!("failed to open raw resource {}", cli.resource))?;
    }
    println!("resource: {}", asset.name);
    println!("size: 0x{:X} ({})", asset.bytes.len(), asset.bytes.len());
    let start = cli.offset.min(asset.bytes.len());
    let end = start.saturating_add(cli.bytes).min(asset.bytes.len());
    dump_hex(start, &asset.bytes[start..end]);
    Ok(())
}

fn dump_hex(base: usize, bytes: &[u8]) {
    for (row, chunk) in bytes.chunks(16).enumerate() {
        print!("{:08X}  ", base + row * 16);
        for i in 0..16 {
            if let Some(byte) = chunk.get(i) {
                print!("{byte:02X} ");
            } else {
                print!("   ");
            }
        }
        print!(" |");
        for byte in chunk {
            let ch = if byte.is_ascii_graphic() || *byte == b' ' {
                char::from(*byte)
            } else {
                '.'
            };
            print!("{ch}");
        }
        println!("|");
    }
}
