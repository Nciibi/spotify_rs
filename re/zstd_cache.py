"""Pull the cached xpui.app.spotify.com response body out of the Chromium
cache and decompress it (the edge serves zstd).
"""

import os
import re
import sys

import zstandard

path = sys.argv[1]
out_dir = sys.argv[2] if len(sys.argv) > 2 else r"C:\Users\rrr\spotify-rs\re\out\cache-extract"

data = open(path, "rb").read()
os.makedirs(out_dir, exist_ok=True)

dctx = zstandard.ZstdDecompressor()

# Find every "content-encoding:zstd" header and try to decompress from the
# end of the following header block.
hits = 0
for m in re.finditer(rb"content-encoding:zstd", data):
    start = m.end()
    # The body begins after the next blank line (end of headers).
    end_of_headers = data.find(b"\x00\x00\x00\x00", start)
    candidates = []
    if end_of_headers > 0:
        candidates.append(end_of_headers + 4)
    candidates.append(start)
    for offset in candidates:
        for length in (282, 4096, 65536, 1 << 20):
            blob = data[offset:offset + length]
            if len(blob) < 4:
                continue
            try:
                out = dctx.decompress(blob, max_output_size=64 << 20)
            except Exception:
                try:
                    out = dctx.stream_reader(blob).read()
                except Exception:
                    continue
            if not out:
                continue
            hits += 1
            target = os.path.join(out_dir, f"body-{hits}-{offset}.bin")
            open(target, "wb").write(out)
            text = out[:400].decode("utf-8", "replace")
            print(f"--- offset {offset} -> {len(out):,} bytes -> {target}")
            print("    " + text.replace("\n", "\n    ")[:400])
            print()
            break
        else:
            continue
        break

if hits == 0:
    print("no zstd bodies recovered")
    # Fall back: show what the raw bytes look like after the header block.
    i = data.find(b"content-encoding:zstd")
    print(repr(data[i:i + 400]))
