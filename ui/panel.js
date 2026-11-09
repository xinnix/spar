// Spar 面板前端：纯 JS，无构建步骤（withGlobalTauri 暴露 window.__TAURI__）

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

function report(msg) {
  try { invoke('log_js', { msg: String(msg) }); } catch (e) { /* ignore */ }
}
window.addEventListener('error', (e) => report(`error: ${e.message} @ ${e.filename}:${e.lineno}`));
window.addEventListener('unhandledrejection', (e) => report(`rejection: ${e.reason}`));

// ============ i18n ============

const I18N = {
  zh: {
    round: '回合', pending: '待完成', done: '完成', skipped: '已跳过',
    finish: '完成', swap: '换一个', skip: '跳过',
    repsUnit: '次', secsUnit: '秒', startTimer: '开始计时', stopTimer: '记 {n} 秒',
    demoTip: '点击动画查看要点',
    bonusSection: '加练一组', logIt: '记一组',
    statsSection: '今日 · 过去半年', less: '少', more: '多', hmWeeks: '26 周',
    roundsBrief: '{done}/{total} 回合 · 加练 {bonus} 组',
    streakDays: '{n} 天',
    nextRound: '下一回合约 {m} 分钟', offDuty: '休息中（工作时段外）',
    snoozedTo: '已顺延至 {hm}', dueNow: '马上！',
    settings: '设置', settingsTitle: '设置', back: '返回',
    reminderSection: '提醒', interval: '提醒间隔', workHours: '工作时段',
    roundsPerDay: '每日回合数', sound: '提醒声音',
    generalSection: '通用', autostart: '开机自启', language: '语言',
    aboutSection: '关于', slogan: '久坐办公，起身一战',
    fireNow: '现在来一回合', snooze: '稍后 10 分钟', quit: '退出',
    saved: '已保存', saveFailed: '保存失败：{msg}',
    loggedToast: '已记录 {name} {n}{unit} 💪', skippedToast: '已跳过第 {n} 回合',
    swappedToast: '已换成 {name}', bonusLogged: '加练已记录！',
    roundDueToast: '第 {n} 回合开始：{name} {goal}',
    allDone: '今日回合全部完成，漂亮！',
    devHint: '开发模式：系统通知在打包后的 .app 内才会生效',
    needNumber: '请输入有效的数量',
  },
  en: {
    round: 'Round', pending: 'Pending', done: 'Done', skipped: 'Skipped',
    finish: 'Done', swap: 'Swap', skip: 'Skip',
    repsUnit: 'reps', secsUnit: 's', startTimer: 'Start timer', stopTimer: 'Log {n}s',
    demoTip: 'Click the demo for cues',
    bonusSection: 'Bonus Set', logIt: 'Log it',
    statsSection: 'Today · last 6 months', less: 'Less', more: 'More', hmWeeks: '26 weeks',
    roundsBrief: '{done}/{total} rounds · {bonus} bonus',
    streakDays: '{n} d',
    nextRound: 'Next round in ~{m} min', offDuty: 'Off duty (outside work hours)',
    snoozedTo: 'Snoozed until {hm}', dueNow: 'Any moment!',
    settings: 'Settings', settingsTitle: 'Settings', back: 'Back',
    reminderSection: 'Reminders', interval: 'Interval', workHours: 'Work hours',
    roundsPerDay: 'Rounds per day', sound: 'Sound',
    generalSection: 'General', autostart: 'Launch at login', language: 'Language',
    aboutSection: 'About', slogan: 'Fight the sitting, one round at a time',
    fireNow: 'Round now', snooze: 'Snooze 10 min', quit: 'Quit',
    saved: 'Saved', saveFailed: 'Save failed: {msg}',
    loggedToast: 'Logged {name} ×{n} {unit} 💪', skippedToast: 'Round {n} skipped',
    swappedToast: 'Swapped to {name}', bonusLogged: 'Bonus set logged!',
    roundDueToast: 'Round {n}: {name} {goal}',
    allDone: 'All rounds done today. Nice!',
    devHint: 'Dev mode: system notifications work only in the packaged .app',
    needNumber: 'Enter a valid number',
  },
};

