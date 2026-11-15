import os
import re
from collections import Counter

dll_path = r"C:\Users\rrr\spotify-rs\re\target\Spotify.dll"
js_root = r"C:\Users\rrr\spotify-rs\re\target\xpui"

# --- strings from the native binary -----------------------------------------
data = open(dll_path, "rb").read()
dll_strings = set()
for m in re.finditer(rb"[\x20-\x7e]{4,}", data):
    dll_strings.add(m.group().decode("ascii", "ignore"))
for m in re.finditer(rb"(?:[\x20-\x7e]\x00){4,}", data):
    dll_strings.add(m.group().decode("utf-16-le", "ignore"))

# --- string literals from the javascript bundle ------------------------------
js_literal = re.compile(r"[\"'`]([A-Za-z_$][A-Za-z0-9_$.\-]{3,60})[\"'`]")
js_strings = Counter()
js_where = {}
for dirpath, _, filenames in os.walk(js_root):
    for name in filenames:
        if not name.endswith(".js"):
            continue
        path = os.path.join(dirpath, name)
        try:
            text = open(path, "r", encoding="utf-8", errors="ignore").read()
        except OSError:
            continue
        for m in js_literal.finditer(text):
            token = m.group(1)
            js_strings[token] += 1
            js_where.setdefault(token, name)

common = {s for s in js_strings if s in dll_strings}

print(f"js literals: {len(js_strings):,}   dll strings: {len(dll_strings):,}")
print(f"intersection: {len(common):,}\n")

# Keep the ones that look like an API surface rather than English prose.
def interesting(token: str) -> bool:
    if not re.fullmatch(r"[A-Za-z_$][A-Za-z0-9_$]*", token):
        return False
    if token in {"string", "number", "boolean", "object", "function", "undefined", "prototype"}:
        return False
    # camelCase or PascalCase identifiers
    if not re.search(r"[a-z][A-Z]|[A-Z][a-z]", token):
        return False
    return True

ranked = sorted(
    (s for s in common if interesting(s)),
    key=lambda s: -js_strings[s],
)

print(f"=== identifier-shaped strings in both the native core and the UI ({len(ranked)}) ===\n")
for token in ranked[:220]:
    print(f"{js_strings[token]:6}  {token:<44} {js_where[token]}")
