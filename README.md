# Spar · 微运动提醒器

> 久坐办公，起身一战。

**官网**：<https://xinnix.github.io/spar/>（`site/` 目录，GitHub Pages 部署）

Spar 驻守 macOS / Windows / Linux 菜单栏，在工作时段定时提醒你打一"回合"微运动（俯卧撑、深蹲、提踵……），做完回来记录成绩，本地统计连胜与热力日历。

**为什么是 Spar**：spar = 一场短交锋。每次提醒就是一回合 Round，今日任务 = Round 1/2/3。市面上的提醒器（Time Out/Stiff）靠手动点 Done、计数器（Pushup Social）是移动端无驻留——"菜单栏驻留 + 摄像头验证计数 + 排行榜"是空白。

## 功能路线

| 阶段 | 内容 | 状态 |
|---|---|---|
| **M1** | 菜单栏驻留 · 定时回合提醒（工作时段感知）· 8 动作库轮换 · SVG 火柴人示范动画 · 手动记录成绩 · 连胜 · GitHub 风格热力日历 · 加练 · zh/en · 开机自启 | ✅ |
| **M2** | 摄像头姿态计数（MediaPipe 本地推理）· 回合语音教练 · 实时纠姿 · 可信成绩 verified | 🔜 |
| **M3** | 云端小圈子排行榜（邀请码小组 · 周榜/总榜）· axum + PostgreSQL 自建后端 · 防作弊 | 🔜 |

## 开发

```bash
npm install          # 安装 @tauri-apps/cli
npx tauri dev        # 开发运行（注意：dev 模式 macOS 无系统通知，打包后才生效）
npx tauri build      # 打包
cargo test           # 后端单测（store/reminder 15 个用例）
```

辅助脚本：

```bash
python3 scripts/preview_poses.py            # 动作关键帧火柴人对照图（调姿势角度用）
python3 scripts/gen_app_icon.py             # 重新生成应用图标源图（再跑 npx tauri icon）
python3 scripts/export_site_exercises.py    # 从 store.rs 种子导出官网动作库数据
```

官网本地预览：`cd site && python3 -m http.server 8931` → <http://127.0.0.1:8931>

## 架构

```
src-tauri/src/
├── main.rs          # 托盘 setup + panel 窗口 + 命令 + 配置 + 自启
├── store.rs         # SQLite：运动库（含示范关键帧）/ 回合计划 / 成绩 / 统计
├── reminder.rs      # 回合调度：纯函数时间数学 + mpsc 唤醒线程
├── tray_icon.rs     # 程序化托盘图标（SDF 渲染，无图片资源）
└── macos_notify.rs  # 原生通知 + 点击回调（dev 模式自动跳过）
ui/
├── panel.html/js    # 面板：回合卡（内嵌动画）/ 记录 / 热力日历 / 设置 / i18n
├── skeleton.js      # 骨架渲染器（关键帧 FK + ping-pong 插值；M2 摄像头叠加复用）
└── style.css
```

- **数据**：`~/.config/spar/spar.db`（SQLite，WAL）+ `config.json`（0600）
- **动作关键帧**三端同构：`store.rs` 种子 JSON ↔ `skeleton.js` FK 数学 ↔ `scripts/preview_poses.py` 预览，角度约定见 `store.rs` 头注释
- **轻量策略**：无前端构建、零第三方 JS；MediaPipe（M2）只在运动窗口懒加载
- 架构模式移植自 [gitCIStatus/Glint](../gitCIStatus)（托盘定位双屏 scale、mpsc 唤醒、原生通知 delegate、三平台自启）
