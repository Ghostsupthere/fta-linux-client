# Login-screen packs

`make-login-pack.py` builds the packs the launcher's **Login** tab chooses between: one small MPQ per screen
(`login-<name>.mpq`) holding the scene's model, textures and music plus rewritten copies of the client's login Lua
(`lgzg.lua`, `AccountLogin.lua`, `XlIlHI.lua`) and the themed interface in `theme-ui.lua` (side columns that fill
wide windows, the sign-in panel, character-select buttons, fonts). Installed as `Data/patch-ZZ.mpq` a pack overrides
the client's own login and character-select screens; removing the file brings the default back.

```
python3 -I make-login-pack.py --source patch-c.mpq --glue DIR --out OUT \
    [--shots clean-shots] [--logos DIR_WITH_logo-COLOUR.png] [--fonts DIR] NAME...
```

- `--source`: a `patch-c.mpq` that holds the login scenes (you need your own copy; none is included here).
- `--glue`: the client's `Interface/GlueXML` files `lgzg.lua`, `AccountLogin.lua`, `XlIlHI.lua`, extracted from
  `Patch-I.mpq`.
- `--shots`: `clean-<name>.png` 16:9 pictures of each scene without any interface. They give the theme colours and
  the blurred side columns. Without them the pack still works, with neutral colours and plain side columns.
- `--logos`: `logo-<colour>.png` files; `LOGO_FOR` at the top of the script says which colour goes with which screen.
- `--fonts`: defaults to the launcher's Cinzel fonts (`launcher/assets/fonts`, SIL Open Font License; the licence
  texts go into each pack).
- Names: northrend, cataclysm, pandaria, warlords, legion, legion-alt, battleforazeroth, shadowlands, dragonflight,
  warwithin.

Needs Python 3 with Pillow and numpy, ImageMagick (`convert`, to turn `.tga` model textures into `.blp`) and
StormLib (`libstorm.so`; set `STORMLIB` to its path if it is not on the loader path).

Test-only options (`--debug-shot N`, `--charsel`, `--fake`, `--hideall`, `--demo`, `--dump`, `--music`, `--clean`)
make the pack take an engine screenshot after N seconds, optionally on the character-select screen with pretend
characters; never ship a pack built with them.

Notes for anyone changing `theme-ui.lua`: the stock widgets keep the logic (they are made invisible and the themed
buttons call `:Click()` on them; the rotate arrows keep the stock buttons because turning needs the held-down
state); an edit box's text colour is set on its unnamed FontString region; frames outside `GlueParent` (a fixed 16:9
area, centred) do draw, which is how the side columns work; the stock gold row highlight is the button's own
highlight texture.
