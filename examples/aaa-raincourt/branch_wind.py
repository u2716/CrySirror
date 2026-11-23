"""Rig each authored heliconia petiole, blade and tip in its own bending plane."""
import json
import numpy as np
from geometry import unit


def rig_broadleaf(path):
    doc=json.loads(path.read_text())
    binary=bytearray(path.with_suffix(".bin").read_bytes())
    def append(array, dtype, kind, component):
        data=np.ascontiguousarray(array,dtype=dtype)
        binary.extend(bytes(-len(binary)%4))
        slot=len(doc["bufferViews"])
        doc["bufferViews"].append(dict(buffer=0,byteOffset=len(binary),byteLength=data.nbytes))
        binary.extend(data.tobytes())
        doc["accessors"].append(dict(bufferView=slot,componentType=component,count=len(data),type=kind))
        return len(doc["accessors"])-1
    primitives={doc["materials"][p["material"]]["name"]:p for p in doc["meshes"][0]["primitives"]}
    def positions(prim):
        acc=doc["accessors"][prim["attributes"]["POSITION"]]
        off=doc["bufferViews"][acc["bufferView"]]["byteOffset"]
        return np.frombuffer(binary,'<f4',count=3*acc["count"],offset=off).copy().reshape(-1,3)
    blade=positions(primitives["banana blade"]).reshape(-1,23,5,3)
    stalk=positions(primitives["leaf midrib"]).reshape(len(blade),2,-1,3)
    joints={"banana blade":[],"leaf midrib":[]}
    weights={k:[] for k in joints}
    doc["nodes"].append(dict(name="root",children=[]))
    doc["nodes"][0]["skin"]=0
    doc["scenes"][0]["nodes"].append(1)
    joint_nodes=[1]; inverse=[np.eye(4)]
    times=np.linspace(0,8,49)
    clock=append(times,"<f4","SCALAR",5126)
    doc["accessors"][clock].update(min=[0],max=[8])
    samplers=[];channels=[]
    for i,rows in enumerate(blade):
        foot=stalk[i,0].mean(axis=0)
        pivots=[foot,rows[0,2],rows[11,2]]
        axis=unit(np.cross(rows[-1,2]-rows[0,2],[0,1,0]))
        parent=1;previous=np.zeros(3);base=len(joint_nodes)
        for j,(pivot,degrees) in enumerate(zip(pivots,[1.9,4.3,7.6])):
            node=len(doc["nodes"])
            doc["nodes"][parent]["children"].append(node)
            doc["nodes"].append(dict(name=f"leaf-{i}-{j}",translation=(pivot-previous).tolist(),children=[]))
            joint_nodes.append(node)
            inv=np.eye(4);inv[3,:3]=-pivot;inverse.append(inv)
            phase=2*np.pi*times/8+i*.89-j*.54
            angle=np.deg2rad(degrees)*(.72*np.sin(phase)+.28*np.sin(2*phase+j))
            quat=np.column_stack((np.sin(angle[:,None]/2)*axis,np.cos(angle/2)))
            samplers.append(dict(input=clock,output=append(quat,"<f4","VEC4",5126)))
            channels.append(dict(sampler=len(samplers)-1,target=dict(node=node,path="rotation")))
            parent=node;previous=pivot
        n=stalk.shape[2]
        joints["leaf midrib"].extend([[0,base,0,0]]*(2*n))
        weights["leaf midrib"].extend([[1,0,0,0]]*n+[[0,1,0,0]]*n)
        for t in np.linspace(0,1,23):
            tip=np.clip((t-.37)/.49,0,1);blade_w=min(1,t/.28)*(1-tip)
            joints["banana blade"].extend([[base,base+1,base+2,0]]*5)
            weights["banana blade"].extend([[1-blade_w-tip,blade_w,tip,0]]*5)
    for tag,prim in primitives.items():
        prim["attributes"].update(JOINTS_0=append(joints[tag],"<u2","VEC4",5123),
                                  WEIGHTS_0=append(weights[tag],"<f4","VEC4",5126))
    doc["skins"]=[dict(joints=joint_nodes,inverseBindMatrices=append(inverse,"<f4","MAT4",5126))]
    doc["animations"]=[dict(name="Breeze",samplers=samplers,channels=channels)]
    doc["buffers"][0]["byteLength"]=len(binary)
    path.with_suffix(".bin").write_bytes(binary)
    path.write_text(json.dumps(doc,separators=(",",":")))
