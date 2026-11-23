# Custom Order Maid 3D 2 standalone

## 2026-11-23 · Unity 5.6.4p2 retail startup

The local installation identifies itself in `COM3D2x64_Data/app.info` as
KISS / CUSTOM ORDER MAID 3D 2. Its `globalgamemanagers` uses serialized file
format 17. Detection produced 20 KISS WARC mounts from GameData and
GameData_20. GameUty, SceneLogo, SceneWarning, TitleMgr and TitleCtrl in
the installed managed assembly supply the archive order, input edges, fades,
camera and title transition. Evidence and private captures live under the
ignored `target/com3d2-validation` directory.

The standard release desktop player ran headless on Apple M4 Max / Metal at
1920×1080, with audio output disabled. The input sent Return down/up at
frames 100/102 and 285/287.

| Frame | Observable result |
| --- | --- |
| 90 | SceneLogo displays the installed KISS logo. |
| 245 | SceneWarning displays the retail caution image. |
| 750 | SceneTitle has 13 image widgets, 26 club mesh renderers and BGM004.ogg selected. The cursor marks Start. |
| 795 | A subsequent Down press moves the cursor to Continue. |

The 1920×1080 Metal readback shows the courtyard, fountain, night sky and
title overlay. Pointer movement onto Config changed the focused button.
Both a same-button mouse press/release on Start and Return resumed AllMain.ks at `*title_end` and
reported the first unsupported scene at line 43, `SceneNetorareCheck`.
Playing a complete new game, the other title actions, original saves, audio
device output and equivalent lighting to the Windows player remain unverified.


The installed-data Unity test `com3d2_boot_scene_and_streamed_logo` verified
the logo scene path, external stream, dimensions and transparent/colored
pixels. An independent CM3D2 1.00 desktop regression reached SceneLogo by
frame 120. The COM3D2 texture header census found 10,922 v1010 ARGB32
containers in parts.arc and 119 more in parts2.arc. These carry encoded
images. The older v1000 layout also occurs in 48 parts.arc textures.

The release owner-corpus test `com3d2_parts_use_shared_kiss_formats` parsed
all 550 meshes in the two parts archives. The main archive contains 56
v1000, 341 v2000 and 145 v2001 meshes; the second has eight v2001 meshes.
The shared reader consumes the optional per-bone local scale of v2001 and
passes it to the exported node. The same test resolved a representative
item menu's icon reference, a material's shader and properties, a keyed
animation track, and a v1010 texture decoded to 512×1024 pixels. These are
resource checks. Character assembly and motion playback in a COM3D2 game
scene still need validation.
