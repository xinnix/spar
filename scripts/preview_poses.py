#!/usr/bin/env python3
"""姿态关键帧预览：从 store.rs 提取种子 JSON，用与 ui/skeleton.js 一致的 FK 数学
渲染每个动作两个关键帧的火柴人，拼成对照图目检（关节角度是否正确）。
"""
import json
import math
import re
import struct
import sys
import zlib

STORE_RS = "src-tauri/src/store.rs"
OUT = "poses-preview.png"

# 与 skeleton.js 一致的肢体长度（渲染单位）
TORSO, UPPER, FORE = 46.0, 22.0, 20.0
THIGH, SHIN = 26.0, 26.0
HEAD_R = 8.0


def D(deg):
    a = math.radians(deg)
    return (math.sin(a), math.cos(a))  # 0=向下, +90=向前(右), -90=向后, 180=向上


def joints(pose, anchor):
    torso, arm, elbow, hip, knee, lift = pose
    torso_dir = D(180 - torso)
    if anchor == "feet":
        foot = (0.0, 0.0)
        hip_pt = (
            foot[0] - (THIGH * D(hip)[0] + SHIN * D(hip - knee)[0]),
            foot[1] - (THIGH * D(hip)[1] + SHIN * D(hip - knee)[1]) + lift,
        )
        neck = (hip_pt[0] + TORSO * torso_dir[0], hip_pt[1] + TORSO * torso_dir[1])
        shoulder = neck
    else:  # hand
        hand = (0.0, 0.0)
        fa = D(arm + elbow)
        ua = D(arm)
        shoulder = (
            hand[0] - (FORE * fa[0] + UPPER * ua[0]),
            hand[1] - (FORE * fa[1] + UPPER * ua[1]),
        )
        neck = shoulder
        hip_pt = (neck[0] - TORSO * torso_dir[0], neck[1] - TORSO * torso_dir[1])
    elbow_pt = (shoulder[0] + UPPER * D(arm)[0], shoulder[1] + UPPER * D(arm)[1])
    hand_pt = (elbow_pt[0] + FORE * D(arm + elbow)[0], elbow_pt[1] + FORE * D(arm + elbow)[1])
    knee_pt = (hip_pt[0] + THIGH * D(hip)[0], hip_pt[1] + THIGH * D(hip)[1])
    foot_pt = (knee_pt[0] + SHIN * D(hip - knee)[0], knee_pt[1] + SHIN * D(hip - knee)[1])
    head = (neck[0] + torso_dir[0] * (HEAD_R + 4), neck[1] + torso_dir[1] * (HEAD_R + 4))
    return dict(hip=hip_pt, neck=neck, head=head, elbow=elbow_pt, hand=hand_pt,
                knee=knee_pt, foot=foot_pt)


def load_seed():
    src = open(STORE_RS, encoding="utf-8").read()
    m = re.search(r'SEED_JSON: &str = r#"(.*?)"#;', src, re.S)
    return json.loads(m.group(1))


# ============ 纯 stdlib 光栅化（线段/圆/文字用色块近似） ============

def set_px(img, w, h, px, py, color):
    if 0 <= px < w and 0 <= py < h:
        idx = (py * w + px) * 4
        img[idx:idx + 3] = bytes(color)
        img[idx + 3] = 255


def draw_line(img, w, h, x0, y0, x1, y1, color, thick):
    steps = int(max(abs(x1 - x0), abs(y1 - y0)) * 2) + 1
    r = thick / 2
    for i in range(steps + 1):
        t = i / steps
        cx, cy = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
        for dy in range(-int(r), int(r) + 1):
            for dx in range(-int(r), int(r) + 1):
                if dx * dx + dy * dy <= r * r:
                    set_px(img, w, h, int(cx + dx), int(cy + dy), color)


def draw_disc(img, w, h, cx, cy, r, color, fill=True):
    for dy in range(-int(r), int(r) + 1):
        for dx in range(-int(r), int(r) + 1):
            d2 = dx * dx + dy * dy
            if fill and d2 <= r * r or (not fill and abs(math.sqrt(d2) - r) <= 1.5):
                set_px(img, w, h, int(cx + dx), int(cy + dy), color)


def write_png(path, w, h, raw):
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    open(path, "wb").write(png)


def main():
    seed = load_seed()
    cell_w, cell_h = 150, 170
    cols = 4
    rows = math.ceil(len(seed) * 2 / cols)
    W, H = cell_w * cols, cell_h * rows
    img = bytearray(W * H * 4)
    fg = (20, 20, 24)
    muted = (150, 150, 158)

    for ei, ex in enumerate(seed):
        anchor = ex["keyframes"]["anchor"]
        for fi, frame in enumerate(ex["keyframes"]["frames"]):
            j = joints(frame["p"], anchor)
            cell = ei * 2 + fi
            ox = (cell % cols) * cell_w + cell_w // 2
            oy = (cell // cols) * cell_h + 120  # 锚点（脚/手）放这里，地面线在其上

            def P(pt):
                return (ox + pt[0], oy + pt[1])

            # 地面线
            draw_line(img, W, H, ox - 55, oy, ox + 55, oy, muted, 1)
            # 躯干/四肢
            draw_line(img, W, H, *P(j["hip"]), *P(j["neck"]), fg, 3.5)
            draw_line(img, W, H, *P(j["neck"]), *P(j["elbow"]), fg, 3)
            draw_line(img, W, H, *P(j["elbow"]), *P(j["hand"]), fg, 3)
            draw_line(img, W, H, *P(j["hip"]), *P(j["knee"]), fg, 3)
            draw_line(img, W, H, *P(j["knee"]), *P(j["foot"]), fg, 3)
            draw_disc(img, W, H, P(j["head"])[0], P(j["head"])[1], HEAD_R, fg)
            # 锚点标记（绿=feet 蓝=hand）
            ac = (40, 160, 80) if anchor == "feet" else (60, 110, 220)
            draw_disc(img, W, H, ox, oy, 3, ac)

            print(f"{ex['id']:14s} frame{fi}  torso={frame['p'][0]:6.1f} arm={frame['p'][1]:6.1f} "
                  f"elbow={frame['p'][2]:6.1f} hip={frame['p'][3]:6.1f} knee={frame['p'][4]:6.1f}")

    # PNG 每行需 filter 字节前缀
    rows = b"".join(bytes([0]) + bytes(img[y * W * 4:(y + 1) * W * 4]) for y in range(H))
    write_png(OUT, W, H, rows)
    print(f"\nwrote {OUT} ({W}x{H})")


if __name__ == "__main__":
    main()