let lang = 'zh';
const t = (k) => I18N[lang][k] ?? k;
const tf = (k, vars) => {
  let s = t(k);
  for (const [k2, v] of Object.entries(vars || {})) s = s.replaceAll(`{${k2}}`, v);
  return s;
};

// ============ 状态 ============

let state = null; // get_state 快照
const demos = new Map(); // canvas → {destroy}
const timers = new Map(); // round_no → interval（计时回合）
let nextDueAt = 0; // 本地倒数（毫秒时间戳）
let detailDemo = null;

const $ = (id) => document.getElementById(id);
const exName = (ex) => (lang === 'en' ? ex.name_en : ex.name_zh);
const exCues = (ex) => (lang === 'en' ? ex.cues_en : ex.cues_zh);
const exById = (id) => state && state.exercises.find((e) => e.id === id);
const goalText = (ex) => (ex.unit === 'seconds' ? `${ex.default_goal}${t('secsUnit')}` : `×${ex.default_goal}`);

function showToast(text) {
  const el = $('toast');
  el.textContent = text;
  el.classList.remove('hidden');
  clearTimeout(el._timer);
  el._timer = setTimeout(() => el.classList.add('hidden'), 2600);
}

// ============ 渲染 ============

function applyI18n() {
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });
  document.documentElement.lang = lang;
}

function renderHeader() {
  const streak = state.streak;
  $('streak-pill').classList.toggle('hidden', streak <= 0);
  $('streak-pill').querySelector('b').textContent = tf('streakDays', { n: streak });
  $('lang-btn').textContent = lang === 'zh' ? 'EN' : '中';
  updateNextDue();
  $('dev-hint').classList.toggle('hidden', state.notif_available);
}

function updateNextDue() {
  const el = $('next-due');
  if (state.snoozed_until) {
    el.textContent = tf('snoozedTo', { hm: state.snoozed_until });
    return;
  }
  if (!state.in_work_hours) {
    el.textContent = t('offDuty');
    return;
  }
  const left = Math.max(0, Math.round((nextDueAt - Date.now()) / 60000));
  el.textContent = left <= 0 ? t('dueNow') : tf('nextRound', { m: left });
}

function destroyDemos() {
  for (const d of demos.values()) d.destroy();
  demos.clear();
  for (const iv of timers.values()) clearInterval(iv);
  timers.clear();
}

function renderRounds() {
  destroyDemos();
  const list = $('round-list');
  list.innerHTML = '';

  state.plan.slots.forEach((slot, i) => {
    const roundNo = i + 1;
    const ex = exById(slot.exercise_id) || state.exercises[0];
    const session = state.sessions.find((s) => s.round_no === roundNo);
    const card = document.createElement('div');
    card.className = `round-card ${slot.status}`;
    card.id = `round-${roundNo}`;

    const statusChip = `<span class="status-chip ${slot.status}">${t(slot.status)}</span>`;
    const recorded = session
      ? `<div class="done-info">✓ ${
          session.seconds != null ? `${session.seconds} ${t('secsUnit')}` : `${session.reps ?? 0} ${t('repsUnit')}`
        } · ${(session.ts || '').slice(11, 16)}</div>`
      : '';

    let actions = '';
    if (slot.status === 'pending') {
      if (ex.unit === 'seconds') {
        actions = `
          <button class="timer-btn primary" data-round="${roundNo}">
            <span class="timer-label">${t('startTimer')}</span>
          </button>`;
      } else {
        actions = `
          <span class="stepper">
            <button class="step-btn" data-round="${roundNo}" data-d="-1">−</button>
            <input class="reps-input" data-round="${roundNo}" type="number" min="1" value="${ex.default_goal}" />
            <button class="step-btn" data-round="${roundNo}" data-d="1">＋</button>
          </span>
          <button class="primary" data-finish="${roundNo}">${t('finish')}</button>`;
      }
      actions += `
        <span class="spacer"></span>
        <button class="ghost-btn small" data-swap="${roundNo}">${t('swap')}</button>
        <button class="ghost-btn small" data-skip="${roundNo}">${t('skip')}</button>`;
    }

    card.innerHTML = `
      <div class="round-main">
        <div class="round-head">
          <span class="round-no">${t('round')} ${roundNo}</span>
          <b class="ex-name">${exName(ex)}</b>
          <span class="goal">${goalText(ex)}</span>
          ${statusChip}
        </div>
        <p class="cues">${exCues(ex)}</p>
        ${actions}
        ${recorded}
      </div>
      <div class="demo-wrap" title="${t('demoTip')}">
        <canvas class="demo"></canvas>
      </div>`;

    list.appendChild(card);
    const canvas = card.querySelector('canvas.demo');
    demos.set(canvas, SparSkeleton.mountDemo(canvas, ex.keyframes, { stroke: '#ff9f0a' }));
    card.querySelector('.demo-wrap').addEventListener('click', () => openDetail(ex));
  });

  // 状态变化后的钩子
  const doneAll = state.plan.slots.length > 0 && state.plan.slots.every((s) => s.status !== 'pending');
  if (doneAll && state._justFinished) {
    showToast(t('allDone'));
    state._justFinished = false;
  }
}

