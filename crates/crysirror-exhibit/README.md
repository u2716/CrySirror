# ExHibit

CrySirror maintains ExHibit in **adopt** mode. The adapter interprets RLD records and maps them to CrySirror storage, media, audio, layers and clock services. The launcher reads the entry and dimensions from the installed ExHIBIT.ini. The RLD header selects the record layout. The v3 loader additionally checks the installed initialization routine.

| Native profile | Implemented paths | Current startup boundary |
| --- | --- | --- |
| RLD v1 (Famisupi installation) | Local and system register operations, conditional jumps and calls, block checkpoints, experience counters, slot index queries, selected CG, audio, movie and wait forms, path billboards, three-cell button attachment and disabling, fixed-list panel motion and fades | Release playback displays the six-button title panel at (338,460), disables unavailable items, reveals the logo, blocks input on other layers and releases the completed CG pan. It reaches the selection wait at title.rld+0x870 (0x5c). Hover, click and keyboard activation remain open. |
| RLD v3 (2011 resident, Midori no Umi installation) | Locally derived scenario parameters, selected register assignments and branches, environment definitions, binary arithmetic and action subcommands 190, 262, 346, 348 and 360 for literal operands | Release playback reaches start.rld+0x114. The taken block requires a camera, layer and printer checkpoint. |
| Later v3 (Stray Sheep volumes) | Header identification and scoped AiFS voice bank | Ordinary scenario initialization and execution remain open. |

Shared resource decoders cover four GYU compression tags, single-volume AiFS v5 numeric Ogg banks and the loose PCM voice page types found in Famisupi. Original UI, dialogue, input, game saves and other native command families remain outstanding.

[Native instruction and syscall inventory](NATIVE_ABI.md) · [Remaining work](SATD.md) · [Static resource evidence](../../docs/validation/exhibit-gyu.md)
