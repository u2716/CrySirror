# CrySirror icons

`crysirror.svg` is the owner's original Glass Clover SVG, copied without changes.
All raster exports retain its transparent canvas, translucent glass and original
composition. There is no background plate, added shadow or baked checkerboard.

| Use | Asset |
| --- | --- |
| Editable master / Linux scalable icon | `crysirror.svg` |
| General PNG / Linux hicolor sizes | `png/crysirror-{size}.png` |
| Windows application/resource icon | `windows/crysirror.ico` (10 sizes, 16–256 px) |
| macOS app bundle icon | `macos/crysirror.icns` (16–1024 px, including Retina) |
| macOS source iconset | `macos/crysirror.iconset/` |

PNG sizes: 16, 20, 22, 24, 32, 40, 48, 64, 96, 128, 192, 256, 384, 512, 768,
1024. Each size is rendered directly from the SVG as 8-bit RGBA. ICO and ICNS
embed those same PNGs, preserving alpha.

Regenerate locally with Python 3 and librsvg (`rsvg-convert`):

```sh
python3 icon/generate.py
```

`manifest.json` records the source hash, renderer version and export hashes.

For packaging, copy the ICNS to `Contents/Resources/crysirror.icns` and set
`CFBundleIconFile` to `crysirror` in a macOS app's `Info.plist`. Windows resource
scripts can reference the ICO. Linux packages can install the SVG as
`share/icons/hicolor/scalable/apps/crysirror.svg` and PNGs in the corresponding
`hicolor/{size}x{size}/apps/crysirror.png` directories, with `Icon=crysirror` in
their desktop entry. These are prepared assets; this change does not create an
app bundle, Windows executable resource or Linux installer.
