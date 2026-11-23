#!/usr/bin/env python3
"""Rasterize the owner's SVG with librsvg; preserve alpha in all containers.

Requires Python 3 and rsvg-convert (librsvg). No fonts or game data are used.
ICO and ICNS contain the same PNG payloads as the individual size exports.
"""

import hashlib
import json
import shutil
import struct
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SIZES = (16, 20, 22, 24, 32, 40, 48, 64, 96, 128, 192, 256, 384, 512, 768, 1024)
ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
MAC_IMAGES = (
    ("icon_16x16.png", b"icp4", 16),
    ("icon_16x16@2x.png", b"ic11", 32),
    ("icon_32x32.png", b"icp5", 32),
    ("icon_32x32@2x.png", b"ic12", 64),
    ("icon_128x128.png", b"ic07", 128),
    ("icon_128x128@2x.png", b"ic13", 256),
    ("icon_256x256.png", b"ic08", 256),
    ("icon_256x256@2x.png", b"ic14", 512),
    ("icon_512x512.png", b"ic09", 512),
    ("icon_512x512@2x.png", b"ic10", 1024),
)


def main():
    renderer = shutil.which("rsvg-convert")
    if renderer is None:
        raise SystemExit("Install librsvg (rsvg-convert) before regenerating icons.")
    for directory in ("png", "windows", "macos/crysirror.iconset"):
        (ROOT / directory).mkdir(parents=True, exist_ok=True)

    images = {}
    for size in SIZES:
        destination = ROOT / "png" / f"crysirror-{size}.png"
        # No background colour: librsvg's default is a transparent canvas.
        # Render each size from the vector, without resampling another PNG.
        subprocess.run(
            [renderer, "--width", str(size), "--height", str(size),
             "--output", str(destination), str(ROOT / "crysirror.svg")],
            check=True,
        )
        payload = destination.read_bytes()
        if payload[:8] != b"\x89PNG\r\n\x1a\n":
            raise RuntimeError(f"Not a PNG: {destination}")
        width, height, depth, colour = struct.unpack(">IIBB", payload[16:26])
        if (width, height, depth, colour) != (size, size, 8, 6):
            raise RuntimeError(f"Expected {size}x{size} 8-bit RGBA: {destination}")
        images[size] = payload

    # PNG-in-ICO preserves full 8-bit alpha on modern Windows.
    offset = 6 + 16 * len(ICO_SIZES)
    entries, payloads = [], []
    for size in ICO_SIZES:
        payload = images[size]
        entries.append(struct.pack(
            "<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32,
            len(payload), offset,
        ))
        payloads.append(payload)
        offset += len(payload)
    (ROOT / "windows/crysirror.ico").write_bytes(
        struct.pack("<HHH", 0, 1, len(entries)) + b"".join(entries + payloads)
    )

    chunks = []
    for filename, kind, size in MAC_IMAGES:
        payload = images[size]
        (ROOT / "macos/crysirror.iconset" / filename).write_bytes(payload)
        chunks.append(kind + struct.pack(">I", 8 + len(payload)) + payload)
    content = b"".join(chunks)
    (ROOT / "macos/crysirror.icns").write_bytes(
        b"icns" + struct.pack(">I", 8 + len(content)) + content
    )

    generated = sorted(
        path for directory in ("png", "windows", "macos")
        for path in (ROOT / directory).rglob("*") if path.is_file()
    )
    manifest = {
        "source": "crysirror.svg",
        "source_sha256": hashlib.sha256((ROOT / "crysirror.svg").read_bytes()).hexdigest(),
        "renderer": subprocess.check_output([renderer, "--version"], text=True).splitlines()[0],
        "transparent_background": True,
        "files": {
            path.relative_to(ROOT).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in generated
        },
    }
    (ROOT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Generated {len(generated)} icon files from {manifest['source']}.")


if __name__ == "__main__":
    main()
