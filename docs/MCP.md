# Local MCP project tools

Run from the repository root with Python 3.11 or newer:

```sh
python3 tools/mcp_server.py --project examples/desktop/project.json
```

The stdio JSON-RPC server reserves stdout for protocol responses and implements
MCP protocol `2025-06-18`. It exposes project tools without an HTTP listener or
arbitrary shell-execution method.

## Available tools

| Tool | Requirement | Operation |
| --- | --- | --- |
| `project_inspect` | Project | Read project data; a configured host can supply its actual adapter registry. |
| `project_validate` | Project | Validate data and explicit assets. |
| `project_replace` | `--allow-write` | Validate and replace the selected project using `expected_sha256`. |
| `shader_validate` | `--executable` | Validate a scoped WGSL surface asset through the host, without a GPU. |
| `engine_trace` | `--executable` | Run fresh-session CPU engine/VM ticks and return state/events. |
| `scene_capture` | `--executable` | Run a fresh-session GPU capture with raster or compute-raytrace. |
| `game_package` | `--executable` | Package the project with the already-built host. |

```sh
cargo build --release --locked
python3 tools/mcp_server.py --project examples/desktop/project.json \
  --executable target/release/crysirror --allow-write
```

`engine_trace` and `scene_capture` accept `frames` (1..3600), `step_ms`
(0..60000) and unique zero-based `press_at` indices inside the requested frame
range. Enter pulses reach retained UI before native VMs. Trace includes actual
World/UI/service state. Capture requires a working renderer; it does not emit
placeholder images on failure. The MCP capture selector currently exposes
raster/compute, separately from the desktop CLI's hardware-raytrace option.

These tools start new sessions from project data. They do not attach to an
existing game window, hot-reload it or provide a live debugger. Validation and
inspection do not establish game compatibility. Project systems and asset
inventory respect client capability roots; distributed fonts must be supplied
with appropriate rights. See the [desktop guide](DESKTOP_API.md) for host
commands and [API reference](api/README.md) for engine interfaces.
