#!/usr/bin/env bash
# Look at and switch the optional pieces the installer adds to a client folder. The launcher's Extras tab uses this
# script, and so can you:
#
#   FTA-Linux/optional.sh list             one line per piece: id, state, title, description (tab separated)
#   FTA-Linux/optional.sh remove ID        take the piece out of the client; later updates leave it out
#   FTA-Linux/optional.sh restore ID       bring a piece back that was taken out here (offline)
#
# Pieces (ID): murloc, realmdesk, bodyshape, clickguard, modelfix, fixes, texture. Nothing is deleted. Whatever is taken out is moved to
# FTA-Linux/removed/<date-time>/ (or into $FTA_REMOVED_DIR when that is set, which uninstall.sh does), so it can be
# put back or deleted by hand. States: installed, removed (taken out here or opted out; the installer will not add it
# again by itself), absent (never added).
#
# Exit status of restore: 0 done, 3 nothing to restore here (run the installer again to add the piece).
set -euo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
client=$(cd -- "$here/.." && pwd -P)
state_dir=$client/FTA-Linux
cfg=$client/WTF/Config.wtf
tex_line='SET componentTextureLevel "9"'

fail() { printf 'optional: %s\n' "$*" >&2; exit 1; }
[[ -f $state_dir/optional.sh ]] || fail 'this script must live in the FTA-Linux folder of a client'

title() {
    case $1 in
        murloc)   printf 'Murloc race' ;;
        realmdesk) printf 'RealmDesk tickets' ;;
        bodyshape) printf 'Body Shape sliders' ;;
        clickguard) printf 'Bot Click Guard' ;;
        modelfix) printf 'Model bounds fix' ;;
        fixes)    printf 'Addon fixes' ;;
        texture)  printf 'Sharper character skins' ;;
        *) return 1 ;;
    esac
}
describe() {
    case $1 in
        murloc)   printf 'The Murloc playable race of the From The Ashes realm (Data/patch-ZD.mpq, 13 MB). Only useful on that realm.' ;;
        realmdesk) printf 'The in-game support ticket window (/desk). Does nothing unless the realm has its server side switched on.' ;;
        bodyshape) printf 'Character build sliders (/bs): height, build, face, and for female characters breast and butt. Your settings in WTF are kept.' ;;
        clickguard) printf 'NetherBot Click Guard (/nbcg): your NPCBots stop catching right-clicks meant for the world; hold a key to click one. Click-through needs this client build.' ;;
        modelfix) printf 'Corrects the animation bounds of 17 HD world models (Data/patch-YF.mpq, 0.9 MB) that the game could not click or kept hiding, such as the Heartswood quest tree. Changes nothing else.' ;;
        fixes)    printf 'Small fixes to a few addons that misbehave on Linux. Taking them out puts your original files back.' ;;
        texture)  printf 'Sets componentTextureLevel 9 in WTF/Config.wtf (512x512 character skins). Costs a little video memory.' ;;
        *) return 1 ;;
    esac
}

state() { # state ID: installed | removed | absent
    case $1 in
        murloc)
            [[ -f $client/Data/patch-ZD.mpq ]] && { printf installed; return; }
            [[ -f $state_dir/no-murloc ]] && { printf removed; return; }
            printf absent ;;
        realmdesk)
            [[ -f $client/Interface/AddOns/RealmDesk/RealmDesk.toc ]] && { printf installed; return; }
            [[ -f $state_dir/no-realmdesk ]] && { printf removed; return; }
            printf absent ;;
        bodyshape)
            [[ -f $client/Interface/AddOns/WowDecompBodyShape/WowDecompBodyShape.toc ]] && { printf installed; return; }
            [[ -f $state_dir/no-bodyshape ]] && { printf removed; return; }
            printf absent ;;
        clickguard)
            [[ -f $client/Interface/AddOns/NetherBotClickGuard/NetherBotClickGuard.toc ]] && { printf installed; return; }
            [[ -f $state_dir/no-clickguard ]] && { printf removed; return; }
            printf absent ;;
        modelfix)
            [[ -f $client/Data/patch-YF.mpq ]] && { printf installed; return; }
            [[ -f $state_dir/no-modelfix ]] && { printf removed; return; }
            printf absent ;;
        fixes)
            [[ -d $state_dir/addon-fix-backup ]] && { printf installed; return; }
            printf absent ;;
        texture)
            if [[ -f $cfg ]] && grep -qxF "$tex_line" <(tr -d '\r' <"$cfg"); then printf installed
            elif [[ -f $state_dir/texture-level-set ]]; then printf removed
            else printf absent; fi ;;
    esac
}

