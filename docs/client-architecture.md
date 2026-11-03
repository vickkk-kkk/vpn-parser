# Shared client architecture and verification status

## Upstream provenance

- Upstream: https://github.com/mateo-ki/vpn-parser
- Baseline commit: `cf3bfcf8fc70518e75d036bd4db72fcee8713541`
- Baseline tree: `ea5ef1ab92ed5fa621df2ac9617fba53dfb9ec70`
- Desktop package: `vpn-parser` 1.15.15, Rust 2021; default feature `gui` (eframe/egui 0.31).
- The original snapshot contained 240 files, 5,089,821 tracked bytes, and a 511-package Cargo.lock with registry/local sources and no Git dependencies.
- The baseline was a single binary package with its modules rooted in `src/main.rs`. The workspace and libraries described below are fork changes.

## Goal and boundaries

Retain the desktop egui application while extracting reusable Rust business logic. Android and iOS ultimately require native VPN lifecycle backends, not desktop process spawning or system-proxy APIs. Existing separately developed Flutter, server, Web and installer projects are not replaced by this project.

A native shell is not a working VPN implementation. Android requires a real VpnService, consent flow, protected sockets, TUN descriptor ownership and a supported embedded engine. iOS requires a Packet Tunnel Provider, entitlements/signing and a supported packet engine. Both need lifecycle/error handling, device testing and leak checks. Neither shell may report a successful VPN connection before these exist.

## Current workspace

The root workspace contains:

- `vpn-parser`: the existing desktop binary.
- `crates/vp-client-core`: shared configuration, node DTOs, subscription/protocol parsers, explicit-provider geo/rule parsing and six-engine configuration compilers.
- `crates/vp-client-mobile-ui`: shared egui configuration/import UI source and a separately testable pure import model.

`apps/android` and `apps/ios` are explicitly excluded from the root workspace and declare independent native workspaces. The initial native shells received static checks only. The subsequent mobile import model now has a passing pure-model gate; native compilation, UI rendering and VPN operation remain unvalidated.

The root Cargo.lock was reconciled using normal Cargo processing for the shared-parser extraction. Its SHA-256 at this checkpoint is:

`4f0c63aab9c029b50ed9236bba135098680289980d69e2b2121417c598a5dc74`

The shared-parser extraction has passed the actual-workspace core test gate and a Linux headless desktop compatibility compile with eight exact pure tests. A subsequent development-updater guard checkpoint compiled the same 213-package headless graph and passed eleven exact pure tests (the prior eight plus three guard tests). These are distinct scoped results, not a GUI, whole-suite, native-platform or VPN-runtime pass.

## Implemented extractions

### Configuration and rules

The upstream `src/proxy/config.rs` implementation and its 12 original tests were moved byte-for-byte to `crates/vp-client-core/src/config.rs`. The desktop file now re-exports those definitions, preserving caller paths, type identity and serialized representations.

Six integration tests in `config_compatibility.rs` exercise the actual desktop facade, stable JSON names, old DNS deserialization, migration idempotence, delimiter rejection and large-list rule ordering/deduplication. They do not start the desktop application.

### Node DTOs and subscription-text helpers

Core `models::{Node, NodeSummary, NodeGroupSummary}` preserves the exact upstream declaration block, including every field and serde behavior. Desktop `models::node` re-exports the same types.

Core `parser` contains the unchanged upstream common parsing helpers, excluding date-template expansion. Desktop `parser::common` re-exports them. The public desktop `parser::expand_url_template` implementation remains unchanged and retains its local-clock behavior outside core. Core chrono enables std/serde, not clock.

Six additional integration tests in `parser_models_compatibility.rs` cover complete DTO JSON roundtrips with arbitrary nested protocol configuration, nullable summary fields and actual desktop aliases; padded/unpadded base64; entity/wrapper normalization; the existing URI extraction schemes; IPv6, query parsing and invalid authorities.

### Full shared subscription/protocol parser

