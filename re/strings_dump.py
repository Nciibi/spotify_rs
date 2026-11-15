import re
from collections import Counter

path = r"C:\Users\rrr\spotify-rs\re\target\Spotify.dll"
out = r"C:\Users\rrr\spotify-rs\re\out\spotify-strings.txt"
data = open(path, "rb").read()

ascii_strings = {m.group().decode("ascii") for m in re.finditer(rb"[\x20-\x7e]{5,}", data)}
utf16_strings = {m.group().decode("utf-16-le") for m in re.finditer(rb"(?:[\x20-\x7e]\x00){5,}", data)}
all_strings = sorted(ascii_strings | utf16_strings)

groups = {
    "URLS": re.compile(r"^(https?|wss?)://"),
    "SPCLIENT": re.compile(r"spclient|dealer|mercury|apresolve|login5|clienttoken|accesspoint"),
    "PATHS": re.compile(r"^/path/v\d+/"),
    "PROTOBUF": re.compile(r"^(com|spotify)\.[a-z0-9_.]+$"),
    "FEATURES": re.compile(r"(?i)feature|experiment|flag|treatment|variant"),
    "PRODUCT": re.compile(r"(?i)premium|free|tier|subscription|product"),
    "AUDIO": re.compile(r"(?i)audio|codec|opus|aac|mp3|bitrate|resampl|equaliz|normalis"),
    "PODCAST": re.compile(r"(?i)podcast|episode|show|rss"),
}

with open(out, "w", encoding="utf-8") as fh:
    fh.write(f"# {len(all_strings):,} unique strings from Spotify.dll\n\n")
    for label, pattern in groups.items():
        hits = [s for s in all_strings if pattern.search(s)]
        fh.write(f"\n{'=' * 70}\n{label}  ({len(hits)} hits)\n{'=' * 70}\n")
        for s in hits:
            fh.write(s + "\n")

    fh.write(f"\n{'=' * 70}\nALL STRINGS\n{'=' * 70}\n")
    for s in all_strings:
        fh.write(s + "\n")

print(f"wrote {out}")
for label, pattern in groups.items():
    print(f"  {label}: {sum(1 for s in all_strings if pattern.search(s))}")
