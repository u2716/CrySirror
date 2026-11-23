"""Small indexed glTF builder for procedurally authored and batched scene meshes."""
from collections import defaultdict
import json
from pathlib import Path
import numpy as np


def unit(v):
    v = np.asarray(v, dtype=np.float32)
    return v / np.maximum(np.linalg.norm(v, axis=-1, keepdims=True), 1e-9)


def yaw(degrees):
    a = np.radians(degrees)
    return np.array([[np.cos(a), 0, np.sin(a)], [0, 1, 0], [-np.sin(a), 0, np.cos(a)]])


class Geometry:
    def __init__(self):
        self.groups = defaultdict(list)

    def add(self, material, vertices, normals, uv, indices):
        self.groups[material].append(tuple(np.asarray(x) for x in (vertices, normals, uv, indices)))

    def face(self, material, points, scale=1, uv=None):
        p = np.array(points)
        n = unit(np.cross(p[1] - p[0], p[2] - p[0]))
        if uv is None:
            axes = np.delete(np.arange(3), np.argmax(abs(n)))
            uv = p[:, axes] / scale
        self.add(material, p, np.tile(n, (len(p), 1)), uv,
                 [i for end in range(2, len(p)) for i in (0, end-1, end)])

    def merge(self, other, point, size=1, angle=0):
        transform = yaw(angle) @ np.diag(np.broadcast_to(size, 3))
        normal = np.linalg.inv(transform).T
        for mat, (p, n, uv, idx) in other.packed().items():
            self.add(mat, p @ transform.T + point, unit(n @ normal.T), uv, idx)

    def packed(self):
        result = {}
        for mat, rows in self.groups.items():
            p, n, uv, indices = zip(*rows)
            starts = np.cumsum([0] + [len(x) for x in p[:-1]])
            result[mat] = (np.concatenate(p), np.concatenate(n), np.concatenate(uv),
                           np.concatenate([i.reshape(-1) + offset for i, offset in zip(indices, starts)]))
        return result


def material(color=(1,1,1), rough=.82, base=None, normal=None, arm=None, metal=0, mask=False):
    return dict(color=color, rough=rough, base=base, normal=normal, arm=arm, metal=metal, mask=mask)


