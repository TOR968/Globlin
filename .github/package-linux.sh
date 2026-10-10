#!/usr/bin/env bash
set -euo pipefail

out_dir="$1"
deb_arch="$(dpkg --print-architecture)"
rpm_arch="$(uname -m)"

mkdir -p "$out_dir"
cargo build --release
GLOBLIN_ICON_DIR="$PWD/target/linux-icons" cargo test -- --ignored --exact icon::tests::dump_linux_icons

cargo deb --no-build --output "$out_dir/globlin-$deb_arch.deb"
cargo generate-rpm --output "$out_dir/globlin-$rpm_arch.rpm"

dpkg-deb --info "$out_dir/globlin-$deb_arch.deb"
dpkg-deb --contents "$out_dir/globlin-$deb_arch.deb"
ls -l "$out_dir"
