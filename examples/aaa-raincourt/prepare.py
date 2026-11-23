"""Cook the Raincourt models into the demo's ignored asset directory."""
import argparse
import fcntl
import hashlib
import json
import math
import random
import shutil
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageOps
from leaves import blade_maps, heliconia, creepers, strangler
from botany import palm, source_meshes
from court import bank, ground, landscape, MATERIALS, sanctuary, stream
from shaft_depth import bake_shafts
from sky_visibility import bake_masonry
from canyon import cook_canyon
from branch_wind import rig_broadleaf
from groundcover import plant_cell
from geometry import Geometry, material, unit, write_model
from sources import ASSETS, prepare_sources

def canopy_cards(tree, materials):
    """Bake two orthogonal silhouettes of the reduced tree for the far grove."""
    g=Geometry()
    for k in range(2):
        side=np.array([[1.,0,0],[0,0,1.]][k])
        front=np.cross(side,[0,1,0])
        polygons=[];depths=[];colors=[]
        for mat,(p,n,uv,idx) in tree.packed().items():
            faces=idx.reshape(-1,3)
            center=p[faces].mean(axis=1)
            norm=unit(n[faces].mean(axis=1))
            image=np.array(Image.open(ASSETS/materials[mat]["base"]).convert("RGB"))
            tc=uv[faces].mean(axis=1)%1
            col=image[(tc[:,1]*(image.shape[0]-1)).astype(int),(tc[:,0]*(image.shape[1]-1)).astype(int)]
            shade=np.clip(.73+norm@unit([-.5,.8,.3])*.25+center[:,1]*.035,.47,1.16)
            colors.extend(np.clip(col*shade[:,None],0,255).astype('u1'))
            coords=np.column_stack(((p@side+2.85)*112, (5.35-p[:,1])*112))
            polygons.extend(np.rint(coords[faces]).astype('i4').reshape(-1,6))
            depths.extend(center@front)
        im=Image.new("RGBA",(640,608));draw=ImageDraw.Draw(im)
        for i in np.argsort(depths):
            draw.polygon(polygons[i].tolist(),fill=tuple(colors[i])+(255,))
        filename=f"canopy-{k}.png";im.save(ASSETS/filename)
        m=f"canopy {k}";materials[m]=material((.86,.94,.84),.98,base=filename,mask=True)
        p=[side*-2.85+[0,5.35-608/112,0],side*(640/112-2.85)+[0,5.35-608/112,0],
           side*(640/112-2.85)+[0,5.35,0],side*-2.85+[0,5.35,0]]
        g.add(m,p,[(0,.93,.36)]*4,[(0,1),(1,1),(1,0),(0,0)],[0,1,2,0,2,3])
    return g

