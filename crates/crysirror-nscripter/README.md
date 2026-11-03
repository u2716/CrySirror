# NScripter native adapter

`crysirror-nscripter` is a first-class Rust native text-script interpreter. Native
commands call CrySirror Layer, Canvas, Text/GlyphRun, UI, Image, Media, Audio and
storage services. ONScripter, SDL and an original engine renderer are not hosted.
Pinned reference provenance and licenses are recorded in SOURCE.json and LICENSE.

## Launch and configuration

Use `crysirror play /path/to/game` for desktop detection and resource bootstrap.
Original archives are mounted through shared resource services without bulk
extraction. Explicit adapter configuration supplies a capability-relative
`script`, licensed project `font` alias, optional `archives` mount prefixes,
`encoding` (`sjis` or `utf8`), canvas and text defaults. Loose resources precede
configured archives with scoped case-insensitive resolution. Numbered source
concatenation and other protection variants are not established by detection.

## Implemented source and limits

The original control-flow and resource adapter is extended by `src/commands.rs`.
Implemented command families include numeric/string/array operations, labels and
subroutines, jumps/loops, selections/buttons/dialogs, text expansion and ruby,
sprite2 transforms and sheet animation, menus/lookback/skip/auto modes, audio and
movie requests, effects, drawing and screen-copy requests. Presence in this list
means source exists; it is not a claim of full ONScripter semantic equivalence or
coverage of every command/argument profile.

Shared image workers prepare alpha/key/mask products; ordinary engine layers,
glyphs and widgets own presentation and interaction. Text-wait cursor animation
uses shared SheetSequence state and is distinct from an OS pointer. Screen-copy
operations request a presenter frame capture. The 2026-11-02 validation entry did
not execute those screen-copy paths. Metadata admission and remaining owner-thread
operations still need migration under the engine-wide async requirement.

`savegame`/`loadgame` now have a private engine slot format. These files and
`nscripter-state.json` global/file-log metadata do not interchange with original
NScripter `.dat`, global or log formats. Framework snapshots remain separate and
reject pending work or prepared-image reconstruction that cannot yet be restored.
`drawsp3`, native DLL execution and Lua execution fail visibly; other unsupported
commands/profiles also retain explicit errors. Window/transition pixel parity
and whole-game compatibility require separate validation.

## Validation evidence

The 2026-11-02 release entry records 13 passing authored tests and one ignored
owner-media test. Earlier 2026-10-30 owner Tsukihime runs reached the title and
hospital opening dialogue through shared services; they predate the command
expansion and do not validate it. See [STATUS.md](../../STATUS.md) for dated
commands, failures and actual scope. Tests read owner fonts/game data at runtime;
none are distributed. A command count or factory registration is not full engine
or game acceptance.
