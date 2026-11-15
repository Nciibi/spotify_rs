"""Recover the API surface the real Spotify desktop client calls, from the
Chromium cache of the installed app.

Cache entries store the request URL followed by the response headers, so a
plain scan yields every URL the client has requested. We normalise the
dynamic parts into templates.
"""

import os
import re
import sys
from collections import Counter, defaultdict

root = sys.argv[1] if len(sys.argv) > 1 else (
    r"C:\Users\rrr\AppData\Local\Packages\SpotifyAB.SpotifyMusic_zpdnekdrzrea0"
    r"\LocalCache\Spotify\Browser\Cache"
)
out = sys.argv[2] if len(sys.argv) > 2 else r"C:\Users\rrr\spotify-rs\re\out\api-surface.txt"

url_re = re.compile(rb"https?://[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]{8,400}")

templates = Counter()
samples = {}
hosts = Counter()

for dirpath, _, filenames in os.walk(root):
    for name in filenames:
        path = os.path.join(dirpath, name)
        data = open(path, "rb").read()
        for m in url_re.finditer(data):
            raw = m.group().decode("ascii", "ignore")
            raw = raw.rstrip("\\\"' )")
            if not raw.startswith("http"):
                continue
            # Drop query strings and fragments for the template.
            base = raw.split("#")[0]
            host = base.split("/")[2]
            hosts[host] += 1
            path_only = base[len("https://" + host):] or "/"
            # Normalise dynamic path segments.
            segs = []
            for seg in path_only.split("/"):
                if re.fullmatch(r"[0-9a-f]{32}", seg):
                    seg = "{hash32}"
                elif re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", seg):
                    seg = "{uuid}"
                elif re.fullmatch(r"\d{6,}", seg):
                    seg = "{num}"
                elif re.fullmatch(r"[0-9a-zA-Z]{22}", seg):
                    seg = "{base62}"
                elif seg.startswith("ab67616d") or seg.startswith("ab6772ab"):
                    seg = "{image_id}"
                segs.append(seg)
            template = host + "/" + "/".join(segs)
            templates[template] += 1
            samples.setdefault(template, raw)

print(f"hosts: {len(hosts)}   endpoint templates: {len(templates)}\n")

with open(out, "w", encoding="utf-8") as fh:
    fh.write("# API surface recovered from the Spotify desktop app's Chromium cache\n")
    fh.write(f"# {len(templates)} endpoint templates across {len(hosts)} hosts\n\n")
    fh.write("## hosts\n\n")
    for host, count in hosts.most_common():
        fh.write(f"{count:5}  {host}\n")
    fh.write("\n## endpoint templates\n\n")
    for template, count in templates.most_common():
        fh.write(f"{count:5}  /{template}\n")
        fh.write(f"        e.g. {samples[template][:150]}\n")

print("top hosts:")
for host, count in hosts.most_common(18):
    print(f"  {count:5}  {host}")

print("\nspotify spclient / api endpoints:")
for template, count in templates.most_common():
    if any(k in template for k in ("spclient", "api.spotify", "dealer", "apresolve", "clienttoken", "login5", "login.")):
        print(f"  {count:4}  {template}")

print(f"\nwritten to {out}")
