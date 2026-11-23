# SPDX-License-Identifier: MPL-2.0
"""Authored hierarchy/attachment fixtures, no game bytes."""
import struct
import sys
import unittest
from pathlib import Path

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
from gow3_character import IDENTITY,decode_skeleton,place_parts
from gow3_scene import WadError


def skeleton():
    count=3;matrices=224
    b=bytearray(matrices+(count*2+1)*64)
    struct.pack_into('>4Ii',b,0,0x10001,count,0,3,matrices-16)
    for i,(flags,child,parent,name)in enumerate([
        (0x8000,1,0xffff,'root'),(0x8020,2,0,'deform'),(0x8000,0xffff,1,'rigid')]):
        struct.pack_into('>4H',b,20+i*8,flags,child,0xffff,parent)
        at=20+count*8+i*56;b[at:at+len(name)]=name.encode()
    for i in range(count*2+1):struct.pack_into('>16f',b,matrices+i*64,*IDENTITY)
    struct.pack_into('>f',b,matrices+2*64+13*4,2)
    struct.pack_into('>f',b,matrices+3*64+12*4,.5)
    struct.pack_into('>f',b,matrices+5*64+13*4,-2)
    return b


class CharacterTests(unittest.TestCase):
    def test_native_hierarchy_places_rigid_and_preserves_bind_space(self):
        pose=decode_skeleton(skeleton())
        self.assertEqual(pose['bones'][1]['bind_world'][13],2)
        parts=[dict(draw_joint=j,positions=[[0,0,0]],normals=[[0,1,0]],
                    tangents=[[1,0,0,1]],indices=[0,0,0])for j in (1,2)]
        place_parts(parts,pose)
        self.assertEqual(parts[0]['positions'],[[0,0,0]])
        self.assertEqual(parts[1]['positions'],[[.5,2,0]])
        self.assertEqual(parts[1]['normals'],[[0,1,0]])

    def test_corrupt_hierarchy_and_singular_inverse_fail(self):
        b=skeleton();struct.pack_into('>H',b,20+8+6,2)
        with self.assertRaisesRegex(WadError,'parent'):decode_skeleton(b)
        b=skeleton();struct.pack_into('>f',b,224+5*64,0)
        with self.assertRaisesRegex(WadError,'singular'):decode_skeleton(b)
        with self.assertRaises(WadError):decode_skeleton(skeleton()[:-1])

    def test_unknown_attachment_joint_fails(self):
        with self.assertRaisesRegex(WadError,'outside'):
            place_parts([{'draw_joint':3}],decode_skeleton(skeleton()))


if __name__=='__main__':unittest.main()
