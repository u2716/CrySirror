# SoftPAL integrated runtime

Mode: `integrated`.

## Optional Vita output

`vita-gxm` retains the original GXM scene driver, its borrowed scene ABI and
vita2d's existing GXP programs. It is off by default. See the
[native output contract](native/vita/README.md) for ownership and dependencies.
The source and build wiring are present; a complete CrySirror console session
has not been connected or validated by this import.

## Optional network resources

`nfs` and `smb` are off by default. The desktop forwards its matching features.
The session worker receives the existing client-scoped storage capability and
performs resource reads there; the host event loop never reconnects or reads
native resources during a VM tick. Native file cursors share bounded block
caches without staging game data. Save data remains in the local user root.
Network gameplay and latency were not exercised against a real server in this
change.
