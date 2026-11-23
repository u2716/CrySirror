// Copyright (C)2026 CrySirror contributors. MPL-2.0.
//! Bounded immutable byte-cell tables and explicit loose line-quoted CSV.
//! This is a data operation, with no VM, text encoding, filesystem or UI policy.
use anyhow::{Context, Result, ensure};
use std::sync::Arc;
/// Storage/work limits checked before a table mutation is installed.
#[derive(Clone, Copy, Debug)]
pub struct ByteTableLimits {
    pub rows: usize,
    pub columns: usize,
    pub cells: usize,
    pub bytes: usize,
}
impl Default for ByteTableLimits {
    fn default() -> Self {
        Self {
            rows: 65536,
            columns: 65536,
            cells: 1_000_000,
            bytes: 64 * 1024 * 1024,
        }
    }
}
impl ByteTableLimits {
    fn validate(self) -> Result<()> {
        ensure!(
            self.rows <= 65536
                && self.columns <= 65536
                && self.cells <= 1_000_000
                && self.bytes <= 64 * 1024 * 1024,
            "byte table limits"
        );
        Ok(())
    }
}
/// Whether an overlong physical input line rejects the input or terminates
/// successfully before that line, retaining only complete preceding records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongLinePolicy {
    Reject,
    Stop,
}
/// Explicit line-oriented quoted-byte grammar. Quotes toggle anywhere in a
/// field, doubled quotes decode to one quote, and unmatched quotes end at the
/// physical line boundary. This deliberately does not claim RFC4180 multiline.
#[derive(Clone, Copy, Debug)]
pub struct ByteCsvReadOptions {
    pub separator: u8,
    pub quote: u8,
    pub line_limit: usize,
    /// Translate CRLF and recognize SUB as EOF, like a Windows text stream.
    pub windows_text: bool,
    /// Ignore content after the first NUL in each already-read physical line.
    pub c_prefix: bool,
    pub long_line: LongLinePolicy,
}
/// Writer options independent of the input grammar. Raw output permits the
/// intentionally unescaped legacy table writer; quote_some is conventional
/// quoted output with double-quote escaping. Text translation affects all LF,
/// including LF stored inside cell data. Records always end in LF first.
#[derive(Clone, Copy, Debug)]
pub struct ByteCsvWriteOptions {
    pub separator: u8,
    pub quote_some: Option<u8>,
    pub windows_text: bool,
    pub c_prefix: bool,
}
/// A raw-byte table; cloning shares every cell payload. Bulk metadata copies
/// and parse/resize/write operations should be scheduled on engine workers.
#[derive(Clone, Debug)]
pub struct ByteTable {
    rows: Arc<Vec<Vec<Arc<[u8]>>>>,
    limits: ByteTableLimits,
    columns: usize,
    cells: usize,
    bytes: usize,
}
/// The complete read result, including a deliberate line-limit termination.
pub struct ByteTableRead {
    pub table: ByteTable,
    pub stopped_at_long_line: bool,
}
impl ByteTable {
    pub fn new(limits: ByteTableLimits) -> Result<Self> {
        limits.validate()?;
        Ok(Self {
            rows: Arc::new(vec![]),
            limits,
            columns: 0,
            cells: 0,
            bytes: 0,
        })
    }
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
    /// Largest row, preserving ragged native input rather than padding it.
    pub fn column_count(&self) -> usize {
        self.columns
    }
    pub fn byte_len(&self) -> usize {
        self.bytes
    }
    pub fn get(&self, row: usize, column: usize) -> Option<&Arc<[u8]>> {
        self.rows.get(row)?.get(column)
    }
    /// An absent cell is a normalfalse result; no implicit table growth.
    pub fn set(&mut self, row: usize, column: usize, bytes: Arc<[u8]>) -> Result<bool> {
        let Some(old) = self.rows.get(row).and_then(|r| r.get(column)) else {
            return Ok(false);
        };
        let total = self
            .bytes
            .checked_sub(old.len())
            .and_then(|v| v.checked_add(bytes.len()))
            .context("byte table payload overflow")?;
        ensure!(total <= self.limits.bytes, "byte table payload limit");
        Arc::make_mut(&mut self.rows)[row][column] = bytes;
        self.bytes = total;
        Ok(true)
    }
    fn push(&mut self, row: Vec<Arc<[u8]>>) -> Result<()> {
        ensure!(
            self.rows.len() < self.limits.rows && row.len() <= self.limits.columns,
            "byte table row/column limit"
        );
        let cells = self
            .cells
            .checked_add(row.len())
            .context("byte table cell overflow")?;
        let bytes = row.iter().try_fold(self.bytes, |a, b| {
            a.checked_add(b.len()).context("byte table byte overflow")
        })?;
        ensure!(
            cells <= self.limits.cells && bytes <= self.limits.bytes,
            "byte table cell/payload limit"
        );
        self.columns = self.columns.max(row.len());
        self.cells = cells;
        self.bytes = bytes;
        Arc::make_mut(&mut self.rows).push(row);
        Ok(())
    }
    /// Construct all resized rows before replacing the original table. Old
    /// payloads remain shared; padding creates empty cells, not guessed values.
    pub fn resize(
        &mut self,
        rows: usize,
        columns: usize,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        ensure!(
            rows <= self.limits.rows && columns <= self.limits.columns,
            "byte table resize dimensions"
        );
        ensure!(
            rows.checked_mul(columns)
                .is_some_and(|n| n <= self.limits.cells),
            "byte table resize cell limit"
        );
        let mut next = Self::new(self.limits)?;
        let empty: Arc<[u8]> = Arc::from([]);
        for index in 0..rows {
            check()?;
            let mut row = self
                .rows
                .get(index)
                .map(|r| r[..r.len().min(columns)].to_vec())
                .unwrap_or_default();
            row.resize(columns, empty.clone());
            next.push(row)?;
        }
        check()?;
        *self = next;
        Ok(())
    }
    pub fn decode_lines(
        bytes: &[u8],
        options: ByteCsvReadOptions,
        limits: ByteTableLimits,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<ByteTableRead> {
        ensure!(
            bytes.len() <= 64 * 1024 * 1024
                && options.line_limit > 0
                && options.line_limit <= 1024 * 1024,
            "byte CSV input/line limit"
        );
        ensure!(
            options.separator != options.quote
                && !matches!(options.separator, 0 | 10 | 13)
                && !matches!(options.quote, 0 | 10 | 13),
            "byte CSV grammar"
        );
        let mut table = Self::new(limits)?;
        let mut at = 0;
        let mut line = vec![];
        let mut next_check = 0;
        while at < bytes.len() {
            if at >= next_check {
                check()?;
                next_check = at + 4096;
            }
            let mut value = bytes[at];
            at += 1;
            if options.windows_text && value == 26 {
                break;
            }
            if options.windows_text && value == 13 && bytes.get(at) == Some(&10) {
                at += 1;
                value = 10;
            }
            if value == 10 {
                table.push(parse_row(
                    &line,
                    options,
                    limits.columns.min(limits.cells - table.cells),
                    limits.bytes - table.bytes,
                    check,
                )?)?;
                line.clear();
                continue;
            }
            if line.len() == options.line_limit {
                return match options.long_line {
                    LongLinePolicy::Reject => anyhow::bail!("byte CSV physical line exceeds limit"),
                    LongLinePolicy::Stop => {
                        check()?;
                        Ok(ByteTableRead {
                            table,
                            stopped_at_long_line: true,
                        })
                    }
                };
            }
            line.push(value);
        }
        if !line.is_empty() {
            table.push(parse_row(
                &line,
                options,
                limits.columns.min(limits.cells - table.cells),
                limits.bytes - table.bytes,
                check,
            )?)?;
        }
        check()?;
        Ok(ByteTableRead {
            table,
            stopped_at_long_line: false,
        })
    }
    pub fn encode_lines(
        &self,
        options: ByteCsvWriteOptions,
        check: &dyn Fn() -> Result<()>,
    ) -> Result<Vec<u8>> {
        ensure!(
            !matches!(options.separator, 0 | 10 | 13)
                && options
                    .quote_some
                    .is_none_or(|q| q != options.separator && !matches!(q, 0 | 10 | 13)),
            "byte CSV writer grammar"
        );
        let mut output = vec![];
        let mut push = |byte: u8| -> Result<()> {
            let count = if byte == 10 && options.windows_text {
                2
            } else {
                1
            };
            ensure!(
                output
                    .len()
                    .checked_add(count)
                    .is_some_and(|n| n <= self.limits.bytes),
                "byte CSV output limit"
            );
            if count == 2 {
                output.push(13);
            }
            output.push(byte);
            Ok(())
        };
        for row in self.rows.iter() {
            check()?;
            // Raw native writers dereference the last field. Do not disguise
            // an undefined zero-column native layout as a successful record.
            ensure!(
                !row.is_empty(),
                "byte CSV writer requires a field in every row"
            );
            for (index, bytes) in row.iter().enumerate() {
                check()?;
                if index > 0 {
                    push(options.separator)?;
                }
                let bytes = if options.c_prefix {
                    bytes.split(|b| *b == 0).next().unwrap()
                } else {
                    bytes.as_ref()
                };
                let quoted = options.quote_some.is_some_and(|q| {
                    bytes
                        .iter()
                        .any(|b| *b == q || *b == options.separator || matches!(*b, 10 | 13))
                });
                if quoted {
                    push(options.quote_some.unwrap())?;
                }
                for chunk in bytes.chunks(4096) {
                    check()?;
                    for &byte in chunk {
                        push(byte)?;
                        if quoted && Some(byte) == options.quote_some {
                            push(byte)?;
                        }
                    }
                }
                if quoted {
                    push(options.quote_some.unwrap())?;
                }
            }
            push(10)?;
        }
        check()?;
        Ok(output)
    }
}
fn parse_row(
    line: &[u8],
    options: ByteCsvReadOptions,
    columns: usize,
    bytes: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<Arc<[u8]>>> {
    let line = if options.c_prefix {
        line.split(|b| *b == 0).next().unwrap()
    } else {
        line
    };
    let mut fields = vec![];
    let mut start = 0;
    let mut used = 0usize;
    loop {
        check()?;
        ensure!(fields.len() < columns, "byte CSV field count limit");
        let mut end = start;
        let mut quoted = false;
        while end < line.len() {
            if (end - start) % 4096 == 0 {
                check()?;
            }
            let b = line[end];
            if !quoted && b == options.separator {
                break;
            }
            if b == options.quote {
                quoted = !quoted;
            }
            end += 1;
        }
        let mut field = Vec::new();
        let mut at = start;
        while at < end {
            if (at - start) % 4096 == 0 {
                check()?;
            }
            let value = line[at];
            at += 1;
            if value == options.quote {
                if at < end && line[at] == options.quote {
                    ensure!(
                        used.checked_add(field.len()).is_some_and(|n| n < bytes),
                        "byte CSV payload limit"
                    );
                    field.push(value);
                    at += 1;
                }
            } else {
                ensure!(
                    used.checked_add(field.len()).is_some_and(|n| n < bytes),
                    "byte CSV payload limit"
                );
                field.push(value);
            }
        }
        used += field.len();
        fields.push(field.into());
        if end == line.len() {
            break;
        }
        start = end + 1;
    }
    Ok(fields)
}
