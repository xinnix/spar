#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Spar · 微运动提醒器：菜单栏驻守，工作时段定时提醒"一回合"微运动，
// 完成后记录成绩。M1 = 提醒 + 记录 + 示范动画 + 本地统计。
// 架构沿用 gitCIStatus（Glint）验证过的模式：托盘 + 隐藏 panel +
// mpsc 唤醒调度线程 + JSON 配置 + 程序化托盘图标。

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Mutex, OnceLock};
use tauri::menu::{IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "macos")]
use tauri::RunEvent;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
mod macos_notify;
mod reminder;
mod store;
mod tray_icon;

use store::{Store, DaySummary, Exercise, HeatCell, Plan, Session};

// ============ 配置 ============

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub interval_minutes: u64,
    pub work_start: String, // HH:MM
    pub work_end: String,   // HH:MM
    pub rounds_per_day: usize,
    pub sound: bool,
    pub lang: String, // zh | en
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_minutes: 60,
            work_start: "09:00".into(),
            work_end: "19:00".into(),
            rounds_per_day: 3,
            sound: true,
            lang: "zh".into(),
        }
    }
}

/// 配置目录：unix 用 ~/.config/spar；Windows 用 %APPDATA%\spar
#[cfg(windows)]
fn config_dir_base() -> std::path::PathBuf {
    dirs::config_dir()
        .map(|d| d.join("spar"))
        .unwrap_or_else(|| std::path::PathBuf::from("spar"))
}

#[cfg(not(windows))]
fn config_dir_base() -> std::path::PathBuf {
    let mut p = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push(".config/spar");
    p
}

fn config_path() -> std::path::PathBuf {
    let mut p = config_dir_base();
    let _ = std::fs::create_dir_all(&p);
    p.push("config.json");
    p
}

fn db_path() -> std::path::PathBuf {
    let mut p = config_dir_base();
    let _ = std::fs::create_dir_all(&p);
    p.push("spar.db");
    p
}

