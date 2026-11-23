"""Prepare photo-scanned vegetation, wind-rigged understory and geometric palms."""
import json
import math
from pathlib import Path
import numpy as np
from PIL import Image
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components
from geometry import Geometry, cylinder, material, unit

DTYPES={5121:"u1",5123:"<u2",5125:"<u4",5126:"<f4"}
WIDTH={"VEC2":2,"VEC3":3,"VEC4":4,"SCALAR":1}


def source_meshes(assets,name,materials):
    folder=assets/name
    doc=json.loads((folder/"model.gltf").read_text())
    buf=(folder/doc["buffers"][0]["uri"]).read_bytes()
    def read(index):
        acc=doc["accessors"][index]; view=doc["bufferViews"][acc["bufferView"]]
        dt=np.dtype(DTYPES[acc["componentType"]]);w=WIDTH[acc["type"]]
        return np.ndarray((acc["count"],w),dtype=dt,buffer=buf,
            offset=view.get("byteOffset",0)+acc.get("byteOffset",0),strides=(view.get("byteStride",w*dt.itemsize),dt.itemsize)).copy()
    models=[]
    for mesh in doc["meshes"]:
        g=Geometry()
        for primitive in mesh["primitives"]:
            mat=doc["materials"][primitive["material"]]
            tag=mat["name"]
            def filename(item):
                image=doc["textures"][item["index"]]["source"]
                return name+"/"+doc["images"][image]["uri"]
            pbr=mat["pbrMetallicRoughness"];tex=pbr["baseColorTexture"]
            diffuse=filename(tex)
            masked=mat.get("alphaMode") in ("MASK","BLEND")
            if masked:
                mask=folder/("leaves_alpha.png" if name.startswith("island") else "Alpha.png")
                img=Image.open(assets/diffuse).convert("RGBA")
                img.putalpha(Image.open(mask).convert("L").resize(img.size,Image.Resampling.LANCZOS))
                diffuse=name+"/cutout.png"
                img.save(assets/diffuse)
            color=(.80,1.0,.72) if masked else (1,1,1)
            materials[tag]=material(color,base=diffuse,normal=filename(mat["normalTexture"]),
                                    arm=filename(pbr["metallicRoughnessTexture"]),mask=masked)
            attr=primitive["attributes"];pos,norm,uv=[read(attr[k]) for k in ("POSITION","NORMAL","TEXCOORD_0")]
            ids=read(primitive["indices"]).reshape(-1,3)
            transform=tex.get("extensions",{}).get("KHR_texture_transform",{})
            uv=uv*transform.get("scale",[1,1])+transform.get("offset",[0,0])
            if name.startswith("island") and masked:
                # Preserve entire connected leaf silhouettes. Select a deterministic subset
                # and widen the survivors around their own centres for a mid-distance crown.
                edges=coo_matrix((np.ones(ids.size,dtype="u1"),(ids.flatten(),ids[:,[1,2,0]].flatten())),
                                 shape=(len(pos),len(pos))).tocsr()
                count,labels=connected_components(edges,directed=False)
                rng=np.random.default_rng(824)
                keep=rng.random(count)<.065
                ids=ids[keep[labels[ids[:,0]]]]
                totals=np.maximum(1,np.bincount(labels,minlength=count))
                centers=np.column_stack([np.bincount(labels,weights=v,minlength=count)/totals for v in pos.T])
                pos=centers[labels]+(pos-centers[labels])*1.72
            if name in ("root_cluster_01","namaqualand_boulder_03","island_tree_01","namaqualand_cliff_02"):
                cell=.045 if masked else (.065 if name.startswith("island") else .037)
                # Quantize geometry per UV island region; keep representative source UVs.
                keys=np.column_stack((np.floor(pos/cell),np.floor(uv*3))).astype("i4")
                _,first,inverse=np.unique(keys,axis=0,return_index=True,return_inverse=True)
                pos,norm,uv=pos[first],norm[first],uv[first]
                ids=inverse[ids]
                ids=ids[(ids[:,0]!=ids[:,1])&(ids[:,0]!=ids[:,2])&(ids[:,1]!=ids[:,2])]
            used,ids=np.unique(ids,return_inverse=True)
            if len(used):g.add(tag,pos[used],norm[used],uv[used],ids)
        models.append(g)
    return models


def palm(materials):
    """Curved ringed stem and a ribbed crown of individual tapered pinnae."""
    g=Geometry()
    materials["palm bark"]=material((.76,.80,.66),.89,base="palm_bark_Diffuse.jpg",normal="palm_bark_nor_gl.jpg")
    materials["frond stem"]=material((.34,.30,.095),.78)
    for i,c in enumerate([(.055,.22,.066),(.12,.31,.072),(.22,.39,.088),(.12,.26,.075)]):
        materials[f"pinna {i}"]=material(c,.64)
    rng=np.random.default_rng(941)
    def axis(t):return np.array([.83*t*t,6.0*t,.24*math.sin(t*2)])
    for i in range(36):
        t=i/36;r=.245-.095*t+.11*math.exp(-12*t)
        cylinder(g,"palm bark",axis(t),axis((i+1)/36),r,r-.019,10,.9)
    center=axis(1)
    for k in range(16):
        theta=k*2.39996
        forward=np.array([math.cos(theta),0,math.sin(theta)])
        side=np.array([-forward[2],0,forward[0]])
        length=rng.uniform(3.2,4.35);lift=rng.uniform(1.1,2.55)
        def rib(t):return center+forward*length*t+np.array([0,lift*math.sin(t*2.65)-1.18*t*t,0])
        for j in range(6):
            cylinder(g,"frond stem",rib(j/6),rib((j+1)/6),.035*(1-j/7),.029*(1-j/7),5)
        for j in range(1,23):
            t=j/24;width=(math.sin(t*math.pi)**.65)*.84+.06
            for sign in [-1,1]:
                start=rib(t)
                end=start+side*sign*width+forward*(.20+width*.38)+[0,-width*rng.uniform(.28,.66),0]
                mid=(start+end)*.5+[0,.13*width,0]
                half=forward*.07*(.45+.7*width)
                tip=mid+[0,.05,0]
                name="pinna "+str((k+j//4)%4)
                for tri in [[start,mid-half,tip],[mid-half,end,tip],[tip,end,mid+half],[tip,mid+half,start]]:
                    g.face(name,tri[::-1] if sign<0 else tri)
    return g
