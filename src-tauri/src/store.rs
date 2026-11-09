//! SQLite 存储：运动库（含示范关键帧）+ 每日回合计划 + 成绩记录 + 统计。
//!
//! 关键帧格式（前端 skeleton.js 消费，两侧必须同构）：
//!   anchor: "feet"（站立系，以脚为锚点） | "hand"（支撑系，以手为锚点）
//!   frames: [{ "p": [torso, arm, elbow, hip, knee, lift], "hold": 毫秒 }, ...]
//! 角度约定（度，侧视火柴人朝右）：
//!   D(a) = (sin a, cos a)：a=0 垂直向下，+90 正前，-90 正后，180 正上
//!   torso  0=直立，+ = 向前倾（D(180-torso) 为颈→头方向）
//!   arm    上臂绝对角（0=垂下，+ = 前抬）
//!   elbow  肘弯，前臂方向 = D(arm + elbow)
//!   hip    大腿绝对角（0=垂直向下，+ = 前抬，支撑类为 -90 即向正后）
//!   knee   膝弯，小腿方向 = D(hip - knee)
//!   lift   髋部竖直像素偏移（- = 上提，提踵/开合跳用）
//! 动画在相邻关键帧间插值并往返循环。

use chrono::{DateTime, Local, NaiveDate};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

// ============ 前端数据模型 ============

