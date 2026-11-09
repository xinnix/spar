// Spar landing — 双语 i18n（模式与 glint/site 同源）+ 动作库现场动画
// 动画驱动：所有 demo canvas 共享一个 rAF 循环，IntersectionObserver 只画可见的

const I18N = {
  en: {
    'meta.title': 'Spar — fight the sitting, one round at a time',

    'nav.demo': 'Moves', 'nav.features': 'Features', 'nav.roadmap': 'Roadmap', 'nav.install': 'Install',

    'hero.tagline': 'Fight the sitting, one round at a time',
    'hero.sub': 'A menu-bar micro-exercise reminder. During work hours Spar calls a quick round — 15 squats, 10 push-ups, a 45-second wall sit — right at your desk. Watch the form demo, do it, log it, keep the streak.',
    'hero.quote': '“One round an hour. The compiler builds, you build too.”',
    'hero.download': 'Download',
    'hero.star': 'Star on GitHub',
    'hero.demo': 'See the moves',
    'hero.meta': 'v0.1.0 · Free & open source · Tauri 2 · Local-first · macOS · Windows · Linux',
    'hero.caption': 'Three rounds a day, rotated from eight desk-friendly moves — the same stick-figure demos you see on this page run inside the app.',

    'mock.next': 'Next round ~24 min',
    'mock.r1': 'Round 1', 'mock.r2': 'Round 2', 'mock.r3': 'Round 3',
    'mock.done': 'Done', 'mock.pending': 'Pending', 'mock.reps': 'reps',

    // hero 面板拟真里引用的动作名/要点（与 exercises.js 数据同源）
    'ex.pushup': 'Push-ups', 'ex.knee_pushup': 'Knee Push-ups', 'ex.squat': 'Squats',
    'ex.calf_raise': 'Calf Raises', 'ex.wall_sit': 'Wall Sit', 'ex.desk_dip': 'Desk Dips',
    'ex.plank': 'Plank', 'ex.jumping_jack': 'Jumping Jacks',
    'cue.calf_raise': 'Rise high onto the balls of your feet; pause, then lower slowly.',
    'cue.wall_sit': 'Back flat on the wall, thighs parallel to the floor.',

    'demo.title': 'The eight moves',
    'demo.sub': 'Desk-friendly, no equipment, done in under two minutes. These are the real demo animations from the app — not videos.',

    'features.title': 'Why Spar',
    'features.sub': "Reminders tell you it's time. Spar tells you what to do, shows you how, and remembers what you did.",
    'f1.t': 'Rounds, not guilt',
    'f1.d': 'Work hours, one call every interval: "Round 2 — calf raises ×20". Three rounds a day rotated from eight moves. Small enough to actually happen between meetings.',
    'f2.t': 'Lives in your menu bar',
    'f2.d': 'One glanceable icon: orange when a round is waiting, green when today is done, gray off duty. Click the notification and the panel drops right under the tray.',
    'f3.t': 'Animated form demos',
    'f3.d': 'Every move carries a looping stick-figure demo with coaching cues — keyframed, not video, a few KB each. You always know what "good form" looks like.',
    'f4.t': 'Streak & heatmap',
    'f4.d': 'A GitHub-style contribution calendar, but for movement. Watch the greens add up, keep the 🔥 alive, and log bonus sets whenever you feel like it.',
    'f5.t': 'Local-first privacy',
    'f5.d': 'SQLite on your machine, zero telemetry, zero cloud. The camera counting in M2 runs fully on-device too — the leaderboard (M3) only ever sees the numbers you upload.',
    'f6.t': 'Seriously light',
    'f6.d': 'Tauri 2 with the system WebView — no Electron. Hand-written JS, no frameworks, no bundler. The tray icon is even drawn in code, pixel by pixel.',

    'how.title': 'A round, in four beats',
    'how.sub': 'Set your work hours and interval once. Then:',
    'how1.t': 'It waits', 'how1.d': "Spar sits in the menu bar, off duty outside your work hours.",
    'how2.t': 'It calls', 'how2.d': "A notification names the round and the move. Snooze 10 minutes if you're mid-flow.",
    'how3.t': 'You spar', 'how3.d': "Follow the animated demo. Fifteen squats later, you're back — no one even noticed.",
    'how4.t': 'It remembers', 'how4.d': 'Log the count, tick the round, and the heatmap turns another shade of green.',

    'road.title': 'Roadmap',
    'road.sub': 'Built in the open, one milestone at a time.',
    'road.m1.s': 'M1 · Shipped', 'road.m1.t': 'Remind & record',
    'road.m1.d': 'Menu-bar rounds with work-hours scheduling, animated demos, manual logging, streak & heatmap, zh/en, autostart.',
    'road.m2.s': 'M2 · Next', 'road.m2.t': 'Count with the camera',
    'road.m2.d': 'On-device pose estimation counts your reps and checks your form — with a voice coach in your ear. Verified scores, all local.',
    'road.m3.s': 'M3 · Planned', 'road.m3.t': 'Spar with friends',
    'road.m3.d': 'Invite-code groups with weekly boards. Camera-verified rounds carry a badge; the server never sees your camera.',

    'footer.cta': "Don't just sit there. Spar.",
    'footer.note': 'Built with Tauri 2 · data stays on your machine',

    'mi.title': 'Installing on macOS',
    'mi.sub': 'Drag Spar into Applications, then approve it once on first launch.',
    'mi.blocked': "First launch gets blocked — don't worry",
    'mi.blockedsub': 'Not notarized yet — follow the three steps below once, and it opens normally from then on.',
    'mi.s1.title': '"Spar" Not Opened',
    'mi.s1.trash': 'Move to Trash',
    'mi.s1.done': 'Done',
    'mi.s1.t': 'Click Done',
    'mi.s1.d': "Don't click Move to Trash",
    'mi.s2.item': 'Privacy & Security',
    'mi.s2.t': 'Open System Settings',
    'mi.s2.d': 'Choose Privacy & Security in the sidebar',
    'mi.s3.pane': 'Security',
    'mi.s3.blocked': 'Spar was blocked',
    'mi.s3.dev': 'unidentified developer',
    'mi.s3.btn': 'Open Anyway',
    'mi.s3.t': 'Click Open Anyway',
    'mi.s3.d': 'In the Security pane, enter your password once',
    'mi.alt1.t': 'In a hurry? Right-click works too',
    'mi.alt1.d': 'Right-click Spar in Applications, choose Open, then Open again.',
    'mi.alt2.t': 'If it says "damaged, can\'t be opened"',
    'mi.alt2.d1': 'Open Terminal and run',
    'mi.alt2.d2': 'then open Spar again.',
  },

  zh: {
    'meta.title': 'Spar — 久坐办公，起身一战',

    'nav.demo': '动作库', 'nav.features': '功能', 'nav.roadmap': '路线图', 'nav.install': '安装',

    'hero.tagline': '久坐办公，起身一战',
    'hero.sub': '驻守菜单栏的微运动提醒器。工作时段里，Spar 定时叫你打一回合——15 个深蹲、10 个俯卧撑、45 秒靠墙静蹲，就在工位完成。看示范、做动作、记成绩，把连胜续上。',
    'hero.quote': '“一小时一回合。编译器在构建，你也在。”',
    'hero.download': '下载安装包',
    'hero.star': '到 GitHub 点个 Star',
    'hero.demo': '看看动作库',
    'hero.meta': 'v0.1.0 · 免费开源 · Tauri 2 · 本地优先 · macOS · Windows · Linux',
    'hero.caption': '每天三回合，从 8 个工位友好动作里轮换——你在本页看到的火柴人示范，就是应用内跑的同款动画。',

    'mock.next': '下一回合 ~24 分钟',
    'mock.r1': '回合 1', 'mock.r2': '回合 2', 'mock.r3': '回合 3',
    'mock.done': '完成', 'mock.pending': '待完成', 'mock.reps': '次',

    // hero 面板拟真里引用的动作名/要点（与 exercises.js 数据同源）
    'ex.pushup': '俯卧撑', 'ex.knee_pushup': '跪姿俯卧撑', 'ex.squat': '深蹲',
    'ex.calf_raise': '提踵', 'ex.wall_sit': '靠墙静蹲', 'ex.desk_dip': '桌边臂屈伸',
    'ex.plank': '平板支撑', 'ex.jumping_jack': '开合跳',
    'cue.calf_raise': '脚跟尽量抬高，顶端停一拍再缓缓落下。',
    'cue.wall_sit': '背贴墙，大腿与地面平行，膝盖不过脚尖。',

    'demo.title': '八个动作',
    'demo.sub': '工位友好、零器械、两分钟内完成。下面跑的是应用内的真实示范动画——不是视频。',

    'features.title': '为什么是 Spar',
    'features.sub': '提醒器只告诉你“到点了”。Spar 告诉你做什么、示范怎么做，还替你记住做了多少。',
    'f1.t': '回合理念，不搞内疚',
    'f1.d': '工作时段内定时叫战：“回合 2 —— 提踵 ×20”。每天三回合、八个动作轮换。小到真的能在会议间隙发生。',
    'f2.t': '驻守菜单栏',
    'f2.d': '一颗图标读懂状态：橙 = 有回合待完成，绿 = 今日已收官，灰 = 下班休息。点通知，面板直接落在托盘下方。',
    'f3.t': '动作示范动画',
    'f3.d': '每个动作都带循环播放的火柴人示范和要点提示——关键帧驱动而非视频，每个只有几 KB。什么算“动作规范”，一眼就懂。',
    'f4.t': '连胜 · 热力日历',
    'f4.d': 'GitHub 贡献图同款的运动热力日历。看绿格慢慢变深，把 🔥 续下去；想加练随时记一组。',
    'f5.t': '本地优先',
    'f5.d': 'SQLite 在本机，零遥测、零云端。M2 的摄像头计数也全部在设备端完成——M3 排行榜只会看到你主动上传的数字。',
    'f6.t': '真的轻',
    'f6.d': 'Tauri 2 + 系统 WebView，没有 Electron。手写 JS、无框架、无打包器——连托盘图标都是代码逐像素画出来的。',

    'how.title': '一回合，四个节拍',
    'how.sub': '工作时段和间隔设一次，然后：',
    'how1.t': '它候着', 'how1.d': 'Spar 驻守菜单栏，下班时段自动息声。',
    'how2.t': '它叫战', 'how2.d': '通知报出回合与动作。正在心流里？顺延 10 分钟。',
    'how3.t': '你迎战', 'how3.d': '跟着示范动画做。15 个深蹲之后回到工位，没人发现你离开过。',
    'how4.t': '它记着', 'how4.d': '记下个数、勾掉回合，热力日历又深一度绿。',

    'road.title': '路线图',
    'road.sub': '开放构建，一个里程碑一个里程碑来。',
    'road.m1.s': 'M1 · 已完成', 'road.m1.t': '提醒 + 记录',
    'road.m1.d': '菜单栏回合、工作时段调度、示范动画、手动记录、连胜热力图、中英双语、开机自启。',
    'road.m2.s': 'M2 · 进行中', 'road.m2.t': '摄像头计数',
    'road.m2.d': '端侧姿态识别帮你数个数、查规范——还有语音教练在耳边指导。成绩带“已验证”标记，全程本地。',
    'road.m3.s': 'M3 · 规划中', 'road.m3.t': '和好友过招',
    'road.m3.d': '邀请码小组 + 每周榜单。摄像头验证的回合带徽章；服务器永远看不到你的摄像头。',

    'footer.cta': '别干坐着，来一回合。',
    'footer.note': '基于 Tauri 2 构建 · 数据只在本机',

    'mi.title': '安装到 macOS',
    'mi.sub': '把 Spar 拖进应用程序，首次打开时放行一次。',
    'mi.blocked': '第一次打开会被拦一下，别担心',
    'mi.blockedsub': '还没做签名公证，照下面三步放行一次，以后就能直接打开。',
    'mi.s1.title': '未打开 Spar',
    'mi.s1.trash': '移到废纸篓',
    'mi.s1.done': '完成',
    'mi.s1.t': '点「完成」',
    'mi.s1.d': '别点「移到废纸篓」',
    'mi.s2.item': '隐私与安全性',
    'mi.s2.t': '打开系统设置',
    'mi.s2.d': '左侧选「隐私与安全性」',
    'mi.s3.pane': '安全性',
    'mi.s3.blocked': '已阻止使用 Spar',
    'mi.s3.dev': '来自身份不明的开发者',
    'mi.s3.btn': '仍要打开',
    'mi.s3.t': '点「仍要打开」',
    'mi.s3.d': '在安全性一栏，输一次密码',
    'mi.alt1.t': '嫌设置麻烦？右键也能放行',
    'mi.alt1.d': '在 Applications 里右键点 Spar，选「打开」，再点一次「打开」即可。',
    'mi.alt2.t': '如果提示「已损坏，无法打开」',
    'mi.alt2.d1': '打开终端执行',
    'mi.alt2.d2': '后再打开。',
  },
};

