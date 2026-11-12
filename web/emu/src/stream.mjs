// SPDX-License-Identifier: GPL-3.0-only
import { createLifetime } from "./lifecycle.mjs";

export function createStreamView(panel, controls, { document = globalThis.document, window = globalThis.window }) {
  const scope = createLifetime();
  const { buttons, knobs } = panel;
  const { location, history } = window;
  const getComputedStyle = window.getComputedStyle.bind(window), matchMedia = window.matchMedia.bind(window);

  const NOTE = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
  const name = "NAIWAN 91";
  const acc = getComputedStyle(document.documentElement).getPropertyValue("--color-blue").trim() || "#1b7fd6";
  const tag = "Moving harmony · Keys, bass and swells";
  const dev = panel.device, home = dev.parentNode, after = dev.nextSibling;
  const stage = document.createElement("div");
  stage.id = "stage";
  stage.style.setProperty("--sv-acc", acc);
  stage.style.setProperty("--sv-glow", "rgba(" + [1, 3, 5].map((i) => parseInt(acc.slice(i, i + 2), 16)).join(",") + ",0.24)");
  stage.innerHTML = `<canvas class="sv-bg" width="192" height="108" aria-hidden="true"></canvas><canvas class="sv-px" width="320" height="180" aria-hidden="true"></canvas><div class="sv-vig"></div><div class="sv-title"><div class="sv-bars"><i style="width:70px;background:${acc}"></i><i style="width:34px;background:#7eccc0"></i></div>
    <div><div class="sv-name">${name}</div><div class="sv-tag">${tag}</div></div></div>
    <div class="sv-link">FM-1 LINKED</div><div class="sv-dev"></div><div class="sv-big"><canvas width="240" height="240"></canvas></div>
    <div class="sv-moves"></div>
    <div class="sv-foot">${name}  ·  NAIWAN 91 (GPL-3.0)</div>
    <div class="sv-ctl"><button type="button" data-a="midi">Connect MIDI</button><button type="button" data-a="exit">Exit stream view</button></div>`;
  document.body.appendChild(stage);
  const big = stage.querySelector(".sv-big canvas"), bctx = big.getContext("2d"), screen = panel.screen;
  const moves = stage.querySelector(".sv-moves");
  bctx.imageSmoothingEnabled = false;
  let on = false;
  function fit() { stage.style.setProperty("--sv-s", Math.min(window.innerWidth / 1920, window.innerHeight / 1080)); }
  function set(v) {
    if (on === v) return;
    on = v;
    document.body.classList.toggle("stream", on);
    if (on) stage.querySelector(".sv-dev").appendChild(dev); else home.insertBefore(dev, after);
    fit(); setAudioNode(source);
    if (on) { woke(); raf = window.requestAnimationFrame(tick); }
    else { window.cancelAnimationFrame(raf); clearTimeout(idleT); }
    try { const u = new URL(location.href); if (on) u.searchParams.set("stream", ""); else u.searchParams.delete("stream"); history.replaceState(null, "", u); } catch {}
  }
  scope.on(window, "resize", fit);
  let idleT = 0;                                  // the mouse still a moment: its bar and pointer gone (for captures)
  function woke() {
    stage.classList.remove("idle");
    clearTimeout(idleT);
    idleT = setTimeout(() => stage.classList.add("idle"), 2000);
  }
  scope.on(stage, "pointermove", woke);
  scope.on(stage, "pointerdown", woke);
  // THE BACKGROUND: an analyser on the emulator's output (the mix: its first two channels) for the music, and
  // what the firmware reports of its sky, star or theme (window.ambLast.bg, from web_bg). Two canvases: soft
  // (192 x 108, smoothed up) and pixel (320 x 180, kept sharp).
  const bg = stage.querySelector(".sv-bg"), gctx = bg.getContext("2d");
  const px = stage.querySelector(".sv-px"), pctx = px.getContext("2d");
  const still = matchMedia("(prefers-reduced-motion: reduce)");
  let an = null, spec = null, binHz = 0, lastBg = 0;
  const lvl = [0, 0, 0], peak = [1e-4, 1e-4, 1e-4], NB = 40, eq = new Float32Array(NB), eqPeak = new Float32Array(NB);
  let bright = 0.5, loud = 0, topDb = -40;
  let source = null;
  function setAudioNode(node) {
    if (source && an) { try { source.disconnect(an); } catch {} }
    if (an) an.disconnect();
    source = node; an = null; spec = null;
    if (!source || !on) return;
    try {
      an = source.context.createAnalyser();
      an.fftSize = 2048; an.smoothingTimeConstant = 0.6;
      an.channelCount = 2; an.channelCountMode = "explicit"; an.channelInterpretation = "discrete";
      source.connect(an);
      spec = new Float32Array(an.frequencyBinCount);
      binHz = source.context.sampleRate / an.fftSize;
    } catch { an = null; }
  }
  function hear(dt) {                             // the music: three bands, brightness, loudness, 40 EQ bands
    an.getFloatFrequencyData(spec);
    const band = [0, 0, 0], eb = new Float32Array(NB).fill(-300);
    let sum = 0, wsum = 0, top = -200;
    for (let i = 1; i < spec.length; i++) {
      const f = i * binHz, db = spec[i], m = Math.pow(10, db / 20);
      if (f < 30 || f > 14000) continue;
      band[f < 250 ? 0 : f < 2000 ? 1 : 2] += m * m;
      sum += m;
      wsum += m * Math.log2(f);
      const k = Math.min(NB - 1, Math.max(0, Math.floor(NB * Math.log2(f / 40) / Math.log2(14000 / 40))));
      if (db > -200) eb[k] = Math.max(eb[k], db);
      top = Math.max(top, db);
    }
    if (sum > 1e-6) {
      const c = Math.min(Math.max((wsum / sum - Math.log2(200)) / Math.log2(15), 0), 1);
      bright += (c - bright) * Math.min(dt * 0.8, 1);
    }
    for (let k = 0; k < 3; k++) {                 // each band against its own recent loudest: quiet music moves too
      const v = Math.sqrt(band[k]);
      peak[k] = Math.max(v, peak[k] * Math.pow(0.5, dt / 8), 1e-4);
      const tgt = Math.min(v / peak[k], 1);
      lvl[k] += (tgt - lvl[k]) * Math.min(dt * (tgt > lvl[k] ? 5 : 0.9), 1);
    }
    topDb = Math.max(top, topDb - dt * 3);        // the EQ's scale: 50 dB under its recent loudest
    const calm = still.matches ? 0.25 : 1;
    for (let k = 0; k < NB; k++) {
      const v = eb[k] > -250 ? Math.min(Math.max((eb[k] - (topDb - 50)) / 50, 0), 1) : 0;
      eq[k] += (v - eq[k]) * Math.min(dt * (v > eq[k] ? 18 : 4) * calm, 1);
      eqPeak[k] = Math.max(eq[k], eqPeak[k] - dt * 0.35);
    }
    loud += ((sum > 1e-4 ? (lvl[0] + lvl[1] + lvl[2]) / 3 : 0) - loud) * Math.min(dt * 2, 1);
    return sum > 1e-4;
  }
  const mixc = (a, b, t) => a.map((v, i) => Math.round(v + (b[i] - v) * t));
  const rgb255 = (b, i) => [b[i] * 255, b[i + 1] * 255, b[i + 2] * 255].map(Math.round);

  // One NAIWAN spectrum view, using the instrument's current theme colours.
  const heat = document.createElement("canvas");
  heat.width = 320;
  heat.height = 120;
  const hctx = heat.getContext("2d");
  function paintSpectrum(look) {
    const pad = look ? rgb255(look, 0) : [27, 127, 214], hot = look ? rgb255(look, 3) : [242, 84, 45];
    const mid = look ? rgb255(look, 9) : [14, 158, 126], base = look ? rgb255(look, 6) : [255, 246, 227];
    const ramp = (v) => v < 0.5 ? mixc(base, pad, v * 2) : v < 0.8 ? mixc(pad, mid, (v - 0.5) / 0.3) : mixc(mid, hot, (v - 0.8) / 0.2);
    if (!still.matches) hctx.drawImage(heat, -1, 0);   // the heatmap: one column a frame, the newest at the right
    for (let k = 0; k < NB; k++) {
      const y0 = 120 - Math.round((k + 1) * 120 / NB), h = Math.round((k + 1) * 120 / NB) - Math.round(k * 120 / NB);
      hctx.fillStyle = `rgb(${ramp(eq[k]).join(",")})`;
      hctx.fillRect(319, y0, 1, h);
    }
    pctx.globalAlpha = 0.22;
    pctx.drawImage(heat, 0, 0);
    pctx.globalAlpha = 1;
    const bw = 8, segH = 3, maxSeg = 20;          // the bars: LED segments, bottom up
    for (let k = 0; k < NB; k++) {
      const x = k * bw + 1, n = Math.round(eq[k] * maxSeg), pk = Math.round(eqPeak[k] * maxSeg);
      for (let j = 0; j < n; j++) {
        const v = j / maxSeg;
        pctx.fillStyle = `rgba(${ramp(0.35 + v * 0.65).join(",")},${0.2 + 0.2 * v})`;
        pctx.fillRect(x, 178 - (j + 1) * (segH + 1), bw - 2, segH);
      }
      if (pk > 0) {
        pctx.fillStyle = `rgba(${hot.join(",")},0.45)`;
        pctx.fillRect(x, 178 - pk * (segH + 1) - 2, bw - 2, 1);
      }
    }
  }

  function drawBg(t) {
    if (t - lastBg < 33) return;                  // 30 a second
    const dt = Math.min((t - lastBg) / 1000, 0.1), s = t / 1000;
    lastBg = t;
    if (an) hear(dt);
    const look = window.ambLast && window.ambLast.bg;
    gctx.globalCompositeOperation = "source-over";
    gctx.clearRect(0, 0, 192, 108);
    pctx.clearRect(0, 0, 320, 180);
    paintSpectrum(look);
    gctx.globalCompositeOperation = "source-over";
  }
  let raf = 0;
  function tick(t) {
    if (!on || scope.disposed) return;
    bctx.drawImage(screen, 0, 0);
    stage.classList.toggle("linked", dev.classList.contains("remote"));
    drawBg(t || 0);
    raf = window.requestAnimationFrame(tick);
  }
  // the last few moves, under the screen
  function say(s, same) {
    const last = moves.lastElementChild;
    if (same && last && last.dataset.k === same && Date.now() - last.dataset.t < 900) {
      last.dataset.n = +last.dataset.n + 1;
      last.textContent = s + (last.dataset.n > 1 ? " ×" + last.dataset.n : "");
      last.dataset.t = Date.now();
      return;
    }
    const e = document.createElement("span");
    e.textContent = s;
    e.dataset.k = same || "";
    e.dataset.n = 1;
    e.dataset.t = Date.now();
    moves.appendChild(e);
    while (moves.children.length > 7) moves.firstElementChild.remove();
  }
  const moveTimer = setInterval(() => { for (const e of moves.children) e.classList.toggle("old", Date.now() - e.dataset.t > 3000); }, 400);
  const BTN_NAME = {};
  for (const [n, b] of Object.entries(buttons)) BTN_NAME[b.bit] = n;
  let seenB = 0, seenK = 0;
  const knobTimers = new Map();
  const unsubscribe = controls.subscribe((event) => {
    if (event.type === "buttons") {
      const m = event.held, down = m & ~seenB; seenB = m;
      if (down) {
        const ns = Object.keys(BTN_NAME).filter((b) => down & (1 << b)).map((b) => BTN_NAME[b]);
        const held = Object.keys(BTN_NAME).filter((b) => (m & ~down) & (1 << b)).map((b) => BTN_NAME[b]);
        say((held.length ? held.join(" + ") + " + " : "") + ns.join(" + "));
      }
    } else if (event.type === "keys") {
      const down = event.mask & ~seenK; seenK = event.mask;
      for (let i = 0; i < 27; i++) if (down & (1 << i)) { const n = 53 + i; say("KEY " + NOTE[n % 12] + (Math.floor(n / 12) - 1)); }
    } else if (event.type === "turn") {
      const { knob: k, n } = event;
      if (!n) return;
      const label = k.name || "KNOB";
      k.g.classList.add("sv-turn");
      clearTimeout(knobTimers.get(k));
      knobTimers.set(k, setTimeout(() => { k.g.classList.remove("sv-turn"); knobTimers.delete(k); }, 700));
      say(label.replace(/^KNOB(\d)/, "KNOB $1") + (n > 0 ? " ▲" : " ▼"), label + (n > 0 ? "+" : "-"));
    }
  });
  scope.on(stage.querySelector('[data-a="exit"]'), "click", () => set(false));
  scope.on(stage.querySelector('[data-a="midi"]'), "click", () => {
    const b = document.getElementById("midi-go") || document.getElementById("midiOn");
    if (b) b.click();
  });
  const btn = document.createElement("button");
  btn.type = "button";
  btn.textContent = "Stream view";
  btn.title = "For livestreams: the FM-1 and its screen large on a 16:9 stage (or open this page with ?stream)";
  const bar = document.querySelector(".bar");
  if (bar) bar.appendChild(btn);
  scope.on(btn, "click", () => set(true));
  scope.add(() => {
    unsubscribe(); clearInterval(moveTimer); clearTimeout(idleT); window.cancelAnimationFrame(raf);
    for (const [k, timer] of knobTimers) { clearTimeout(timer); k.g.classList.remove("sv-turn"); }
    on = false; setAudioNode(null);
    document.body.classList.remove("stream"); home.insertBefore(dev, after);
    stage.remove(); btn.remove();
  });
  if (new URLSearchParams(location.search).has("stream")) set(true);
  return { set, setAudioNode, dispose() { scope.dispose(); } };
}
