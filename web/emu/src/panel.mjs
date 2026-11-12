// SPDX-License-Identifier: GPL-3.0-only
import { ENC, BTN, WHITE_K, BLACK_K, BLACK_AFTER } from "./constants.mjs";
import { createLifetime } from "./lifecycle.mjs";

export function createPanel(document = globalThis.document) {
  const scope = createLifetime();
  const BODY = "#3a3d42", TRAY = "#26282c", KEY_W = "#d9dadc", KEY_B = "#c9cacd", KNOB = "#1f2124", BTNC = "#2e3135", LABEL = "#e8e8ea";
  const NS = "http://www.w3.org/2000/svg";

  const svg = document.getElementById("panel");
  function el(tag, attrs, parent = svg) {
    const e = document.createElementNS(NS, tag);
    for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
    parent.appendChild(e);
    return e;
  }
  function text(parent, x, y, s, size, fill = LABEL, weight = 600) {
    const t = el("text", { x, y, "text-anchor": "middle", "font-family": "'Barlow Semi Condensed', 'Arial Narrow', Arial, sans-serif",
                           "font-size": size, "font-weight": weight, fill, "letter-spacing": ".05em", class: "btn", "pointer-events": "none" }, parent);
    t.textContent = s;
    return t;
  }

  // ---------------------------------------------------------------- the panel
  el("rect", { x: 4, y: 4, width: 992, height: 632, rx: 44, fill: BODY });
  const knobs = {};
  function knob(x, y, r, label, role) {
    const g = el("g", { class: "ctl knob", tabindex: 0, role: "slider", "aria-label": label });
    el("circle", { cx: x, cy: y, r: r + 5, fill: TRAY }, g);
    el("circle", { cx: x, cy: y, r, fill: KNOB, stroke: "#44474c", "stroke-width": 2 }, g);
    const ind = el("line", { x1: x, y1: y - r + 5, x2: x, y2: y - r + 15, stroke: LABEL, "stroke-width": 3, "stroke-linecap": "round" }, g);
    text(g, x, y - r - 12, label, 17);
    el("circle", { cx: x, cy: y, r: r + 14, fill: "transparent" }, g);
    knobs[label] = { g, ind, x, y, angle: 0, role };
    return knobs[label];
  }
  knob(105, 100, 24, "MASTER", "master");
  knob(215, 100, 24, "SELECT", ENC.SELECT);
  knob(105, 205, 24, "PRESETS", ENC.PRESETS);
  knob(215, 205, 24, "ALGORITHM", ENC.ALGORITHM);
  for (let i = 0; i < 4; i++) knob(610 + i * 104, 100, 22, `KNOB${i + 1}`, ENC[`KNOB${i + 1}`]);

  const buttons = {};
  function button(x, y, w, h, label, two) {
    const g = el("g", { class: "ctl", role: "button", "aria-label": label });
    el("rect", { x, y, width: w, height: h, rx: 9, fill: BTNC, stroke: "#4a4d52", "stroke-width": 1.5, class: "btnface" }, g);
    if (two) { text(g, x + w / 2, y + h / 2 - 1, label, 12); text(g, x + w / 2, y + h / 2 + 12, two, 12); }
    else text(g, x + w / 2, y + h / 2 + 5, label, 15);
    buttons[label] = { g, bit: BTN[label] };
  }
  el("rect", { x: 58, y: 258, width: 210, height: 58, rx: 14, fill: TRAY });
  button(72, 268, 82, 38, "OCT-"); button(170, 268, 82, 38, "OCT+");
  el("rect", { x: 300, y: 44, width: 236, height: 236, rx: 26, fill: "#111214" });
  el("rect", { x: 568, y: 160, width: 400, height: 140, rx: 18, fill: TRAY });
  const ROW1 = ["FX", "SEL", "ENV", "LFO", "EDIT", "GLO"], ROW2 = ["HOME", "SAVE", "ARP", "SEQ", "PLAY", "REC"];
  for (let i = 0; i < 6; i++) {
    button(582 + i * 63, 174, 54, 48, ROW1[i]);
    if (ROW2[i] === "PLAY") button(582 + i * 63, 236, 54, 48, "PLAY", "STOP"); else button(582 + i * 63, 236, 54, 48, ROW2[i]);
  }

  const keys = [];
  (function keybed(x0, y0, w, h) {
    el("rect", { x: x0, y: y0, width: w, height: h, rx: 18, fill: TRAY });
    const pitch = (w - 24) / 16, kw = pitch * 0.78;
    for (let i = 0; i < 16; i++) {
      const g = el("g", { class: "ctl", role: "button", "aria-label": `white key ${i + 1}` });
      el("rect", { x: x0 + 12 + i * pitch + (pitch - kw) / 2, y: y0 + h * 0.43, width: kw, height: h * 0.52, rx: kw / 2, fill: KEY_W, class: "keyface white" }, g);
      keys[WHITE_K[i]] = { g };
    }
    BLACK_AFTER.forEach((a, k) => {
      const g = el("g", { class: "ctl", role: "button", "aria-label": `black key ${k + 1}` });
      el("rect", { x: x0 + 12 + (a + 1) * pitch - kw / 2, y: y0 + h * 0.06, width: kw, height: h * 0.33, rx: kw / 2, fill: KEY_B, class: "keyface black" }, g);
      keys[BLACK_K[k]] = { g };
    });
  })(36, 340, 928, 270);

  // ---------------------------------------------------------------- touch targets
  // The drawing is only a picture (pointer-events: none). Each control gets an invisible HTML target
  // over it: browsers (iOS Safari above all) honour touch-action and touchstart on HTML elements but
  // not on SVG ones, so the controls keep their touches while a finger anywhere else on the device
  // scrolls the page natively.
  const device = document.getElementById("device");
  function hit(c, cls) {
    const b = c.g.getBBox(), d = document.createElement("div");
    d.className = "hit" + (cls ? " " + cls : "");
    Object.assign(d.style, { left: `${b.x / 10}%`, top: `${b.y / 6.4}%`, width: `${b.width / 10}%`, height: `${b.height / 6.4}%` });
    for (const a of ["role", "aria-label", "tabindex"]) if (c.g.hasAttribute(a)) { d.setAttribute(a, c.g.getAttribute(a)); c.g.removeAttribute(a); }
    if (c.g.hasAttribute("aria-description")) { d.title = c.g.getAttribute("aria-description"); d.setAttribute("aria-description", d.title); }
    c.g.setAttribute("aria-hidden", "true");
    scope.on(d, "touchstart", (e) => e.preventDefault(), { passive: false });
    device.insertBefore(d, document.getElementById("start"));
    c.hit = d;
  }
  // what each control does, on hover (the Buttons list below says the same)
  const TIP = { FX: "The effects menu, on / off", ENV: "Pad (again: env, LFO, chords); with SEL: the voice on screen by hand", LFO: "Bass (again: env, LFO)", EDIT: "The next chord", GLO: "Global", SEL: "Shift (hold): keys 1-6 pick the pattern; PRESETS on the next bar; ALGORITHM the sound of the voice on screen; ENV the voice on screen by hand", HOME: "Tempo", SAVE: "Breakdown / build; hold: drop. On Global: store the piece (hold: factory)",
    ARP: "Acid (again: env, line)", SEQ: "Drums (again: Links, Kit, Drum drift); hold: the sequencer; with SEL: links on / off", PLAY: "Play / stop", REC: "Freeze: the chord holds, the reverb rings on",
    "OCT-": "An octave down: this page's voice, or the song; sequencer: the drum above", "OCT+": "An octave up: this page's voice, or the song; sequencer: the drum below", SELECT: "Tempo", ALGORITHM: "Mode", PRESETS: "Piece, 01 to 10",
    MASTER: "Volume" };
  for (const [n, c] of [...Object.entries(buttons), ...Object.entries(knobs)]) if (TIP[n]) c.g.setAttribute("aria-description", TIP[n]);
  for (const k of Object.values(knobs)) hit(k, "knob");
  for (const b of Object.values(buttons)) hit(b);
  keys.forEach((k) => hit(k));
  for (const [n, k] of Object.entries(knobs)) { k.hit.dataset.t = k.role === "master" ? "m" : "e" + k.role; k.name = n; }
  for (const [n, b] of Object.entries(buttons)) { b.hit.dataset.t = "b" + b.bit; b.name = n; }
  keys.forEach((k, i) => { k.hit.dataset.t = "k" + i; });


  const screen = document.getElementById("screen"), ctx = screen.getContext("2d");
  const img = ctx.createImageData(240, 240);
  function drawScreen(fb) {
    const d = img.data;
    for (let i = 0; i < 240 * 240; i++) {
      const v = fb[i], p = ((v >> 8) | (v << 8)) & 0xFFFF;
      d[i * 4] = ((p >> 11) & 31) * 255 / 31;
      d[i * 4 + 1] = ((p >> 5) & 63) * 255 / 63;
      d[i * 4 + 2] = (p & 31) * 255 / 31;
      d[i * 4 + 3] = 255;
    }
    ctx.putImageData(img, 0, 0);
  }
  function drawLights(b, k) {
    for (const x of Object.values(buttons)) x.g.classList.toggle("lit", !!(b & (1 << x.bit)));
    keys.forEach((x, i) => x.g.classList.toggle("lit", !!(k & (1 << i))));
  }


  return { device, svg, knobs, buttons, keys, screen, drawScreen, drawLights, dispose() { scope.dispose(); svg.replaceChildren(); for (const c of [...Object.values(knobs), ...Object.values(buttons), ...keys]) c.hit.remove(); } };
}
