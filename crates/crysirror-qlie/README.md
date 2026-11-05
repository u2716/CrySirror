# QLIE

The native `.s` interpreter calls CrySirror's resource, Layer, Canvas, UI,
image-processing, Media and Audio services. The desktop host owns the window,
input and output devices. No original engine executable or DLL is loaded.

```sh
./target/release/crysirror play ~/Downloads/game-QLIE-5
```

Detection reads the original `GameData/*.pack` files, mounts them in data/patch
order and reads the canvas size from `System/Etc/Game.txt`. Installation sources
do not need to be unpacked or converted into a project file first.

Resource support includes FilePack 3.1 names, embedded key files and data ciphers;
ABMP nested resources; DPNG tiled images; old compact and current animation keys;
native external value banks, image replacement;
and archive-backed audio/video. Image decoding and media preparation run on the
shared engine workers. Native collision tracks become UI hit rectangles, and
named button motions respond to engine-owned interaction state.

The interpreter retains native variables, arrays, expressions, include scope,
branches, loops, calls and waits. Reader replacement and ET/NT commands execute
inside the same VM. Formatted messages use shared cell layout, font resources,
positioned glyphs and engine reveal clocks. Native `*MesSurface` bindings place
those glyphs within the original ABMP animation hierarchy. Ruby, width conversion,
line-breaking rules and formatted numbers use the shared text crate.

`Layers.setParent` preserves an owned node's paint and clocks while changing its
parent. `Layers.advanceTime` advances one client's automatic layer motion, text
reveal and transitions, independently of other clients and external producers.
Both APIs also work in authored CrySirror scenes without this adapter.
Camera oscillation uses shared `ScalarOscillator` signals and versioned Layer
external-motion producers. Constant-colour screen filtering uses the shared
`RgbLookup::screen_constant_byte256` arithmetic and image workers. Quote-aware
native data splitting preserves nested filter and generator parameters.

Original save serialization and temporary checkpoints, script threads,
additional transition layouts, generator profiles and some native object
operations still require implementation. A failed temporary checkpoint capture
reports its missing capability and returns the native failure value zero;
it does not create a save. Other unknown commands fail at their script location.
ABMP dual-image rule drawing uses the shared affine mask filter; native integer
rounding and softness have not been verified for pixel parity. Script-level
rule-image fades use the same shared mask and motion services. Single-capture
animated transitions bind retained Canvas paint to ABMP image slots and run on
the shared motion clock. Dual-capture and mosaic scene transitions still require
implementation. `Canvases.apply` also exposes whole-target presentation for
global masks across storage pages, independently of this adapter.

Format and syntax references:
[QLIE analysis](https://habr.com/en/articles/426431/),
[QlieScript.cs](https://github.com/arcusmaximus/VNTranslationTools/blob/main/VNTextPatch.Shared/Scripts/QlieScript.cs),
[qlie-unpack](https://github.com/rafi441/qlie-unpack),
[QlieKit](https://github.com/Inori/FuckGalEngine/tree/master/QLIE).
