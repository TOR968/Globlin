#!/usr/bin/env bash
set -euo pipefail

deb="$1"
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
work="$(mktemp -d)"

bash .github/render-pkgbuild.sh "$version" "$(sha256sum "$deb" | cut -d' ' -f1)" SKIP "$work"
cp "$deb" "$work/globlin-$version-amd64.deb"
chmod -R a+rwX "$work"

docker run --rm -v "$work:/pkg" archlinux:base-devel bash -euo pipefail -c '
    pacman -Syu --noconfirm --needed namcap >/dev/null
    useradd -m builder
    echo "builder ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/builder
    cp -r /pkg /home/builder/pkg
    chown -R builder /home/builder/pkg
    cd /home/builder/pkg
    sudo -u builder makepkg --printsrcinfo > .SRCINFO
    cat .SRCINFO
    sudo -u builder makepkg --syncdeps --install --noconfirm
    test -x /usr/bin/globlin
    if ldd /usr/bin/globlin | grep "not found"; then exit 1; fi
    test -f /usr/share/applications/globlin.desktop
    test -f /usr/share/icons/hicolor/256x256/apps/globlin.png
    namcap PKGBUILD || true
    pacman -Qi globlin-bin
    pacman -R --noconfirm globlin-bin
    test ! -e /usr/bin/globlin
'
