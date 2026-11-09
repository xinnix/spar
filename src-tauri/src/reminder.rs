//! 回合调度器：工作时段内每隔 N 分钟提醒一回合。
//!
//! 时间数学全部抽成纯函数（可单测）；线程循环沿用 gitCIStatus 验证过的
//! mpsc `recv_timeout` 模式——既是定时器，又能被 wake() 立即唤醒
//! （配置修改、稍后提醒、手动触发都走唤醒路径）。

use chrono::{DateTime, Duration, Local, NaiveTime, TimeZone};
use std::sync::mpsc;
use tauri::{AppHandle, Emitter, Manager};

/// 工作时段（本地时间，同日窗口；start >= end 视为全天）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkWindow {
    pub start: (u32, u32), // (时, 分)
    pub end: (u32, u32),
}

pub fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

fn at(date: chrono::NaiveDate, hm: (u32, u32)) -> Option<DateTime<Local>> {
    Local.from_local_datetime(&date.and_hms_opt(hm.0, hm.1, 0)?).single()
}

pub fn in_work_hours(now: DateTime<Local>, w: WorkWindow) -> bool {
    if w.start >= w.end {
        return true; // 全天
    }
    let t = now.time();
    let s = NaiveTime::from_hms_opt(w.start.0, w.start.1, 0).unwrap();
    let e = NaiveTime::from_hms_opt(w.end.0, w.end.1, 0).unwrap();
    t >= s && t < e
}

/// 把到期时刻夹进工作窗口：早于当天开始 → 当天开始；不早于当天结束 → 次日开始。
fn clamp_to_work(due: DateTime<Local>, w: WorkWindow) -> DateTime<Local> {
    if w.start >= w.end {
        return due;
    }
    let day = due.date_naive();
    let ws = at(day, w.start).unwrap_or(due);
    let we = at(day, w.end).unwrap_or(due);
    if due < ws {
        ws
    } else if due >= we {
        at(day + Duration::days(1), w.start).unwrap_or(due)
    } else {
        due
    }
}

/// 距下次提醒的休眠时长。
/// - 贪睡未到点 → 睡到贪睡时刻
/// - due = 上次提醒 + 间隔；再夹进工作窗口
pub fn next_sleep(
    interval_min: i64,
    w: WorkWindow,
    now: DateTime<Local>,
    last_fire: Option<DateTime<Local>>,
    snooze_until: Option<DateTime<Local>>,
) -> std::time::Duration {
    if let Some(sz) = snooze_until {
        if sz > now {
            return (sz - now).to_std().unwrap_or(std::time::Duration::from_secs(1));
        }
    }
    let due = match last_fire {
        Some(t) => t + Duration::minutes(interval_min),
        None => now, // 调用方在启动时把 last_fire 初始化为启动时刻，这里只兜底
    };
    let due = clamp_to_work(due, w);
    if due <= now {
        return std::time::Duration::from_secs(1);
    }
    (due - now).to_std().unwrap_or(std::time::Duration::from_secs(1))
}

/// 调度线程主体：到点 → 挑当前 pending 回合 → 通知 + 广播 + 托盘刷新。
pub fn run(app: AppHandle, rx: mpsc::Receiver<()>) {
    loop {
        let shared = app.state::<crate::Shared>();
        let cfg = crate::load_config();
        let window = WorkWindow {
            start: parse_hhmm(&cfg.work_start).unwrap_or((9, 0)),
            end: parse_hhmm(&cfg.work_end).unwrap_or((19, 0)),
        };
        let now = Local::now();
        let last_fire = *shared.last_fire.lock().unwrap();
        let snooze = *shared.snooze_until.lock().unwrap();
        let wait = next_sleep(cfg.interval_minutes as i64, window, now, last_fire, snooze);

        match rx.recv_timeout(wait) {
            Err(mpsc::RecvTimeoutError::Timeout) => {
                fire(&app);
            }
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                // 被唤醒：重读配置/贪睡状态再算（配置变更、稍后提醒、手动触发）
                crate::update_tray(&app);
            }
        }
    }
}

