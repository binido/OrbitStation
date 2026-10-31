"""Extracts a .NET single-file bundle: unbundle.py <SpaceStationService.exe> <out_dir>"""
import sys, struct, os, zlib

SIG = bytes.fromhex("8b1202b96a612038727b930214d7a03213f5b9e6efae3318ee3b2dce24b36aae")
data = open(sys.argv[1], "rb").read()
out = sys.argv[2]
hdr = struct.unpack_from("<q", data, data.index(SIG) - 8)[0]


def rstr(p):
    n = s = 0
    while True:
        b = data[p]; p += 1
        n |= (b & 0x7F) << s; s += 7
        if not b & 0x80:
            break
    return data[p:p + n].decode(), p + n


major, minor, count = struct.unpack_from("<IIi", data, hdr)
_, p = rstr(hdr + 12)
if major >= 2:
    p += 8 * 4 + 8
print("bundle v%d.%d, %d files" % (major, minor, count))
for _ in range(count):
    off, size = struct.unpack_from("<qq", data, p); p += 16
    csize = 0
    if major >= 6:
        csize = struct.unpack_from("<q", data, p)[0]; p += 8
    p += 1
    path, p = rstr(p)
    blob = data[off:off + (csize or size)]
    if csize:
        blob = zlib.decompress(blob, -15)
    dst = os.path.join(out, path.replace("\\", "/"))
    os.makedirs(os.path.dirname(dst) or out, exist_ok=True)
    open(dst, "wb").write(blob)
