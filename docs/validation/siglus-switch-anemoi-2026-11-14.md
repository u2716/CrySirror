# Siglus Switch startup comparison: Anemoi

## Environment and scope

| Item | Value |
| --- | --- |
| Date | 2026-11-14 |
| Host | macOS, Apple M5 |
| Emulator | Ryujinx Canary 1.3.351, Vulkan/MoltenVK |
| Game | Matching Anemoi Gameexe.dat and Scene.pck SHA-256 hashes for both runs |
| Sessions | Sequential runs, separate emulator profiles and saves, same existing resource files |
| CrySirror NRO SHA-256 | `b6b78eebcb18da0d2d9b081ba11f532ab3f3e32c8341429083817a27b0b3c07e` |
| siglus_rs NRO SHA-256 | `8f560edeaddc19a580be3681abba6e7dca800b7bc6b75675538b14b07a9d3150` |

## Results

| Measurement | siglus_rs | CrySirror |
| --- | --- | --- |
| First 600-frame period | 57.3 FPS | 58.2 FPS |
| Subsequent 600-frame periods through frame 3600 | 59.4–60.0 FPS | 59.6–60.0 FPS |
| First 60 frames with private emulator game caches and saves reset | 40.9 FPS | 41.7 FPS |
| Next 60-frame period | 59.9 FPS | 60.0 FPS |
| Generated native global.sav | 22,104 bytes | 22,104 bytes |
| Upstream global-save container reader | Decoded, 19,630 payload bytes | Decoded, 19,630 payload bytes |

The initial unmodified packages ran for 75 seconds each. The cold-start check
used temporary C shells reporting every 60 frames instead of every 600 frames,
linked against the existing release libraries and shaders. Each ran for 26
seconds. Those logging-only changes were not added to project sources.

The measured startup did not reproduce a sustained 20–40 FPS gap between the
two players. Both had a slower first 60-frame period. This comparison does not
identify the exact cause of that initial delay.

## Limits

These are main-loop throughput measurements on an emulator, not real Switch
or Vita hardware results. They do not certify title rendering, full gameplay,
audio fidelity or a complete save/load round trip. Reading the native save
container confirms its format, not restoration of every VM field. No desktop
screen capture was used.
