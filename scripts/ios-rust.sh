#!/usr/bin/env bash
# The iOS project's "Build Rust Code" phase (src-tauri/gen/apple/project.yml), in place of
# `tauri ios xcode-script`: that needs Xcode's iOS simulator platform installed and the Tauri CLI
# running beside Xcode. Builds the front end (dist/), then the launcher library for arm64 iPhones
# with the front end embedded (tauri/custom-protocol, as `tauri build` does), as the project's
# Externals/arm64/<configuration>/libapp.a.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
config=${CONFIGURATION:-debug}
out=${SRCROOT:-$root/src-tauri/gen/apple}/Externals/arm64/$config
(cd "$root" && npm run --silent build)
profile=debug release=()
if [[ $config == release ]]; then profile=release release=(--release); fi
# Xcode's SDKROOT names the iOS SDK; Cargo's build scripts run on the Mac and need its own.
cd "$root/src-tauri"
env -u SDKROOT cargo build --lib --target aarch64-apple-ios --features tauri/custom-protocol "${release[@]}"
mkdir -p "$out"
cp "target/aarch64-apple-ios/$profile/libbloodborne_launcher_lib.a" "$out/libapp.a"
