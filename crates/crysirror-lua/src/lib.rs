// SPDX-License-Identifier: MPL-2.0
//! Pure-Rust Lua 5.3: binary chunk loading and a run-to-completion interpreter
//! whose host functions are dispatched through a caller-supplied `Host`.
#![forbid(unsafe_code)]
pub mod chunk;
pub mod interp;
pub mod stdlib;
pub mod value;
pub use interp::{Host, Lua};
pub use value::{FuncRef, TableRef, Value};
