#!/usr/bin/env python3
"""Builds the aged, cracked stone border drawn around the launcher window: assets/frame.png (day) and
assets/frame-night.png (the same stone, cooler).

The window is 1000 x 640; the frame is made at twice that. It is a ring of stone with a chipped outer edge (the
chips are transparent, so the window outline looks broken), bevelled edges lit from the top left, pitted and
mottled stone, and cracks of three kinds: short splits that run across the ring, cracks that wander along it, and a
loose network of cell cracks. The damage grows toward the bottom right corner (bigger chips, more and longer
cracks, more grime). A few cracks glow with ember light. The launcher's content starts 5 px inside the window
(INSET below), so the outer 5 px of the ring show only the frame and the chips cut into the window outline.

    python3 assets/make-frame.py [--seed 7] [--band 18] [--glow 0.16]
"""
import argparse
import math
import pathlib
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = pathlib.Path(__file__).resolve().parent
S = 2                       # pixels per window pixel
W, H = 1000 * S, 640 * S
RADIUS = 20 * S             # outer corner radius
INSET = 5 * S               # the launcher content starts this far inside the window

STONES = {
    "day": ((52, 44, 68), (122, 108, 140)),      # dark and light stone, violet cast
    "night": ((40, 48, 78), (104, 120, 158)),    # the same stone under moonlight
}


def smooth(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)


def noise(rng, cells, shape=(H, W)):
    """Smooth value noise in 0..1 with about `cells` cells across the width."""
    h = max(2, int(cells * shape[0] / shape[1]) + 2)
    w = int(cells) + 2
    a = rng.random((h, w)).astype(np.float32)
    img = Image.fromarray((a * 255).astype(np.uint8)).resize((shape[1], shape[0]), Image.BICUBIC)
    return np.asarray(img, np.float32) / 255.0


def fbm(rng, cells, octaves=4):
    out = np.zeros((H, W), np.float32)
    amp, total = 1.0, 0.0
    for _ in range(octaves):
        out += noise(rng, cells) * amp
        total += amp
        amp *= 0.5
        cells *= 2
    return out / total


def rounded_rect_sd(inset):
    """Signed distance (negative inside) to the window's rounded rectangle shrunk by `inset`."""
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)
    cx, cy = W / 2, H / 2
    hw, hh = W / 2 - inset, H / 2 - inset
    r = max(RADIUS - inset, 4)
    qx = np.abs(xx + 0.5 - cx) - (hw - r)
    qy = np.abs(yy + 0.5 - cy) - (hh - r)
    outside = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0))
    inside = np.minimum(np.maximum(qx, qy), 0)
    return outside + inside - r


def voronoi_edges(rng, cell, warp):
    """F2 - F1 of a jittered point grid (small along cell borders), with the coordinates warped for jagged lines."""
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)
    xx = xx + (fbm(rng, 18, 3) - 0.5) * warp
    yy = yy + (fbm(rng, 18, 3) - 0.5) * warp
    gx, gy = int(W / cell) + 3, int(H / cell) + 3
    px = (np.arange(gx)[None, :] + rng.random((gy, gx))) * cell - cell
    py = (np.arange(gy)[:, None] + rng.random((gy, gx))) * cell - cell
    ci = np.clip((xx / cell + 1).astype(int), 1, gx - 2)
    cj = np.clip((yy / cell + 1).astype(int), 1, gy - 2)
    f1 = np.full((H, W), 1e9, np.float32)
    f2 = np.full((H, W), 1e9, np.float32)
    for dj in (-1, 0, 1):
        for di in (-1, 0, 1):
            d = np.hypot(xx - px[cj + dj, ci + di], yy - py[cj + dj, ci + di])
            f2 = np.where(d < f1, f1, np.minimum(f2, d))
            f1 = np.minimum(f1, d)
    return f2 - f1


