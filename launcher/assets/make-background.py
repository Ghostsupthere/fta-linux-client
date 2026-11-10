#!/usr/bin/env python3
"""Builds the launcher backgrounds from the artwork: for each of day and night, assets/bg-<name>.jpg and its
blurred twin assets/bg-<name>-blur.jpg (the frosted panel shows the blurred one). The art sits at the top right
and fades into a dark base on its left and bottom edges.

    python3 assets/make-background.py [--only day|night] [--width 0.82] [--height 0.82] [--fade-left 0.65]
                                      [--fade-bottom 0.80] [--anchor-x 1.0] [--blur 30]

The art is assets/art-day.jpg and assets/art-night.jpg (any size or aspect; scaled to cover its area). The window
is 1000 x 640; the pictures are made at twice that so they stay sharp on high-density screens.
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


# Dark bases the art fades into: (top, bottom, glow added at the top left where the logo sits).
BASES = {
    "day": ((44, 22, 64), (20, 12, 38), (70, 38, 78)),
    "night": ((14, 20, 62), (8, 10, 30), (24, 40, 110)),
}


def base(name):
    """Deep dusk purple (day) or midnight indigo (night), lighter at the top left, darker toward the bottom."""
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)
    t = yy / H
    top, bottom, glow_c = (np.array(c, np.float32) for c in BASES[name])
    img = top + (bottom - top) * smooth(t * 1.1)[..., None]
    glow = np.exp(-(np.hypot((xx - W * 0.18) / W, (yy - H * 0.20) / H * 1.3) / 0.35) ** 2)[..., None]
    return img + glow * glow_c * 0.6


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", choices=["day", "night"], help="build just one of the two")
    ap.add_argument("--width", type=float, default=0.82, help="art width as a share of the window width")
    ap.add_argument("--height", type=float, default=0.82, help="art height as a share of the window height")
    ap.add_argument("--fade-left", type=float, default=0.65, help="share of the art's width that fades in from the left")
    ap.add_argument("--fade-bottom", type=float, default=0.80, help="share of the art's height that fades out at the bottom")
    ap.add_argument("--anchor-x", type=float, default=1.0, help="which part of the art stays when it is cropped sideways: 0 left edge, 1 right edge")
    ap.add_argument("--blur", type=float, default=30, help="blur radius of the frosted version")
    args = ap.parse_args()

    for name in ([args.only] if args.only else ["day", "night"]):
        build(name, args)


def build(name, args):
    art_path = HERE / f"art-{name}.jpg"
    aw, ah = int(W * args.width), int(H * args.height)
    art = Image.open(art_path).convert("RGB")
    scale = max(aw / art.width, ah / art.height)  # cover
    art = art.resize((int(art.width * scale + 0.5), int(art.height * scale + 0.5)), Image.LANCZOS)
    left, top = int((art.width - aw) * args.anchor_x), 0
    art = np.asarray(art.crop((left, top, left + aw, top + ah)), np.float32)

    xs = np.linspace(0, 1, aw)[None, :]
    ys = np.linspace(0, 1, ah)[:, None]
    alpha = smooth(xs / args.fade_left) * smooth((1 - ys) / args.fade_bottom)
    # keep a hint of the very top edge so the art starts at the window edge
    canvas = base(name)
    x0 = W - aw
    region = canvas[0:ah, x0:W]
    canvas[0:ah, x0:W] = region * (1 - alpha[..., None]) + art * alpha[..., None]

    img = Image.fromarray(np.clip(canvas, 0, 255).astype(np.uint8))
    img.save(HERE / f"bg-{name}.jpg", quality=90)
    img.filter(ImageFilter.GaussianBlur(args.blur)).save(HERE / f"bg-{name}-blur.jpg", quality=88)
    print(f"wrote bg-{name}.jpg and bg-{name}-blur.jpg ({W}x{H}) from {art_path.name}")


main()