/// 触发一次提醒。今日回合全部 done/skipped 时不打扰（只刷新托盘为完成态）。
pub(crate) fn fire(app: &AppHandle) {
    let shared = app.state::<crate::Shared>();
    *shared.last_fire.lock().unwrap() = Some(Local::now());
    *shared.snooze_until.lock().unwrap() = None;

    let cfg = crate::load_config();
    let en = cfg.lang == "en";
    let now = Local::now();
    let store = &shared.store;
    let plan = store.ensure_plan(&now.date_naive(), cfg.rounds_per_day);

    let Some(round_no) = plan
        .slots
        .iter()
        .position(|s| s.status == "pending")
        .map(|i| i + 1)
    else {
        crate::update_tray(app); // 今日全部完成/跳过 → 绿色完成态
        return;
    };
    let ex_id = plan.slots[round_no - 1].exercise_id.clone();
    let ex = store.exercise_map().get(&ex_id).cloned();
    let (name, goal_text) = match &ex {
        Some(e) => {
            let name = if en { e.name_en.clone() } else { e.name_zh.clone() };
            let goal = if e.unit == "seconds" {
                if en { format!("{}s", e.default_goal) } else { format!("{} 秒", e.default_goal) }
            } else {
                format!("×{}", e.default_goal)
            };
            (name, goal)
        }
        None => (ex_id.clone(), String::new()),
    };

    let (title, body) = if en {
        (format!("Spar · Round {round_no}"), format!("{name} {goal_text} — go!"))
    } else {
        (format!("Spar · 第 {round_no} 回合"), format!("{name} {goal_text}，做完回来打卡！"))
    };

    #[cfg(target_os = "macos")]
    crate::macos_notify::notify(app, &title, &body, &format!("round-{round_no}"));
    #[cfg(not(target_os = "macos"))]
    {
        use tauri_plugin_notification::NotificationExt;
        let _ = app.notification().builder().title(title).body(body).show();
    }

    let _ = app.emit("round-due", serde_json::json!({ "round_no": round_no, "exercise_id": ex_id }));
    crate::update_tray(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap()
    }

    fn win() -> WorkWindow {
        WorkWindow { start: (9, 0), end: (19, 0) }
    }

    #[test]
    fn parse_hhmm_valid_and_invalid() {
        assert_eq!(parse_hhmm("09:30"), Some((9, 30)));
        assert_eq!(parse_hhmm("23:59"), Some((23, 59)));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("9:60"), None);
        assert_eq!(parse_hhmm("abc"), None);
    }

    #[test]
    fn in_hours_boundaries() {
        let w = win();
        assert!(!in_work_hours(dt(2026, 11, 9, 8, 59), w));
        assert!(in_work_hours(dt(2026, 11, 9, 9, 0), w));
        assert!(in_work_hours(dt(2026, 11, 9, 18, 59), w));
        assert!(!in_work_hours(dt(2026, 11, 9, 19, 0), w));
        // 全天窗口
        let all = WorkWindow { start: (9, 0), end: (9, 0) };
        assert!(in_work_hours(dt(2026, 11, 9, 3, 0), all));
    }

    #[test]
    fn sleep_is_interval_inside_window() {
        let now = dt(2026, 11, 9, 14, 0);
        let last = Some(dt(2026, 11, 9, 13, 0));
        let d = next_sleep(60, win(), now, last, None);
        assert_eq!(d.as_secs(), 1); // due 14:00 已到点 → 1s 内触发
    }

    #[test]
    fn due_before_window_waits_for_start() {
        // 上次提醒 18:40，间隔 60 分钟 → due 19:40 已出窗 → 次日 09:00
        let now = dt(2026, 11, 9, 18, 50);
        let last = Some(dt(2026, 11, 9, 18, 40));
        let d = next_sleep(60, win(), now, last, None);
        // 18:50 → 次日 09:00 = 14h10m
        assert_eq!(d.as_secs(), (14 * 3600 + 10 * 60));
    }

    #[test]
    fn morning_boot_first_reminder_after_interval() {
        // 08:30 开机（上班前）：启动时 last_fire=08:30 → due 09:30 在窗口内
        let now = dt(2026, 11, 9, 8, 30);
        let last = Some(dt(2026, 11, 9, 8, 30));
        let d = next_sleep(60, win(), now, last, None);
        // 08:30 → 09:30 = 60 分钟
        assert_eq!(d.as_secs(), 3600);
    }

    #[test]
    fn snooze_overrides_everything() {
        let now = dt(2026, 11, 9, 14, 0);
        let snooze = Some(dt(2026, 11, 9, 14, 10));
        let d = next_sleep(60, win(), now, Some(now), snooze);
        assert_eq!(d.as_secs(), 600);
    }

    #[test]
    fn expired_snooze_is_ignored() {
        let now = dt(2026, 11, 9, 14, 0);
        let snooze = Some(dt(2026, 11, 9, 13, 0)); // 已过期
        let d = next_sleep(60, win(), now, Some(dt(2026, 11, 9, 13, 0)), snooze);
        assert_eq!(d.as_secs(), 1); // due 14:00 到点
    }

    #[test]
    fn no_window_means_plain_interval() {
        let all = WorkWindow { start: (9, 0), end: (9, 0) };
        let now = dt(2026, 11, 9, 22, 0);
        let d = next_sleep(30, all, now, Some(now), None);
        assert_eq!(d.as_secs(), 1800);
    }
}
