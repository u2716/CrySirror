"""Reach all three ravine views through a continuous walk, with no camera teleport."""
from pathlib import Path
import argparse
import json
import subprocess
import numpy as np
from PIL import Image

demo=Path(__file__).resolve().parents[1]
repo=demo.parents[1]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output",type=Path)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    track=out/"route.json"
    track.write_text(json.dumps([dict(frame=1,down=["t"],pressed=["t"]),dict(frame=2,down=[])]))
    previews=[];states=[]
    for name,frame in [("hall",18),("reveal",325),("falls",607)]:
        image=out/(name+".png")
        cmd=[repo/"target/release/crysirror","render",demo/"project.json","--script",demo/"canyon.ts",
             "--renderer","raster","--asset-cache-mib","1200","--frames",str(frame),"--input-track",track,
             "--data-dir",out/"state","--output",image]
        state=json.loads(subprocess.check_output(cmd))
        (out/(name+".json")).write_text(json.dumps(state))
        world=state["runtime"]["engine"]["world"]
        assert not any(x["error"] for x in state["runtime"]["tasks"]["assets"])
        states.append(world["view"]["camera"]["eye"])
        with Image.open(image) as im:
            assert im.size==(1920,1080)
            previews.append(np.asarray(im.convert("RGB").resize((160,90)),dtype=float))
    assert states[0][2]>18 and states[1][2]<2 and states[2][0]>15
    assert previews[1].mean()>previews[0].mean()+8
    assert np.abs(previews[1]-previews[2]).mean()>15
    print("Continuous route passes at",states)


if __name__=="__main__":
    main()
