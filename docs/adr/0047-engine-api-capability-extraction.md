# ADR 0047: Engine API capability extraction and runtime exceptions

Date: 2026-10-26. Status: accepted for `adopt` semantic adapters.
[ADR 0102](0102-engine-adoption-and-runtime-integration.md) defines the separate
`integrated` mode and its host/output/platform requirements.

The purpose of studying external engines is to expand CrySirror's reusable
capabilities. An `adopt` VM needs a thin semantic/syscall mapping to shared APIs,
not carry its original rendering, text, UI and audio engine with it.

BGI and SoftPAL use `integrated`, retaining complete implementations as internal modules of
workspace crates. Their session workers publish frames and PCM through
CrySirror; the host owns the window, event loop and output devices. Their
capability-extraction implementations remain preserved and inactive in the
default player. See [integration boundaries](../NATIVE_ADAPTERS.md#integration-boundaries).

`adopt` adapters follow the capability-extraction boundary. Preserving some
shared helper calls while flattening a native scene into an uploaded frame
does not satisfy this requirement. Running a game or matching screenshots
cannot be used as the acceptance criterion for capability extraction.

Generic mechanisms belong to engine crates and must have VM-independent API
coverage. Adapters preserve original instruction/value/layout/resource-policy/
wait/save semantics and translate their operations into those APIs and shared
logical state. Missing APIs must be implemented in CrySirror before integration.
