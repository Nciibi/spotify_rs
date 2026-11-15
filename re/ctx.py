import re
import sys

path = sys.argv[1]
needles = sys.argv[2].split("|")
window = int(sys.argv[3]) if len(sys.argv) > 3 else 160

text = open(path, "r", encoding="utf-8", errors="ignore").read()
print(f"file: {path}  ({len(text):,} bytes)\n")

seen = set()
for needle in needles:
    pattern = re.compile(re.escape(needle))
    count = 0
    for m in pattern.finditer(text):
        start = max(0, m.start() - window)
        end = min(len(text), m.end() + window)
        chunk = re.sub(r"\s+", " ", text[start:end])
        key = (needle, chunk[:120])
        if key in seen:
            continue
        seen.add(key)
        count += 1
        print(f"--- [{needle}] hit {count} @ {m.start()} ---")
        print(chunk)
        print()
        if count >= 4:
            break
