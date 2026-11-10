# ADR 0092: Shared document grammar and native JSON facade

Date: 2026-10-28
Status: implemented bounded document/native JSON profile; async read boundaries remain partial

## Decision and ownership

KiriKiri's json.dll provides four Scripts members but its parser/writer are
reusable engine capabilities. Add `crysirror-runtime::structured_text` with
DocumentValue, explicit JsonDialect, parse_json and write_json. Values keep exact
i64, real, text, void, ordered dictionary pairs and arrays. The pure safe Rust
kernel has no VM, storage, window, renderer or device dependency and is tested
without a native interpreter.

The explicit KiriKiri dialect admits comments, quoted strings/keys, alternative
separators, trailing separators and empty array slots. `1e5` retains the original
integer strtoll quirk and becomes integer 1; decimal tokens use real conversion.
Duplicate keys overwrite the earlier position. Bare-word keys are rejected,
following the native JSON grammar.
The reference's single-value/trailing-input behavior is retained. Pretty output
uses one-space indentation and CRLF by default, or explicit LF.

## Native facade and I/O completion

Plugins.link('json.dll') installs actual Scripts.evalJSON, evalJSONStorage,
toJSONString and saveJSON methods, backed by the shared kernel. Native value and
encoding conversion are semantic adapters, not a private document parser.
Structured ordinary dictionaries/arrays/scalars are the admitted serializer
profile. Callable objects, cycles, nonfinite numbers, malformed Unicode and
excessive shapes fail visibly. No guess at unavailable hidden-member flags or
arbitrary custom object serializer/getter behavior is claimed.

The facade's files stay within the client's scoped Storage service. UTF-8 is
explicit; default storage encoding uses the documented Japanese CP932 profile.
It does not claim arbitrary Windows ACP equivalence. Reading checks stream length
before allocation through Storage::read_limited. Saving uses the existing atomic
VFS worker and retained write completion. The original TJS caller suspends until
that commit finishes; nonresumable callbacks reject before task admission.
No OS DLL, absolute filename or ambient host filesystem is exposed.

Input text is at most 1 MiB, graph depth at most 64 and total nodes at most 65536;
output is at most 4 MiB. Object-key lookup during parsing uses an ordered index,
avoiding quadratic duplicate-key searches. In-memory parse/write and bounded
read calls currently execute synchronously at the owner/native boundary. This
remaining ADR 0050 boundary is explicit; this phase does not declare whole-engine
async completion or hide this work inside an unrelated worker API.

## Plugin support claims

The current user-facing plug-in compatibility list is in
[the KiriKiri guide](../KIRIKIRI.md#plug-in-compatibility). A DLL distributed with
a game does not by itself establish compatibility with that interface.

This batch implements the bounded JSON facade. ADR 0093 separately records the
solid-linear LayerExDraw mapping. No alternate software rasterizer is imported.
Extended KAG parsers
are not aliases for the current KAGParser facade. Unknown plugins continue to
fail visibly; no automatically enabled registration shells are added.

## Validation

Implementation, release compilation and behavior results remain separate.
Release `cargo check -p crysirror-runtime -p crysirror-krkr --tests` passed during
implementation. Release shared-kernel tests: **2 passed**. Release native fixture
tests: **2 passed**. Local logs are target/verification/krkr-json-{shared,native}-tests.log.
The integrating owner records these separately in STATUS.md. They cover dialect
quirks, independent round trips, shape/error
budgets, actual plugin registration, original TJS waits, UTF-8 atomic save/load,
cycles and rejection of an oversized stream. No game files or fonts are shipped.
