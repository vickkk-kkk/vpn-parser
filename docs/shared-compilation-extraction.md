# Shared six-engine compilation extraction

Based on client commit `8d51b8484a0b64e1011b3a38d83b83a2fb348ee0`.

`vp_client_core::compilation::compile_configuration` is a checked, explicitly
parameterized, pure configuration API for Mihomo, Sing-box, Xray, Shoes, Meow and
VP-Core. It validates options and backend capabilities, collects diagnostics,
and refuses an artifact when an error diagnostic occurs. Backend error text is
not exposed by this entrypoint: messages exclude supplied names and credentials.
The lower-level upstream compiler API retains its existing error text and is not
suitable for unfiltered user-visible logging.

Callers provide a `CompilationContext` containing geo inventory, independent
Shoes raw-asset inventory and opaque target paths, probe URL and direct mirror
hosts. There is no default provider or filesystem inference in core. Shoes raw
inventory must not substitute the general provider's legacy two-letter country
shortcut. The compiler emits existing resource URLs but never fetches them.

Desktop wrappers retain the old compile-only behavior and return the existing
artifact even when new capability diagnostics would reject the checked path.
They deliberately discard the diagnostic sink rather than change desktop launch
behavior during extraction. Desktop executable, platform, probing and networking
logic remains outside core. Desktop and core kernel identities use exhaustive
conversions; desktop options are copied field-by-field to retain serde-skipped
runtime values. Serialization roundtrip alone would lose those values.

Compilation is not native VPN support. Neither Android VpnService nor iOS Packet
Tunnel integration is implemented by this change. Desktop supports_tun flags
must not be used as evidence of native embedded-engine availability. Upstream
protocol conversion limitations remain, including Sing-box's limited conversion
set, VP-Core pruning and Meow TUIC/WireGuard incompatibilities.

## Validation and scope

No dependency/manifest/lock change was needed for this extraction. The actual-workspace core gate passed 102 normal, non-ignored tests (76 unit + 26 integration). The current 213-package headless desktop graph compiled and eleven exact pure regression tests passed. All fifteen mobile import/editor model tests also passed against the newly extracted core, without GUI dependencies.

Static checks found all 29 desktop options fields explicitly mapped, no core filesystem/process/network/clock calls in the added compiler tree, and no legacy crate::proxy or crate::kernel paths in extracted core sources. The checked API rejects known unsupported combinations and dangling rule_sets with sanitized diagnostics; it does not automatically select a different engine.

The default golden test includes six exact serialized artifacts captured from the original desktop compilers. Each extracted output matches its captured original byte-for-byte. Fixture provenance and hashes are recorded in `../crates/vp-client-core/tests/fixtures/compiler-baseline/provenance.json`. Tests embed the fixture bytes and do not fetch or start anything.

This is one synthetic default-options configuration per engine, with explicit missing Geo inventories, fixed metadata and example-only credentials. Sing-box uses Hysteria2; the other five engines use Trojan. It is not comprehensive protocol, routing, Geo-data or platform coverage. The golden capture did not perform filesystem inventory lookup or run external engines. Pure tests additionally cover deterministic output, direct/checked parity, unsupported capability errors, field preservation, raw Shoes inventory distinction and compatibility adapters.

No application main, GUI, installer, real engine or native VPN runtime was launched by these gates. GUI compilation/rendering and Android/iOS native VPN integration remain unvalidated. Legacy desktop facades preserve old output behavior; stricter checked-API rejection is intentionally a separate contract, not proof of runtime acceptance.
