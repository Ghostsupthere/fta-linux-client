#!/usr/bin/env python3
"""Builds the small white-on-transparent glyphs in assets/icons/ (the launcher tints them with `colorize`):
play, game, settings, diagnostics, check, refresh, maximize, restore, external.

    python3 assets/make-icons.py

Each is drawn at 8x and scaled down to 64 x 64 so the edges are smooth.
"""
import math
import pathlib
from PIL import Image, ImageDraw

HERE = pathlib.Path(__file__).resolve().parent / "icons"
S, K = 64, 8  # final size, supersampling
W = S * K
WHITE = (255, 255, 255, 255)
CLEAR = (255, 255, 255, 0)


def canvas():
    img = Image.new("RGBA", (W, W), CLEAR)
    return img, ImageDraw.Draw(img)


def p(v):
    return v * K


def save(img, name):
    HERE.mkdir(exist_ok=True)
    img.resize((S, S), Image.LANCZOS).save(HERE / f"{name}.png")


def play():
    img, d = canvas()
    d.polygon([(p(18), p(9)), (p(18), p(55)), (p(56), p(32))], fill=WHITE)
    # soften the corners
    save(img, "play")


def game():
    img, d = canvas()
    # controller body: a wide rounded shape with two grips
    d.rounded_rectangle((p(5), p(17), p(59), p(43)), radius=p(13), fill=WHITE)
    d.ellipse((p(4), p(26), p(24), p(58)), fill=WHITE)
    d.ellipse((p(40), p(26), p(60), p(58)), fill=WHITE)
    # cut-outs: d-pad on the left, two buttons on the right
    cut = (0, 0, 0, 0)
    d.rectangle((p(15), p(27), p(25), p(31)), fill=cut)
    d.rectangle((p(18), p(24), p(22), p(34)), fill=cut)
    d.ellipse((p(40), p(24), p(46), p(30)), fill=cut)
    d.ellipse((p(47), p(29), p(53), p(35)), fill=cut)
    save(img, "game")


def settings():
    img, d = canvas()
    cx = cy = 32
    teeth = 8
    pts = []
    r_out, r_in = 28, 21
    for i in range(teeth * 4):
        a = 2 * math.pi * i / (teeth * 4) - math.pi / (teeth * 4) * 1.0
        phase = i % 4
        r = r_out if phase in (1, 2) else r_in
        pts.append((p(cx + r * math.cos(a)), p(cy + r * math.sin(a))))
    d.polygon(pts, fill=WHITE)
    d.ellipse((p(cx - 16), p(cy - 16), p(cx + 16), p(cy + 16)), fill=WHITE)
    d.ellipse((p(cx - 8), p(cy - 8), p(cx + 8), p(cy + 8)), fill=(0, 0, 0, 0))
    save(img, "settings")


def diagnostics():
    img, d = canvas()
    pts = [(4, 34), (16, 34), (23, 14), (33, 52), (41, 24), (46, 34), (60, 34)]
    d.line([(p(x), p(y)) for x, y in pts], fill=WHITE, width=p(5), joint="curve")
    for x, y in (pts[0], pts[-1]):
        d.ellipse((p(x - 2.5), p(y - 2.5), p(x + 2.5), p(y + 2.5)), fill=WHITE)
    save(img, "diagnostics")


def check():
    img, d = canvas()
    d.line([(p(14), p(34)), (p(27), p(47)), (p(51), p(18))], fill=WHITE, width=p(8), joint="curve")
    for x, y in ((14, 34), (51, 18)):
        d.ellipse((p(x - 4), p(y - 4), p(x + 4), p(y + 4)), fill=WHITE)
    save(img, "check")


def refresh():
    img, d = canvas()
    d.arc((p(10), p(10), p(54), p(54)), start=-60, end=225, fill=WHITE, width=p(7))
    # arrow head at the end of the arc (angle 225 deg, clockwise in screen space)
    a = math.radians(225)
    ex, ey = 32 + 22 * math.cos(a), 32 + 22 * math.sin(a)
    d.polygon([(p(ex - 11), p(ey - 3)), (p(ex + 6), p(ey - 12)), (p(ex + 5), p(ey + 8))], fill=WHITE)
    save(img, "refresh")


def maximize():
    img, d = canvas()
    d.rounded_rectangle((p(10), p(10), p(54), p(54)), radius=p(5), outline=WHITE, width=p(6))
    save(img, "maximize")


def restore():
    img, d = canvas()
    d.rounded_rectangle((p(8), p(20), p(44), p(56)), radius=p(5), outline=WHITE, width=p(6))
    d.line([(p(20), p(14)), (p(56), p(14)), (p(56), p(46))], fill=WHITE, width=p(6), joint="curve")
    save(img, "restore")


def external():
    img, d = canvas()
    d.line([(p(24), p(40)), (p(52), p(12))], fill=WHITE, width=p(7))
    d.polygon([(p(30), p(10)), (p(54), p(10)), (p(54), p(34))], fill=WHITE)
    d.line([(p(44), p(38)), (p(44), p(54)), (p(10), p(54)), (p(10), p(20)), (p(26), p(20))], fill=WHITE, width=p(6), joint="curve")
    save(img, "external")


for fn in (play, game, settings, diagnostics, check, refresh, maximize, restore, external):
    fn()
print("wrote", ", ".join(sorted(f.name for f in HERE.glob("*.png"))))
