#!/bin/sh
# Build the original Siglus shader compiler, or import its completed output.
# Never launches or terminates an emulator. The owner supplies libshacccg.suprx.
set -eu
COMPILER_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$COMPILER_DIR/../../.." && pwd)
SHADERS="$ROOT/crates/crysirror-siglus-integrated/src/native_vm/render/vita/shaders"

case "${1:-build}" in
    build)
        cd "$COMPILER_DIR"
        VITASDK=${VITASDK:-/usr/local/vitasdk} cargo +nightly vita build vpk -- --release
        ;;
    import)
        OUTPUT=${2:?usage: compile-shaders.sh import COMPILER_OUTPUT_DIRECTORY}
        grep -qx 'DONE failed=0' "$OUTPUT/report.txt"
        # Check the entire expected set before replacing any embedded program.
        for source in "$SHADERS"/*.cg; do
            name=$(basename "$source" .cg)
            test -s "$OUTPUT/$name.gxp" || {
                echo "Missing compiled shader: $name.gxp" >&2
                exit 1
            }
        done
        for source in "$SHADERS"/*.cg; do
            name=$(basename "$source" .cg)
            cp "$OUTPUT/$name.gxp" "$SHADERS/gxp/$name.gxp"
        done
        ;;
    *)
        echo 'usage: compile-shaders.sh [build | import COMPILER_OUTPUT_DIRECTORY]' >&2
        exit 2
        ;;
esac
