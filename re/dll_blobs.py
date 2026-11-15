"""Scan a PE for compressed blobs and report any that decompress into
JavaScript containing webpack/rspack markers.
"""

import sys
import zlib

import zstandard

path = sys.argv[1]
needles = [b"__webpack", b"rspack", b"webpack_require"]

data = open(path, "rb").read()
print(f"{path}: {len(data):,} bytes\n")

dctx = zstandard.ZstdDecompressor()
hits = 0

# zstd frames
start = 0
while True:
    i = data.find(b"\x28\xb5\x2f\xfd", start)
    if i < 0:
        break
    start = i + 1
    try:
        out = dctx.decompress(data[i:], max_output_size=64 << 20)
    except Exception:
        try:
            reader = dctx.stream_reader(data[i:])
            out = reader.read()
        except Exception:
            continue
    if out and any(n in out for n in needles):
        hits += 1
        print(f"ZSTD hit at {i}: {len(out):,} bytes")
        open(rf"C:\Users\rrr\spotify-rs\re\out\cache-extract\dll-zstd-{i}.js", "wb").write(out)
        print(out[:300].decode("utf-8", "replace"))
        print()

# gzip frames
start = 0
while True:
    i = data.find(b"\x1f\x8b\x08", start)
    if i < 0:
        break
    start = i + 1
    try:
        out = zlib.decompress(data[i:], 16 + zlib.MAX_WBITS)
    except zlib.error:
        continue
    if len(out) > 20000 and any(n in out for n in needles):
        hits += 1
        print(f"GZIP hit at {i}: {len(out):,} bytes")
        open(rf"C:\Users\rrr\spotify-rs\re\out\cache-extract\dll-gzip-{i}.js", "wb").write(out)
        print(out[:300].decode("utf-8", "replace"))
        print()

# raw deflate
for i in range(0, len(data), 1):
    try:
        out = zlib.decompress(data[i:], -zlib.MAX_WBITS)
    except zlib.error:
        continue
    except Exception:
        continue
    if len(out) > 50000 and any(n in out for n in needles):
        hits += 1
        print(f"DEFLATE hit at {i}: {len(out):,} bytes")
        open(rf"C:\Users\rrr\spotify-rs\re\out\cache-extract\dll-deflate-{i}.js", "wb").write(out)
        print(out[:300].decode("utf-8", "replace"))
        break

print(f"total hits: {hits}")
