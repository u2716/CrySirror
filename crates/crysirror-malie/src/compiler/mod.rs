// SPDX-License-Identifier: MPL-2.0
//! The Malie script compiler of `malie.exe` (6758A0 → 675AE0), used by
//! MalieSystem 120 (613800) to compile a source file into an EXEC module.
//! The output has exec.dat's sections: the global scope's symbols, the
//! link table (functions), labels, strings, code and messages.
pub mod lex;
pub mod parse;
pub mod source;

use crate::program::Program;
use anyhow::Result;
pub use source::Source;

/// Compile `name` (extension replaced as 67E7D0 does). `None` when no
/// source file exists, which makes 613800 return 0.
pub fn compile(source: &mut dyn Source, name: &str) -> Result<Option<Program>> {
    let n = lex::w(name);
    let Some(chars) = source::open(source, &n)? else { return Ok(None) };
    let lexer = lex::Lexer::new(source, &n, chars);
    parse::Compiler::new(lexer).compile().map(Some)
}
