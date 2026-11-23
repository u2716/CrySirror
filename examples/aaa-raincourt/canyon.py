"""Basalt ravine, vaulted approach, terraced cliff settlement and split cataract."""
import math
import random
import numpy as np
from canyon_sky import write_sky
from court import block, arch, spire
from geometry import Geometry, material, unit, cylinder, write_model


def cook_canyon(assets,mats):
    write_sky(assets/"ravine-day.hdr")
    r=random.Random(8751)
    hall=Geometry(); citadel=Geometry(); gorge=Geometry(); water=Geometry()
    mats['vault']=dict(mats['basalt'],color=(.43,.47,.41))
    mats['cliff']=dict(mats['ashlar'],color=(.55,.56,.46))
    mats['ledge']=dict(mats['lichen'],color=(.48,.55,.35))
    for side in (-1,1):
        for row in range(7):
            for j in range(11):
                z=6+j*1.54 + .24*(row%2)
                block(hall,'vault' if row%3 else 'basalt',side*(1.94+.03*r.random()),12+row*.53,z,
                      .85,.54,1.55,r.uniform(-1.2,1.2),.10)
        for z in [6,10.7,16.6,21.4]:
            block(hall,'basalt',side*1.60,12,z,.66,2.85,.68,bevel=.11)
            block(hall,'lichen',side*1.58,14.78,z,.88,.29,.94)
    for j in range(12):
        block(hall,'vault',0,15.35,5.8+j*1.46,4.8,.67,1.49,bevel=.11)
    for j in range(24):
        for x in [-1.05,1.05]:
            y=12-.023*max(0,8-j)
            block(hall,'ashlar',x,y-.38,22-j*1.02,2.09,.38,1.04,bevel=.09)
    # Broken lookout ledge sweeps right and descends below the level of the gallery.
    for j in range(24):
        x=j*.72;z=-1.65-j*.20;top=11.80-.13*j
        block(hall,'basalt',x,top-1.35,z,1.48,1.35,4.15,bevel=.18)
        if j%4==0:
            block(hall,'lichen',x,top,z-1.75,.87,r.uniform(.4,.91),.78,bevel=.16)
    # Cliff curtains wrap the pool; each ring changes relief with elevation.
    def escarpment(points,near=False):
        pos=[];uv=[];inds=[]
        for i,(x,z,top) in enumerate(points):
            for j in range(17):
                t=j/16; y=-32+t*(top+32)
                bulge=2.4*math.sin(i*.48+.42*math.sin(j*.8))+1.1*math.cos(i*.88+j*.47)+.65*math.sin(j*2.3)
                zz=z+(-1 if near else 1)*bulge
                pos.append([x,y,zz]);uv.append([x/7.8,y/8.1])
                if i and j:
                    k=i*17+j;inds.extend([k-18,k-1,k,k-18,k,k-17])
        p=np.asarray(pos).reshape(len(points),17,3)
        norm=unit(np.cross(np.gradient(p,axis=0),np.gradient(p,axis=1)))
        if near:norm=-norm
        gorge.add('cliff',p.reshape(-1,3),norm.reshape(-1,3),uv,inds)
    xs=np.linspace(-120,120,150)
    def rim(x):return -62-2.8*math.sin(x*.085)-1.4*math.cos(x*.26)
    def crest(x):return 8+.10*abs(x)+2.1*math.sin(x*.11)
    escarpment([(x,rim(x),crest(x)) for x in xs])
    escarpment([(x,2.7-2.6*math.sin(x*.085),11.0-.15*max(0,x)+1.0*math.sin(x*.18)) for x in xs],True)
    for x in np.arange(-110,114,4.2):
        z=rim(x);h=crest(x)
        block(gorge,'ledge',x,h-1,z-7,5.0,1.23,15,angle=r.uniform(-8,8),bevel=.36)
    # Rock shelves bed the near vegetation and make the overlook read as a cliff.
    for sign in [-1,1]:
        for i in range(8):
            x=sign*(5.7+6*i)
            block(gorge,'ledge',x,9.1,18,6.4,2.45,31+r.uniform(-1,1),bevel=.45)
    # Three overlapping mountain ridges give the city a receding skyline.
    for layer in range(3):
        ridge=[]
        for x in np.linspace(-180,180,150):
            h=22+10*layer+9*math.sin(x*.046+1.9*layer)+7*math.cos(x*.091+layer)+3*math.sin(x*.31)
            h-=13*math.exp(-((x+9)/24)**2)
            ridge.append((x,-111-27*layer+4*math.sin(x*.06),h))
        escarpment(ridge)
    # Galleries have genuine open bays. Their colonnades, parapets and roofs step
    # up the mountain instead of copying the old courtyard's symbolic centrepiece.
    for x,z,base,bays,tiers in [(-7,-71,9,8,3),(-30,-66,11,4,2),(17,-73,10,4,2),(-13,-83,20,4,2)]:
        for level in range(tiers):
            count=max(2,bays-2*level);span=count*3.7;height=4.0-level*.23
            xc=x+.55*level;zc=z-1.1*level
            block(citadel,'basalt',xc,base-.9,zc,span+2.0,1.00,8.3,bevel=.37)
            for k in range(count+1):
                px=xc+(k-count/2)*3.7
                block(citadel,'lichen',px,base,zc+2.8,1.44,.46,1.54)
                block(citadel,'ashlar',px,base+.46,zc+2.8,1.03,height-.56,1.11,bevel=.16)
                block(citadel,'trim',px,base+height-.26,zc+2.8,1.5,.41,1.6)
            for k in range(count):
                block(citadel,'lichen' if k%3 else 'basalt',xc+(k+.5-count/2)*3.7,base+height,zc,3.77,.75,8.55,bevel=.21)
            block(citadel,'wall',xc,base,zc-2.9,span+.45,height+.3,1.72)
            base+=height+1.0
        spire(citadel,x,base-.35,z-2.5,5.8 if tiers==3 else 3.8,7 if tiers==3 else 4.5)
    for k in range(19):
        width=9.6-.14*k
        z=-54-.58*k
        block(citadel,'trim' if k%4==0 else 'basalt',-7,4.9+.34*k,z,width,.55,1.2,bevel=.12)
        block(citadel,'cliff',-7,-23,z-.15,width-.42,27.93+.34*k,1.04,bevel=.21)
    for sign in [-1,1]:
        for k in range(6):
            block(citadel,'lichen',-7+sign*5.2,4.8+k,-54.6-k*1.74,1.02,.93,2.05)
    # An interrupted aqueduct spans the chasm behind the main cataract.
    for i in range(5):
        xx=-47+i*5.8
        block(citadel,'wall',xx,0,-53,1.8,13.5,3.6)
        if i<4:
            arch(citadel,xx+2.9,8.0,-53,4.1)
            block(citadel,'lichen',xx+2.9,13.22,-53,5.85,.75,3.8)
    # Independently scalloped water ribbons accelerate away from a projecting lip.
    mats['fall']=material((.60,.78,.77),.29)
    mats['fall']['blend']=True
    mats['fall']['surface']=('cataract.wgsl',[0,0,0,0])
    for k,(x,z,top,width,phase) in enumerate([(10,-55,11.3,4.5,.2),(14,-56,10.9,1.5,2.1),(-37,-57,12.8,1.15,3.2)]):
        p=[];uv=[];idx=[]
        for j in range(42):
            t=j/41
            for i in range(13):
                u=i/12
                xx=x+(u-.5)*width*(1+.43*t)+.26*math.sin(t*12+phase+u*3)*t
                zz=z+3.7*math.sqrt(t)+.40*math.sin(u*8+phase)+.15*math.sin(23*t+u*9)
                p.append([xx,top-(top+26)*t,zz]);uv.append((u,t))
                if i and j:
                    a=j*13+i;idx.extend([a-14,a-13,a,a-14,a,a-1])
        grid=np.array(p).reshape(42,13,3)
        n=unit(np.cross(np.gradient(grid,axis=0),np.gradient(grid,axis=1)))
        water.add('fall',p,n.reshape(-1,3),uv,idx)
        block(gorge,'basalt',x,top-2.2,z-8,width+2.1,2.23,16.45,bevel=.38)
    pool=Geometry()
    for z in range(-68,0,4):
        for x in range(-105,105,7):
            pool.face('water',[[x,-24,z],[x,-24,z+4],[x+7,-24,z+4],[x+7,-24,z]],4.8)
    for name,geo in [('gallery',hall),('citadel',citadel),('gorge',gorge),('cataract',water),('pool',pool)]:
        write_model(assets/(name+'.gltf'),geo,mats)
