import os
import re
from collections import defaultdict

root = r"C:\Users\rrr\spotify-rs\re\target\xpui"

service_re = re.compile(r"SERVICE_ID\s*=\s*\"([^\"]+)\"")
methods_re = re.compile(r"METHODS\s*=\s*\{")

services = defaultdict(set)
service_file = {}
all_service_ids = []
method_only = []

for dirpath, _, filenames in os.walk(root):
    for name in sorted(filenames):
        if not name.endswith(".js"):
            continue
        path = os.path.join(dirpath, name)
        try:
            text = open(path, "r", encoding="utf-8", errors="ignore").read()
        except OSError:
            continue

        ids = [(m.start(), m.group(1)) for m in service_re.finditer(text)]
        for _, sid in ids:
            all_service_ids.append(sid)

        for m in methods_re.finditer(text):
            service = "<unknown>"
            for pos, sid in ids:
                if pos < m.start():
                    service = sid
                else:
                    break
            depth = 0
            start = m.end() - 1
            end = start
            for i in range(start, min(len(text), start + 12000)):
                if text[i] == "{":
                    depth += 1
                elif text[i] == "}":
                    depth -= 1
                    if depth == 0:
                        end = i
                        break
            body = text[start + 1:end]
            for mm in re.finditer(r"([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*\{", body):
                services[service].add(mm.group(1))
            if service == "<unknown>":
                method_only.append((name, body[:200]))

print(f"=== SERVICE_ID literals in the bundle: {len(all_service_ids)} ({len(set(all_service_ids))} distinct) ===\n")
for sid in sorted(set(all_service_ids)):
    methods = sorted(services.get(sid, []))
    print(f"{sid}")
    for method in methods:
        print(f"    {method}")
    if not methods:
        print("    (methods not co-located)")
    print()

print(f"TOTAL services: {len(set(all_service_ids))}   TOTAL methods: {sum(len(v) for v in services.values())}")
if method_only:
    print(f"\n=== {len(method_only)} METHODS blocks without a nearby SERVICE_ID ===")
    for name, snippet in method_only[:10]:
        print(f"  {name}: {snippet}")
