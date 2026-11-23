"""Raincourt's terraced sanctuary, corbelled gates, rain dial and tidal basin."""
import math
import random
import numpy as np
from geometry import Geometry, material, unit, yaw, cylinder

ROCK = dict(base="mossy_sandstone_Diffuse.jpg", normal="mossy_sandstone_nor_gl.jpg", arm="mossy_sandstone_arm.jpg")
WALL = dict(base="mossy_stone_wall_Diffuse.jpg", normal="mossy_stone_wall_nor_gl.jpg", arm="mossy_stone_wall_arm.jpg")
MATERIALS = {
    "ashlar":material((.92,.84,.66),.87,**ROCK),
    "lichen":material((.62,.73,.51),.91,**ROCK),
    "wall":material((.80,.76,.62),.91,**WALL),
    "basalt":material((.47,.49,.40),.60,**ROCK),
    "trim":material((.96,.80,.53),.80,**ROCK),
    "earth":material((.63,.61,.49),.95,base="forest_ground_04_Diffuse.jpg",normal="forest_ground_04_nor_gl.jpg",arm="forest_ground_04_arm.jpg"),
    "bronze":material((.36,.23,.074),.37,metal=.66),
    "patina":material((.14,.26,.19),.51,metal=.35),
    "water":material((.014,.049,.040),.155,normal="ripple.png",metal=.42),
}
def bank(x):
    return 9.5+1.85*math.sin(x*.165)+1.2*math.sin(x*.076+.6)


def ground(x,z):
    d=abs(z-bank(x))
    base=.15+.12*math.sin(x*.43+z*.16)+.14*math.sin(z*.34)
    slope=max(0,-z-17)*.075+max(0,abs(x)-20)*.095
    return base + slope - 1.9*math.exp(-(d/3.5)**3)


def block(g, m, x,y,z,w,h,d,angle=0, bevel=.08):
    """Chipped octagonal corners and a bevel on both horizontal courses."""
    cut=min(w*.18,d*.18,bevel*1.7)
    v=min(h*.16,bevel*.61)
    outline=np.array([[-w/2+cut,-d/2],[-w/2,-d/2+cut],[-w/2,d/2-cut],[-w/2+cut,d/2],
                      [w/2-cut,d/2],[w/2,d/2-cut],[w/2,-d/2+cut],[w/2-cut,-d/2]])
    rings=[]
    for height,offset in [(0,v), (v,0),(h-v,0),(h,v)]:
        q=outline*(1-2*offset/np.array([w,d]))
        rings.append(np.column_stack((q[:,0],np.repeat(height,8),q[:,1]))@yaw(angle).T+[x,y,z])
    for lo,hi in zip(rings,rings[1:]):
        for i in range(8):
            j=(i+1)%8
            g.face(m,[lo[j],lo[i],hi[i],hi[j]],3.1)
    g.face(m,rings[-1],3.1)


def masonry(g,rng,x,z,width,rows,depth,base=0,broken=False):
    for row in range(rows):
        count=max(2,round(width/1.65))
        cuts=np.linspace(-width/2,width/2,count+1)
        cuts[1:-1]+=[rng.uniform(-.32,.32)+(.19 if row%2 else -.19) for _ in range(count-1)]
        for i,(a,b) in enumerate(zip(cuts,cuts[1:])):
            if broken and row>rows-3 and (i+row*2)%7<2:continue
            block(g,rng.choice(["ashlar","wall","wall","lichen"]),x+(a+b)/2,
                  base+row*.52,z+rng.uniform(-.055,.055),b-a-.02,.52,depth+rng.uniform(-.12,.12),rng.uniform(-1,1))


def spire(g,x,y,z,w,height):
    # Tapered, stepped masonry with deeply shadowed cornices; each crown has a lotus finial.
    for i in range(7):
        t=i/7
        side=w*(1-.78*t)
        block(g,"ashlar",x,y+t*height,z,side,height/7,side*.88,bevel=.16)
        block(g,"trim" if i%2==0 else "lichen",x,y+(t+.095)*height,z,side+.24,.18,side*.88+.24,bevel=.08)
        for sign in [-1,1]:
            block(g,"basalt",x+sign*(side/2-.13),y+t*height,z+side*.42,.26,height*.15,.29,bevel=.06)
    cylinder(g,"trim",(x,y+height,z),(x,y+height+.73,z),w*.13,.06,8)


def ring(g,mat,x,y,z,inner,outer,depth,n=48):
    for i in range(n):
        a,b=2*math.pi*i/n,2*math.pi*(i+1)/n
        def p(r,t,dz): return (x+r*math.cos(t),y+r*math.sin(t),z+dz)
        g.face(mat,[p(inner,a,depth),p(outer,a,depth),p(outer,b,depth),p(inner,b,depth)])
        g.face(mat,[p(outer,b,0),p(outer,b,depth),p(outer,a,depth),p(outer,a,0)])


def arch(g,x,y,z,width):
    # Voussoirs are individual stones with real radial joints and a walk-through opening.
    r=width/2; foot=2.5; depth=1.65
    for sign in [-1,1]:
        for j in range(5):
            block(g,"ashlar",x+sign*(r+.39),y+j*.5,z,.83,.49,depth)
        block(g,"trim",x+sign*(r+.39),y+2.29,z,1.14,.24,depth+.22)
    for i in range(15):
        a=(i+.03)*math.pi/15;b=(i+.97)*math.pi/15
        outer=r+.72+(.22 if i==7 else 0)
        coords=[(r,a),(outer,a),(outer,b),(r,b)]
        front=[(x+rad*math.cos(t),y+foot+rad*math.sin(t),z+depth/2) for rad,t in coords]
        back=[(a,b,c-depth) for a,b,c in front]
        g.face("trim",front,2.3);g.face("ashlar",back[::-1],2.3)
        for j in range(4):
            k=(j+1)%4;g.face("basalt",[front[k],front[j],back[j],back[k]],2.3)