#[derive(Debug, Clone, Serialize)]
pub struct Exercise {
    pub id: String,
    pub name_zh: String,
    pub name_en: String,
    pub unit: String, // reps | seconds
    pub default_goal: i64,
    pub anchor: String,          // feet | hand（示范动画锚点）
    pub keyframes: serde_json::Value, // {anchor, frames:[{p:[6], hold}]}
    pub cues_zh: String,
    pub cues_en: String,
    pub sort: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Slot {
    pub exercise_id: String,
    pub status: String, // pending | done | skipped
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub date: String, // YYYY-MM-DD
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub id: i64,
    pub exercise_id: String,
    pub ts: String,
    pub local_date: String,
    pub round_no: Option<i64>,
    pub reps: Option<i64>,
    pub seconds: Option<i64>,
    pub source: String, // manual | camera
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PerExerciseTotal {
    pub exercise_id: String,
    pub sets: i64,
    pub reps: i64,
    pub seconds: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DaySummary {
    pub rounds_total: i64,
    pub rounds_done: i64,
    pub rounds_skipped: i64,
    pub per_exercise: Vec<PerExerciseTotal>,
    pub bonus_sets: i64, // 加练组数（不属于任何回合）
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatCell {
    pub date: String,
    pub count: i64,
}

// ============ 运动库种子（8 项办公微运动，含示范关键帧） ============

const SEED_JSON: &str = r#"[
  {
    "id": "pushup", "name_zh": "俯卧撑", "name_en": "Push-ups",
    "unit": "reps", "default_goal": 10, "anchor": "hand",
    "cues_zh": "核心收紧，身体成一条直线；下到肘约 90°。", "cues_en": "Keep core tight, body in a line; lower to ~90° elbows.",
    "keyframes": {
      "anchor": "hand",
      "frames": [
        { "p": [90, 0, 0, -90, 0, 0], "hold": 500 },
        { "p": [90, -8, 95, -90, 0, 0], "hold": 400 }
      ]
    }
  },
  {
    "id": "knee_pushup", "name_zh": "跪姿俯卧撑", "name_en": "Knee Push-ups",
    "unit": "reps", "default_goal": 12, "anchor": "hand",
    "cues_zh": "膝盖着地降低难度，从头到膝仍成一条直线。", "cues_en": "Knees down for an easier scale; head-to-knees still a straight line.",
    "keyframes": {
      "anchor": "hand",
      "frames": [
        { "p": [70, -5, 0, -66, 24, 0], "hold": 500 },
        { "p": [70, -15, 95, -66, 24, 0], "hold": 400 }
      ]
    }
  },
  {
    "id": "squat", "name_zh": "深蹲", "name_en": "Squats",
    "unit": "reps", "default_goal": 15, "anchor": "feet",
    "cues_zh": "臀部向后坐，大腿至少与地面平行，膝盖对脚尖。", "cues_en": "Hips back, thighs at least parallel; knees track over toes.",
    "keyframes": {
      "anchor": "feet",
      "frames": [
        { "p": [5, 10, 0, 5, 5, 0], "hold": 450 },
        { "p": [30, 85, 0, 100, 105, 0], "hold": 400 }
      ]
    }
  },
  {
    "id": "calf_raise", "name_zh": "提踵", "name_en": "Calf Raises",
    "unit": "reps", "default_goal": 20, "anchor": "feet",
    "cues_zh": "脚跟尽量抬高，顶端停一拍再缓缓落下。", "cues_en": "Rise high onto the balls of your feet; pause, then lower slowly.",
    "keyframes": {
      "anchor": "feet",
      "frames": [
        { "p": [3, 8, 0, 3, 3, 0], "hold": 400 },
        { "p": [3, 14, 0, 3, 8, -9], "hold": 350 }
      ]
    }
  },
  {
    "id": "wall_sit", "name_zh": "靠墙静蹲", "name_en": "Wall Sit",
    "unit": "seconds", "default_goal": 45, "anchor": "feet",
    "cues_zh": "背贴墙，大腿与地面平行，膝盖不过脚尖。", "cues_en": "Back flat on the wall, thighs parallel to the floor.",
    "keyframes": {
      "anchor": "feet",
      "frames": [
        { "p": [0, 25, 0, 90, 92, 0], "hold": 900 },
        { "p": [0, 27, 0, 91, 93, -1], "hold": 900 }
      ]
    }
  },
  {
    "id": "desk_dip", "name_zh": "桌边臂屈伸", "name_en": "Desk Dips",
    "unit": "reps", "default_goal": 12, "anchor": "hand",
    "cues_zh": "手撑桌沿、背对桌子，肘向后弯沉身体。", "cues_en": "Hands on desk edge, back to the desk; bend elbows to sink down.",
    "keyframes": {
      "anchor": "hand",
      "frames": [
        { "p": [-10, -115, 0, 70, -25, 0], "hold": 450 },
        { "p": [-12, -170, 43, 72, -25, 0], "hold": 400 }
      ]
    }
  },
  {
    "id": "plank", "name_zh": "平板支撑", "name_en": "Plank",
    "unit": "seconds", "default_goal": 40, "anchor": "hand",
    "cues_zh": "肘撑地正下方，从头到脚跟一条直线，别塌腰。", "cues_en": "Elbows under shoulders; head-to-heels one straight line.",
    "keyframes": {
      "anchor": "hand",
      "frames": [
        { "p": [90, 0, 90, -90, 0, 0], "hold": 800 },
        { "p": [90, 0, 92, -90, 2, -1], "hold": 800 }
      ]
    }
  },
  {
    "id": "jumping_jack", "name_zh": "开合跳", "name_en": "Jumping Jacks",
    "unit": "reps", "default_goal": 30, "anchor": "feet",
    "cues_zh": "手臂过头打开，双脚开合，保持节奏轻落地。", "cues_en": "Arms overhead, feet in and out; stay light on the landing.",
    "keyframes": {
      "anchor": "feet",
      "frames": [
        { "p": [0, 12, 0, 3, 3, 0], "hold": 350 },
        { "p": [0, 172, 5, 25, 2, -7], "hold": 350 }
      ]
    }
  }
]"#;

#[derive(Debug, Deserialize)]
struct SeedExercise {
    id: String,
    name_zh: String,
    name_en: String,
    unit: String,
    default_goal: i64,
    cues_zh: String,
    cues_en: String,
    keyframes: serde_json::Value,
}

// ============ 存储本体 ============

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// 打开（或创建）数据库；`None` 为内存库（单测用）。
    pub fn open(path: Option<&str>) -> rusqlite::Result<Store> {
        let conn = match path {
            Some(p) => Connection::open(p)?,
            None => Connection::open_in_memory()?,
        };
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS exercises(
               id TEXT PRIMARY KEY,
               name_zh TEXT NOT NULL,
               name_en TEXT NOT NULL,
               unit TEXT NOT NULL,
               default_goal INTEGER NOT NULL,
               anchor TEXT NOT NULL,
               keyframes TEXT NOT NULL,
               cues_zh TEXT NOT NULL DEFAULT '',
               cues_en TEXT NOT NULL DEFAULT '',
               enabled INTEGER NOT NULL DEFAULT 1,
               sort INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS sessions(
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               exercise_id TEXT NOT NULL,
               ts TEXT NOT NULL,
               local_date TEXT NOT NULL,
               round_no INTEGER,
               reps INTEGER,
               seconds INTEGER,
               source TEXT NOT NULL DEFAULT 'manual',
               verified INTEGER NOT NULL DEFAULT 0,
               note TEXT,
               synced_at TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_sessions_date ON sessions(local_date);
             CREATE TABLE IF NOT EXISTS day_plan(
               local_date TEXT PRIMARY KEY,
               slots TEXT NOT NULL
             );",
        )?;
        let store = Store { conn: Mutex::new(conn) };
        store.seed()?;
        Ok(store)
    }

    /// 首次启动播种运动库；已播种则跳过（保证用户后续可自定义）
    fn seed(&self) -> rusqlite::Result<()> {
        let n: i64 = self
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM exercises", [], |r| r.get(0))?;
        if n > 0 {
            return Ok(());
        }
        let seeds: Vec<SeedExercise> = serde_json::from_str(SEED_JSON).expect("种子数据合法");
        let conn = self.conn.lock().unwrap();
        for (i, s) in seeds.iter().enumerate() {
            conn.execute(
                "INSERT INTO exercises(id,name_zh,name_en,unit,default_goal,anchor,keyframes,cues_zh,cues_en,enabled,sort)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,1,?10)",
                params![
                    s.id, s.name_zh, s.name_en, s.unit, s.default_goal,
                    s.keyframes["anchor"].as_str().unwrap_or("feet"),
                    serde_json::to_string(&s.keyframes).unwrap(),
                    s.cues_zh, s.cues_en, i as i64
                ],
            )?;
        }
        Ok(())
    }

    pub fn exercises(&self) -> Vec<Exercise> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT id,name_zh,name_en,unit,default_goal,anchor,keyframes,cues_zh,cues_en,sort
             FROM exercises WHERE enabled = 1 ORDER BY sort",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = stmt.query_map([], |r| {
            let kf: String = r.get(6)?;
            Ok(Exercise {
                id: r.get(0)?,
                name_zh: r.get(1)?,
                name_en: r.get(2)?,
                unit: r.get(3)?,
                default_goal: r.get(4)?,
                anchor: r.get(5)?,
                keyframes: serde_json::from_str(&kf).unwrap_or(serde_json::Value::Null),
                cues_zh: r.get(7)?,
                cues_en: r.get(8)?,
                sort: r.get(9)?,
            })
        });
        match rows {
            Ok(it) => it.filter_map(|x| x.ok()).collect(),
            Err(_) => vec![],
        }
    }

