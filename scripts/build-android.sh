#!/usr/bin/env bash
# Builds the Rust core for Android and generates the Kotlin bindings.
# Needs: rustup target aarch64-linux-android, cargo-ndk, and ANDROID_NDK_HOME.
# Afterwards: cd android && gradle assembleRelease
set -euo pipefail
cd "$(dirname "$0")/.."

# ABI=x86_64 builds for the emulator instead of a phone.
ABI="${ABI:-arm64-v8a}"
JNI=android/app/src/main/jniLibs
KOTLIN=android/app/src/main/java

echo "== Rust library for Android ($ABI)"
cargo ndk -t "$ABI" -P 26 -o "$JNI" build --release -p snowtrack-core --lib
SO="$JNI/$ABI/libsnowtrack_core.so"
ls -la "$SO"
# Ship the C++ runtime too if the library needs it.
if readelf -d "$SO" | grep -q "libc++_shared"; then
  TRIPLE=$([ "$ABI" = "x86_64" ] && echo x86_64-linux-android || echo aarch64-linux-android)
  cp "$(find "${ANDROID_NDK_HOME:?}" -path "*$TRIPLE/libc++_shared.so" | head -1)" "$JNI/$ABI/"
fi

echo "== Kotlin bindings"
# The bindings are read from a host build of the same source.
cargo build -p snowtrack-core --lib
HOST_LIB=""
for candidate in target/debug/libsnowtrack_core.so target/debug/libsnowtrack_core.dylib; do
  if [ -f "$candidate" ]; then HOST_LIB="$candidate"; break; fi
done
if [ -z "$HOST_LIB" ]; then echo "Host build of the core library not found in target/debug" >&2; exit 1; fi
rm -rf "$KOTLIN/uniffi"
cargo run -q -p snowtrack-core --features cli --bin uniffi-bindgen -- \
  generate --library "$HOST_LIB" --language kotlin --no-format --out-dir "$KOTLIN"
ls -la "$KOTLIN/uniffi/snowtrack_core/snowtrack_core.kt"