def sanctuary():
    g=Geometry();r=random.Random(921)
    # A raised plinth meets the stair at its top landing.
    for i in range(4):
        block(g,"lichen" if i%2 else "ashlar",0,-.22+i*.4,-4,18-i*.45,.42,11-i*.34,bevel=.19)
    for i in range(13):
        z=10.9-i*.76
        y=-.89+i*.18
        for k in range(3):
            block(g,r.choice(["ashlar","basalt","lichen"]),-2.34+k*2.34,y-.44,z,2.32,.62,1.06,bevel=.08)
        if i%2==0:
            for sign in [-1,1]:
                block(g,"basalt",sign*3.72,y-.26,z,.52,.79,1.55)
                block(g,"trim",sign*3.72,y+.53,z,.70,.15,1.65)
    plinth=1.38
    # Flanking towers and colonnade recede behind the sculptural front gate.
    for sign in [-1,1]:
        masonry(g,r,sign*6.25,-4.0,5.4,7,1.85,plinth,True)
        for tx,tz,scale in [(3.0,-2.8,1),(7.35,-4.2,.69),(4.7,-8.7,.84)]:
            x=sign*tx;w=3.15*scale; h=3.7*scale
            block(g,"basalt",x,plinth,tz,w+.7,.42,w+.7)
            for j in range(6):
                block(g,"ashlar" if j%3 else "lichen",x,plinth+.42+j*h/6,tz,w,h/6-.014,w)
            for j in [0,1]:
                block(g,"trim",x,plinth+h+j*.27,tz,w+.47-j*.15,.22,w+.47-j*.15)
            spire(g,x,plinth+h+.46,tz,w+.2,(3.2 if sign>0 or tx<4 else 1.7)*scale)
        for x,z,h in [(10,0,2.7),(12,-3,3.3),(12,-7,1.45)]:
            block(g,"basalt",sign*x,.02,z,1.52,.25,1.52)
            block(g,"ashlar",sign*x,.27,z,1.03,h,1.06)
            block(g,"lichen",sign*x,h+.27,z,1.47,.25,1.5)
    arch(g,0,plinth,-2.48,3.05)
    block(g,"trim",0,plinth+4.7,-2.55,4.7,.37,2.08)
    spire(g,0,plinth+5.07,-2.55,4.2,4.45)
    # Original tide calendar: twelve weathered bronze teeth and a concentric jade dial.
    x,y,z=0,plinth+5.83,-.59
    ring(g,"basalt",x,y,z,0,1.13,.16)
    ring(g,"trim",x,y,z+.16,.77,1.01,.10)
    ring(g,"basalt",x,y,z+.22,.46,.76,.08)
    for count, radius, size in [(24,1.10,.32),(18,.90,.19)]:
        for i in range(count):
            t=i*2*math.pi/count
            u=np.array([math.cos(t),math.sin(t),0]);v=np.array([-u[1],u[0],0])
            p=np.array([x,y,z+.33])+u*(radius-.10)
            ridge=p+u*(size*.47)+[0,0,.135];tip=p+u*size
            for tri in ([p-v*.11,ridge,p+v*.11],[p-v*.11,tip,ridge],[ridge,tip,p+v*.11]):
                g.face("trim",tri,2.3)
    for i in range(12):
        a=i*math.pi/6;u=np.array([math.cos(a),math.sin(a),0]);v=np.array([-u[1],u[0],0])
        center=np.array([x,y,z+.32])
        g.face("bronze",[center+u*.69-v*.05,center+u*1.04,center+u*.69+v*.05])
    arch(g,0,plinth,-8.6,3.05)
    # Broken blocks and collapsed masonry where the left bank has subsided.
    for _ in range(72):
        x=r.choice([-1,1])*r.uniform(4.15,12);z=r.uniform(-2,8)
        block(g,r.choice(["ashlar","lichen","basalt"]),x,ground(x,z)-.03,z,r.uniform(.5,1.8),r.uniform(.23,.85),r.uniform(.48,1.3),r.uniform(-60,60))
    return g


def landscape():
    g=Geometry()
    xs=np.linspace(-66,66,133);zs=np.linspace(-58,51,110)
    pts=[];norm=[];uv=[];idx=[]
    for j,z in enumerate(zs):
        for i,x in enumerate(xs):
            pts.append((x,ground(x,z),z))
            norm.append(unit((ground(x-.1,z)-ground(x+.1,z),.2,ground(x,z-.1)-ground(x,z+.1))))
            uv.append((x/3,z/3))
            if i and j:
                a=j*len(xs)+i;idx.extend([a,a-len(xs),a-1,a-1,a-len(xs),a-len(xs)-1])
    g.add("earth",pts,norm,uv,idx)
    return g


def stream():
    g=Geometry();p=[];n=[];uv=[];idx=[]
    for i,x in enumerate(np.linspace(-52,52,285)):
        for j,t in enumerate(np.linspace(-1,1,23)):
            z=bank(x)+t*3.85
            wave=.018*math.sin(3.2*x+2.55*z)+.012*math.sin(1.9*x-4.3*z)
            dx=.0576*math.cos(3.2*x+2.55*z)+.0228*math.cos(1.9*x-4.3*z)
            dz=.0459*math.cos(3.2*x+2.55*z)-.0516*math.cos(1.9*x-4.3*z)
            p.append((x,-.55+wave,z));n.append(unit((-dx,1,-dz)));uv.append((x/3,z/3))
            if i and j:
                a=i*23+j;idx.extend([a-24,a-23,a,a-24,a,a-1])
    g.add("water",p,n,uv,idx)
    return g
