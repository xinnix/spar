// Spar 骨架渲染器 + 示范动画引擎（零依赖）。
// 数据来自后端运动库 keyframes：{anchor:"feet"|"hand", frames:[{p:[torso,arm,elbow,hip,knee,lift], hold}]}
// 角度约定与 src-tauri/src/store.rs 注释、scripts/preview_poses.py 三方一致：
//   D(a) = (sin a, cos a)：0=垂直向下，+90=正前(右)，-90=正后，180=正上
//   torso 0=直立 + 前倾；arm 上臂绝对角；elbow 肘弯（前臂=D(arm+elbow)）；
//   hip 大腿绝对角；knee 膝弯（小腿=D(hip-knee)）；lift 髋部竖直偏移（-=上提）
// M2 的摄像头骨架叠加将复用同一渲染器（视觉语言统一）。

const SparSkeleton = (() => {
  // 肢体长度（渲染单位，画布内按需缩放）
  const L = { torso: 46, upper: 22, fore: 20, thigh: 26, shin: 26, headR: 8 };

  const D = (a) => {
    const r = (a * Math.PI) / 180;
    return [Math.sin(r), Math.cos(r)];
  };

  const lerp = (a, b, t) => a + (b - a) * t;
  const easeInOut = (t) => (t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2);

  // 正向运动学：pose + anchor → 各关节坐标（y 向下，锚点在原点）
  function joints(pose, anchor) {
    const [torso, arm, elbow, hip, knee, lift] = pose.map(Number);
    const td = D(180 - torso);
    const ua = D(arm);
    const fa = D(arm + elbow);
    const th = D(hip);
    const sh = D(hip - knee);
    if (anchor === 'feet') {
      const foot = [0, 0];
      const hipPt = [
        foot[0] - (L.thigh * th[0] + L.shin * sh[0]),
        foot[1] - (L.thigh * th[1] + L.shin * sh[1]) + lift,
      ];
      const neck = [hipPt[0] + L.torso * td[0], hipPt[1] + L.torso * td[1]];
      return finish(hipPt, neck, td, ua, fa, th, sh, foot);
    }
    // hand 锚点：手在桌面/地面（原点）
    const hand = [0, 0];
    const shoulder = [
      hand[0] - (L.fore * fa[0] + L.upper * ua[0]),
      hand[1] - (L.fore * fa[1] + L.upper * ua[1]),
    ];
    const neck = shoulder;
    const hipPt = [neck[0] - L.torso * td[0], neck[1] - L.torso * td[1]];
    return finish(hipPt, neck, td, ua, fa, th, sh, hand);
  }

  function finish(hipPt, neck, td, ua, fa, th, sh, anchorPt) {
    const head = [neck[0] + td[0] * (L.headR + 4), neck[1] + td[1] * (L.headR + 4)];
    const elbow = [neck[0] + L.upper * ua[0], neck[1] + L.upper * ua[1]];
    const hand = [elbow[0] + L.fore * fa[0], elbow[1] + L.fore * fa[1]];
    const knee = [hipPt[0] + L.thigh * th[0], hipPt[1] + L.thigh * th[1]];
    const foot = [knee[0] + L.shin * sh[0], knee[1] + L.shin * sh[1]];
    return { hip: hipPt, neck, head, elbow, hand, knee, foot, anchor: anchorPt };
  }

  // 所有关键帧的联合包围盒 → 稳定的缩放/平移（动画期间不抖动）
  function fitTransform(keyframes, cw, ch, pad) {
    let minX = 1e9, minY = 1e9, maxX = -1e9, maxY = -1e9;
    for (const f of keyframes.frames) {
      const j = joints(f.p, keyframes.anchor);
      for (const p of Object.values(j)) {
        minX = Math.min(minX, p[0]); maxX = Math.max(maxX, p[0]);
        minY = Math.min(minY, p[1]); maxY = Math.max(maxY, p[1]);
      }
    }
    const bw = Math.max(maxX - minX, 1), bh = Math.max(maxY - minY, 1);
    const s = Math.min((cw - pad * 2) / bw, (ch - pad * 2) / bh);
    const cx = (minX + maxX) / 2, cy = (minY + maxY) / 2;
    return { s, tx: cw / 2 - cx * s, ty: ch / 2 - cy * s };
  }

  function setupCanvas(canvas) {
    const dpr = window.devicePixelRatio || 1;
    const cw = canvas.clientWidth || canvas.width;
    const ch = canvas.clientHeight || canvas.height;
    if (canvas.width !== cw * dpr || canvas.height !== ch * dpr) {
      canvas.width = cw * dpr;
      canvas.height = ch * dpr;
    }
    const ctx = canvas.getContext('2d');
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    return { ctx, cw, ch };
  }

  // 画一个姿态。opts: {stroke, ground, groundColor, lineW}
  function drawPose(canvas, pose, keyframes, opts = {}) {
    const { ctx, cw, ch } = setupCanvas(canvas);
    const stroke = opts.stroke || '#ff9f0a';
    const lw = opts.lineW || 3.2;
    const t = canvas._sparFit || (canvas._sparFit = fitTransform(keyframes, cw, ch, 8));
    const j = joints(pose, keyframes.anchor);
    const P = (p) => [t.tx + p[0] * t.s, t.ty + p[1] * t.s];

    ctx.clearRect(0, 0, cw, ch);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';

    // 地面线（沿锚点所在水平线）
    if (opts.ground !== false) {
      const [, ayy] = P(j.anchor);
      ctx.strokeStyle = opts.groundColor || 'rgba(128,128,134,0.35)';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(6, ayy);
      ctx.lineTo(cw - 6, ayy);
      ctx.stroke();
    }

    const seg = (a, b, w) => {
      const A = P(a), B = P(b);
      ctx.strokeStyle = stroke;
      ctx.lineWidth = w;
      ctx.beginPath();
      ctx.moveTo(A[0], A[1]);
      ctx.lineTo(B[0], B[1]);
      ctx.stroke();
    };
    seg(j.hip, j.neck, lw + 0.6); // 躯干略粗
    seg(j.neck, j.elbow, lw);
    seg(j.elbow, j.hand, lw);
    seg(j.hip, j.knee, lw);
    seg(j.knee, j.foot, lw);

    // 头
    const H = P(j.head);
    ctx.fillStyle = stroke;
    ctx.beginPath();
    ctx.arc(H[0], H[1], L.headR * t.s, 0, Math.PI * 2);
    ctx.fill();

    // 关节点（示范动画的“规范感”细节）
    ctx.fillStyle = 'rgba(255,255,255,0.85)';
    for (const k of [j.elbow, j.knee]) {
      const K = P(k);
      ctx.beginPath();
      ctx.arc(K[0], K[1], Math.max(1.6, lw * 0.42), 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // ============ 动画时间线（ping-pong） ============
  // hold[i] 语义：到达帧 i 所需时长（i=0 时为起始停留）。
  // 周期 = 停留f0 + Σ(hold[i] 移入) + 停留fn + Σ(反向移回)
  function timeline(keyframes) {
    const fs = keyframes.frames;
    const n = fs.length;
    const dwellStart = fs[0].hold || 400;
    const dwellEnd = fs[n - 1].hold || 400;
    const segs = []; // {from, to, dur} 按播放顺序（去程 + 回程）
    for (let i = 1; i < n; i++) segs.push({ from: i - 1, to: i, dur: fs[i].hold || 400 });
    // 回程 = 去程全部段的反转（两帧时也有 1→0 的回程，否则会瞬移）
    const back = segs.map((s) => ({ ...s, from: s.to, to: s.from })).reverse();
    const all = [{ from: 0, to: 0, dur: dwellStart }, ...segs, { from: n - 1, to: n - 1, dur: dwellEnd }, ...back];
    const total = all.reduce((s, x) => s + x.dur, 0);
    return { all, total, fs };
  }

  function poseAt(tl, t) {
    let tt = t % tl.total;
    for (const s of tl.all) {
      if (tt < s.dur) {
        if (s.from === s.to) return tl.fs[s.from].p;
        const k = easeInOut(s.dur > 0 ? tt / s.dur : 1);
        return tl.fs[s.from].p.map((v, i) => lerp(v, tl.fs[s.to].p[i], k));
      }
      tt -= s.dur;
    }
    return tl.fs[0].p;
  }

  // 挂载循环播放的示范动画；返回 {destroy()}
  function mountDemo(canvas, keyframes, opts = {}) {
    const tl = timeline(keyframes);
    let raf = 0;
    const t0 = performance.now();
    const loop = (now) => {
      // 面板隐藏时暂停（省电）：document.hidden 由调用方处理，这里只管画
      drawPose(canvas, poseAt(tl, now - t0), keyframes, opts);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return {
      destroy() {
        cancelAnimationFrame(raf);
        canvas._sparFit = null;
      },
    };
  }

  return { mountDemo, drawPose, joints, poseAt, timeline };
})();

// 供 Node/测试环境做姿势数值校验（浏览器不触发）
if (typeof module !== 'undefined' && module.exports) {
  module.exports = SparSkeleton;
}