def write_model(path, geometry, materials, bend=None, translation_clip=None):
    doc = dict(asset=dict(version="2.0", generator="CrySirror Raincourt"), scene=0,
               scenes=[dict(nodes=[0])], nodes=[dict(name=path.stem, mesh=0)],
               meshes=[dict(primitives=[])], materials=[], accessors=[], bufferViews=[])
    binary = bytearray()
    images = []
    triangles = 0

    def stream(values, dtype, kind, bounds=False):
        array = np.ascontiguousarray(values, dtype=dtype)
        binary.extend(bytes((-len(binary)) % 4))
        view = len(doc["bufferViews"])
        doc["bufferViews"].append(dict(buffer=0, byteOffset=len(binary), byteLength=array.nbytes))
        binary.extend(array.tobytes())
        accessor = dict(bufferView=view, componentType={"<f4":5126,"<u4":5125,"<u2":5123}[dtype],
                        count=len(array), type=kind)
        if bounds:
            accessor.update(min=array.min(axis=0).reshape(-1).tolist(), max=array.max(axis=0).reshape(-1).tolist())
        doc["accessors"].append(accessor)
        return len(doc["accessors"])-1

    def texture(name):
        if name not in images:
            images.append(name)
        return {"index": images.index(name)}

    for slot, (name, (p, n, uv, idx)) in enumerate(geometry.packed().items()):
        m = materials[name]
        pbr = dict(baseColorFactor=[*m["color"],1], roughnessFactor=m["rough"], metallicFactor=m["metal"])
        entry = dict(name=name, pbrMetallicRoughness=pbr, doubleSided=True)
        if m["base"]: pbr["baseColorTexture"] = texture(m["base"])
        if m["normal"]: entry["normalTexture"] = texture(m["normal"])
        if m["arm"]:
            pbr["metallicRoughnessTexture"] = texture(m["arm"])
            entry["occlusionTexture"] = texture(m["arm"])
        if m.get("occlusion"):
            entry["occlusionTexture"] = texture(m["occlusion"])
        if m["mask"]: entry.update(alphaMode="MASK", alphaCutoff=.42)
        if "surface" in m:
            shader, parameters = m["surface"]
            spec = dict(base_color_factor=pbr["baseColorFactor"], roughness_factor=m["rough"],
                        metallic_factor=m["metal"], double_sided=True,
                        effect=dict(kind="custom", asset=shader, parameters=[parameters]+[[0]*4 for _ in range(3)]))
            if m.get("blend"):
                spec["alpha_mode"]="blend"
                entry["alphaMode"]="BLEND"
            for src, key in [("base","base_color_texture"), ("normal","normal_texture"),
                             ("arm","metallic_roughness_texture"), ("arm","occlusion_texture")]:
                if m[src]:
                    spec[key] = images.index(m[src])
            entry["extras"] = {"crysirror_material": {"schema":1, "material":spec}}
        doc["materials"].append(entry)
        attributes = dict(POSITION=stream(p,"<f4","VEC3",True), NORMAL=stream(n,"<f4","VEC3"),
                          TEXCOORD_0=stream(uv,"<f4","VEC2"))
        if bend:
            # Root held in place; weight ramps smoothly from trunk to outer fronds.
            pivot, span, amplitude = bend
            w = np.clip((p[:,1]-pivot)/span,0,1)
            weights = np.column_stack((1-w,w,np.zeros((len(w),2))))
            attributes.update(JOINTS_0=stream(np.tile([0,1,0,0],(len(w),1)),"<u2","VEC4"),
                              WEIGHTS_0=stream(weights,"<f4","VEC4"))
        if translation_clip:
            # Apply the current as a rigid pose, so water motion is not a static-GI invalidation.
            attributes.update(JOINTS_0=stream(np.zeros((len(p),4)),"<u2","VEC4"),
                              WEIGHTS_0=stream(np.tile([1,0,0,0],(len(p),1)),"<f4","VEC4"))
        doc["meshes"][0]["primitives"].append(dict(attributes=attributes, indices=stream(idx,"<u4","SCALAR"),material=slot))
        triangles += len(idx)//3
    if bend:
        doc["nodes"][0]["skin"]=0
        doc["nodes"] += [dict(name="root",children=[2]),dict(name="crown",translation=[0,pivot,0])]
        doc["scenes"][0]["nodes"].append(1)
        inverse=np.tile(np.eye(4),(2,1,1))
        inverse[1,3,1]=-pivot
        doc["skins"]=[dict(joints=[1,2],inverseBindMatrices=stream(inverse,"<f4","MAT4"))]
        keys=25
        t=np.linspace(0,6,keys)
        a=np.radians(amplitude)*(np.sin(t*np.pi/3)+.23*np.sin(t*np.pi*2/3))
        rot=np.column_stack((np.zeros((keys,2)),np.sin(a/2),np.cos(a/2)))
        doc["animations"]=[dict(name="Breeze",samplers=[dict(input=stream(t,"<f4","SCALAR",True),output=stream(rot,"<f4","VEC4"))],
                                channels=[dict(sampler=0,target=dict(node=2,path="rotation"))])]
    if translation_clip:
        clip_name,times,offsets = translation_clip
        doc["nodes"][0]["skin"] = 0
        doc["nodes"].append(dict(name="current"))
        doc["scenes"][0]["nodes"].append(1)
        doc["skins"] = [dict(joints=[1])]
        doc["animations"] = [dict(name=clip_name,samplers=[dict(input=stream(times,"<f4","SCALAR",True),
            output=stream(offsets,"<f4","VEC3"))],channels=[dict(sampler=0,target=dict(node=1,path="translation"))])]
    if images:
        doc["images"]=[dict(uri=x) for x in images]
        doc["textures"]=[dict(source=i) for i in range(len(images))]
    doc["buffers"]=[dict(uri=path.with_suffix(".bin").name,byteLength=len(binary))]
    path.with_suffix(".bin").write_bytes(binary)
    path.write_text(json.dumps(doc,separators=(",",":")))
    return triangles


def cylinder(g, mat, a, b, r0, r1, sides=8, tex=1):
    a,b = np.array(a),np.array(b)
    axis=unit(b-a)
    u=unit(np.cross(axis,[0,0,1] if abs(axis[2])<.93 else [1,0,0]))
    v=np.cross(axis,u)
    theta=np.linspace(0,2*np.pi,sides+1)
    normals=np.cos(theta)[:,None]*u+np.sin(theta)[:,None]*v
    p=np.vstack((a+r0*normals,b+r1*normals))
    cols=sides+1
    uv=[(i/sides*2*np.pi*r0/tex, level*np.linalg.norm(b-a)/tex) for level in range(2) for i in range(cols)]
    idx=[k for i in range(sides) for k in (i,i+1,i+cols,i+1,i+cols+1,i+cols)]
    g.add(mat,p,np.vstack((normals,normals)),uv,idx)
