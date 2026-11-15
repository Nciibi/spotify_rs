import re
import sys
from collections import Counter

path = r"C:\Users\rrr\spotify-rs\re\target\Spotify.dll"
data = open(path, "rb").read()
print(f"scanned {len(data):,} bytes")

# ASCII strings of length >= 6.
ascii_strings = set()
for match in re.finditer(rb"[\x20-\x7e]{6,}", data):
    ascii_strings.add(match.group().decode("ascii"))

# UTF-16LE strings (Windows binaries keep a lot of them).
utf16_strings = set()
for match in re.finditer(rb"(?:[\x20-\x7e]\x00){6,}", data):
    utf16_strings.add(match.group().decode("utf-16-le"))

all_strings = ascii_strings | utf16_strings
print(f"unique strings: {len(all_strings):,}")

patterns = {
    "spclient paths": re.compile(r"^/path/v\d+/[A-Za-z0-9_./\-]+$"),
    "https urls": re.compile(r"^https://[A-Za-z0-9._\-]+(?::\d+)?(?:/[A-Za-z0-9._\-/%?=&+]*)?$"),
    "wss urls": re.compile(r"^wss://[A-Za-z0-9._\-]+(?::\d+)?(?:/[A-Za-z0-9._\-/%?=&+]*)?$"),
}

for label, pattern in patterns.items():
    found = sorted(s for s in all_strings if pattern.match(s))
    print(f"\n=== {label}: {len(found)} ===")
    for s in found[:120]:
        print("   ", s)

print("\n=== interesting keywords ===")
keywords = [
    "dealer", "mercury", "spclient", "accesspoint", "ap-", "keymaster",
    "login5", "premium", "product", "open.spotify", "audio-preview",
    "pathfinder", "extensions", "web-api",
]
for keyword in keywords:
    hits = sorted(s for s in all_strings if keyword in s.lower() and 4 < len(s) < 120)
    if hits:
        print(f"\n-- {keyword} ({len(hits)}) --")
        for s in hits[:25]:
            print("   ", s)
