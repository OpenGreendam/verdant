"""Capture an X11 input display without a screenshot-tool dependency.

For nested Wayland tests this records the visible Weston X11 host window and
its surrounding input display. It does not connect to another user's display.
"""

import ctypes as c
import ctypes.util
from pathlib import Path
import struct
import zlib


class XImage(c.Structure):
    _fields_ = [("width", c.c_int), ("height", c.c_int), ("xoffset", c.c_int),
                ("format", c.c_int), ("data", c.c_void_p), ("byte_order", c.c_int),
                ("bitmap_unit", c.c_int), ("bitmap_bit_order", c.c_int),
                ("bitmap_pad", c.c_int), ("depth", c.c_int),
                ("bytes_per_line", c.c_int), ("bits_per_pixel", c.c_int),
                ("red_mask", c.c_ulong), ("green_mask", c.c_ulong),
                ("blue_mask", c.c_ulong)]


def capture(display_name: str, destination: Path):
    library = ctypes.util.find_library("X11")
    if not library:
        raise RuntimeError("libX11 is required for the private display screenshot")
    xlib = c.CDLL(library)
    xlib.XOpenDisplay.argtypes = [c.c_char_p]
    xlib.XOpenDisplay.restype = c.c_void_p
    xlib.XCloseDisplay.argtypes = [c.c_void_p]
    xlib.XDefaultScreen.argtypes = [c.c_void_p]
    xlib.XDefaultScreen.restype = c.c_int
    xlib.XRootWindow.argtypes = [c.c_void_p, c.c_int]
    xlib.XRootWindow.restype = c.c_ulong
    for name in ("XDisplayWidth", "XDisplayHeight"):
        function = getattr(xlib, name)
        function.argtypes = [c.c_void_p, c.c_int]
        function.restype = c.c_int
    xlib.XGetImage.argtypes = [c.c_void_p, c.c_ulong, c.c_int, c.c_int,
                              c.c_uint, c.c_uint, c.c_ulong, c.c_int]
    xlib.XGetImage.restype = c.POINTER(XImage)
    xlib.XDestroyImage.argtypes = [c.POINTER(XImage)]
    display = xlib.XOpenDisplay(display_name.encode())
    if not display:
        raise RuntimeError(f"Cannot open explicit input display {display_name}")
    image = None
    try:
        screen = xlib.XDefaultScreen(display)
        width, height = xlib.XDisplayWidth(display, screen), xlib.XDisplayHeight(display, screen)
        image = xlib.XGetImage(display, xlib.XRootWindow(display, screen), 0, 0,
                              width, height, c.c_ulong(-1).value, 2)  # ZPixmap
        if not image:
            raise RuntimeError("XGetImage failed on the private input display")
        source = image.contents
        if (source.red_mask, source.green_mask, source.blue_mask) != (0xFF0000, 0xFF00, 0xFF):
            raise RuntimeError("Screenshot requires the normal 24-bit RGB X11 visual")
        if source.bits_per_pixel not in (24, 32):
            raise RuntimeError(f"Unsupported screenshot pixel width {source.bits_per_pixel}")
        stride = source.bits_per_pixel // 8
        data = c.string_at(source.data, source.bytes_per_line * height)
        rgb = bytearray()
        channels = (2, 1, 0) if source.byte_order == 0 else (stride - 3, stride - 2, stride - 1)
        for y in range(height):
            row = data[y * source.bytes_per_line:y * source.bytes_per_line + width * stride]
            converted = bytearray(width * 3)
            for output_channel, input_channel in enumerate(channels):
                converted[output_channel::3] = row[input_channel::stride]
            rgb.append(0)  # PNG scanline filter: none.
            rgb.extend(converted)

        def chunk(kind, content):
            return (struct.pack(">I", len(content)) + kind + content
                    + struct.pack(">I", zlib.crc32(kind + content)))

        destination.write_bytes(b"\x89PNG\r\n\x1a\n"
                                + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
                                + chunk(b"IDAT", zlib.compress(rgb, 3)) + chunk(b"IEND", b""))
        return {"display": display_name, "width": width, "height": height,
                "path": str(destination)}
    finally:
        if image:
            xlib.XDestroyImage(image)
        xlib.XCloseDisplay(display)
