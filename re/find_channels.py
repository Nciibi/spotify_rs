import os
import re
import sys
from collections import Counter, defaultdict

root = r"C:\Users\rrr\spotify-rs\re\target\xpui"

# Message channels are string literals that start with '@' (Spotify's bridge
# namespace) plus anything that looks like a bridge command.
literal = re.compile(r"[\"'](@[A-Za-z0-9_.:\-]+)[\"']")
bridge_word = re.compile(r"[\"']([a-z0-9_\-]*bridge[a-z0-9_\-]*)[\"']", re.I)

channels = Counter()
where = defaultdict(set)
bridges = Counter()

for dirpath, _, filenames in os.walk(root):
    for name in filenames:
        if not name.endswith(".js"):
            continue
        path = os.path.join(dirpath, name)
        try:
            text = open(path, "r", encoding="utf-8", errors="ignore").read()
        except OSError:
            continue
        for match in literal.finditer(text):
            token = match.group(1)
            channels[token] += 1
            where[token].add(name)
        for match in bridge_word.finditer(text):
            bridges[match.group(1)] += 1

print(f"=== '@' message channels ({len(channels)} distinct) ===")
for token, count in channels.most_common(120):
    files = ", ".join(sorted(where[token])[:3])
    print(f"{count:5}  {token:<52} {files}")

print(f"\n=== bridge-named literals ({len(bridges)}) ===")
for token, count in bridges.most_common(60):
    print(f"{count:5}  {token}")