def perimeter(inset):
    """Points (and arc length) along the rounded rectangle shrunk by `inset`, clockwise from the top edge."""
    r = max(RADIUS - inset, 4)
    x0, y0, x1, y1 = inset, inset, W - inset, H - inset
    pts = []

    def line(a, b, n):
        for i in range(n):
            t = i / n
            pts.append((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t))

    def arc(cx, cy, a0, a1, n=24):
        for i in range(n):
            a = math.radians(a0 + (a1 - a0) * i / n)
            pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))

    line((x0 + r, y0), (x1 - r, y0), 200)
    arc(x1 - r, y0 + r, -90, 0)
    line((x1, y0 + r), (x1, y1 - r), 130)
    arc(x1 - r, y1 - r, 0, 90)
    line((x1 - r, y1), (x0 + r, y1), 200)
    arc(x0 + r, y1 - r, 90, 180)
    line((x0, y1 - r), (x0, y0 + r), 130)
    arc(x0 + r, y0 + r, 180, 270)
    return np.array(pts, np.float32)


def draw_crack(draw, pts, width):
    """Polyline with a width that tapers toward the end."""
    n = len(pts) - 1
    for i in range(n):
        w = max(1, round(width * (1.0 - 0.75 * i / max(n, 1))))
        draw.line([pts[i], pts[i + 1]], fill=255, width=w)


def walk(rng, start, heading, length, step, jitter, branch_p, out, depth=0, glow=False, glow_out=None):
    """A crack as a random walk that sometimes forks; every polyline is appended to `out`."""
    pts = [tuple(start)]
    x, y = start
    turn = 0.0
    n = int(length / step)
    for i in range(n):
        turn = turn * 0.6 + rng.normal(0, jitter)
        heading += turn
        x += math.cos(heading) * step
        y += math.sin(heading) * step
        pts.append((x, y))
        if depth < 2 and rng.random() < branch_p:
            side = rng.choice([-1, 1]) * rng.uniform(0.5, 1.1)
            walk(rng, (x, y), heading + side, length * rng.uniform(0.25, 0.5), step, jitter * 1.2, branch_p * 0.5,
                 out, depth + 1, glow, glow_out)
    out.append((pts, 1.0 - 0.25 * depth, glow))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--band", type=float, default=18, help="width of the stone ring in window pixels")
    ap.add_argument("--glow", type=float, default=0.16, help="share of the cracks that glow with ember light")
    ap.add_argument("--only", choices=["day", "night"], help="build just one of the two")
    args = ap.parse_args()
    for name in ([args.only] if args.only else ["day", "night"]):
        build(name, args)


