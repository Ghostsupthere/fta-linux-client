# data/

`patch-YF.mpq` (0.9 MB) is the model bounds fix. `tools/package.sh` puts it into the installer as `FTA-Data/patch-YF.mpq`; the installer copies it to `Data/patch-YF.mpq`
(`--no-modelfix` / `--modelfix`, and the Extras tab, switch it). It sorts after the base client's `Patch-YD.mpq`, so its files win.

It holds 17 world models (the Heartswood quest tree, a campfire, tents, fallen trees ...) copied from `Patch-YD.mpq` with only their animation bounds and radius rewritten.
Those models shipped with a corrupt radius, so the client culled them from low camera angles and could not pick them with the mouse. Rebuild it with
`tools/model-fix/build-patch-yf.py CLIENT_DATA_DIR data/patch-YF.mpq` (the script lists the models it repairs and touches nothing else).
