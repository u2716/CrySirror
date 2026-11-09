# SoftPAL integrated runtime

Mode: `integrated`.

## Optional network resources

`nfs` and `smb` are off by default. The desktop forwards its matching features.
The session worker receives the existing client-scoped storage capability and
performs resource reads there; the host event loop never reconnects or reads
native resources during a VM tick. Native file cursors share bounded block
caches without staging game data. Save data remains in the local user root.
Network gameplay and latency were not exercised against a real server in this
change.
