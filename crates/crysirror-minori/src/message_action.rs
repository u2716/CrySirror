// SPDX-License-Identifier: MPL-2.0
use crate::vm::decimal_prefix;
use anyhow::{Result, bail, ensure};

pub(super) enum Operation {
    Load {
        image: String,
        duration: i32,
        blend: i32,
    },
    Position([i32; 2]),
    Visible(bool),
    Blend {
        duration: i32,
        blend: i32,
    },
}

pub(super) struct MessageAction {
    pub due: u64,
    pub id: i32,
    pub generation: Option<u32>,
    pub started: bool,
    pub operation: Operation,
}

impl MessageAction {
    pub fn parse(command: &str, now: u64) -> Result<Self> {
        let fields: Vec<_> = command.split(',').collect();
        let number = |i: usize| fields.get(i).and_then(|s| decimal_prefix(s)).unwrap_or(0);
        ensure!(
            fields.len() >= 4,
            "minori inline command needs four operands: {command}"
        );
        let operation = match fields[0] {
            "load" => Operation::Load {
                image: fields[3].to_owned(),
                duration: number(4),
                blend: if fields.len() >= 6 { number(5) } else { 255 },
            },
            "pos" if fields.len() >= 5 => Operation::Position([number(3), number(4)]),
            "vis" => Operation::Visible(matches!(
                fields[3].as_bytes().first(),
                Some(b'1'..=b'9' | b't' | b'T')
            )),
            "trans" if fields.len() >= 5 => Operation::Blend {
                duration: number(3),
                blend: number(4),
            },
            _ => bail!("unsupported minori inline command {command}"),
        };
        Ok(Self {
            due: (now / 10).wrapping_add_signed(i64::from(number(1))),
            id: number(2),
            generation: None,
            started: false,
            operation,
        })
    }
}
