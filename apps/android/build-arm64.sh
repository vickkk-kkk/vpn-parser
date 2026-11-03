#!/bin/sh
# Build the current UI shell, not a VPN client. No toolchain installation.
set -eu
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 OUTPUT_DIRECTORY" >&2
    exit 2
fi
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to an existing Android NDK}"
case "$(uname -s)" in
    Darwin) host=darwin-x86_64 ;;
    Linux) host=linux-x86_64 ;;
    *) echo "Unsupported build host" >&2; exit 2 ;;
esac
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
repo=$(CDPATH= cd -- "$script_dir/../.." && pwd -P)
toolchain="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$host/bin"
linker="$toolchain/aarch64-linux-android26-clang"
for tool in "$linker" "$toolchain/llvm-ar" "$toolchain/llvm-readelf" "$toolchain/llvm-nm"; do
    [ -x "$tool" ] || { echo "Missing NDK tool: $tool" >&2; exit 2; }
done
command -v cargo >/dev/null
command -v java >/dev/null
command -v unzip >/dev/null
mkdir -p "$1"
out=$(CDPATH= cd -- "$1" && pwd -P)
apk_out="$out/vpn-parser-android-arm64-debug.apk"
so_out="$out/libvp_client_android.so"
[ ! -e "$apk_out" ] && [ ! -e "$so_out" ] || { echo "Output already exists; choose an empty output directory." >&2; exit 2; }
work=$(mktemp -d "${TMPDIR:-/tmp}/vp-android-build.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP
export CARGO_TARGET_DIR="$work/cargo-target"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$linker"
export CC_aarch64_linux_android="$linker"
export AR_aarch64_linux_android="$toolchain/llvm-ar"
export GRADLE_USER_HOME="$work/gradle-home"
cd "$repo"
cargo build --locked --release --target aarch64-linux-android --manifest-path apps/android/Cargo.toml
so="$CARGO_TARGET_DIR/aarch64-linux-android/release/libvp_client_android.so"
"$toolchain/llvm-readelf" -h "$so" | grep -Eq 'Machine:.*AArch64' || { echo "Not an AArch64 ELF" >&2; exit 1; }
"$toolchain/llvm-nm" -D --defined-only "$so" | grep -Eq '[[:space:]]ANativeActivity_onCreate$' || { echo "NativeActivity entry missing" >&2; exit 1; }
mkdir -p "$work/jni/arm64-v8a"
cp "$so" "$work/jni/arm64-v8a/libvp_client_android.so"
"$script_dir/gradlew" -p "$script_dir" --project-cache-dir "$work/project-cache" \
    -PvpNativeLibDir="$work/jni" -PvpBuildDir="$work/app-build" assembleDebug
apk="$work/app-build/outputs/apk/debug/app-debug.apk"
[ -f "$apk" ] || { echo "Gradle did not produce the expected APK" >&2; exit 1; }
unzip -p "$apk" lib/arm64-v8a/libvp_client_android.so | cmp -s - "$so" || { echo "APK native library mismatch" >&2; exit 1; }
cp "$so" "$so_out"
cp "$apk" "$apk_out"
echo "Built $apk_out and $so_out. Device and VPN verification remain required."
