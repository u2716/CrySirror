# KiriKiri 2 / KiriKiri Z / KAG3

```sh
./target/release/crysirror play "/path/to/game"
```

Keep XP3 archives, patches, the game's executable and companion files in their
original layout. CrySirror reads them in place and starts the original TJS/KAG
scripts. Supported resource protection includes HXV4 and supported cxdec-family
filters; additional game-specific filters may require their accompanying files.

## Game features

- TJS scripts and KAG3 scenarios, labels, macros, calls, choices and message waits.
- Layer positioning, scaling, rotation, clipping, blending, image filters and
  custom mouse cursors.
- Text windows, font selection, ruby, typewriter text and the TextRender extension.
- Music, voice, sound effects, Vorbis/Opus audio, ordinary movies and AJPM alpha movies.
- Menu bars, layer movies, PSB images and M2 motion playback.
- Crossfade, rule-image transitions, and Kaicho blur/dim transitions.

Plug-in names below describe the interfaces built into CrySirror. They do not
imply every export of a similarly named DLL is available.

## Plug-in compatibility

### Scenarios and script data

| Plug-in | Available features | Restrictions |
| --- | --- | --- |
| `KAGParser.dll` | KAG scenario parsing, labels, macros and scenario execution. | Game-specific tags still need a matching handler. |
| `KAGParserEx.dll` | Extended KAG parser and tag-list access. | Supports the tag-list interface used by the scenario parser. |
| `ExtKAGParser.dll` | Extended scenarios with `while`, local dictionaries, multiline commands and parameter macros. | Parser-state saves and fuzzy parsing modes are unavailable. |
| `ScriptsEx.dll` | Script reflection and structure helpers. | — |
| `json.dll` | JSON parsing, serialization, loading and saving; Japanese game-script extensions. | Plain values, arrays and dictionaries; callable/cyclic objects are not serializable. |
| `toml.dll` | Loading and decoding TOML documents. | `Scripts.tomlDecodeFromStorage`. |
| `csvparser.dll` | CSV document parsing. | — |
| `lineparser.dll` | Line-by-line text parsing. | — |
| `dirlist.dll` | Directory listings and file metadata. | Access is limited to the game's resource namespace. |
| `varfile.dll` | In-memory binary files and image loading from those files. | — |
| `saveStruct.dll` | Structured data storage. | — |
| `fstat.dll` | File information queries. | — |

### Graphics and text

| Plug-in | Available features | Restrictions |
| --- | --- | --- |
| `layerExImage.dll` | Brightness/contrast, colorization, HSL adjustment, noise, white noise and Gaussian blur. | Only these image operations are available. |
| `layerExRaster.dll` | Sine-wave row copying with `copyRaster`. | Overlapping in-place self-copy is unavailable. |
| `layerExDraw.dll` | Solid-colour paths, rectangles, polygons and connected lines, with transforms. | Pens, gradients, curves, ellipses, GDI text and metafiles are unavailable. |
| `layerExBTOA.dll` | Alpha-boundary clipping with `clipAlphaRect`. | Other exports are unavailable. |
| `textrender.dll` | `TextRenderBase`, styled text layout, ruby, width measurement and rendering callbacks. | — |
| `addFont.dll` | Registering fonts supplied with the game. | — |
| `psbfile.dll` | PSBFile/PSBValue access and `psb://` image resources. | Does not enable every animation format that uses a PSB container. |

### Movies, animation and sound

| Plug-in | Available features | Restrictions |
| --- | --- | --- |
| `krmovie.dll` | VideoOverlay movie playback. | Playback depends on the movie's supported container and codecs. |
| `AlphaMovie.dll` | AJPM alpha-video playback. | Other alpha-video layouts are not implied. |
| `layerExMovie.dll` | RGB movies rendered into layers, looping and playback status. | Side-by-side alpha and resized-image modes are unavailable. |
| `motionplayer.dll`, `motionplayer_nod3d.dll` | M2 motion resources, timeline playback and layer attachment. | Single-timeline playback; advanced masks, actions and physics are unavailable. |
| `wuvorbis.dll` | Vorbis playback and seeking. | Native gain, ReplayGain and PCM-format command-line overrides are unavailable. |
| `kropus.dll` | Opus playback and seeking. | Native gain and PCM-format command-line overrides are unavailable. |

### Transitions and windows

| Plug-in | Available features | Restrictions |
| --- | --- | --- |
| `menu.dll` | Menu bars and menu items. | Synchronous modal popup calls are unavailable. |
| `extrans.dll`, `extNagano.dll` | `crossfade` and `universal` rule-image transitions. | Other named transition methods are unavailable. |
| `kaichoTrans.dll` | `blur` and `dim` transitions. | — |
| `windowEx.dll` | Window geometry, game-window cursor-position queries, constants and script helpers. | Win32 message hooks and HWND-only window actions are unavailable. |

## Interfaces without usable plug-in functions

| Plug-in | Behaviour |
| --- | --- |
| `msdfrender.dll` | Loading is recognized, but SDF glyph-generation functions are unavailable. Scripts can use their bitmap-atlas fallback. |
| `layerStwCopy.dll` | `stitchWrappedCopy` is unavailable. |
| `k2compat.dll` | TouchMouse and ModelessOwnerWindow functions are unavailable. |
| `win32dialog.dll` | Native Win32 dialog windows are unavailable. |

The following plug-in interfaces are unavailable: `DrawDeviceD2D.dll`,
`SteamDrawDevice.dll`, `GlitchEffect.dll`, `PackinOne.dll`, `dmmcloud.dll`,
`gamepad.dll`, `getLangName.dll`, `getSample.dll`, `httprequest.dll`,
`kagexopt.dll`, `krkrsteam.dll`, `lzfs.dll`, `psd.dll`, `wfBasicEffect.dll`,
`wfTypicalDSP.dll`, `win32ole.dll`, `wumultitrack.dll`, `wuopus.dll`,
`wvdecoder.dll` and `yuzuex.dll`.

An unavailable plug-in function produces an error naming the interface or method.
A DLL being present in the game folder does not add that functionality.

## Launch options

The player uses the game's normal startup scripts and disables the optional
Windows developer console by default. You do not need to extract XP3 archives
or write a bootstrap project.

Fonts, save locations, window behaviour and SMB/NFS game paths are described in
the [desktop guide](DESKTOP_API.md). See [all supported engines](NATIVE_ADAPTERS.md)
for other game formats.