Core now exposes `parser::parse_subscription`, URI and Clash parsers, OpenRung parsing, and pure WARP/MASQUE parsing. These reuse the upstream implementations rather than introducing a second parser. `NodeMetadataProvider` explicitly supplies each node ID and timestamp; the desktop compatibility adapter retains UUID generation and the clock. Date-template expansion also remains desktop-side.

Rule parsing and geo category scanning use an explicit `GeoProvider`. Native callers can supply asset bytes through `BytesGeoProvider` or a precise unavailable reason. Present categories are accepted; known absent categories are filtered with diagnostics. Missing, unreadable or malformed inventories remain distinguishable from an actual scanned absence and retain upstream defer-to-engine validation behavior. Desktop filesystem inventory access stays in its provider; the core does not choose data paths or read geo files.

Entropy, key generation and WARP registration remain desktop-side. Moving pure MASQUE parsing does not move those operations into core. Desktop public parser signatures are retained through compatibility adapters. The mobile import UI source now uses this parser through a paste/preview/confirm model. Native VPN integration remains unwired.

### Shared six-engine configuration compilation

The shared core now reuses the upstream IR and compilers for Mihomo, Sing-box, Xray, Shoes, Meow and VP-Core. `compilation::compile_configuration` is a checked API: it validates known capabilities, returns structured sanitized diagnostics, and rejects unsupported combinations or dangling `rule_sets` instead of silently falling back to another engine. A successful artifact means configuration generation, not engine availability, execution or VPN connectivity.

Callers explicitly provide geo inventories, independent Shoes raw-asset inventory/opaque target paths, a probe URL and direct-mirror hosts. Core does not infer paths or fetch resource URLs. Desktop compatibility facades preserve the old compile-only output behavior and explicitly map runtime fields and engine identities. The stricter checked API does not retroactively change those legacy desktop semantics.

Six captured original-desktop artifacts are included in the normal, non-ignored golden test, with fixture provenance under `crates/vp-client-core/tests/fixtures/compiler-baseline`. The extracted output matches all six byte-for-byte. Coverage is deliberately narrow: one synthetic default-options configuration per engine, explicit missing Geo inventories, fixed metadata and example-only credentials; Sing-box uses Hysteria2, the other engines Trojan. This does not cover every protocol, geo state, routing option, real engine or native platform. See [the compilation extraction notes](shared-compilation-extraction.md).

### Mobile import model and UI source

The paste/preview/confirm model imports nodes and rules into memory only. Its host metadata source supplies real UUIDs and UTC timestamps; parsing does not fetch subscriptions, persist data or start a VPN. The egui import controls are wired in source, not yet rendering/native validated.

The `rendering` feature remains enabled by default, with eframe/once_cell optional so `--no-default-features` can test the pure import/editor model without GUI dependencies. The existing `cfg(not(test))` exclusion for the desktop theme/app prevents those components from being compiled by unit tests: passing the model gate does not prove GUI compilation. The mobile-model gate left the root Cargo.lock checkpoint hash above unchanged.

### Checked mobile configuration preview

The preview source requires an explicit selection among six engines and an imported node selected by stable ID. It uses the checked core compiler; failures never substitute another engine or change routing mode. Current DNS draft text is validated on Compile rather than silently reusing an older valid configuration. Both geo providers explicitly report missing inventory, and no engine asset paths are supplied.

A successful result is a non-runnable template, not a configured mobile VPN: native engines, packet flow, asset placement and controller security are not provisioned. The model performs no network access, export, persistence or engine execution. Default summaries hide the artifact and credentials; a separate reveal action warns that credentials become visible. Navigation hides the revealed text and relevant edits invalidate the artifact, but native background/screenshot privacy has not been validated. Hiding or clearing is not secure memory erasure.

The current actual-workspace pure-model gate passed all 27 tests (12 preview plus 15 existing import/editor tests), with the same 67-package no-GUI graph and unchanged root Cargo.lock hash. Core 102 and headless root 11 results remain the preceding unchanged-code checkpoints. The next meaningful gate is real GUI/native compilation and validation, not additional untested UI behavior.

### Bundled Chinese font

