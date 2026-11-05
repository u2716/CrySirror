use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use crate::native_asset::{Nls, ResourceManager};
use crate::native_vm::GraphicIndex;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(value_name = "GAME_ROOT")]
    root: PathBuf,

    #[arg(value_name = "KEY")]
    key: String,

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
    let asset = manager.open_decrypting("graphic.dat")?;
    let index = match GraphicIndex::parse(&asset.bytes) {
        Ok(index) => index,
        Err(err) => {
            eprintln!("parse failed: {err}");
            eprintln!("graphic.dat len=0x{:X}", asset.bytes.len());
            dump_buckets(&asset.bytes);
            anyhow::bail!("graphic.dat parse failed");
        }
    };
    let needle = cli.key.to_ascii_uppercase();

    for entry in index.entries_by_key.values() {
        let key = key_string(&entry.key);
        if !key.contains(&needle) {
            continue;
        }
        let start = index.data_offset + entry.data_offset as usize;
        let end = start + entry.data_size as usize;
        println!(
            "key={key} bucket={} record_data_offset=0x{:X} start=0x{start:X} size=0x{:X}",
            entry.bucket, entry.data_offset, entry.data_size
        );
        if end > asset.bytes.len() {
            println!("  payload out of range");
            continue;
        }
        dump_words(&asset.bytes[start..end.min(start + 0x180)]);
    }
    Ok(())
}

fn dump_buckets(bytes: &[u8]) {
    for bucket in 0..255usize {
        let off = 0x10 + bucket * 8;
        if off + 8 > bytes.len() {
            break;
        }
        let b0 = u16::from_le_bytes([bytes[off], bytes[off + 1]]);
        let b2 = u16::from_le_bytes([bytes[off + 2], bytes[off + 3]]);
        let b4 = u32::from_le_bytes([
            bytes[off + 4],
            bytes[off + 5],
            bytes[off + 6],
            bytes[off + 7],
        ]);
        let d0 = u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]);
        if d0 != 0 || b4 != 0 {
            println!("bucket[{bucket:02X}] raw_d0=0x{d0:08X} w0={b0} w2={b2} d4=0x{b4:X}");
        }
    }
}

fn key_string(key: &[u8; 32]) -> String {
    let end = key.iter().position(|&b| b == 0).unwrap_or(key.len());
    String::from_utf8_lossy(&key[..end]).to_string()
}

fn dump_words(bytes: &[u8]) {
    for (row, chunk) in bytes.chunks(16).enumerate() {
        print!("  {:04X}:", row * 16);
        for byte in chunk {
            print!(" {byte:02X}");
        }
        for _ in chunk.len()..16 {
            print!("   ");
        }
        print!("  |");
        for &byte in chunk {
            let ch = if byte.is_ascii_graphic() || byte == b' ' {
                byte as char
            } else {
                '.'
            };
            print!("{ch}");
        }
        println!("|");
    }
}
