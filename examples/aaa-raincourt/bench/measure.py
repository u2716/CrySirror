"""Replay cold and warm laps on the release offscreen renderer and retain individual frame stages."""
import argparse
import json
from pathlib import Path
import subprocess

here = Path(__file__).resolve().parent
demo = here.parent
repo = demo.parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--script", type=Path, default=demo/"game.ts")
    parser.add_argument("--live-probes", action="store_true")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    first = json.loads((here/"tour.json").read_text())
    warm = [dict(frame=180,pressed=["1"],down=["1"]), dict(frame=181,down=[])]
    warm += [dict(e,frame=e["frame"]+180) for e in first]
    start_probes = [dict(frame=0,down=["4"],pressed=["4"])] if args.live_probes else []
    for name, frames, events in [
        ("walking",360,first+warm),
        ("cycle",2628,[dict(frame=1,down=["t"],pressed=["t"]),dict(frame=2,down=[])])
    ]:
        track=output/(name+"-keys.json")
        track.write_text(json.dumps(start_probes+events))
        command=[repo/"target/release/raincourt-frame-bench",demo/"project.json",args.script,
                 output/"state",track,str(frames)]
        with (output/(name+".json")).open("w") as result:
            subprocess.run(command, stdout=result, check=True)


if __name__ == "__main__":
    main()
