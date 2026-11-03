# Desktop player and project guide

The default workspace target is `crysirror-desktop`, binary `crysirror`.
Build locally with the locked dependencies and release profile:

```sh
cargo build --release --locked
```

All native VM adapters, scripting and resource providers are linked by this
build. Desktop features now select device options: `audio`, `gamepad` (defaults)
and optional `acoustics-obvhs`. `--no-default-features` disables audio output and
gamepad support; it does not remove native adapters. The minimum Rust version is
declared in the root Cargo.toml. OS graphics/input/audio libraries remain native
build prerequisites. Windows calls the executable `crysirror.exe`.

## Play an installed game

```sh
./target/release/crysirror play /path/to/game
./target/release/crysirror play /path/to/game --detect-only
./target/release/crysirror play /path/to/game --headless --frames 1200 \
  --capture /tmp/game.png
```

The path may be a game directory or a file inside it. The player detects the
native profile, builds its project and installs capability-scoped resource
mounts internally. No hand-written bootstrap JSON or manual bulk extraction is
needed for supported layouts. Unknown formats, protection profiles and native
operations can still fail explicitly. See [native adapters](NATIVE_ADAPTERS.md)
for profile and ownership boundaries.

`play` supports `--data-dir`, `--font`, `--no-audio`, `--input-track`, `--frames`,
`--capture`, `--headless`, `--detect-only` and `--allow-borderless`. Headless mode
creates no window/audio device but capture still requires the renderer.
Without a subcommand the executable attempts to play the game beside itself.

Windowed games retain their OS title bar unless borderless behavior is allowed.
macOS menus use the screen menu bar; Windows uses native window menus; Linux
currently uses the engine menu fallback. An offscreen capture cannot verify OS
menu presentation or physical input.

## Run and inspect an engine project

```sh
./target/release/crysirror run examples/desktop/project.json --no-audio
./target/release/crysirror inspect examples/desktop/project.json
./target/release/crysirror trace examples/desktop/project.json --frames 120 --press-at 1
./target/release/crysirror render examples/desktop/project.json --output /tmp/scene.png
./target/release/crysirror gpu-info
```

| Command | Purpose |
| --- | --- |
| `run` | Windowed project; supports font/state overrides, scripts, capture and profiling. |
| `render` | Offscreen GPU PNG; supports bounded warm-up frames, step/input tracks, scripts and consecutive movie-frame captures. |
| `trace` | Engine/VM ticks and service diagnostics without a window/GPU/audio device. |
| `validate` | Project schema/data validation; not complete asset or game compatibility. |
| `inspect` | Project data and the executable's actually registered adapters. |
| `gpu-info` | Adapter features, enabled device capabilities and implemented rendering selections. |
| `script-types` | Generate project-bound script declarations from the class registry. |
| `shader-check` | Validate a scoped custom WGSL surface asset without a GPU. |
| `puppet-import` | Convert supplied E-mote data to engine puppet assets; native format limits apply. |

Use `crysirror <command> --help` for that build's option definitions. `render`
and `trace` accept repeatable zero-based `--press-at` indices and JSON
`--input-track` input. `run`, `render` and `trace` accept JS/TS `--script` sources
with `--script-client` selecting their project client. Offline `buildProject()`
recipes use the separate [authoring tool](AUTHORING.md).

`run` and `render` select `--renderer raster`, `compute-raytrace` or
`hardware-raytrace`. Hardware queries require supported device features/limits;
unsupported devices fail with a reason rather than silently switching paths.
Capabilities, encoded work, successful pixels and performance measurements are
separate evidence. See [architecture](ARCHITECTURE.md) and the
[frame/profiling guide](FRAME_PIPELINE.md).

F5/F9 request framework save/restore, F12 captures, and Escape exits the host.
Native adapters may reject unsupported snapshot states. These shortcuts do not
promise original-game save compatibility; see [save boundaries](SAVE_FORMAT.md).

## Project data and fonts

Schema-2 projects name an asset root and client instances. Each client's resource
paths remain scoped to its capability root. `crysirror-script-v1` data actions
are separate from original native script formats; native adapters run their own
instruction sets. `ClientSpec.systems` declares shared World/Layer/UI state.
See [engine systems](ENGINE_SYSTEMS.md) and [generated data/API declarations](api/README.md).

Projects can declare licensed fonts in `project.fonts`. Development `--font`
reads a caller-supplied face; it is not a portable asset declaration. G00 and
other native image formats use shared decoder registries; adapter-specific
cuts/pivots remain native policy. No game data or fonts are bundled.

`--data-dir` overrides state storage. Otherwise the host selects the platform's
user-data location under CrySirror; project fingerprints or installed-game
identity distinguish state. Change `content_revision` when asset semantics
change. Logical relative asset references survive moving the project together.
Budgets and supported data fields are defined by code/API validation, rather
than a second independently maintained limit table here.

## Packaging

```sh
python3 tools/desktop_project.py examples/desktop/project.json --assets
python3 tools/package_game.py --project examples/desktop/project.json \
  --executable target/release/crysirror --target linux --output dist/CrySirror-linux.zip
```

Use an executable built for the requested target. The packager checks its format
and explicit resources; it does not compile/sign the host, test target execution
or prove dependency/asset licensing completeness. `--data-only` replaces
`--executable`/`--target` when distributing project data without a runtime.
See [NOTICE.md](../NOTICE.md) for component notices and [icons](../icon/README.md)
for prepared platform icon assets.

## Local validation

`python3 tools/verify.py` runs Python tests plus release Cargo checks/tests.
`--gpu` explicitly includes the ignored GPU smoke test. Use local checks; do not
enable GitHub workflows. Test commands, failures, platform evidence and bounded
game milestones belong in dated [STATUS.md](../STATUS.md) entries.