def build(name, args):
    rng = np.random.default_rng(args.seed)
    band = args.band * S
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)

    # Weathering: mild everywhere, worst at the bottom right corner and creeping up the right edge and along the bottom.
    wear = 0.2 + 0.8 * smooth(((xx / W) * 0.6 + (yy / H) * 0.4 - 0.52) / 0.48)

    sd = rounded_rect_sd(0)  # negative inside the window outline
    # Chipped outer edge: slow waviness, fine roughness and chunks missing (never deeper than INSET).
    wav = np.clip((fbm(rng, 14, 3) - 0.5) * 2 * 2.2 * S, 0, 3 * S)
    rough = np.clip((noise(rng, 240) - 0.5) * 2 * 0.9 * S, 0, None)
    chunk_noise = fbm(rng, 28, 3)
    chunks = smooth((chunk_noise - (0.73 - 0.13 * wear)) / 0.1) * ((2.0 + 3.0 * wear) * S)
    outer = sd + wav + rough + np.clip(chunks, 0, INSET + 1.0 * S)
    # Inner edge sits `band` inside the outline, with its own wobble and more chips toward the bottom right.
    inner_wob = (fbm(rng, 18, 3) - 0.5) * 2 * 2.5 * S + (noise(rng, 160) - 0.5) * 1.2 * S
    inner_chip = smooth((fbm(rng, 24, 3) - (0.76 - 0.08 * wear)) / 0.1) * ((2.0 + 3.0 * wear) * S)
    depth = -sd
    band_edge = band + inner_wob - inner_chip

    aa = 1.3
    alpha_outer = smooth((-outer) / aa + 0.5)
    alpha_inner = smooth((band_edge - depth) / aa + 0.5)
    alpha = alpha_outer * alpha_inner
    d_outer = np.clip(-outer, 0, None)
    d_inner = np.clip(band_edge - depth, 0, None)

    # ---- stone colour: mottled, with strata, pits and fine grain ----
    c_dark, c_light = (np.array(c, np.float32) for c in STONES[name])
    m1, m2 = fbm(rng, 9, 4), fbm(rng, 40, 3)
    mottle = smooth((m1 - 0.3) / 0.5)[..., None]
    stone = c_dark + (c_light - c_dark) * mottle
    stone = stone * (0.86 + 0.28 * m2[..., None])
    strata = (fbm(rng, 6, 2) * 0.5 + np.sin((xx * 0.012 + yy * 0.05 + fbm(rng, 5, 2) * 6)) * 0.5 + 0.5) * 0.5
    stone = stone * (0.93 + 0.14 * strata[..., None])
    stone = stone + (noise(rng, 520) - 0.5)[..., None] * 28
    # pits (dark) and mineral flecks (light); more pits where the stone is worn
    fine = noise(rng, 900)
    pits = smooth((fine - (0.9 - 0.06 * wear)) / 0.05)
    flecks = smooth((noise(rng, 760) - 0.9) / 0.04)
    stone = stone * (1 - 0.3 * pits[..., None]) + 22 * flecks[..., None]

    # ---- bevel: height rises from both edges; lit from the top left ----
    h = smooth(np.minimum(d_outer, d_inner) / (5.5 * S)) + (fbm(rng, 60, 3) - 0.5) * 0.35 + (noise(rng, 300) - 0.5) * 0.12
    gy_, gx_ = np.gradient(h)
    shade = np.clip(1.0 + ((gx_ + gy_) * 60.0), 0.6, 1.5)[..., None]
    stone = stone * shade
    stone = stone * (1 - (1 - smooth(np.minimum(d_outer, d_inner) / (1.6 * S)))[..., None] * 0.55)
    # freshly broken edges (where a chunk is missing) are lighter, old edges darker
    fresh = smooth(chunks / (2.0 * S)) * (1 - smooth(d_outer / (3.5 * S)))
    stone = stone + fresh[..., None] * 36

    # ---- cracks ----
    ring_mask = (alpha > 0.5)
    crack_img = Image.new("L", (W, H), 0)
    glow_img = Image.new("L", (W, H), 0)
    cd, gd = ImageDraw.Draw(crack_img), ImageDraw.Draw(glow_img)
    n_xy = np.gradient(sd)  # (d/dy, d/dx) of the signed distance: points outward
    polys = []

    # 1. short splits that run across the ring, starting at either edge; many more where the stone is worn
    attempts = 2600
    for _ in range(attempts):
        x, y = rng.integers(8, W - 8), rng.integers(8, H - 8)
        if not ring_mask[y, x]:
            continue
        if rng.random() > 0.05 + 0.95 * float(wear[y, x]) ** 1.6:
            continue
        ny, nx = float(n_xy[0][y, x]), float(n_xy[1][y, x])
        norm = math.hypot(nx, ny) or 1
        heading = math.atan2(ny / norm, nx / norm) + (math.pi if rng.random() < 0.5 else 0) + rng.normal(0, 0.35)
        length = band * rng.uniform(0.55, 1.35) * (0.8 + 0.5 * float(wear[y, x]))
        walk(rng, (x, y), heading, length, 5 * S / 2, 0.22, 0.10, polys, glow=rng.random() < args.glow)

    # 2. cracks that wander along the ring; a lot of them near the bottom right
    per = perimeter(band * 0.5)
    seg = np.hypot(*(np.roll(per, -1, axis=0) - per).T)
    for i in range(14):
        # pick a start biased to the bottom right
        c = per[:, 0] / W + per[:, 1] / H
        p = np.exp((c - c.max()) * 7) + 0.02
        k = int(rng.choice(len(per), p=p / p.sum()))
        total = rng.uniform(160, 520) * S / 2
        pts, acc, j = [], 0.0, k
        lateral = np.cumsum(rng.normal(0, 0.9, 400))
        lateral = (lateral - lateral.mean()) / (np.abs(lateral).max() + 1e-6) * band * 0.3
        while acc < total and len(pts) < 399:
            px, py = per[j % len(per)]
            ix, iy = int(np.clip(px, 0, W - 1)), int(np.clip(py, 0, H - 1))
            ny, nx = float(n_xy[0][iy, ix]), float(n_xy[1][iy, ix])
            norm = math.hypot(nx, ny) or 1
            pts.append((px + nx / norm * lateral[len(pts)], py + ny / norm * lateral[len(pts)]))
            acc += seg[j % len(per)]
            j += 1
        pts = [(px + rng.normal(0, 1.6 * S / 2), py + rng.normal(0, 1.6 * S / 2)) for px, py in pts[::2]] if len(pts) > 8 else pts
        # break the line into pieces with gaps, so it reads as a crack that comes and goes
        a = 0
        while a < len(pts) - 2:
            b = min(len(pts), a + int(rng.integers(5, 14)))
            polys.append((pts[a:b], 0.75, False))
            a = b + int(rng.integers(0, 3))
        # a few forks off the long cracks
        for q in range(0, len(pts) - 6, max(8, len(pts) // 5)):
            if rng.random() < 0.55:
                ang = math.atan2(pts[q + 3][1] - pts[q][1], pts[q + 3][0] - pts[q][0]) + rng.choice([-1, 1]) * rng.uniform(0.7, 1.3)
                walk(rng, pts[q], ang, band * rng.uniform(0.5, 1.0), 5 * S / 2, 0.25, 0.05, polys, depth=1)

    for pts, scale, glow in polys:
        draw_crack(cd, pts, max(1.0, 3.0 * S / 2 * scale))
        if glow:
            draw_crack(gd, pts, max(1.0, 2.2 * S / 2 * scale))

    # 3. a loose network of cell cracks, again stronger toward the bottom right
    edges = voronoi_edges(rng, 78 * S / 2, 26 * S)
    cell_where = smooth((fbm(rng, 5, 3) - 0.62) / 0.1) * wear
    cell = (1 - smooth((edges - 0.6 * S) / (1.0 * S))) * cell_where

    crack = np.maximum(np.asarray(crack_img.filter(ImageFilter.GaussianBlur(0.7)), np.float32) / 255.0, cell)
    crack = np.clip(crack * 1.4, 0, 1) * ring_mask
    ao = np.asarray(Image.fromarray((crack * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(3.5 * S / 2)), np.float32) / 255.0
    stone = stone * (1 - 0.35 * np.clip(ao * 2.2, 0, 1)[..., None])       # shadowed stone beside a crack
    stone = stone * (1 - 0.88 * crack[..., None])                           # the crack itself
    lip = np.roll(np.roll(crack, 1 * S, axis=0), 1 * S, axis=1) * (1 - crack)
    stone = stone + 26 * lip[..., None]                                     # light edge on the lower right side
    # spalling: little flakes broken out where cracks run
    flake = smooth((fine - 0.78) / 0.05) * np.clip(ao * 3, 0, 1)
    stone = stone * (1 - 0.35 * flake[..., None])

    # ember light deep in some cracks
    g = np.asarray(glow_img.filter(ImageFilter.GaussianBlur(0.8)), np.float32) / 255.0
    gcore = np.clip((g - 0.35) * 1.8, 0, 1)
    gglow = np.asarray(glow_img.filter(ImageFilter.GaussianBlur(4 * S / 2)), np.float32) / 255.0
    ember = np.array([255, 150, 40], np.float32)
    stone = stone + ember * (gglow * 0.30)[..., None] * ring_mask[..., None] + np.array([255, 200, 90], np.float32) * (gcore * 0.45)[..., None]

    # grime and soot: darker in the lower right, and in the low spots
    grime = smooth((fbm(rng, 12, 4) - 0.35) / 0.4) * wear
    stone = stone * (1 - 0.38 * grime[..., None])

    # soft shadow cast inward onto the launcher by the inner edge (part of the frame image)
    shadow_d = np.clip(depth - band_edge, 0, None)
    shadow = (1 - smooth(shadow_d / (10 * S))) * 0.55 * (1 - alpha_inner)

    rgb = np.clip(stone, 0, 255)
    a = np.clip(alpha + shadow * (1 - alpha), 0, 1)
    out_rgb = np.where(alpha[..., None] > 0.001, rgb, 0)
    col = (out_rgb * alpha[..., None]) / np.maximum(a, 1e-4)[..., None]
    img = np.dstack([np.clip(col, 0, 255), a * 255]).astype(np.uint8)
    fname = "frame.png" if name == "day" else "frame-night.png"
    Image.fromarray(img, "RGBA").save(HERE / fname, optimize=True)
    print(f"wrote {HERE / fname} ({W}x{H}); content inset {INSET // S} px; {len(polys)} crack paths")


main()
