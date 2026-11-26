#!/usr/bin/env bash
# Builds the GPUI Android smoke example into a signed debug APK.
#
# Requires: the aarch64-linux-android Rust target, the Android NDK (ANDROID_NDK_HOME),
# SDK build-tools (ANDROID_BUILD_TOOLS, containing aapt2/zipalign/apksigner), a platform
# android.jar (ANDROID_JAR) and a JDK for keytool/apksigner.
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
workspace_dir="$(cd "$crate_dir/../.." && pwd)"
profile="${PROFILE:-release}"
target="aarch64-linux-android"
api="${ANDROID_API:-26}"

: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to the NDK directory}"
: "${ANDROID_BUILD_TOOLS:?set ANDROID_BUILD_TOOLS to an SDK build-tools directory}"
: "${ANDROID_JAR:?set ANDROID_JAR to a platform android.jar}"

toolchain="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
export CC_aarch64_linux_android="$toolchain/aarch64-linux-android$api-clang"
export CXX_aarch64_linux_android="$toolchain/aarch64-linux-android$api-clang++"
export AR_aarch64_linux_android="$toolchain/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$toolchain/aarch64-linux-android$api-clang"

target_dir="${CARGO_TARGET_DIR:-$workspace_dir/target}"
library="$target_dir/$target/$profile/examples/libgpui_android_example.so"
out="$target_dir/gpui-android-apk"
# The APK output directory is replaced on every build. Keep private keys elsewhere.
keystore="${DEBUG_KEYSTORE:-$HOME/.android/debug.keystore}"
keystore_path="$(realpath -m -- "$keystore")"
out_path="$(realpath -m -- "$out")"
case "$keystore_path" in
    "$out_path" | "$out_path"/*)
        echo "DEBUG_KEYSTORE must be outside $out" >&2
        exit 1
        ;;
esac

cargo_profile_flag=()
[ "$profile" = release ] && cargo_profile_flag=(--release)
# Align ELF load segments as well as the APK entry. Pass flags only to the final
# example link so Cargo still honors the caller's configured Rust flags.
(cd "$workspace_dir" && cargo rustc -p gpui_android --example gpui_android_example --target "$target" "${cargo_profile_flag[@]}" -- \
    -C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384)

rm -rf "$out" && mkdir -p "$out/stage/lib/arm64-v8a"
cp "$library" "$out/stage/lib/arm64-v8a/libgpui_android_example.so"
"$toolchain/llvm-strip" --strip-debug "$out/stage/lib/arm64-v8a/libgpui_android_example.so"

"$ANDROID_BUILD_TOOLS/aapt2" link -o "$out/unaligned.apk" \
    --manifest "$crate_dir/android/AndroidManifest.xml" -I "$ANDROID_JAR"
(cd "$out/stage" && zip -q -0 -r "$out/unaligned.apk" lib)
# 16 KiB page alignment keeps uncompressed native libraries loadable on Android 15+.
"$ANDROID_BUILD_TOOLS/zipalign" -f -P 16 4 "$out/unaligned.apk" "$out/aligned.apk"

if [ ! -f "$keystore" ]; then
    (
        umask 077
        mkdir -p -- "$(dirname -- "$keystore")"
        keytool -genkeypair -keystore "$keystore" -storepass android -keypass android \
            -alias androiddebugkey -dname "CN=Android Debug,O=Android,C=US" \
            -keyalg RSA -keysize 2048 -validity 10000
    )
fi
"$ANDROID_BUILD_TOOLS/apksigner" sign --ks "$keystore" --ks-pass pass:android \
    --out "$out/gpui-android-example.apk" "$out/aligned.apk"
echo "$out/gpui-android-example.apk"
