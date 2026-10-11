#!/usr/bin/env bash
set -euo pipefail

out_dir="$1"
arch="$(uname -m)"
tool_version="1.9.1"
runtime_version="20251108"

case "$arch" in
  x86_64)
    tool_sha="ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0"
    runtime_sha="2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d"
    ;;
  aarch64)
    tool_sha="f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158"
    runtime_sha="00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444"
    ;;
  *)
    echo "no pinned AppImage tooling for $arch" >&2
    exit 1
    ;;
esac

tools="$(mktemp -d)"
curl -sSfL -o "$tools/appimagetool" "https://github.com/AppImage/appimagetool/releases/download/$tool_version/appimagetool-$arch.AppImage"
curl -sSfL -o "$tools/runtime" "https://github.com/AppImage/type2-runtime/releases/download/$runtime_version/runtime-$arch"
echo "$tool_sha  $tools/appimagetool" | sha256sum --check
echo "$runtime_sha  $tools/runtime" | sha256sum --check
chmod +x "$tools/appimagetool"

app_dir="target/Globlin.AppDir"
rm -rf "$app_dir"
install -Dm755 target/release/globlin "$app_dir/usr/bin/globlin"
install -Dm644 packaging/linux/globlin.desktop "$app_dir/usr/share/applications/globlin.desktop"
mkdir -p "$app_dir/usr/share/icons/hicolor"
cp -r target/linux-icons/. "$app_dir/usr/share/icons/hicolor/"
ln -s usr/bin/globlin "$app_dir/AppRun"
ln -s usr/share/applications/globlin.desktop "$app_dir/globlin.desktop"
ln -s usr/share/icons/hicolor/256x256/apps/globlin.png "$app_dir/globlin.png"
ln -s globlin.png "$app_dir/.DirIcon"

mkdir -p "$out_dir"
ARCH="$arch" APPIMAGE_EXTRACT_AND_RUN=1 "$tools/appimagetool" --no-appstream \
  --runtime-file "$tools/runtime" "$app_dir" "$out_dir/globlin-$arch.AppImage"
chmod +x "$out_dir/globlin-$arch.AppImage"
rm -rf "$tools"
