"""Measure the time until a process shows a visible top level window."""

import ctypes
import ctypes.wintypes as w
import subprocess
import sys
import time

user32 = ctypes.windll.user32
EnumWindowsProc = ctypes.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)


def windows_for(pid):
    found = []

    def callback(hwnd, _):
        owner = w.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value != pid:
            return True
        if not user32.IsWindowVisible(hwnd):
            return True
        length = user32.GetWindowTextLengthW(hwnd)
        if length == 0:
            return True
        buffer = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, buffer, length + 1)
        found.append((hwnd, buffer.value))
        return True

    user32.EnumWindows(EnumWindowsProc(callback), 0)
    return found


def main():
    target = sys.argv[1]
    args = sys.argv[2:]

    proc = subprocess.Popen([target] + args)
    start = time.time()
    title = None
    while time.time() - start < 90:
        found = windows_for(proc.pid)
        if found:
            title = found[0][1]
            break
        if proc.poll() is not None:
            print(f"{target}: exited early with {proc.returncode}")
            return
        time.sleep(0.02)

    if title is None:
        print(f"{target}: no visible window within 90s")
    else:
        print(f"{time.time() - start:6.2f}s   {title}")

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


if __name__ == "__main__":
    main()
