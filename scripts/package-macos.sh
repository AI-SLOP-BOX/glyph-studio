#!/bin/sh
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
release_dir="$project_dir/target/release"
app_dir="$release_dir/Glyph Studio.app"

cargo build --release --manifest-path "$project_dir/Cargo.toml"
mkdir -p "$app_dir/Contents/MacOS"
cp "$release_dir/glyph-studio" "$app_dir/Contents/MacOS/glyph-studio"
cp "$project_dir/Info.plist" "$app_dir/Contents/Info.plist"
chmod 755 "$app_dir/Contents/MacOS/glyph-studio"

if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "$app_dir" >/dev/null
fi

echo "Packaged: $app_dir"