// 动作名/要点（demo 网格由 JS 渲染，不走 data-i18n）
const exName = (ex, lang) => (lang === 'zh' ? ex.nameZh : ex.nameEn);
const exCues = (ex, lang) => (lang === 'zh' ? ex.cuesZh : ex.cuesEn);
const goalText = (ex, lang) => (ex.unit === 'seconds' ? (lang === 'zh' ? `${ex.goal} 秒` : `${ex.goal}s`) : `×${ex.goal}`);

let lang = localStorage.getItem('spar-site-lang');
if (lang !== 'zh' && lang !== 'en') {
  lang = (navigator.language || 'en').toLowerCase().startsWith('zh') ? 'zh' : 'en';
}

function t(key) {
  return I18N[lang][key] ?? I18N.en[key] ?? key;
}

// ============ 动作库渲染 + 动画驱动 ============

const runners = []; // {canvas, tl, t0, visible}
const io = new IntersectionObserver(
  (entries) => {
    for (const e of entries) {
      const r = runners.find((x) => x === e.target);
      if (r) r._visible = e.isIntersecting;
    }
  },
  { rootMargin: '60px' }
);

function bindCanvas(canvas, keyframes) {
  canvas._kf = keyframes;
  canvas._tl = SparSkeleton.timeline(keyframes);
  canvas._t0 = performance.now() + Math.random() * 1600; // 错开起播，避免整页同步
  canvas._visible = true;
  runners.push(canvas);
  io.observe(canvas);
}

