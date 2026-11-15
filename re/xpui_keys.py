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


path = sys.argv[1]
data = load_jsonl_or_json(path)
print("TOP KEYS:", list(data.keys()))

nr = data.get("normalized_result") or {}
print("\nnormalized_result:")
for key, value in nr.items():
    if isinstance(value, list):
        print(f"  {key}: list[{len(value)}]")
    elif isinstance(value, dict):
        print(f"  {key}: dict[{len(value)}] keys={list(value.keys())[:12]}")
    else:
        print(f"  {key}: {repr(value)[:140]}")
