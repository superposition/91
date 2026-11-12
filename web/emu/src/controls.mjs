// SPDX-License-Identifier: GPL-3.0-only
import { BTN, WHITE_K, BLACK_K } from "./constants.mjs";
import { createLifetime } from "./lifecycle.mjs";

export function createControls(panel, { send, isActive, window = globalThis.window }) {
  const scope = createLifetime();
  const { device, knobs, buttons, keys } = panel;

  let btnMask = 0, keyMask = 0, latched = 0, master = 2800;
  const btnHeldBy = new Map(), keyHeldBy = new Map();      // pointer / key id -> bit
  const subscribers = new Set();
  function notify(event) { for (const listener of subscribers) listener(event); }
  function syncButtons() {
    let m = latched;
    for (const b of btnHeldBy.values()) m |= 1 << b;
    if (m !== btnMask) { btnMask = m; send({ type: "buttons", mask: m }); }
    notify({ type: "buttons", mask: m, held: [...btnHeldBy.values()].reduce((mask, bit) => mask | (1 << bit), 0) });
    for (const b of Object.values(buttons)) {
      b.g.classList.toggle("held", !!(m & (1 << b.bit)));
      b.g.classList.toggle("latched", !!(latched & (1 << b.bit)));
      b.hit.classList.toggle("latched", !!(latched & (1 << b.bit)));
      b.hit.setAttribute("aria-pressed", m & (1 << b.bit) ? "true" : "false");
    }
  }
  function syncKeys() {
    let m = 0;
    for (const k of keyHeldBy.values()) m |= 1 << k;
    if (m !== keyMask) { keyMask = m; send({ type: "keys", mask: m }); }
    keys.forEach((k, i) => k.g.classList.toggle("held", !!(m & (1 << i))));
    notify({ type: "keys", mask: m });
  }

  for (const b of Object.values(buttons)) {
    scope.on(b.hit, "contextmenu", (e) => e.preventDefault());
    scope.on(b.hit, "pointerdown", (e) => {
      e.preventDefault();
      if (e.shiftKey || e.button === 2) { latched ^= 1 << b.bit; syncButtons(); return; }
      if (latched & (1 << b.bit)) { latched &= ~(1 << b.bit); syncButtons(); return; }
      b.hit.setPointerCapture(e.pointerId);
      btnHeldBy.set("p" + e.pointerId, b.bit);
      syncButtons();
    });
  }
  keys.forEach((k, i) => {
    scope.on(k.hit, "pointerdown", (e) => {
      e.preventDefault();
      k.hit.setPointerCapture(e.pointerId);
      keyHeldBy.set("p" + e.pointerId, i);
      syncKeys();
    });
  });
  function pointerUp(e) {
    const id = "p" + e.pointerId;
    if (btnHeldBy.delete(id)) syncButtons();
    if (keyHeldBy.delete(id)) syncKeys();
  }
  scope.on(device, "pointerup", pointerUp);
  scope.on(device, "pointercancel", pointerUp);
  scope.on(device, "lostpointercapture", pointerUp);

  function drawKnob(k) {
    k.ind.setAttribute("transform", `rotate(${k.angle} ${k.x} ${k.y})`);
  }
  function turn(k, n) {
    if (!n) return;
    if (k.role === "master") {
      master = Math.max(0, Math.min(4096, master + n * 64));
      k.angle = -135 + 270 * master / 4096;
      send({ type: "master", value: master });
    } else {
      k.angle = (k.angle + n * 18) % 360;
      send({ type: "enc", role: k.role, n });
    }
    drawKnob(k);
    notify({ type: "turn", knob: k, n });
  }
  for (const k of Object.values(knobs)) {
    if (k.role === "master") { k.angle = -135 + 270 * master / 4096; drawKnob(k); }
    let drag = null, wheel = 0;
    scope.on(k.hit, "pointerdown", (e) => {
      e.preventDefault();
      k.hit.setPointerCapture(e.pointerId);
      drag = { y: e.clientY, acc: 0 };
    });
    scope.on(k.hit, "pointermove", (e) => {
      if (!drag) return;
      drag.acc += drag.y - e.clientY;
      drag.y = e.clientY;
      const step = k.role === "master" ? 3 : 7;
      const n = Math.trunc(drag.acc / step);
      if (n) { drag.acc -= n * step; turn(k, n); }
    });
    const end = () => { drag = null; };
    scope.on(k.hit, "pointerup", end);
    scope.on(k.hit, "pointercancel", end);
    scope.on(k.hit, "wheel", (e) => {
      e.preventDefault();
      const dy = e.deltaMode === 1 ? e.deltaY * 33 : e.deltaY;
      if (Math.abs(dy) >= 50) { turn(k, dy < 0 ? 1 : -1); wheel = 0; return; }   // a mouse wheel notch: one detent
      wheel -= dy;                                                                   // a trackpad: by distance
      const n = Math.trunc(wheel / 25);
      if (n) { wheel -= n * 25; turn(k, n); }
    }, { passive: false });
    scope.on(k.hit, "keydown", (e) => {
      if (e.key === "ArrowUp" || e.key === "ArrowRight") { turn(k, 1); e.preventDefault(); e.stopPropagation(); }
      if (e.key === "ArrowDown" || e.key === "ArrowLeft") { turn(k, -1); e.preventDefault(); e.stopPropagation(); }
    });
  }

  const KEYMAP = {};
  "qwertyui".split("").forEach((c, i) => KEYMAP["Key" + c.toUpperCase()] = { key: WHITE_K[i] });
  "asdfghjk".split("").forEach((c, i) => KEYMAP["Key" + c.toUpperCase()] = { key: WHITE_K[8 + i] });
  "1234567890".split("").forEach((c, i) => KEYMAP["Digit" + c] = { key: BLACK_K[i] });
  KEYMAP.Minus = { key: BLACK_K[10] };
  Object.assign(KEYMAP, { Space: { btn: BTN.PLAY }, Enter: { btn: BTN.REC }, KeyV: { btn: BTN.ENV },
                          Escape: { btn: BTN.HOME }, KeyZ: { btn: BTN["OCT-"] }, KeyX: { btn: BTN["OCT+"] },
                          Backquote: { btn: BTN.FX }, KeyB: { btn: BTN.SAVE }, KeyL: { btn: BTN.LFO }, KeyP: { btn: BTN.ARP },
                          KeyM: { btn: BTN.SEQ }, KeyO: { btn: BTN.GLO }, KeyN: { btn: BTN.SEL }, KeyC: { btn: BTN.EDIT } });
  scope.on(window, "keydown", (e) => {
    if (!isActive() || e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.code === "ArrowUp" || e.code === "ArrowDown") { turn(knobs.SELECT, e.code === "ArrowUp" ? 1 : -1); e.preventDefault(); return; }
    const m = KEYMAP[e.code];
    if (!m) return;
    e.preventDefault();
    if (e.repeat) return;
    if (m.key !== undefined) { keyHeldBy.set("k" + e.code, m.key); syncKeys(); }
    else { btnHeldBy.set("k" + e.code, m.btn); syncButtons(); }
  });
  scope.on(window, "keyup", (e) => {
    const id = "k" + e.code;
    if (keyHeldBy.delete(id)) syncKeys();
    if (btnHeldBy.delete(id)) syncButtons();
  });
  scope.on(window, "blur", () => {
    for (const id of [...btnHeldBy.keys()]) if (id[0] === "k") btnHeldBy.delete(id);
    for (const id of [...keyHeldBy.keys()]) if (id[0] === "k") keyHeldBy.delete(id);
    syncButtons(); syncKeys();
  });

  function hold(id, t, down) {               // a button or key, held from MIDI (id: who holds it)
    const map = t[0] === "b" ? btnHeldBy : keyHeldBy;
    if (down) map.set(id, +t.slice(1)); else map.delete(id);
    if (t[0] === "b") syncButtons(); else syncKeys();
  }
  function setMaster(v) {
    const k = knobs.MASTER;
    master = Math.max(0, Math.min(4096, Math.round(v)));
    k.angle = -135 + 270 * master / 4096;
    send({ type: "master", value: master });
    drawKnob(k);
  }
  function releaseFrom(prefix) {
    for (const id of [...btnHeldBy.keys()]) if (id.startsWith(prefix)) btnHeldBy.delete(id);
    for (const id of [...keyHeldBy.keys()]) if (id.startsWith(prefix)) keyHeldBy.delete(id);
    syncButtons(); syncKeys();
  }


  return {
    turn, hold, setMaster, releaseFrom,
    get master() { return master; },
    subscribe(listener) { subscribers.add(listener); return () => subscribers.delete(listener); },
    resync() { send({ type: "buttons", mask: btnMask }); send({ type: "keys", mask: keyMask }); },
    dispose() {
      scope.dispose(); btnHeldBy.clear(); keyHeldBy.clear(); latched = 0;
      syncButtons(); syncKeys(); subscribers.clear();
    },
  };
}
