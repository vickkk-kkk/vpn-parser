# iOS egui host — source integration, not a working tunnel

`App/main.m` calls the Rust `vp_client_ios_main` on the process main thread.
Locked eframe 0.31.1/winit 0.30.13 own UIApplication and the event loop. Do not add
a SwiftUI `@main`, call UIApplicationMain twice, or invoke this from a background
thread. The renderer is wgpu/Metal: the Android glow/glutin backend must not be
feature-unified into the iOS build. `run_and_return` is false. The shared
`vp-client-mobile-ui` reuses the upstream theme and extracted core logic; neither
the desktop binary nor its runtime services are dependencies.

`project.yml` is an XcodeGen **source specification**, not a generated or verified
Xcode project. The main scheme links a separately supplied Rust static archive
under `Rust/iphoneos/` or `Rust/iphonesimulator/` as appropriate. Simulator and
device archives are not interchangeable. There are no download/build scripts;
XcodeGen, Rust target installation, Xcode/SDK setup, compilation, signing,
entitlements and provisioning remain unperformed. Bundle IDs are development
placeholders. Automatic signing is disabled.

## Explicitly unavailable VPN

A separate `PacketTunnel` target defines an NEPacketTunnelProvider boundary.
`startTunnel` always completes with a concrete unavailable error, never success.
It sets no routes/DNS and starts no packet engine. The extension is deliberately
not embedded in the app, and the app does not create NETunnelProviderManager
configurations or prompt for VPN approval. The extension entitlement file is a
future signing input, not evidence of authorization or capability.

Before exposing connection controls: integrate a supported embedded packet
engine and packetFlow bridge, app/extension IPC and configuration ownership,
cancellation/stop/revoke, correct settings, signing and provisioning. Then embed
the extension and add the app's required entitlements under an approved release
identity. Test real-device foreground/background, lock/unlock, memory pressure,
network handover, DNS/routing and IPv4/IPv6 leaks for every supported engine.

Configuration drafts currently live only in memory, not a secure persistent repository.
Screen safe areas, keyboard/paste, accessibility and touch behavior require
actual simulator/device checks. Source review does not establish UI usability,
VPN function, battery behavior or App Store acceptance.

Versioned API references: [eframe iOS event loop](https://github.com/emilk/egui/blob/0.31.1/crates/eframe/src/native/run.rs),
[eframe Metal dependency](https://github.com/emilk/egui/blob/0.31.1/crates/eframe/Cargo.toml).
