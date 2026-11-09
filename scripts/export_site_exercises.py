#!/usr/bin/env python3
"""从 store.rs 的 SEED_JSON 导出 site/exercises.js（官网动作库演示数据）。
单一数据源：改了种子关键帧后重跑本脚本，官网动画同步更新。
"""
import json
import re

SRC = "src-tauri/src/store.rs"
DST = "site/exercises.js"

src = open(SRC, encoding="utf-8").read()
m = re.search(r'SEED_JSON: &str = r#"(.*?)"#;', src, re.S)
seed = json.loads(m.group(1))

items = []
for ex in seed:
    items.append({
        "id": ex["id"],
        "nameZh": ex["name_zh"],
        "nameEn": ex["name_en"],
        "unit": ex["unit"],
        "goal": ex["default_goal"],
        "cuesZh": ex["cues_zh"],
        "cuesEn": ex["cues_en"],
        "keyframes": ex["keyframes"],
    })

js = (
    "// 由 scripts/export_site_exercises.py 从 src-tauri/src/store.rs SEED_JSON 生成——勿手改\n"
    "// （改动作/关键帧请改种子后重跑脚本，与应用内示范保持同一份数据）\n"
    "const SPAR_EXERCISES = " + json.dumps(items, ensure_ascii=False, indent=2) + ";\n"
)
open(DST, "w", encoding="utf-8").write(js)
print(f"wrote {DST} ({len(items)} exercises)")
