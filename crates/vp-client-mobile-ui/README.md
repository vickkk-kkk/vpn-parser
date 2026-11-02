# Portable mobile client slice

This fork-based slice uses the unchanged upstream production theme by path and
`vp-client-core`'s actual subscription parser, node DTOs, fingerprints, geo
provider, DNS validator and routing models. It never starts the desktop App.

## Paste import

Both native hosts inject UUID v4 IDs and UTC timestamps. The pure import model
contains no clock/randomness, filesystem, downloader, process or network API.
Paste content (not a URL) into Nodes, preview recognized nodes and geo warnings,
then confirm into memory. Editing or canceling invalidates preview; confirmation
consumes it. About offers a separate confirmed clear of all imported nodes, rule
drafts, preview and paste; cancel leaves data unchanged. The Routing / DNS reset
is separate. Clearing app state does not claim secure memory erasure or clipboard
clearing. Existing nodes are deduplicated by the core fingerprint without
replacing their IDs. Transport-distinct configurations remain distinct.

Input is limited to 128 KiB before parsing; previews and cumulative imports are
limited to 512 nodes and 512 rule drafts. Oversized inputs are rejected, not
silently truncated. Parsing is synchronous on explicit Preview and still needs
real-device latency/memory evaluation. These bounds are not a parser sandbox.
Unknown/malformed entries may be skipped by the existing parser; the UI does not
invent rejected-entry counts or claim that all content was accepted.

Geo assets are explicitly missing in the current hosts. Warnings retain the
rule index/type/status but never copy untrusted category strings. Existing
legacy behavior retains unavailable categories and filters known-absent ones;
two-letter GeoIP compatibility bypasses remain unchanged. All rule drafts remain
unapplied. No connection or latency test is performed. Connect stays disabled.

Imported credentials and draft text exist in process memory only. No secure
persistence, memory-zeroization, clipboard clearing, URL subscription fetching,
auto-refresh or native VPN integration is claimed. Node summaries are bounded
plain text with control/bidi characters removed; raw URI/config is not rendered
in summaries or logged. The paste box necessarily displays user-supplied text.

## Features and validation boundaries

`default = ["rendering"]` preserves the package's rendering default. `eframe` and
its direct theme helper `once_cell` are optional, enabled by `rendering`. Both
native apps explicitly opt in. Pure model tests should select this package with
`--no-default-features`; they must not enable a renderer by feature unification.
Core still legitimately uses once_cell for parser regexes.

The production renderer still has an explicit `not(test)` guard because the
reused upstream theme's unit tests reference desktop AppSettings. This exception
is not a fake desktop model and is not renderer validation. Future theme-test
extraction should remove it. Normal renderer compilation and render/UI tests
remain a separate gate from pure editor/import model tests. Do not interpret a
host build of target-gated mobile entries as an Android/iOS compilation.

## Font and device gates

The rendering-only `fonts` module embeds `assets/fonts/PrivateVPSansSC-Regular.ttf`
and its complete `OFL.txt`, installing a fallback once per MobileApp creation in
both proportional and monospace families. Default egui fonts stay first. No
font loading runs from desktop main.rs, the filesystem or network. Builds with
`--no-default-features` do not compile the font module or embed these assets.

The bundled OFL derivative is named PrivateVP Sans SC Regular (static weight
400, no subsetting). About exposes the complete license/copyright text and a
coverage notice. `assets/fonts/provenance.json` and `derive_font.py` record the
source identity and derivation. Simplified-Chinese coverage is not all CJK or
emoji coverage and does not guarantee locale-specific Japanese forms. English
UI text remains unchanged; imported Chinese names use fallback glyphs.

Bundling and registering a font does not establish native UI rendering. The
separate actual glyph/layout preflight, then Android/iOS Chinese name/input,
keyboard, safe-area, accessibility and narrow-screen checks remain required.

Android uses NativeActivity/glow; iOS uses wgpu/Metal in its own Cargo workspace.
Their exact host chrono/uuid versions match the reviewed root lock, but each
native dependency graph/lock and its SDK build still require independent review.
