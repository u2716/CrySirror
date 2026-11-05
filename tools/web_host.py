#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Build or serve the local CrySirror browser host. Release artifacts only."""
from __future__ import annotations
import argparse
import functools
import http.server
import io
import json
import math
import os
from pathlib import Path
import shutil
import struct
import subprocess
import re
import wave
import zlib

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / 'crates/crysirror-web/web'
DEFAULT_OUTPUT = ROOT / 'target/web'


def demo_files():
    """Generate original test assets; never import local game data or fonts."""
    def chunk(kind, payload):
        return struct.pack('>I', len(payload)) + kind + payload + struct.pack('>I', zlib.crc32(kind + payload))
    width, height = 32, 32
    pixels = bytearray()
    for y in range(height):
        pixels.append(0)
        for x in range(width):
            pixels.extend((57, 196, 162, 255) if (x // 8 + y // 8) % 2 else (32, 81, 119, 255))
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(pixels)) + chunk(b'IEND', b'')
    audio = io.BytesIO()
    with wave.open(audio, 'wb') as out:
        out.setparams((1, 2, 48000, 0, 'NONE', 'not compressed'))
        out.writeframes(b''.join(struct.pack('<h', int(2200 * math.sin(2 * math.pi * 440 * i / 48000) * min(i / 240, (12000 - i) / 240, 1))) for i in range(12000)))
    sprite = lambda x, tint: {'op': 'sprite', 'id': 1, 'asset': 'card.png', 'rect': [x, 90, 180, 180], 'z': 1, 'tint': tint}
    project = {'schema': 2, 'name': 'CrySirror Web demo', 'content_revision': '1', 'canvas': [640, 360], 'background': [16, 25, 39, 255], 'assets': 'assets', 'clients': [
        {'name': 'demo', 'adapter': 'crysirror-script-v1', 'game': 'web-demo', 'program': [
            sprite(70, [255, 255, 255, 255]), {'op': 'wait_input'},
            {'op': 'play', 'channel': 1, 'asset': 'tone.wav'}, sprite(390, [255, 210, 140, 255]), {'op': 'wait_input'},
            {'op': 'stop', 'channel': 1}, sprite(230, [190, 255, 220, 255]), {'op': 'halt'},
        ]}]}
    cube = {'schema': 2, 'name': 'CrySirror PBR demo', 'content_revision': '1', 'canvas': [800, 450], 'assets': 'assets',
        'scene3d': {'camera': {'eye': [5, 3.2, 7], 'target': [0, 0, 0], 'up': [0, 1, 0], 'fov_y_degrees': 45, 'near': 0.1, 'far': 100}, 'meshes': []},
        'clients': [{'name': 'pbr', 'adapter': 'crysirror-script-v1', 'game': 'web-pbr', 'program': [{'op': 'halt'}], 'systems': {
            'environment': {'temporal_aa': False, 'ambient_occlusion': 0, 'bloom': 0},
            'entities': [
                {'id': 1, 'name': 'Teal cube', 'geometry': {'kind': 'cube'}, 'transform': {'translation': [-1.2, 0, 0]}, 'angular_velocity': [18, 35, 0], 'material': {'base_color_factor': [0.05, 0.65, 0.4, 1], 'metallic_factor': 0.25, 'roughness_factor': 0.3}},
                {'id': 2, 'name': 'Copper cube', 'geometry': {'kind': 'cube'}, 'transform': {'translation': [1.2, 0, 0]}, 'angular_velocity': [0, -28, 12], 'material': {'base_color_factor': [0.8, 0.3, 0.08, 1], 'metallic_factor': 0.8, 'roughness_factor': 0.22}},
                {'id': 3, 'name': 'Ground', 'geometry': {'kind': 'cube'}, 'transform': {'translation': [0, -1.3, 0], 'scale': [6, 0.15, 6]}, 'material': {'base_color_factor': [0.3, 0.32, 0.35, 1], 'metallic_factor': 0, 'roughness_factor': 0.8}},
            ]}}]}
    return {'demo/pbr.json': (json.dumps(cube, indent=2) + '\n').encode(), 'demo/project.json': (json.dumps(project, ensure_ascii=False, indent=2) + '\n').encode(), 'demo/assets/card.png': png, 'demo/assets/tone.wav': audio.getvalue()}


def stage(output):
    output.mkdir(parents=True, exist_ok=True)
    for source in WEB.iterdir():
        if source.is_file():
            shutil.copy2(source, output / source.name)
    for name, content in demo_files().items():
        path = output / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)


def build(output, bindgen=None, threaded=True):
    lock = (ROOT / 'Cargo.lock').read_text()
    expected = re.search(r'(?m)^name = "wasm-bindgen"\nversion = "([^"]+)"', lock).group(1)
    if bindgen is None:
        cached = sorted((ROOT / 'target/wasm-tools').glob(f'wasm-bindgen-{expected}-*/wasm-bindgen'))
        bindgen = str(cached[0]) if cached else 'wasm-bindgen'
    version = subprocess.check_output([bindgen, '--version'], text=True).strip()
    if version != f'wasm-bindgen {expected}':
        raise SystemExit(f'Need wasm-bindgen {expected}; found {version}. Supply --bindgen PATH or install the matching wasm-bindgen-cli release.')
    command = ['cargo']
    env = dict(os.environ)
    if threaded:
        command += ['+nightly']
        env['RUSTFLAGS'] = env.get('RUSTFLAGS', '') + ' -C target-feature=+atomics,+bulk-memory,+mutable-globals'
    command += ['rustc' if threaded else 'build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'crysirror-web']
    if threaded:
        command += ['-Z', 'build-std=std,panic_abort', '--features', 'threads', '--', '-C', 'link-arg=--shared-memory', '-C', 'link-arg=--import-memory', '-C', 'link-arg=--max-memory=2147483648', '-C', 'link-arg=--export=__wasm_init_tls', '-C', 'link-arg=--export=__tls_size', '-C', 'link-arg=--export=__tls_align', '-C', 'link-arg=--export=__tls_base']
    subprocess.run(command, cwd=ROOT, check=True, env=env)
    stage(output)
    subprocess.run([bindgen, str(ROOT / 'target/wasm32-unknown-unknown/release/crysirror_web.wasm'), '--target', 'web', '--out-dir', str(output / 'pkg')], cwd=ROOT, check=True)
    print(f'Browser host ready: {output}')


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, '.wasm': 'application/wasm', '.js': 'text/javascript'}
    def end_headers(self):
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        self.send_header('Cache-Control', 'no-store')
        self.send_header('X-Content-Type-Options', 'nosniff')
        super().end_headers()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['build', 'serve'])
    parser.add_argument('--output', type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument('--bindgen')
    parser.add_argument('--port', type=int, default=8080)
    parser.add_argument('--single-threaded', action='store_true', help='Build the cooperative fallback instead of shared-memory CPU workers')
    args = parser.parse_args()
    output = args.output.resolve()
    if args.action == 'build':
        build(output, args.bindgen, threaded=not args.single_threaded)
    else:
        if not (output / 'pkg/crysirror_web_bg.wasm').is_file():
            parser.error('Build first: python3 tools/web_host.py build')
        print(f'CrySirror Web: http://127.0.0.1:{args.port}', flush=True)
        with http.server.ThreadingHTTPServer(('127.0.0.1', args.port), functools.partial(Handler, directory=str(output))) as server:
            server.serve_forever()


if __name__ == '__main__':
    main()
