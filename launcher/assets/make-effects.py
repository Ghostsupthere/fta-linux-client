#!/usr/bin/env python3
"""Builds the two small effect images the launcher draws on top of the artwork:

  swirl.png     a green portal vortex (transparent outside a circle); the Play button turns it slowly
  icon-dim.png  the app icon in grey, shown while no client folder is found
  sun.png, moon.png  the two icons on the day / night switch in the header

    python3 assets/make-effects.py
"""
import pathlib
import math
import numpy as np
from PIL import Image, ImageDraw, ImageEnhance

HERE = pathlib.Path(__file__).resolve().parent


def smooth(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)


def swirl(n=512):
    y, x = np.mgrid[0:n, 0:n].astype(np.float32)
    x = (x - n / 2 + 0.5) / (n / 2)
    y = (y - n / 2 + 0.5) / (n / 2)
    r = np.hypot(x, y)
    th = np.arctan2(y, x)
    # three arms of a logarithmic spiral, warped so it looks like churning energy rather than a pinwheel
    phase = 3 * th + 6.0 * np.log(r + 0.16) + 0.9 * np.sin(5 * th + 7 * r) + 0.5 * np.sin(11 * th - 13 * r)
    v = 0.5 + 0.5 * np.cos(phase)
    v = v ** 1.6
    glow = np.exp(-r * 1.6)
    v = v * smooth(r / 0.22)  # no hard point in the middle
    t = np.clip(v * 0.72 + glow * 0.22, 0, 1)[..., None]
    dark = np.array([4, 30, 8], np.float32)
    mid = np.array([38, 150, 22], np.float32)
    hi = np.array([165, 240, 95], np.float32)
    rgb = np.where(t < 0.55, dark + (mid - dark) * (t / 0.55), mid + (hi - mid) * ((t - 0.55) / 0.45))
    alpha = smooth((1.0 - r) / 0.18)
    out = np.dstack([rgb, alpha[..., None] * 255]).astype(np.uint8)
    return Image.fromarray(out, "RGBA")


def dim_icon():
    im = Image.open(HERE / "icon.png").convert("RGBA")
    grey = im.convert("LA").convert("RGBA")
    grey.putalpha(im.getchannel("A"))
    return ImageEnhance.Brightness(grey).enhance(0.55)


def sun(n=64, k=4):
    m = n * k
    im = Image.new("RGBA", (m, m), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    c = m / 2
    for i in range(8):
        a = i * math.pi / 4
        d.line([(c + math.cos(a) * m * 0.30, c + math.sin(a) * m * 0.30), (c + math.cos(a) * m * 0.46, c + math.sin(a) * m * 0.46)], fill=(255, 196, 88, 255), width=int(m * 0.07))
    d.ellipse([c - m * 0.2, c - m * 0.2, c + m * 0.2, c + m * 0.2], fill=(255, 205, 100, 255))
    return im.resize((n, n), Image.LANCZOS)


def moon(n=64, k=4):
    m = n * k
    im = Image.new("RGBA", (m, m), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    c = m / 2
    d.ellipse([c - m * 0.34, c - m * 0.34, c + m * 0.34, c + m * 0.34], fill=(205, 220, 255, 255))
    cut = Image.new("L", (m, m), 0)
    ImageDraw.Draw(cut).ellipse([c - m * 0.18, c - m * 0.44, c + m * 0.5, c + m * 0.24], fill=255)
    im.putalpha(Image.fromarray((np.asarray(im.getchannel("A"), np.float32) * (1 - np.asarray(cut, np.float32) / 255)).astype(np.uint8)))
    return im.resize((n, n), Image.LANCZOS)


swirl().save(HERE / "swirl.png", optimize=True)
sun().save(HERE / "sun.png", optimize=True)
moon().save(HERE / "moon.png", optimize=True)
dim_icon().save(HERE / "icon-dim.png", optimize=True)
print("wrote swirl.png, icon-dim.png, sun.png and moon.png")
