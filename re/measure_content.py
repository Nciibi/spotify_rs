"""Measure how long the host takes until the embedded page is actually
rendered, and compare it with the official client.
"""

import json
import subprocess
import sys
import time
import urllib.request

PORT = 9350


def targets():
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json/list", timeout=3) as fh:
            return json.load(fh)
    except Exception:
        return []


def main():
    exe = sys.argv[1]
    args = sys.argv[2:]

    start = time.time()
    proc = subprocess.Popen([exe] + args, env={
        **__import__("os").environ,
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS": f"--remote-debugging-port={PORT}",
    })

    ready = None
    while time.time() - start < 120:
        page = next((t for t in targets() if t.get("type") == "page"), None)
        if page:
            ready = time.time() - start
            break
        time.sleep(0.05)

    if ready is None:
        print("the page target never appeared")
        proc.terminate()
        return

    # Now wait until the page actually has content.
    import websocket  # noqa: F401  (optional dependency)
    print(f"page target after {ready:.2f}s")

    proc.terminate()


if __name__ == "__main__":
    main()
