import os
import re
import sys
import zlib

path = sys.argv[1]
out_dir = sys.argv[2] if len(sys.argv) > 2 else r"C:\Users\rrr\spotify-rs\re\out\cache-extract"

data = open(path, "rb").read()
print(f"{path}: {len(data):,} bytes\n")

os.makedirs(out_dir, exist_ok=True)

# Where is the URL, and what follows it?
m = re.search(rb"https://xpui\.app\.spotify\.com[^\x00]{0,300}", data)
if m:
    print(f"url at {m.start()}: {m.group()[:160]!r}\n")

# Try every plausible compression start and see what decompresses to JS.
results = []

# gzip
for magic, name in ((b"\x1f\x8b\x08", "gzip"), (b"\x28\xb5\x2f\xfd", "zstd")):
    start = 0
    while True:
        i = data.find(magic, start)
        if i < 0:
            break
        start = i + 1
        blob = data[i:]
        out = None
        if name == "gzip":
            try:
                out = zlib.decompress(blob, 16 + zlib.MAX_WBITS)
            except zlib.error:
                try:
                    out = zlib.decompress(blob)
                except zlib.error:
                    out = None
        else:
            try:
                import zstandard

                out = zstandard.ZstdDecompressor().decompress(blob, max_output_size=64 << 20)
            except Exception:
                out = None
        if out and (b"webpack" in out[:200000] or b"rspack" in out[:200000] or b"__webpack" in out[:200000]):
            results.append((name, i, out))

# raw deflate attempts at many offsets
for i in range(0, min(len(data), 400000), 1):
    if i % 1:
        continue
    try:
        out = zlib.decompress(data[i:], -zlib.MAX_WBITS)
    except zlib.error:
        continue
    except Exception:
        continue
    if len(out) > 50000 and (b"webpack" in out[:200000] or b"rspack" in out[:200000]):
        results.append(("deflate", i, out))
        break

print(f"decompressed candidates: {len(results)}")
for name, offset, out in results:
    print(f"  {name} at {offset} -> {len(out):,} bytes")
    target = os.path.join(out_dir, f"runtime-{name}-{offset}.js")
    open(target, "wb").write(out)
    print(f"    saved {target}")
    head = out[:300].decode("utf-8", "replace")
    print(f"    head: {head}")
