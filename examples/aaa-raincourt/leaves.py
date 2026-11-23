"""Veined broad leaves, a gnomon frieze and climbing vegetation for Raincourt."""
import math
import random
import numpy as np
from PIL import Image
from geometry import Geometry, cylinder, material, unit


def blade_maps(assets):
    # The axial rib and lateral veins are sampled in leaf UV space. Both colour and
    # signed gradients use the same vein height, including fine ridges near the margin.
    y,x=np.mgrid[:1536,:768];u=(x+.5)/768;v=(y+.5)/1536
    d=np.abs(u-.5)*2
    rng=np.random.default_rng(7152)
    phase=(v+(.28*d+.055*d*d)*(1.1-v*.3))*26
    ripple=np.cos(2*np.pi*phase)
    vein=np.exp(-22*(1-ripple))
    mid=np.exp(-d*d*3300)
    fine=.5+.5*np.cos(2*np.pi*(phase*4.23+.21*np.sin(phase*4)))
    grain=rng.normal(0,1,(1536,768))
    gain=.76+.085*ripple+.17*(1-d)+.16*v+.07*vein+.035*fine
    color=np.stack((49*gain+23*mid+7*vein,85*gain+30*mid+10*vein,45*gain+9*mid),axis=-1)
    color+=grain[:,:,None]*2.3
    Image.fromarray(np.clip(color,0,255).astype("u1")).save(assets/"blade-color.jpg",quality=94)
    height=(.29*vein+1.15*mid+.09*fine+.012*grain)
    dy,dx=np.gradient(height)
    normal=unit(np.stack((-2.7*dx,2.7*dy,np.ones_like(d)),axis=-1))
    Image.fromarray(((normal*.5+.5)*255).astype("u1")).save(assets/"blade-normal.png")


def heliconia(mats):
    g=Geometry();rng=random.Random(512)
    mats["banana blade"]=material((.95,1,.80),.52,base="blade-color.jpg",normal="blade-normal.png")
    mats["banana blade"]["surface"]=("leaf.wgsl",[-.57,.91,.56,1.8])
    mats["leaf midrib"]=material((.21,.31,.07),.72)
    for j in range(11):
        a=j*2.39996
        f=np.array([math.cos(a),0,math.sin(a)]);side=np.array([-f[2],0,f[0]])
        h=rng.uniform(1.3,2.7);length=rng.uniform(.88,1.8);spread=rng.uniform(.24,.46)
        origin=np.array([0.,.02,0.])+f*rng.uniform(0,.14)
        stem=origin+f*rng.uniform(.32,.73)+[0,h*.61,0]
        cylinder(g,"leaf midrib",origin,stem,.035,.018,7)
        rows=[];uv=[]
        for i in range(23):
            t=i/22
            center=stem+f*length*t+[0,h*(.52*math.sin(t*2.44)-.13*t*t),0]
            width=spread*math.sin(math.pi*t)**.72*(.87+.13*t)
            for u in [-1,-.46,0,.46,1]:
                edge=width*(1-.26*(i in (13,18) and u<-.9)-.37*(i==7 and u>.9))
                p=center+side*(u*edge)+[0,-.12*abs(u)*math.sin(t*math.pi)+.035*u*math.sin(22*t+j),0]
                rows.append(p);uv.append(((u+1)*.5,1-t))
        p=np.array(rows).reshape(23,5,3)
        across=np.gradient(p,axis=1);along=np.gradient(p,axis=0)
        norm=unit(np.cross(across,along))
        if norm[11,2,1]<0:norm=-norm
        idx=[v for i in range(22) for j in range(4) for a0 in [i*5+j] for v in (a0,a0+1,a0+5,a0+5,a0+1,a0+6)]
        g.add("banana blade",p.reshape(-1,3),norm.reshape(-1,3),uv,idx)
    return g


