import json
import sys


def load_jsonl_or_json(path):
    """REA writes progress lines then a final result document."""
    with open(path, "r", encoding="utf-8") as fh:
        raw = fh.read()
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        pass
    # Fall back to the last complete JSON document in the stream.
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


def walk(node, prefix="", depth=0, maxdepth=3, out=None):
    if out is None:
        out = []
    if depth > maxdepth:
        return out
    if isinstance(node, dict):
        for k, v in node.items():
            path = f"{prefix}.{k}" if prefix else k
            if isinstance(v, (dict, list)):
                out.append((path, type(v).__name__, len(v)))
                walk(v, path, depth + 1, maxdepth, out)
            else:
                out.append((path, type(v).__name__, repr(v)[:120]))
    elif isinstance(node, list):
        if node and isinstance(node[0], (dict, list)):
            walk(node[0], prefix + "[0]", depth + 1, maxdepth, out)
        else:
            out.append((prefix, "list-items", repr(node[:3])[:160]))
    return out


path = sys.argv[1]
data = load_jsonl_or_json(path)
if data is None:
    print("could not parse", path)
    raise SystemExit(1)

print("=" * 70)
print("TOP LEVEL KEYS:", list(data.keys())[:40])
for row in walk(data, maxdepth=int(sys.argv[2]) if len(sys.argv) > 2 else 2):
    print("  ", row[0], "|", row[1], "|", row[2])
