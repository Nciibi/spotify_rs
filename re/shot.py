"""Capture a specific window with PrintWindow, so it works even when another
window has focus.
"""

import ctypes
import ctypes.wintypes as w
import sys

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
dwmapi = ctypes.windll.dwmapi

PW_RENDERFULLCONTENT = 0x00000002
SRCCOPY = 0x00CC0020


class RECT(ctypes.Structure):
    _fields_ = [
        ("left", w.LONG),
        ("top", w.LONG),
        ("right", w.LONG),
        ("bottom", w.LONG),
    ]


def capture(hwnd, path):
    rect = RECT()
    dwmapi.DwmGetWindowAttribute(
        w.HWND(hwnd), w.DWORD(9), ctypes.byref(rect), ctypes.sizeof(rect)
    )
    width = rect.right - rect.left
    height = rect.bottom - rect.top
    if width <= 0 or height <= 0:
        raise RuntimeError("the window has no size")

    window_dc = user32.GetWindowDC(w.HWND(hwnd))
    mem_dc = gdi32.CreateCompatibleDC(window_dc)
    bitmap = gdi32.CreateCompatibleBitmap(window_dc, width, height)
    gdi32.SelectObject(mem_dc, bitmap)

    ok = user32.PrintWindow(w.HWND(hwnd), mem_dc, PW_RENDERFULLCONTENT)
    if not ok:
        gdi32.BitBlt(mem_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY)

    class BITMAPINFOHEADER(ctypes.Structure):
        _fields_ = [
            ("biSize", w.DWORD),
            ("biWidth", w.LONG),
            ("biHeight", w.LONG),
            ("biPlanes", w.WORD),
            ("biBitCount", w.WORD),
            ("biCompression", w.DWORD),
            ("biSizeImage", w.DWORD),
            ("biXPelsPerMeter", w.LONG),
            ("biYPelsPerMeter", w.LONG),
            ("biClrUsed", w.DWORD),
            ("biClrImportant", w.DWORD),
        ]

    header = BITMAPINFOHEADER()
    header.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    header.biWidth = width
    header.biHeight = -height
    header.biPlanes = 1
    header.biBitCount = 32
    header.biCompression = 0

    buffer = ctypes.create_string_buffer(width * height * 4)
    gdi32.GetDIBits(mem_dc, bitmap, 0, height, buffer, ctypes.byref(header), 0)

    gdi32.DeleteObject(bitmap)
    gdi32.DeleteDC(mem_dc)
    user32.ReleaseDC(w.HWND(hwnd), window_dc)

    from PIL import Image

    image = Image.frombuffer("RGBA", (width, height), buffer, "raw", "BGRA", 0, 1)
    image.convert("RGB").save(path)
    print(f"saved {path} ({width}x{height})")


if __name__ == "__main__":
    capture(int(sys.argv[1]), sys.argv[2])
