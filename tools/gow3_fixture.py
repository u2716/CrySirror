#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Retain one local OLY100 scene + one character, with a checked dependency closure.

Copies selected native inputs and referenced glTF/image/buffer assets only.
Never deletes sources. Game content remains ignored and must not be distributed.
"""
import argparse
import copy
import hashlib
import json
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path
from local_assets import output_root


def check(value, message):
    if not value:
        raise ValueError(message)


def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda:stream.read(1024*1024),b''):h.update(block)
    return h.hexdigest()


def copy_reports(scene, character, out):
    metadata=out/'metadata';metadata.mkdir(exist_ok=True)
    for label,project in [('scene',scene),('character',character)]:
        source=Path(project).resolve().parent.parent/'import-report.json'
        if source.is_file():shutil.copyfile(source,metadata/f'{label}-import.json')
    skeleton=Path(character).resolve().parent.parent/'source/native-skeleton.json'
    if skeleton.is_file():shutil.copyfile(skeleton,metadata/'native-skeleton.json')


def write_receipt(out, manifest):
    manifest['files']={}
    for path in sorted(out.rglob('*')):
        if path.is_file()and path!=out/'fixture.json':manifest['files'][path.relative_to(out).as_posix()]={
            'bytes':path.stat().st_size,'sha256':digest(path)}
    (out/'fixture.json').write_text(json.dumps(manifest,indent=2))


def copy_model(project_path, destination):
    project_path=Path(project_path).resolve()
    project=json.loads(project_path.read_text())
    check(len(project.get('clients',[]))==1,'fixture needs one source client')
    root=(project_path.parent/project['assets']).resolve()
    entities=project['clients'][0]['systems']['entities']
    models=[e for e in entities if e.get('geometry')]
    check(len(models)==1 and models[0]['geometry'].get('kind')=='scene',
          'fixture needs exactly one glTF scene entity per source project')
    source=(root/models[0]['geometry']['asset']).resolve()
    check(source.is_relative_to(root) and source.suffix=='.gltf','fixture model escapes asset root or is not glTF')
    document=json.loads(source.read_text())
    check(document.get('asset',{}).get('version')=='2.0','fixture glTF version')
    pending={source}
    for obj in document.get('buffers',[])+document.get('images',[]):
        uri=obj.get('uri')
        if uri is None or uri.startswith('data:'):continue
        check(isinstance(uri,str) and ':'not in uri and '%'not in uri and '\\'not in uri,
              'unsupported fixture asset URI')
        candidate=(source.parent/uri).resolve()
        check(candidate.is_relative_to(root) and candidate.is_file(),'fixture dependency escapes or is missing')
        pending.add(candidate)
    for material in document.get('materials',[]):
        effect=material.get('extras',{}).get('crysirror_material',{}).get('material',{}).get('effect',{})
        if effect.get('kind')=='custom':
            candidate=(source.parent/effect['asset']).resolve()
            check(candidate.is_relative_to(root) and candidate.is_file(),'fixture shader dependency escapes or is missing')
            pending.add(candidate)
    for path in sorted(pending):
        out=destination/path.relative_to(root)
        out.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(path,out)
    return project,source.relative_to(root).as_posix()


def compact_textures(out, codec):
    """Keep base-color previews at 512, other sampled images at 256 pixels.

    The shared engine codec does all decoding/resizing/encoding. Original high
    mips remain available in the selected native WAD/TS inputs.
    """
    jobs=[];targets=[]
    with tempfile.TemporaryDirectory(dir=out,prefix='.textures-')as temporary:
        temporary=Path(temporary)
        for model in sorted((out/'assets').rglob('*.gltf')):
            doc=json.loads(model.read_text());limits={i:256 for i in range(len(doc.get('images',[])))}
            for material in doc.get('materials',[]):
                base=material.get('pbrMetallicRoughness',{}).get('baseColorTexture')
                if base:
                    image=doc['textures'][base['index']]['source'];limits[image]=512
            for i,image in enumerate(doc.get('images',[])):
                name=image.get('uri')
                if not name or name.startswith('data:'):continue
                path=(model.parent/name).resolve()
                check(path.is_relative_to(out)and path.suffix=='.png','fixture preview needs an in-root PNG')
                with path.open('rb')as stream:header=stream.read(24)
                check(header[:8]==b'\x89PNG\r\n\x1a\n'and len(header)==24,'fixture PNG header')
                width,height=struct.unpack_from('>II',header,16)
                if max(width,height)<=limits[i]:continue
                output=temporary/f'{len(jobs)}.png'
                jobs.append({'file':path.relative_to(out).as_posix(),
                             'output':output.relative_to(out).as_posix(),'max_size':limits[i]})
                targets.append((output,path))
        if jobs:
            manifest=out/'.texture-jobs.json'
            try:
                manifest.write_text(json.dumps({'schema':1,'textures':jobs}))
                subprocess.run([str(Path(codec).resolve()),str(manifest),str(temporary/'results.json')],check=True)
                for source,target in targets:source.replace(target)
            finally:manifest.unlink(missing_ok=True)
    return len(jobs)


def build(scene, character, native, out, codec=None):
    out=output_root(out)
    check(not out.exists(),'fixture destination already exists')
    out.mkdir(parents=True)
    project,scene_asset=copy_model(scene,out/'assets'/'scene')
    _,character_asset=copy_model(character,out/'assets'/'character')
    resized=compact_textures(out,codec)if codec else 0
    project=copy.deepcopy(project)
    project.update(name='God of War III: OLY100 + Kratos regression',
                   content_revision='gow3-scene-character-bind-v1',assets='assets')
    systems=project['clients'][0]['systems']
    entities=systems['entities']
    for entity in entities:
        if entity.get('geometry'):
            entity['geometry']['asset']='scene/'+scene_asset
    character_id=max(e['id']for e in entities)+1
    entities.append({'id':character_id,'name':'Kratos native bind pose',
                     'transform':{'translation':[155,-26,-312],'scale':[3.28,3.28,3.28]},
                     'geometry':{'kind':'scene','asset':'character/'+character_asset}})
    # The selected OLY100 floor intersects x=155,z=-312 at y=-26. Scale is an
    # explicit preview convention inherited from the hall demo, not calibration.
    project['scene3d']['camera']={'eye':[163,-20,-326],'target':[155,-22,-312],
                                'up':[0,1,0],'fov_y_degrees':55,'near':.1,'far':2000}
    # Preserve any project environment/probe image dependencies.
    scene_path=Path(scene).resolve()
    scene_data=json.loads(scene_path.read_text())
    asset_root=(scene_path.parent/scene_data['assets']).resolve()
    def environment_image(name):
        candidate=(asset_root/name).resolve()
        check(candidate.is_relative_to(asset_root) and candidate.is_file(),'fixture environment dependency escapes or is missing')
        dest=out/'assets'/'scene'/name;dest.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(candidate,dest)
        return 'scene/'+name
    env=systems.get('environment',{})
    if env.get('hdr'):env['hdr']=environment_image(env['hdr'])
    for entity in entities:
        probe=entity.get('reflection_probe')or{}
        if probe.get('source'):probe['source']=environment_image(probe['source'])
    from desktop_project import validate
    validate(project)
    (out/'project.json').write_text(json.dumps(project,indent=2))
    inputs={}
    for name,path in native.items():
        path=Path(path).resolve();check(path.is_file(),'missing native fixture input')
        destination=out/'native'/name;destination.parent.mkdir(parents=True,exist_ok=True)
        shutil.copyfile(path,destination)
        inputs[name]={'bytes':path.stat().st_size,'sha256':digest(destination)}
    copy_reports(scene,character,out)
    (out/'README.md').write_text('''# Local GoW III regression fixture

One OLY100 scene and one Kratos character. Private owner-supplied content;
never commit, package or distribute this directory.

`project.json` uses ordinary shared engine glTF/material/scene APIs. Kratos is
in a static native bind pose, including native rigid attachments and texture
masks. Native clips, compressed skin weights and cloth simulation are not
exported. Source shader/material equivalence is not claimed. The scene-to-
character scale is an explicit preview convention, not a physical calibration.
Base-color previews are capped at 512 pixels and detail maps at 256 to fit the
default prepared-asset budget. The native inputs retain their original mips.

`native/` retains only OLY100.WAD, its OLY100.TS stream, and R_HERO00.WAD for
future parser tests/reconversion. No full disc, other levels, movie/audio
archives, alternate outfits or stand-in character is required.

Run from the repository with the current release player:

    target/release/crysirror run local/work/gow3/scene-character-regression/project.json

Verify every retained input/dependency with:

    python3 tools/gow3_fixture.py verify local/work/gow3/scene-character-regression

The generator only copies files. Source pruning is a separate owner operation
performed after compilation, load/render acceptance and dependency verification.
''')
    manifest={'schema':1,'native_inputs':inputs,'character_entity':character_id,'resized_images':resized,
              'character_pose':'static_bind','native_animation':False,'files':{}}
    write_receipt(out,manifest)
    return verify(out)


def verify(root):
    root=Path(root).resolve();manifest=json.loads((root/'fixture.json').read_text())
    check(manifest.get('schema')==1,'unknown fixture schema')
    for name,receipt in manifest['files'].items():
        path=(root/name).resolve()
        check(path.is_relative_to(root) and path.is_file(),'fixture file escapes or is missing')
        check(path.stat().st_size==receipt['bytes'] and digest(path)==receipt['sha256'],
              'fixture file fingerprint mismatch: '+name)
    present={p.relative_to(root).as_posix()for p in root.rglob('*')if p.is_file()}
    check(present==set(manifest['files'])|{'fixture.json'},'unrecorded fixture files')
    return {'files':len(manifest['files']),'bytes':sum(r['bytes']for r in manifest['files'].values()),
            'native_inputs':list(manifest['native_inputs']),'verified':True}


def main():
    ap=argparse.ArgumentParser(description=__doc__)
    sub=ap.add_subparsers(dest='command',required=True)
    b=sub.add_parser('build');b.add_argument('output',type=Path)
    b.add_argument('--codec',type=Path,required=True)
    for flag in ('scene','character','level-wad','level-ts','character-wad'):b.add_argument('--'+flag,type=Path,required=True)
    v=sub.add_parser('verify');v.add_argument('output',type=Path)
    args=ap.parse_args()
    if args.command=='verify':result=verify(args.output)
    else:result=build(args.scene,args.character,{'OLY100.WAD':args.level_wad,'OLY100.TS':args.level_ts,
                                              'R_HERO00.WAD':args.character_wad},args.output,args.codec)
    print(json.dumps(result,indent=2))


if __name__=='__main__':main()
