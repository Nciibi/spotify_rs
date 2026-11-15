import os
import re
from collections import Counter, defaultdict

root = r"C:\Users\rrr\spotify-rs\re\target\xpui"

# Property bases that look like they could be a host-injected global.
candidates = [
    "native", "host", "desktop", "bridge", "player", "spotify", "cef",
    "client", "api", "core", "ipc", "shell", "app", "win", "chrome",
    "external", "os", "sys", "runtime", "platform", "lib", "sdk", "backend",
]
pattern = re.compile(r"(?<![\w$.])(" + "|".join(candidates) + r")\.([A-Za-z_$][A-Za-z0-9_$]*)")

hits = Counter()
examples = {}

for dirpath, _, filenames in os.walk(root):
    for name in filenames:
        if not name.endswith(".js"):
            continue
        path = os.path.join(dirpath, name)
        try:
            text = open(path, "r", encoding="utf-8", errors="ignore").read()
        except OSError:
            continue
        for match in pattern.finditer(text):
            key = f"{match.group(1)}.{match.group(2)}"
            hits[key] += 1
            if key not in examples:
                examples[key] = (name, re.sub(r"\s+", " ", text[max(0, match.start() - 90):match.start() + 110]))

print(f"distinct candidate base.method pairs: {len(hits)}\n")
for key, count in hits.most_common(70):
    name, ctx = examples[key]
    print(f"{count:5}  {key:<40} {name:<32} ...{ctx}...")
