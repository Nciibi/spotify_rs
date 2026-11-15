import json
import sys
from collections import Counter


def load_jsonl_or_json(path):
    with open(path, "r", encoding="utf-8") as fh:
        raw = fh.read()
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        pass
    decoder = json.JSONDecoder()
    best = None
    index = 0
    length = len(raw)
    while index < length:
        while index < length and raw[index] in " \r\n\t":
            index += 1
        if index >= length:
            break
        try:
            obj, end = decoder.raw_decode(raw, index)
        except json.JSONDecodeError:
            break
        best = obj
        index = end
    return best


data = load_jsonl_or_json(sys.argv[1])
nr = data["normalized_result"]

print("### SUMMARY ###")
for key, value in nr["summary"].items():
    if isinstance(value, list):
        print(f"  {key}: list[{len(value)}]")
    else:
        print(f"  {key}: {repr(value)[:200]}")

print("\n### STATISTICS ###")
for key, value in nr["statistics"].items():
    print(f"  {key}: {repr(value)[:120]}")

print("\n### ROUTE INVENTORY ###")
routes = nr["route_inventory"]
print("  statistics:", json.dumps(routes.get("statistics", {}))[:500])
for route in routes.get("routes", [])[:80]:
    if isinstance(route, dict):
        label = route.get("label") or route.get("path") or route.get("route") or route.get("id")
        src = route.get("source") or route.get("source_file") or ""
        print(f"   - {label}   [{str(src)[:70]}]")
    else:
        print(f"   - {route}")
