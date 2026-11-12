// SPDX-License-Identifier: GPL-3.0-only
// Browser-independent capability/state helpers used by all connected tools.
export const DEVICE_TEXT = {
  en: {
    unsupported: "This FM-1 did not report the requested {product} feature. Check its firmware on the Install page.",
    noanswer: "The FM-1 did not answer. Check the USB connection and try again.",
    disconnected: "The FM-1 disconnected. Reconnect it to continue with {product}.",
    connected: "{product}: connected to ", connecting: "Connecting to {product}…", idle: "{product}: not connected.",
    busy: "{product} is already reading or writing. Wait for it to finish.",
    empty: "Nothing is saved on this {product} yet.",
    browser: "This browser cannot connect to the FM-1. Open {product} in Chrome or Edge.",
    insecure: "Connecting an FM-1 needs a secure page (HTTPS, or localhost on the computer serving it). You can still play {product} here without MIDI.",
    denied: "MIDI (SysEx) was not allowed. Allow MIDI in the site settings and try {product} again.",
    notfound: "FM-1 not found. Check the data USB cable and close other apps using its MIDI port.",
    mapSettings: "This firmware did not accept a MIDI map with settings. Check its mapping support on the Install page.",
    badBackup: "This is not a compatible {product} backup, or the file is damaged.",
    badMap: "This is not a compatible {product} MIDI map.",
    badImport: "This file is not a compatible {product} piece or set.",
  },
  ja: {
    unsupported: "この FM-1 は必要な {product} の機能を応答しません。Install ページでファームウェアを確認してください。",
    noanswer: "FM-1 から返事がありません。USB 接続を確かめ、もう一度試してください。",
    disconnected: "FM-1 の接続が切れました。つなぎ直して {product} を続けてください。",
    connected: "{product}: 接続しました: ", connecting: "{product} に接続中…", idle: "{product}: 未接続。",
    busy: "{product} は読み書き中です。完了するまでお待ちください。",
    empty: "この {product} にはまだ何も保存されていません。",
    browser: "このブラウザーでは FM-1 に接続できません。Chrome または Edge で {product} を開いてください。",
    insecure: "FM-1 への接続には HTTPS、または配信元のパソコンの localhost が必要です。MIDI なしで {product} をこのまま演奏できます。",
    denied: "MIDI（SysEx）が許可されませんでした。サイト設定で MIDI を許可し、{product} をもう一度試してください。",
    notfound: "FM-1 が見つかりません。データ通信用 USB ケーブルを確認し、MIDI を使うほかのアプリを閉じてください。",
    mapSettings: "このファームウェアは設定付き MIDI マップを受け付けませんでした。Install ページで対応機能を確認してください。",
    badBackup: "対応する {product} のバックアップではないか、ファイルが壊れています。",
    badMap: "対応する {product} の MIDI マップではありません。",
    badImport: "対応する {product} のピースまたはセットではありません。",
  },
};

export function deviceText(data, language, key, detail = "") {
  return (DEVICE_TEXT[language]?.[key] || DEVICE_TEXT.en[key]).replaceAll("{product}", data.product.name) + detail;
}

export function deviceCapabilities(about, jam = null) {
  const valid = about && Number.isInteger(about.ver) && about.ver >= 1 && Number.isInteger(about.npar) && about.npar > 0;
  return { backup: !!valid, parameters: valid ? about.npar : 0, genres: valid ? about.genres : 0,
    extras: !!valid && about.ver >= 2, bass: !!valid && about.ver >= 3, jam: !!jam,
    // The about response never advertised MAP2; do not infer it from release numbers.
    mapSettings: null };
}

export function deviceParseAbout(bytes) {
  if (bytes.length !== 10 || bytes[0] !== 240 || bytes[1] !== 125 || bytes[2] !== 80 || bytes[3] !== 69 || bytes[9] !== 247 || bytes.slice(4, 9).some((v) => v > 127)) return null;
  const about = { ver: bytes[4], npar: bytes[5] * 128 + bytes[6], genres: bytes[7], voices: bytes[8] };
  return deviceCapabilities(about).backup ? about : null;
}

