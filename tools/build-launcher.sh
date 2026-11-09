#!/usr/bin/env bash
# Build the launcher against the same Nix glibc as the game, so the one glibc the overlay carries runs both.
#
#   tools/build-launcher.sh [OUT_FILE]       (default: dist/fta-launcher)
#
# Needs Nix (nix-shell). The compiler, linker and glibc come from the nixpkgs channel the game itself is built
# with (see shell-native.nix in the source repository); nothing here uses the host's glibc.
set -euo pipefail

repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
out=${1:-$repo/dist/fta-launcher}
command -v nix-shell >/dev/null || { printf 'build-launcher: nix-shell is required\n' >&2; exit 1; }
mkdir -p -- "$(dirname -- "$out")"

target=$repo/launcher/target-nix
( cd -- "$repo/launcher" && CARGO_TARGET_DIR=$target nix-shell -p cargo rustc gcc pkg-config fontconfig freetype --run 'cargo build --release --locked' )
cp -f -- "$target/release/fta-launcher" "$out"
printf 'Launcher: %s (%s)\n' "$out" "$(du -h "$out" | cut -f1)"
