use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, PalError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PalError {
    ScriptTooSmall {
        len: usize,
    },
    BadMagic {
        got: [u8; 4],
    },
    ReadOutOfBounds {
        offset: usize,
        len: usize,
        need: usize,
    },
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    InvalidNumber {
        value: String,
    },
    PointTableMisaligned {
        len: usize,
    },
    PointIdOutOfRange {
        point_id: u32,
        entries: usize,
    },
    ArithmeticOverflow,
}

impl fmt::Display for PalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScriptTooSmall { len } => {
                write!(
                    f,
                    "Script.src is too small: len=0x{len:08X}, need at least 0x0000000C"
                )
            }
            Self::BadMagic { got } => {
                write!(f, "bad Script.src magic {:?}, expected b\"Sv20\"", got)
            }
            Self::ReadOutOfBounds { offset, len, need } => {
                write!(
                    f,
                    "read out of bounds at 0x{offset:08X}: need {need} byte(s), len=0x{len:08X}"
                )
            }
            Self::InvalidRange { start, end, len } => {
                write!(f, "invalid disassembly range: start=0x{start:08X}, end=0x{end:08X}, len=0x{len:08X}")
            }
            Self::InvalidNumber { value } => {
                write!(f, "invalid number: {value}")
            }
            Self::PointTableMisaligned { len } => {
                write!(
                    f,
                    "Point.dat length must be a multiple of 4, got len=0x{len:08X}"
                )
            }
            Self::PointIdOutOfRange { point_id, entries } => {
                write!(
                    f,
                    "point id {point_id} is out of range for Point.dat entries={entries}"
                )
            }
            Self::ArithmeticOverflow => write!(f, "arithmetic overflow"),
        }
    }
}

impl Error for PalError {}
