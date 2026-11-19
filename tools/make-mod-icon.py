"""Gera o icon.dds (256x256, BGRA sem compressão) do mod FS25_PXNV99Telemetry.

Uso: python tools/make-mod-icon.py fs25-mod/FS25_PXNV99Telemetry/icon.dds
"""
import math
import struct
import sys

SIZE = 256
BG = (16, 13, 11)          # grafite (BGR)
RING = (255, 155, 31)      # azul elétrico (BGR)
DIM = (82, 74, 64)


def pixel(x, y):
    cx = cy = (SIZE - 1) / 2
    dx, dy = x - cx, y - cy
    r = math.hypot(dx, dy)
    # Aro do volante
    if 78 <= r <= 98:
        return RING
    # Raios horizontais e vertical inferior
    if r < 80 and abs(dy) < 9 and abs(dx) > 26:
        return DIM
    if r < 80 and dy > 26 and abs(dx) < 9:
        return DIM
    # Cubo
    if r < 28:
        return RING if r > 22 else DIM
    # Ondas de vibração nas laterais
    for side in (-1, 1):
        for k, radius in enumerate((108, 118)):
            ring = math.hypot(dx, dy)
            ang = math.degrees(math.atan2(dy, dx * side))
            if abs(ring - radius) < 2.5 and abs(ang) < 28 - k * 6 and dx * side > 0:
                return RING
    return BG


def main(path):
    header = struct.pack(
        "<4sIIIIIII44sIIIIIIIIIIIII",
        b"DDS ", 124, 0x1 | 0x2 | 0x4 | 0x1000 | 0x8, SIZE, SIZE, SIZE * 4, 0, 1, b"\0" * 44,
        32, 0x41, 0, 32, 0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000,
        0x1000, 0, 0, 0, 0,
    )
    body = bytearray()
    for y in range(SIZE):
        for x in range(SIZE):
            b, g, r = pixel(x, y)
            body += bytes((b, g, r, 255))
    with open(path, "wb") as f:
        f.write(header)
        f.write(body)


if __name__ == "__main__":
    main(sys.argv[1])
