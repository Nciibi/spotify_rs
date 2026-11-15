import re

path = r"C:\Users\rrr\spotify-rs\re\target\xpui\xpui-snapshot.js"
text = open(path, "r", encoding="utf-8", errors="ignore").read()

start = text.find("n.u=e=>")
if start < 0:
    print("could not find the chunk url map")
    raise SystemExit(1)
window = text[start:start + 8000]

mapping = dict(
    (int(k), v) for k, v in re.findall(r'(\d+):"([^"]+)"', window)
)
print(f"named chunks: {len(mapping)}\n")

required = [7207, 2457, 4764, 2207, 9420, 7628, 6026, 6342, 5679, 4406, 1546, 7031, 8399, 846, 1498, 1723]

import os
root = r"C:\Users\rrr\spotify-rs\re\target\xpui"
present = set(os.listdir(root))

print("=== chunks required by the entry ===")
for chunk in required:
    name = mapping.get(chunk, str(chunk))
    file = f"{name}.js"
    print(f"  {chunk:6} -> {file:<48} {'present' if file in present else 'MISSING'}")

print("\n=== all named chunk files referenced by the map, presence ===")
missing = []
for chunk, name in sorted(mapping.items()):
    file = f"{name}.js"
    if file not in present:
        missing.append((chunk, file))
print(f"  mapped chunks: {len(mapping)}   missing files: {len(missing)}")
for chunk, file in missing:
    print(f"    {chunk:6} -> {file}")