pub fn load_config() -> Config {
    std::fs::read_to_string(config_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_config_file(config: &Config) -> Result<(), String> {
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(config_path(), json).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(config_path(), std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

// ============ 共享状态 ============

pub struct Shared {
    pub store: Store,
    pub wake: Mutex<mpsc::Sender<()>>,
    pub last_fire: Mutex<Option<chrono::DateTime<Local>>>,
    pub snooze_until: Mutex<Option<chrono::DateTime<Local>>>,
}

fn wake_up(app: &AppHandle) {
    if let Some(s) = app.try_state::<Shared>() {
        let _ = s.wake.lock().unwrap().send(());
    }
}

// ============ 前端快照 ============

#[derive(Serialize, Clone)]
struct StateSnapshot {
    exercises: Vec<Exercise>,
    plan: Plan,
    summary: DaySummary,
    sessions: Vec<Session>, // 今日全部成绩（含回合与加练，前端按 round_no 区分）
    streak: i64,
    heatmap: Vec<HeatCell>,
    config: Config,
    in_work_hours: bool,
    next_due_in_secs: u64,
    snoozed_until: Option<String>,
    notif_available: bool, // dev 模式（非 .app bundle）下 macOS 无真通知
    version: String,
}

fn snapshot(app: &AppHandle) -> StateSnapshot {
    let shared = app.state::<Shared>();
    let cfg = load_config();
    let now = Local::now();
    let today = now.date_naive();
    let plan = shared.store.ensure_plan(&today, cfg.rounds_per_day);
    let summary = shared.store.day_summary(&today, cfg.rounds_per_day);
    let sessions = shared.store.today_sessions(&today);
    let window = reminder::WorkWindow {
        start: reminder::parse_hhmm(&cfg.work_start).unwrap_or((9, 0)),
        end: reminder::parse_hhmm(&cfg.work_end).unwrap_or((19, 0)),
    };
    let next = reminder::next_sleep(
        cfg.interval_minutes as i64,
        window,
        now,
        *shared.last_fire.lock().unwrap(),
        *shared.snooze_until.lock().unwrap(),
    );
    // 先落局部变量再进结构体字面量，避免尾表达式里 MutexGuard 临时值活得比 `shared` 久
    let snoozed_until: Option<String> = shared
        .snooze_until
        .lock()
        .unwrap()
        .map(|t| t.format("%H:%M").to_string());
    StateSnapshot {
        exercises: shared.store.exercises(),
        plan,
        summary,
        sessions,
        streak: shared.store.streak(&today),
        heatmap: shared.store.heatmap(&today, 182),
        config: cfg,
        in_work_hours: reminder::in_work_hours(now, window),
        next_due_in_secs: next.as_secs(),
        snoozed_until,
        notif_available: notif_available(),
        version: env!("CARGO_PKG_VERSION").into(),
    }
}

/// macOS：dev 模式（非 .app bundle）下无真通知；其他平台恒可用
#[cfg(target_os = "macos")]
fn notif_available() -> bool {
    macos_notify::available()
}
#[cfg(not(target_os = "macos"))]
fn notif_available() -> bool {
    true
}

fn emit_state(app: &AppHandle) {
    let _ = app.emit("state-updated", snapshot(app));
}

// ============ 命令 ============

#[tauri::command]
fn get_state(app: AppHandle) -> StateSnapshot {
    snapshot(&app)
}

/// 完成某回合（手动记录成绩；M2 起摄像头模式 source=camera）
#[tauri::command]
fn log_round(app: AppHandle, round_no: usize, reps: Option<i64>, seconds: Option<i64>) -> Result<(), String> {
    let shared = app.state::<Shared>();
    let today = Local::now().date_naive();
    let plan = shared.store.ensure_plan(&today, load_config().rounds_per_day);
    let ex_id = plan
        .slots
        .get(round_no - 1)
        .map(|s| s.exercise_id.clone())
        .ok_or("回合不存在")?;
    shared
        .store
        .log(&ex_id, &today, &Local::now(), Some(round_no), reps, seconds, "manual", false);
    emit_state(&app);
    update_tray(&app);
    Ok(())
}

/// 加练：不属于任何回合的额外成绩
#[tauri::command]
fn log_bonus(app: AppHandle, exercise_id: String, reps: Option<i64>, seconds: Option<i64>) -> Result<(), String> {
    let shared = app.state::<Shared>();
    shared
        .store
        .log(&exercise_id, &Local::now().date_naive(), &Local::now(), None, reps, seconds, "manual", false);
    emit_state(&app);
    update_tray(&app);
    Ok(())
}

#[tauri::command]
fn skip_round(app: AppHandle, round_no: usize) {
    let shared = app.state::<Shared>();
    shared.store.skip_round(&Local::now().date_naive(), round_no);
    emit_state(&app);
    update_tray(&app);
}

#[tauri::command]
fn swap_round(app: AppHandle, round_no: usize, new_exercise_id: String) -> Result<(), String> {
    let shared = app.state::<Shared>();
    shared
        .store
        .swap_round(&Local::now().date_naive(), round_no, &new_exercise_id)
        .map(|_| ())
        .ok_or("仅待完成状态的回合可更换动作")?;
    emit_state(&app);
    Ok(())
}

/// 稍后提醒：到点顺延 N 分钟
#[tauri::command]
fn snooze(app: AppHandle, minutes: i64) {
    let shared = app.state::<Shared>();
    let until = Local::now() + chrono::Duration::minutes(minutes.max(1));
    *shared.snooze_until.lock().unwrap() = Some(until);
    eprintln!("[spar] snoozed until {until}");
    wake_up(&app);
    emit_state(&app);
}

/// 手动触发一次提醒（托盘菜单「现在来一回合」/ 面板按钮）
#[tauri::command]
fn fire_now(app: AppHandle) {
    reminder::fire(&app);
}

#[tauri::command]
fn get_config() -> Config {
    load_config()
}

#[tauri::command]
fn save_config(app: AppHandle, config: Config) -> Result<(), String> {
    // 字段合法性：时段可解析、间隔下限 5 分钟、回合数 1..=8
    if reminder::parse_hhmm(&config.work_start).is_none()
        || reminder::parse_hhmm(&config.work_end).is_none()
    {
        return Err("工作时段格式应为 HH:MM".into());
    }
    let cfg = Config {
        interval_minutes: config.interval_minutes.clamp(5, 240),
        rounds_per_day: config.rounds_per_day.clamp(1, 8),
        ..config
    };
    save_config_file(&cfg)?;
    wake_up(&app); // 间隔/时段变更立即生效
    emit_state(&app);
    Ok(())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn log_js(msg: String) {
    eprintln!("[ui] {msg}");
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

// ============ 托盘 ============

/// 托盘角标状态色：橙 = 有待办回合，绿 = 今日完成，灰 = 非工作时段/无计划
fn tray_status(app: &AppHandle) -> (u8, u8, u8, String) {
    let shared = app.state::<Shared>();
    let cfg = load_config();
    let now = Local::now();
    let window = reminder::WorkWindow {
        start: reminder::parse_hhmm(&cfg.work_start).unwrap_or((9, 0)),
        end: reminder::parse_hhmm(&cfg.work_end).unwrap_or((19, 0)),
    };
    let en = cfg.lang == "en";
    if !reminder::in_work_hours(now, window) {
        return (142, 142, 147, if en { "Off duty".into() } else { "下班休息中".into() });
    }
    let plan = shared.store.ensure_plan(&now.date_naive(), cfg.rounds_per_day);
    let done = plan.slots.iter().filter(|s| s.status == "done").count();
    let total = plan.slots.len();
    if done >= total {
        (29, 185, 84, if en { format!("All {total} rounds done today") } else { format!("今日 {total} 回合全部完成") })
    } else {
        (255, 159, 10, if en { format!("Today {done}/{total} rounds") } else { format!("今日 {done}/{total} 回合") })
    }
}

fn tray_image(app: &AppHandle, r: u8, g: u8, b: u8) -> tauri::image::Image<'static> {
    let size = 32u32;
    let rgba = tray_icon::render_tray_rgba(r, g, b, size * 2, menu_bar_is_dark(app)); // 2x 抗锯齿
    tauri::image::Image::new_owned(rgba, size * 2, size * 2)
}

/// 菜单栏深浅判定（macOS 26+ 菜单栏不随主题翻转，恒白；旧版用隐藏窗口 theme 探测）
fn menu_bar_is_dark(app: &AppHandle) -> bool {
    #[cfg(target_os = "macos")]
    if macos_major() >= 26 {
        return true;
    }
    app.get_webview_window("panel")
        .and_then(|w| w.theme().ok())
        .map(|t| t == tauri::Theme::Dark)
        .unwrap_or(true)
}

/// macOS 产品版本主版本号（kern.osproductversion），只查一次并缓存
#[cfg(target_os = "macos")]
fn macos_major() -> u32 {
    static MAJOR: OnceLock<u32> = OnceLock::new();
    *MAJOR.get_or_init(|| {
        std::process::Command::new("sysctl")
            .args(["-n", "kern.osproductversion"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().split('.').next()?.parse().ok())
            .unwrap_or(0)
    })
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let cfg = load_config();
    let en = cfg.lang == "en";
    let (_, _, _, status_text) = tray_status(app);

    let mut items: Vec<Box<dyn IsMenuItem<tauri::Wry>>> = Vec::new();
    items.push(Box::new(MenuItem::with_id(
        app, "noop", &format!("Spar · {status_text}"), false, None::<&str>,
    )?));
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(MenuItem::with_id(
        app, "panel",
        if en { "Open Panel" } else { "打开面板" }, true, None::<&str>,
    )?));
    items.push(Box::new(MenuItem::with_id(
        app, "fire",
        if en { "Start a Round Now" } else { "现在来一回合" }, true, None::<&str>,
    )?));
    items.push(Box::new(MenuItem::with_id(
        app, "snooze10",
        if en { "Snooze 10 min" } else { "稍后提醒（10 分钟）" }, true, None::<&str>,
    )?));
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(MenuItem::with_id(
        app, "quit",
        if en { "Quit" } else { "退出" }, true, None::<&str>,
    )?));

    let refs: Vec<&dyn IsMenuItem<tauri::Wry>> = items.iter().map(|b| b.as_ref()).collect();
    Menu::with_items(app, &refs)
}

pub fn update_tray(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else { return };
    let (r, g, b, tip) = tray_status(app);
    let _ = tray.set_icon(Some(tray_image(app, r, g, b)));
    let _ = tray.set_tooltip(Some(tip));
    if let Ok(menu) = build_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn show_panel(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("panel") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().0.as_str() {
        "panel" => show_panel(app),
        "fire" => reminder::fire(app),
        "snooze10" => snooze(app.clone(), 10),
        "quit" => app.exit(0),
        _ => {}
    }
}

// ============ 面板定位到托盘图标下方 ============
//
// 双屏坑（gitCIStatus 实证）：tray-icon 在 macOS 上报的是「全局逻辑坐标 × 托盘屏
// scale」的伪物理坐标，而 set_position(PhysicalPosition) 又按窗口所在屏 scale 反推——
// 两屏缩放不同时面板会错位。解法：先还原为全局逻辑点，再用 LogicalPosition 设置。

#[cfg(target_os = "macos")]
fn position_panel_below_tray(
    w: &tauri::WebviewWindow,
    click: PhysicalPosition<f64>,
    rect: tauri::Rect,
) {
    let mon = w
        .available_monitors()
        .ok()
        .and_then(|ms| {
            ms.into_iter().find(|m| {
                let (mp, ms) = (m.position(), m.size());
                click.x >= mp.x as f64
                    && click.x < (mp.x + ms.width as i32) as f64
                    && click.y >= mp.y as f64
                    && click.y < (mp.y + ms.height as i32) as f64
            })
        })
        .or_else(|| w.primary_monitor().ok().flatten());
    let Some(mon) = mon else { return };

    let k = mon.scale_factor().max(0.0001);
    let (mx, my, mw, mh) = (
        mon.position().x as f64 / k,
        mon.position().y as f64 / k,
        mon.size().width as f64 / k,
        mon.size().height as f64 / k,
    );

    let wk = w.scale_factor().unwrap_or(1.0).max(0.0001);
    let (pw, ph) = match w.outer_size() {
        Ok(s) => (s.width as f64 / wk, s.height as f64 / wk),
        Err(_) => (400.0, 660.0),
    };

    let rpos = rect.position.to_physical::<f64>(k);
    let rsize = rect.size.to_physical::<f64>(k);
    let icon_cx = (rpos.x + rsize.width / 2.0) / k;
    let icon_bottom = (rpos.y + rsize.height) / k;
    let x = (icon_cx - pw / 2.0).clamp(mx + 4.0, (mx + mw - pw - 4.0).max(mx + 4.0));
    let y = (icon_bottom + 6.0).clamp(my, (my + mh - ph).max(my));

    let _ = w.set_position(tauri::LogicalPosition::new(x, y));
}

/// Win/Linux：tray-icon 上报真实物理像素，直接定位；任务栏在下时翻转到图标上方
#[cfg(not(target_os = "macos"))]
fn position_panel_below_tray(
    w: &tauri::WebviewWindow,
    click: PhysicalPosition<f64>,
    rect: tauri::Rect,
) {
    let _ = click;
    let rpos = rect.position.to_physical::<f64>(1.0);
    let rsize = rect.size.to_physical::<f64>(1.0);
    let icon_cx = rpos.x + rsize.width / 2.0;
    let icon_bottom = rpos.y + rsize.height;

    let (pw, ph) = match w.outer_size() {
        Ok(s) => (s.width as f64, s.height as f64),
        Err(_) => (400.0, 660.0),
    };

    let mon = w
        .available_monitors()
        .ok()
        .and_then(|ms| {
            ms.into_iter().find(|m| {
                let (mp, msz) = (m.position(), m.size());
                let (cx, cy) = (rpos.x + rsize.width / 2.0, rpos.y + rsize.height / 2.0);
                cx >= mp.x as f64
                    && cx < (mp.x + msz.width as i32) as f64
                    && cy >= mp.y as f64
                    && cy < (mp.y + msz.height as i32) as f64
            })
        })
        .or_else(|| w.primary_monitor().ok().flatten());
    let Some(mon) = mon else { return };
    let (mx, my, mw, mh) = (
        mon.position().x as f64,
        mon.position().y as f64,
        mon.size().width as f64,
        mon.size().height as f64,
    );

    let x = (icon_cx - pw / 2.0).clamp(mx + 4.0, (mx + mw - pw - 4.0).max(mx + 4.0));
    let mut y = icon_bottom + 6.0;
    if y + ph > my + mh {
        y = (rpos.y - ph - 6.0).max(my);
    }
    let _ = w.set_position(tauri::PhysicalPosition::new(x.round() as i32, y.round() as i32));
}

// ============ 开机自启（LaunchAgent / 注册表 / XDG，不引插件） ============

#[tauri::command]
async fn autostart_state() -> bool {
    tauri::async_runtime::spawn_blocking(autostart_enabled)
        .await
        .unwrap_or(false)
}

#[tauri::command]
async fn set_autostart_cmd(enable: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || set_autostart(enable))
        .await
        .map_err(|e| format!("JOIN {e}"))?
}

#[cfg(target_os = "macos")]
fn autostart_file() -> std::path::PathBuf {
    let mut p = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push("Library/LaunchAgents/io.github.spar.plist");
    p
}

#[cfg(target_os = "macos")]
fn autostart_enabled() -> bool {
    autostart_file().exists()
}

#[cfg(target_os = "macos")]
fn set_autostart(enable: bool) -> Result<(), String> {
    let path = autostart_file();
    if !enable {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>io.github.spar</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#,
        exe.display()
    );
    std::fs::write(&path, plist).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
const AUTOSTART_REG: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(target_os = "windows")]
fn autostart_enabled() -> bool {
    std::process::Command::new("reg")
        .args(["query", AUTOSTART_REG, "/v", "Spar"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn set_autostart(enable: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if enable {
        let val = format!("\"{}\"", exe.display());
        std::process::Command::new("reg")
            .args(["add", AUTOSTART_REG, "/v", "Spar", "/t", "REG_SZ", "/d", &val, "/f"])
            .output()
            .map_err(|e| e.to_string())?;
    } else {
        std::process::Command::new("reg")
            .args(["delete", AUTOSTART_REG, "/v", "Spar", "/f"])
            .output()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn autostart_file() -> std::path::PathBuf {
    let mut p = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"));
    p.push("autostart/spar.desktop");
    p
}

#[cfg(target_os = "linux")]
fn autostart_enabled() -> bool {
    autostart_file().exists()
}

#[cfg(target_os = "linux")]
fn set_autostart(enable: bool) -> Result<(), String> {
    let path = autostart_file();
    if !enable {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Spar\nExec=\"{}\"\nX-GNOME-Autostart-enabled=true\n",
        exe.display()
    );
    std::fs::write(&path, entry).map_err(|e| e.to_string())
}

// ============ 入口 ============

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .on_window_event(|window, event| {
            if window.label() == "panel" {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = window.hide();
                }
                // 系统深浅色切换：菜单栏外观跟着变，托盘色块颜色即时重绘
                if let tauri::WindowEvent::ThemeChanged(_) = event {
                    update_tray(window.app_handle());
                }
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // macOS：装原生通知 delegate（必须在应用完成启动前，点击回调才生效）
            #[cfg(target_os = "macos")]
            macos_notify::init(app.handle().clone());

            let store = Store::open(Some(db_path().to_str().expect("db 路径合法")))
                .map_err(|e| format!("打开数据库失败: {e}"))?;

            let (tx, rx) = mpsc::channel::<()>();
            app.manage(Shared {
                store,
                wake: Mutex::new(tx),
                last_fire: Mutex::new(Some(Local::now())), // 启动即起算，首个间隔后才提醒
                snooze_until: Mutex::new(None),
            });

            // 先建隐藏面板窗口：托盘色块颜色要用它的 theme() 探测系统深浅
            let panel_builder =
                WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
                    .title("Spar")
                    .inner_size(400.0, 660.0)
                    .decorations(false)
                    .transparent(true)
                    .shadow(true)
                    .resizable(false)
                    .visible(false)
                    .skip_taskbar(true)
                    .always_on_top(true);
            // macOS 26+：原生液态玻璃；旧版回退 Popover vibrancy；
            // 前端收到 html.glass 后表面走半透明，让玻璃透出来（gitCIStatus 同款机制）
            #[cfg(target_os = "macos")]
            let panel_builder = {
                let b = panel_builder.effects(tauri::utils::config::WindowEffectsConfig {
                    effects: vec![
                        tauri::window::Effect::LiquidGlassRegular,
                        tauri::window::Effect::Popover,
                    ],
                    state: None,
                    radius: Some(16.0),
                    color: None,
                    interactive: false,
                });
                if macos_major() >= 26 {
                    b.initialization_script("document.documentElement.classList.add('glass')")
                } else {
                    b
                }
            };
            #[cfg(target_os = "macos")]
            let _panel = {
                let p = panel_builder.build()?;
                // 方形阴影 rim 修复：WKWebView 图层是全窗方形，底部两角会露出方形阴影线；
                // 把 webview 图层裁成玻璃同款圆角（窗口刚建好时未 layer-backed，需延迟取）
                if macos_major() >= 26 {
                    let w2 = p.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(800));
                        let w3 = w2.clone();
                        let _ = w3.run_on_main_thread(move || {
                            unsafe {
                                let Ok(view_ptr) = w2.ns_view() else { return };
                                let webview: &objc2_app_kit::NSView =
                                    &*(view_ptr as *const objc2_app_kit::NSView);
                                webview.setWantsLayer(true);
                                if let Some(layer) = webview.layer() {
                                    layer.setCornerRadius(16.0);
                                    layer.setMasksToBounds(true);
                                } else {
                                    eprintln!("[glass] webview layer unavailable; shadow rim stays square");
                                }
                            }
                        });
                    });
                }
                p
            };
            #[cfg(not(target_os = "macos"))]
            panel_builder.build()?;

            let menu = build_menu(app.handle())?;
            TrayIconBuilder::with_id("main")
                .icon(tray_image(app.handle(), 255, 159, 10))
                .tooltip("Spar")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| handle_menu_event(app, event))
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("panel") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                position_panel_below_tray(&w, position, rect);
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            let handle = app.handle().clone();
            std::thread::spawn(move || reminder::run(handle, rx));

            update_tray(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            log_round,
            log_bonus,
            skip_round,
            swap_round,
            snooze,
            fire_now,
            get_config,
            save_config,
            quit_app,
            log_js,
            app_version,
            autostart_state,
            set_autostart_cmd
        ])
        .build(tauri::generate_context!())
        .expect("Spar 启动失败")
        .run(|app, event| {
            // macOS：点击系统通知会激活应用并触发 reopen；此时唤起面板
            #[cfg(target_os = "macos")]
            if let RunEvent::Reopen { .. } = event {
                show_panel(app);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