function renderBonus() {
  const sel = $('bonus-ex');
  const prev = sel.value;
  sel.innerHTML = '';
  for (const ex of state.exercises) {
    const opt = document.createElement('option');
    opt.value = ex.id;
    opt.textContent = exName(ex);
    sel.appendChild(opt);
  }
  if (prev && exById(prev)) sel.value = prev;
  syncBonusUnit();
}

function syncBonusUnit() {
  const ex = exById($('bonus-ex').value);
  if (!ex) return;
  const isSecs = ex.unit === 'seconds';
  $('bonus-unit').textContent = isSecs ? t('secsUnit') : t('repsUnit');
  $('bonus-num').value = ex.default_goal;
  $('bonus-num').min = isSecs ? 5 : 1;
}

function renderStats() {
  const s = state.summary;
  $('today-brief').textContent = tf('roundsBrief', {
    done: s.rounds_done, total: s.rounds_total, bonus: s.bonus_sets,
  });

  // 热力日历：周一为一周之始，列 = 周，行 = 周一..周日
  const grid = $('heatmap');
  grid.innerHTML = '';
  const cells = state.heatmap; // 升序，末尾是今天
  if (!cells.length) return;
  const first = new Date(cells[0].date + 'T00:00:00');
  let lead = (first.getDay() + 6) % 7; // 周一=0
  const cols = document.createDocumentFragment();
  // 首列补白
  for (let i = 0; i < lead; i++) {
    const c = document.createElement('i');
    c.className = 'hm-cell pad';
    cols.appendChild(c);
  }
  for (const cell of cells) {
    const level = cell.count >= 3 ? 3 : cell.count;
    const c = document.createElement('i');
    c.className = `hm-cell l${level}`;
    c.title = `${cell.date.slice(5)} · ${cell.count}`;
    cols.appendChild(c);
  }
  grid.appendChild(cols);
}

function render() {
  applyI18n();
  renderHeader();
  renderRounds();
  renderBonus();
  renderStats();
  renderSettings();
}

function renderSettings() {
  const cfg = state.config;
  $('interval-select').value = String(cfg.interval_minutes);
  if (![...$('interval-select').options].some((o) => o.value === String(cfg.interval_minutes))) {
    $('interval-select').value = '60';
  }
  $('work-start').value = cfg.work_start;
  $('work-end').value = cfg.work_end;
  $('rounds-select').value = String(cfg.rounds_per_day);
  $('sound-toggle').checked = cfg.sound;
  $('lang-select').value = cfg.lang;
  $('about-version').textContent = state.version;
}

// ============ 详情浮层 ============

