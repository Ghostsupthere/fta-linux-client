"""Writes a BLP2 (DXT5, with mipmaps) from any image file, using ImageMagick for the block compression."""
import os, struct, subprocess, tempfile

def to_blp(src, dst):
    tmp = tempfile.mkdtemp()
    dds = os.path.join(tmp, "t.dds")
    subprocess.run(["magick", src, "-define", "dds:compression=dxt5", "-define", "dds:mipmaps=11", dds], check=True)
    d = open(dds, "rb").read()
    h, w, mips = struct.unpack_from("<II", d, 12)[0], struct.unpack_from("<I", d, 16)[0], struct.unpack_from("<I", d, 28)[0]
    height, width = struct.unpack_from("<II", d, 12)
    mips = max(1, struct.unpack_from("<I", d, 28)[0])
    assert d[84:88] == b"DXT5", d[84:88]
    off, level, sizes, datas = 128, 0, [], []
    cw, ch = width, height
    for i in range(mips):
        sz = max(1, (cw + 3) // 4) * max(1, (ch + 3) // 4) * 16
        datas.append(d[off:off + sz]); sizes.append(sz); off += sz
        cw, ch = max(1, cw // 2), max(1, ch // 2)
    hdr_len = 4 + 4 + 4 + 8 + 64 + 64 + 1024
    offs, pos = [], hdr_len
    for sz in sizes:
        offs.append(pos); pos += sz
    offs += [0] * (16 - len(offs)); sizes_p = sizes + [0] * (16 - len(sizes))
    hdr = b"BLP2" + struct.pack("<I", 1) + struct.pack("<BBBB", 2, 8, 7, 1) + struct.pack("<II", width, height)
    hdr += struct.pack("<16I", *offs) + struct.pack("<16I", *sizes_p) + b"\0" * 1024
    open(dst, "wb").write(hdr + b"".join(datas))
    return width, height, mips
