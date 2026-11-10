# Supported game engines

CrySirror opens original game directories and reads their scripts and resources
in place. Game detection, archive mounting and startup are automatic.

## Installed-game launch

```sh
./target/release/crysirror play "/path/to/game"
```

You can also select a file inside the game directory. Keep the original directory
layout, patch archives, executable and companion files together: some games use
them to identify or decode their resources.

Use `--font /path/to/font.ttc` to choose a font, `--no-audio` to mute playback,
and `--data-dir /path/to/user-data` to choose where saves and settings are stored.
The [desktop guide](DESKTOP_API.md) lists the available launch options.

## Engine support

| Engine | Game data and features |
| --- | --- |
| [KiriKiri 2 / KiriKiri Z · KAG3](KIRIKIRI.md) | XP3 archives, TJS/KAG scenarios, layers, text, choices, transitions, audio, movies and the [plug-ins listed here](KIRIKIRI.md#plug-in-compatibility). |
| [Siglus](#siglus) | Gameexe/Scene data, G00 graphics, message windows, character graphics, animation, effects, audio, movies and game saves. |
| [RealLive](#reallive) | Gameexe.ini/Seen scenarios, PDT/G00 graphics, text windows, choices, menus, backlog, music, sound, voice and movies. |
| [AVG32](#avg32) | Scenario data, PDT10/11 graphics, text, choices, menus, backlog, music, voice, Cinepak AVI and SAVE.INI slots. |
| [BGI / Ethornell](#bgi-and-softpal) | BP scenarios, ARC resources, CBG graphics, character composition, text, effects, audio and game saves. |
| [SoftPAL](#bgi-and-softpal) | PAL resources and scripts, sprites, text, choices, animation, audio, movies and game saves. |
| [RFVP / FAVORITE View Point](#rfvp) | HCB scenarios, BIN archives and loose resources, graphics, text, effects, audio, movies, custom cursors and game saves. |
| [NScripter](#nscripter) | Text scenarios, SAR/NSA archives, sprites, animation, ruby, choices, menus, backlog, skip/auto, audio and movies. |
| [Circus](#circus) | DC4 Plus Harmony MES scenarios, PCK/DAT resources, CRXG graphics, XPCM audio, dialogue, menus, screen effects and native save slots. |
| [CatSystem2](#catsystem2) | INT/FWP resources, HG3 graphics, KCS/FES/CST scenarios, dialogue, image effects, E-mote and menus. |
| [QLIE](#qlie) | FilePack archives, .s scenarios, ABMP animation, DPNG graphics, text/ruby, buttons, transitions, audio, movies and saves. |
| [Majiro](#majiro) | MJO scenarios, ARC resources, RCT/RC8 graphics, text, choices, sprite animation, music, voice, backlog and saves. |
| [AliceSoft System 4](#alicesoft-system-4) | System40.ini, AIN v5 scenarios, ALD/ALK resources and QNT graphics. |
| [System-NNN](#system-nnn) | SPT scenarios, Black Cyc archives, DWQ graphics, VAW audio, text, choices and picture effects. |
| [SugarPot](#sugarpot) | The Tsukumo no Tsuki format: AOS2/SCR resources, ABM/BMP graphics, sprites, text, choices, audio, movies and the H-scene gallery. |

## KiriKiri

KiriKiri 2, KiriKiri Z and KAG3 games use their original TJS scripts and XP3
resources. Support includes custom cursors, layer effects, menus, video and
several extended KAG parsers.

Plug-ins differ between games. See the [KiriKiri compatibility guide](KIRIKIRI.md)
for supported DLL interfaces, image effects, transitions and plug-in restrictions.

## Siglus

Reads the original Gameexe and Scene files, graphics, voice banks, music and
movies. Character composition, message windows, effects, animation and native
save files use the game's original runtime behaviour.

Decoding material is resolved from the installed Gameexe/Scene pair where
available. An explicit `key.toml` can supply game-specific material. Keep patch
files and any existing key file with the installation.

## RealLive

Reads Gameexe.ini, Seen scenarios and the original resource directories. Text
windows, names, choices, save/load menus, backlog, music, sound and voice use
the game's own scenario and configuration data. Native cursors and window
settings are supported.

Resource naming and text encoding follow the selected game. Use the launch
options in the [desktop guide](DESKTOP_API.md) for a font or encoding override.

## AVG32

Supports PDT10/11 images, message windows, choices, menus, backlog, name entry,
music, sound and voice. Original `SAVE.INI` save slots are supported.

AVI playback supports Cinepak video. AVI files using other video codecs or
compressed audio are outside this playback format.

## BGI and SoftPAL

Both engines open their original resources directly and keep their native text,
graphics, audio and save behaviour.

BGI also recognizes game data under `Install/`. Installed-game configuration and
save writes go to a writable user directory, with later reads seeing those files
first. See [BGI / Ethornell](BGI.md) for directory and launch details.

SoftPAL preserves the game's sprite positioning, aspect-mode choices and native
save layout. Window resizing changes presentation without changing game
coordinates.

## RFVP

Runs original FAVORITE View Point HCB scenarios with BIN archives and loose
resources. Features include native graphics and text composition, transitions,
streaming audio, custom cursors and original game saves. Movies can be read from
native archives as well as loose files.

## NScripter

Supports SAR/NSA archives and original text scenarios, numeric/string/array
variables, calls and loops, sprites and sprite animation, ruby, selections,
buttons, menus, backlog, skip/auto, audio and movies. Loose files take precedence
over archives; numbered NSA patches take precedence over the base archive.

Saves created by CrySirror are not interchangeable with original NScripter
`.dat` save files. Native DLL calls, Lua execution and `drawsp3` are unavailable.

## Circus

The executable scenario format is the DC4 Plus Harmony MES variant. Features
include dialogue, menus, CRXG images, XPCM PCM/Ogg audio, transitions, falling
sprite effects, native save slots and thumbnails.

The older DC4 MES format can be recognized but cannot be played. XPCM spectral
audio formats are unavailable.

## CatSystem2

Reads INT archives, automatically decodes supported FWP-wrapped resources, and
loads HG3 images. KCS/FES/CST scenarios use the game's text, image, animation,
input and menu definitions. E-mote character animation and image filters are
included.

Game-specific classes and script extensions affect compatibility. An unsupported
class or command is reported by name rather than skipped.

## QLIE

Reads original FilePack resources in game/patch order, including supported
embedded keys and encrypted resources. Features include ABMP animation, DPNG
images, formatted text and ruby, collision buttons, image filters, transitions,
audio, movies and save thumbnails.

CrySirror save files retain a separate state body. Saves and SystemData files
created by the original player are not imported. The retired Twitter posting
service is unavailable.

## Majiro

Supports MJO scripts and ARC resources, RCT/RC8 images, picture blending and
masks, sprite animation, text and choices, music/sound/voice, message history and
save/load menus.

Saves use CrySirror's own format. Original Majiro save files cannot be loaded.
Movie playback, save thumbnails and backlog voice replay are unavailable.

## AliceSoft System 4

Recognizes System40.ini installations and reads AIN v5 scenarios, ALD/ALK
resources and QNT images. SACT2 and other game libraries have game-specific
interfaces; unsupported library functions are reported when used.

Original game-save import and export are unavailable.

## System-NNN

Supports the Legacy1000 resource and script family, including SPT scenarios,
Black Cyc archives, DWQ images, VAW audio, text, selection regions, music, sound,
voice and picture effects.

Music supports MML segment playlists and nested loops. Text includes inline
colours, player-name substitutions, ruby and image glyphs from the game's own
font sheet. Native `dataN.sav` quick-save/load supports notifications, thumbnails
and pre-load screen capture.

Original saves restore message and ordinary/timed selection states, including
Cutin row records and configured ChartMenu selection/road extensions. Omake
includes CG catalogues, differences, frame animations, scrolling and tiled
pictures, delayed replay voices, scene-film replay, music and movies. Some
gallery transitions, ChartMenu's map interface, game-specific save extensions
and other native mode variants remain unavailable.

## SugarPot

Supports the 2012 Tsukumo no Tsuki script and resource format. Features include
AOS2/SCR loading, ABM frame animation, sprites, text pagination, choices,
background music, sound, voice, movies, screen fades and the H-scene gallery.

Settings, global flags and read-history files use their original formats.
Save-slot restoration is unavailable. Other games using similar LiLiM/AOS2
containers may use a different scenario language.

## Network storage

Game paths can use SMB or NFS:

```sh
./target/release/crysirror play 'smb://server/share/game'
./target/release/crysirror play 'nfs://server/export/game'
```

Resources are read on demand; saves remain local. Siglus, BGI, SoftPAL and RFVP
network access requires the matching optional `smb` or `nfs` build feature.
See [SMB](DESKTOP_API.md#games-on-an-smb-share) and
[NFS](DESKTOP_API.md#games-on-an-nfs-export) for credentials and server options.

## Fonts, windows and menus

`--font` selects the player-provided font. Games can also select fonts from
their own resources. Fonts are not bundled with CrySirror.

The game canvas keeps its original coordinates when the window is resized.
On macOS the application menu is at the top of the screen; Windows uses the
window menu bar. Linux uses the engine menu interface. Windowed games retain
their title bar unless borderless mode is explicitly enabled.
