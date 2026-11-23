"""Fetch only the pinned data files from the Poly Haven CC0 asset library."""
from concurrent.futures import ThreadPoolExecutor
from hashlib import md5
import json
from pathlib import Path
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ASSETS = HERE / "assets"


def fetch(entry):
    relative, asset = entry
    path = ASSETS / relative
    path.parent.mkdir(exist_ok=True, parents=True)
    def matches(candidate):
        return candidate.is_file() and candidate.stat().st_size == asset["size"] and md5(candidate.read_bytes()).hexdigest() == asset["md5"]
    if matches(path):
        return
    with tempfile.TemporaryDirectory(dir=path.parent) as scratch:
        temp = Path(scratch) / "download"
        subprocess.run(["curl", "-fLsS", "--retry", "3", "--max-time", "240",
                        "-A", "CrySirror/1.0 (local demo asset preparation)",
                        "-o", str(temp), asset["url"]], check=True)
        if not matches(temp):
            raise ValueError(f"Source checksum mismatch for {relative}")
        temp.replace(path)


def prepare_sources():
    with ThreadPoolExecutor(6) as workers:
        list(workers.map(fetch, json.loads((HERE / "sources.json").read_text()).items()))


if __name__ == "__main__":
    prepare_sources()
