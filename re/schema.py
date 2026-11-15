"""Reconstruct protobuf message definitions from the ts-proto generated code
in the Spotify desktop UI bundle.

The bundle ships ts-proto output: for every message there is an `encode`
arrow function whose body writes each field with an explicit field number,
e.g.

    void 0!==e.uri && r.uint32(10).string(e.uri)
    void 0!==e.cover && S.encode(e.cover, r.uint32(18).fork()).ldelim()

From that we can recover the .proto definition of every message used by the
esperanto bridge.
"""

import json
import re
import sys
from collections import defaultdict

BUNDLE = r"C:\Users\rrr\spotify-rs\re\target\xpui\7309.js"
OUT = r"C:\Users\rrr\spotify-rs\re\out\esperanto-schema.json"

text = open(BUNDLE, "r", encoding="utf-8", errors="ignore").read()

# --- 1. map variable name -> its encode function body -----------------------
encode_defs = {}
for m in re.finditer(r"([A-Za-z_$][A-Za-z0-9_$]*)=\{encode:\(e,\s*(\w+)(?:=\w+\.Writer\.create\(\))?\)\s*=>", text):
    name = m.group(1)
    body_start = m.end()
    depth = 0
    i = m.start() - 1  # at the '{' of the object literal
    # Walk the object literal to find the encode arrow body end.
    depth = 0
    for j in range(i, min(len(text), i + 40000)):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                obj_end = j
                break
    else:
        continue
    encode_defs.setdefault(name, text[body_start:obj_end])

# --- 2. parse an encode body into fields ------------------------------------
FIELD_RE = re.compile(
    r"\.(?:void 0!==|null!==|0!==|!0===|!1===)\s*"
    r"(?:(?P<obj>e)\.)?(?P<field>[A-Za-z_$][A-Za-z0-9_$]*)"
    r"(?:&&\s*(?P<obj2>e)\.)?"
)
CALL_RE = re.compile(
    r"\.(?P<writer>[A-Za-z_$][A-Za-z0-9_$]*)\.uint32\((?P<tag>\d+)\)"
    r"(?:\.(?P<kind>string|int32|int64|uint32|uint64|bool|bytes|double|float|sint32|sfixed32|sfixed64|fixed32|fixed64))?"
)


def parse_fields(body):
    """Return [(name, number, type, repeated)] for one encode body."""
    fields = {}
    # Walk the body looking for `X.encode(e.Y, W.uint32(N).fork())` (nested)
    # and `W.uint32(N).<kind>(e.Y)` (scalar).
    for m in re.finditer(
        r"(?P<writer>[A-Za-z_$][A-Za-z0-9_$]*)\.uint32\((?P<tag>\d+)\)"
        r"(?P<tail>\.(?P<kind>string|int32|int64|uint32|uint64|bool|bytes|double|float|sint32|sfixed32|sfixed64|fixed32|fixed64)\((?P<arg>[^()]*)\))?",
        body,
    ):
        tag = int(m.group("tag"))
        number = tag >> 3
        kind = m.group("kind")
        # The argument is the field expression, e.g. "e.uri" or "e.tags.length"
        arg = (m.group("arg") or "").strip()
        if not arg:
            # nested message: `.fork()` follows, the field name is in the
            # preceding `void 0!==e.field&&X.encode(e.field,`
            prefix = body[max(0, m.start() - 260):m.start()]
            fm = re.findall(r"(?:void 0!==|null!==)\s*e\.([A-Za-z_$][A-Za-z0-9_$]*)\s*&&", prefix)
            field = fm[-1] if fm else f"field{number}"
            fields[number] = (field, "message", False)
            continue
        field = None
        am = re.search(r"e\.([A-Za-z_$][A-Za-z0-9_$]*)", arg)
        if am:
            field = am.group(1)
        if field is None:
            continue
        repeated = ".length" in arg
        if kind is None:
            kind = "message"
        fields[number] = (field, kind, repeated)
    return sorted((n, *fields[n]) for n in fields)


