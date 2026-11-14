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
#   FTA-Linux/fta-launcher                the launcher (optional; tools/build-launcher.sh builds it)
#   Utils/WarcraftXL/visuals/*.pack       offline shader packs (optional)
# The addon fixes (the folder FTA-Addon-Fixes built by tools/make-addon-fixes.sh) are taken from ADDON_FIXES_DIR,
# default OUT_DIR/FTA-Addon-Fixes, and travel inside the installer; they are not a separate download.
# RELEASE_TAG (for example v0.2.10) is recorded in FTA-Linux/RELEASE so the launcher knows which release is
# installed; leave it out for a development build.
# Addons that ship with the installer come from ADDONS_DIR, default addons/ in this repository (one folder per addon;
# tools/update-realmdesk.sh refreshes addons/RealmDesk from a player ZIP). They travel inside the installer as FTA-Addons/.
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
cp -- "$repo/overlay/FTA-Linux/play.sh" "$repo/overlay/FTA-Linux/install-deps.sh" "$repo/overlay/FTA-Linux/icon.png" "$stage/payload/FTA-Linux/"
if [[ -f $payload/FTA-Linux/fta-launcher ]]; then
    cp -- "$repo/overlay/FTA-Linux/launcher.sh" "$stage/payload/FTA-Linux/"
    chmod +x "$stage/payload/FTA-Linux/launcher.sh" "$stage/payload/FTA-Linux/fta-launcher"
    printf 'Launcher bundled.\n'
else
    printf 'No launcher at %s; the installer will not carry one.\n' "$payload/FTA-Linux/fta-launcher"
fi
if [[ -n ${RELEASE_TAG:-} ]]; then
    [[ $RELEASE_TAG =~ ^v[0-9]+(\.[0-9]+)*$ ]] || fail "RELEASE_TAG must look like v0.2.10, not '$RELEASE_TAG'"
    printf '%s\n' "$RELEASE_TAG" >"$stage/payload/FTA-Linux/RELEASE"
    printf 'Release recorded: %s\n' "$RELEASE_TAG"
fi
cp -- "$repo/overlay/install.sh" "$stage/install.sh"
cp -- "$repo/README.md" "$stage/README.md"
fixes=${ADDON_FIXES_DIR:-$out/FTA-Addon-Fixes}
if [[ -f $fixes/fixes.manifest ]]; then
    cp -a -- "$fixes" "$stage/FTA-Addon-Fixes"
    printf 'Addon fixes bundled from %s\n' "$fixes"
else
    printf 'No addon fixes found at %s; the installer will have none.\n' "$fixes"
fi
addons=${ADDONS_DIR:-$repo/addons}
if [[ -d $addons/RealmDesk ]]; then
    # The installer treats RealmDeskConfig.lua as the player's own file; only the empty template may ship.
    [[ -f $addons/RealmDesk/RealmDesk.toc && -f $addons/RealmDesk/MANIFEST.sha256 ]] || fail "$addons/RealmDesk is not a RealmDesk player bundle"
    (cd -- "$addons/RealmDesk" && sha256sum -c --quiet MANIFEST.sha256) || fail "$addons/RealmDesk does not match its MANIFEST.sha256"
    [[ ! -e $addons/RealmDesk/Media/Avatars ]] || fail "$addons/RealmDesk/Media/Avatars must not ship"
    (( $(stat -c %s -- "$addons/RealmDesk/RealmDeskConfig.lua") < 512 )) || fail "$addons/RealmDesk/RealmDeskConfig.lua is not the empty template"
    mkdir -p -- "$stage/FTA-Addons"
    cp -a -- "$addons/RealmDesk" "$stage/FTA-Addons/RealmDesk"
    printf 'Addon bundled: RealmDesk %s\n' "$(sed -n 's/^## Version: *//p' "$addons/RealmDesk/RealmDesk.toc")"
else
    printf 'No RealmDesk addon at %s; the installer will not carry it.\n' "$addons"
fi
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
