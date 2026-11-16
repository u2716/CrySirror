// SPDX-License-Identifier: MPL-2.0
//! Lua 5.3 binary chunk loader. The header's `size_t` width is honoured, so
//! chunks produced by 32-bit hosts (size_t = 4) load on 64-bit targets.
use anyhow::{bail, ensure, Context, Result};
use std::sync::Arc;

pub type Bytes = Arc<[u8]>;

#[derive(Clone, Debug)]
pub enum Constant {
    Nil,
    Bool(bool),
    Int(i64),
    Num(f64),
    Str(Bytes),
}

#[derive(Debug)]
pub struct Proto {
    pub source: Option<Bytes>,
    pub line: i32,
    pub params: u8,
    pub vararg: bool,
    pub stack: u8,
    pub code: Vec<u32>,
    pub constants: Vec<Constant>,
    /// (instack, index) per upvalue.
    pub upvalues: Vec<(bool, u8)>,
    pub protos: Vec<Arc<Proto>>,
    pub lines: Vec<i32>,
}

const MAX_CHUNK: usize = 64 << 20;
const MAX_ITEMS: usize = 1 << 22;

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    size_t: usize,
}
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.at.checked_add(n).context("Lua chunk extent")?;
        let out = self.bytes.get(self.at..end).context("truncated Lua chunk")?;
        self.at = end;
        Ok(out)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn int(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn count(&mut self) -> Result<usize> {
        let n = self.int()?;
        ensure!((0..=MAX_ITEMS as i32).contains(&n), "Lua chunk item count {n}");
        Ok(n as usize)
    }
    fn size(&mut self) -> Result<usize> {
        let raw = self.take(self.size_t)?;
        let mut value = 0u64;
        for (n, byte) in raw.iter().enumerate() {
            value |= u64::from(*byte) << (8 * n);
        }
        usize::try_from(value).context("Lua string size")
    }
    fn string(&mut self) -> Result<Option<Bytes>> {
        let mut n = usize::from(self.byte()?);
        if n == 0xff {
            n = self.size()?;
        }
        if n == 0 {
            return Ok(None);
        }
        ensure!(n <= MAX_CHUNK, "Lua string budget");
        Ok(Some(self.take(n - 1)?.into()))
    }
    fn function(&mut self, depth: usize) -> Result<Proto> {
        ensure!(depth < 200, "Lua prototype nesting");
        let source = self.string()?;
        let line = self.int()?;
        let _last = self.int()?;
        let params = self.byte()?;
        let vararg = self.byte()? != 0;
        let stack = self.byte()?;
        let n = self.count()?;
        let code = (0..n).map(|_| Ok(u32::from_le_bytes(self.take(4)?.try_into()?))).collect::<Result<Vec<_>>>()?;
        let n = self.count()?;
        let mut constants = Vec::with_capacity(n);
        for _ in 0..n {
            constants.push(match self.byte()? {
                0 => Constant::Nil,
                1 => Constant::Bool(self.byte()? != 0),
                3 => Constant::Num(f64::from_le_bytes(self.take(8)?.try_into()?)),
                0x13 => Constant::Int(i64::from_le_bytes(self.take(8)?.try_into()?)),
                4 | 0x14 => Constant::Str(self.string()?.unwrap_or_else(|| Arc::from(&b""[..]))),
                other => bail!("Lua constant tag {other:#x}"),
            });
        }
        let n = self.count()?;
        let upvalues = (0..n).map(|_| Ok((self.byte()? != 0, self.byte()?))).collect::<Result<Vec<_>>>()?;
        let n = self.count()?;
        let protos = (0..n).map(|_| self.function(depth + 1).map(Arc::new)).collect::<Result<Vec<_>>>()?;
        let n = self.count()?;
        let lines = (0..n).map(|_| self.int()).collect::<Result<Vec<_>>>()?;
        for _ in 0..self.count()? {
            self.string()?;
            self.int()?;
            self.int()?;
        }
        for _ in 0..self.count()? {
            self.string()?;
        }
        Ok(Proto { source, line, params, vararg, stack, code, constants, upvalues, protos, lines })
    }
}

/// Parse a precompiled Lua 5.3 chunk (`luac` output, possibly stripped).
pub fn load(bytes: &[u8]) -> Result<Arc<Proto>> {
    ensure!(bytes.len() <= MAX_CHUNK, "Lua chunk budget");
    ensure!(bytes.starts_with(b"\x1bLua") && bytes.get(4) == Some(&0x53) && bytes.get(5) == Some(&0), "not a Lua 5.3 binary chunk");
    ensure!(bytes.get(6..12) == Some(&b"\x19\x93\r\n\x1a\n"[..]), "Lua chunk LUAC_DATA");
    let sizes = bytes.get(12..17).context("Lua chunk header")?;
    ensure!(sizes[0] == 4 && sizes[2] == 4 && sizes[3] == 8 && sizes[4] == 8, "Lua chunk int/instruction/number sizes {sizes:?}");
    ensure!(matches!(sizes[1], 4 | 8), "Lua chunk size_t width {}", sizes[1]);
    let mut reader = Reader { bytes, at: 17, size_t: usize::from(sizes[1]) };
    ensure!(i64::from_le_bytes(reader.take(8)?.try_into()?) == 0x5678, "Lua chunk integer format");
    ensure!(f64::from_le_bytes(reader.take(8)?.try_into()?) == 370.5, "Lua chunk float format");
    let _upvalues = reader.byte()?;
    let main = reader.function(0)?;
    ensure!(reader.at == bytes.len(), "trailing bytes after Lua chunk");
    Ok(Arc::new(main))
}