ids="murloc realmdesk bodyshape clickguard modelfix fixes texture"
valid() { case " $ids " in *" $1 "*) return 0 ;; *) return 1 ;; esac; }

game_running() { pgrep -x Wow64 >/dev/null 2>&1; }

removed_root() { printf '%s' "${FTA_REMOVED_DIR:-$state_dir/removed}"; }
new_removed_dir() { # a fresh folder for what is about to be moved out of the client
    local dir
    dir=$(removed_root)/$(date +%Y%m%d-%H%M%S)-$$
    mkdir -p -- "$dir"
    printf '%s' "$dir"
}
latest_removed_with() { # latest_removed_with NAME: the newest removed folder that holds NAME
    local d
    for d in $(find "$(removed_root)" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -r); do
        [[ -e $d/$1 ]] && { printf '%s' "$d"; return 0; }
    done
    return 1
}

cmd_list() {
    local id
    for id in $ids; do printf '%s\t%s\t%s\t%s\n' "$id" "$(state "$id")" "$(title "$id")" "$(describe "$id")"; done
}

cmd_remove() {
    local id=$1 dir
    valid "$id" || fail "unknown piece '$id' (one of: $ids)"
    game_running && fail 'the game is running; close it first'
    case $id in
        murloc)
            [[ -f $client/Data/patch-ZD.mpq ]] || printf 'The Murloc patch is not installed here.\n'
            if [[ -f $client/Data/patch-ZD.mpq ]]; then
                dir=$(new_removed_dir); mv -- "$client/Data/patch-ZD.mpq" "$dir/patch-ZD.mpq"
                printf 'Moved Data/patch-ZD.mpq to %s\n' "$dir"
            fi
            : >"$state_dir/no-murloc"
            printf 'Updates will leave the Murloc race out. Existing Murloc characters stay on the realm but cannot be shown without the patch.\n' ;;
        realmdesk)
            if [[ -d $client/Interface/AddOns/RealmDesk ]]; then
                dir=$(new_removed_dir); mv -- "$client/Interface/AddOns/RealmDesk" "$dir/RealmDesk"
                printf 'Moved Interface/AddOns/RealmDesk to %s (your RealmDeskConfig.lua and avatars are in it)\n' "$dir"
            else
                printf 'RealmDesk is not installed here.\n'
            fi
            : >"$state_dir/no-realmdesk"
            printf 'Updates will leave RealmDesk out.\n' ;;
        bodyshape)
            if [[ -d $client/Interface/AddOns/WowDecompBodyShape ]]; then
                dir=$(new_removed_dir); mv -- "$client/Interface/AddOns/WowDecompBodyShape" "$dir/WowDecompBodyShape"
                printf 'Moved Interface/AddOns/WowDecompBodyShape to %s (your slider settings stay in WTF)\n' "$dir"
            else
                printf 'Body Shape is not installed here.\n'
            fi
            : >"$state_dir/no-bodyshape"
            printf 'Updates will leave Body Shape out.\n' ;;
        clickguard)
            if [[ -d $client/Interface/AddOns/NetherBotClickGuard ]]; then
                dir=$(new_removed_dir); mv -- "$client/Interface/AddOns/NetherBotClickGuard" "$dir/NetherBotClickGuard"
                printf 'Moved Interface/AddOns/NetherBotClickGuard to %s (your settings stay in WTF)\n' "$dir"
            else
                printf 'NetherBot Click Guard is not installed here.\n'
            fi
            : >"$state_dir/no-clickguard"
            printf 'Updates will leave NetherBot Click Guard out.\n' ;;
        modelfix)
            if [[ -f $client/Data/patch-YF.mpq ]]; then
                dir=$(new_removed_dir); mv -- "$client/Data/patch-YF.mpq" "$dir/patch-YF.mpq"
                printf 'Moved Data/patch-YF.mpq to %s\n' "$dir"
            else
                printf 'The model bounds fix is not installed here.\n'
            fi
            : >"$state_dir/no-modelfix"
            printf 'Updates will leave it out. The HD tree, tents and the rest of the 17 models go back to their broken bounds (some cannot be clicked).\n' ;;
        fixes)
            [[ -d $state_dir/addon-fix-backup ]] || { printf 'No addon fixes were applied here.\n'; return 0; }
            dir=$(new_removed_dir)
            local b rel n=0
            local -A seen=()
            # Each fixed file was backed up the first time it changed, in a folder named by the date: walking the
            # folders oldest first, the first copy of a file seen is the original.
            while IFS= read -r -d '' b; do
                rel=${b#"$state_dir"/addon-fix-backup/}; rel=${rel#*/}
                [[ -n ${seen[$rel]:-} ]] && continue
                seen[$rel]=1
                case $rel in Interface/AddOns/*) ;; *) continue ;; esac
                [[ $rel == *..* ]] && continue
                if [[ -f $client/$rel ]]; then mkdir -p -- "$dir/fixed/$(dirname -- "$rel")" && cp -p -- "$client/$rel" "$dir/fixed/$rel"; fi
                mkdir -p -- "$client/$(dirname -- "$rel")" && cp -p -- "$b" "$client/$rel"
                n=$((n + 1))
            done < <(find "$state_dir/addon-fix-backup" -type f -print0 | sort -z)
            mv -- "$state_dir/addon-fix-backup" "$dir/addon-fix-backup"
            printf 'Put %d original addon file(s) back. The fixed copies are in %s/fixed. Updates will not apply the fixes again unless you run the installer by hand.\n' "$n" "$dir" ;;
        texture)
            if [[ -f $cfg ]] && grep -qxF "$tex_line" <(tr -d '\r' <"$cfg"); then
                dir=$(new_removed_dir); cp -p -- "$cfg" "$dir/Config.wtf"
                local tmp; tmp=$(mktemp "$cfg.XXXXXX")
                # drop only our line, keep the file's own line endings
                grep -vxF -e "$tex_line" -e "$tex_line"$'\r' "$cfg" >"$tmp" || true
                cat -- "$tmp" >"$cfg"; rm -f -- "$tmp"
                printf 'Removed the componentTextureLevel line from WTF/Config.wtf (copy of the old file: %s/Config.wtf).\n' "$dir"
            else
                printf 'The texture setting is not in WTF/Config.wtf; nothing to remove.\n'
            fi
            : >"$state_dir/texture-level-set"
            printf 'Updates will not add it again. In game, /console componentTextureLevel 8 sets the default size right away.\n' ;;
    esac
}

cmd_restore() {
    local id=$1 dir
    valid "$id" || fail "unknown piece '$id' (one of: $ids)"
    game_running && fail 'the game is running; close it first'
    case $id in
        murloc)
            [[ -f $client/Data/patch-ZD.mpq ]] && { printf 'The Murloc patch is already installed.\n'; return 0; }
            dir=$(latest_removed_with patch-ZD.mpq) || { printf 'No copy of the Murloc patch is kept here. Run FTA-Linux-Installer.run --murloc to download it.\n'; return 3; }
            mv -- "$dir/patch-ZD.mpq" "$client/Data/patch-ZD.mpq"; rm -f -- "$state_dir/no-murloc"
            printf 'Put Data/patch-ZD.mpq back.\n' ;;
        realmdesk)
            [[ -d $client/Interface/AddOns/RealmDesk ]] && { printf 'RealmDesk is already installed.\n'; return 0; }
            dir=$(latest_removed_with RealmDesk) || { printf 'No copy of RealmDesk is kept here. Run FTA-Linux-Installer.run --realmdesk to add it.\n'; return 3; }
            mv -- "$dir/RealmDesk" "$client/Interface/AddOns/RealmDesk"; rm -f -- "$state_dir/no-realmdesk"
            printf 'Put RealmDesk back.\n' ;;
        bodyshape)
            [[ -d $client/Interface/AddOns/WowDecompBodyShape ]] && { printf 'Body Shape is already installed.\n'; return 0; }
            dir=$(latest_removed_with WowDecompBodyShape) || { printf 'No copy of Body Shape is kept here. Run FTA-Linux-Installer.run --bodyshape to add it.\n'; return 3; }
            mv -- "$dir/WowDecompBodyShape" "$client/Interface/AddOns/WowDecompBodyShape"; rm -f -- "$state_dir/no-bodyshape"
            printf 'Put Body Shape back.\n' ;;
        clickguard)
            [[ -d $client/Interface/AddOns/NetherBotClickGuard ]] && { printf 'NetherBot Click Guard is already installed.\n'; return 0; }
            dir=$(latest_removed_with NetherBotClickGuard) || { printf 'No copy of NetherBot Click Guard is kept here. Run FTA-Linux-Installer.run --clickguard to add it.\n'; return 3; }
            mv -- "$dir/NetherBotClickGuard" "$client/Interface/AddOns/NetherBotClickGuard"; rm -f -- "$state_dir/no-clickguard"
            printf 'Put NetherBot Click Guard back.\n' ;;
        modelfix)
            [[ -f $client/Data/patch-YF.mpq ]] && { printf 'The model bounds fix is already installed.\n'; return 0; }
            dir=$(latest_removed_with patch-YF.mpq) || { printf 'No copy of the model bounds fix is kept here. Run FTA-Linux-Installer.run --modelfix to add it.\n'; return 3; }
            mv -- "$dir/patch-YF.mpq" "$client/Data/patch-YF.mpq"; rm -f -- "$state_dir/no-modelfix"
            printf 'Put Data/patch-YF.mpq back.\n' ;;
        fixes)
            printf 'Addon fixes come with the installer: run FTA-Linux-Installer.run and answer yes to review them.\n'; return 3 ;;
        texture)
            [[ -f $cfg ]] || { printf 'There is no WTF/Config.wtf yet (start the game once).\n'; return 3; }
            if grep -qi 'componentTextureLevel' "$cfg"; then printf 'componentTextureLevel is already set in Config.wtf; left alone.\n'; : >"$state_dir/texture-level-set"; return 0; fi
            local eol=$'\n'; grep -q $'\r' "$cfg" && eol=$'\r\n'
            [[ -s $cfg && $(tail -c1 "$cfg" | od -An -c | tr -d ' ') != '\n' ]] && printf '%s' "$eol" >>"$cfg"
            printf '%s%s' "$tex_line" "$eol" >>"$cfg"; : >"$state_dir/texture-level-set"
            printf 'Set componentTextureLevel 9 again.\n' ;;
    esac
}

case ${1:-} in
    list) cmd_list ;;
    remove) cmd_remove "${2:?usage: optional.sh remove ID}" ;;
    restore) cmd_restore "${2:?usage: optional.sh restore ID}" ;;
    ''|-h|--help) sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' ;;
    *) fail "unknown command '$1' (list, remove ID, restore ID)" ;;
esac
