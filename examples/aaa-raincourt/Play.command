#!/bin/sh
set -eu
demo=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH='' cd -- "$demo/../.." && pwd)
player="${CRYSIRROR_PLAYER:-$repo/target/release/crysirror}"
python="${CRYSIRROR_PYTHON:-python3.11}"
"$python" "$demo/prepare.py"
script="$demo/canyon.ts"
if [ "${1:-}" = "--court" ]; then
    shift
    script="$demo/game.ts"
fi
exec "$player" run "$demo/project.json" --script "$script" --renderer raster --asset-cache-mib 1200 "$@"
