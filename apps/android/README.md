# Android egui host — source integration, not a VPN build

The `android_main` entry uses the locked eframe 0.31.1 NativeActivity integration
and winit 0.30.13's re-exported `AndroidApp` (android-activity 0.6.1). It passes
that exact handle to `NativeOptions.android_app`; eframe owns window/surface
suspend and resume handling. It does **not** invoke desktop `main` or `App::new`.
The UI is shared with iOS through `vp-client-mobile-ui`, using the upstream
palette and extracted configuration logic, not a separate mock dashboard.

## Packaging boundary

`app/src/main/AndroidManifest.xml` registers `android.app.NativeActivity`, loading
`libvp_client_android.so`. The Gradle source project expects a separately built
arm64 library at `app/src/main/jniLibs/arm64-v8a/`. No Gradle wrapper binary or
Rust build hook is included. SDK/NDK/Gradle installation, dependency resolution,
builds and device execution require a separately reviewed environment. Package
IDs are development placeholders, not an established release/signing identity.
The source pins AGP 8.7.3, SDK 35 and min SDK 26; these have not been exercised.

The independent Android workspace has its own checked-in `Cargo.lock`, generated
from the current manifest rather than copied from an earlier validation run.
The lock was regenerated without change using Cargo 1.88 offline. Offline
metadata/build remains blocked here by missing crate sources, and the arm64
target has not been compiled. On a prepared Android build machine, start with
`cargo build --locked --release --target aarch64-linux-android --manifest-path apps/android/Cargo.toml`
and verify the lock remains unchanged; a successful build is still not an APK.

The application intentionally has no INTERNET permission, VPN service,
VpnService consent request, embedded engine, downloader, process runner, desktop
system-proxy integration or background service. Configuration drafts are memory-only.
Closing/recreating the process loses them. This is deliberate, not persistence.

## Next acceptance gate

1. Compile the Rust target and Android package with reviewed tools; verify the
   packaged ABI and exported NativeActivity entry rather than accepting an empty
   APK. Never execute the desktop app as a substitute.
2. Test rotation, background/resume, activity recreation, Back, soft-keyboard
   text input/paste, narrow screens and scrolling on actual Android. NativeActivity
   IME/accessibility behavior needs device validation; comfortable spacing alone
   does not establish accessibility or usable keyboard input.
3. Implement a real VpnService adapter with explicit user consent, protected
   sockets, owned TUN descriptor, engine stop/revoke handling and failure states.
   Only then register the service/permissions and enable connection actions.
4. Test routing, DNS, IPv4/IPv6 leakage, reconnect and cancellation for each engine.

No native compilation, UI render, VPN connection, leak test or all-core support
is claimed by the presence of these source files.

Versioned API references: [eframe NativeOptions](https://github.com/emilk/egui/blob/0.31.1/crates/eframe/src/epi.rs),
[eframe lifecycle](https://github.com/emilk/egui/blob/0.31.1/crates/eframe/src/native/run.rs).
