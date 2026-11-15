import os
import re
import sys

root = sys.argv[1] if len(sys.argv) > 1 else (
    r"C:\Users\rrr\AppData\Local\Packages\SpotifyAB.SpotifyMusic_zpdnekdrzrea0"
    r"\LocalCache\Spotify\Browser\Cache"
)

url_re = re.compile(rb"https?://[\x21-\x7e]{6,300}")
markers = [b"__webpack_modules__", b"rspackChunk", b"webpackChunk", b"7207"]

found_urls = {}
for dirpath, _, filenames in os.walk(root):
    for name in filenames:
        path = os.path.join(dirpath, name)
        data = open(path, "rb").read()
        hits = [m.decode() for m in markers if m in data]
        urls = [u.decode("ascii", "ignore") for u in url_re.findall(data)]
        interesting = [u for u in urls if u.endswith(".js") or ".js?" in u or "xpui" in u]
        if hits or interesting:
            print(f"--- {path}  ({len(data):,} bytes)  markers={hits}")
            for u in sorted(set(interesting))[:12]:
                print(f"      {u}")
            print()

print("=== done ===")
