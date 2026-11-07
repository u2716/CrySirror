# BGI / Ethornell

The desktop player uses `crysirror-bgi-integrated`, registered as `bgi`.
Its current source pin is `3ee43dab43854e2f9c614dd1d26213e17911d877`;
see the [crate contract and source record](../crates/crysirror-bgi-integrated/README.md)
and [update validation](validation/bgi-upstream-sync.md).
It contains the BP interpreter, archive and image readers, graphics, text,
effects, audio and native-save implementation. A session worker produces
frames and PCM; CrySirror owns the desktop window, input and output devices.

```sh
crysirror play /path/to/game
```

Detection selects the original BP bootstrap and also recognizes game resources
under `Install/`. Archives are read in place without an external extraction or
bootstrap step. Installation-media layouts are exposed through an installed
resource view. Configuration and native-save writes use a per-client writable
overlay, with subsequent native reads seeing those files first.

The separate `crysirror-bgi` and `crysirror-bgi-vm` capability-extraction crates
are preserved; the default desktop registers the integrated runtime.

See [BGI and SoftPAL](NATIVE_ADAPTERS.md#bgi-and-softpal) for integration details
and [desktop launch options](DESKTOP_API.md) for headless rendering and save
directory selection.
