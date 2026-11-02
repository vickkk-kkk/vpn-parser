# Portable mobile configuration surface

This is a deliberately limited first slice of the upstream VPN Parser client,
not a completed mobile client. It uses the unchanged production implementation
of `src/gui/theme.rs` by path, plus `vp-client-core`'s `DnsConfig`, `RoutingMode`
and `TrafficRule`. No desktop app initialization, database, process manager,
updater, downloader, system proxy or native tunnel API is linked through these
modules. `MobileApp::new(&CreationContext, platform)` is used by both hosts.

Routing and DNS edits are memory-only drafts. Rule generation and DNS validation
call the real shared implementation, but validation is only the upstream model's
validation—not a connectivity, syntax-completeness or security guarantee. There
is no invented node parser, imported node, successful tunnel or traffic counter.
Connect remains disabled. Node import/subscriptions, persistent secure storage,
actual native lifecycle/engine binding and full upstream views remain to port.

The appearance adapts the upstream palette with 48-point interaction targets,
scrolling, three compact tabs and an explicit nonfunctional tunnel notice. This
has not been rendered or checked for safe areas, IME, screen reader behavior or
narrow-screen layout on either platform. Source intent is not usability proof.

The shared eframe dependency enables fonts only. Android chooses glow with
NativeActivity; iOS chooses wgpu/Metal. Each native app is an independent Cargo
workspace. Resolve and validate its own lockfile before claiming reproducibility;
no native locks were synthesized in this change.

Unit-test builds expose only the editor model because the directly reused
upstream theme's own tests depend on desktop `AppSettings`. Those upstream tests
remain in the desktop source; this crate does not stub desktop models. Its two
pure editor tests must be run separately from compile/render checks of the
normal UI. A host build of target-gated mobile entry files is an empty-library
check, not validation of Android or iOS code.