function loop(now) {
  for (const c of runners) {
    if (!c._visible || now < c._t0) continue;
    SparSkeleton.drawPose(c, SparSkeleton.poseAt(c._tl, now - c._t0), c._kf, { stroke: '#ffa416' });
  }
  requestAnimationFrame(loop);
}

function renderDemoGrid() {
  const grid = document.getElementById('demo-grid');
  grid.innerHTML = '';
  // 清掉旧 runner 中的网格画布（hero 的保留）
  for (let i = runners.length - 1; i >= 0; i--) {
    if (runners[i].dataset.fromGrid) {
      io.unobserve(runners[i]);
      runners.splice(i, 1);
    }
  }
  for (const ex of SPAR_EXERCISES) {
    const cell = document.createElement('div');
    cell.className = 'demo-cell';
    cell.innerHTML = `
      <canvas data-from-grid="1"></canvas>
      <div class="demo-name">${exName(ex, lang)}</div>
      <div class="demo-goal">${goalText(ex, lang)}</div>
      <div class="demo-cues">${exCues(ex, lang)}</div>`;
    grid.appendChild(cell);
    const canvas = cell.querySelector('canvas');
    canvas.style.width = '150px';
    canvas.style.height = '132px';
    bindCanvas(canvas, ex.keyframes);
  }
}

// hero 面板里的三个迷你动画
document.querySelectorAll('.demo-canvas.mini').forEach((canvas) => {
  const ex = SPAR_EXERCISES.find((x) => x.id === canvas.dataset.ex);
  if (ex) bindCanvas(canvas, ex.keyframes);
});

// ============ i18n ============

function applyLang() {
  document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en';
  document.title = t('meta.title');
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });
  document.getElementById('lang-toggle').textContent = lang === 'zh' ? 'EN' : '中文';
  // 网格重建单独兜底：exercises.js 加载失败（部署切换瞬间的缓存错配）时
  // 只丢网格、不中断语言应用，避免出现「标题中文 + 卡片英文」的混合态
  try {
    renderDemoGrid();
  } catch (e) {
    console.error('renderDemoGrid failed:', e);
  }
}

document.getElementById('lang-toggle').addEventListener('click', () => {
  lang = lang === 'zh' ? 'en' : 'zh';
  localStorage.setItem('spar-site-lang', lang);
  applyLang();
});

applyLang();
requestAnimationFrame(loop);
