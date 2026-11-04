#!/usr/bin/env bash
# From The Ashes native Linux client: one-click installer (self-extracting).
#
#   Double-click this file, or run:  bash FTA-Linux-Installer.run
#
# It finds your From The Ashes client folder (the one with Wow.exe), installs the Linux client into it,
# offers to install the system libraries it needs, adds a menu shortcut and offers to start the game.
# Nothing the Windows client uses is changed, except the small addon fixes bundled in this file (it lists
# them, asks first and keeps backups).
#
#   --client DIR     use this client folder instead of looking for one
#   --yes            answer the questions with their defaults (installs missing libraries with sudo);
#                    only installs into --client DIR or the folder you run it from
#   --no-shortcut    do not add the menu shortcut
#   --fixes PATH     apply this addon-fixes folder (or .tar.gz) instead of the ones bundled in this file
#   --no-fixes       do not apply addon fixes
set -euo pipefail

PAYLOAD_SHA256=@PAYLOAD_SHA256@

self=$(readlink -f -- "$0")
ORIGINAL_ARGS="$*"
client="" assume=0 shortcut=1 interm=0 pause=0 fixes_path="" use_fixes=1
out_tty=0; [[ -t 1 ]] && out_tty=1   # the log below replaces stdout, so ask now
while (( $# )); do
    case $1 in
        --client) client=${2:?--client needs a folder}; shift ;;
        --yes|-y) assume=1 ;;
        --no-shortcut) shortcut=0 ;;
        --fixes) fixes_path=${2:?--fixes needs a folder or .tar.gz}; shift ;;
        --no-fixes) use_fixes=0 ;;
        --in-terminal) interm=1 ;;
        --pause) pause=1 ;;
        --help|-h) sed -n '2,16p' "$self" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
    esac
    shift
done

# Opened from a file manager there is no terminal to show progress or ask for a password: open one.
if (( ! interm )) && [[ ! -t 0 || ! -t 1 ]]; then
    relaunch=("$self" --in-terminal --pause)
    (( assume )) && relaunch+=(--yes)
    (( ! shortcut )) && relaunch+=(--no-shortcut)
    (( ! use_fixes )) && relaunch+=(--no-fixes)
    [[ -n $fixes_path ]] && relaunch+=(--fixes "$fixes_path")
    [[ -n $client ]] && relaunch+=(--client "$client")
    for term in x-terminal-emulator gnome-terminal ptyxis konsole xfce4-terminal mate-terminal lxterminal tilix kitty alacritty foot xterm; do
        command -v "$term" >/dev/null 2>&1 || continue
        case $term in
            gnome-terminal|ptyxis) exec "$term" -- bash "${relaunch[@]}" ;;
            xfce4-terminal|mate-terminal) exec "$term" -x bash "${relaunch[@]}" ;;
            kitty|foot) exec "$term" bash "${relaunch[@]}" ;;
            *) exec "$term" -e bash "${relaunch[@]}" ;;
        esac
    done
    # No terminal emulator found: carry on with dialogs only.
fi

# ---- install log ----------------------------------------------------------------------------------
# Everything printed from here on is also kept in a log: Logs/fta-linux-install.log in the client folder (appended
# on every run), or ~/.local/state/fta-linux-client/install.log if the run ends before a client folder is chosen.
tmplog=$(mktemp)
exec 3>&1
exec > >(tee -a -- "$tmplog") 2>&1
log_saved=0
finish_log() {
    (( log_saved )) && return 0
    log_saved=1
    sleep 0.3   # let the tee process write the last lines
    local dest
    if [[ -n $client && -d $client ]]; then
        dest=$client/Logs/fta-linux-install.log
    else
        dest=${XDG_STATE_HOME:-$HOME/.local/state}/fta-linux-client/install.log
    fi
    if mkdir -p -- "$(dirname -- "$dest")" 2>/dev/null && cat -- "$tmplog" >>"$dest" 2>/dev/null; then
        printf '\nInstall log: %s\n(attach this file when reporting an install problem)\n' "$dest" >&3
    else
        printf '\nCould not save the install log; it is at %s\n' "$tmplog" >&3
        return 0
    fi
    rm -f -- "$tmplog"
}
hash_of() { if [[ -f $1 ]] && command -v sha256sum >/dev/null 2>&1; then sha256sum -- "$1" | cut -d' ' -f1; else echo 'none'; fi; }

