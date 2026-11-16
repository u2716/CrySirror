#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Write the showcase's self-authored assets: a unit screen quad and the logo."""
import json, shutil, struct, pathlib

here = pathlib.Path(__file__).resolve().parent
out = here / "assets"
out.mkdir(exist_ok=True)

# Unit quad in XY facing +Z; UV origin at the top-left (image row 0 at the top).
pos = [-0.5, -0.5, 0, 0.5, -0.5, 0, 0.5, 0.5, 0, -0.5, 0.5, 0]
nrm = [0, 0, 1] * 4
uv = [0, 1, 1, 1, 1, 0, 0, 0]
idx = [0, 1, 2, 0, 2, 3]
blob = struct.pack("<12f", *pos) + struct.pack("<12f", *nrm) + struct.pack("<8f", *uv) + struct.pack("<6I", *idx)
gltf = {
    "asset": {"version": "2.0", "generator": "CrySirror showcase make_assets.py"},
    "buffers": [{"byteLength": len(blob)}],
    "bufferViews": [
        {"buffer": 0, "byteOffset": 0, "byteLength": 48},
        {"buffer": 0, "byteOffset": 48, "byteLength": 48},
        {"buffer": 0, "byteOffset": 96, "byteLength": 32},
        {"buffer": 0, "byteOffset": 128, "byteLength": 24},
    ],
    "accessors": [
        {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3", "min": [-0.5, -0.5, 0], "max": [0.5, 0.5, 0]},
        {"bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC3"},
        {"bufferView": 2, "componentType": 5126, "count": 4, "type": "VEC2"},
        {"bufferView": 3, "componentType": 5125, "count": 6, "type": "SCALAR"},
    ],
    "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2}, "indices": 3}]}],
    "nodes": [{"mesh": 0}],
    "scenes": [{"nodes": [0]}],
    "scene": 0,
}
js = json.dumps(gltf, separators=(",", ":")).encode()
js += b" " * (-len(js) % 4)
blob += b"\0" * (-len(blob) % 4)
glb = b"glTF" + struct.pack("<II", 2, 12 + 8 + len(js) + 8 + len(blob))
glb += struct.pack("<I", len(js)) + b"JSON" + js + struct.pack("<I", len(blob)) + b"BIN\0" + blob
(out / "screen.glb").write_bytes(glb)
shutil.copyfile(here.parent.parent / "icon" / "png" / "crysirror-1024.png", out / "logo.png")
print("wrote", out / "screen.glb", out / "logo.png")
