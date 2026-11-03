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
arm64 library at `app/src/main/jniLibs/arm64-v8a/`. The Gradle 8.9 wrapper is
included with the official distribution SHA-256; first use downloads Gradle.
There is no Rust build hook. SDK/NDK/Gradle installation, dependency resolution,
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
Install the reviewed `.so` into the stated jniLibs path, then invoke
`apps/android/gradlew -p apps/android assembleDebug` with JDK 17 and SDK 35.
The packaging gate rejects a missing file or an ELF header that is not arm64;
it does not prove exported symbols, successful linking, or runtime behavior.
The wrapper JAR matches Gradle 8.9's published SHA-256
`498495120a03b9a6ab5d155f5de3c8f0d986a449153702fb80fc80e134484f17`.

The Rust cross build also needs an NDK linker. For the reported macOS NDK
27.2 setup, set `ANDROID_NDK_HOME` to that installed NDK and set
`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER` to
`$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android26-clang`.
The host-tag spelling follows Android's NDK documentation even on an Apple
Silicon Mac. Confirm that the file exists and the Rust target is installed;
this repository does not install the toolchain or accept SDK terms for you.

For a clean verification build on a prepared macOS or x86_64 Linux host, run
`apps/android/build-arm64.sh OUTPUT_DIRECTORY`. The output directory must not
already contain the two named output files. The script uses a fresh temporary
Cargo target, Gradle home and Gradle project cache, and deletes that temporary
directory on exit. It preserves the resulting APK and library in the output
directory, checks AArch64, the NativeActivity entry and byte-for-byte APK
contents, and never installs an SDK or changes the existing Gradle cache.
The script has not yet run end-to-end on an Android build host.

An experimental native host can be included by setting `VP_EGUI_PRODUCT_ROOT`
to the unified product checkout and `VP_CORE_ADAPTER_SO` to an independently
reviewed AArch64 `libvp_product_core_adapter.so` built from its matching source.
The script then compiles the product's JNI host and adds the Kotlin/manifest
module. It verifies the architecture and APK bytes of all three libraries.
This optional variant changes Android permissions and has not been built or
run; the egui UI does not yet invoke the bridge, and this is not a working VPN.

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
