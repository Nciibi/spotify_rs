import os
import re
import sys

root = r"C:\Users\rrr\spotify-rs\re\target\xpui"
module_id = sys.argv[1]
radius = int(sys.argv[2]) if len(sys.argv) > 2 else 1400

# webpack/rspack module definition:  <id>(e,t,s){ ... }
pattern = re.compile(r"(?<![0-9])" + re.escape(module_id) + r"\((e|t|s|_\w+),\s*(\w+),\s*(\w+)\)\s*\{")

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
            start = m.start()
            end = min(len(text), start + radius)
            print(f"--- module {module_id} in {name} @ {start} ---")
            print(re.sub(r"\s+", " ", text[start:end]))
            print()
            sys.exit(0)

print(f"module {module_id} not found")
