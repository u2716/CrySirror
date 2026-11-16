# crysirror-lua

Pure-Rust Lua 5.3 for engine-hosted scripts. It loads precompiled binary
chunks (including 32-bit `size_t = 4` chunks such as AdvHD's Script.arc) and
runs functions to completion; the embedding engine supplies host functions
and userdata through the `Host` trait.

- `chunk`: loader, opcode decoder and a diagnostic disassembler
  (`examples/luadis.rs`).
- `interp`: all 47 Lua 5.3 opcodes with an explicit frame stack, open/closed
  upvalue cells, a per-call instruction budget and mark-sweep collection
  from explicit roots (tables, closures and cells live in index arenas).
- `stdlib`: base, `string` (Lua patterns, `format`), `math` (deterministic
  per-VM random) and `table`. `io` and `os` are host policy and are not
  opened here.

Not implemented: source compilation, metatables, coroutines, `goto`-free
`load` of text, and the `utf8` library. The AdvHD scripts use none of them.

Validation (2026-11-13): unit tests cover patterns and `string.format`; all
13 Script.arc chunks of the WillPlus test game load and run, and the game's
own title menu runs on this VM (see `crysirror-willplus`).
