"""Render the three live cameras with the release player and check the captured state."""
import argparse
import json
import subprocess
from itertools import combinations
from pathlib import Path

import numpy as np
from PIL import Image


def main():
    demo = Path(__file__).resolve().parent
    repo = demo.parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=repo / "target/raincourt")
    parser.add_argument("--player", type=Path, default=repo / "target/release/crysirror")
    parser.add_argument("--script",type=Path,default=demo/"game.ts")
    args = parser.parse_args()
    target = args.output.resolve()
    target.mkdir(exist_ok=True, parents=True)
    assets = demo / "assets"
    for stone in ("mossy_sandstone", "mossy_stone_wall"):
        for channel in ("Diffuse", "nor_gl"):
            with Image.open(assets / f"{stone}_{channel}.jpg") as tex:
                assert tex.size == (4096,4096)
    layout = json.loads((assets/"layout.json").read_text())
    assert len(layout["cells"]) == 4 and sum(c["groundcover"] for c in layout["cells"]) >= 20_000
    eyes = [[8.2,3.15,18.2],[-6.9,1.46,12.05],[-3.1,3.5,2.65]]
    previews = []
    for i, name in enumerate(["hero", "shot2", "shot3"], 1):
        image = target / f"{name}.png"
        command = [str(args.player), "render", str(demo/"project.json"), "--script", str(args.script),
                   "--renderer", "raster", "--asset-cache-mib", "1200", "--frames", "18",
                   "--data-dir", str(target/"state"), "--output", str(image)]
        if i > 1:
            track = target / f"shot{i}-keys.json"
            track.write_text(json.dumps([dict(frame=2, down=[str(i)], pressed=[str(i)]), dict(frame=3, down=[])]))
            command.extend(["--input-track", str(track)])
        report = json.loads(subprocess.check_output(command))
        (target/f"{name}-final.json").write_text(json.dumps(report, indent=2))
        assert report["runtime"]["frame"] == 18
        assert report["runtime"]["tasks"]["resident_bytes"] > 0
        world = report["runtime"]["engine"]["world"]
        assert np.allclose(world["view"]["camera"]["eye"], eyes[i-1], atol=.002)
        assert sum(v["loaded"] is not None for v in world["streaming"].values()) == [3,4,4][i-1]
        assert all(a["error"] is None for a in report["runtime"]["tasks"]["assets"])
        with Image.open(image) as picture:
            assert picture.size == (1920,1080)
            preview = np.asarray(picture.convert("RGB").resize((160,90)), dtype=float)
        assert np.std(preview) > 18, f"{name} lacks image detail"
        previews.append(preview)
    for left, right in combinations(previews, 2):
        assert np.mean(np.abs(left-right)) > 15, "two camera buttons returned similar images"
    print(f"Three distinct 1920 × 1080 camera renders passed in {target}")


if __name__ == "__main__":
    main()
