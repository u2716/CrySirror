# KiriKiri 2 / KiriKiri Z / KAG3

[TJS2 opcodes, native APIs and KAG control tags](native-api/krkr.md).

```sh
./target/release/crysirror play "/path/to/game"
```

Keep XP3 archives, patches, the game's executable and companion files in their
original layout. CrySirror reads them in place and starts the original TJS/KAG
scripts. Supported resource protection includes HXV4 and supported cxdec-family
filters; additional game-specific filters may require their accompanying files.

## XP3 resource filters

MadoCrypt uses protected TLG or KBAD100 resources in the archive or in a
sibling `fgimage.xp3`, including `arc/fgimage.xp3`. CrySirror requires two
matching image headers and whole-entry checksums, then checks encrypted bytes
from the target archive with the same seed. Mounted resources keep their
indexed filenames. This path runs entirely in the player.

For a Nana `dls:` archive, installed-game detection can use the game's
i386 executable and TPM modules. It recognizes references to a contiguous
six-word name state in initialized PE data, including supported embedded
modules. The complete inflated directory must match the archive's name
tokens and Adler-32 values. The existing CXDEC module reader supplies the
Cabbage content parameters and control table. Every protected resource is
checksum-verified before it is returned. This layout can be read directly
from the original files. Retail Loca-Love startup remains unverified.

