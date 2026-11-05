# SPDX-License-Identifier: MPL-2.0
"""Observed PS3 character hierarchy and bind-space placement, not animation VM.

This frontend exports ordinary engine geometry. Native joint-local rigid pieces
use their declared MG joint; authored bind-space pieces retain bind positions.
No external model, fitted transform, guessed bone or nearest-bone assignment is
used. Native animation clips, compressed skin weights and cloth remain separate.
"""
import math
from gow3_scene import Reader, check

IDENTITY = [1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.]


def multiply(a, b):
    return [sum(a[k*4+i]*b[j*4+k] for k in range(4)) for j in range(4) for i in range(4)]


def inverse(a):
    rows = [[a[j*4+i] for j in range(4)] + [float(i == j) for j in range(4)] for i in range(4)]
    for col in range(4):
        pivot = max(range(col,4), key=lambda i: abs(rows[i][col]))
        check(abs(rows[pivot][col]) > 1e-12, 'singular native bind matrix')
        rows[col], rows[pivot] = rows[pivot], rows[col]
        scale = rows[col][col]
        rows[col] = [x/scale for x in rows[col]]
        for row in range(4):
            if row != col:
                scale = rows[row][col]
                rows[row] = [x-scale*y for x,y in zip(rows[row],rows[col])]
    return [rows[i][4+j] for j in range(4) for i in range(4)]


def matrix(reader, at):
    result = list(reader.get('>16f',at))
    check(all(math.isfinite(v) and abs(v) < 1e7 for v in result), 'invalid native joint matrix')
    check(max(abs(result[i]) for i in (3,7,11)) < 1e-5 and abs(result[15]-1) < 1e-5,
          'non-affine native joint matrix')
    return result


def decode_skeleton(payload):
    r = Reader(payload)
    check(r.u32(0) == 0x10001, 'unknown character skeleton version')
    count = r.u32(4)
    check(0 < count <= 1024 and r.u32(8) == 0 and r.u32(12) == 3,
          'unknown character skeleton layout/count')
    matrices = r.rel(16)
    names = 20 + count*8
    check(names + count*56 <= matrices and matrices + (count*2+1)*64 == len(payload),
          'character skeleton table ranges')
    bones = []
    unique = set()
    for i in range(count):
        flags,child,sibling,parent = r.get('>4H',20+i*8)
        check(flags in (0x8000,0x8020), 'unknown skeleton node flags')
        check(parent == 0xffff or parent < i, 'native skeleton parent order/cycle')
        check(all(v == 0xffff or v < count for v in (child,sibling)), 'native skeleton hierarchy index')
        name = r.raw(names+i*56,56).split(b'\0')[0].decode('ascii',errors='strict')
        check(name and name not in unique, 'empty/duplicate native joint name')
        unique.add(name)
        local = matrix(r,matrices+(i+1)*64)
        inv_bind = matrix(r,matrices+(count+1+i)*64)
        # Deform joints carry inverse world bind matrices. Non-deform attachment
        # nodes carry an identity inverse and their local placement in hierarchy.
        if flags == 0x8020:
            world = inverse(inv_bind)
        else:
            check(max(abs(a-b) for a,b in zip(inv_bind,IDENTITY)) < 1e-5,
                  'unclassified attachment inverse-bind policy')
            world = multiply(bones[parent]['bind_world'],local) if parent != 0xffff else local
        bones.append({'name':name,'parent':None if parent == 0xffff else parent,
                      'child':None if child == 0xffff else child,
                      'sibling':None if sibling == 0xffff else sibling,
                      'deform':flags == 0x8020,'native_local':local,
                      'inverse_bind':inv_bind,'bind_world':world})
    check(sum(b['parent'] is None for b in bones) == 1, 'native skeleton root count')
    for i,bone in enumerate(bones):
        if bone['child'] is not None:
            check(bones[bone['child']]['parent'] == i, 'native skeleton child/parent mismatch')
        if bone['sibling'] is not None:
            check(bones[bone['sibling']]['parent'] == bone['parent'], 'native skeleton sibling/parent mismatch')
    return {'schema':1,'bones':bones,'animation_exported':False,'skin_weights_exported':False}


def bind_pose(data, info):
    records = [r for r in info['records'] if r['kind'] == '0x0001001f' and not r['name']]
    check(len(records) == 1, 'character needs one unambiguous skeleton resource')
    record = records[0]
    return decode_skeleton(data[record['offset']+64:record['offset']+64+record['size']])


def place_parts(parts, skeleton):
    def normalized(v):
        length = math.sqrt(sum(x*x for x in v))
        check(length > 1e-12, 'zero transformed character normal/tangent')
        return [x/length for x in v]

    for part in parts:
        joint = part['draw_joint']
        check(0 <= joint < len(skeleton['bones']), 'MG joint outside native skeleton')
        bone = skeleton['bones'][joint]
        transform = multiply(bone['bind_world'],bone['inverse_bind'])
        inv = inverse(transform)
        determinant = (transform[0]*(transform[5]*transform[10]-transform[9]*transform[6])
                       - transform[4]*(transform[1]*transform[10]-transform[9]*transform[2])
                       + transform[8]*(transform[1]*transform[6]-transform[5]*transform[2]))
        part['positions'] = [[sum(p[k]*transform[k*4+i] for k in range(3))+transform[12+i]
                              for i in range(3)] for p in part['positions']]
        part['normals'] = [normalized([sum(n[k]*inv[i*4+k] for k in range(3)) for i in range(3)])
                           for n in part['normals']]
        part['tangents'] = [None if t is None else normalized([
            sum(t[k]*transform[k*4+i] for k in range(3)) for i in range(3)])
            + [t[3]*(-1 if determinant < 0 else 1)] for t in part['tangents']]
        if determinant < 0:
            part['indices'] = [value for at in range(0,len(part['indices']),3)
                               for value in (part['indices'][at],part['indices'][at+2],part['indices'][at+1])]
        part['attachment'] = {'joint':joint,'name':bone['name'],'matrix':transform}
