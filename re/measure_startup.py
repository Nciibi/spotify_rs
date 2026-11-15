"""Measure time-to-window for our host and, for comparison, the installed
Spotify client.
"""

import subprocess
import time
import sys
import os


def window_ready(pid, timeout=60):
    """Return the seconds until the process owns a visible main window."""
    import ctypes
    from ctypes import wintypes

    user32 = ctypes.windll.user32
    start = time.time()
    while time.time() - start < timeout:
        try:
            import psutil  # type: ignore
        except ImportError:
            psutil = None
        if psutil is not None:
            try:
                proc = psutil.Process(pid)
                for win in proc.windows():
                    if win.title():
                        return time.time() - start, win.title()
            except Exception:
                pass
        time.sleep(0.05)
    return None, None


def main():
    target = sys.argv[1]
    args = sys.argv[2:]
    proc = subprocess.Popen([target] + args)
    elapsed, title = window_ready(proc.pid)
    if elapsed is None:
        print(f"{os.path.basename(target)}: no window within the timeout")
    else:
        print(f"{os.path.basename(target)}: window after {elapsed:.2f}s  '{title}'")
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


if __name__ == "__main__":
    main()
