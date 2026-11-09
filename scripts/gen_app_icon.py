#!/usr/bin/env python3
"""生成 Spar 应用图标源图（纯 stdlib，无 Pillow）。

设计：深灰圆角方块 + 白色闪电（Feather zap 多边形），与托盘图标同一视觉语言。
渲染 2048 再 2x 降采样抗锯齿，输出 1024 RGBA PNG，供 `tauri icon` 生成全套图标。
"""
import struct
import sys
import zlib

SIZE = 1024          # 输出尺寸
SS = 2               # 超采样倍数
N = SIZE * SS        # 渲染尺寸

BG = (45, 45, 51)    # #2d2d33，与 gitCIStatus 托盘同款深灰
FG = (255, 255, 255)

# 圆角方块（inset 64pt，macOS 风格圆角）
INSET = 64.0
RADIUS = 232.0

# 闪电多边形（Feather zap，24 网格），居中放大
BOLT = [(13, 2), (3, 14), (12, 14), (11, 22), (21, 10), (12, 10)]
BOLT_SCALE = 640.0 / 24.0
BOLT_OFF = (1024 - 640) / 2.0


def bolt_points():
    return [(BOLT_OFF + x * BOLT_SCALE, BOLT_OFF + y * BOLT_SCALE) for x, y in BOLT]


def rounded_rect_dist(px, py):
    """圆角方形的带符号距离（<0 在内）。坐标为 1024 逻辑坐标。"""
    half = (1024 - 2 * INSET) / 2
    cx = cy = 512.0
    r = RADIUS
    qx = abs(px - cx) - (half - r)
    qy = abs(py - cy) - (half - r)
    outside = (max(qx, 0) ** 2 + max(qy, 0) ** 2) ** 0.5
    inside = min(max(qx, qy), 0)
    return outside + inside - r


def in_poly(px, py, poly):
    """射线法点在多边形内判定。"""
    inside = False
    n = len(poly)
    j = n - 1
    for i in range(n):
        xi, yi = poly[i]
        xj, yj = poly[j]
        if (yi > py) != (yj > py):
            xint = (xj - xi) * (py - yi) / (yj - yi) + xi
            if px < xint:
                inside = not inside
        j = i
    return inside


def render():
    bolt = bolt_points()
    rows = []
    step = 1024.0 / N
    for yy in range(N):
        py = (yy + 0.5) * step
        row = bytearray([0])  # filter byte
        for xx in range(N):
            px = (xx + 0.5) * step
            # 超采样中心点的逻辑坐标；用 2x2 四个子样本平均
            r = g = b = a = 0
            for sy in (0.0, 0.5):
                for sx in (0.0, 0.5):
                    qx = px + (sx - 0.25) * step
                    qy = py + (sy - 0.25) * step
                    if rounded_rect_dist(qx, qy) < 0:
                        if in_poly(qx, qy, bolt):
                            c = FG
                        else:
                            c = BG
                        r += c[0]; g += c[1]; b += c[2]; a += 255
            row += bytes((r // 4, g // 4, b // 4, a // 4))
        rows.append(bytes(row))
    return b"".join(rows)


def write_png(path, w, h, raw):
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", ihdr)
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def downsample2x(raw, w, h):
    """2x 盒式降采样（SS=2 时输出 SIZE 尺寸）。"""
    out = bytearray()
    sw = w * 4
    for y in range(0, h, 2):
        r0 = raw[y * sw:(y + 1) * sw]
        r1 = raw[(y + 1) * sw:(y + 2) * sw]
        row = bytearray([0])
        for x in range(0, sw, 8):
            for ch in range(4):
                v = (r0[x + ch] + r0[x + 4 + ch] + r1[x + ch] + r1[x + 4 + ch]) // 4
                row.append(v)
        out += row
    return bytes(out)


if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "app-icon-source.png"
    raw = render()
    small = downsample2x(raw, N, N)
    write_png(path, SIZE, SIZE, small)
    print(f"wrote {path} ({SIZE}x{SIZE})")
