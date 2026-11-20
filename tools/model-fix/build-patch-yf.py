#!/usr/bin/env python3
"""Build data/patch-YF.mpq: repaired animation bounds for the HD world models in the base client's Patch-YD.mpq that the game cannot cull or pick correctly.

  tools/model-fix/build-patch-yf.py CLIENT_DATA_DIR OUT.mpq

The client culls and mouse-picks a model with the bounds and radius of the animation it is playing (M2Model.cpp, GetBoundingBox / bounding sphere). Seventeen of the
models Patch-YD ships for game objects have sequence records whose radius is a denormal (about 1e-40) and whose bounds are shifted or zero, so the object is hidden from
low camera angles and gets no tooltip or cursor (the Heartswood quest tree, GameObjectDisplayInfo 967, could not be clicked). A model is repaired when any of its
sequences has a radius under 1e-3 although its geometry has size. Only the bounds fields change: every sequence's box and radius (and the header box and radius) are
rewritten from the real vertex box plus 10%. Models with sane bounds are never touched.

Needs: StormLib (STORMLIB, a libstorm.so) and an MPQ writer (MPQ_LIB, wow-retail-backport's scripts/lib/mpq_write.py); both paths below are this project's.
The models checked are those named in the client's GameObjectDisplayInfo.dbc (read from patch-ZX.mpq, the last pack that carries one).
"""
import ctypes, math, os, re, struct, sys
from pathlib import Path

STORMLIB = os.environ.get("STORMLIB", "/home/shawn/Documents/FTA-Project/Tools/stormlib/libstorm.so")
MPQ_LIB = os.environ.get("MPQ_LIB", "/home/shawn/Documents/FTA-Project/Tools/wow-retail-backport/wow-retail-backport/scripts/lib")
sys.path.insert(0, MPQ_LIB)
from mpq_write import MpqWriter  # noqa: E402

L = ctypes.CDLL(STORMLIB)
L.SFileOpenArchive.argtypes = [ctypes.c_char_p, ctypes.c_uint, ctypes.c_uint, ctypes.POINTER(ctypes.c_void_p)]
L.SFileOpenFileEx.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_uint, ctypes.POINTER(ctypes.c_void_p)]
L.SFileGetFileSize.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint)]
L.SFileGetFileSize.restype = ctypes.c_uint
L.SFileReadFile.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_uint, ctypes.POINTER(ctypes.c_uint), ctypes.c_void_p]


def open_archive(path):
    h = ctypes.c_void_p()
    if not L.SFileOpenArchive(str(path).encode(), 0, 0x100, ctypes.byref(h)):
        sys.exit(f"cannot open {path}")
    return h


def read(h, name):
    f = ctypes.c_void_p()
    if not L.SFileOpenFileEx(h, name.encode(), 0, ctypes.byref(f)):
        return None
    sz = L.SFileGetFileSize(f, None)
    buf = ctypes.create_string_buffer(sz)
    n = ctypes.c_uint()
    L.SFileReadFile(f, buf, sz, ctypes.byref(n), None)
    return buf.raw[:n.value]


def vertex_box(m):
    n, o = struct.unpack_from("<2I", m, 0x3C)
    mn, mx = [1e9] * 3, [-1e9] * 3
    for i in range(n):
        p = struct.unpack_from("<3f", m, o + i * 48)
        for a in range(3):
            mn[a], mx[a] = min(mn[a], p[a]), max(mx[a], p[a])
    return mn, mx


def broken(m):
    """a sequence radius under 1e-3 although the model has size (the corrupt records)"""
    n, o = struct.unpack_from("<2I", m, 0x1C)
    if not n:
        return False
    mn, mx = vertex_box(m)
    if max(mx[a] - mn[a] for a in range(3)) < 0.1:
        return False
    return any(struct.unpack_from("<f", m, o + i * 64 + 56)[0] < 1e-3 for i in range(n))


def repair(m):
    m = bytearray(m)
    mn, mx = vertex_box(m)
    c = [(mn[i] + mx[i]) / 2 for i in range(3)]
    h = [max((mx[i] - mn[i]) / 2 * 1.1, 0.05) for i in range(3)]
    lo, hi = [c[i] - h[i] for i in range(3)], [c[i] + h[i] for i in range(3)]
    rad = math.sqrt(sum(x * x for x in h))
    n, o = struct.unpack_from("<2I", m, 0x1C)
    for i in range(n):
        b = o + i * 64
        struct.pack_into("<3f", m, b + 32, *lo)
        struct.pack_into("<3f", m, b + 44, *hi)
        struct.pack_into("<f", m, b + 56, rad)
    struct.pack_into("<6f", m, 0xA0, *lo, *hi)
    struct.pack_into("<f", m, 0xB8, rad)
    return bytes(m)


def display_models(zx):
    d = read(zx, "DBFilesClient\\GameObjectDisplayInfo.dbc")
    _, cnt, _, rs, _ = struct.unpack("<4sIIII", d[:20])
    strs = 20 + cnt * rs
    for i in range(cnt):
        r = d[20 + i * rs:20 + (i + 1) * rs]
        did = struct.unpack_from("<I", r, 0)[0]
        if did >= 1000000:           # our housing decor rows
            continue
        off = struct.unpack_from("<I", r, 4)[0]
        name = d[strs + off:d.index(b"\0", strs + off)].decode()
        if name.lower().endswith((".mdx", ".m2")):
            yield did, re.sub(r"\.mdx$", ".m2", name, flags=re.I)


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    data, out = Path(sys.argv[1]), sys.argv[2]
    yd, zx = open_archive(data / "Patch-YD.mpq"), open_archive(data / "patch-ZX.mpq")
    w = MpqWriter(out)
    fixed = []
    for did, name in sorted(display_models(zx)):
        m = read(yd, name)
        if m and broken(m):
            w.add(name, repair(m))
            fixed.append((did, name))
    w.close()
    print(f"{len(fixed)} models repaired -> {out}")
    for did, name in fixed:
        print(" ", did, name)


if __name__ == "__main__":
    main()