    pub fn exercise_map(&self) -> HashMap<String, Exercise> {
        self.exercises().into_iter().map(|e| (e.id.clone(), e)).collect()
    }

    /// 读取当日回合计划；没有则按「天数 × 回合数」轮换生成（每天组合不同）。
    pub fn ensure_plan(&self, date: &NaiveDate, rounds: usize) -> Plan {
        let key = date.format("%Y-%m-%d").to_string();
        let conn = self.conn.lock().unwrap();
        if let Ok(slots_json) = conn
            .query_row("SELECT slots FROM day_plan WHERE local_date = ?1", [&key], |r| {
                r.get::<_, String>(0)
            })
        {
            if let Ok(mut slots) = serde_json::from_str::<Vec<Slot>>(&slots_json) {
                // 回合数配置变多时补齐；变少则裁掉多余的 pending（已完成保留）
                while slots.len() < rounds {
                    slots.push(Slot { exercise_id: "squat".into(), status: "pending".into() });
                }
                return Plan { date: key, slots };
            }
        }
        drop(conn);

        // 生成：enabled 列表按 (天数×rounds + i) 取模轮换，保证每天组合不同且依次覆盖
        let list = self.exercises();
        let n = list.len().max(1);
        let day_no = (*date - NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()).num_days();
        let mut slots = Vec::with_capacity(rounds);
        let mut used = std::collections::HashSet::new();
        for i in 0..rounds {
            let mut idx = ((day_no as usize) * rounds + i) % n;
            while used.contains(&list[idx].id) {
                idx = (idx + 1) % n;
            }
            used.insert(list[idx].id.clone());
            slots.push(Slot { exercise_id: list[idx].id.clone(), status: "pending".into() });
        }
        let plan = Plan { date: key, slots };
        if let Ok(json) = serde_json::to_string(&plan.slots) {
            let _ = self.conn.lock().unwrap().execute(
                "INSERT OR REPLACE INTO day_plan(local_date, slots) VALUES (?1, ?2)",
                params![plan.date, json],
            );
        }
        plan
    }

