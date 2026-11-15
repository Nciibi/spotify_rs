import os
import re
from collections import defaultdict

root = r"C:\Users\rrr\spotify-rs\re\target\xpui"

service_re = re.compile(r"SERVICE_ID\s*=\s*\"([^\"]+)\"")
methods_re = re.compile(r"METHODS\s*=\s*\{", re.M)
method_name_re = re.compile(r"([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*\{[^}]*isStreaming|([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*\{")
simple_method_re = re.compile(r"([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*\{[^}]*\}")

services = defaultdict(set)
service_files = {}

for dirpath, _, filenames in os.walk(root):
    for name in filenames:
        if not name.endswith(".js"):
            continue
        path = os.path.join(dirpath, name)
        try:
            text = open(path, "r", encoding="utf-8", errors="ignore").read()
        except OSError:
            continue

        service_positions = [(m.start(), m.group(1)) for m in service_re.finditer(text)]

        for m in methods_re.finditer(text):
            # Nearest SERVICE_ID before this METHODS block.
            service = "<unknown>"
            for pos, sid in service_positions:
                if pos < m.start():
                    service = sid
                else:
                    break

            # Take the balanced-brace body of the METHODS object.
            depth = 0
            start = m.end() - 1
            end = start
            for i in range(start, min(len(text), start + 8000)):
                if text[i] == "{":
                    depth += 1
                elif text[i] == "}":
                    depth -= 1
                    if depth == 0:
                        end = i
                        break
            body = text[start + 1:end]

            for mm in simple_method_re.finditer(body):
                services[service].add(mm.group(1))
            service_files.setdefault(service, name)

total = 0
print(f"=== esperanto services ({len(services)}) ===\n")
for service in sorted(services):
    methods = sorted(services[service])
    total += len(methods)
    print(f"{service}   [{service_files.get(service, '?')}]  ({len(methods)} methods)")
    for method in methods:
        print(f"    - {method}")
    print()

print(f"TOTAL methods: {total}")
