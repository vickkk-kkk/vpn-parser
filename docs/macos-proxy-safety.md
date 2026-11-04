# macOS system proxy: experimental safety changes

This branch contains unverified source changes, not a signed or device-tested release.
HTTP and HTTPS settings are captured separately for the selected network service.
The implementation refuses authenticated proxies because their credentials cannot be
captured by `networksetup`. Older snapshots without the macOS fields are not applied.

A failed restore retains the snapshot in process memory and blocks another start;
this is not durable across a crash or process exit. When the current endpoint is
not the managed endpoint or the exact original state, restoration stops rather
than overwriting an external change. `networksetup` read and write are not atomic,
so a separate program can still change settings between the check and the write.
A partial restore with an ambiguous state may require manual recovery. If an
original disabled endpoint had no address, the enabled state can be restored but
the previously empty hidden address fields are not restored. Automatic rearming
while a macOS snapshot is pending is deliberately blocked to preserve that snapshot.

Only standalone source-level simulation and stub type checking have been run.
Full Cargo compilation, actual `networksetup` output/permissions, partial failure,
network switching and macOS device behavior require separate validation. Do not
claim production readiness or that system proxy restoration is race-free.
