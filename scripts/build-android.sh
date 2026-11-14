#!/usr/bin/env bash
# Builds the Rust core for Android and generates the Kotlin bindings.
# Needs: rustup target aarch64-linux-android, cargo-ndk, and ANDROID_NDK_HOME.
# Afterwards: cd android && gradle assembleRelease
set -euo pipefail
cd "$(dirname "$0")/.."

JNI=android/app/src/main/jniLibs
KOTLIN=android/app/src/main/java

echo "== Rust library for arm64 Android"
cargo ndk -t arm64-v8a -P 26 -o "$JNI" build --release -p snowtrack-core --lib
SO="$JNI/arm64-v8a/libsnowtrack_core.so"
ls -la "$SO"
# Ship the C++ runtime too if the library needs it.
if readelf -d "$SO" | grep -q "libc++_shared"; then
  cp "$(find "${ANDROID_NDK_HOME:?}" -path "*aarch64-linux-android/libc++_shared.so" | head -1)" "$JNI/arm64-v8a/"
fi

echo "== Kotlin bindings"
# The bindings are read from a host build of the same source.
cargo build -p snowtrack-core --lib
HOST_LIB=$(ls target/debug/libsnowtrack_core.so target/debug/libsnowtrack_core.dylib 2>/dev/null | head -1)
rm -rf "$KOTLIN/uniffi"
cargo run -q -p snowtrack-core --features cli --bin uniffi-bindgen -- \
  generate --library "$HOST_LIB" --language kotlin --no-format --out-dir "$KOTLIN"
ls -la "$KOTLIN/uniffi/snowtrack_core/snowtrack_core.kt"
