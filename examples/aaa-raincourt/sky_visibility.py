"""Bake static sky accessibility into the PBR occlusion term of the masonry."""
import math

import numpy as np
from PIL import Image
from scipy.ndimage import maximum_filter

from geometry import Geometry, unit


def surface_samples(meshes, pitch):
    # Barycentric sampling is shared by all of the orthographic sky depth maps.
    points = []
    for mesh in meshes:
        for pos, _, _, indices in mesh.packed().values():
            tris = pos[indices.reshape(-1,3)]
            edges = np.roll(tris,-1,axis=1)-tris
            divisions = np.maximum(1,np.ceil(np.linalg.norm(edges,axis=2).max(axis=1)/pitch).astype(int))
            for n in np.unique(divisions):
                triangles = tris[divisions == n]
                i,j = np.triu_indices(n+1)
                u,v = i/n,(j-i)/n
                for start in range(0,len(triangles),256):
                    a,b,c = triangles[start:start+256].transpose(1,0,2)
                    p = a[:,None] + u[None,:,None]*(b-a)[:,None] + v[None,:,None]*(c-a)[:,None]
                    points.append(p.reshape(-1,3).astype('f4'))
    return np.vstack(points)


def accessible(meshes, centers, normals):
    # The scanned roughness/metal and the direct sun stay independent of this bake.
    pitch = .30
    samples = surface_samples(meshes,.17)
    lit = np.zeros(len(centers))
    total = np.zeros_like(lit)
    for i in range(20):
        y=(i+.5)/20
        r=math.sqrt(1-y*y)
        sun=unit([r*math.cos(i*2.39996),y,r*math.sin(i*2.39996)])
        weight=np.maximum(0,normals@sun)
        right=unit(np.cross(sun,[0,1,0]))
        up=np.cross(sun,right)
        basis=np.stack([right,up],axis=1)
        uv=samples@basis/pitch
        origin=np.floor(uv.min(axis=0)).astype(int)-2
        xy=np.floor(uv).astype(int)-origin
        size=xy.max(axis=0)+3
        depth=np.full(size,-np.inf,dtype='f4')
        np.maximum.at(depth,(xy[:,0],xy[:,1]),samples@sun)
        # Interpolate holes in the sampled surface. A normal offset handles self-shadow.
        depth=maximum_filter(depth, size=3)
        test=centers+normals*.20+sun*.28
        ij=np.clip(np.floor(test@basis/pitch).astype(int)-origin,0,size-1)
        lit+=weight*(test@sun+.20>=depth[ij[:,0],ij[:,1]])
        total+=weight
    return np.divide(lit,total,out=np.zeros_like(lit),where=total>1e-6)


def bake_masonry(ruins, blockers, materials, asset_dir):
    faces=[(name,row) for name,rows in ruins.groups.items() for row in rows]
    centers=np.array([p.mean(axis=0) for _,(p,n,uv,idx) in faces])
    normals=unit(np.array([n.mean(axis=0) for _,(p,n,uv,idx) in faces]))
    visibility=accessible(blockers,centers,normals)
    # Diffuse ground bounce keeps shaded stone from becoming a black cavity.
    factors=np.array([.40,.60,.80,1.])
    bins=np.abs((.40+.60*visibility[:,None])-factors).argmin(axis=1)
    baked=Geometry()
    written=set()
    for (name,row),index in zip(faces,bins):
        tag=name
        if index < len(factors)-1:
            tag=f"{name} sky {index}"
            if tag not in materials:
                mat=dict(materials[name])
                source=mat["arm"]
                filename=f"sky-{index}-{source or 'plain'}.png"
                tile=asset_dir/filename
                if filename not in written:
                    if source:
                        with Image.open(asset_dir/source) as im:
                            im.getchannel("R").point(lambda x:round(x*factors[index])).save(tile)
                    else:
                        Image.new("L",(1,1),round(255*factors[index])).save(tile)
                    written.add(filename)
                mat["occlusion"]=filename
                materials[tag]=mat
        baked.add(tag,*row)
    return baked
