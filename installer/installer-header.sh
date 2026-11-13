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
#   --login-screens  download the login screens (about 207 MB from Google Drive) without asking
#   --no-login-screens  do not offer the login screens
#   --murloc         download the Murloc race patch (about 13 MB from Google Drive) without asking
#   --no-murloc      do not install the Murloc race patch, now or in later updates (the launcher's Update remembers this)
#   --murloc-file F  install the Murloc patch from this local patch-ZD.mpq instead of downloading it
#   --no-texture-level  do not set componentTextureLevel 9 in WTF/Config.wtf (see the Murloc skins below)
#   --update         for the launcher's Update button: no questions, no sudo, no terminal; keeps the shortcut
#                    (pointing it at the launcher), applies addon fixes only if they were applied before
set -euo pipefail

PAYLOAD_SHA256=@PAYLOAD_SHA256@

# The login screens (FTA-Login-Screens.zip: ten packs for the launcher's Login tab) are a separate download, kept
# on Google Drive. The installer offers to fetch them; the file must match this checksum or nothing is installed from it.
LOGIN_SCREENS_URL='https://drive.google.com/file/d/13WPuTZ77LCYdlWmD3Ia8pWvgxXBau5E-/view?usp=sharing'
LOGIN_SCREENS_ID=13WPuTZ77LCYdlWmD3Ia8pWvgxXBau5E-
LOGIN_SCREENS_SHA256=a03c6881b1ece9525020972ffbcc3f510f6b9d58c94a166b800f0d13179040d3

# The Murloc race patch (patch-ZD.mpq, about 12 MB: the custom playable Murloc race of the From The Ashes realm) is a separate download too,
# because it carries game models and textures. The file must match this checksum or nothing is installed from it.
MURLOC_PATCH_URL='https://drive.google.com/file/d/1r84grn0yiBx3A063Vl-5wrkK9rwi2gm0/view?usp=sharing'
MURLOC_PATCH_ID=1r84grn0yiBx3A063Vl-5wrkK9rwi2gm0
MURLOC_PATCH_SHA256=df580c5ef1afaa8f31e6c962b0490e04ad8c940e3f6818c6cf9adb51d2fbb156