# --- 3. service classes ------------------------------------------------------
service_re = re.compile(r"static SERVICE_ID=\"([^\"]+)\"")
methods_re = re.compile(r"static METHODS=\{")
decoders_re = re.compile(r"static DECODERS=\{")


def balanced(text, start):
    depth = 0
    for i in range(start, min(len(text), start + 20000)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[start:i + 1], i
    return "", start


services = {}
service_positions = [(m.start(), m.group(1)) for m in service_re.finditer(text)]
for idx, (pos, service_id) in enumerate(service_positions):
    # Bound the search at the next service so blocks are not misattributed.
    end = service_positions[idx + 1][0] if idx + 1 < len(service_positions) else len(text)
    tail = text[pos:end]

    mm = methods_re.search(tail)
    if not mm:
        continue
    methods_block, _ = balanced(tail, mm.end() - 1)

    dm = decoders_re.search(tail)
    decoders_block = ""
    if dm:
        decoders_block, _ = balanced(tail, dm.end() - 1)

    # method -> {streaming, requestVar, responseVar}
    method_vars = {}
    for entry in re.finditer(r"([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*\{([^}]*)\}", methods_block):
        name, body = entry.group(1), entry.group(2)
        streaming = "isStreaming:!0" in body or "isStreaming: true" in body
        req = re.search(r"requestType:([A-Za-z_$][A-Za-z0-9_$]*)", body)
        res = re.search(r"responseType:([A-Za-z_$][A-Za-z0-9_$]*)", body)
        method_vars[name] = {
            "streaming": streaming,
            "requestVar": req.group(1) if req else None,
            "responseVar": res.group(1) if res else None,
        }

    for entry in re.finditer(
        r"([A-Za-z_$][A-Za-z0-9_$]*)\s*:\s*\{\s*request:([A-Za-z_$][A-Za-z0-9_$]*)\.decode,"
        r"\s*response:([A-Za-z_$][A-Za-z0-9_$]*)\.decode", decoders_block
    ):
        method, req, res = entry.group(1), entry.group(2), entry.group(3)
        # DECODERS keys are the PascalCase method names; match them to the
        # camelCase entry in METHODS.
        match = next(
            (k for k in method_vars if k.lower() == method.lower()),
            None,
        )
        if match is None:
            continue
        info = method_vars[match]
        info["requestVar"] = req
        info["responseVar"] = res

    services[service_id] = method_vars

# --- 4. emit ---------------------------------------------------------------
out = {}
for service_id, methods in sorted(services.items()):
    entry = {}
    for method, info in sorted(methods.items()):
        req_fields = []
        res_fields = []
        if info.get("requestVar") in encode_defs:
            req_fields = parse_fields(encode_defs[info["requestVar"]])
        if info.get("responseVar") in encode_defs:
            res_fields = parse_fields(encode_defs[info["responseVar"]])
        entry[method] = {
            "streaming": info.get("streaming", False),
            "request": [
                {"name": f, "number": n, "type": t, "repeated": r} for n, f, t, r in req_fields
            ],
            "response": [
                {"name": f, "number": n, "type": t, "repeated": r} for n, f, t, r in res_fields
            ],
        }
    out[service_id] = entry

with open(OUT, "w", encoding="utf-8") as fh:
    json.dump(out, fh, indent=2)

print(f"services: {len(out)}   methods: {sum(len(v) for v in out.values())}")
print(f"written to {OUT}\n")

for service_id, methods in out.items():
    print(f"{service_id}")
    for method, info in methods.items():
        req = ", ".join(f"{f['name']}:{f['type']}{'[]' if f['repeated'] else ''}={f['number']}" for f in info["request"])
        res = ", ".join(f"{f['name']}:{f['type']}{'[]' if f['repeated'] else ''}={f['number']}" for f in info["response"])
        print(f"  {method}{' (stream)' if info['streaming'] else ''}")
        if req:
            print(f"      req: {req}")
        if res:
            print(f"      res: {res}")
    print()
