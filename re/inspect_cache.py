import re
import sys

path = sys.argv[1]
data = open(path, "rb").read()

i = data.find(b"https://xpui.app.spotify.com")
print(f"url offset {i}\n")

def dump(start, length):
    chunk = data[start:start + length]
    # printable view
    text = "".join(chr(b) if 32 <= b < 127 else "." for b in chunk)
    print(f"--- bytes {start}..{start + length} ---")
    for off in range(0, length, 96):
        piece = chunk[off:off + 96]
        hexs = " ".join(f"{b:02x}" for b in piece[:48])
        print(f"  {start + off:08x}  {hexs}")
        print(f"            {text[off:off + 96]}")
    print()

dump(max(0, i - 160), 512)

print("=== searching the whole file for JS/runtime markers ===")
for marker in (b"__webpack", b"rspack", b"webpack", b"(()=>{", b"Content-Encoding", b"content-encoding", b"br", b"gzip", b"octet-stream", b"text/javascript", b"application/javascript"):
    positions = [m.start() for m in re.finditer(re.escape(marker), data)]
    print(f"  {marker!r}: {len(positions)}  {positions[:8]}")

# Where does the printable content end?
last = 0
for m in re.finditer(rb"[\x20-\x7e]{40,}", data):
    last = m.end()
print(f"\nlast long printable run ends at {last} of {len(data):,}")
print("tail preview:")
print(data[max(0, last - 200):last + 60].decode("utf-8", "replace"))
