// Copyright (c) xmoezzz. MPL-2.0.
//! Bounded DSC Huffman/LZ byte decoding shared by resources and native VMs.
use anyhow::Result;

#[derive(Clone, Debug)]
struct DscNode {
    is_parent: bool,
    code: Option<u16>,
    left: usize,
    right: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct DscCode {
    depth: u8,
    code: u16,
}

/// Decode a DSC payload into its original bytes, including native image headers.
pub fn decode(data: &[u8]) -> Result<Vec<u8>> {
    decode_with_limit(data, 64 * 1024 * 1024)
}

pub fn decode_with_limit(data: &[u8], limit: usize) -> Result<Vec<u8>> {
    anyhow::ensure!(
        data.len() <= limit && data.len() >= 0x220 && data.starts_with(b"DSC FORMAT 1.00"),
        "DSC header/packed byte limit"
    );
    let mut key = u32::from_le_bytes([data[0x10], data[0x11], data[0x12], data[0x13]]);
    let magic = (u16::from_le_bytes([data[0], data[1]]) as u32) << 16;
    let output_size = u32::from_le_bytes(data[0x14..0x18].try_into().unwrap()) as usize;
    anyhow::ensure!(
        output_size <= limit.min(512 * 1024 * 1024),
        "DSC decoded byte limit"
    );
    let dec_count = u32::from_le_bytes([data[0x18], data[0x19], data[0x1a], data[0x1b]]) as usize;
    if output_size == 0 {
        return Ok(Vec::new());
    }
    let mut codes = Vec::new();
    for i in 0..512usize {
        let depth = data[0x20 + i].wrapping_sub(update_key(&mut key, magic));
        if depth > 0 {
            codes.push(DscCode {
                depth,
                code: i as u16,
            });
        }
    }
    codes.sort();
    let nodes = create_dsc_tree(&codes)?;
    let mut reader = BitReader::new(&data[0x220..]);
    let mut output = Vec::new();
    output.try_reserve_exact(output_size)?;
    output.resize(output_size, 0);
    let mut dst = 0usize;

    for _ in 0..dec_count {
        let mut node_index = 0usize;
        loop {
            let bit = reader.next_bit()?;
            let node = nodes
                .get(node_index)
                .ok_or_else(|| anyhow::Error::msg("DSC huffman node out of range"))?;
            node_index = if bit { node.right } else { node.left };
            let node = nodes
                .get(node_index)
                .ok_or_else(|| anyhow::Error::msg("DSC huffman child out of range"))?;
            if !node.is_parent {
                let code = node
                    .code
                    .ok_or_else(|| anyhow::Error::msg("DSC missing leaf code"))?;
                if (code >> 8) == 1 {
                    let offset = reader.next_bits(12)? as usize + 2;
                    let count = (code as usize & 0xff) + 2;
                    if offset > dst {
                        return Err(anyhow::Error::msg("DSC back-reference before output start"));
                    }
                    for _ in 0..count {
                        if dst >= output.len() {
                            break;
                        }
                        let value = output[dst - offset];
                        output[dst] = value;
                        dst += 1;
                    }
                } else if dst < output.len() {
                    output[dst] = code as u8;
                    dst += 1;
                }
                break;
            }
        }
        if dst >= output.len() {
            break;
        }
    }
    Ok(output)
}

fn create_dsc_tree(codes: &[DscCode]) -> Result<Vec<DscNode>> {
    if codes.is_empty() {
        return Err(anyhow::Error::msg("DSC huffman tree is empty"));
    }
    let mut nodes = vec![
        DscNode {
            is_parent: false,
            code: None,
            left: 0,
            right: 0,
        };
        1024
    ];
    let mut left_index = vec![0usize; 512];
    let mut right_index = vec![0usize; 512];
    let mut next_node = 1usize;
    let mut depth_nodes = 1usize;
    let mut depth = 0u16;
    let mut left_child = true;
    let mut n = 0usize;
    while n < codes.len() {
        if depth > u8::MAX as u16 || depth_nodes > 512 || next_node > 1024 {
            return Err(anyhow::Error::msg("DSC huffman tree exceeds node budget"));
        }
        let target_left = left_child;
        left_child = !left_child;
        let mut depth_existing = 0usize;
        while n < codes.len() && u16::from(codes[n].depth) == depth {
            if depth_existing >= depth_nodes {
                return Err(anyhow::Error::msg("DSC huffman tree is oversubscribed"));
            }
            let index = if target_left {
                left_index[depth_existing]
            } else {
                right_index[depth_existing]
            };
            nodes[index].code = Some(codes[n].code);
            n += 1;
            depth_existing += 1;
        }
        let to_create = depth_nodes.saturating_sub(depth_existing);
        if to_create > 256 || next_node + to_create * 2 > nodes.len() {
            return Err(anyhow::Error::msg("DSC huffman tree exceeds node budget"));
        }
        for i in 0..to_create {
            let index = if target_left {
                left_index[depth_existing + i]
            } else {
                right_index[depth_existing + i]
            };
            nodes[index].is_parent = true;
            if left_child {
                left_index[i * 2] = next_node;
                nodes[index].left = next_node;
                next_node += 1;
                left_index[i * 2 + 1] = next_node;
                nodes[index].right = next_node;
                next_node += 1;
            } else {
                right_index[i * 2] = next_node;
                nodes[index].left = next_node;
                next_node += 1;
                right_index[i * 2 + 1] = next_node;
                nodes[index].right = next_node;
                next_node += 1;
            }
        }
        depth += 1;
        depth_nodes = to_create * 2;
    }
    Ok(nodes)
}

fn update_key(key: &mut u32, magic: u32) -> u8 {
    let v0 = 20021u32.wrapping_mul(*key & 0xffff);
    let mut v1 = magic | (*key >> 16);
    v1 = v1
        .wrapping_mul(20021)
        .wrapping_add((*key).wrapping_mul(346));
    v1 = v1.wrapping_add(v0 >> 16) & 0xffff;
    *key = (v1 << 16).wrapping_add(v0 & 0xffff).wrapping_add(1);
    v1 as u8
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bits: u32,
    nbits: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            bits: 0,
            nbits: 0,
        }
    }

    fn next_bit(&mut self) -> Result<bool> {
        if self.nbits == 0 {
            let byte = self
                .data
                .get(self.pos)
                .ok_or_else(|| anyhow::Error::msg("DSC bitstream exhausted"))?;
            self.bits = *byte as u32;
            self.pos += 1;
            self.nbits = 8;
        }
        let bit = (self.bits & 0x80) != 0;
        self.bits = (self.bits << 1) & 0xff;
        self.nbits -= 1;
        Ok(bit)
    }

    fn next_bits(&mut self, count: u8) -> Result<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | u32::from(self.next_bit()?);
        }
        Ok(value)
    }
}
