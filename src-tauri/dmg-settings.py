# CI/本地无头产 DMG：纯 Python 写 .DS_Store（背景图 + 图标坐标），不依赖 AppleScript/Finder
# （bundler 的 DMG 视觉靠 osascript 驱动 Finder，CI runner 上静默失败——背景图进了
#  .background/ 但视图设置缺失，表现为「背景图不显示」）
#
# 视觉参数与 tauri.conf.json 的 bundle.macOS.dmg 保持一致（后者服务本地 bundler 构建）。
# ⚠ dmgbuild（glint 项目 1.6.x 实测）icon_locations 的 y 是**左上原点**，与文档「左下原点」
#   不符——直接用与背景图一致的左上坐标：图标中心 y=180，正对箭头线。
# 路径由调用方以 -D 注入：-Dapp=<Spar.app 路径> -Dbackground=<背景图路径>
# 背景图内容见 scripts/dmg-background.html（渲染步骤在该文件头注释）。
# ⚠ window_rect 的第二个元组是**宽高**（core.py 直接写 Finder WindowBounds
#   {{x,y},{w,h}}）——按 (right, top) 理解会把窗口开错尺寸（glint v1.2.2 实证翻车）。
# 窗口 660×712：背景画板同尺寸，正文内容只占顶部 620，底部 ~92px 纯背景色安全区
# ——macOS 26 Finder 对 DMG 卷强制加标签栏+状态栏，不加安全区则底部说明文字被遮住。

files = [defines["app"]]
background = defines["background"]
window_rect = ((100, 100), (660, 712))
icon_size = 90
icon_locations = {
    "Spar.app": (180, 180),
    "Applications": (480, 180),
}
symlinks = { "Applications": "/Applications" }
format = "UDZO"
