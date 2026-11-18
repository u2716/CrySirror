// SPDX-License-Identifier: MIT
//! Squirrel source VM without file, OS or device access. Every invocation has
//! an instruction allowance and a 32 MiB native arena. Host values are bounded
//! logical JSON values; retained closures are opaque per-session indices.
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    marker::PhantomData,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr::NonNull,
    rc::Rc,
};
unsafe extern "C" {
    fn csq_new(
        cb: extern "C" fn(*mut c_void, *mut c_void) -> c_int,
        ctx: *mut c_void,
    ) -> *mut c_void;
    fn csq_free(p: *mut c_void);
    fn csq_eval(p: *mut c_void, text: *const c_char, size: usize, budget: u64) -> c_int;
    fn csq_call(p: *mut c_void, name: *const c_char, reference: c_int, budget: u64) -> c_int;
    fn csq_error(p: *mut c_void) -> *const c_char;
    fn csq_fail(p: *mut c_void, text: *const c_char);
    fn csq_top(p: *mut c_void) -> c_int;
    fn csq_kind(p: *mut c_void, index: c_int) -> c_int;
    fn csq_integer(p: *mut c_void, index: c_int) -> i64;
    fn csq_push_integer(p: *mut c_void, n: i64) -> c_int;
    fn csq_number(p: *mut c_void, index: c_int) -> f64;
    fn csq_bool(p: *mut c_void, index: c_int) -> c_int;
    fn csq_string(p: *mut c_void, index: c_int, size: *mut usize) -> *const c_char;
    fn csq_next(p: *mut c_void, index: c_int) -> c_int;
    fn csq_pop(p: *mut c_void, count: c_int);
    fn csq_ref(p: *mut c_void, index: c_int) -> c_int;
    fn csq_push(p: *mut c_void, kind: c_int, n: f64, text: *const c_char, size: usize) -> c_int;
    fn csq_slot(p: *mut c_void, array: c_int) -> c_int;
}
type Host = Box<dyn FnMut(Vec<Value>) -> Result<Value>>;
struct ContextState {
    host: Host,
}
pub struct Interpreter {
    raw: NonNull<c_void>,
    context: Box<ContextState>,
    owner: PhantomData<Rc<()>>,
}
impl Interpreter {
    pub fn new(host: impl FnMut(Vec<Value>) -> Result<Value> + 'static) -> Result<Self> {
        let mut context = Box::new(ContextState {
            host: Box::new(host),
        });
        // Box keeps callback context stable. No callback can outlive the VM.
        let raw =
            NonNull::new(unsafe { csq_new(callback, (&mut *context as *mut ContextState).cast()) })
                .context("Squirrel VM allocation")?;
        Ok(Self {
            raw,
            context,
            owner: PhantomData,
        })
    }
    fn check(&self, status: i32) -> Result<()> {
        if status < 0 {
            let message = unsafe { CStr::from_ptr(csq_error(self.raw.as_ptr())) }.to_string_lossy();
            bail!("{message}");
        }
        Ok(())
    }
    pub fn eval(&mut self, text: &str, budget: u64) -> Result<()> {
        ensure!(text.len() <= 256 * 1024, "Squirrel source budget");
        // Bound recursive compiler nesting, excluding quoted strings/comments.
        let (mut depth, mut quote, mut escape, mut block, mut line) =
            (0u32, 0u8, false, false, false);
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            let next = bytes.get(i + 1).copied().unwrap_or(0);
            if line {
                if c == b'\n' {
                    line = false;
                }
            } else if block {
                if c == b'*' && next == b'/' {
                    block = false;
                    i += 1;
                }
            } else if quote != 0 {
                if escape {
                    escape = false;
                } else if c == b'\\' {
                    escape = true;
                } else if c == quote {
                    quote = 0;
                }
            } else if c == b'/' && next == b'/' {
                line = true;
                i += 1;
            } else if c == b'/' && next == b'*' {
                block = true;
                i += 1;
            } else if c == b'"' || c == b'\'' {
                quote = c;
            } else if b"({[".contains(&c) {
                depth += 1;
                ensure!(depth <= 128, "Squirrel compiler nesting budget");
            } else if b")}]".contains(&c) {
                depth = depth.saturating_sub(1);
            }
            i += 1;
        }
        self.check(unsafe { csq_eval(self.raw.as_ptr(), text.as_ptr().cast(), text.len(), budget) })
    }
    pub fn call(&mut self, name: &str, budget: u64) -> Result<()> {
        let name = CString::new(name)?;
        self.check(unsafe { csq_call(self.raw.as_ptr(), name.as_ptr(), -1, budget) })
    }
    pub fn callback(&mut self, reference: u32, budget: u64) -> Result<()> {
        ensure!(reference < 4096, "Squirrel closure reference");
        self.check(unsafe {
            csq_call(
                self.raw.as_ptr(),
                std::ptr::null(),
                reference as i32,
                budget,
            )
        })
    }
}
impl Drop for Interpreter {
    fn drop(&mut self) {
        unsafe { csq_free(self.raw.as_ptr()) };
        let _ = &self.context;
    }
}
extern "C" fn callback(context: *mut c_void, raw: *mut c_void) -> c_int {
    let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
        let top = unsafe { csq_top(raw) };
        ensure!(top <= 64, "Squirrel host arity budget");
        let mut bytes = 0;
        let mut args = Vec::new();
        for index in 2..=top {
            args.push(read(raw, index, 0, &mut bytes)?);
        }
        let context = unsafe { &mut *context.cast::<ContextState>() };
        let value = (context.host)(args)?;
        push(raw, &value, 0)
    }));
    match result {
        Ok(Ok(())) => 1,
        other => {
            let message = match other {
                Ok(Err(e)) => format!("{e:#}"),
                _ => "Squirrel host callback panicked".into(),
            };
            let message = message.replace('\0', " ");
            if let Ok(c) = CString::new(message) {
                unsafe { csq_fail(raw, c.as_ptr()) };
            }
            -1
        }
    }
}
fn read(raw: *mut c_void, index: i32, depth: u32, bytes: &mut usize) -> Result<Value> {
    ensure!(
        depth < 16 && *bytes < 1024 * 1024,
        "Squirrel host value budget"
    );
    *bytes += 8;
    // All indices refer to the active callback stack. Values are copied before
    // returning to the VM; no guest pointer escapes this module.
    unsafe {
        Ok(match csq_kind(raw, index) {
            0 => Value::Null,
            1 => Value::Bool(csq_bool(raw, index) != 0),
            2 => json!(csq_integer(raw, index)),
            3 => json!(csq_number(raw, index)),
            4 => {
                let mut n = 0;
                let ptr = csq_string(raw, index, &mut n);
                ensure!(
                    !ptr.is_null() && n <= 256 * 1024 && *bytes + n <= 1024 * 1024,
                    "Squirrel string budget"
                );
                *bytes += n;
                Value::String(
                    std::str::from_utf8(std::slice::from_raw_parts(ptr.cast(), n))?.into(),
                )
            }
            5 | 6 => {
                let array = csq_kind(raw, index) == 6;
                let absolute = if index < 0 {
                    csq_top(raw) + index + 1
                } else {
                    index
                };
                ensure!(
                    csq_push(raw, 0, 0., std::ptr::null(), 0) >= 0,
                    "Squirrel iteration allocation"
                );
                let mut out = serde_json::Map::new();
                let mut list = Vec::new();
                let mut count = 0;
                loop {
                    let result = csq_next(raw, absolute);
                    ensure!(result >= 0, "Squirrel table iteration");
                    if result == 0 {
                        break;
                    }
                    count += 1;
                    ensure!(count <= 4096, "Squirrel host table budget");
                    let key = read(raw, -2, depth + 1, bytes)?;
                    let value = read(raw, -1, depth + 1, bytes)?;
                    csq_pop(raw, 2);
                    if array {
                        list.push(value);
                    } else {
                        out.insert(
                            key.as_str()
                                .context("Squirrel host table key must be a string")?
                                .into(),
                            value,
                        );
                    }
                }
                csq_pop(raw, 1);
                if array {
                    Value::Array(list)
                } else {
                    Value::Object(out)
                }
            }
            7 => {
                let reference = csq_ref(raw, index);
                ensure!(reference >= 0, "Squirrel callback reference budget");
                json!({"$function":reference})
            }
            kind => bail!("Squirrel value kind {kind} cannot cross host boundary"),
        })
    }
}
fn push(raw: *mut c_void, value: &Value, depth: u32) -> Result<()> {
    ensure!(depth < 16, "Squirrel host return nesting");
    if let Value::Number(n) = value {
        if let Some(n) = n.as_i64() {
            ensure!(
                unsafe { csq_push_integer(raw, n) } >= 0,
                "Squirrel integer return allocation"
            );
            return Ok(());
        }
    }
    let (kind, n, text) = match value {
        Value::Null => (0, 0., ""),
        Value::Bool(v) => (1, f64::from(*v), ""),
        Value::Number(v) => (3, v.as_f64().context("Squirrel number")?, ""),
        Value::String(v) => (4, 0., v.as_str()),
        Value::Object(_) => (5, 0., ""),
        Value::Array(_) => (6, 0., ""),
    };
    ensure!(text.len() <= 256 * 1024, "Squirrel return string budget");
    ensure!(
        unsafe { csq_push(raw, kind, n, text.as_ptr().cast(), text.len()) } >= 0,
        "Squirrel return allocation"
    );
    match value {
        Value::Object(map) => {
            ensure!(map.len() <= 4096, "Squirrel return table budget");
            for (key, value) in map {
                push(raw, &Value::String(key.clone()), depth + 1)?;
                push(raw, value, depth + 1)?;
                ensure!(unsafe { csq_slot(raw, 0) } >= 0, "Squirrel return slot");
            }
        }
        Value::Array(list) => {
            ensure!(list.len() <= 4096, "Squirrel return array budget");
            for value in list {
                push(raw, value, depth + 1)?;
                ensure!(unsafe { csq_slot(raw, 1) } >= 0, "Squirrel return item");
            }
        }
        _ => {}
    }
    Ok(())
}