def cook():
    prepare_sources()
    rng=random.Random(64021)
    mats=dict(MATERIALS);counts={};props=[];trees=[];cells=[]
    def model(name,mesh,bend=None,translation_clip=None):
        counts[name]=write_model(ASSETS/(name+".gltf"),mesh,mats,bend,translation_clip)
    def prop(name,x,z,scale,rotation=0,offset=0):
        props.append(dict(model=name,position=[x,ground(x,z)+offset,z],scale=scale,yaw=rotation))
    y,x=np.mgrid[:512,:512]*2*np.pi/512
    a=.085*np.sin(9*x+3*y+1.7*np.sin(4*y))+.032*np.sin(23*x+13*y)
    b=.06*np.cos(7*x+8*y)+.031*np.sin(13*x-19*y)
    vectors=unit(np.stack((a,b,np.ones_like(a)),axis=-1))
    Image.fromarray(((vectors*.5+.5)*255).astype('u1')).save(ASSETS/"ripple.png")
    ruins, earth = sanctuary(), landscape()
    model("earth",earth)
    # Smooth periodic drift of the rippled sheet with the same XYZ amplitudes as the original water.
    times = np.linspace(0,36,433)
    phase = 2*np.pi*times/36
    offsets = np.column_stack((.12*np.sin(4*phase),.009*np.sin(10*phase),.07*np.sin(3*phase)))
    model("creek",stream(),translation_clip=("Current",times,offsets))
    names=["fern_02","shrub_03","island_tree_01","root_cluster_01","namaqualand_boulder_03"]
    ferns,shrubs,tree,root,boulder=[source_meshes(ASSETS,n,mats) for n in names]
    model("tree",tree[0]);cards=canopy_cards(tree[0],mats);model("tree-proxy",cards)
    for name in boulder[0].packed():
        mats[name]["surface"]=("wetstone.wgsl",[-.50,.68,0,0])
    model("roots",root[0]);model("riverstone",boulder[0]);model("palm",palm(mats),(3.1,3.5,2.1))
    model("fern",ferns[0],(.03,.44,4.1))
    for shader in Path(__file__).parent.glob("*.wgsl"):
        shutil.copyfile(shader, ASSETS/shader.name)
    trunk = strangler(mats)
    model("strangler",trunk)
    model("sanctuary",bake_masonry(ruins,[ruins,trunk],mats,ASSETS))
    shafts = bake_shafts([ruins, earth, trunk])
    (ASSETS/"shaft-depth.json").write_text(json.dumps(shafts))
    blade_maps(ASSETS)
    model("heliconia",heliconia(mats))
    rig_broadleaf(ASSETS/"heliconia.gltf")
    cook_canyon(ASSETS,mats)
    model("creepers",creepers(mats))
    head=source_meshes(ASSETS,"lion_head",mats)[0]
    stone=ImageOps.grayscale(Image.open(ASSETS/mats["lion_head"]["base"]))
    stone=ImageOps.colorize(stone,(35,43,37),(191,174,129))
    stone.save(ASSETS/"guardian-stone.jpg",quality=94)
    mats["lion_head"]["base"]="guardian-stone.jpg"
    model("guardian",head)
    model("bluff",source_meshes(ASSETS,"namaqualand_cliff_02",mats)[0])
    for x,z,sz,angle in [(-24,-26,[1.7,2.65,1.5],16),(8,-36,[1.8,2.0,2],-6),
                          (-31,-4,[1.15,1.8,1.4],75),(29,-19,[1.6,2.5,1.5],-29)]:
        prop("bluff",x,z,sz,angle,-1.5)
    for x,z,s,angle in [(-5.1,6.5,1.4,35),(7,4,1.15,70),(9.55,16,1.12,15),(-5.1,15,1.4,110),
                          (-9,-2,1.4,0),(5.55,-1,1.08,80),(-12,4,1.24,75),(14,1,1.35,150),
                          (-6.8,1.3,.84,-14),(4.8,8,.72,135),(10,-5,1.12,95),(-11,-5,1.4,100)]:
        prop("heliconia",x,z,[s]*3,angle)
    # Keep the central staircase and creek open; both hero views carry foreground foliage.
    for x,z,s,yaw in [(-7.8,6.2,1.68,-39),(8.3,1.0,1.74,25),(-9,-8,1.54,-50),
                        (13.8,-6,1.50,25),(-14,11,1.7,-63),(9,-14,1.63,90),(15,13,1.5,80)]:
        prop("palm",x,z,[s]*3,yaw)
    for i in range(17):
        x=rng.choice([-1,1])*rng.uniform(5.0,20);z=rng.uniform(-2,15);s=rng.uniform(1.4,3.1)
        prop("fern",x,z,[s]*3,rng.randrange(360))
    for x,z,s,angle in [(-4.3,6,1.5,40),(6.0,3,1.6,130),(-8,-.5,2,100),(4.7,-8,1.5,-20)]:
        prop("roots",x,z,[s]*3,angle,.1)
    for i in range(28):
        x=rng.uniform(-23,23)
        if abs(x)<3.8:continue
        z=bank(x)+rng.choice([-1,1])*rng.uniform(1.5,4.5)
        s=rng.uniform(.45,1.4);prop("riverstone",x,z,[s,s*rng.uniform(.6,.95),s],rng.randrange(360),rng.uniform(.05,.40))
    for i in range(48):
        angle=2*math.pi*i/48+rng.uniform(-.09,.09);distance=rng.uniform(20,38)
        x=distance*math.cos(angle);z=-4+distance*math.sin(angle)
        if abs(x)<11 and z>10:continue
        s=rng.uniform(1.85,3.35)
        trees.append(dict(position=[x,ground(x,z),z],scale=s,yaw=rng.randrange(360)))
    # Lower-cost distant canopy consolidated into four grove meshes.
    for quadrant in range(4):
        patch=Geometry()
        for i in range(50):
            theta=(quadrant+rng.random())*math.pi/2
            radius=rng.uniform(40,63);x=radius*math.cos(theta);z=-5+radius*math.sin(theta)
            patch.merge(cards,[x,ground(x,z),z],rng.uniform(2.3,4.8),rng.randrange(360))
        model(f"horizon-{quadrant}",patch)
    # Each understory cell is compacted offline by material. It owns real plants, not point markers.
    for ix in range(2):
        for iz in range(2):
            patch=Geometry();count=0;cx=(-1 if ix==0 else 1)*13;cz=(-1 if iz==0 else 1)*10
            for _ in range(230):
                x=cx+rng.uniform(-12.5,12.5);z=cz+rng.uniform(-10,10)
                if abs(x)<4.15 and z>-10 or abs(x)<9.6 and -10.5<z<1.1 or abs(z-bank(x))<3.6:continue
                source=rng.choice(ferns[2:]) if rng.random()<.65 else rng.choice(shrubs)
                size=rng.uniform(1.2,3.0)
                patch.merge(source,[x-cx,ground(x,z)-.025,z-cz],size,rng.randrange(360))
                count+=1
            groundcover = plant_cell(patch, mats, cx, cz, 8302 + 2*ix + iz)
            name=f"understory-{ix}-{iz}";model(name,patch)
            cells.append(dict(name=name,center=[cx,0,cz],plants=count,groundcover=groundcover))
    (ASSETS/"layout.json").write_text(json.dumps(dict(props=props,trees=trees,cells=cells,triangles=counts),indent=2))
    print("Prepared models:",counts)
    print("Understory plants:",sum(c["plants"] for c in cells))