The rendering-source integration embeds `assets/fonts/PrivateVPSansSC-Regular.ttf` and installs it as a fallback for egui proportional and monospace families while retaining default fonts. The font is a full-coverage static weight-400 instance derived from Noto Sans SC and renamed PrivateVP Sans SC; it is not a subset. `assets/fonts/OFL.txt` carries the complete SIL Open Font License 1.1, alongside provenance, verification metadata and the reproducible derivation recipe. The source About panel includes the full license.

Structural checks and an ab_glyph software check of 63 glyphs passed. These establish font-data/outline compatibility, not egui layout, screenshots, native compilation or device rendering. Coverage is Simplified-Chinese-oriented, not all CJK or emoji; locale-specific Japanese forms and every possible glyph are not guaranteed. The rendering integration remains source-level work, distinct from the passing no-GUI model gate.

## Verification status

| Scope | Evidence | Limit |
|---|---|---|
| Initial configuration extraction | Standalone copied core/facade workspace: 12 original + 6 compatibility tests passed using Rust 1.88 and normal Cargo `--locked --offline` | Historical scope, before the DTO/parser expansion; not a whole-root-workspace pass |
| Expanded core including DTO/parser extraction | Standalone copied core plus actual config/model/parser facades: 24 tests passed (12 original + 6 config compatibility + 6 parser/DTO compatibility), zero failed/ignored/filtered, Rust 1.88 with normal Cargo `--locked --offline`; 23-package graph | Copied sources reverified against the current fork; this is not an integrated root-workspace, GUI or mobile gate |
| Full shared parser checkpoint | 71/71 core tests passed: 54 unit + 17 integration (6 config compatibility, 6 parser/DTO compatibility, 5 full-parser extraction) | Core scope only; not GUI, native platforms or runtime |
| Current shared compilation/core gate | Actual workspace: 102 normal, non-ignored tests passed (76 unit + 26 integration), including six original-artifact golden comparisons | Fixed synthetic golden scope above; not GUI, engine runtime or all-platform validation |
| Linux headless desktop compatibility | Current 213-package root headless graph compiled; eleven exact pure regression tests passed | Not the complete root test suite; application main, GUI and engine runtime were not launched |
| Root dependency lock | Normal Cargo reconciliation for the shared parser; checkpoint hash above | Lock reconciliation alone is not cross-platform or runtime validation |
| Mobile import/editor/preview model | Actual workspace: 27/27 ordinary pure-model tests passed (12 preview + 15 prior import/editor) using `--no-default-features`; unchanged 67-package graph, no GUI dependencies; source and test executable verified | UI source is wired, but no native compile, rendering, device lifecycle or VPN evidence |
| Development application-updater guard | Same 213-package headless graph compiled; eleven exact pure tests passed (prior eight + three guard tests) | GUI guard presentation is source-only, not compiled; main, native clients and runtime remain unvalidated |
| Desktop runtime | Not launched as validation | No claim that main/GUI, engine execution or host networking has been tested |

Core has no build.rs, network client, engine downloader, shell command or implicit data-root lookup. Its test dependencies still require normal dependency/build-script review. Reviewed serde/serde_core/proc-macro2/quote/zmij build scripts perform compiler probes and OUT_DIR operations. The standalone core gates subsequently acquired and checked their exact missing dependency sources. These focused checks are not a complete audit of all desktop or mobile transitive dependencies.

## Runtime and build boundaries found in upstream source

