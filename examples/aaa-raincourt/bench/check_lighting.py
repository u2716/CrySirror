"""Isolate the effects of the masonry occlusion and shaft depth using the normal release player."""
import argparse
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import tempfile

import numpy as np
from PIL import Image

demo = Path(__file__).resolve().parents[1]
repo = demo.parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True,exist_ok=True)
    assets = demo/"assets"
    scene = (demo/"game.ts").read_text()
    control = json.loads((assets/"sanctuary.gltf").read_text())
    # Keep the same primitives, UVs, factor, original scanned AO, 4K colour and normal.
    for m in control["materials"]:
        original = m["pbrMetallicRoughness"].get("metallicRoughnessTexture")
        if original:
            m["occlusionTexture"] = deepcopy(original)
        else:
            m.pop("occlusionTexture", None)
    open_words = "["+",".join([str((2**24-1)/2**24)]*4)+"]"
    report = {}
    with tempfile.NamedTemporaryFile(prefix="sky-control-",suffix=".gltf",dir=assets,mode="w") as model:
        json.dump(control,model); model.flush()
        variants = {"final":scene,
                    "original-ao":scene.replace('"earth","sanctuary"', f'"earth","{Path(model.name).stem}"'),
                    "open-shafts":scene.replace("depth.slice(0,4),depth.slice(4,8)",open_words+","+open_words)}
        assert len(set(variants.values())) == 3
        images = {}
        for name,variant in variants.items():
            script = output/(name+".ts")
            script.write_text(variant)
            image = output/(name+".png")
            cmd=[repo/"target/release/crysirror","render",demo/"project.json","--script",script,
                 "--renderer","raster","--asset-cache-mib","1200","--frames","18","--data-dir",output/"state",
                 "--output",image]
            result=json.loads(subprocess.check_output(cmd))
            view=result["runtime"]["engine"]["world"]["view"]
            assert abs(view["environment"][1]["exposure"]+.31)<1e-4
            assert result["stats"]["irradiance_volume"]["probes"]==0
            assert np.allclose(view["camera"]["eye"],[8.2,3.15,18.2])
            images[name]=np.asarray(Image.open(image).convert("RGB"),dtype=np.int16)
        for name in ("original-ao","open-shafts"):
            delta=(images[name]-images["final"]).mean(axis=2)
            report[name]={"pixels_darker_by_over_four":int((delta>4).sum()),
                          "mean_signed_rgb_delta":float(delta.mean())}
            assert report[name]["pixels_darker_by_over_four"]>1000, name
    (output/"comparison.json").write_text(json.dumps(report,indent=2))
    print(report)


if __name__=="__main__":
    main()
