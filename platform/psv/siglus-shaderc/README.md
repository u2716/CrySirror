# Siglus Vita shader compiler

This is the original `siglus_rs` compiler utility for the integrated Siglus
renderer. It embeds the Cg programs and includes from
`crates/crysirror-siglus-integrated/src/native_vm/render/vita/shaders`.
The renderer embeds the existing GXP programs from that same directory; normal
game startup does not invoke a shader compiler.

Build the utility explicitly with VitaSDK and cargo-vita:

```sh
sh platform/psv/siglus-shaderc/compile-shaders.sh build
```

The resulting `siglus_vita_shaderc.vpk` is in this utility's target directory.
Manually run it on a Vita or Vita3K installation with the owner's
`ur0:data/libshacccg.suprx`. It writes the programs and `report.txt` under
`ux0:data/siglus_shaderc`. The compiler module and SDK binaries are not bundled.
After copying that output directory to the host, import it explicitly:

```sh
sh platform/psv/siglus-shaderc/compile-shaders.sh import /path/to/siglus_shaderc
```

Import requires a completed successful report and every expected program before
replacing embedded files. The script never starts or stops an emulator.
This utility is a standalone workspace and is not built by the desktop player.