function openDetail(ex) {
  $('detail-name').textContent = exName(ex);
  $('detail-goal').textContent = goalText(ex);
  $('detail-cues').textContent = exCues(ex);
  $('detail-overlay').classList.remove('hidden');
  const canvas = $('detail-canvas');
  canvas.style.width = '280px';
  canvas.style.height = '220px';
  if (detailDemo) detailDemo.destroy();
  detailDemo = SparSkeleton.mountDemo(canvas, ex.keyframes, { stroke: '#ff9f0a', lineW: 4.2 });
}

function closeDetail() {
  $('detail-overlay').classList.add('hidden');
  if (detailDemo) { detailDemo.destroy(); detailDemo = null; }
}

// ============ 动作 ============

async function refresh() {
  try {
    state = await invoke('get_state');
    lang = state.config.lang === 'en' ? 'en' : 'zh';
    nextDueAt = Date.now() + state.next_due_in_secs * 1000;
    render();
  } catch (e) {
    report(`get_state failed: ${e}`);
  }
}

function findCard(btn, attr) {
  const n = Number(btn.dataset[attr]);
  return { n, card: $(`round-${n}`) };
}

async function finishReps(roundNo) {
  const input = document.querySelector(`.reps-input[data-round="${roundNo}"]`);
  const reps = Number(input && input.value);
  if (!reps || reps < 0) { showToast(t('needNumber')); return; }
  const ex = exById(state.plan.slots[roundNo - 1].exercise_id);
  await invoke('log_round', { roundNo, reps, seconds: null });
  state._justFinished = true;
  showToast(tf('loggedToast', { name: exName(ex), n: reps, unit: t('repsUnit') }));
}

function startTimer(roundNo, btn) {
  const label = btn.querySelector('.timer-label');
  const start = Date.now();
  btn.classList.add('ticking');
  const iv = setInterval(() => {
    const sec = Math.round((Date.now() - start) / 1000);
    label.textContent = `${String(Math.floor(sec / 60)).padStart(2, '0')}:${String(sec % 60).padStart(2, '0')}`;
  }, 500);
  timers.set(roundNo, iv);
  btn.dataset.elapsedStart = String(start);
}

async function stopTimer(roundNo, btn) {
  const iv = timers.get(roundNo);
  if (iv) { clearInterval(iv); timers.delete(roundNo); }
  const sec = Math.max(5, Math.round((Date.now() - Number(btn.dataset.elapsedStart || 0)) / 1000));
  const ex = exById(state.plan.slots[roundNo - 1].exercise_id);
  await invoke('log_round', { roundNo, reps: null, seconds: sec });
  state._justFinished = true;
  showToast(tf('loggedToast', { name: exName(ex), n: sec, unit: t('secsUnit') }));
}

async function swapTo(roundNo) {
  const cur = state.plan.slots[roundNo - 1].exercise_id;
  const idx = state.exercises.findIndex((e) => e.id === cur);
  const next = state.exercises[(idx + 1) % state.exercises.length];
  try {
    await invoke('swap_round', { roundNo, newExerciseId: next.id });
    showToast(tf('swappedToast', { name: exName(next) }));
  } catch (e) {
    report(`swap failed: ${e}`);
  }
}

// 事件委托：回合卡上的所有按钮
$('round-list').addEventListener('click', async (ev) => {
  const btn = ev.target.closest('button');
  if (!btn) return;
  if (btn.dataset.finish) return finishReps(Number(btn.dataset.finish));
  if (btn.dataset.skip) {
    const { n } = findCard(btn, 'skip');
    await invoke('skip_round', { roundNo: n });
    showToast(tf('skippedToast', { n }));
    return;
  }
  if (btn.dataset.swap) return swapTo(Number(btn.dataset.swap));
  if (btn.dataset.round && btn.classList.contains('timer-btn')) {
    if (btn.classList.contains('ticking')) stopTimer(Number(btn.dataset.round), btn);
    else startTimer(Number(btn.dataset.round), btn);
    return;
  }
  if (btn.dataset.d) {
    const n = Number(btn.dataset.round);
    const input = document.querySelector(`.reps-input[data-round="${n}"]`);
    input.value = Math.max(1, Number(input.value) + Number(btn.dataset.d));
  }
});

