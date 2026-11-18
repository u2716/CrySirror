# RScript ADR 0001: Adapt native plugins when required by a game

Date: 2026-11-17
Status: accepted
Scope: codeX RScript

## Context

In the audited 1.11.0.3 profile, opcode `0x19` starts a plugin scene through
`LoadLibraryA` and `GetInfo` (`sub_43D500`). Decoding that instruction does not
provide the plugin's behavior or establish portable plugin support.

## Decision

Implement ordinary RScript runtime features independently of plugin support.
Adapt a named plugin when an actual game requires it, using its verified ABI
and recovered behavior. Keep plugin argument, state, completion and save
semantics in the RScript adapter and reuse CrySirror capabilities for output.

Unknown plugins fail explicitly with their name and instruction context.
They must not be silently skipped, reported as successful, or loaded as native
host code. A supported plugin uses a maintained portable implementation rather
than a general game-DLL loader.

Plugin deferral does not defer load/restore, configuration, backlog, controls,
animation, transitions or other ordinary runtime behavior. Instruction coverage
and functional runtime coverage remain separate claims.

## Implementation (2026-11-17)

`crysirror_rscript::plugins` holds the facade registry keyed by canonical
DLL name (scene and extension interfaces). A DLL the game does not ship
takes the native load-failure path; a shipped DLL without a facade fails
with its name.
