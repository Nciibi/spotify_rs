"""Serve the extracted xpui.spa over HTTP, optionally injecting a bootstrap
script before the entry chunk.

The real app's CEF host injects the webpack runtime (`__webpack_modules__`)
and the Esperanto transport module; this server lets us experiment with what
exactly has to be injected.
"""

import argparse
import http.server
import json
import mimetypes
import os
import socketserver

ROOT = r"C:\Users\rrr\spotify-rs\re\target\xpui"

EXTRA_TYPES = {
    ".js": "text/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".woff2": "font/woff2",
    ".woff": "font/woff",
    ".ttf": "font/ttf",
    ".json": "application/json; charset=utf-8",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".svg": "image/svg+xml",
    ".mp3": "audio/mpeg",
    ".m4a": "audio/mp4",
    ".webp": "image/webp",
}


class Handler(http.server.SimpleHTTPRequestHandler):
    bootstrap = ""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=ROOT, **kwargs)

    def guess_type(self, path):
        ext = os.path.splitext(path)[1].lower()
        if ext in EXTRA_TYPES:
            return EXTRA_TYPES[ext]
        return super().guess_type(path)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def do_GET(self):
        path = self.path.split("?", 1)[0]
        if path in ("/", "/index.html") and self.bootstrap:
            body = open(os.path.join(ROOT, "index.html"), "rb").read().decode("utf-8")
            inject = f"<script>{self.bootstrap}</script>"
            body = body.replace('<script defer="defer" src="/xpui-snapshot.js"></script>',
                                inject + '<script defer="defer" src="/xpui-snapshot.js"></script>')
            data = body.encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
            return
        super().do_GET()

    def log_message(self, fmt, *args):
        pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=8899)
    parser.add_argument("--bootstrap", default="")
    args = parser.parse_args()

    Handler.bootstrap = args.bootstrap
    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.ThreadingTCPServer(("127.0.0.1", args.port), Handler) as httpd:
        print(f"serving {ROOT} on http://127.0.0.1:{args.port}", flush=True)
        httpd.serve_forever()


if __name__ == "__main__":
    main()
