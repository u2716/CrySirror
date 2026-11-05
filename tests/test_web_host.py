# SPDX-License-Identifier: MPL-2.0
import importlib.util
import io
import json
from pathlib import Path
import struct
import tempfile
import unittest
import wave
import zlib

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('web_host', ROOT / 'tools/web_host.py')
web_host = importlib.util.module_from_spec(spec)
spec.loader.exec_module(web_host)


class WebHostTests(unittest.TestCase):
    def test_demo_contains_original_decodable_assets_and_sequential_input_waits(self):
        files = web_host.demo_files()
        project = json.loads(files['demo/project.json'])
        self.assertEqual(project['canvas'], [640, 360])
        self.assertEqual(project['clients'][0]['adapter'], 'crysirror-script-v1')
        self.assertEqual(sum(a['op'] == 'wait_input' for a in project['clients'][0]['program']), 2)
        png = files['demo/assets/card.png']
        self.assertEqual(png[:8], b'\x89PNG\r\n\x1a\n')
        offset = 8
        while offset < len(png):
            length = struct.unpack('>I', png[offset:offset + 4])[0]
            kind, payload = png[offset + 4:offset + 8], png[offset + 8:offset + 8 + length]
            expected = struct.unpack('>I', png[offset + 8 + length:offset + 12 + length])[0]
            self.assertEqual(zlib.crc32(kind + payload), expected)
            if kind == b'IDAT':
                self.assertEqual(len(zlib.decompress(payload)), 32 * (32 * 4 + 1))
            offset += 12 + length
        with wave.open(io.BytesIO(files['demo/assets/tone.wav']), 'rb') as audio:
            self.assertEqual(audio.getframerate(), 48000)
            self.assertEqual(audio.getnframes(), 12000)
            self.assertEqual(audio.getsampwidth(), 2)

    def test_staging_does_not_copy_game_data_or_replace_existing_bindings(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            pkg = out / 'pkg'
            pkg.mkdir()
            (pkg / 'preserve').write_text('existing binding')
            web_host.stage(out)
            self.assertEqual((pkg / 'preserve').read_text(), 'existing binding')
            self.assertTrue((out / 'index.html').is_file())
            self.assertTrue((out / 'worker.js').is_file())
            self.assertTrue((out / 'demo/assets/card.png').is_file())
            self.assertFalse(any(p.suffix in {'.ttf', '.xp3', '.pck', '.exe'} for p in out.rglob('*')))
