// SPDX-License-Identifier: GPL-3.0-only
import { MAP_KEY } from "./storage.mjs";
import { createLifetime } from "./lifecycle.mjs";

export function createMidi(panel, controls, { storage, window = globalThis.window, document = globalThis.document, navigator = globalThis.navigator }) {
  const scope = createLifetime();
  let rows = createLifetime();
  const attached = new Map();
  let connecting = null;
  const { device, knobs, buttons } = panel;
  const { hold, setMaster, releaseFrom, turn } = controls;
  const NOTE = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
  const midiUI = { on: document.getElementById("midiOn"), status: document.getElementById("midiStatus"), learn: document.getElementById("learn"),
                   unlearn: document.getElementById("unlearn"), maps: document.getElementById("maps") };
  let midi = null, learning = false, picked = null, remote = null, sentB = 0, sentK = 0, ackTimer = 0, maps = {};
  maps = storage.loadMaps();
  const knobByRole = (r) => Object.values(knobs).find((k) => k.role === r);
  function targetName(t, m) {
    if (t === "m") return "MASTER";
    if (t[0] === "p") return `${(m && m.name) || "Setting " + t.slice(1)} (a setting, on the FM-1)`;   // (0.16.18: learned on the editor page)
    if (t[0] === "e") return knobByRole(+t.slice(1)).name.replace("KNOB", "KNOB ");
    if (t[0] === "b") return Object.values(buttons).find((b) => b.bit === +t.slice(1)).name;
    const k = +t.slice(1), n = 53 + k;
    return `Key ${NOTE[n % 12]}${Math.floor(n / 12) - 1}`;
  }
  const srcName = (id) => { const [ty, ch, n] = id.split(":"); return `${ty === "n" ? "Note " + NOTE[n % 12] + (Math.floor(n / 12) - 1) : "CC " + n} ch ${+ch + 1}`; };
  function saveMaps() { storage.saveMaps(maps); drawMaps(); }
  function drawMaps() {
    rows.dispose(); rows = createLifetime();
    midiUI.maps.textContent = "";
    for (const [id, m] of Object.entries(maps)) {
      const li = document.createElement("li");
      li.innerHTML = `<span>${srcName(id)}</span> → <b></b>`;
      li.querySelector("b").textContent = targetName(m.t, m);
      if (id[0] === "c" && (m.t[0] === "e" || m.t[0] === "p" || m.t === "m")) {
        const sel = document.createElement("select");
        for (const [v, l] of [["abs", "absolute"], ["rel64", "relative (64 ± n)"], ["rel2c", "relative (1 up, 127 down)"]]) sel.add(new window.Option(l, v, false, m.mode === v));
        rows.on(sel, "change", () => { m.mode = sel.value; saveMaps(); });
        sel.setAttribute("aria-label", "How the controller sends this knob");
        li.append(sel);
      }
      const x = document.createElement("button");
      x.type = "button"; x.textContent = "✕"; x.title = "Forget this one"; x.setAttribute("aria-label", `Forget ${srcName(id)}`);
      rows.on(x, "click", () => { delete maps[id]; saveMaps(); });
      li.append(x);
      midiUI.maps.append(li);
    }
    midiUI.unlearn.hidden = !midi || !Object.keys(maps).length;
    document.getElementById("mapsave").hidden = document.getElementById("mapsend").hidden = !Object.keys(maps).length;
  }

  function outFor(input) {                   // the FM-1's own output: the same port name
    if (!midi) return null;
    for (const o of midi.outputs.values()) if (o.name === input.name && o.state === "connected") return o;
    for (const o of midi.outputs.values()) if (input.manufacturer && o.manufacturer === input.manufacturer && o.state === "connected") return o;
    return null;
  }
  function midiLights(b, k, all) {           // our lights to the FM-1's panel, as they change
    if (!remote || !remote.out) return;
    const o = remote.out, cb = all ? 0x3FFF : (b ^ sentB), ck = all ? 0x7FFFFFF : (k ^ sentK);
    try {
      for (let i = 0; i < 14; i++) if (cb & (1 << i)) o.send([0xBF, 102 + i, b & (1 << i) ? 127 : 0]);
      for (let i = 0; i < 27; i++) if (ck & (1 << i)) o.send(k & (1 << i) ? [0x9F, 48 + i, 100] : [0x8F, 48 + i, 0]);
    } catch { /* unplugged mid-send */ }
    sentB = b; sentK = k;
  }
  function remoteStart(input) {
    if (remote) remoteStop();
    const out = outFor(input);
    remote = { input, out, ownsOutput: out?.connection === "closed" };
    document.getElementById("device").classList.add("remote");
    const ack = () => { try { remote && remote.out && remote.out.send([0xBF, 117, 127]); } catch {} };
    ack();
    clearInterval(ackTimer);
    ackTimer = setInterval(ack, 1000);
    const m = window.ambLast;
    if (m) midiLights(m.buttons, m.keys, true);
    midiStatusText();
  }
  function remoteStop() {
    if (!remote) return;
    clearInterval(ackTimer);
    try { remote.out && remote.out.send([0xBF, 117, 0]); } catch {}
    if (remote.ownsOutput) { try { remote.out.close()?.catch(() => {}); } catch {} }
    remote = null;
    releaseFrom("r");
    document.getElementById("device").classList.remove("remote");
    midiStatusText();
  }

  function onMidi(e, input) {
    const [st, d1 = 0, d2 = 0] = e.data;
    if (st >= 0xF0) return;                  // clock, SysEx: not ours
    const type = st & 0xF0, ch = st & 15, on = type === 0x90 && d2 > 0, off = type === 0x80 || (type === 0x90 && d2 === 0);
    if (type !== 0x90 && type !== 0x80 && type !== 0xB0) return;
    const isRemote = ch === 15 && ((type === 0xB0 && (d1 === 116 || d1 === 7 || (d1 >= 85 && d1 <= 91) || (d1 >= 102 && d1 <= 115))) ||
                                   (type !== 0xB0 && d1 >= 48 && d1 < 75));
    // the HYP-1 remote
    if (ch === 15 && type === 0xB0 && d1 === 116) { if (d2) { if (!remote || remote.input !== input) remoteStart(input); } else remoteStop(); return; }
    if (learning && !(remote && remote.input === input && isRemote)) {
      if (type === 0x80 || (type === 0x90 && !on)) return;
      if (type === 0xB0 && d1 >= 120) return;
      if (!picked) { midiUI.status.textContent = `Heard ${srcName((type === 0xB0 ? "c" : "n") + ":" + ch + ":" + d1)} from ${input.name}. Click a control here first, then move yours.`; return; }   // all notes off, reset and the like: a device sends them when its MIDI port changes, not a control (0.16.12)
      const t = picked.dataset.t, id = (type === 0xB0 ? "c" : "n") + ":" + ch + ":" + d1;
      if (type !== 0xB0 && (t[0] === "e" || t === "m")) { midiUI.status.textContent = "A knob needs a knob or slider (a CC), not a key."; return; }
      for (const [k, m] of Object.entries(maps)) if (m.t === t) delete maps[k];     // one source per control
      maps[id] = { t, mode: type === 0xB0 && (t[0] === "e" || t === "m") ? (d2 === 63 || d2 === 65 ? "rel64" : "abs") : undefined };
      picked.classList.remove("picked"); picked = null;
      midiUI.status.textContent = `${srcName(id)} → ${targetName(t)}. Click another control, or MIDI learn to finish.`;
      saveMaps();
      return;
    }
    // learned controls
    const id = (type === 0xB0 ? "c" : "n") + ":" + ch + ":" + d1, m = maps[id];
    if (m && m.t[0] === "p") return;          // a setting (the editor's): the FM-1 and the editor page play it, not this panel
    if (m) {
      if (type === 0xB0) {
        if (m.t[0] === "b" || m.t[0] === "k") { hold("m" + id, m.t, d2 >= 64); return; }
        let n = 0;
        if (m.mode === "rel64") n = d2 - 64;
        else if (m.mode === "rel2c") n = d2 < 64 ? d2 : d2 - 128;
        else if (m.t === "m") { setMaster(d2 * 4096 / 127); m.last = d2; return; }
        else { m.acc = (m.acc || 0) + (m.last === undefined ? 0 : d2 - m.last); m.last = d2; n = Math.trunc(m.acc / 2); m.acc -= n * 2; }
        if (m.t === "m") setMaster(controls.master + n * 64); else turn(knobByRole(+m.t.slice(1)), n);
      } else if (m.t[0] === "b" || m.t[0] === "k") hold("m" + id, m.t, on);
      return;
    }
    // the remote's panel
    if (remote && remote.input === input && isRemote) {
      if (type === 0xB0) {
        if (d1 === 7) setMaster(d2 * 4096 / 127);
        else if (d1 >= 102) hold("rb" + (d1 - 102), "b" + (d1 - 102), d2 >= 64);
        else turn(knobByRole(d1 - 85), d2 - 64);
      } else hold("rk" + (d1 - 48), "k" + (d1 - 48), on);
      return;
    }
    // any other note: the key of its pitch, folded into F3 .. G5
    if (type !== 0xB0) {
      let n = d1;
      while (n < 53) n += 12;
      while (n > 79) n -= 12;
      hold("n" + input.id + ":" + d1, "k" + (n - 53), on);
    }
  }

  function midiStatusText() {
    if (!midi) return;
    const ins = [...midi.inputs.values()].filter((i) => i.state === "connected");
    if (remote) midiUI.status.textContent = `${remote.input.name}: NAIWAN 91 remote, playing this one${remote.out ? "" : " (its lights: no matching output)"}`;
    else if (learning) midiUI.status.textContent = picked ? `${targetName(picked.dataset.t)}: now move or press a control on your controller.` : "Click a control here to learn it.";
    else midiUI.status.textContent = ins.length ? `Listening to ${ins.map((i) => i.name).join(", ")}.` : "No MIDI devices. Plug one in.";
  }
  function attachInputs() {
    const inputs = new Set([...midi.inputs.values()].filter((input) => input.state === "connected"));
    for (const [input, detach] of attached) if (!inputs.has(input)) { detach(); attached.delete(input); }
    for (const input of inputs) if (!attached.has(input)) {
      const owned = input.connection === "closed";
      const off = scope.on(input, "midimessage", (event) => onMidi(event, input));
      attached.set(input, () => {
        off(); releaseFrom("n" + input.id + ":"); releaseFrom("m");
        if (owned) { try { input.close()?.catch(() => {}); } catch {} }
      });
    }
    if (remote && remote.input.state !== "connected") remoteStop();
    if (remote && !remote.out) {
      remote.out = outFor(remote.input);
      remote.ownsOutput = remote.out?.connection === "closed";
    }
    midiStatusText();
  }
  function midiConnect(quiet) {
    if (scope.disposed || midi) return Promise.resolve();
    if (connecting) return connecting;
    if (!navigator.requestMIDIAccess) { if (!quiet) midiUI.status.textContent = "This browser has no Web MIDI: use Chrome, Edge or Firefox."; return; }
    connecting = (async () => { try {
      midi = await navigator.requestMIDIAccess({ sysex: false });
      if (scope.disposed) { midi = null; return; }
      scope.on(midi, "statechange", attachInputs);
      attachInputs();
      midiUI.on.hidden = true;
      midiUI.learn.hidden = false;
      drawMaps();
    } catch (err) {
      if (!quiet && !scope.disposed) midiUI.status.textContent = "MIDI was not allowed" + (err && err.message ? ": " + err.message : ".");
    } finally { connecting = null; } })();
    return connecting;
  }
  scope.on(midiUI.on, "click", () => midiConnect(false));
  scope.on(midiUI.learn, "click", () => {
    learning = !learning;
    midiUI.learn.setAttribute("aria-pressed", learning);
    midiUI.learn.textContent = learning ? "Done learning" : "MIDI learn";
    document.getElementById("device").classList.toggle("learning", learning);
    if (picked) { picked.classList.remove("picked"); picked = null; }
    midiStatusText();
  });
  scope.on(midiUI.unlearn, "click", () => { maps = {}; releaseFrom("m"); saveMaps(); });
  scope.on(document.getElementById("mapsave"), "click", () => {   // the learned list as a .perfmap file (0.16.11; the Install page reads the same)
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([JSON.stringify({ perfmap: 1, maps: Object.fromEntries(Object.entries(maps).map(([k, m]) => [k, { t: m.t, mode: m.mode, name: m.name }])) }, null, 1)], { type: "application/json" }));
    a.download = "perf1-midi-map.perfmap";
    a.click();
    const url = a.href;
    let timer;
    const revoke = scope.add(() => { clearTimeout(timer); URL.revokeObjectURL(url); });
    timer = setTimeout(revoke, 0);
  });
  scope.on(document.getElementById("mapload"), "change", async (e) => {
    const f = e.target.files[0];
    if (!f) return;
    try { const j = JSON.parse(await f.text()); if (scope.disposed) return; if (!j || j.perfmap !== 1 || !j.maps || typeof j.maps !== "object") throw 0; releaseFrom("m"); maps = j.maps; saveMaps(); midiUI.status.textContent = "Loaded " + f.name + "."; }
    catch { if (!scope.disposed) midiUI.status.textContent = "That is not a MIDI map file."; }
    e.target.value = "";
  });
  scope.on(window, "storage", (e) => { if (e.key === MAP_KEY) { try { maps = JSON.parse(e.newValue || "{}") || {}; } catch { maps = {}; } drawMaps(); } });   // (read back from the FM-1 on the Install page)
  scope.on(document.getElementById("device"), "pointerdown", (e) => {    // learning: a click picks, nothing pressed
    if (!learning) return;
    const h = e.target.closest && e.target.closest(".hit");
    if (!h || !h.dataset.t) return;
    e.preventDefault(); e.stopPropagation();
    if (picked) picked.classList.remove("picked");
    picked = h; h.classList.add("picked");
    midiStatusText();
  }, true);
  drawMaps();
  if (navigator.permissions && navigator.requestMIDIAccess)                       // allowed before: connect at once
    navigator.permissions.query({ name: "midi" }).then((p) => { if (!scope.disposed && p.state === "granted") midiConnect(true); }).catch(() => {});


  scope.add(() => {
    remoteStop(); clearInterval(ackTimer); rows.dispose();
    for (const detach of attached.values()) detach();
    attached.clear(); releaseFrom("m");
    learning = false; picked?.classList.remove("picked"); picked = null;
    device.classList.remove("learning");
    midiUI.on.hidden = false; midiUI.learn.hidden = midiUI.unlearn.hidden = true;
    midiUI.learn.textContent = "MIDI learn"; midiUI.learn.setAttribute("aria-pressed", "false");
    midiUI.status.textContent = "Not connected.";
    midiUI.maps.textContent = ""; midi = null;
  });
  return { connect: midiConnect, lights: midiLights, dispose() { scope.dispose(); } };
}
