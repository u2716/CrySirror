# crysirror-amanatsu

Onboarding mode: **adopt** (ADR 0102). 甘夏ろけーしょん (ILLGAMES
`AmanatsuLocation`, Unity 6000.0.65f1, IL2CPP). Game logic is ported from the
original program (method bodies from `GameAssembly.dll` under IDA, type and
method names from `global-metadata.dat` through Cpp2IL); all assets are read
from the user's installation through `crysirror-unity` (UnityFS bundles,
serialized format 22, type trees). No game data, metadata dumps or code are
distributed.

## Implemented

- `adv.rs`: ADV scenarios — `ILLGAMES.ADV.ScenarioData` MemoryPack bytes of
  the scenario MonoBehaviours in `lib/adv/scene` (`ScenarioCommand`: hash,
  version, multi, `Command`, string arguments; the `Command` names).

## Validation

`AMANATSU_ROOT=<game> cargo test --release -p crysirror-amanatsu --test owner_corpus -- --ignored --nocapture`:
2,678 scenarios (35,224 commands, 45 command kinds in use) decode; textures
and meshes of `lib` decode through the shared readers (see
`crysirror-unity` README). Scope still to port: [SATD.md](SATD.md).
