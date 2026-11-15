# Native GXM shader validation

This standalone VitaSDK utility registers the checked-in GXP catalog, creates
all vertex programs and links each fragment with its documented vertex family.
It draws synthetic 64x64 cases for shared presentation, quads, five sampling
policies, transition, legacy 3D mesh lighting and packed-BVH primary/shadow rays.
It loads no games, fonts or shader compiler module.

Run from the repository root:

```sh
python3 tools/validate_psv_shaders.py OUTPUT_DIR \
  --vita3k /path/to/Vita3K --fs-root /path/to/fs \
  --config /path/to/config.yml --log-file /path/to/vita3k.log
```

`--build-only` needs only VitaSDK and produces the portable diagnostic VPK.
`--compare-only` evaluates previously captured RGBA files without compilation or
emulator execution. Pixel mismatch returns failure, including constant colours
and retained initialization markers.

The driver cross-builds optimized C, creates a VPK and a temporary filesystem,
then runs only this utility. It disables asynchronous pipeline compilation and
uses a 180-second deadline, bounded logs and a 2 GiB disk floor. Existing games
and configuration are not changed. Temporary resources are removed after the
owned process stops. The VPK has title ID `CRYSGXMV1`; the private launch folder
uses the existing shader compiler's `CRYSHADR1` inventory alias to accommodate
Vita3K's pre-configuration CLI validation. The installed compiler is untouched.

| Output | Meaning |
| --- | --- |
| `report.txt` | Native program admission, reflected attributes, patcher linkage and draw return codes. GXP byte containers may include trailing alignment beyond `sceGxmProgramGetSize`. |
| `*.rgba` | Tightly packed 64x64 RGBA8 native readback of synthetic draws. |
| `pixels.json` | Pixel oracle results when native readback is available. |
| `validator.vpk` | The generated native diagnostic application. |
| `build.log`, `emulator.log` | Bounded build/execution diagnostics. |

Return code 0 requires both API completion and the pixel comparisons. Code 2
means API admission completed but readback retained the initialization marker;
this is not pixel success. Other failures return 1 or the failing build status.

In particular, Vita3K Vulkan disables memory mapping on macOS, so its native
CPU surface-readback path can leave original memory unchanged. The diagnostic
uses a second scene boundary after the first CPU read to request surface sync
where supported. Emulator API acceptance does not establish hardware behavior,
full effect coverage, framebuffer timing or performance.
