#!/usr/bin/env bash
# Refresh addons/RealmDesk from a RealmDesk player ZIP (RealmDesk-X.Y.Z-players.zip, built by the RealmDesk project).
#
#   tools/update-realmdesk.sh PLAYERS_ZIP
#
# Checks the ZIP against its .sha256 file (when present) and against the MANIFEST.sha256 inside it, refuses anything
# that looks like a personal config or custom avatars, then replaces addons/RealmDesk with the ZIP's contents.
# tools/package.sh bundles addons/ into the installer; the installer copies it to Interface/AddOns/RealmDesk.
set -euo pipefail

zip=${1:?usage: tools/update-realmdesk.sh PLAYERS_ZIP}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
fail() { printf 'update-realmdesk: %s\n' "$*" >&2; exit 1; }
[[ -f $zip ]] || fail "$zip does not exist"
if [[ -f $zip.sha256 ]]; then
    want=$(cut -d' ' -f1 <"$zip.sha256")
    [[ $(sha256sum -- "$zip" | cut -d' ' -f1) == "$want" ]] || fail "$zip does not match $zip.sha256"
fi
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
unzip -q -- "$zip" -d "$work"
[[ -f $work/RealmDesk/RealmDesk.toc && -f $work/RealmDesk/MANIFEST.sha256 ]] || fail 'the ZIP needs RealmDesk/RealmDesk.toc and RealmDesk/MANIFEST.sha256 at its top'
[[ $(find "$work" -mindepth 1 -maxdepth 1 | wc -l) -eq 1 ]] || fail 'the ZIP must hold only the RealmDesk folder'
(cd -- "$work/RealmDesk" && sha256sum -c --quiet MANIFEST.sha256) || fail 'a file does not match MANIFEST.sha256'
# Every file must be listed in the manifest (MANIFEST.sha256 itself excepted), and nothing personal may travel.
extra=$(cd -- "$work/RealmDesk" && find . -type f ! -name MANIFEST.sha256 | sed 's|^\./||' | sort | comm -23 - <(awk '{print $2}' "$work/RealmDesk/MANIFEST.sha256" | sort))
[[ -z $extra ]] || fail "files not in the manifest: $extra"
[[ ! -e $work/RealmDesk/Media/Avatars ]] || fail 'the ZIP carries Media/Avatars (a person'"'"'s custom pictures)'
(( $(stat -c %s -- "$work/RealmDesk/RealmDeskConfig.lua") < 512 )) || fail 'RealmDeskConfig.lua is not the empty template (it may hold someone'"'"'s imported settings)'
rm -rf -- "$repo/addons/RealmDesk"
mkdir -p -- "$repo/addons"
mv -- "$work/RealmDesk" "$repo/addons/RealmDesk"
printf 'addons/RealmDesk is now %s\n' "$(sed -n 's/^## Version: *//p' "$repo/addons/RealmDesk/RealmDesk.toc")"
