# minori

**Mode** `adopt` · **Adapter authors** CrySirror contributors · **Player ID** `minori`

The Rust scenario interpreter reads original minori scripts and submits layers, text, audio and movies through CrySirror services. The shared resource registry can mount a PAZ bank independently of the scenario interpreter.

| Contract | Reference installations | Current scope |
| --- | --- | --- |
| `sc-next` scenario ABI | Ten Windows i386 installations with 33–52 registered commands | All ten reach the title. 夏空のペルセウス, ソレヨリノ前奏詩 and 天使の日曜日 also enter dialogue from New Game. |
| `minori/paz` archive | The same ten installations | Installed-executable parameter discovery, headerless and prefixed banks, split banks, script/image/audio decode and seekable movies. |

Use `crysirror play <game-directory>` with the executable, `scr.paz` and companion banks together. Detection identifies the scenario ABI from native command markers and scenario records. The archive reader independently locates supported key parameters in the installation. Generated projects select the Japanese language variant. Authored projects can set `language` to `"e"` for the English variant.

CrySirror contributors authored the launch detection, PAZ reader and host bindings and maintain the workspace adapter. The three adopted scenario modules retain their upstream license and attribution. [Source manifest](SOURCE.json) · [Native ABI](NATIVE_ABI.md) · [Open implementation and validation work](SATD.md) · [Retail playback evidence](../../docs/validation/minori-2026-11-21.md)
