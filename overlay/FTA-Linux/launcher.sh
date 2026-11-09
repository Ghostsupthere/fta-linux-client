#!/usr/bin/env bash
# Start the launcher (update check, settings, diagnostics and the Play button) of the Linux client installed in
# this From The Ashes client folder. FTA-Linux/play.sh starts the game without it.
#   FTA-Linux/launcher.sh [launcher arguments]
set -euo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
client=$(dirname -- "$here")
cd -- "$client"

fail() {
    printf 'FTA Linux launcher: %s\n' "$*" >&2
    if command -v notify-send >/dev/null; then notify-send 'FTA Linux Launcher' "$*" 2>/dev/null || true; fi
    exit 1
}
[[ -x $here/fta-launcher ]] || fail "the launcher is not installed in $client (run the installer again). FTA-Linux/play.sh starts the game without it"
[[ -x $here/lib/ld-linux-x86-64.so.2 ]] || fail "FTA-Linux/lib is missing (run the installer again)"

# Same library order as play.sh: the overlay's own libraries first, then the system's (Wayland, X11, GL and
# the graphics driver come from the system). The loader is given the path directly instead of through
# LD_LIBRARY_PATH, so the programs the launcher starts (the installer, xdg-open, the game) do not inherit it.
libpath="$here/lib:$here/dxvk"
for dir in /usr/lib /usr/lib64 /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu /lib64 /lib /usr/local/lib; do
    [[ -d $dir ]] && libpath+=":$dir"
done
unset LD_LIBRARY_PATH
listing=$("$here/lib/ld-linux-x86-64.so.2" --library-path "$libpath" --list "$here/fta-launcher" 2>&1) || fail "the launcher cannot start: $listing"
missing=$(grep -oE '[A-Za-z0-9_.+-]+\.so[.0-9]*' <<<"$(grep 'not found' <<<"$listing")" | sort -u | tr '\n' ' ' || true)
[[ -z $missing ]] || fail "system libraries are missing: $missing. Run FTA-Linux/install-deps.sh --install"
exec "$here/lib/ld-linux-x86-64.so.2" --library-path "$libpath" "$here/fta-launcher" --client "$client" "$@"