// 加练
$('bonus-ex').addEventListener('change', syncBonusUnit);
$('bonus-log-btn').addEventListener('click', async () => {
  const ex = exById($('bonus-ex').value);
  const n = Number($('bonus-num').value);
  if (!ex || !n || n <= 0) { showToast(t('needNumber')); return; }
  const reps = ex.unit === 'seconds' ? null : n;
  const seconds = ex.unit === 'seconds' ? n : null;
  await invoke('log_bonus', { exerciseId: ex.id, reps, seconds });
  showToast(tf('loggedToast', { name: exName(ex), n, unit: ex.unit === 'seconds' ? t('secsUnit') : t('repsUnit') }));
});

// 顶栏/底栏
$('fire-btn').addEventListener('click', () => invoke('fire_now'));
$('snooze-btn').addEventListener('click', () => invoke('snooze', { minutes: 10 }));
$('quit-btn').addEventListener('click', () => invoke('quit_app'));
$('lang-btn').addEventListener('click', async () => {
  const next = lang === 'zh' ? 'en' : 'zh';
  await invoke('save_config', { config: { ...state.config, lang: next } });
});

// 设置视图
$('settings-btn').addEventListener('click', async () => {
  $('autostart-toggle').checked = await invoke('autostart_state');
  $('main-view').classList.add('hidden');
  $('settings-view').classList.remove('hidden');
});
$('back-btn').addEventListener('click', () => {
  $('settings-view').classList.add('hidden');
  $('main-view').classList.remove('hidden');
});

async function saveSettings() {
  const cfg = {
    interval_minutes: Number($('interval-select').value),
    work_start: $('work-start').value || '09:00',
    work_end: $('work-end').value || '19:00',
    rounds_per_day: Number($('rounds-select').value),
    sound: $('sound-toggle').checked,
    lang: $('lang-select').value,
  };
  try {
    await invoke('save_config', { config: cfg });
    const msg = $('settings-message');
    msg.textContent = t('saved');
    msg.classList.remove('hidden', 'error');
    setTimeout(() => msg.classList.add('hidden'), 1800);
  } catch (e) {
    const msg = $('settings-message');
    msg.textContent = tf('saveFailed', { msg: e });
    msg.classList.remove('hidden');
    msg.classList.add('error');
  }
}

for (const id of ['interval-select', 'work-start', 'work-end', 'rounds-select', 'sound-toggle', 'lang-select']) {
  $(id).addEventListener('change', saveSettings);
}

$('autostart-toggle').addEventListener('change', async (ev) => {
  try {
    await invoke('set_autostart_cmd', { enable: ev.target.checked });
  } catch (e) {
    ev.target.checked = !ev.target.checked;
    report(`autostart failed: ${e}`);
  }
});

// 详情浮层
$('detail-close').addEventListener('click', closeDetail);
$('detail-overlay').addEventListener('click', (ev) => {
  if (ev.target === $('detail-overlay')) closeDetail();
});

// ============ 后端事件 ============

listen('state-updated', (ev) => {
  const wasLang = lang;
  state = ev.payload;
  lang = state.config.lang === 'en' ? 'en' : 'zh';
  nextDueAt = Date.now() + state.next_due_in_secs * 1000;
  render();
  if (wasLang !== lang) return; // 语言切换会整体重渲染
});

listen('round-due', (ev) => {
  const { round_no, exercise_id } = ev.payload;
  const ex = exById(exercise_id);
  showToast(tf('roundDueToast', { n: round_no, name: ex ? exName(ex) : '', goal: ex ? goalText(ex) : '' }));
  const card = $(`round-${round_no}`);
  if (card) {
    card.classList.remove('pulse');
    void card.offsetWidth; // 重启动画
    card.classList.add('pulse');
  }
});

// 倒数每 30 秒本地刷新一次（不重新拉快照）
setInterval(updateNextDue, 30000);

refresh();
