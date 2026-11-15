# CrySirror showcase

A recorded flight through a neon city whose screens show live native games.
Everything you see is drawn by one CrySirror renderer in a single process:

- The intro, city, race gates, particles, lights, fog, bloom and screen-space
  reflections are a `crysirror-script-v1` scene ([showcase.ts](showcase.ts)).
- Each screen is a PBR material whose base-colour and emissive textures name a
  live engine surface (`__surface/<slot>`).
- The recorder `crysirror showcase` boots every game headless through the
  ordinary launch path, with its own adapter and renderer. Each recorded frame
  it publishes the game's composed picture into the scene client's surface
  slot.

The script owns layout, the brand intro, the FPV camera path (Catmull-Rom keys
with bank-from-acceleration roll and airframe vibration) and the HUD. The host
does not move the camera. It reads the camera the script set and the
`screen:<slot>` entity poses to spatialise each game's captured 48 kHz PCM
(distance, facing and equal-power pan), and adds a procedural intro sting and
rotor sound. Audio goes only into the file; no output device is opened.

## Assets

`assets/screen.glb` (unit quad with top-left UV origin) and `assets/logo.png`
(the CrySirror icon) are written by [make_assets.py](make_assets.py). The scene
uses no third-party assets.

## Recording

The manifest lists the local games. Keep it outside the repository, since it
names local game folders (for example under `local/`). Its fields:

```json
{
  "scene": "path/to/examples/showcase/project.json",
  "scripts": ["path/to/examples/showcase/showcase.ts"],
  "width": 3840, "height": 2160, "fps": 60, "seconds": 84, "intro_seconds": 7,
  "font": "/System/Library/Fonts/Avenir Next.ttc",
  "video_codec": "hevc_videotoolbox", "bitrate": "80M",
  "games": [
    {"engine": "KiriKiri Z", "caption": "…", "path": "~/Downloads/game-kr",
     "warmup_frames": 1100, "input_track": "tracks/generic.json"}
  ]
}
```

Game entry fields:

- `warmup_frames`: frames each game runs before the recording starts. These
  frames are neither rendered nor heard.
- `input_track`: a track in the usual headless format, indexed from the
  game's first frame.
- `pacing_ms`: a wall-clock sleep per warm-up tick, for integrated runtimes
  whose worker threads need real time.

```sh
crysirror showcase local/showcase/manifest.json --output ~/Downloads/CrySirror-Showcase-4K.mp4 \
  [--width 960 --height 540] [--seconds 20] [--start-seconds 10] \
  [--preview-dir dir --preview-every 60] [--keep-work]
```

`ffmpeg` must be on `PATH`. Video frames are streamed to it as raw RGBA and
never staged on disk. After the last frame the PCM is written as a WAV and
muxed as AAC. `--start-seconds` still simulates from time zero but records
only from the given time, which is useful for previewing one section.

Recordings contain pictures and sound produced by the games' own data. Do not
add them to this repository.