    fn save_slots(&self, date: &str, slots: &[Slot]) {
        if let Ok(json) = serde_json::to_string(slots) {
            let _ = self.conn.lock().unwrap().execute(
                "INSERT OR REPLACE INTO day_plan(local_date, slots) VALUES (?1, ?2)",
                params![date, json],
            );
        }
    }

    /// 记录一次成绩。round_no 属于今日某回合时同时把该回合置 done；None = 加练。
    #[allow(clippy::too_many_arguments)]
    pub fn log(
        &self,
        exercise_id: &str,
        date: &NaiveDate,
        ts: &DateTime<Local>,
        round_no: Option<usize>,
        reps: Option<i64>,
        seconds: Option<i64>,
        source: &str,
        verified: bool,
    ) -> i64 {
        let key = date.format("%Y-%m-%d").to_string();
        let ts_s = ts.to_rfc3339();
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO sessions(exercise_id, ts, local_date, round_no, reps, seconds, source, verified)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![exercise_id, ts_s, key, round_no.map(|r| r as i64), reps, seconds, source, verified as i64],
        );
        let id = conn.last_insert_rowid();
        drop(conn);

        if let Some(r) = round_no {
            let plan = self.ensure_plan(date, r + 1); // 至少 r 个回合
            let mut slots = plan.slots;
            if let Some(slot) = slots.get_mut(r - 1) {
                slot.status = "done".into();
                self.save_slots(&plan.date, &slots);
            }
        }
        id
    }

    /// 跳过某回合（仍可事后补做：重新记成绩即可覆盖状态）
    pub fn skip_round(&self, date: &NaiveDate, round_no: usize) {
        let rounds = round_no + 1;
        let plan = self.ensure_plan(date, rounds);
        let mut slots = plan.slots;
        if let Some(slot) = slots.get_mut(round_no - 1) {
            if slot.status == "pending" {
                slot.status = "skipped".into();
                self.save_slots(&plan.date, &slots);
            }
        }
    }

    /// 把某回合换成别的动作（仅 pending 状态可换）
    pub fn swap_round(&self, date: &NaiveDate, round_no: usize, new_id: &str) -> Option<Plan> {
        let rounds = round_no + 1;
        let plan = self.ensure_plan(date, rounds);
        let mut slots = plan.slots;
        let slot = slots.get_mut(round_no - 1)?;
        if slot.status != "pending" || slot.exercise_id == new_id {
            return None;
        }
        slot.exercise_id = new_id.to_string();
        self.save_slots(&plan.date, &slots);
        Some(Plan { date: plan.date, slots })
    }

    pub fn day_summary(&self, date: &NaiveDate, rounds_total: usize) -> DaySummary {
        let key = date.format("%Y-%m-%d").to_string();
        let conn = self.conn.lock().unwrap();
        let mut summary = DaySummary {
            rounds_total: rounds_total as i64,
            ..Default::default()
        };

        // 当日计划状态
        if let Ok(slots_json) = conn
            .query_row("SELECT slots FROM day_plan WHERE local_date = ?1", [&key], |r| {
                r.get::<_, String>(0)
            })
        {
            if let Ok(slots) = serde_json::from_str::<Vec<Slot>>(&slots_json) {
                summary.rounds_total = slots.len() as i64;
                summary.rounds_done = slots.iter().filter(|s| s.status == "done").count() as i64;
                summary.rounds_skipped = slots.iter().filter(|s| s.status == "skipped").count() as i64;
            }
        }

        // 按动作聚合当日总量（回合 + 加练合计，用于展示）
        let mut stmt = match conn.prepare(
            "SELECT exercise_id, COUNT(*), SUM(COALESCE(reps,0)), SUM(COALESCE(seconds,0))
             FROM sessions WHERE local_date = ?1 GROUP BY exercise_id ORDER BY MIN(id)",
        ) {
            Ok(s) => s,
            Err(_) => return summary,
        };
        let rows = stmt.query_map([&key], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        });
        if let Ok(it) = rows {
            for (eid, sets, reps, secs) in it.filter_map(|x| x.ok()) {
                summary.per_exercise.push(PerExerciseTotal {
                    exercise_id: eid,
                    sets,
                    reps,
                    seconds: secs,
                });
            }
        }

        // 加练组数单独数（同动作的回合与加练会在 GROUP BY 里并组，不能靠 MIN(round_no) 区分）
        let _ = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE local_date = ?1 AND round_no IS NULL",
                [&key],
                |r| {
                    summary.bonus_sets = r.get(0)?;
                    Ok(())
                },
            );
        summary
    }

    /// 连续运动天数：从 today 往回数；今天还没动则从昨天起算（连胜仍算活着）。
    pub fn streak(&self, today: &NaiveDate) -> i64 {
        let conn = self.conn.lock().unwrap();
        let mut days: Vec<String> = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT local_date FROM sessions ORDER BY local_date DESC") {
            if let Ok(it) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                for d in it.filter_map(|x| x.ok()) {
                    days.push(d);
                }
            }
        }
        drop(conn);
        let set: std::collections::HashSet<&str> = days.iter().map(|s| s.as_str()).collect();

        let mut cursor = *today;
        // 今天没有成绩则从昨天开始数（不因“今天还没练”清零连胜）
        if !set.contains(cursor.format("%Y-%m-%d").to_string().as_str()) {
            cursor -= chrono::Duration::days(1);
        }
        let mut streak = 0i64;
        loop {
            let key = cursor.format("%Y-%m-%d").to_string();
            if set.contains(key.as_str()) {
                streak += 1;
                cursor -= chrono::Duration::days(1);
            } else {
                break;
            }
        }
        streak
    }

    /// 最近 `days` 天的每日组数（GitHub 贡献图热力数据）
    pub fn heatmap(&self, today: &NaiveDate, days: usize) -> Vec<HeatCell> {
        let from = (*today - chrono::Duration::days(days as i64 - 1))
            .format("%Y-%m-%d")
            .to_string();
        let conn = self.conn.lock().unwrap();
        let mut map: HashMap<String, i64> = HashMap::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT local_date, COUNT(*) FROM sessions WHERE local_date >= ?1 GROUP BY local_date",
        ) {
            if let Ok(it) = stmt.query_map([&from], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            }) {
                for (d, c) in it.filter_map(|x| x.ok()) {
                    map.insert(d, c);
                }
            }
        }
        drop(conn);
        (0..days)
            .map(|i| {
                let d = *today - chrono::Duration::days(days as i64 - 1 - i as i64);
                let key = d.format("%Y-%m-%d").to_string();
                HeatCell { count: *map.get(&key).unwrap_or(&0), date: key }
            })
            .collect()
    }

    pub fn today_sessions(&self, date: &NaiveDate) -> Vec<Session> {
        let key = date.format("%Y-%m-%d").to_string();
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT id, exercise_id, ts, local_date, round_no, reps, seconds, source, verified
             FROM sessions WHERE local_date = ?1 ORDER BY id",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = stmt.query_map([&key], |r| {
            Ok(Session {
                id: r.get(0)?,
                exercise_id: r.get(1)?,
                ts: r.get(2)?,
                local_date: r.get(3)?,
                round_no: r.get(4)?,
                reps: r.get(5)?,
                seconds: r.get(6)?,
                source: r.get(7)?,
                verified: r.get::<_, i64>(8)? != 0,
            })
        });
        match rows {
            Ok(it) => it.filter_map(|x| x.ok()).collect(),
            Err(_) => vec![],
        }
    }
}

