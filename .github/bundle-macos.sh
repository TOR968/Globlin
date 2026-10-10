#!/usr/bin/env bash
set -euo pipefail

binary="$1"
out_dir="$2"

version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
work="$(mktemp -d)"
app="$work/Globlin.app"

mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$out_dir"
cp "$binary" "$out_dir/globlin-macos-arm64"
cp "$binary" "$app/Contents/MacOS/globlin"
chmod 755 "$app/Contents/MacOS/globlin" "$out_dir/globlin-macos-arm64"

GLOBLIN_ICONSET="$work/Globlin.iconset" cargo test -- --ignored --exact icon::tests::dump_macos_iconset
iconutil -c icns "$work/Globlin.iconset" -o "$app/Contents/Resources/Globlin.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>globlin</string>
    <key>CFBundleIdentifier</key>
    <string>dev.globlin.app</string>
    <key>CFBundleName</key>
    <string>Globlin</string>
    <key>CFBundleDisplayName</key>
    <string>Globlin</string>
    <key>CFBundleIconFile</key>
    <string>Globlin</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$version</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - --identifier dev.globlin.app "$app"
codesign --verify --strict --verbose=2 "$app"

ditto -c -k --keepParent "$app" "$out_dir/Globlin-macos-arm64.zip"
ls -l "$out_dir"
