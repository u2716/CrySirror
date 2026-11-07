//! Bounded word-oriented run-length streams with a64-entry literal-block
//! dictionary. The byte tail lives in the64-byte stream header.
use anyhow::{ensure, Context, Result};
fn take<'a>(source: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    ensure!(source.len() >= len, "truncated word run-length stream");
    let (head, tail) = source.split_at(len);
    *source = tail;
    Ok(head)
}
/// Decode the `runlengthpack` profile, including native clock-evicted literal
/// dictionary references. Malformed references and size overruns fail.
pub fn decode(mut source: &[u8], limit: usize) -> Result<Vec<u8>> {
    let header = take(&mut source, 64)?;
    ensure!(
        header[..16].split(|b| *b == 0).next() == Some(b"runlengthpack"),
        "word run-length signature"
    );
    let word = |i: usize| u32::from_le_bytes(header[i..i + 4].try_into().unwrap()) as usize;
    ensure!(
        word(16) == 1 && word(32) == 64,
        "word run-length header version/layout"
    );
    let main = word(20);
    let tail = word(24);
    ensure!(
        main % 4 == 0 && tail < 4 && main.checked_add(tail).is_some_and(|n| n <= limit),
        "word run-length output budget/alignment"
    );
    let mut out = Vec::with_capacity(main + tail);
    let mut dictionary: [Option<&[u8]>; 64] = [None; 64];
    let mut referenced = [false; 64];
    let mut cursor = 0;
    while out.len() < main {
        let code = take(&mut source, 1)?[0] as i8;
        if code > 0 {
            let count = code as usize * 4;
            ensure!(
                out.len() + count <= main,
                "word run-length repetition overrun"
            );
            let bytes = take(&mut source, 4)?;
            for _ in 0..code {
                out.extend_from_slice(bytes);
            }
        } else if code > -64 {
            ensure!(code != 0, "zero word run-length instruction");
            let count = (-i32::from(code)) as usize * 4;
            ensure!(out.len() + count <= main, "word run-length literal overrun");
            let bytes = take(&mut source, count)?;
            //4E7DD4 walks a clock, clearing each recently referenced entry
            //once before overwriting the first cold entry.
            while referenced[cursor] {
                referenced[cursor] = false;
                cursor = (cursor + 1) & 63;
            }
            dictionary[cursor] = Some(bytes);
            referenced[cursor] = false;
            cursor = (cursor + 1) & 63;
            out.extend_from_slice(bytes);
        } else {
            let index = (-i32::from(code) - 64) as usize;
            ensure!(index < 64, "word run-length dictionary index");
            let bytes = dictionary[index].context("undefined word run-length dictionary entry")?;
            ensure!(
                out.len() + bytes.len() <= main,
                "word run-length reference overrun"
            );
            out.extend_from_slice(bytes);
            referenced[index] = true;
        }
    }
    ensure!(
        source.is_empty(),
        "word run-length trailing instruction bytes"
    );
    out.extend_from_slice(&header[28..28 + tail]);
    Ok(out)
}
/// Encode a lossless literal-only stream. Run selection is an optimization;
/// literal blocks retain exactly the same compatible stream grammar.
pub fn encode_literals(source: &[u8], limit: usize) -> Result<Vec<u8>> {
    ensure!(source.len() <= limit, "word run-length input budget");
    let main = source.len() / 4 * 4;
    let tail = source.len() - main;
    let mut out = vec![0; 64];
    out[..13].copy_from_slice(b"runlengthpack");
    out[16..20].copy_from_slice(&1u32.to_le_bytes());
    out[20..24].copy_from_slice(&u32::try_from(main)?.to_le_bytes());
    out[24..28].copy_from_slice(&(tail as u32).to_le_bytes());
    out[28..28 + tail].copy_from_slice(&source[main..]);
    out[32..36].copy_from_slice(&64u32.to_le_bytes());
    for block in source[..main].chunks(63 * 4) {
        out.push((-(block.len() as i32 / 4)) as i8 as u8);
        out.extend_from_slice(block);
    }
    Ok(out)
}
