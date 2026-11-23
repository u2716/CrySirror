"""First opaque surface along each sun column, in the coordinates of its mist shaft."""
import numpy as np

from geometry import unit

SUN_TRAVEL = np.array([.57,-.91,-.56])
OPENINGS = [(-5.6,11,3.7,1.10), (-4.1,10.3,3,.52), (-2.75,11.2,6.1,.68),
            (1.9,12.4,-3,.68), (4.1,11,-4.4,.93), (-10.1,11.7,-7.1,.80)]


def first_hit(triangles, direction):
    # Möller–Trumbore with the direction held fixed for all parallel columns.
    v0 = triangles[:,0]
    e1, e2 = triangles[:,1]-v0, triangles[:,2]-v0
    p = np.cross(direction, e2)
    det = np.sum(e1*p, axis=1)
    valid = np.abs(det) > 1e-7
    v0, e1, e2, p = (v[valid] for v in (v0,e1,e2,p))
    inv = 1.0/det[valid]

    def query(origin):
        offset = origin-v0
        u = inv*np.sum(offset*p,axis=1)
        q = np.cross(offset,e1)
        v = inv*np.sum(q*direction,axis=1)
        t = inv*np.sum(q*e2,axis=1)
        return np.min(t[(u>=0)&(v>=0)&(u+v<=1)&(t>=0)],initial=np.inf)
    return query


def bake_shafts(meshes):
    triangles = np.concatenate([p[idx.reshape(-1,3)] for mesh in meshes
                                for p, _, _, idx in mesh.packed().values()])
    axis = unit(SUN_TRAVEL)
    right = unit(np.cross(axis,[0,1,0]))
    up = np.cross(axis,right)
    query = first_hit(triangles,axis)
    shafts = []
    for x,y,z,radius in OPENINGS:
        origin = np.array([x,y,z])
        length = (.4-y)/axis[1]
        values = []
        for iy in range(6):
            for ix in range(4):
                column = origin+radius*((2*ix/3-1)*right+(2*iy/5-1)*up)
                depth = query(column-32*axis)-32
                values.append(int(np.rint(np.clip(depth/length,0,1)*255)))
        # Three exact 8-bit depths per f32 shader parameter, eight parameters per shaft.
        packed = [(a+256*b+65536*c)/16777216 for a,b,c in zip(values[::3],values[1::3],values[2::3])]
        shafts.append(dict(start=origin.tolist(),end=(origin+length*axis).tolist(),
                           radius=radius,depth=packed))
    return shafts