pub const OPCODES: [&str; 47] = [
    "MOVE", "LOADK", "LOADKX", "LOADBOOL", "LOADNIL", "GETUPVAL", "GETTABUP", "GETTABLE", "SETTABUP", "SETUPVAL", "SETTABLE",
    "NEWTABLE", "SELF", "ADD", "SUB", "MUL", "MOD", "POW", "DIV", "IDIV", "BAND", "BOR", "BXOR", "SHL", "SHR", "UNM", "BNOT",
    "NOT", "LEN", "CONCAT", "JMP", "EQ", "LT", "LE", "TEST", "TESTSET", "CALL", "TAILCALL", "RETURN", "FORLOOP", "FORPREP",
    "TFORCALL", "TFORLOOP", "SETLIST", "CLOSURE", "VARARG", "EXTRAARG",
];

#[derive(Clone, Copy, Debug)]
pub struct Op {
    pub code: u8,
    pub a: u32,
    pub b: u32,
    pub c: u32,
}
impl Op {
    #[inline]
    pub fn decode(word: u32) -> Self {
        Op { code: (word & 0x3f) as u8, a: (word >> 6) & 0xff, c: (word >> 14) & 0x1ff, b: (word >> 23) & 0x1ff }
    }
    #[inline]
    pub fn bx(word: u32) -> u32 {
        word >> 14
    }
    #[inline]
    pub fn sbx(word: u32) -> i32 {
        (word >> 14) as i32 - 131071
    }
    #[inline]
    pub fn ax(word: u32) -> u32 {
        word >> 6
    }
}

/// Human-readable listing (diagnostics; no execution).
pub fn disassemble(proto: &Proto, path: &str, out: &mut String) {
    use std::fmt::Write;
    let k = |index: u32| -> String {
        match &proto.constants[index as usize] {
            Constant::Nil => "nil".into(),
            Constant::Bool(v) => v.to_string(),
            Constant::Int(v) => v.to_string(),
            Constant::Num(v) => format!("{v:?}"),
            Constant::Str(s) => format!("{:?}", String::from_utf8_lossy(s)),
        }
    };
    let rk = |v: u32| if v & 256 != 0 { k(v & 255) } else { format!("r{v}") };
    let _ = writeln!(out, "function {path} line {} params {} vararg {} stack {}", proto.line, proto.params, proto.vararg, proto.stack);
    for (pc, word) in proto.code.iter().enumerate() {
        let op = Op::decode(*word);
        let name = OPCODES.get(usize::from(op.code)).copied().unwrap_or("?");
        let line = proto.lines.get(pc).copied().unwrap_or(0);
        let detail = match name {
            "LOADK" => format!("r{} = {}", op.a, k(Op::bx(*word))),
            "GETTABUP" => format!("r{} = U{}[{}]", op.a, op.b, rk(op.c)),
            "SETTABUP" => format!("U{}[{}] = {}", op.a, rk(op.b), rk(op.c)),
            "GETTABLE" => format!("r{} = r{}[{}]", op.a, op.b, rk(op.c)),
            "SETTABLE" => format!("r{}[{}] = {}", op.a, rk(op.b), rk(op.c)),
            "SELF" => format!("r{} = r{}; r{} = r{}[{}]", op.a + 1, op.b, op.a, op.b, rk(op.c)),
            "ADD" | "SUB" | "MUL" | "MOD" | "POW" | "DIV" | "IDIV" | "BAND" | "BOR" | "BXOR" | "SHL" | "SHR" | "EQ" | "LT" | "LE" => {
                format!("{} {} {}", op.a, rk(op.b), rk(op.c))
            }
            "JMP" | "FORLOOP" | "FORPREP" | "TFORLOOP" => format!("{} -> {}", op.a, pc as i32 + 1 + Op::sbx(*word)),
            "CLOSURE" => format!("r{} = closure {path}/{}", op.a, Op::bx(*word)),
            _ => format!("{} {} {}", op.a, op.b, op.c),
        };
        let _ = writeln!(out, "  {pc:5} [{line:5}] {name:9} {detail}");
    }
    for (n, child) in proto.protos.iter().enumerate() {
        disassemble(child, &format!("{path}/{n}"), out);
    }
}
