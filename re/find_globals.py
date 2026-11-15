import os
import re
import sys
from collections import Counter

root = r"C:\Users\rrr\spotify-rs\re\target\xpui"
pattern = re.compile(sys.argv[1] if len(sys.argv) > 1 else r"window\.[A-Za-z_$][A-Za-z0-9_$]*")

hits = Counter()
files = {}
for dirpath, _, filenames in os.walk(root):
    for name in filenames:
        if not name.endswith(".js"):
            continue
        path = os.path.join(dirpath, name)
        try:
            with open(path, "r", encoding="utf-8", errors="ignore") as fh:
                text = fh.read()
        except OSError:
            continue
        for match in pattern.finditer(text):
            token = match.group(0)
            hits[token] += 1
            if token not in files:
                files[token] = (name, text[max(0, match.start() - 60):match.start() + 120])

print(f"distinct matches: {len(hits)}\n")
for token, count in hits.most_common(80):
    name, ctx = files[token]
    ctx = re.sub(r"\s+", " ", ctx)
    print(f"{count:6}  {token:<38} {name:<34} ...{ctx}...")
