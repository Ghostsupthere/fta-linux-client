#!/usr/bin/env bash
# Package the Linux client for release: the install archive, the one-click installer and the checksums.
#
#   tools/package.sh PAYLOAD_DIR [OUT_DIR]
#
# PAYLOAD_DIR holds the binary parts, which are built from the source tree (see the README, "Releasing"):
#   Wow64                                 the client executable (loader path rewritten to FTA-Linux/lib)
#   FTA-Linux/lib/                        glibc, loader, C++ runtime, xvidcore
#   FTA-Linux/dxvk/                       dxvk-native (d3d9, d3d11, dxgi)
#   FTA-Linux/VERSION.txt                 build stamp (optional)
#   Utils/WarcraftXL/visuals/*.pack       offline shader packs (optional)
# The scripts and README come from this repository. Output (OUT_DIR, default ./dist):
#   fta-linux-client.tar.gz   FTA-Linux-Installer.run   FTA-Linux-SHA256SUMS.txt
set -euo pipefail

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
payload=${1:?usage: tools/package.sh PAYLOAD_DIR [OUT_DIR]}
out=${2:-$repo/dist}
fail() { printf 'package: %s\n' "$*" >&2; exit 1; }

[[ -x $payload/Wow64 ]] || fail "$payload/Wow64 is missing"
[[ -d $payload/FTA-Linux/lib && -d $payload/FTA-Linux/dxvk ]] || fail "$payload/FTA-Linux/lib and /dxvk are required"
[[ -x $payload/FTA-Linux/lib/ld-linux-x86-64.so.2 ]] || fail 'the bundled loader FTA-Linux/lib/ld-linux-x86-64.so.2 is missing'
mkdir -p -- "$out"
out=$(cd -- "$out" && pwd -P)

stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
mkdir -p -- "$stage/payload"
cp -a -- "$payload/." "$stage/payload/"
cp -- "$repo/overlay/FTA-Linux/play.sh" "$repo/overlay/FTA-Linux/install-deps.sh" "$stage/payload/FTA-Linux/"
cp -- "$repo/overlay/install.sh" "$stage/install.sh"
cp -- "$repo/README.md" "$stage/README.md"
chmod +x "$stage/install.sh" "$stage/payload/Wow64" "$stage/payload/FTA-Linux/play.sh" "$stage/payload/FTA-Linux/install-deps.sh"
if find "$stage" -type l | grep -q .; then fail 'the payload contains symlinks'; fi

tarball=$out/fta-linux-client.tar.gz
installer=$out/FTA-Linux-Installer.run
tar -C "$stage" --sort=name --owner=0 --group=0 --numeric-owner -cf - . | gzip -n >"$tarball"
sum=$(sha256sum "$tarball" | cut -d' ' -f1)
# Written aside and moved into place, so nobody copies a half-written file.
{ sed "s/@PAYLOAD_SHA256@/$sum/" "$repo/installer/installer-header.sh"; cat "$tarball"; } >"$installer.tmp"
chmod +x "$installer.tmp"
mv -f -- "$installer.tmp" "$installer"
( cd -- "$out" && sha256sum FTA-Linux-Installer.run fta-linux-client.tar.gz >FTA-Linux-SHA256SUMS.txt )

printf 'Archive:   %s (%s)\nInstaller: %s (%s)\nChecksums: %s\n' "$tarball" "$(du -sh "$tarball" | cut -f1)" \
    "$installer" "$(du -sh "$installer" | cut -f1)" "$out/FTA-Linux-SHA256SUMS.txt"
