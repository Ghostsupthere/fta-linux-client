#!/usr/bin/env python3
"""Builds the launcher's pictures from the artwork (WIKI/backgroundnew-day.png and backgroundnew-night.png, copied to
assets/art-day.jpg and assets/art-night.jpg; both 3:2 and aligned with each other). For each of day and night:

  hero-<name>.jpg      the artwork itself, shown sharp in the Play page's framed scene
  bg-<name>.jpg        the window backdrop: the same picture, softly blurred and darkened, 2000 x 1280
  bg-<name>-blur.jpg   a heavier blur that the frosted panels of the Game, Settings and Diagnostics pages show

    python3 assets/make-background.py [--only day|night] [--backdrop-blur 9] [--panel-blur 30]

The window is 1000 x 640; the backdrop is made at twice that so it stays sharp on high-density screens and is drawn
"cover". The hero is drawn "cover" inside its frame; ui/main.slint maps effect positions (ember sources, portal glow)
onto the artwork using the Scene constants, which are in artwork pixels (1536 x 1024).
"""
import argparse
import pathlib
import numpy as np
from PIL import Image, ImageFilter

W, H = 2000, 1280
HERE = pathlib.Path(__file__).resolve().parent


def smooth(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)


def cover(img, w, h):
    s = max(w / img.width, h / img.height)
    img = img.resize((int(img.width * s + 0.5), int(img.height * s + 0.5)), Image.LANCZOS)
    l, t = (img.width - w) // 2, (img.height - h) // 2
    return img.crop((l, t, l + w, t + h))


def darken(img, level, vignette):
    a = np.asarray(img, np.float32)
    yy, xx = np.mgrid[0:a.shape[0], 0:a.shape[1]].astype(np.float32)
    r = np.hypot((xx / a.shape[1] - 0.5) * 1.6, (yy / a.shape[0] - 0.5) * 1.2)
    v = 1 - vignette * smooth((r - 0.25) / 0.7)
    return Image.fromarray(np.clip(a * level * v[..., None], 0, 255).astype(np.uint8))


def build(name, args):
    art = Image.open(HERE / f"art-{name}.jpg").convert("RGB")
    art.save(HERE / f"hero-{name}.jpg", quality=92)
    wide = cover(art, W, H)
    darken(wide.filter(ImageFilter.GaussianBlur(args.backdrop_blur)), 0.55, 0.55).save(HERE / f"bg-{name}.jpg", quality=88)
    darken(wide.filter(ImageFilter.GaussianBlur(args.panel_blur)), 0.7, 0.35).save(HERE / f"bg-{name}-blur.jpg", quality=85)
    print(f"wrote hero-{name}.jpg, bg-{name}.jpg, bg-{name}-blur.jpg from art-{name}.jpg")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", choices=["day", "night"], help="build just one of the two")
    ap.add_argument("--backdrop-blur", type=float, default=9)
    ap.add_argument("--panel-blur", type=float, default=30)
    args = ap.parse_args()
    for name in ([args.only] if args.only else ["day", "night"]):
        build(name, args)


main()