self=$(readlink -f -- "$0")
ORIGINAL_ARGS="$*"
client="" assume=0 shortcut=1 interm=0 pause=0 fixes_path="" use_fixes=1 update=0 login_dl=ask murloc_dl=ask murloc_file="" texture_level=1
out_tty=0; [[ -t 1 ]] && out_tty=1   # the log below replaces stdout, so ask now
while (( $# )); do
    case $1 in
        --client) client=${2:?--client needs a folder}; shift ;;
        --yes|-y) assume=1 ;;
        --no-shortcut) shortcut=0 ;;
        --fixes) fixes_path=${2:?--fixes needs a folder or .tar.gz}; shift ;;
        --no-fixes) use_fixes=0 ;;
        --login-screens) login_dl=yes ;;
        --no-login-screens) login_dl=no ;;
        --no-texture-level) texture_level=0 ;;
        --murloc) murloc_dl=yes ;;
        --no-murloc) murloc_dl=no ;;
        --murloc-file) murloc_file=${2:?--murloc-file needs a patch-ZD.mpq}; murloc_dl=yes; shift ;;
        --update) update=1; assume=1; interm=1 ;;
        --in-terminal) interm=1 ;;
        --pause) pause=1 ;;
        --help|-h) sed -n '2,24p' "$self" | sed 's/^# \{0,1\}//'; exit 0 ;;
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
    [[ $login_dl == yes ]] && relaunch+=(--login-screens)
    [[ $login_dl == no ]] && relaunch+=(--no-login-screens)
    [[ $murloc_dl == yes && -z $murloc_file ]] && relaunch+=(--murloc)
    [[ $murloc_dl == no ]] && relaunch+=(--no-murloc)
    (( ! texture_level )) && relaunch+=(--no-texture-level)
    [[ -n $murloc_file ]] && relaunch+=(--murloc-file "$murloc_file")
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
    if (( ! out_tty && ! update )) && command -v zenity >/dev/null 2>&1; then zenity --error --text="$*" 2>/dev/null || true; fi
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
        if [[ ",$orig," == *",$cur,"* && -f $fixes_dir/$rel && $(hash_of "$fixes_dir/$rel") == "$fixed" ]]; then
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
fixes_applied_before() { # true if this client has had addon fixes applied by an earlier install
    local rel orig fixed _
    [[ -d $client/FTA-Linux/addon-fix-backup ]] && return 0
    while IFS=$'\t' read -r rel orig fixed _; do
        [[ -z $rel || $rel == \#* ]] && continue
        [[ -f $client/$rel && $(hash_of "$client/$rel") == "$fixed" ]] && return 0
    done <"$fixes_dir/fixes.manifest"
    return 1
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
        # An update from the launcher asks nothing, so it only carries on what the person accepted before.
        if (( update )); then
            if fixes_applied_before; then
                printf 'Addon fixes were applied here before; applying the current ones.\n'
                fixes_apply || printf 'Addon fixes could not be applied (the install itself is fine).\n'
            else
                printf 'Addon fixes were not applied here before, so they are left out. Run the installer by hand to review them.\n'
            fi
        elif ask 'Apply these addon fixes?' Y; then
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

# ---- optional login screens ----------------------------------------------------------------------
# A separate download (about 207 MB) kept on Google Drive: ten small packs, installed as
# Data/LoginScreens/login-<name>.mpq, that the launcher's Login tab chooses between. The result must match
# LOGIN_SCREENS_SHA256, and only files named login-*.mpq are copied out of it. Nothing here can make the install fail.
drive_fetch() { # drive_fetch FILE_ID OUTFILE: downloads a public Google Drive file (big files skip the scan page with confirm=t)
    curl -fsS -L --retry 3 --max-time 3600 -o "$2" "https://drive.usercontent.google.com/download?id=$1&export=download&confirm=t"
}
login_screens_manual() {
    printf 'To add them by hand: download %s\nand extract the archive inside %s/Data, so the files are in Data/LoginScreens/.\n' "$LOGIN_SCREENS_URL" "$client"
}
login_screens() {
    local have dl="" got="" x
    have=$(find "$client/Data/LoginScreens" -maxdepth 1 -name 'login-*.mpq' 2>/dev/null | wc -l)
    if (( have >= 10 )); then printf '\nLogin screens: already installed (%d) in Data/LoginScreens.\n' "$have"; return 0; fi
    printf '\nLogin screens: ten themed login and character-select screens you can pick in the launcher (Login tab).\n'
    printf 'They are a separate download of about 207 MB from Google Drive and go into Data/LoginScreens (nothing else is changed).\n'
    if [[ $login_dl != yes ]] && ! ask 'Download them now?' N; then
        printf 'Skipped. You can add them later; the launcher'\''s Login tab has a download button.\n'
        return 0
    fi
    for x in curl sha256sum; do
        command -v "$x" >/dev/null 2>&1 || { printf 'Cannot download them here: %s is not installed.\n' "$x"; login_screens_manual; return 0; }
    done
    local free; free=$(df -Pk -- "$client/Data" 2>/dev/null | awk 'NR==2 { print int($4 / 1024) }')
    if [[ -n $free ]] && (( free < 700 )); then printf 'Not enough free space in %s/Data (%s MB free, about 700 MB needed).\n' "$client" "$free"; return 0; fi
    dl=$(mktemp -d -p "$client/Data" .fta-login-download.XXXXXX) || { printf 'Cannot write to %s/Data.\n' "$client"; return 0; }
    printf 'Downloading (a few minutes)...\n'
    if ! drive_fetch "$LOGIN_SCREENS_ID" "$dl/screens.zip"; then
        printf 'The download failed (no network, or Google Drive refused it, for example when the daily download limit is reached).\n'; login_screens_manual; rm -rf -- "$dl"; return 0
    fi
    got=$(hash_of "$dl/screens.zip")
    if [[ $got != "$LOGIN_SCREENS_SHA256" ]]; then
        printf 'The downloaded file is not the expected one (sha256 %s), so it was not installed.\n' "$got"
        login_screens_manual; rm -rf -- "$dl"; return 0
    fi
    mkdir -p -- "$dl/x"
    if command -v unzip >/dev/null 2>&1; then unzip -q -o "$dl/screens.zip" -d "$dl/x" >/dev/null 2>&1 || true
    elif command -v python3 >/dev/null 2>&1; then python3 -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])' "$dl/screens.zip" "$dl/x" 2>/dev/null || true
    elif command -v bsdtar >/dev/null 2>&1; then bsdtar -xf "$dl/screens.zip" -C "$dl/x" 2>/dev/null || true
    fi
    mkdir -p -- "$client/Data/LoginScreens"
    local n=0 f
    for f in "$dl"/x/LoginScreens/login-*.mpq; do
        [[ -f $f ]] || continue
        cp -f -- "$f" "$client/Data/LoginScreens/$(basename -- "$f")" && n=$((n + 1))
    done
    rm -rf -- "$dl"
    if (( n )); then printf 'Installed %d login screens in Data/LoginScreens. Pick one in the launcher (Login tab).\n' "$n"
    else printf 'The archive could not be unpacked (it needs unzip, python3 or bsdtar).\n'; login_screens_manual; fi
    return 0
}
if (( update )); then
    printf '\nLogin screens: not touched by an update.\n'
elif [[ $login_dl != no ]]; then
    login_screens || printf 'Login screens could not be set up (the install itself is fine).\n'
fi

# ---- optional Murloc race patch ------------------------------------------------------------------
# One file, Data/patch-ZD.mpq: the Murloc playable race (model, nine skin colours, the matching game tables and character-creation
# screen). Without it a client cannot create a Murloc. It is a separate download (about 12 MB, Google Drive) that must match
# MURLOC_PATCH_SHA256, or a local file given with --murloc-file. An older copy is kept in FTA-Linux/murloc-backup/. To remove the race
# from this client, delete Data/patch-ZD.mpq. Nothing here can make the install fail.
murloc_manual() {
    printf 'To add it by hand: download %s\nand put the file patch-ZD.mpq in %s/Data.\n' "$MURLOC_PATCH_URL" "$client"
}
murloc_patch() {
    local dest=$client/Data/patch-ZD.mpq have src dl="" got
    have=$(hash_of "$dest")
    if [[ $have == "$MURLOC_PATCH_SHA256" ]]; then printf '\nMurloc race patch: already installed.\n'; return 0; fi
    if [[ $murloc_dl == yes ]]; then rm -f -- "$client/FTA-Linux/no-murloc"
    elif [[ -f $client/FTA-Linux/no-murloc ]]; then
        printf '\nMurloc race patch: left out (you opted out earlier). Run the installer by hand with --murloc to add it.\n'; return 0
    fi
    printf '\nMurloc race patch: adds the Murloc playable race (Alliance) to the character-creation screen and the game tables.\n'
    printf 'It is one 13 MB file, Data/patch-ZD.mpq; delete it to remove the race again.\n'
    [[ -f $dest ]] && printf 'A different patch-ZD.mpq is installed here; it will be kept in FTA-Linux/murloc-backup/.\n'
    (( update )) && printf 'An update installs it so the race shows up. To opt out, run the installer once with --no-murloc; updates then leave it alone.\n'
    if [[ $murloc_dl != yes ]] && ! ask 'Install it now?' Y; then printf 'Skipped.\n'; return 0; fi
    if [[ -n $murloc_file ]]; then
        [[ -f $murloc_file ]] || { printf 'Local file %s does not exist.\n' "$murloc_file"; return 0; }
        src=$murloc_file
    else
        local x
        for x in curl sha256sum; do
            command -v "$x" >/dev/null 2>&1 || { printf 'Cannot download it here: %s is not installed.\n' "$x"; murloc_manual; return 0; }
        done
        dl=$(mktemp -d -p "$client/Data" .fta-murloc-download.XXXXXX) || { printf 'Cannot write to %s/Data.\n' "$client"; return 0; }
        printf 'Downloading...\n'
        if ! drive_fetch "$MURLOC_PATCH_ID" "$dl/patch-ZD.mpq"; then
            printf 'The download failed (no network, or Google Drive refused it).\n'; murloc_manual; rm -rf -- "$dl"; return 0
        fi
        src=$dl/patch-ZD.mpq
    fi
    got=$(hash_of "$src")
    if [[ $got != "$MURLOC_PATCH_SHA256" ]]; then
        printf 'The file is not the expected one (sha256 %s), so it was not installed.\n' "$got"
        murloc_manual; [[ -z $dl ]] || rm -rf -- "$dl"; return 0
    fi
    if [[ -f $dest ]]; then
        local bk=$client/FTA-Linux/murloc-backup/$(date +%Y%m%d-%H%M%S)
        mkdir -p -- "$bk" && cp -p -- "$dest" "$bk/patch-ZD.mpq" && printf 'Kept the previous patch-ZD.mpq in %s\n' "$bk"
    fi
    # copied beside the target and renamed into place, so a running game never sees a half-written archive
    if cp -f -- "$src" "$dest.new" && mv -f -- "$dest.new" "$dest"; then
        printf 'Installed Data/patch-ZD.mpq (Murloc race). Restart the game to see it in character creation.\n'
    else
        rm -f -- "$dest.new"; printf 'Could not write %s.\n' "$dest"
    fi
    [[ -z $dl ]] || rm -rf -- "$dl"
    return 0
}
if [[ $murloc_dl == no ]]; then
    mkdir -p -- "$client/FTA-Linux" && : >"$client/FTA-Linux/no-murloc" || true
    printf '\nMurloc race patch: not installed; later updates leave it out too (run with --murloc to change that).\n'
else
    murloc_patch || printf 'The Murloc patch could not be set up (the install itself is fine).\n'
fi

# ---- character texture size ----------------------------------------------------------------------
# The game builds every character skin in a working picture whose size is the console variable componentTextureLevel: the default 8 means
# 256x256, 9 means 512x512. Higher resolution character skins (the Murloc skins) only show their detail at 9. Once per client folder (a marker
# file FTA-Linux/texture-level-set records it), this adds SET componentTextureLevel "9" to WTF/Config.wtf, only if the file exists and does not
# mention the variable. Nothing already in the file is changed; later changes by the player are never overwritten. --no-texture-level skips it.
texture_level_apply() {
    local cfg=$client/WTF/Config.wtf marker=$client/FTA-Linux/texture-level-set eol=$'\n'
    [[ -f $marker ]] && return 0
    [[ -f $cfg ]] || { printf '\nCharacter textures: no WTF/Config.wtf yet (start the game once), so componentTextureLevel is left at the default. Type /console componentTextureLevel 9 in game for sharper character skins.\n'; return 0; }
    if grep -qi 'componentTextureLevel' "$cfg"; then
        printf '\nCharacter textures: componentTextureLevel is already set in Config.wtf; left alone.\n'
        : >"$marker"; return 0
    fi
    grep -q $'\r' "$cfg" && eol=$'\r\n'
    cp -p -- "$cfg" "$client/FTA-Linux/Config.wtf.before-texture-level" 2>/dev/null || true
    [[ -s $cfg && $(tail -c1 "$cfg" | od -An -c | tr -d ' ') != '\n' ]] && printf '%s' "$eol" >>"$cfg"
    printf 'SET componentTextureLevel "9"%s' "$eol" >>"$cfg" || { printf 'Could not write %s.\n' "$cfg"; return 0; }
    : >"$marker"
    printf '\nCharacter textures: set componentTextureLevel 9 in Config.wtf (512x512 character skins; the old file is kept as FTA-Linux/Config.wtf.before-texture-level). Undo with /console componentTextureLevel 8.\n'
}
if (( texture_level )); then
    texture_level_apply || printf 'The texture setting could not be applied (the install itself is fine).\n'
fi

# ---- system libraries ----------------------------------------------------------------------------
printf '\nChecking system libraries...\n'
if ! "$client/FTA-Linux/install-deps.sh"; then
    if (( update )); then
        printf 'Some system libraries are missing. Run FTA-Linux/install-deps.sh --install in the client folder to see and install them.\n'
    elif ask 'Install the missing system libraries now (asks for your password)?' Y; then
        "$client/FTA-Linux/install-deps.sh" --install --yes || fail 'the libraries could not be installed. Run FTA-Linux/install-deps.sh to see the exact command.'
    else
        printf 'Skipped. Run FTA-Linux/install-deps.sh --install in the client folder when you are ready.\n'
    fi
fi

# ---- menu shortcut -------------------------------------------------------------------------------
# With the launcher installed the shortcut opens it (Play is one click inside); "Play directly" starts the game
# without it. An update keeps an existing shortcut, switching it to the launcher, and never adds a new one.
apps=${XDG_DATA_HOME:-$HOME/.local/share}/applications
entry=$apps/fta-linux-client.desktop
if [[ -x $client/FTA-Linux/launcher.sh ]]; then start_script=$client/FTA-Linux/launcher.sh; else start_script=$client/FTA-Linux/play.sh; fi
write_shortcut() {
    mkdir -p -- "$apps"
    {
        printf '[Desktop Entry]\nType=Application\nName=Realmgate\n'
        printf 'Comment=Native Linux client for From The Ashes\nKeywords=From The Ashes;FTA;WoW;Linux;launcher;\n'
        printf 'Exec="%s"\nPath=%s\nTerminal=false\nCategories=Game;\n' "$start_script" "$client"
        if [[ -f $client/FTA-Linux/icon.png ]]; then
            printf 'Icon=%s/FTA-Linux/icon.png\nStartupWMClass=fta-linux-client\n' "$client"
        fi
        if [[ $start_script == */launcher.sh ]]; then
            printf 'Actions=Play;\n\n[Desktop Action Play]\nName=Play directly\nExec="%s/FTA-Linux/play.sh"\n' "$client"
        fi
    } >"$entry"
}
if (( update )); then
    if [[ -f $entry ]] && grep -qF "Exec=\"$client/FTA-Linux/" "$entry" 2>/dev/null; then
        write_shortcut
        printf 'Menu shortcut updated.\n'
    fi
elif (( shortcut )); then
    write_shortcut
    printf 'Added "Realmgate" to your applications menu.\n'
fi

printf '\nDone. Start it any time with:  %s\n' "$start_script"
if [[ $start_script == */launcher.sh ]]; then start_question='Open the launcher now?'; else start_question='Start the game now?'; fi
if ask "$start_question" N; then
    pause=0
    "$start_script" || true
fi
pause_and_exit 0
exit 0
__PAYLOAD__
