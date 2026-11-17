#!/usr/bin/env bash
# Take the native Linux client out of a client folder again.
#
#   FTA-Linux/uninstall.sh [--yes] [--delete] [--dry-run] [--purge-settings]
#
# Removes Wow64, the FTA-Linux folder and the menu shortcut, and puts back what the installer changed:
#   - the Murloc patch (Data/patch-ZD.mpq), the RealmDesk and Body Shape addons, the login screens and the texture line in
#     WTF/Config.wtf are moved out of the client, and your original addon files are restored from the backups.
# Nothing is deleted outright: what was moved out waits in FTA-Linux-removed-<date-time>/ next to Wow.exe. Delete that
# folder when you are sure (--delete does it for you). Your WTF settings, saved variables, screenshots, the Windows
# client and the offline shader packs in Utils/WarcraftXL/visuals are not touched.
#   --yes             do not ask
#   --delete          delete the moved-out files at the end instead of keeping them
#   --dry-run         only say what would happen
#   --purge-settings  also delete the launcher's saved choices (~/.config/fta-linux-launcher)
set -euo pipefail

# This script lives in the folder it deletes: carry on from a copy.
if [[ -z ${FTA_UNINSTALL_STAGE:-} ]]; then
    here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
    copy=$(mktemp "${TMPDIR:-/tmp}/fta-uninstall.XXXXXX")
    cp -- "${BASH_SOURCE[0]}" "$copy"
    FTA_UNINSTALL_STAGE=1 FTA_UNINSTALL_HOME=$here exec bash "$copy" "$@"
fi
trap 'rm -f -- "${BASH_SOURCE[0]}"' EXIT

fail() { printf 'uninstall: %s\n' "$*" >&2; exit 1; }
yes=0 delete=0 dry=0 purge=0
for arg in "$@"; do
    case $arg in
        --yes|-y) yes=1 ;;
        --delete) delete=1 ;;
        --dry-run) dry=1 ;;
        --purge-settings) purge=1 ;;
        --help|-h) sed -n '2,15p' "$FTA_UNINSTALL_HOME/uninstall.sh" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) fail "unknown option $arg" ;;
    esac
done

fta=$FTA_UNINSTALL_HOME
client=$(cd -- "$fta/.." && pwd -P)
[[ $(basename -- "$fta") == FTA-Linux && -f $fta/play.sh && -f $client/Wow64 ]] || fail "$fta does not look like an installed FTA-Linux folder"
[[ -d $client/Data && -d $client/Interface ]] || fail "$client does not look like a client folder"
pgrep -x Wow64 >/dev/null 2>&1 && fail 'the game is running; close it first'

removed=$client/FTA-Linux-removed-$(date +%Y%m%d-%H%M%S)
shortcut=${XDG_DATA_HOME:-$HOME/.local/share}/applications/fta-linux-client.desktop
launcher_cfg=${XDG_CONFIG_HOME:-$HOME/.config}/fta-linux-launcher

pieces=()
while IFS=$'\t' read -r id state title _; do
    [[ $state == installed ]] && pieces+=("$id|$title")
done < <("$fta/optional.sh" list)
login_link=$client/Data/patch-ZZ.mpq
login_dir=$client/Data/LoginScreens

printf 'This will take the native Linux client out of:\n  %s\n\n' "$client"
printf 'Removed:\n  Wow64\n  FTA-Linux/ (the launcher, the bundled libraries, your launcher backups)\n'
[[ -f $shortcut ]] && printf '  the menu shortcut (%s)\n' "$shortcut"
(( purge )) && printf '  the launcher'"'"'s saved choices (%s)\n' "$launcher_cfg"
if ((${#pieces[@]})) || [[ -d $login_dir ]]; then
    printf '\nMoved to %s (nothing is deleted%s):\n' "$removed" "$( (( delete )) && printf ', except this folder at the end' )"
    for p in "${pieces[@]}"; do printf '  %s\n' "${p#*|}"; done
    [[ -d $login_dir ]] && printf '  the login screens (Data/LoginScreens)\n'
fi
printf '\nNot touched: WTF (your settings and saved variables), Screenshots, Logs, the Windows client, Utils/WarcraftXL/visuals.\n\n'
(( dry )) && { printf '(dry run: nothing changed)\n'; exit 0; }

if (( ! yes )); then
    [[ -t 0 ]] || fail 'not a terminal: run it in a terminal, or add --yes'
    read -r -p 'Go ahead? [y/N] ' answer
    [[ $answer == [yY]* ]] || { printf 'Nothing changed.\n'; exit 0; }
fi

export FTA_REMOVED_DIR=$removed
for p in "${pieces[@]}"; do
    id=${p%%|*}
    printf '\n== %s ==\n' "${p#*|}"
    "$fta/optional.sh" remove "$id" || printf 'Could not take that out; carrying on.\n'
done
if [[ -d $login_dir ]]; then
    # the chosen pack is a link made by the launcher: only our own link is removed
    if [[ -L $login_link ]]; then
        case $(readlink -- "$login_link") in */LoginScreens/*|LoginScreens/*) rm -f -- "$login_link" ;; esac
    fi
    mkdir -p -- "$removed" && mv -- "$login_dir" "$removed/LoginScreens"
    printf '\nMoved the login screens to %s/LoginScreens\n' "$removed"
fi

rm -f -- "$client/Wow64"
rm -rf -- "$client/FTA-Linux"
[[ -f $shortcut ]] && rm -f -- "$shortcut"
(( purge )) && [[ -d $launcher_cfg ]] && rm -rf -- "$launcher_cfg"
if (( delete )) && [[ -d $removed ]]; then rm -rf -- "$removed"; printf 'Deleted the moved-out files.\n'; fi

printf '\nDone. The native Linux client is gone from %s.\n' "$client"
[[ -d $removed ]] && printf 'What was moved out is in %s; delete that folder when you no longer need it.\n' "$removed"
exit 0
