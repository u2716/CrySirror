# SPDX-License-Identifier: MPL-2.0
"""Local fixture closure/integrity tests using authored empty glTF scenes."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
from gow3_fixture import build,copy_model,verify
from local_assets import ROOT,output_root


def project(root):
    assets=root/'assets';assets.mkdir(parents=True)
    (assets/'scene.bin').write_bytes(b'authored buffer')
    (assets/'unused.bin').write_bytes(b'must not be retained')
    (assets/'scene.gltf').write_text(json.dumps({'asset':{'version':'2.0'},
        'buffers':[{'uri':'scene.bin','byteLength':15}]}))
    data={'schema':2,'name':'authored','content_revision':'test','canvas':[1280,720],
          'assets':'assets','scene3d':{'camera':{'eye':[0,1,-5],'target':[0,1,0],
          'up':[0,1,0],'fov_y_degrees':55,'near':.1,'far':2000},'meshes':[]},
          'clients':[{'name':'game','adapter':'crysirror-script-v1','game':'authored',
          'systems':{'entities':[{'id':1,'name':'authored scene',
          'geometry':{'kind':'scene','asset':'scene.gltf'}}]}}]}
    path=root/'project.json';path.write_text(json.dumps(data));return path


class FixtureTests(unittest.TestCase):
    def test_only_scene_character_and_native_inputs_are_retained(self):
        with tempfile.TemporaryDirectory()as temporary:
            root=Path(temporary);scene=project(root/'scene');character=project(root/'character')
            native=root/'authored.wad';native.write_bytes(b'authored native input')
            out=root/'fixture';receipt=build(scene,character,{'test.WAD':native},out)
            self.assertTrue(receipt['verified'])
            self.assertFalse(list(out.rglob('unused.bin')))
            data=json.loads((out/'project.json').read_text())
            models=[e['geometry']['asset']for e in data['clients'][0]['systems']['entities']if e.get('geometry')]
            self.assertEqual(models,['scene/scene.gltf','character/scene.gltf'])
            (out/'assets/character/scene.bin').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError,'fingerprint'):verify(out)

    def test_dependency_escape_is_rejected(self):
        with tempfile.TemporaryDirectory()as temporary:
            root=Path(temporary);path=project(root/'scene')
            (root/'outside.bin').write_bytes(b'private')
            (path.parent/'assets/scene.gltf').write_text(json.dumps({'asset':{'version':'2.0'},
                'buffers':[{'uri':'../../outside.bin','byteLength':7}]}))
            with self.assertRaisesRegex(ValueError,'escapes'):copy_model(path,root/'destination')

    def test_local_outputs_stay_in_ignored_work_or_external_paths(self):
        self.assertEqual(output_root(ROOT/'local/work/authored-test'),ROOT/'local/work/authored-test')
        with self.assertRaisesRegex(ValueError,'ignored'):output_root(ROOT/'examples/game-data')


if __name__=='__main__':unittest.main()