printf '\n######## FTA Linux installer log: %s ########\n' "$(date '+%Y-%m-%d %H:%M:%S %z')"
printf 'installer file: %s\n' "$self"
printf 'installer sha256: %s\n' "$(hash_of "$self")"
printf 'payload sha256 (expected): %s\n' "$PAYLOAD_SHA256"
printf 'arguments: %s\n' "${ORIGINAL_ARGS:-(none)}"
printf 'working folder: %s\n' "$PWD"
printf 'system: %s | %s | session %s\n' "$(uname -srm)" \
    "$(. /etc/os-release 2>/dev/null && printf %s "${PRETTY_NAME:-unknown}" || echo unknown)" "${XDG_SESSION_TYPE:-unknown}"

pause_and_exit() {
    local code=$1
    finish_log
    if (( pause )); then read -r -p 'Press Enter to close this window. ' _ || true; fi
    exit "$code"
}
fail() {
    printf '\nInstall failed: %s\n' "$*" >&2
    if (( ! out_tty )) && command -v zenity >/dev/null 2>&1; then zenity --error --text="$*" 2>/dev/null || true; fi
    pause_and_exit 1
}
ask() { # ask "question" default(Y|N): returns 0 for yes
    local def=$2 reply
    if (( assume )); then [[ $def == Y ]]; return; fi
    if [[ ! -t 0 ]]; then [[ $def == Y ]]; return; fi
    read -r -p "$1 $([[ $def == Y ]] && echo '[Y/n]' || echo '[y/N]') " reply || reply=""
    case ${reply:-$def} in [yY]*) return 0 ;; *) return 1 ;; esac
}

printf '\n=== From The Ashes: native Linux client installer ===\n\n'

# ---- find the client folder --------------------------------------------------------------------
valid() { [[ -d $1/Data && -d $1/Interface ]]; }
candidates=()
add() { local d; d=$(cd -- "$1" 2>/dev/null && pwd -P) || return 0; valid "$d" || return 0
        local c; for c in "${candidates[@]:-}"; do [[ $c == "$d" ]] && return 0; done; candidates+=("$d"); }
if [[ -n $client ]]; then
    valid "$client" || fail "$client has no Data and Interface folders (give the folder that contains Wow.exe)"
    add "$client"
