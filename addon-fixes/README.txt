FTA addon fixes
===============

Fourteen small Lua fixes for addons in the From The Ashes base client (one file in AshenBotUpgrade, three in ElvUI,
eight in Questie-335, one in NPCBotPaperdoll, one in MobSpells), one change to the old ncBiggerMacros addon, and the new
macro window (three files that the base client does not have as separate files: item 16).
They are optional and apply to the Interface folder on any platform; the macro window's long macros need a Linux
client that has native macros (it falls back to the normal 255-character window on any other client).
Close the game before copying.

1. ElvUI 6.09, Modules/Chat/Chat.lua: achievement chat messages
   Symptom: "ElvUI\Modules\Chat\Chat.lua:1055: invalid option in `format'" after an achievement message.
   Cause: ElvUI runs format() on the server's broadcast text, and a literal % in that text makes format() raise.
   Fix: a FormatAchievementMessage helper tries format() under pcall and otherwise puts the player link in
   place of the first %s. It covers both ACHIEVEMENT and GUILD_ACHIEVEMENT messages.
   Status: loads under Lua 5.1; the failing achievement was never captured, so it has not been re-triggered.

2. ElvUI 6.09, Media/SharedMedia.lua: quest-start icon in the bags
   Symptom: items that start a quest (for example a Westfall Deed) show an opaque white box over their icon.
   Cause: ElvUI's BagQuestIcon points at a file the client folder only has as an opaque .blp with no alpha, so the
   "!" is drawn on a solid block.
   Fix: BagQuestIcon uses Blizzard's own Interface\ContainerFrame\UI-Icon-QuestBang (gold border and "!", with alpha).
   Status: confirmed in play.

3. ElvUI 6.09, Modules/Skins/Blizzard/Mail.lua: attachments without an icon
   Symptom: a Lua error while styling the mail window when an attachment's icon file cannot be loaded.
   Cause: the client drops the normal texture of such a button, and the skin restyled it unconditionally.
   Fix: the restyling is skipped when there is no icon.
   Status: loads under Lua 5.1; the error was not re-triggered.

4. Questie-335, ServerBridge/Bridge.lua: exploration quests, catalog loading, map area
   a) Symptom: "Questie: [ERROR] Missing objective data for quest 62 - Scout through the Fargodeep Mine",
      repeated. Affects quests whose objective is to explore an area (quest 62; 76 is the same kind).
      Cause: with the server bridge connected, server records replace Questie's quest data, and the bridge
      did not keep the exploration trigger ("triggerEnd"). The server-side code that builds the records was not
      available, so this is inferred from the client side.
      Fix: Store() copies the bundled triggerEnd into a server quest record that has none, for quests in the
      bundled database.
      Status: the errors stopped for quest 62 in play.
   b) Symptom: map icons disappearing while the world map sits open.
      Cause: a restarted or overlapping catalog fetch for the same area emptied the live catalog table before the
      new fetch finished.
      Fix: each fetch fills a separate staging table and replaces the live catalog only when it is complete.
   c) Symptom: the viewed map area flickered between values with the map untouched.
      Cause: ViewedArea() called QuestieCompat.GetCurrentMapUiMapId, which does not exist, and then scanned
      UiMapData with pairs(), whose order is not stable.
      Fix: it uses QuestieCompat.GetCurrentUiMapID().
   Status: items b and c confirmed in play together with fixes 5 to 7.

5. Questie-335, Compat/HBD.lua: stale icons when changing the map zone
   Symptom: after choosing another zone in the world map's dropdown, the previous zone's icons stayed on the map.
   Cause: UpdateWorldMap() skipped redrawing when the resolved uiMapID looked unchanged.
   Fix: it also compares the dropdown's displayed zone name and redraws when that changed.
   Status: confirmed in play.

6. Questie-335, Compat/Compat.lua: map icons vanishing (root cause)
   Symptom: icons showed correctly and then disappeared after a few seconds.
   Cause: RegisterServerMap() overwrote mapIdToUiMapId[map.id] on every server sync. Several server areas share
   one native zone (for example Westfall), so the last one registered replaced Questie's own correct mapping.
   Fix: the entry is only set when none exists yet.
   Status: confirmed in play.

7. Questie-335, Database/Zones/zoneDB.lua: zone lookup
   Symptom: the same input could resolve to different areas between calls.
   Cause: GetAreaIdByUiMapId() scanned its table with pairs() and returned the first match.
   Fix: it indexes the table directly.
   Status: confirmed in play together with fixes 4 to 6.

