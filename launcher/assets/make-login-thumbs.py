#!/usr/bin/env python3
"""Thumbnails for the Login tab: assets/login/<id>.jpg, 320 x 180, from full-size screenshots of each login screen.

    python3 assets/make-login-thumbs.py SHOTS_DIR      (SHOTS_DIR holds <id>.png, 16:9)

The ids are the ones in src/loginscreen.rs. The default screen has no picture (the tab draws it).
"""
import pathlib, sys
from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
IDS = ["northrend", "cataclysm", "pandaria", "warlords", "legion", "legion-alt", "battleforazeroth", "shadowlands", "dragonflight", "warwithin"]
src = pathlib.Path(sys.argv[1])
out = HERE / "login"
out.mkdir(exist_ok=True)
for i in IDS:
    Image.open(src / f"{i}.png").convert("RGB").resize((320, 180), Image.LANCZOS).save(out / f"{i}.jpg", quality=84)
print("wrote", ", ".join(sorted(p.name for p in out.glob("*.jpg"))))
