# BGI / Ethornell

```sh
./target/release/crysirror play "/path/to/game"
```

CrySirror opens BP scenarios and ARC resources in place. Features include CBG
images, character composition, text, choices, effects, music, sound, voice and
original game saves. No extraction or separate bootstrap project is required.

## Game directories

Both installed directories and supported installation-media layouts with an
`Install/` directory are recognized. Keep the executable, archives and companion
files together so startup and resource identification can use them.

Configuration and save files are written to CrySirror's user-data directory.
Later game reads see these writable files before the original resources.
Use `--data-dir` to select a different user-data location.

## Network storage

SMB and NFS game directories are supported when the player is built with the
matching optional `smb` or `nfs` feature. Resources stay on the server and saves
stay local.

See the [desktop guide](DESKTOP_API.md) for launch options, fonts, credentials
and network paths, or the [engine compatibility guide](NATIVE_ADAPTERS.md) for
other engines.
