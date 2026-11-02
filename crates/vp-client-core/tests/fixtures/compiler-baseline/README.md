# Six-engine compiler baselines

These exact serialized artifacts were captured from the original desktop
compilers and successfully compared with the extracted shared core before being
promoted into the default hermetic regression test. `provenance.json` records
source commits, capture-source hashes, and each artifact's size and SHA-256.

The fixed fixture uses Hysteria2 for Sing-box and Trojan for the other five
engines, default options, fixed IDs/timestamps, an example.invalid server and a
synthetic test-only password. Probe URL and mirror-host inputs are fixed to the
original desktop constants; no request is made to them.

GeoSite and GeoIP inventories are explicitly missing. The baseline test's
thread-local hook returned before filesystem lookup or data-root creation;
the general two-letter GeoIP shortcut was retained and Shoes had no asset
paths. Core comparison supplies the equivalent explicit providers.

This is one fixture per engine, not comprehensive protocol, geo, routing,
engine-runtime or Android/iOS VPN validation. The test includes fixture bytes
at compile time, reads no environment/filesystem at runtime and reports only
the engine identity on mismatch, never artifact contents.
