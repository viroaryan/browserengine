#!/usr/bin/env bash
set -euo pipefail

# Build script for Nirvana Engine Android Shared Libraries

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
JNI_LIBS_DIR="${ROOT_DIR}/android/app/src/main/jniLibs"

# Verify Android NDK environment
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -z "${NDK_HOME:-}" ]; then
    echo "WARNING: Neither ANDROID_NDK_HOME nor NDK_HOME environment variable is set."
    echo "cargo-ndk requires the Android NDK path to compile for Android targets."
fi

echo "==> Checking cargo-ndk..."
if ! command -v cargo-ndk &> /dev/null; then
    echo "cargo-ndk not found, installing..."
    cargo install cargo-ndk
fi

TARGETS=("arm64-v8a" "x86_64")

for ABI in "${TARGETS[@]}"; do
    echo "==> Building for ABI: ${ABI} (release)..."
    cargo ndk -t "${ABI}" -o "${JNI_LIBS_DIR}" build -p nirvana_android --release
done

echo "==> Native build complete! Libraries installed to: ${JNI_LIBS_DIR}"