else
    add "$PWD"; add "$(dirname -- "$self")"; add "$(dirname -- "$self")/.."
    # A disk search is only ever offered to someone who can see and confirm the result: never with --yes.
    if (( ${#candidates[@]} == 0 && ! assume )); then
        printf 'Looking for your From The Ashes client folder (this can take a few seconds)...\n'
        while IFS= read -r exe; do add "$(dirname -- "$exe")"; done < <(
            timeout 25 find "$HOME" /mnt /media "/run/media/${USER:-$(id -un)}" -maxdepth 6 -name Wow.exe -type f 2>/dev/null || true)
    fi
fi
if (( ${#candidates[@]} == 0 )); then
    picked=""
    if (( ! out_tty )) && command -v zenity >/dev/null 2>&1; then
        picked=$(zenity --file-selection --directory --title='Select your From The Ashes client folder (with Wow.exe)' 2>/dev/null || true)
    elif (( ! out_tty )) && command -v kdialog >/dev/null 2>&1; then
        picked=$(kdialog --getexistingdirectory "$HOME" --title 'Select your From The Ashes client folder' 2>/dev/null || true)
    elif (( ! assume )) && [[ -t 0 ]]; then
        read -r -e -p 'Could not find it. Type or paste the path of your client folder (the one with Wow.exe): ' picked || true
        picked=${picked/#\~/$HOME}
    fi
    [[ -n $picked ]] || fail 'no client folder chosen. Run it from inside your client folder, or run again with --client /path/to/client'
    valid "$picked" || fail "$picked has no Data and Interface folders (give the folder that contains Wow.exe)"
    add "$picked"
fi
if (( ${#candidates[@]} == 1 )); then
    client=${candidates[0]}
    printf 'Client folder: %s\n' "$client"
    ask 'Install the Linux client into it?' Y || { printf 'Nothing installed.\n'; pause_and_exit 1; }
else
    (( assume )) && fail 'several client folders found; choose one with --client'
    printf 'Found several client folders:\n'
    for i in "${!candidates[@]}"; do printf '  %d) %s\n' $((i + 1)) "${candidates[$i]}"; done
    read -r -p 'Which one? [1] ' pick || pick=1
    pick=${pick:-1}
    [[ $pick =~ ^[0-9]+$ ]] && (( pick >= 1 && pick <= ${#candidates[@]} )) || fail 'not a valid choice'
    client=${candidates[$((pick - 1))]}
fi

# ---- unpack and install -------------------------------------------------------------------------
tmp=$(mktemp -d)
trap 'finish_log; rm -rf -- "$tmp"' EXIT
printf 'chosen client folder: %s\n' "$client"
printf 'folder check: Data %s, Interface %s, Wow.exe %s, existing Wow64 sha256 %s\n' \
    "$([[ -d $client/Data ]] && echo yes || echo NO)" "$([[ -d $client/Interface ]] && echo yes || echo NO)" \
    "$([[ -f $client/Wow.exe ]] && echo yes || echo no)" "$(hash_of "$client/Wow64")"
offset=$(LC_ALL=C awk '/^__PAYLOAD__$/ { print NR + 1; exit }' "$self")
[[ -n $offset ]] || fail 'this installer file is damaged (no payload). Download it again.'
if command -v sha256sum >/dev/null 2>&1 && [[ $PAYLOAD_SHA256 != @* ]]; then
    actual=$(tail -n +"$offset" "$self" | sha256sum | cut -d' ' -f1)
    if [[ $actual != "$PAYLOAD_SHA256" ]]; then
        size=$(stat -c %s -- "$self" 2>/dev/null || echo '?')
        fail "the installer file is damaged (checksum mismatch; file size $size bytes, expected about 30 MB). Copy or download it again, and compare its sha256sum with FTA-Linux-SHA256SUMS.txt."
    fi
fi
printf 'Unpacking...\n'
tail -n +"$offset" "$self" | tar -xz -C "$tmp"
printf 'payload Wow64 sha256: %s\n' "$(hash_of "$tmp/payload/Wow64")"
"$tmp/install.sh" "$client" --no-deps || fail 'the installer could not copy the files (see above)'
printf 'installed Wow64 sha256: %s\n' "$(hash_of "$client/Wow64")"
[[ $(hash_of "$client/Wow64") == "$(hash_of "$tmp/payload/Wow64")" ]] || fail 'the installed Wow64 does not match the one in the installer (the file may be in use; close the game and run the installer again)'

# ---- optional addon fixes ------------------------------------------------------------------------
# The folder FTA-Addon-Fixes travels inside this installer (--fixes PATH names another folder or .tar.gz). It
# holds small fixes for addons in the base client (see its README.txt). Each file is changed only if it is the
# version the fix was made for, or if the patch still applies cleanly; the originals are kept in
# FTA-Linux/addon-fix-backup/. Nothing here can make the install fail.
fixes_dir=""
fixes_origin=""
fixes_resolve() { # fixes_resolve PATH: sets fixes_dir if PATH is a usable fixes folder or archive
    local p=$1 x
    if [[ -d $p && -f $p/fixes.manifest ]]; then fixes_dir=$p; return 0; fi
    if [[ -f $p && $p == *.tar.gz ]]; then
        x=$(mktemp -d -p "$tmp") || return 1
        if tar -xzf "$p" -C "$x" 2>/dev/null; then
            local m
            m=$(find "$x" -maxdepth 2 -name fixes.manifest -type f | head -n 1)
            if [[ -n $m ]]; then fixes_dir=$(dirname -- "$m"); return 0; fi
        fi
    fi
    return 1
}
fixes_find() {
    [[ -n $fixes_path ]] && { fixes_resolve "$fixes_path" && fixes_origin=$fixes_path; return; }
    fixes_resolve "$tmp/FTA-Addon-Fixes" && fixes_origin="the ones bundled with this installer"
}
fixes_backup_dir=""
fixes_backup() { # fixes_backup RELPATH: keeps the current file before it changes
    [[ -n $fixes_backup_dir ]] || fixes_backup_dir=$client/FTA-Linux/addon-fix-backup/$(date +%Y%m%d-%H%M%S)
    mkdir -p -- "$fixes_backup_dir/$(dirname -- "$1")" && cp -p -- "$client/$1" "$fixes_backup_dir/$1"
}
fixes_apply() {
    local rel orig fixed patchf target cur applied=0 same=0 skipped=0 patchfile
    while IFS=$'\t' read -r rel orig fixed patchf _; do
        [[ -z $rel || $rel == \#* ]] && continue
        case $rel in
            Interface/AddOns/*) ;;
            *) printf '  skipped %s (only files under Interface/AddOns are accepted)\n' "$rel"; skipped=$((skipped + 1)); continue ;;
        esac
        if [[ $rel == *..* || $patchf == */* ]]; then
            printf '  skipped %s (unsafe path)\n' "$rel"; skipped=$((skipped + 1)); continue
        fi
        target=$client/$rel
        patchfile=$fixes_dir/patches/$patchf
        if [[ ! -f $target ]]; then
            printf '  skipped %s (not installed in this client)\n' "$rel"; skipped=$((skipped + 1)); continue
        fi
        cur=$(hash_of "$target")
        if [[ $cur == "$fixed" ]]; then
            printf '  already applied: %s\n' "$rel"; same=$((same + 1)); continue
        fi
        if [[ $cur == "$orig" && -f $fixes_dir/$rel && $(hash_of "$fixes_dir/$rel") == "$fixed" ]]; then
            if fixes_backup "$rel" && cp -- "$fixes_dir/$rel" "$target"; then
                printf '  fixed: %s\n' "$rel"; applied=$((applied + 1))
            else
                printf '  could not write %s\n' "$rel"; skipped=$((skipped + 1))
            fi
            continue
        fi
        if [[ -f $patchfile ]] && command -v patch >/dev/null 2>&1; then
            if patch -s -R -p1 -d "$client" --dry-run -i "$patchfile" >/dev/null 2>&1; then
                printf '  already applied: %s\n' "$rel"; same=$((same + 1)); continue
            fi
            if patch -s -p1 -d "$client" --dry-run -i "$patchfile" >/dev/null 2>&1; then
                if fixes_backup "$rel" && patch -s -p1 -d "$client" -i "$patchfile" >/dev/null 2>&1; then
                    printf '  patched: %s\n' "$rel"; applied=$((applied + 1))
                else
                    printf '  could not patch %s (restoring)\n' "$rel"
                    [[ -f $fixes_backup_dir/$rel ]] && cp -p -- "$fixes_backup_dir/$rel" "$target"
                    skipped=$((skipped + 1))
                fi
                continue
            fi
        fi
        printf '  left unchanged: %s (a different version from the one these fixes were made for%s)\n' "$rel" \
            "$(command -v patch >/dev/null 2>&1 || printf ', and the patch tool is not installed')"
        skipped=$((skipped + 1))
    done <"$fixes_dir/fixes.manifest"
    printf 'Addon fixes: %d applied, %d already in place, %d skipped.\n' "$applied" "$same" "$skipped"
    [[ -z $fixes_backup_dir ]] || printf 'Originals kept in %s\n' "$fixes_backup_dir"
}
if (( use_fixes )); then
    if fixes_find; then
        printf '\nAddon fixes: %s\n' "$fixes_origin"
        printf 'It changes these files in your client folder:\n'
        while IFS=$'\t' read -r rel _ _ _ note; do
            [[ -z $rel || $rel == \#* ]] && continue
            printf '  %s\n' "$rel"
            [[ -z ${note:-} ]] || printf '    %s\n' "$note"
        done <"$fixes_dir/fixes.manifest"
        if ask 'Apply these addon fixes?' Y; then
            fixes_apply || printf 'Addon fixes could not be applied (the install itself is fine).\n'
        else
            printf 'Addon fixes skipped.\n'
        fi
    elif [[ -n $fixes_path ]]; then
        printf '\nNo usable addon fixes at %s (it needs a fixes.manifest); skipped.\n' "$fixes_path"
    else
        printf '\nNo addon fixes are bundled with this installer.\n'
    fi
fi

# ---- system libraries ----------------------------------------------------------------------------
printf '\nChecking system libraries...\n'
if ! "$client/FTA-Linux/install-deps.sh"; then
    if ask 'Install the missing system libraries now (asks for your password)?' Y; then
        "$client/FTA-Linux/install-deps.sh" --install --yes || fail 'the libraries could not be installed. Run FTA-Linux/install-deps.sh to see the exact command.'
    else
        printf 'Skipped. Run FTA-Linux/install-deps.sh --install in the client folder when you are ready.\n'
    fi
fi

# ---- menu shortcut -------------------------------------------------------------------------------
if (( shortcut )); then
    apps=${XDG_DATA_HOME:-$HOME/.local/share}/applications
    mkdir -p -- "$apps"
    {
        printf '[Desktop Entry]\nType=Application\nName=From The Ashes (Linux)\n'
        printf 'Comment=Native Linux client for From The Ashes\n'
        printf 'Exec="%s/FTA-Linux/play.sh"\nPath=%s\nTerminal=false\nCategories=Game;\n' "$client" "$client"
    } >"$apps/fta-linux-client.desktop"
    printf 'Added "From The Ashes (Linux)" to your applications menu.\n'
fi

printf '\nDone. Start it any time with:  %s/FTA-Linux/play.sh\n' "$client"
if ask 'Start the game now?' N; then
    pause=0
    "$client/FTA-Linux/play.sh" || true
fi
pause_and_exit 0
exit 0
__PAYLOAD__