// ============ 单测（内存库） ============

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone};

    fn ts(y: i32, m: u32, d: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn seed_populates_eight_exercises() {
        let s = Store::open(None).unwrap();
        let list = s.exercises();
        assert_eq!(list.len(), 8);
        assert!(list.iter().any(|e| e.id == "pushup"));
        // 关键帧必须是可解析对象，含 anchor 与 frames
        for e in &list {
            assert!(e.keyframes["anchor"].is_string(), "{} anchor", e.id);
            assert!(e.keyframes["frames"].as_array().unwrap().len() >= 2, "{} frames", e.id);
        }
    }

    #[test]
    fn plan_rotates_by_day_and_is_distinct() {
        let s = Store::open(None).unwrap();
        let d1 = s.ensure_plan(&date(2026, 11, 9), 3);
        let d2 = s.ensure_plan(&date(2026, 11, 10), 3);
        assert_eq!(d1.slots.len(), 3);
        assert_eq!(d2.slots.len(), 3);
        // 同一天三个回合动作互不相同
        let ids: Vec<&str> = d1.slots.iter().map(|x| x.exercise_id.as_str()).collect();
        let uniq: std::collections::HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(uniq.len(), 3, "{ids:?}");
        // 相邻两天组合不同（轮换推进）
        assert_ne!(
            d1.slots.iter().map(|x| x.exercise_id.clone()).collect::<Vec<_>>(),
            d2.slots.iter().map(|x| x.exercise_id.clone()).collect::<Vec<_>>()
        );
        // 重复读取返回持久化结果（不重新生成）
        let d1b = s.ensure_plan(&date(2026, 11, 9), 3);
        assert_eq!(d1.slots, d1b.slots);
    }

    #[test]
    fn log_round_marks_done_and_summary_counts() {
        let s = Store::open(None).unwrap();
        let d = date(2026, 11, 9);
        let plan = s.ensure_plan(&d, 3);
        let ex = &plan.slots[0].exercise_id;

        s.log(ex, &d, &ts(2026, 11, 9), Some(1), Some(12), None, "manual", false);
        let sum = s.day_summary(&d, 3);
        assert_eq!(sum.rounds_done, 1);
        assert_eq!(sum.per_exercise[0].reps, 12);
        assert_eq!(sum.per_exercise[0].sets, 1);

        // 加练（无回合）
        s.log("squat", &d, &ts(2026, 11, 9), None, Some(15), None, "manual", false);
        let sum = s.day_summary(&d, 3);
        assert_eq!(sum.bonus_sets, 1);
        assert_eq!(sum.rounds_done, 1); // 不变
    }

    #[test]
    fn skip_and_swap_only_touch_pending() {
        let s = Store::open(None).unwrap();
        let d = date(2026, 11, 9);
        s.ensure_plan(&d, 3);
        let before = s.ensure_plan(&d, 3);

        s.skip_round(&d, 2);
        let after = s.ensure_plan(&d, 3);
        assert_eq!(after.slots[1].status, "skipped");
        assert_eq!(after.slots[0].status, "pending");

        // 已 done 的回合不能换
        s.log(&before.slots[0].exercise_id.clone(), &d, &ts(2026, 11, 9), Some(1), Some(10), None, "manual", false);
        assert!(s.swap_round(&d, 1, "plank").is_none());
        // pending 的回合可以换
        assert!(s.swap_round(&d, 3, "plank").is_some());
        let swapped = s.ensure_plan(&d, 3);
        assert_eq!(swapped.slots[2].exercise_id, "plank");
    }

    #[test]
    fn streak_breaks_on_gap() {
        let s = Store::open(None).unwrap();
        let today = date(2026, 11, 9);
        // 三连：7、8、9（今天）
        for d in [date(2026, 11, 7), date(2026, 11, 8), today] {
            s.log("squat", &d, &ts(d.year(), d.month(), d.day()), None, Some(10), None, "manual", false);
        }
        assert_eq!(s.streak(&today), 3);
        // 中间隔天（6 号没练）：8、9 相连但 7 号缺 → 连胜 2
        let s2 = Store::open(None).unwrap();
        for d in [date(2026, 11, 8), date(2026, 11, 9)] {
            s2.log("squat", &d, &ts(d.year(), d.month(), d.day()), None, Some(10), None, "manual", false);
        }
        assert_eq!(s2.streak(&today), 2);
        // 完全没练 → 0
        let s3 = Store::open(None).unwrap();
        assert_eq!(s3.streak(&today), 0);
    }

    #[test]
    fn heatmap_covers_range_with_zeros() {
        let s = Store::open(None).unwrap();
        let today = date(2026, 11, 9);
        s.log("squat", &today, &ts(2026, 11, 9), None, Some(10), None, "manual", false);
        s.log("squat", &today, &ts(2026, 11, 9), None, Some(10), None, "manual", false);
        let heat = s.heatmap(&today, 28);
        assert_eq!(heat.len(), 28);
        assert_eq!(heat.last().unwrap().date, "2026-11-09");
        assert_eq!(heat.last().unwrap().count, 2);
        assert_eq!(heat.first().unwrap().count, 0);
    }
}
