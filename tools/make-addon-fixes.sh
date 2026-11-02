#!/usr/bin/env bash
# Build the optional addon-fixes side payload that the installer applies when it finds it next to itself.
#
#   tools/make-addon-fixes.sh BEFORE_DIR AFTER_DIR [OUT_DIR] [README_FILE]
#
# BEFORE_DIR and AFTER_DIR are two copies of the client's folder tree holding only Interface/AddOns/... files:
# the originals and the fixed versions. Every file that differs becomes a manifest entry, a patch and a drop-in
# copy. New files are not supported (the installer only fixes files the client already has).
# Output (OUT_DIR, default ./dist): FTA-Addon-Fixes/ (the folder) and FTA-Addon-Fixes.tar.gz.
set -euo pipefail

before=${1:?usage: tools/make-addon-fixes.sh BEFORE_DIR AFTER_DIR [OUT_DIR] [README_FILE]}
after=${2:?usage: tools/make-addon-fixes.sh BEFORE_DIR AFTER_DIR [OUT_DIR] [README_FILE]}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
out=${3:-$repo/dist}
readme=${4:-}
fail() { printf 'make-addon-fixes: %s\n' "$*" >&2; exit 1; }
[[ -d $before/Interface/AddOns && -d $after/Interface/AddOns ]] || fail 'both trees need an Interface/AddOns folder'
mkdir -p -- "$out"
out=$(cd -- "$out" && pwd -P)
dest=$out/FTA-Addon-Fixes
rm -rf -- "${dest:?}"
mkdir -p -- "$dest/patches"
printf '# relative path\toriginal sha256\tfixed sha256\tpatch\n' >"$dest/fixes.manifest"

n=0
while IFS= read -r -d '' file; do
    rel=${file#"$after"/}
    [[ -f $before/$rel ]] || fail "$rel is new; only changes to existing files are supported"
    cmp -s -- "$before/$rel" "$after/$rel" && continue
    n=$((n + 1))
    patch=$(printf '%02d-%s.patch' "$n" "$(basename -- "$rel" | tr -c 'A-Za-z0-9.\n_-' '_')")
    # diff exits 1 when the files differ; the labels give the patch a/ b/ paths for patch -p1.
    diff -u --label "a/$rel" --label "b/$rel" -- "$before/$rel" "$after/$rel" >"$dest/patches/$patch" || [[ $? -eq 1 ]]
    mkdir -p -- "$dest/$(dirname -- "$rel")"
    cp -- "$after/$rel" "$dest/$rel"
    printf '%s\t%s\t%s\t%s\n' "$rel" "$(sha256sum -- "$before/$rel" | cut -d' ' -f1)" \
        "$(sha256sum -- "$after/$rel" | cut -d' ' -f1)" "$patch" >>"$dest/fixes.manifest"
done < <(find "$after/Interface/AddOns" -type f -print0 | sort -z)
(( n )) || fail 'no differing files found'

if [[ -n $readme ]]; then cp -- "$readme" "$dest/README.txt"; fi
tar -C "$out" --sort=name --owner=0 --group=0 --numeric-owner -cf - FTA-Addon-Fixes | gzip -n >"$out/FTA-Addon-Fixes.tar.gz"
printf 'Folder:  %s (%d file(s))\nArchive: %s\nsha256:  %s\n' "$dest" "$n" "$out/FTA-Addon-Fixes.tar.gz" \
    "$(sha256sum -- "$out/FTA-Addon-Fixes.tar.gz" | cut -d' ' -f1)"
