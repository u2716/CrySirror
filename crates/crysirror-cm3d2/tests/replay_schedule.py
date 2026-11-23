# SPDX-License-Identifier: MPL-2.0
"""Replay the ordinary day-2 schedule UI with a private copy of an owner-provided slot."""
import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path

from PIL import Image, ImageChops


def point(native, suffix):
    matches = [h["rect"] for h in native["hits"] if h["path"].endswith(suffix)]
    assert len(matches) == 1, (suffix, matches)
    left, top, right, bottom = matches[0]
    return [int((left + right) / 2), int((top + bottom) / 2)]


def press(frame, xy, button="mouse_left"):
    return {"frame": frame, "pointer": xy, "pressed": [button], "down": [button]}


def tap(frame, xy, button="mouse_left"):
    return [press(frame, xy, button), {"frame": frame + 1, "pointer": xy, "down": []}]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("player", type=Path)
    parser.add_argument("install", type=Path)
    parser.add_argument("day2_slot", type=Path, help="daytime, one scheduled leader and one spare")
    parser.add_argument("output", type=Path, help="a new output directory outside the installation")
    args = parser.parse_args()
    out = args.output.resolve()
    assert not out.is_relative_to(args.install.resolve())
    out.mkdir(parents=True, exist_ok=False)
    slot = Path("cm3d2/local-installed/save/SaveData/SaveData000.save")

    def run(name, track, frames, seed=args.day2_slot):
        work = out / name
        dest = work / slot
        dest.parent.mkdir(parents=True)
        shutil.copyfile(seed, dest)
        track_path = work / "track.json"
        track_path.write_text(json.dumps(track))
        command = [args.player.resolve(), "play", args.install.resolve(), "--headless", "--no-audio",
                   "--frames", str(frames), "--data-dir", work, "--input-track", track_path]
        with (work / "report.json").open("w") as report, (work / "player.log").open("w") as log:
            env = {k: v for k, v in os.environ.items() if not k.startswith("CM3D2_DEBUG_")}
            subprocess.run(command, stdout=report, stderr=log, env=env, check=True)
        native = json.loads((work / "report.json").read_text())["runtime"]["sessions"][0]["native"]
        assert not native["error"], native["error"]
        return native

    def check_hover(name, neutral, track, frame, suffix):
        idle = run(name + "-rest", track + [{"frame": frame - 2, "pointer": [960, 540]}], frame)
        run(name + "-hover", track + [{"frame": frame - 2, "pointer": point(neutral, suffix)}], frame)
        rect = next(h["rect"] for h in idle["hits"] if h["path"].endswith(suffix))
        images = []
        for action in ("rest", "hover"):
            with Image.open(out / f"{name}-{action}" / "last-frame.png") as image:
                images.append(image.convert("RGB").crop(tuple(map(int, rect))))
        assert ImageChops.difference(*images).getbbox(), f"{name} hover did not change its rendered control"

    boot = []
    for frame, xy in [(90,[960,540]), (260,[960,540]), (650,[388,705]), (720,[455,226]), (760,[1163,640])]:
        boot += tap(frame, xy)
    daily = run("daily", boot, 1700)
    original = daily["schedule"]
    assert original["day"] == 2 and original["slots"][0] and not any(original["slots"][1:])
    schedule = point(daily, "/DailyButtonGroup/Schedule")
    check_hover("daily", daily, boot, 1700, "/DailyButtonGroup/Schedule")
    seq = boot + tap(1710, schedule)
    rows = run("rows", seq, 1810)
    assert not any(h["path"].endswith("/slot_1/DaytimeTask") for h in rows["hits"])
    # An empty row has no task collider. Removing the sole leader is forbidden.
    seq += tap(1815, [628,420]) + tap(1820, point(rows, "/slot_0/MaidStatus"), "mouse_right")
    unchanged = run("protected", seq, 1830)
    assert unchanged["schedule"] == original and "delete: None" in unchanged["active"]
    assert not any("SystemDialog/Base/Cancel" in h["path"] for h in unchanged["hits"])

    seq += tap(1832, point(rows, "/slot_1/MaidStatus"))
    roster = run("roster", seq, 1845)
    check_hover("roster", roster, seq, 1848, "/MaidPlate/Button")
    plate = next(h["rect"] for h in roster["hits"] if h["path"].endswith("/MaidPlate/Button"))
    left, top, right, bottom = plate
    assert abs(right - left - 419) < 1 and abs(bottom - top - 111) < 1, plate
    # Click near the portrait, outside the old 100-pixel hit area.
    seq += tap(1850, [int(left + 10), int((top + bottom) / 2)])
    assigned = run("assigned", seq, 1870)
    selected = assigned["schedule"]["slots"][1]
    assert selected and selected["maid"] != original["slots"][0]["maid"]
    seq += tap(1880, point(assigned, "/slot_1/DaytimeTask"))
    tasks = run("tasks", seq, 1895)
    task = next(h for h in tasks["hits"] if "/DaytimeTaskViewer/" in h["path"]
                and h["path"].endswith("/Main") and int(h["path"].split("/")[-2]) != selected["day_work"])
    work_id = int(task["path"].split("/")[-2])
    seq += tap(1900, point(tasks, task["path"]))
    edited = run("edited", seq, 1920)
    after = edited["schedule"]
    assert after["slots"][0] == original["slots"][0]
    assert after["slots"][1]["maid"] == selected["maid"]
    assert after["slots"][1]["night_work"] == selected["night_work"]
    assert after["slots"][1]["day_work"] == work_id

    seq += tap(1930, point(edited, "/slot_1/MaidStatus"), "mouse_right")
    dialog = run("dialog", seq, 1945)
    # The task and panel OK are still geometrically underneath this dialog.
    seq += tap(1950, point(edited, "/SchedulePanel/Ok"))
    blocked = run("blocked", seq, 1965)
    assert point(blocked, "/SystemDialog/Base/Cancel") == point(dialog, "/SystemDialog/Base/Cancel")
    assert blocked["schedule"] == after
    seq += tap(1970, point(dialog, "/SystemDialog/Base/Cancel"))
    seq += tap(1980, point(edited, "/SchedulePanel/Ok"))
    returned = run("returned", seq, 2100)
    assert returned["schedule"] == after and "step: Open" in returned["active"]
    seq += tap(2110, point(returned, "/DailyButtonGroup/Save"))
    saves = run("slots", seq, 2220)
    seq += tap(2230, point(saves, "/DataUnitParent/serialNo_0"))
    confirm = run("overwrite", seq, 2250)
    seq += tap(2260, point(confirm, "/SystemDialog/Base/Ok"))
    run("saved", seq, 2330)
    saved = out / "saved" / slot
    assert saved.read_bytes() != args.day2_slot.read_bytes()
    reloaded = run("reloaded", boot, 1700, saved)
    assert reloaded["schedule"] == after, (after, reloaded["schedule"])

    # Leave the independent reload through the ordinary daytime Next control.
    advance = boot + tap(1710, point(reloaded, "/DailyPanel/Next"))
    advance += tap(2310, [960, 540])
    prompt = run("daytime-choice", advance, 2370, saved)
    decline = next(c for c in prompt["choices"] if c["text"] == "コミュニケーションを行わない")
    left, top, right, bottom = decline["rect"]
    advance += tap(2380, [int((left + right) / 2), int((top + bottom) / 2)])
    # Declining the optional event still plays the normal daytime work summaries.
    # Click the ADV background only. The result panel waits for its own OK.
    for frame in range(2660, 8500, 40):
        advance += tap(frame, [960, 540])
    result = run("results", advance, 8510, saved)
    assert any("Result" in h["path"] and h["path"].endswith("/Ok") for h in result["hits"]), (result["active"], result["kag"])
    completed = result["schedule"]["slots"][1]
    assert completed["day_work"] == work_id
    assert completed["day_work_plays"] == after["slots"][1]["day_work_plays"] + 1
    print(f'Day {after["day"]}, slot 2, character {selected["maid"]}, work {selected["day_work"]} -> {work_id}; reload and results passed.')


if __name__ == "__main__":
    main()
