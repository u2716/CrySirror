// Copyright (C)2019 Nunuhara Cabbage; Rust (C)2026 CrySirror contributors.
// GPL-2.0-or-later. xsystem4 a85467d savedata.c/resume.c legacy JSON profiles.
//! Native schema conversion; generic byte-string JSON parsing is shared.
use super::{
    codec::{Array, Named, Record, Save, Value},
    resume::{Body, CallFrame, HeapValue, Resume, ReturnRecord, Symbol},
};
use crate::{
    ain::{Program, Variable},
    vm::heap::{element_kind, is_array, is_reference},
};
use anyhow::{ensure, Context, Result};
use crysirror_resource::byte_json::{
    parse_byte_json_profile, ByteJson, ByteJsonLimits, ByteJsonProfile,
};
pub(super) fn parse(bytes: &[u8], check: &dyn Fn() -> Result<()>) -> Result<ByteJson> {
    parse_byte_json_profile(
        bytes,
        ByteJsonLimits::default(),
        ByteJsonProfile::LegacyCJson,
        check,
    )
}
fn field<'a>(json: &'a ByteJson, key: &[u8]) -> Result<&'a ByteJson> {
    optional_field(json, key).context("System4 legacy JSON field missing")
}
fn optional_field<'a>(json: &'a ByteJson, key: &[u8]) -> Option<&'a ByteJson> {
    let ByteJson::Object(fields) = json else {
        return None;
    };
    fields
        .iter()
        .find(|(name, _)| {
            name.split(|b| *b == 0)
                .next()
                .unwrap()
                .eq_ignore_ascii_case(key)
        })
        .map(|(_, value)| value)
}
pub(super) fn check_key(json: &ByteJson, key: &[u8]) -> Result<()> {
    ensure!(
        cstring(field(json, b"key")?)? == key,
        "System4 legacy JSON key mismatch"
    );
    Ok(())
}
fn number(json: &ByteJson, key: &[u8]) -> Result<i32> {
    field(json, key)?.integer32()
}
fn cstring(json: &ByteJson) -> Result<Vec<u8>> {
    Ok(json
        .string_bytes()?
        .split(|b| *b == 0)
        .next()
        .unwrap()
        .to_vec())
}
pub(super) fn comments(json: &ByteJson, key: &[u8]) -> Result<Vec<Vec<u8>>> {
    ensure!(
        cstring(field(json, b"key")?)? == key,
        "System4 legacy JSON key mismatch"
    );
    let Some(values) = optional_field(json, b"comments") else {
        return Ok(Vec::new());
    };
    values.array()?.iter().map(cstring).collect()
}
pub(super) fn globals(
    program: &Program,
    json: &ByteJson,
    key: &[u8],
    group: &[u8],
    check: &dyn Fn() -> Result<()>,
) -> Result<Save> {
    ensure!(
        cstring(field(json, b"key")?)? == key,
        "System4 legacy JSON global key mismatch"
    );
    let mut b = GlobalBuilder {
        program,
        save: Save::new(
            key.to_vec(),
            group.to_vec(),
            i32::try_from(program.globals.len())?,
        ),
        nodes: 0,
        check,
    };
    b.save.original_global_indices = Some(Vec::new());
    b.save.records.push(Record {
        kind: 1000,
        name: Vec::new(),
        indices: Vec::new(),
    });
    for record in field(json, b"globals")?.array()? {
        check()?;
        let index = usize::try_from(number(record, b"index")?)?;
        let var = program
            .globals
            .get(index)
            .context("System4 legacy JSON global index")?;
        let value = b.value(var, field(record, b"value")?, 0)?;
        b.save.original_global_indices.as_mut().unwrap().push(index);
        b.save.globals.push(Named {
            name: var.name.clone(),
            value,
            unknown: 1,
        });
    }
    b.save.records[0].indices = (0..b.save.globals.len())
        .map(i32::try_from)
        .collect::<std::result::Result<_, _>>()?;
    b.save.validate()?;
    Ok(b.save)
}
struct GlobalBuilder<'a> {
    program: &'a Program,
    save: Save,
    nodes: usize,
    check: &'a dyn Fn() -> Result<()>,
}
impl GlobalBuilder<'_> {
    fn value(&mut self, var: &Variable, json: &ByteJson, depth: usize) -> Result<Value> {
        (self.check)()?;
        self.nodes += 1;
        ensure!(
            self.nodes <= 1_000_000 && depth <= 64,
            "System4 legacy JSON value graph limit"
        );
        let bits = match var.kind {
            10 | 47 | 55 => json.integer32()?,
            11 => {
                let ByteJson::Number(n) = json else {
                    anyhow::bail!("System4 legacy JSON float value")
                };
                (*n as f32).to_bits() as i32
            }
            12 => {
                let id = i32::try_from(self.save.strings.len())?;
                self.save.strings.push(cstring(json)?);
                id
            }
            13 => {
                let definition = self
                    .program
                    .structures
                    .get(usize::try_from(var.structure)?)
                    .context("System4 legacy JSON structure identity")?
                    .clone();
                let values = json.array()?;
                ensure!(
                    values.len() == definition.members.len(),
                    "System4 legacy JSON structure field extent"
                );
                let mut indices = Vec::with_capacity(values.len());
                for (var, json) in definition.members.iter().zip(values) {
                    let value = self.value(var, json, depth + 1)?;
                    indices.push(i32::try_from(self.save.fields.len())?);
                    self.save.fields.push(Named {
                        name: var.name.clone(),
                        value,
                        unknown: 1,
                    });
                }
                let id = i32::try_from(self.save.records.len())?;
                self.save.records.push(Record {
                    kind: 13,
                    name: definition.name,
                    indices,
                });
                id
            }
            kind if is_array(kind) => {
                let mut array = Array {
                    dimensions: None,
                    flat: Vec::new(),
                };
                if !matches!(json, ByteJson::Null) {
                    ensure!(
                        (1..=32).contains(&var.rank),
                        "System4 legacy JSON array rank"
                    );
                    let mut dimensions = Vec::with_capacity(var.rank as usize);
                    let mut next = Some(json);
                    for _ in 0..var.rank {
                        let values = next.map(ByteJson::array).transpose()?.unwrap_or(&[]);
                        dimensions.push(values.len());
                        next = values.first();
                    }
                    dimensions.reverse();
                    array.dimensions = Some(dimensions);
                    self.array(var, json, var.rank as usize, &mut array, depth + 1)?;
                }
                let id = i32::try_from(self.save.arrays.len())?;
                self.save.arrays.push(array);
                id
            }
            kind if is_reference(kind) => -1,
            _ => anyhow::bail!("unsupported System4 legacy JSON global type {}", var.kind),
        };
        Ok(Value {
            kind: var.kind,
            bits,
        })
    }
    fn array(
        &mut self,
        var: &Variable,
        json: &ByteJson,
        rank: usize,
        array: &mut Array,
        depth: usize,
    ) -> Result<()> {
        (self.check)()?;
        ensure!(depth <= 64 && rank > 0, "System4 legacy JSON array nesting");
        let values = json.array()?;
        ensure!(
            array.dimensions.as_ref().unwrap()[rank - 1] == values.len(),
            "System4 legacy JSON ragged array"
        );
        if rank > 1 {
            for value in values {
                self.array(var, value, rank - 1, array, depth + 1)?;
            }
        } else {
            let mut element = var.clone();
            element.kind = element_kind(var.kind)?;
            element.rank = 0;
            let mut flat = Vec::with_capacity(values.len());
            for value in values {
                flat.push(self.value(&element, value, depth + 1)?);
            }
            array.flat.push(flat);
        }
        Ok(())
    }
}
/// Translate original sparse JSON heap/call-stack fields to the same validated
/// RSM reconstruction, without changing raw strings into guessed CP932 text.
pub(super) fn resume(
    program: &Program,
    json: &ByteJson,
    key: &[u8],
    heap_extent: usize,
    check: &dyn Fn() -> Result<()>,
) -> Result<Resume> {
    ensure!(
        cstring(field(json, b"key")?)? == key,
        "System4 legacy JSON resume key mismatch"
    );
    let version = if program.delegates.is_empty() { 7 } else { 9 };
    let mut heap = vec![HeapValue::Null; heap_extent];
    for value in field(json, b"heap")?.array()? {
        check()?;
        let item = value.array()?;
        ensure!(
            item.len() >= if version >= 9 { 4 } else { 3 },
            "System4 legacy JSON heap record extent"
        );
        let index = usize::try_from(item[0].integer32()?)?;
        ensure!(index < 1_000_000, "System4 legacy JSON heap slot limit");
        if index >= heap.len() {
            heap.resize(index + 1, HeapValue::Null);
        }
        ensure!(
            matches!(heap[index], HeapValue::Null),
            "System4 legacy JSON duplicate heap slot"
        );
        let references = u32::try_from(item[1].integer32()?)?;
        ensure!(references > 0, "System4 legacy JSON heap refcount");
        let sequence = if version >= 9 {
            item[3].integer32()? as u32
        } else {
            0
        };
        heap[index] = match &item[2] {
            ByteJson::String(_) => {
                let mut bytes = cstring(&item[2])?;
                bytes.push(0);
                HeapValue::String {
                    references,
                    sequence,
                    unknown: 0,
                    bytes,
                }
            }
            ByteJson::Null => HeapValue::Array {
                references,
                sequence,
                rank_minus_one: -1,
                kind: 0,
                structure: Symbol::Name(Vec::new()),
                root_rank: -1,
                allocated: false,
                values: Vec::new(),
            },
            page => {
                let kind = cstring(field(page, b"type")?)?;
                let subtype = number(page, b"subtype")?;
                let values = field(page, b"values")?
                    .array()?
                    .iter()
                    .map(ByteJson::integer32)
                    .collect::<Result<Vec<_>>>()?;
                match kind.as_slice() {
                    b"globals" => HeapValue::Frame {
                        references,
                        sequence,
                        global: true,
                        function: Symbol::Id(-1),
                        types: program.globals.iter().map(|v| v.kind).collect(),
                        structure: -1,
                        values,
                    },
                    b"locals" => {
                        let f = program
                            .functions
                            .get(usize::try_from(subtype)?)
                            .context("System4 legacy JSON local function")?;
                        HeapValue::Frame {
                            references,
                            sequence,
                            global: false,
                            function: Symbol::Name(f.name.clone()),
                            types: f.variables.iter().map(|v| v.kind).collect(),
                            structure: -1,
                            values,
                        }
                    }
                    b"struct" => {
                        let s = program
                            .structures
                            .get(usize::try_from(subtype)?)
                            .context("System4 legacy JSON structure")?;
                        HeapValue::Structure {
                            references,
                            sequence,
                            constructor: function_symbol(program, s.constructor)?,
                            destructor: function_symbol(program, s.destructor)?,
                            structure: Symbol::Name(s.name.clone()),
                            types: s.members.iter().map(|v| v.kind).collect(),
                            values,
                        }
                    }
                    b"array" => {
                        let structure = number(page, b"struct-type")?;
                        let structure = if structure < 0 {
                            Vec::new()
                        } else {
                            program
                                .structures
                                .get(structure as usize)
                                .context("System4 legacy JSON array structure")?
                                .name
                                .clone()
                        };
                        let rank = number(page, b"rank")?;
                        ensure!((1..=32).contains(&rank), "System4 legacy JSON array rank");
                        HeapValue::Array {
                            references,
                            sequence,
                            rank_minus_one: rank - 1,
                            kind: subtype,
                            structure: Symbol::Name(structure),
                            root_rank: rank,
                            allocated: !values.is_empty(),
                            values,
                        }
                    }
                    b"delegate" => {
                        ensure!(
                            version >= 9 && values.len() % 3 == 0,
                            "System4 legacy JSON delegate profile"
                        );
                        HeapValue::Delegate {
                            references,
                            sequence,
                            values,
                        }
                    }
                    _ => anyhow::bail!("unknown System4 legacy JSON page type"),
                }
            }
        };
    }
    let calls = field(json, b"call-stack")?.array()?;
    ensure!(
        !calls.is_empty() && calls.len() <= 4096,
        "System4 legacy JSON call stack size"
    );
    let mut frames = vec![CallFrame {
        kind: 4,
        local: -1,
        structure: -1,
    }];
    let mut returns = vec![ReturnRecord {
        address: -1,
        function: None,
        local: 0,
        crc: 0,
    }];
    let mut previous = None;
    for call in calls {
        check()?;
        let function = usize::try_from(number(call, b"function")?)?;
        program
            .functions
            .get(function)
            .context("System4 legacy JSON call function")?;
        let structure = number(call, b"struct-page")?;
        frames.push(CallFrame {
            kind: if structure >= 0 {
                2
            } else if function == program.main as usize {
                0
            } else {
                1
            },
            local: number(call, b"local-page")?,
            structure,
        });
        if let Some(parent) = previous {
            returns.push(make_return(
                program,
                parent,
                number(call, b"return-address")?,
            )?);
        }
        previous = Some(function);
    }
    let ip = number(json, b"ip")?
        .checked_add(6)
        .context("System4 legacy JSON instruction overflow")?;
    let ip = make_return(program, previous.unwrap(), ip)?;
    let mut stack = field(json, b"stack")?
        .array()?
        .iter()
        .map(ByteJson::integer32)
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        (2..=65536).contains(&stack.len()),
        "System4 legacy JSON resume operand stack"
    );
    stack.truncate(stack.len() - 2);
    let next_sequence = if version >= 9 {
        number(json, b"next_seq")? as u32
    } else {
        0
    };
    let comments = optional_field(json, b"comments")
        .map(|v| v.array()?.iter().map(cstring).collect::<Result<Vec<_>>>())
        .transpose()?
        .unwrap_or_default();
    Ok(Resume {
        version,
        key: key.to_vec(),
        comments,
        body: Some(Body {
            ip,
            stack,
            frames,
            returns,
            next_sequence,
            heap,
            functions: program.functions.iter().map(|f| f.name.clone()).collect(),
        }),
    })
}
fn function_symbol(program: &Program, index: i32) -> Result<Symbol> {
    Ok(Symbol::Name(if index < 0 {
        Vec::new()
    } else {
        program
            .functions
            .get(index as usize)
            .context("System4 legacy JSON function identity")?
            .name
            .clone()
    }))
}
fn make_return(program: &Program, index: usize, address: i32) -> Result<ReturnRecord> {
    let f = program
        .functions
        .get(index)
        .context("System4 legacy JSON caller identity")?;
    let local = address
        .checked_sub(i32::try_from(f.address)?)
        .context("System4 legacy JSON caller offset")?;
    ensure!(local >= 0, "System4 legacy JSON negative caller offset");
    Ok(ReturnRecord {
        address,
        function: Some(f.name.clone()),
        local,
        crc: f.crc,
    })
}
