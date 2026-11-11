#!/usr/bin/env python3
"""Pictures that let the launcher animate the whole portal opening (they are laid over the same spot of the artwork).

    python3 assets/make-portal.py

For each of day and night it writes
  vortex-<name>.jpg   a square texture of the vortex, 600 artwork pixels across at 1.5x: the real swirl out to
                      RADIUS from its eye, then that swirl mirrored outwards so the texture is big enough to
                      fill the whole opening however it is turned. ui/main.slint turns it slowly inside the
                      opening (a clipped box), so the whole portal swirls, not just its middle.
  edge-<name>.png     the painted rim of the opening (artwork pixels, soft inner edge, transparent elsewhere),
                      drawn over the turning vortex so the glowing stone edge stays where it was painted.

Numbers are in artwork pixels (1536 x 1024): EYE is the middle of the vortex, OPENING the clipped box
(left, top, right, bottom) just inside the stone, RADIUS how much of the real swirl is used. The same numbers are in
ui/main.slint (PortalLayer calls).
"""
import pathlib
import numpy as np
from PIL import Image

HERE = pathlib.Path(__file__).resolve().parent
EYE = (810.0, 620.0)
OPENING = (626, 378, 954, 842)
RADIUS = 135.0
SIZE = 600      # artwork pixels covered by the texture
SCALE = 1.5     # texture pixels per artwork pixel
PAD = 4         # how far the rim picture reaches past the box
RIM = 46        # width of the painted rim kept on top (it fades out over this distance)


def smooth(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)


def bilinear(img, x, y):
    h, w = img.shape[:2]
    x = np.clip(x, 0, w - 1.001)
    y = np.clip(y, 0, h - 1.001)
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
    fx, fy = (x - x0)[..., None], (y - y0)[..., None]
    a = img[y0, x0] * (1 - fx) + img[y0, x0 + 1] * fx
    b = img[y0 + 1, x0] * (1 - fx) + img[y0 + 1, x0 + 1] * fx
    return a * (1 - fy) + b * fy


for name in ("day", "night"):
    art = np.asarray(Image.open(HERE / f"art-{name}.jpg").convert("RGB"), np.float32)

    n = int(SIZE * SCALE)
    v, u = np.mgrid[0:n, 0:n].astype(np.float32)
    dx, dy = (u - n / 2 + 0.5) / SCALE, (v - n / 2 + 0.5) / SCALE
    r = np.hypot(dx, dy)
    t = np.mod(r, 2 * RADIUS)
    rr = np.where(t <= RADIUS, t, 2 * RADIUS - t)           # mirrored outwards, over and over
    k = np.where(r > 1e-3, rr / np.maximum(r, 1e-3), 1.0)
    tex = bilinear(art, EYE[0] + dx * k, EYE[1] + dy * k)
    Image.fromarray(np.clip(tex, 0, 255).astype(np.uint8)).save(HERE / f"vortex-{name}.jpg", quality=88)

    # The rim picture reaches PAD pixels past the box on every side (all opaque there), so its own soft edge never
    # lands on the box edge, where the turning vortex would otherwise show through as a thin dark line.
    l, t_, rgt, b = OPENING
    w, h = rgt - l, b - t_
    px = art[t_ - PAD:b + PAD, l - PAD:rgt + PAD]
    yy, xx = np.mgrid[-PAD:h + PAD, -PAD:w + PAD].astype(np.float32)
    d = np.maximum(np.minimum.reduce([xx, w - 1 - xx, yy, h - 1 - yy]), 0)   # distance in from the box edge
    alpha = (1 - smooth(d / RIM)) ** 1.3
    out = np.dstack([px.astype(np.uint8), (alpha * 255).astype(np.uint8)])
    img = Image.fromarray(out, "RGBA").resize(((w + 2 * PAD) * 2, (h + 2 * PAD) * 2), Image.LANCZOS)
    img.save(HERE / f"edge-{name}.png", optimize=True)
    print(f"wrote vortex-{name}.jpg ({n}x{n}) and edge-{name}.png ({img.width}x{img.height})")
