"""Bake small, individually rooted tropical plants into opaque, material-batched cell meshes."""
import numpy as np

from court import bank, ground
from geometry import material, unit


def plant_cell(mesh, materials, center_x, center_z, seed):
    rng = np.random.default_rng(seed)
    # Stratify the roots, then cut channels through the carpet for stone, paths and water.
    ix, iz = np.meshgrid(np.arange(106), np.arange(82))
    x = center_x + ((ix.ravel()+rng.random(ix.size))/106 - .5) * 25
    z = center_z + ((iz.ravel()+rng.random(iz.size))/82 - .5) * 20
    channel = np.array([bank(px) for px in x])
    density = .86 + .14 * np.sin(x * 1.21 + .7 * np.cos(z)) * np.cos(z * .79)
    keep = ((np.abs(x) > 4.15) | (z < -10))
    keep &= (np.abs(x) > 9.6) | (z > 1.1) | (z < -10.5)
    keep &= (np.abs(z-channel) > 3.4) & (rng.random(len(x)) < density)
    x, z = x[keep], z[keep]
    roots = np.column_stack((x-center_x, [ground(px,pz)-.016 for px,pz in zip(x,z)], z-center_z))
    colors = [(.078,.23,.087), (.14,.30,.082), (.16,.25,.072)]
    species = rng.integers(0, len(colors), len(roots))
    for s, color in enumerate(colors):
        tag = f"forest floor leaf {s}"
        materials[tag] = material(color, .76)
        root = roots[species == s]
        if not len(root):
            continue
        height = rng.uniform(.11,.36,(len(root),1)) * (1+.24*s)
        azimuth = rng.uniform(0,2*np.pi,len(root))
        for j in range(4):
            theta = azimuth + j*2.4 + rng.uniform(-.23,.23,len(root))
            direction = np.column_stack((np.cos(theta),np.zeros(len(root)),np.sin(theta)))
            side = direction[:,[2,1,0]] * [-1,0,1]
            up = np.array([0.,1.,0.])
            extent = height * rng.uniform(.55,1.22,(len(root),1))
            middle = root + direction*extent*.36 + up*height*.61
            tip = root + direction*extent + up*height*rng.uniform(.52,.92,(len(root),1))
            width = height * (.09+.026*s)
            vertices = np.stack((root, middle-side*width, middle+up*width*.44, middle+side*width, tip),axis=1)
            normals = np.stack((unit(up-.4*direction), unit(up-.4*side), unit(up-.25*direction),
                                unit(up+.4*side), unit(up+.35*direction)),axis=1)
            indices = np.arange(len(root))[:,None]*5 + np.array([0,2,1,0,3,2,1,2,4,2,3,4])
            # Winding follows the raised upper surface. glTF tangents and normals agree above and below.
            uv = np.tile([[.5,0],[0,.6],[.5,.6],[1,.6],[.5,1]],(len(root),1))
            mesh.add(tag,vertices.reshape(-1,3),normals.reshape(-1,3),uv,indices.ravel())
    return len(roots)
