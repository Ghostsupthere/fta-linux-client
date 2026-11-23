# data/

`patch-YF.mpq` (0.9 MB) is the model bounds fix. `tools/package.sh` puts it into the installer as `FTA-Data/patch-YF.mpq`; the installer copies it to `Data/patch-YF.mpq`
(`--no-modelfix` / `--modelfix`, and the Extras tab, switch it). It sorts after the base client's `Patch-YD.mpq`, so its files win.

It holds 17 world models (the Heartswood quest tree, a campfire, tents, fallen trees ...) copied from `Patch-YD.mpq` with only their animation bounds and radius rewritten.
Those models shipped with a corrupt radius, so the client culled them from low camera angles and could not pick them with the mouse. Rebuild it with
`tools/model-fix/build-patch-yf.py CLIENT_DATA_DIR data/patch-YF.mpq` (the script lists the models it repairs and touches nothing else).

## Flamestrike fix: `patch-Z9.mpq` + `patch-ZY.mpq` (always a pair)

Flamestrike ranks 2-9 played a male voice line and stood with their hands down: the spell visual kit pointed at a sound with the spell's own ID, and no channel kit was set.
`patch-Z9.mpq` (0.4 MB) carries `SpellVisual.dbc` and `SpellVisualKit.dbc` (same sizes as the From The Ashes tables in `Patch-I.mpq`); `patch-ZY.mpq` (4.6 MB) carries `Spell.dbc`
with spell 2120 repointed to visual 23372. Visual 10383 is shared by about 30 spells, so the spell is repointed, never the shared row.

**Maintenance rule:** a DBC inside a patch replaces the whole table, there is no row merge. `patch-ZY.mpq` hides every earlier `Spell.dbc`, so it **must be rebuilt on the newest
`Spell.dbc`** whenever the realm or the Murloc patch (`patch-ZD.mpq`) changes it. Today its table is byte-identical in size to the Murloc patch's (53,365,541 bytes), which is the
newest in the client. Before shipping a rebuilt pair, compare its row counts with the highest-priority existing copy. The installer takes both files together (`--no-flamestrike` /
`--flamestrike`, Extras tab "Flamestrike fix"); the names have to be `patch-??.mpq` to load.