8. AshenBotUpgrade 1.3.4, AshenBotUpgrade.lua: tooltip feedback and caster wand upgrades
   Symptom: hovering an item sometimes showed nothing, with no way to tell "no upgrade" from "not checked", and a
   clearly better wand was never reported as an upgrade for a caster bot.
   Cause: "No bot upgrade found" was hidden by DB.showEmpty, which defaulted to off. Separately, a weapon's damage
   counted as a stat in HasRoleScorableStats, so every weapon, even a plain wand, was kept out of the item-level
   fallback; and no caster profile weighted ranged weapon damage, so wand damage never changed the score.
   Fix: showEmpty defaults to on (saves that already stored "off" are switched on once, and /abu showempty still
   toggles it); the empty line is only shown for gear, not potions or reagents. Weapon damage no longer counts as a
   stat for the fallback, and the 12 wand-capable caster and healer profiles (Mage, Priest, Warlock) get a small
   ranged weapon damage weight.
   Note: the fallback now applies to every statless weapon. A statless melee weapon with a higher item level but lower
   damage can be shown as an upgrade; weapons with real stats are scored as before.
   Status: tooltip lines confirmed in play; a wand upgrade was not re-tested after the change.

9. Murloc race (custom playable race 31): Questie-335 Compat/Compat.lua and Modules/QuestiePlayer.lua, NPCBotPaperdoll.lua
   Symptom: "Questie: [ERROR] Unsupported player race: Murloc" for a Murloc character; the bot paperdoll has no name for the race.
   Cause: the race is not in the addons' race tables, and Questie's quest data only carries race bits for the original races.
   Fix: Compat.lua gets Murloc = 31; QuestiePlayer.lua matches race-restricted quests as a High Elf (the server mirrors the High
   Elf's quest permissions onto the Murloc); the paperdoll's race-name table gets [31] = "Murloc". Harmless without a Murloc.
   Status: the Questie error is gone in play; the paperdoll name was not looked at in play.

10. Questie-335, Modules/Quest/QuestEventHandler.lua: error after accepting and quickly removing a quest
   Symptom: "QuestEventHandler.lua:285: attempt to index field '?' (a nil value)" (seen with quest 1447).
   Cause: accepting a quest queues its accept handling (combat queue, or a retry while objectives are not cached yet).
   If the quest is abandoned, turned in or auto-completed before that runs, its entry in Questie's quest-log table is
   already gone and the handler indexed nil. MarkQuestAsAbandoned had the same unguarded access.
   Fix: HandleQuestAccepted returns early and MarkQuestAsAbandoned does nothing when the quest is no longer tracked.
   Status: found from an in-game error report; the fix has not been re-tested in play.

11. MobSpells, core.lua: the ability list only shows while Alt is held
   Symptom: every mob tooltip carried a long list of the mob's abilities.
   Fix: the list (and its descriptions) is only added while Alt is held; without Alt the tooltip says "Hold Alt to
   show abilities". Pressing or releasing Alt rebuilds the open unit tooltip. The addon's options are unchanged.
   Status: the logic was tested outside the game with stand-in functions; not yet looked at in play.

12. Questie-335, Modules/Libs/ThreadLib.lua: available-quest icons gone until a UI reload
   Symptom: the "!" icons for available quests disappear (seen after opening the big map) and stay gone until /reload.
   Cause: AvailableQuests runs its icon calculation in a ThreadLib thread and keeps a "thread running" flag that only the
   thread's completion callback clears. When the thread raised an error, ThreadLib cancelled its timer without calling
   the callback, so the flag stayed set: every later refresh (including the server bridge's reset) queued behind the dead
   thread and nothing was drawn again. The error itself was not captured (the player had Lua errors hidden), so this
   fixes the lock-up, not whatever raises the error; if it recurs the error now shows up in chat as a Questie [ERROR].
   Fix: ThreadLib also calls the completion callback (under pcall) when the thread fails.
   Status: found by reading the code; not reproduced and not yet tested in play.

13. NPCBotPaperdoll, NPCBotPaperdoll.lua: bot equipment refreshes by itself when a bot joins the party
   Symptom: after summoning or inviting bots (or after alt bots rejoin at login), bag tooltips showed no bot upgrade line
   until the Bot Roster was opened and Refresh pressed for each bot.
   Cause: the paperdoll only requested equipment on a manual Refresh (and once at login, before the bots have joined).
   Fix: when the group gains a member the addon waits 3 seconds for the group to settle, then runs the same full
   sync as the roster's sync-all button (at most one automatic sync per 15 seconds, postponed while in combat).
   Status: the timing logic was tested outside the game with stand-in functions; not yet seen in play.

14. Questie-335, Compat/Compat.lua (second change in the same file): world map icons stay gone after an internal error
   Symptom: after opening the big map the quest icons never came back on it until /reload.
   Cause: while the world map is open, Questie reads the player position through an "internal map read" that raises a
   depth counter and lowers it afterwards. If anything in between raised an error the counter stayed up for the rest of
   the session, and the world map update (HBD) returns early whenever the counter is up, so it was never redrawn.
   Fix: the read runs under pcall and the counter is always lowered.
   Status: found by reading the code; not reproduced and not yet tested in play.

15. Questie-335, Database/Zones/zoneTables.lua: quest objectives on a whole-continent map raised an error
   Symptom: "No UiMapID or fitting parentAreaId for areaId : 10074 - Matrix Punchograph 3005-A" (quest Data Rescue), logged
   as "[ERROR] [QuestieQuest]: [SpecialObjectives] There was an error populating objectives", and that objective had no
   map icon. Player ticket #8.
   Cause: thirteen of Questie's data corrections place an object or NPC on a fake zone id that stands for a continent
   (10073 Kalimdor, 10074 Eastern Kingdoms, 10089 Azeroth), and the zone table had no map for those ids.
   Fix: the three ids are mapped to the continent maps the compat layer already has (1414, 1415, 947).
   Status: found by reading the code; the files load under Lua 5.1; not yet tested in play (check the Data Rescue objective).

16. Macro window (new files) and ncBiggerMacros: long macros
   Needs a client with native macros (the Linux client from the release that carries this bundle).
   What it adds: Interface/AddOns/Blizzard_MacroUI/Blizzard_MacroUI.lua, Blizzard_MacroUI.xml and FTA_MacroEditor.lua.
   They replace the macro window with one that keeps macros up to 2048 characters (the normal limit is 255), shows a
   character counter, has a Save button, and an Expand button that opens a larger editor with Undo/Redo, a syntax
   preview, Check (validates the macro), Complete, Duplicate, and Find/Replace. The files come from the game's own
   archives plus our changes, so the installer ADDS them (the client has no such loose files) and leaves a different
   file that is already there alone. On a client without native macros the window works like the normal one.
   ncBiggerMacros.lua (a change to an existing file): that addon replaced the macro window's functions with ones made
   for the 255-character window; with the new window they made clicking a macro do nothing. It now steps aside when the
   client has native macros, and moves anything it stored earlier (a macro whose text is only "/click ExtraMacrosN") back
   into the client's own macro storage.
   Where macros are kept: the Linux client saves them in WTF\Account\<account>\<realm>\macros-cache.fta (account
   macros) and in the character's folder. Your existing macros are imported from macros-cache.txt the first time;
   that file is left alone. Macros are not uploaded to the realm any more (they are not synchronised between PCs).
   Status: tested in play on one account (import, select, edit, long macros, Duplicate, Find/Replace, saving).

Credit: the Questie map-icon fixes (items 4b and 4c, 5, 6 and 7) are by Grandpa_Boris.

Install
-------
Option 0, with the installer: it carries these fixes. It lists the files it will change, asks, backs the originals
up under FTA-Linux/addon-fix-backup/ and applies the fixes: a file is replaced only if it is the version these
fixes were made for, otherwise the patch is applied if it fits (needs the "patch" tool), otherwise the file is left
alone. Use --no-fixes to skip it, or --fixes PATH to point at a fixes folder.

Option A, replace the files by hand (items 1-15 and ncBiggerMacros; item 16's three new files are simply copied in). These are complete copies of the files as shipped in the base client, so
only use this if your ElvUI is 6.09 and your Questie-335 and AshenBotUpgrade match the base client. Copy the Interface folder from
this archive over the Interface folder of your client folder (the one that holds Wow64 and Data).

Option B, apply the patches by hand. Works with other versions of those files if the surrounding lines match. From your
client folder:

    for p in FTA-Addon-Fixes/patches/*.patch; do patch -p1 --dry-run < "$p"; done      # check first
    for p in FTA-Addon-Fixes/patches/*.patch; do patch -p1 < "$p"; done

Then /reload in game, or restart the client.

Undo: patch -R -p1 with the same patch files (after Option B), or restore your backup (the installer keeps
one in FTA-Linux/addon-fix-backup/). The files item 16 added are listed in FTA-Linux/addon-fix-new-files.txt; delete
them (or use the launcher's Extras tab, "Addon fixes", which takes them out and puts the originals back).

ElvUI and Questie keep their own licenses. These changes are offered under the same terms as the files they
modify. No game data is included.
