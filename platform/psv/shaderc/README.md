# Shared PS Vita shader compiler

Compiles CrySirror-owned native Cg shaders with `libshacccg.suprx` on Vita or
Vita3K. This utility opens no audio device and never starts a game. It is a
standalone workspace, outside the desktop build.

## Export and build

From the repository root, export the current shared 2D, reduced 3D and BVH
programs and their binding manifest:

```sh
cargo run --release -p crysirror-shader-tools -- psv /tmp/crysirror-psv-shaders
cd platform/psv/shaderc
CRYSIRROR_SHADER_SOURCES=/tmp/crysirror-psv-shaders cargo +nightly vita build vpk -- --release
```

`programs.txt` selects the exact exported source set. The utility does not scan
unlisted files. VitaSDK and the nightly `rust-src` component are build inputs.
Set `CARGO_TARGET_DIR` to an existing release target directory to reuse local
build intermediates.

## Compile and import

Install the generated VPK and run **CrySirror Shader Compiler** (`CRYSHADR1`).
The compiler module must exist at `ur0:data/libshacccg.suprx`. It is not bundled.
The utility writes under `ux0:data/crysirror_shaderc`:

| File | Purpose |
| --- | --- |
| `report.txt` | Compiler diagnostics and completion result for this invocation. |
| `<program>.gxp` | Successful native program. A failed attempt removes its previous GXP. |
| `<program>.cg` | Exact source used to compile that program. |
| `<program>.receipt` | Compiler version, program size, precision and optimization options. |

`ux0:data/crysirror_shaderc-selection.txt` selects one program stem. Each
invocation compiles at most one selected program and then retires the process.
An absent selection chooses the first program. The output directory is retained
across invocations. Fatal compiler errors end the invocation.

The explicit host driver runs the full exported catalog without opening a
window or starting a game:

```sh
python3 tools/compile_psv_shaders.py EXPORTED_DIR COMPILER.vpk OUTPUT_DIR \
  --vita3k /path/to/Vita3K --fs-root /path/to/fs \
  --config /path/to/config.yml --log-file /path/to/vita3k.log --jobs 4
```

The driver creates temporary compiler-only filesystems, copies the local compiler
module, reads firmware through local symlinks, and removes its temporary files
when finished. It installs the compiler VPK in the temporary filesystem
and forces the SDL offscreen video driver and dummy audio driver. It uses a
fresh compiler process per program, disables emulator logging, limits each
run to 90 seconds, stops at log
limits or less than 2 GiB free space, and reuses matching completed artifacts.
It never truncates the supplied global log or terminates an unrelated process.
`--program STEM` selects a particular program. `--jobs` defaults to one.
Failed/missing programs are listed in `OUTPUT_DIR/missing.json`.

The export manifest selects the optimization level per program: filtered and
premultiplied-filter fragments and the ray fragment use level 1, and
other stages use level 3. Fast math, precision and integer shortcuts stay off.
The driver injects `crysirror_shaderc-optimization.txt` in its private filesystem
and admits cached outputs only when their receipt matches that level.
`--optimization 0|1|2|3` explicitly overrides the policy for diagnosis; outputs
with a different policy are rejected by the normal importer.

After copying the output directory to the host, admit the complete source set:

```sh
cargo run --release -p crysirror-shader-tools -- import-psv \
  /tmp/crysirror-psv-shaders /path/to/crysirror_shaderc crates/crysirror-shaders
```

Import validates every source, receipt, GXP header and byte count before
replacing repository outputs. It writes the compiled catalog and the binding
manifest under `crates/crysirror-shaders/native/psv`. Compilation establishes
neither GXM pipeline linkage nor visual/performance parity.

The host shader ABI is documented in
[the native program reference](../../../crates/crysirror-shaders/native/psv/README.md).
