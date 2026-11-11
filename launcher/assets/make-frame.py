#!/usr/bin/env python3
"""Builds the launcher's window frame: weathered black metal with chipped edges, sharp splinters and a little fel
green glow along the inside. Static (nothing in it moves).

    python3 assets/make-frame.py [--seed 7]

It draws the whole frame once on a 3840 x 2400 canvas (a 1920 x 1200 window at 2x) and writes only the pieces the
window needs to assets/frame/: four corners (corner-tl/tr/bl/br.png, 120 x 120) and four edge strips (edge-top,
edge-bottom: 3600 x 68; edge-left, edge-right: 68 x 2160). ui/main.slint (MetalFrame) draws the corners at 60 x 60
and stretches the strips between them, so any window size works; the rivets are drawn by the window itself.

Layout in canvas pixels: the band's outer edge is 10 px inside the canvas edge (room for splinters), the band is
30 px thick, and the glow spills 28 px past its inner edge into the window.
"""
import argparse
import pathlib
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = pathlib.Path(__file__).resolve().parent
W, H = 3840, 2400
OUT, BAND, GLOW = 10, 30, 28
R_OUT = 44


def smooth(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)


def noise(rng, shape, cell):
    """Smooth value noise in 0..1 with features about `cell` pixels (cell may be a (cy, cx) pair)."""
    cy, cx = (cell, cell) if np.isscalar(cell) else cell
    h, w = shape
    small = rng.random((max(2, h // cy + 2), max(2, w // cx + 2))).astype(np.float32)
    img = Image.fromarray((small * 255).astype(np.uint8)).resize((w, h), Image.BICUBIC)
    return np.asarray(img, np.float32) / 255


def fbm(rng, shape, cells, weights):
    total = sum(weights)
    return sum(w * noise(rng, shape, c) for c, w in zip(cells, weights)) / total


def sd_rrect(x, y, x0, y0, x1, y1, r):
    """Signed distance to a rounded rectangle (negative inside)."""
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    qx = np.abs(x - cx) - ((x1 - x0) / 2 - r)
    qy = np.abs(y - cy) - ((y1 - y0) / 2 - r)
    return np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - r


def tri(draw, base_pt, normal, along, depth, width, skew, fill=255):
    """A triangle with its base centred on base_pt, apex `depth` px along `normal`, base `width` px along `along`."""
    bx, by = base_pt
    ax, ay = bx + normal[0] * depth + along[0] * skew, by + normal[1] * depth + along[1] * skew
    p1 = (bx - along[0] * width / 2, by - along[1] * width / 2)
    p2 = (bx + along[0] * width / 2, by + along[1] * width / 2)
    draw.polygon([p1, p2, (ax, ay)], fill=fill)


def perimeter_points(rng, x0, y0, x1, y1, margin, count):
    """Random points on the straight parts of a rectangle's sides, with the inward normal and the side direction."""
    sides = [
        ((x0 + margin, y0), (x1 - margin, y0), (0, 1), (1, 0)),
        ((x0 + margin, y1), (x1 - margin, y1), (0, -1), (1, 0)),
        ((x0, y0 + margin), (x0, y1 - margin), (1, 0), (0, 1)),
        ((x1, y0 + margin), (x1, y1 - margin), (-1, 0), (0, 1)),
    ]
    lengths = np.array([np.hypot(b[0] - a[0], b[1] - a[1]) for a, b, _, _ in sides])
    pts = []
    for _ in range(count):
        a, b, n, d = sides[rng.choice(4, p=lengths / lengths.sum())]
        t = rng.random()
        pts.append(((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t), n, d))
    return pts


def build(seed):
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)

    # ---- outline: rounded rectangles with rough edges ----
    x0, y0, x1, y1 = OUT, OUT, W - OUT, H - OUT
    sd_out = sd_rrect(xx, yy, x0, y0, x1, y1, R_OUT)
    sd_out += (fbm(rng, (H, W), [90, 24, 7], [3, 2, 1]) - 0.5) * 7
    ix0, iy0, ix1, iy1 = x0 + BAND, y0 + BAND, x1 - BAND, y1 - BAND
    sd_in = sd_rrect(xx, yy, ix0, iy0, ix1, iy1, 14)
    sd_in += (fbm(rng, (H, W), [120, 30, 8], [3, 2, 1]) - 0.5) * 8
    outer = (sd_out < 0)
    hole = (sd_in < 0)

    # chips bitten out of the outer edge, splinters sticking out of it, and the same on the inner edge
    bite_out = Image.new("L", (W, H), 0)
    spike_out = Image.new("L", (W, H), 0)
    bite_in = Image.new("L", (W, H), 0)   # removes band from the hole side (hole grows)
    spike_in = Image.new("L", (W, H), 0)  # band splinters pointing into the hole
    d1, d2, d3, d4 = (ImageDraw.Draw(m) for m in (bite_out, spike_out, bite_in, spike_in))
    margin = 90
    for p, n, a in perimeter_points(rng, x0, y0, x1, y1, margin, 150):
        tri(d1, p, n, a, rng.uniform(5, 20), rng.uniform(6, 26), rng.uniform(-8, 8))
    for p, n, a in perimeter_points(rng, x0, y0, x1, y1, margin, 14):  # a few big missing chunks
        tri(d1, p, n, a, rng.uniform(18, 26), rng.uniform(40, 80), rng.uniform(-14, 14))
    for p, n, a in perimeter_points(rng, x0, y0, x1, y1, margin, 70):  # sharp splinters (apex points outward)
        tri(d2, p, (-n[0], -n[1]), a, rng.uniform(5, 9), rng.uniform(5, 14), rng.uniform(-5, 5))
    for p, n, a in perimeter_points(rng, ix0, iy0, ix1, iy1, margin, 60):
        tri(d3, p, (-n[0], -n[1]), a, rng.uniform(4, 12), rng.uniform(6, 22), rng.uniform(-6, 6))
    for p, n, a in perimeter_points(rng, ix0, iy0, ix1, iy1, margin, 22):  # slivers pointing into the window
        tri(d4, p, n, a, rng.uniform(5, 13), rng.uniform(4, 11), rng.uniform(-5, 5))
    outer = (outer & ~(np.asarray(bite_out) > 0)) | (np.asarray(spike_out) > 0)
    hole = (hole | (np.asarray(bite_in) > 0)) & ~(np.asarray(spike_in) > 0)
    band = outer & ~hole
    band_img = Image.fromarray((band * 255).astype(np.uint8))
    band_a = np.asarray(band_img.filter(ImageFilter.GaussianBlur(0.8)), np.float32) / 255

    # ---- metal ----
    base = np.array([36, 36, 46], np.float32)
    dx, dy = np.minimum(xx, W - xx), np.minimum(yy, H - yy)
    horizontal = (dy < dx)[..., None]
    streak_h = noise(rng, (H, W), (2, 70))
    streak_v = noise(rng, (H, W), (70, 2))
    streaks = np.where(horizontal[..., 0], streak_h, streak_v)
    mottle = fbm(rng, (H, W), [160, 40, 10], [3, 2, 1])
    shade = 0.62 + 0.55 * mottle + 0.35 * (streaks - 0.5)
    col = base[None, None, :] * shade[..., None]
    # dark grime and pitting
    grime = fbm(rng, (H, W), [260, 60], [2, 1])
    col *= (1 - 0.55 * smooth((grime - 0.58) / 0.2))[..., None]
    pits = noise(rng, (H, W), 4)
    col *= (1 - 0.5 * smooth((pits - 0.82) / 0.1))[..., None]
    # scratches and scuffs (thin bright lines)
    sc = Image.new("L", (W, H), 0)
    ds = ImageDraw.Draw(sc)
    for _ in range(900):
        x, y = rng.uniform(0, W), rng.uniform(0, H)
        ang = rng.choice([0, np.pi / 2]) + rng.normal(0, 0.12)
        ln = rng.uniform(20, 130)
        ds.line([(x, y), (x + ln * np.cos(ang), y + ln * np.sin(ang))], fill=int(rng.uniform(40, 120)), width=1)
    scratch = np.asarray(sc.filter(ImageFilter.GaussianBlur(0.5)), np.float32) / 255
    col += scratch[..., None] * np.array([95, 98, 112], np.float32)
    # bare, bright metal where the edges are worn: a rim along every edge (including chips and splinters)
    erode = lambda m, k: np.asarray(m.filter(ImageFilter.MinFilter(k)), np.float32) / 255
    rim = np.clip(1 - erode(band_img, 7), 0, 1) * band_a            # about 3 px deep
    wear = smooth((noise(rng, (H, W), 14) - 0.35) / 0.4)
    col = col * (1 - rim[..., None] * 0.85 * wear[..., None]) + np.array([150, 152, 168], np.float32) * (rim * 0.85 * wear)[..., None]
    col += np.array([40, 42, 52], np.float32) * (rim * (1 - wear))[..., None]
    # a darker groove just inside the outer rim
    groove = np.clip(erode(band_img, 7) - erode(band_img, 13), 0, 1) * band_a
    col *= (1 - 0.45 * groove)[..., None]

    # ---- fel green ----
    green = np.array([125, 255, 58], np.float32)
    hole_img = Image.fromarray((hole * 255).astype(np.uint8))
    near_hole = np.clip(1 - np.asarray(hole_img.filter(ImageFilter.MaxFilter(1)), np.float32) / 255, 0, 1)
    inner_rim = np.clip(np.asarray(hole_img.filter(ImageFilter.MaxFilter(5)), np.float32) / 255 - hole * 1.0, 0, 1) * band_a  # ~4 px
    inner_rim *= 0.55 + 0.45 * noise(rng, (H, W), 30)
    col = col * (1 - 0.6 * inner_rim[..., None]) + green * (0.6 * inner_rim)[..., None]
    # glowing cracks running out of the inner edge into the band
    cr = Image.new("L", (W, H), 0)
    dc = ImageDraw.Draw(cr)
    for p, n, a in perimeter_points(rng, ix0, iy0, ix1, iy1, margin, 16):
        x, y = p
        ang = np.arctan2(-n[1], -n[0])
        pts = [(x, y)]
        for _ in range(int(rng.uniform(4, 9))):
            ang += rng.normal(0, 0.55)
            step = rng.uniform(4, 9)
            x, y = x + step * np.cos(ang), y + step * np.sin(ang)
            pts.append((x, y))
            if abs(x - ix0) > BAND + 2 and abs(x - ix1) > BAND + 2 and abs(y - iy0) > BAND + 2 and abs(y - iy1) > BAND + 2:
                break
        dc.line(pts, fill=255, width=2)
    crack = np.asarray(cr, np.float32) / 255 * band_a
    crack_glow = np.asarray(cr.filter(ImageFilter.GaussianBlur(3)), np.float32) / 255 * band_a
    col = col * (1 - 0.8 * crack[..., None]) + green * (0.8 * crack)[..., None]
    col += green * (0.25 * crack_glow)[..., None]
    # faint green staining in the band near the inner edge
    stain = smooth((fbm(rng, (H, W), [70, 18], [2, 1]) - 0.5) / 0.25) * np.asarray(hole_img.filter(ImageFilter.GaussianBlur(16)), np.float32) / 255
    col += np.array([10, 48, 6], np.float32) * (stain * band_a)[..., None]

    # glow spilling from the inner edge into the window
    spill = np.asarray(Image.fromarray((band * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(9)), np.float32) / 255
    glow = np.clip(spill * 1.15 - 0.1, 0, 1) * hole * (1 - smooth((np.maximum(sd_in, 0) - 4) / GLOW * 0 + 0))
    glow *= smooth(1 - np.maximum(0, -sd_in) / GLOW * 0)  # (kept simple: the blur sets the falloff)
    glow = glow * (0.22 + 0.14 * noise(rng, (H, W), 90)) * 1.0
    glow += np.asarray(cr.filter(ImageFilter.GaussianBlur(5)), np.float32) / 255 * hole * 0.0

    alpha = band_a + glow * (1 - band_a)
    rgb = (col * band_a[..., None] + green * (glow * (1 - band_a))[..., None]) / np.maximum(alpha, 1e-4)[..., None]
    out = np.dstack([np.clip(rgb, 0, 255), np.clip(alpha * 255, 0, 255)]).astype(np.uint8)
    return Image.fromarray(out, "RGBA")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=7)
    args = ap.parse_args()
    img = build(args.seed)
    out = HERE / "frame"
    out.mkdir(exist_ok=True)
    C, T = 120, BAND + OUT + GLOW  # corner size, strip thickness (68)
    img.crop((0, 0, C, C)).save(out / "corner-tl.png", optimize=True)
    img.crop((W - C, 0, W, C)).save(out / "corner-tr.png", optimize=True)
    img.crop((0, H - C, C, H)).save(out / "corner-bl.png", optimize=True)
    img.crop((W - C, H - C, W, H)).save(out / "corner-br.png", optimize=True)
    img.crop((C, 0, W - C, T)).save(out / "edge-top.png", optimize=True)
    img.crop((C, H - T, W - C, H)).save(out / "edge-bottom.png", optimize=True)
    img.crop((0, C, T, H - C)).save(out / "edge-left.png", optimize=True)
    img.crop((W - T, C, W, H - C)).save(out / "edge-right.png", optimize=True)
    print("wrote", ", ".join(sorted(p.name for p in out.glob("*.png"))))


main()