def creepers(mats):
    g=Geometry();rng=random.Random(207)
    mats["vine bark"]=material((.19,.15,.065),.98)
    for i,c in enumerate([(.23,.40,.095),(.10,.28,.075),(.18,.34,.09)]):
        mats["creeper "+str(i)]=material(c,.68)
    def leaf(base,size,theta,name):
        f=np.array([math.cos(theta),-.34,math.sin(theta)]);s=unit(np.cross(f,[0,1,0]))
        tip=base+f*size+[0,-.13*size,0];center=base+f*(size*.48)
        left=center+s*size*.34;right=center-s*size*.34;rib=center+[0,.08*size,0]
        g.face(name,[base,left,rib]);g.face(name,[left,tip,rib]);g.face(name,[rib,tip,right]);g.face(name,[base,rib,right])
    # Strands follow front faces and wrap over tower ledges before trailing down.
    for cluster,(x,y,z,length) in enumerate([(-3.5,7.75,-.96,4.9),(-2.52,7.1,-.87,3.6),
        (2.7,8.13,-.91,3.0),(4.45,5.47,-1.00,3.5),(6.7,5.26,-2.9,3),
        (-6.9,5.5,-2.93,3.4),(-1.24,6.38,-1.3,1.8),(.75,7.14,-.45,1.9)]):
        for strand in range(rng.randint(9,18)):
            offset=rng.uniform(-.73,.73);fall=length*rng.uniform(.37,1.12);steps=max(5,round(fall*4))
            prev=np.array([x+offset,y+rng.uniform(-.14,.15),z-.35])
            for k in range(1,steps+1):
                t=k/steps
                node=np.array([x+offset+math.sin(t*4+strand)*.12,y-fall*t,z+min(t*2,.65)+.11*math.sin(t*4+strand)])
                cylinder(g,"vine bark",prev,node,.025*(1-.7*t),.019*(1-.7*t),4)
                for j in range(3):
                    theta=(strand*1.63+k*2.4+j*1.8)
                    leaf(node+(prev-node)*j/3,rng.uniform(.19,.38)*(1-.38*t),theta,"creeper "+str((j+k+cluster)%3))
                prev=node
    return g


def strangler(mats):
    """Buttressed trunk, climbing roots and aerial lianas occupying the near bank."""
    rng=random.Random(1051);g=Geometry()
    mats["fig bark"]=material((.52,.57,.39),.95,base="palm_bark_Diffuse.jpg",normal="palm_bark_nor_gl.jpg")
    mats["liana"]=material((.23,.25,.117),.94,base="forest_ground_04_Diffuse.jpg")
    trunk=np.array([[-11.3,.02,3.6],[-11.15,2.9,3.4],[-10.63,6.3,3.0],[-9.0,9.0,2.6],[-8.3,12.3,3.45],[-7.8,15.8,4.2]])
    radii=[1.49,1.16,.99,.86,.64,.19]
    for i,(a,b) in enumerate(zip(trunk,trunk[1:])):
        cylinder(g,"fig bark",a,b,radii[i],radii[i+1],14,1.85)
    for i in range(13):
        a=2*math.pi*i/13;f=np.array([math.cos(a),0,math.sin(a)]);side=np.array([-f[2],0,f[0]])
        foot=trunk[0]+f*rng.uniform(3.1,5.5);tall=trunk[0]+f*.90+[0,rng.uniform(2.0,3.8),0]
        mid=trunk[0]+f*2.35+[0,rng.uniform(.6,1.27),0]
        for sign in [-1,1]:
            g.face("fig bark",[tall,mid,mid+side*.35*sign-[0,.68,0],tall+side*.52*sign-[0,2.0,0]],1.9)
            g.face("fig bark",[mid,foot,foot+side*.26*sign,mid+side*.35*sign-[0,.68,0]],1.9)
    # Large crooked limbs bridge the shot; hanging roots carry leaf clusters only near sunlight.
    for j,(start,end) in enumerate([(trunk[3],[-1.25,12.8,3.6]),(trunk[4],[-14.5,15.9,4.4]),(trunk[2],[-15,10,-2.1]),(trunk[4],[1.1,15.2,-1.3])]):
        end=np.array(end);old=start
        for i in range(1,8):
            t=i/7
            p=start*(1-t)+end*t+[0,math.sin(math.pi*t)*1.05,math.sin(1.4*math.pi*t)*.5]
            cylinder(g,"fig bark",old,p,.58*(1-(i-1)/8),.55*(1-i/8),10,1.6);old=p
            if i>1:
                tendril=p.copy();length=rng.uniform(1.6,4.0)
                for k in range(1,10):
                    u=k/9
                    tip=p+[.26*math.sin(4*u+j),-length*u,.24*math.sin(u*5+i)]
                    cylinder(g,"liana",tendril,tip,.029*(1-.45*u),.023*(1-.45*u),5)
                    tendril=tip
    return g
