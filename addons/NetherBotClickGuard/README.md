# NetherBotClickGuard

Makes your NPCBots (and alt bots) click-through: a right-click on one does nothing to it, so you can move the camera, or click the ground or another NPC behind it, without targeting it or opening its gossip, quest, merchant or unit menu. Hold the **modifier key** while you click when you do want to interact with a bot; it then works exactly as normal, including the gossip menu.

## Click-through needs the From The Ashes client

The click itself is decided inside the game engine, before any addon runs. The From The Ashes Linux client has a function for this (`WXL_SetClickThrough`); with it the bot is skipped by the click hit test and the click really goes through it. On a client without that function (stock or Windows) the addon falls back to the older behaviour: the click still lands on the bot, and the addon immediately closes whatever opened (target, gossip, quest, merchant, unit menu). `/nbcg status` says which one you have.

Left-click is never touched (unless you switch the guarded button). Real players are left alone unless you mark them with `/nbcg mark` or `/nbcg scan`.

## The modifier key

The modifier is the key you hold together with the mouse click to interact with a bot normally.

- Default: **SHIFT**. Right-click a bot: it is ignored. **SHIFT + right-click** a bot: its normal menu or window opens. The key must be held when you press the mouse button.
- Choices: `ALT`, `SHIFT` or `CTRL`.
- Change it in game with `/nbcg modifier <alt|shift|ctrl>`. It is saved and kept between sessions.

## Commands

Type `/nbcg` (or `/nbcg help`) to list them in the game.

| Command | What it does |
|---|---|
| `/nbcg modifier <alt\|shift\|ctrl>` | Set the key that lets you click a bot normally. Default `shift`. |
| `/nbcg button <left\|right>` | Set which mouse button is guarded. Default `right`. The other button is left alone. |
| `/nbcg clickthrough <on\|off>` | `on` (default): the engine ignores bots on world clicks (FTA client only). `off`: use the older close-the-menu fallback. |
| `/nbcg tooltip <on\|off>` | `off` (default) hides the unit tooltip on bots unless the modifier is held. `on` always shows it. |
| `/nbcg party <on\|off>` | `on` (default): anyone in your party or raid who is a creature (NPCBots, alt bots) counts as a bot. `off`: only marked bots and NPCBotPaperdoll's roster count. |
| `/nbcg mark` | Hover a unit first. Always treat it as a bot. |
| `/nbcg unmark` | Hover a unit first. Remove it from the bot list. |
| `/nbcg scan` | Mark everyone currently in your party and raid as a bot. Not needed with `party on`; it also marks real players, so use `/nbcg unmark` on them if that happens. |
| `/nbcg ignore` | Hover a unit first. Never treat it as a bot, even if it would otherwise count. |
| `/nbcg unignore` | Hover a unit first. Remove it from the ignore list. |
| `/nbcg status` | Show the current settings. |
| `/nbcg debug` | Hover a unit first. Shows whether it counts as a bot, why, and its GUID. Also lists what NPCBotPaperdoll knows about. |
| `/nbcg trace <on\|off>` | Print what the guard decides on each click, to troubleshoot. Turn it off again afterwards. |

## How a unit is recognised as a bot

Checked in this order:

1. `/nbcg ignore` list: never a bot.
2. `/nbcg mark` / `/nbcg scan` list: a bot.
3. Party or raid member whose GUID is a creature (starts with `0xF`), when `party` is on. Real players have a player GUID (starts with `0x0`) and are skipped.
4. A bot in NPCBotPaperdoll's roster.

## Changing settings in the saved-variables file

Everything the commands change is kept in `NetherBotClickGuardDB` in:

`WTF/Account/<ACCOUNT>/SavedVariables/NetherBotClickGuard.lua`

You can edit it by hand, but only while the game is closed, because the game rewrites it when you log out.

| Key | Values | Default |
|---|---|---|
| `modifier` | `"ALT"`, `"SHIFT"`, `"CTRL"` (any capitalisation is accepted) | `"SHIFT"` |
| `button` | `"RightButton"` or `"LeftButton"` | `"RightButton"` |
| `clickThrough` | `true` or `false` | `true` |
| `hideTooltip` | `true` or `false` | `true` |
| `partyIsBot` | `true` or `false` | `true` |
| `trace` | `true` or `false` | `false` |
| `whitelist` / `blacklist` | lists of names, `["Name"] = true` | empty |

A value that is not valid is replaced by the default when the addon loads.

## Troubleshooting

1. Hover the bot and run `/nbcg debug`. It should say `isBot=true` and give a reason. A bot's GUID should start with `0xF`.
2. Run `/nbcg trace on`, right-click the bot, and read the grey `NBCG trace` lines.
3. If the bot is not recognised, run `/nbcg mark` on it as a workaround.

## Install

For real click-through you also need an FTA client build that contains `WXL_SetClickThrough`; the addon works without it, with the fallback described above.

Copy the `NetherBotClickGuard` folder into `Interface/AddOns` (replacing any older copy), then run `/reload` or restart the game. Saved settings are kept.

Tested on the Linux native client. It uses only standard 3.3.5 interface functions, so it should work on the Windows client too, but that has not been tested.

## Authors

Grandpa_Boris, Ghostsupthere and Claude.
