# RFVP native runtime

Mode: `integrated`. Complete maintained RFVP source, including native image,
text, primitive composition, software raster and mixer modules. The default
`no_std` + `host-session` profile injects resources, clocks and output services;
its mutable state is restricted to one fixed owner thread. `host-session`
requires Rust std for thread-local isolation; it is not a console build claim.
No bundled font or native output device is used by this profile.

The host contract, entry points, encoding/save policy, bounds and platform
matrix are in [the integrated adapter](../crysirror-rfvp-integrated/README.md).
`nfs` / `smb` are optional, default-off features exposing `host_api::network`.
`SOURCE.json` identifies the source and local host seams; MPL-2.0 notices apply.
