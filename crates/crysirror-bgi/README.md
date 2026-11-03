# BGI native adapter

The BP VM and script parser from xmoezzz's [ethornell-rs](https://github.com/xmoezzz/ethornell-rs)
are maintained directly in `crysirror-bgi-vm`. This includes the original operand
ring, memory/heap, BP modules, native thread and cooperative scheduler logic,
scenario helpers, native ABI profiles and native save-data mechanisms.
The complete original application, renderer and audio backend are not linked.

Config: `{ "script": "main._bp", "root": "", "font": "main" }`.
The entry point is a BP bootstrap, not a standalone Buriko scenario file.
Dynamic BP reads run on bounded workers through client storage and shared
BGI archive readers. Missing modules fail; they never become empty scripts.

Native bitmaps are immutable CrySirror image resources presented through
Canvas/Layer. Integer blits, region copies, transforms, colour curves and native
shadow glyph records execute in shared image/text workers. Font resources use
the client font library. Markup and ruby registries use the shared text module.
DSC decoding is in `crysirror-resource::dsc`, CBG decoding in `crysirror-image`.
Strategy grids, body queries, splines and tactical evaluation are public
`crysirror-scene::strategy` capabilities, independent of this VM.
Native sound slots, gain banks, ramps and virtual CD-DA use shared audio
resources, streaming decoders and channels. Host calls suspend their issuing
thread; per-resource dependencies order bitmap/audio mutations while other
native threads continue. Installed BP bootstraps and BGI archive mounts are
recognized by the desktop detector.

Full native host coverage is still incomplete: window/message procedures,
remaining object/effect classes, movie controls and native save-host operations
need further mappings. Whole-adapter snapshot/restore remains unsupported.
Unknown operations fail explicitly. These changes have not been compiled or
behavior-tested.