- Root `build.rs` writes `target/cargo-version.iss`; on Windows it invokes the resource compiler through embed-resource. It does not itself download engines. `.cargo/config.toml` sets the target directory.
- Ordinary startup in `src/main.rs` cleans orphan engine processes, heals system proxy/DNS, probes installed engines by running version commands, opens/migrates SQLite, scrubs records and automatically provisions missing GeoIP/GeoSite data. Startup is not a side-effect-free smoke test.
- Network engine updates require release SHA-256 and size matches, then extract, execute a version probe, back up and replace engines. Local imports attempt to fetch release metadata and may skip SHA-256 when an official asset cannot be matched. Mirrors include ghfast.top and gh-proxy.com. A digest does not independently authenticate its metadata publisher.
- The development-fork application binary updater is explicitly disabled by `app_update_policy.rs`, with guards before HTTP checks, installer downloads and install launch. This prevents the retained upstream release implementation from being used as the fork's binary update channel. Kernel updates and update-cache cleanup are unchanged. Upstream source synchronization is distinct from installing an upstream binary; neither an upstream tag nor a fork version establishes a release channel. Future distribution requires a separately reviewed fork release identity, artifact provenance and update policy. GUI presentation changes for this guard remain source-only and have not been compiled.
- Linux package hooks refresh desktop/icon caches; RPM scripts do likewise. The OpenWrt tooling installer can install system packages, compile tools and copy libraries into system paths. Packaging scripts are not validation commands.
- The remaining desktop models reference db identifiers, proxy rules and kernel source preferences. Profile/proxy types use desktop data-root lookup; desktop compiler adapters consult geo files and supply explicit context to the shared compilers. Moving every module into lib.rs would expose desktop side effects rather than yield a portable core.

## Next extraction boundaries

1. **Parser consumers:** validate the now-wired mobile import UI on real rendering/native targets. Its nodes/rules remain memory-only; persistence and native VPN application are separate future integrations. Preserve diagnostics and protocol fields; never report a VPN connection from a successful import.
2. **Rules/config compilation:** extend the six-engine golden matrix beyond one synthetic configuration per engine, and integrate checked compile previews without confusing artifact generation with engine validation or VPN readiness.
3. **Data layer:** accept explicit storage roots and repository interfaces. Place SQLite in a separate, feature-gated data adapter rather than the always-linked pure core. Keep profile file import/export explicit.
4. **Native adapters:** implement lifecycle/capability contracts with honest unsupported states, cancellation and typed failures. Never route a mobile build through the desktop platform fallback to Linux executable assets.
5. **Desktop adapter:** retain egui and the binary; isolate process/system-proxy responsibilities behind a desktop adapter before linking native clients.

## Change coordination and validation ladder

Keep one owner for root manifests and Cargo.lock. Treat core APIs, desktop compatibility facades and their tests as one extraction seam. Mobile UI/native manifests are separate from root dependency integration. Do not regenerate the lock concurrently or commit build caches, real subscriptions, credentials or generated binaries.

Validation proceeds from source/import review to bounded pure-core tests, then compile-only desktop checks, then platform-specific native compilation and device tests. Native device validation must cover consent/cancellation, reconnect/revoke, background transitions, routing/DNS and leaks. Never substitute launching the application or installer for a pure unit-test gate.


### Development-updater validation checkpoint

The headless updater-guard checkpoint used test-harness ELF SHA-256 `2473812d5d5b6be607f2a7c054fb822d2f4bfd7e61dc742654ce90aa943e6171`. Eleven exact pure tests passed without launching application main, GUI, an installer or a native VPN runtime. This evidence covers the headless policy/API guard scope, not rendering or execution of the About-page controls.

### Work in progress: platform lifecycle and asset selection

The development branch adds a pure, platform-neutral mobile connection state
model. It requires an explicitly available engine and user permission, does not
mark connected until protected sockets, packet forwarding and DNS evidence are
reported, invalidates stale callbacks, and blocks new starts when stop is
uncertain. Eight standalone Rust unit tests cover these transitions. The model
is not yet connected to Android `VpnService`, a real engine, a device UI, or
network traffic; it is not a working Android VPN.

The desktop source also selects macOS engine release assets by the explicit CPU
architecture, without cross-architecture fallback; Linux ARM64 and x86_64 are
kept distinct, while unsupported combinations fail closed. Update checks try
the installer's ordered candidates for that architecture and reject assets
without a well-formed SHA-256 or a valid size. Seven standalone tests, including
the actual architecture dispatcher, pass against asset names observed in the
corresponding upstream releases. The full desktop application has not been compiled or run after this
change; actual downloads, executable verification, engine startup, and native
macOS behavior still require independent validation. Existing user permissions and
upstream attribution remain unchanged.