def ensure_assets(force):
    directory = Path(__file__).resolve().parent
    recipe = hashlib.sha256()
    inputs = ["prepare.py", "sources.py", "botany.py", "court.py", "leaves.py", "geometry.py", "groundcover.py", "shaft_depth.py", "sky_visibility.py", "canyon.py", "canyon_sky.py", "branch_wind.py", "sources.json"]
    for path in sorted([*(directory/name for name in inputs), *directory.glob("*.wgsl")]):
        recipe.update(path.name.encode())
        recipe.update(path.read_bytes())
    ASSETS.mkdir(exist_ok=True)
    receipt = ASSETS / ".raincourt.json"
    with (ASSETS / ".prepare.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if receipt.exists() and not force:
            try:
                cached = json.loads(receipt.read_text())
                if cached["recipe"] == recipe.hexdigest() and all(
                    (ASSETS/name).is_file() and (ASSETS/name).stat().st_size == size
                    for name, size in cached["files"].items()
                ):
                    return
            except (OSError, ValueError, KeyError):
                pass
        receipt.unlink(missing_ok=True)
        cook()
        sizes = {str(p.relative_to(ASSETS)): p.stat().st_size for p in ASSETS.rglob("*")
                 if p.is_file() and not p.name.startswith(".")}
        ready = receipt.with_suffix(".tmp")
        ready.write_text(json.dumps(dict(recipe=recipe.hexdigest(), files=sizes), sort_keys=True))
        ready.replace(receipt)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--force", action="store_true", help="recook even with a valid receipt")
    ensure_assets(parser.parse_args().force)
