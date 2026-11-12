// SPDX-License-Identifier: GPL-3.0-only
// Keep these keys and the byte/base64 encoding compatible with existing saves.
export const STORE_KEY = "hyp1-emu-store";
export const MAP_KEY = "hyp1-emu-midimap";

export function createStorage(getStorage = () => globalThis.localStorage) {
  function read(key) {
    try { return JSON.parse(getStorage().getItem(key) || "{}"); } catch { return {}; }
  }
  function write(key, value) {
    try { getStorage().setItem(key, JSON.stringify(value)); } catch { /* this session only */ }
  }
  return {
    loadStore() {
      try {
        const out = {};
        for (const [obj, base64] of Object.entries(read(STORE_KEY))) {
          out[obj] = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
        }
        return out;
      } catch { return {}; }
    },
    saveStore(store) {
      const raw = {};
      for (const [obj, bytes] of Object.entries(store)) {
        let binary = "";
        for (const value of bytes) binary += String.fromCharCode(value);
        raw[obj] = btoa(binary);
      }
      write(STORE_KEY, raw);
    },
    forgetStore() { try { getStorage().removeItem(STORE_KEY); } catch {} },
    loadMaps() { return read(MAP_KEY) || {}; },
    saveMaps(maps) { write(MAP_KEY, maps); },
  };
}
