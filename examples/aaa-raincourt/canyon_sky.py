"""Original equirectangular daylight sky for the exposed valley, stored as Radiance RGBE."""
import numpy as np


def write_sky(path):
    h,w=384,768
    y,x=np.mgrid[:h,:w]
    lat=np.pi*(.5-(y+.5)/h);lon=2*np.pi*(x+.5)/w
    v=np.stack((np.cos(lat)*np.cos(lon),np.sin(lat),np.cos(lat)*np.sin(lon)),axis=-1)
    sky=np.maximum(v[:,:,1],0)
    color=(np.array([.57,.70,.74])*(1-sky[:,:,None])**2+
           np.array([.15,.34,.53])*(1-(1-sky[:,:,None])**2))
    q=v/np.maximum(.12,v[:,:,1:2]+.12)
    signal=np.zeros((h,w));rng=np.random.default_rng(8193)
    for octave in range(5):
        noise=np.zeros_like(signal)
        for _ in range(5):
            a,b=rng.normal(size=2)
            noise+=np.cos((a*q[:,:,0]+b*q[:,:,2])*(1.7*2**octave)+rng.uniform(0,6.28))
        signal+=noise*.19/(1.9**octave)
    cloud=np.clip((signal+.28)*1.8,0,1)**1.4
    color=color*(1-.52*cloud[:,:,None])+np.array([.97,.98,.91])*.80*cloud[:,:,None]
    color=np.where((lat<0)[:,:,None],np.array([.21,.27,.24]),color)
    color*=1.6
    mant,exp=np.frexp(color.max(axis=2))
    rgb=(color*np.ldexp(np.ones_like(mant),8-exp)[:,:,None]).astype('u1')
    rgbe=np.dstack((rgb,(exp+128).astype('u1')))
    with path.open('wb') as out:
        out.write(f"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {h} +X {w}\n".encode())
        for row in rgbe:
            out.write(bytes([2,2,w>>8,w&255]))
            for channel in row.T:
                for k in range(0,w,120):
                    chunk=channel[k:k+120];out.write(bytes([len(chunk)]));out.write(chunk.tobytes())