export function createDeviceClient(data, { requestMidi, onMessage = () => {}, onDisconnect = () => {}, language = () => "en", isBusy = () => false, secureContext = globalThis.isSecureContext !== false } = {}) {
  let current = null, opening = null, cancelOpen = null, busy = false, generation = 0;
  const text = (key, detail) => deviceText(data, language(), key, detail);
  const error = (code) => Object.assign(new Error(text(code)), { code });
  async function access() {
    if (!requestMidi) throw error(secureContext ? "browser" : "insecure");
    try { return await requestMidi(); }
    catch (failure) { throw error(!secureContext && failure?.name === "SecurityError" ? "insecure" : "denied"); }
  }
  function close(notify = false) {
    generation++;
    cancelOpen?.(); cancelOpen = null;
    const previous = current; current = null;
    if (!previous) return;
    previous.detach();
    for (const cancel of [...previous.pending]) cancel(error("disconnected"));
    previous.pending.clear();
    for (const port of previous.owned) { try { port.close()?.catch(() => {}); } catch {} }
    if (notify) onDisconnect(previous, error("disconnected"));
  }
  async function open() {
    if (current && current.inp.state !== "disconnected" && current.out.state !== "disconnected") return current;
    if (current) close();
    if (opening) return opening;
    opening = (async () => {
      const token = generation;
      let midi;
      const canceled = new Promise((resolve, reject) => { cancelOpen = () => reject(error("disconnected")); });
      midi = await Promise.race([access(), canceled]);
      if (token !== generation) throw error("disconnected");
      const pick = (ports) => [...ports.values()].find((port) => /fm-?1/i.test(port.name) && !/ota/i.test(port.name) && port.state !== "disconnected");
      const inp = pick(midi.inputs), out = pick(midi.outputs);
      if (!inp || !out) throw error("notfound");
      const listeners = new Set();
      const session = { inp, out, pending: new Set(), owned: [inp, out].filter((port) => port.connection === "closed"),
        listen(fn) { listeners.add(fn); return () => listeners.delete(fn); } };
      const receive = (event) => { for (const fn of [...listeners]) fn(event.data); onMessage(event, session); };
      const changed = () => { if (inp.state === "disconnected" || out.state === "disconnected") close(true); };
      inp.addEventListener("midimessage", receive); midi.addEventListener("statechange", changed);
      session.detach = () => { inp.removeEventListener("midimessage", receive); midi.removeEventListener("statechange", changed); listeners.clear(); };
      current = session;
      return session;
    })();
    try { return await opening; } finally { opening = null; cancelOpen = null; }
  }
  function request(session, message, parse, timeout = 1500) {
    if (session !== current || session.inp.state === "disconnected" || session.out.state === "disconnected") return Promise.reject(error("disconnected"));
    return new Promise((resolve, reject) => {
      let timer, off = () => {};
      const finish = (value, failure) => { clearTimeout(timer); off(); session.pending.delete(cancel); failure ? reject(failure) : resolve(value); };
      const cancel = (failure) => finish(null, failure);
      session.pending.add(cancel);
      off = session.listen((bytes) => { const result = parse(bytes); if (result !== undefined) finish(result); });
      timer = setTimeout(() => finish(null), timeout);
      try { session.out.send(message); } catch { cancel(error("disconnected")); }
    });
  }
  return { text, error, access, open, close, request,
    get busy() { return busy; },
    async run(action) { if (busy || isBusy()) throw error("busy"); busy = true; try { return await action(); } finally { busy = false; } },
    about: (session, timeout) => request(session, [240,125,80,66,6,247], (bytes) => deviceParseAbout(bytes) || undefined, timeout),
  };
}

export function deviceMapValid(file, parameterCount) {
  if (!file || file.perfmap !== 1 || !file.maps || typeof file.maps !== "object" || Array.isArray(file.maps)) return false;
  return Object.entries(file.maps).every(([source, mapping]) => {
    const match = /^(c|n):(\d+):(\d+)$/.exec(source);
    if (!match || +match[2] > 15 || +match[3] > 127 || !mapping || typeof mapping !== "object") return false;
    if (mapping.mode && !["abs", "rel64", "rel2c"].includes(mapping.mode)) return false;
    const target = /^(m|[bkep]\d+)$/.test(mapping.t || "") ? mapping.t : "";
    return target === "m" || (target[0] === "b" && +target.slice(1) < 14) || (target[0] === "k" && +target.slice(1) < 27) || (target[0] === "e" && +target.slice(1) < 7) || (target[0] === "p" && match[1] === "c" && +target.slice(1) < parameterCount);
  });
}

export function deviceAction(client, action, report) {
  return (...args) => client.run(() => action(...args)).catch((error) => report(error.message));
}

export function deviceParseParameters(bytes, count) {
  if (bytes.length < 8 || bytes[0] !== 240 || bytes[1] !== 125 || bytes[2] !== 80 || bytes[3] !== 81 || bytes.at(-1) !== 247) return null;
  const first = bytes[4] * 128 + bytes[5], length = bytes[6];
  if (!length || bytes.length !== 8 + length * 2 || bytes.slice(4, -1).some((value) => value > 127)) return null;
  return Array.from({ length: Math.max(0, Math.min(length, count - first)) }, (_, index) => [first + index, bytes[7 + index * 2] * 128 + bytes[8 + index * 2] - 8192]);
}

export function deviceReadParameters(client, session, count, apply, timeout) {
  const seen = new Set();
  return client.request(session, [240,125,80,80,2,247], (bytes) => {
    const entries = deviceParseParameters(bytes, count);
    if (!entries) return undefined;
    apply(entries);
    for (const [index] of entries) seen.add(index);
    return seen.size === count ? count : undefined;
  }, timeout);
}

export function deviceParseVoice(bytes, slots, requested = bytes[4]) {
  if (bytes[0] !== 240 || bytes[1] !== 125 || bytes[2] !== 80 || bytes[3] !== 88 || bytes.at(-1) !== 247 || bytes[4] >= slots || bytes[5] > slots) return null;
  if (bytes[4] !== requested) return null;
  const filled = bytes[4] < bytes[5];
  if (bytes.length !== (filled ? 136 : 7) || bytes.slice(4, -1).some((value) => value > 127)) return null;
  if (filled && bytes.slice(6, -1).reduce((sum, value) => sum + value, 0) % 128) return null;
  return { slot: bytes[4], loaded: bytes[5], data: filled ? Uint8Array.from(bytes.slice(6, 134)) : null };
}