For other layouts with independently recovered parameters, the
[xp3-brute PR #3 profile format](https://github.com/xmoezzz/xp3-brute/pull/3)
is also supported. An ordinary XP3 with a `dls:` descriptor can read
`nana-profile.json` beside the archive or in the parent of its `arc/` folder.
Its `content.control_block_file` names a 4096-byte table relative to the JSON.
An explicit `kirikiri/xp3.nana_profile` selects a different JSON filename
relative to the archive.

## Game features

- TJS scripts and KAG3 scenarios, labels, macros, calls, choices and message waits.
- Layer positioning, scaling, rotation, clipping, blending, image filters and
  custom mouse cursors.
- Text windows, font selection, ruby, typewriter text and the TextRender extension.
- Music, voice, sound effects, Vorbis/Opus audio, voice filters (`WaveSoundBuffer.filters`, including `PhaseVocoder` pitch and time scaling), ordinary movies and AJPM alpha movies.
- Menu bars, layer movies, PSB images and M2 motion playback.
- Crossfade, rule-image transitions, and Kaicho blur/dim transitions.

Plug-in names below describe the interfaces built into CrySirror. They do not
imply every export of a similarly named DLL is available. The corresponding
`.tpm` names use the same built-in interfaces. TPM modules in the game,
`system` and native-bitness `plugin`/`plugin64` directories register automatically
before startup scripts, in filename order.

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
| `layerExBTOA.dll` | `clipAlphaRect`, `copyRightBlueToLeftAlpha`, `copyBottomBlueToTopAlpha`, `fillAlpha`, `overwrapRect`, `copyAlphaToProvince`, `fillByProvince` and `fillToProvince`. | — |
| `shrinkCopy.dll` | `shrinkCopy` area-average reduction and `shrinkCopyFast` integer reduction. | — |
| `layerStwCopy.dll` | `stitchWrappedCopy` wrapped row copies. | — |
| `GlitchEffect.dll` | `glitchCopy` and `doGlitch` block/line displacement with channel splitting. The `glitch` transition (rows switch to the new image as it progresses), `fadeglitch` (one image glitches while fading to or from `color`; `nofade` keeps colours) and `loopglitch` (finishes at its first update without drawing, as natively). | Without `seed`, and in transitions, the pattern differs from run to run, as natively. `fadeglitch` with `fadein` needs a source layer. Transition sources must match the destination size. |
| `PackinOne.dll` | The bundled members listed for the plug-ins above, data-pack and octet storage functions, `Layer.fetchImageSize`, `System.waitForAppLock` and `System.setDpiAwareness`. `SliceLayer` and its loader. `System.commandExecute` runs nothing and reports `status` `"failed"`. | Temporary-file helpers are unavailable. |
| `textrender.dll` | `TextRenderBase`, styled text layout, ruby, width measurement and rendering callbacks. | — |
| `addFont.dll` | Registering fonts supplied with the game. | — |
| `msdfrender.dll` | `makeGlyphSDF`, `makeGlyphMSDF`, `makeBitmapFromProvince`, `operateGlyphToProvince`, `getAscentHeight`, `copyGlyphByColorTable` and `Scripts.makeColorTable`. | None known. |
| `psbfile.dll` | PSBFile/PSBValue access and `psb://` image resources. | Does not enable every animation format that uses a PSB container. |
| `psd.dll` | `PSD` documents: layer pixels, masks, names, types, blend modes, groups, guides, slices and layer comps; `psd://` layer images for `Layer.loadImages`. | Adjustment layers, layer effects and smart objects load as their stored pixels, not re-rendered. |

### Movies, animation and sound

| Plug-in | Available features | Restrictions |
| --- | --- | --- |
| `fftgraph.dll` | Fire and LCD spectrum graphs from playing audio, with peak hold and decay. | Source decoding and rasterization run on shared workers. |
| `krmovie.dll` | VideoOverlay movie playback. | Playback depends on the movie's supported container and codecs. |
| `AlphaMovie.dll` | AJPM alpha-video playback. | Other alpha-video layouts are not implied. |
| `layerExMovie.dll` | RGB movies rendered into layers, looping and playback status. | Side-by-side alpha and resized-image modes are unavailable. |
| `motionplayer.dll`, `motionplayer_nod3d.dll` | M2 motion resources, timeline playback and layer attachment. | Single-timeline playback; advanced masks, actions and physics are unavailable. |
| `wuvorbis.dll` | Vorbis playback and seeking. | Native gain, ReplayGain and PCM-format command-line overrides are unavailable. |
| `wumsadp.dll` | Microsoft ADPCM WAVE playback through the shared decoder. | Other ADPCM formats are not implied. |
| `kropus.dll` | Opus playback and seeking. | Native gain and PCM-format command-line overrides are unavailable. |
| `wvdecoder.dll` | `WaveSoundData.F32` and `WaveSoundData.S16`: `open`, `close`, `getSample`, `setSample`, `channels`, `frequency`, `samples`, `totalTime`, `format`, `foreach`, `getLipSync`, `makeThumbnail`, `save` and `rawdata` (an opaque token for `OscLoop`). | Sounds longer than 16 Mi frames do not open. |
| `wumultitrack.dll` | `.mtrk` (`dir/a|b|ID.mtrk` track files, or a TAB list of `name, path, flag`) and `.mchx` (`channel`, `source`, `track`, `volume` over one multichannel source) multi-track music, each track an independently gained stem of one voice. `setMultiTrackMask(id, tracks, gain, time)` fades tracks linearly; `setDelayMultiTrackMask(id, positions, [tracks, gain, …], time)` waits for the first listed sample position; `rebindMultiTrackID`, `multiTrackOpenCallbacks` (called with id, storage, track count, names and aliases) and the gain-callback functions. | `.mchx` output is mono or stereo, and Opus sources must be mono or stereo. WaveSoundBuffer `filters` cannot be combined with multi-track music. |
| `gamepad.dll` | `GamepadPort`. | No controllers are reported. |
| `wfBasicEffect.dll` | `WaveSoundBuffer.DelayEffect`, `StkFreeVerb`, `GraphicEqualizer`, `GainLimit`, `MathAbs`/`MathConstAdd`/`MathConstMul`, `RingModulation` and `Vocoder` filters with the `Osc*` carrier generators (sine, noise, Blit/BlitSaw/BlitSquare, modulate, mix, filter, const), including the `extend` tail after the sound ends (the voice effects of `voiceeffect.tjs`). | Filters apply when the sound opens; changing a sound's filters after `open` and before `play` is an error. `OscLoop` needs `wvdecoder.dll`. Unseeded `OscNoise` uses a fixed seed. |
| `wfTypicalDSP.dll` | `WaveSoundBuffer.WaveDSPFilter`: RBJ, Butterworth, Chebyshev I/II, Elliptic, Bessel, Legendre and custom pole/zero designs, all four filter structures, `setParams` and `getParamInfo`. | As above; parameter changes while a sound plays take effect at its next `open`, without the gradual 1024-sample change. |

### Transitions and windows

| Plug-in | Available features | Restrictions |
| --- | --- | --- |
| `menu.dll` | Menu bars and menu items. | Synchronous modal popup calls are unavailable. |
| `extrans.dll`, `extNagano.dll` | `crossfade` and `universal` rule-image transitions. | Other named transition methods are unavailable. |
| `kaichoTrans.dll` | `blur` and `dim` transitions. | — |
| `k2compat.dll` | `Window.TouchMouse` forwarding of touch handlers to mouse handlers and a hidden `System.ModelessOwnerWindow`. | The engine itself raises no touch events. |
| `windowEx.dll` | Window geometry, game-window cursor-position queries, constants and script helpers. | Win32 message hooks and HWND-only window actions are unavailable. |

## Interfaces without usable plug-in functions

| Plug-in | Behaviour |
| --- | --- |
| `win32dialog.dll` | `WIN32Dialog` objects can be created; `open` shows nothing and returns 0. |

An unavailable plug-in function produces an error naming the interface or method.
A DLL being present in the game folder does not add that functionality.

## Launch options

The player uses the game's normal startup scripts and disables the optional
Windows developer console by default. You do not need to extract XP3 archives
or write a bootstrap project.

For installed-game launches, the selected game directory is published as virtual
media labelled with the executable's stem. `Storages.searchCD` can find this
label; its returned drive routes back into that scoped game directory. This
mapping does not require an optical drive. Unregistered labels remain absent.

Fonts, save locations, window behaviour and SMB/NFS game paths are described in
the [desktop guide](DESKTOP_API.md). See [all supported engines](NATIVE_ADAPTERS.md)
for other game formats.
