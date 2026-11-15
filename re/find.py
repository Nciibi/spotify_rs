import os
import re
import sys
from collections import Counter

root = r"C:\Users\rrr\spotify-rs\re\target\xpui"
needle = sys.argv[1]
radius = int(sys.argv[2]) if len(sys.argv) > 2 else 260

pattern = re.compile(re.escape(needle))
hits = 0

for dirpath, _, filenames in os.walk(root):
    for name in sorted(filenames):
        if not name.endswith(".js"):
            continue
        path = os.path.join(dirpath, name)
        try:
            text = open(path, "r", encoding="utf-8", errors="ignore").read()
        except OSError:
            continue
        for m in pattern.finditer(text):
            start = max(0, m.start() - radius)
            end = min(len(text), m.end() + radius)
            print(f"--- {name} @ {m.start()} ---")
            print(re.sub(r"\s+", " ", text[start:end]))
            print()
            hits += 1
            if hits >= 12:
                sys.exit(0)
