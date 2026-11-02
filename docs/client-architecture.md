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
- `crates/vp-client-core`: shared configuration, node DTOs and pure parsing helpers.
- `crates/vp-client-mobile-ui`: shared egui mobile configuration-editor source.

`apps/android` and `apps/ios` are explicitly excluded from the root workspace and declare independent native workspaces. The mobile contribution contains 21 source files and has received static checks only; native compilation, UI rendering and VPN operation have not been validated.

The root Cargo.lock has been reconciled through normal Cargo processing during a targeted `cargo tree` preflight. Its SHA-256 at this checkpoint is:

`146873de3879fe0d4bc9b7e03188c097aa5561cde08b317b3b91ddb42a1f5f30`

The reconciliation added the expanded local dependency edges and mobile-UI package record without changing any original registry version or checksum. The preflight stopped at its source-download budget before completing graph output. Lock reconciliation is not evidence that the complete selected metadata graph, desktop build or test suite passed.

## Implemented extractions

### Configuration and rules

The upstream `src/proxy/config.rs` implementation and its 12 original tests were moved byte-for-byte to `crates/vp-client-core/src/config.rs`. The desktop file now re-exports those definitions, preserving caller paths, type identity and serialized representations.

Six integration tests in `config_compatibility.rs` exercise the actual desktop facade, stable JSON names, old DNS deserialization, migration idempotence, delimiter rejection and large-list rule ordering/deduplication. They do not start the desktop application.

### Node DTOs and subscription-text helpers

Core `models::{Node, NodeSummary, NodeGroupSummary}` preserves the exact upstream declaration block, including every field and serde behavior. Desktop `models::node` re-exports the same types.

Core `parser` contains the unchanged upstream common parsing helpers, excluding date-template expansion. Desktop `parser::common` re-exports them. The public desktop `parser::expand_url_template` implementation remains unchanged and retains its local-clock behavior outside core. Core chrono enables std/serde, not clock.

Six additional integration tests in `parser_models_compatibility.rs` cover complete DTO JSON roundtrips with arbitrary nested protocol configuration, nullable summary fields and actual desktop aliases; padded/unpadded base64; entity/wrapper normalization; the existing URI extraction schemes; IPv6, query parsing and invalid authorities.

Full protocol-to-node parsing remains desktop-side. There is no core `parse_subscription` API yet. In particular, URI text extraction is not equivalent to validating or constructing protocol nodes.

## Verification status

| Scope | Evidence | Limit |
|---|---|---|
| Initial configuration extraction | Standalone copied core/facade workspace: 12 original + 6 compatibility tests passed using Rust 1.88 and normal Cargo `--locked --offline` | Historical scope, before the DTO/parser expansion; not a whole-root-workspace pass |
| Expanded core including DTO/parser extraction | Standalone copied core plus actual config/model/parser facades: 24 tests passed (12 original + 6 config compatibility + 6 parser/DTO compatibility), zero failed/ignored/filtered, Rust 1.88 with normal Cargo `--locked --offline`; 23-package graph | Copied sources reverified against the current fork; this is not an integrated root-workspace, GUI or mobile gate |
| Root dependency lock | Cargo reconciled local package edges; original registry versions/checksums retained | Selected graph output and full build remain incomplete |
| Mobile source | 21 files, static checks only | No native compile, rendering, device lifecycle or VPN evidence |
| Desktop runtime | Not launched as validation | No claim that main/GUI, engine execution or host networking has been tested |

Core has no build.rs, network client, engine downloader, shell command or implicit data-root lookup. Its test dependencies still require normal dependency/build-script review. Reviewed serde/serde_core/proc-macro2/quote/zmij build scripts perform compiler probes and OUT_DIR operations. The standalone core gates subsequently acquired and checked their exact missing dependency sources. These focused checks are not a complete audit of all desktop or mobile transitive dependencies.

## Runtime and build boundaries found in upstream source

- Root `build.rs` writes `target/cargo-version.iss`; on Windows it invokes the resource compiler through embed-resource. It does not itself download engines. `.cargo/config.toml` sets the target directory.
- Ordinary startup in `src/main.rs` cleans orphan engine processes, heals system proxy/DNS, probes installed engines by running version commands, opens/migrates SQLite, scrubs records and automatically provisions missing GeoIP/GeoSite data. Startup is not a side-effect-free smoke test.
- Network engine updates require release SHA-256 and size matches, then extract, execute a version probe, back up and replace engines. Local imports attempt to fetch release metadata and may skip SHA-256 when an official asset cannot be matched. Mirrors include ghfast.top and gh-proxy.com. A digest does not independently authenticate its metadata publisher.
- `src/app_update.rs` targets upstream releases and downloads installers; fork distribution needs a separately reviewed update identity.
- Linux package hooks refresh desktop/icon caches; RPM scripts do likewise. The OpenWrt tooling installer can install system packages, compile tools and copy libraries into system paths. Packaging scripts are not validation commands.
- The remaining desktop models reference db identifiers, proxy rules and kernel source preferences. Profile/proxy types use desktop data-root lookup; config compilers consult geo files and other runtime helpers. Moving every module into lib.rs would expose desktop side effects rather than yield a portable core.

## Next extraction boundaries

1. **Protocol parser:** separate Clash rule parsing from filesystem-backed geo-category inventories without silently dropping rules; separate WARP/MASQUE parsing from random-key generation; parameterize node UUID/time construction. Preserve protocols and desktop policy through explicit context/adapters, not absent-data stubs.
2. **Rules/config compilation:** reuse upstream IR rather than introduce a second compiler. Inject geo inventories and connectivity-test settings; separate pure compiler inputs from file lookup and executable discovery. Preserve golden outputs per backend.
3. **Data layer:** accept explicit storage roots and repository interfaces. Place SQLite in a separate, feature-gated data adapter rather than the always-linked pure core. Keep profile file import/export explicit.
4. **Native adapters:** implement lifecycle/capability contracts with honest unsupported states, cancellation and typed failures. Never route a mobile build through the desktop platform fallback to Linux executable assets.
5. **Desktop adapter:** retain egui and the binary; isolate process/system-proxy responsibilities behind a desktop adapter before linking native clients.

## Change coordination and validation ladder

Keep one owner for root manifests and Cargo.lock. Treat core APIs, desktop compatibility facades and their tests as one extraction seam. Mobile UI/native manifests are separate from root dependency integration. Do not regenerate the lock concurrently or commit build caches, real subscriptions, credentials or generated binaries.

Validation proceeds from source/import review to bounded pure-core tests, then compile-only desktop checks, then platform-specific native compilation and device tests. Native device validation must cover consent/cancellation, reconnect/revoke, background transitions, routing/DNS and leaks. Never substitute launching the application or installer for a pure unit-test gate.
