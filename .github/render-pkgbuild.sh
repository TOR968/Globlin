#!/usr/bin/env bash
set -euo pipefail

version="$1"
sha_amd64="$2"
sha_arm64="$3"
out_dir="$4"

mkdir -p "$out_dir"
sed -e "s/@VERSION@/$version/" \
    -e "s/@SHA_AMD64@/$sha_amd64/" \
    -e "s/@SHA_ARM64@/$sha_arm64/" \
    packaging/aur/PKGBUILD.in > "$out_dir/PKGBUILD"
cat "$out_dir/PKGBUILD"
