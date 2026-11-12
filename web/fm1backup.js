// SPDX-License-Identifier: GPL-3.0-only
// Backup and restore of PERF-1's stored data over USB MIDI (the installer page): the messages, the file, and nothing that touches the browser, so a node test can check
// them against the firmware's (firmware/src/backup_proto.h, which describes the protocol).

import { getCodecs, codecWords, codecIntegers, codecConcat } from "./codec-runtime.mjs";
import { DEVICE_DATA } from "./device-data.js";
const bkCall = (op, args = [], blobs = []) => getCodecs().call(op, args, blobs);
export const BK_CHUNK = DEVICE_DATA.backup.chunk, BK_FRAME = DEVICE_DATA.backup.frame, BK_MAX = DEVICE_DATA.backup.max;

// the flash objects worth keeping (plat.h OBJ_*; the panel's calibration, 1, is the unit's own and is never saved), in the order they are read and written
const BK_GENRES = DEVICE_DATA.genres, BK_IDS = DEVICE_DATA.objects;
export const BK_OBJECTS = [
  { id: BK_IDS.settings, name: "settings" }, { id: BK_IDS.voices, name: "voices" }, { id: BK_IDS.globals, name: "globals" },
  ...BK_GENRES.map((name, i) => ({ id: BK_IDS.genres[i], name })),
  { id: BK_IDS.extras, name: "piece extras" }, { id: BK_IDS.map, name: "MIDI map" }, { id: BK_IDS.bass, name: "bass steps" },
];

// New objects are requested only after the separate Jam capability probe succeeds.
export const BK_JAM_OBJECTS = [...Array.from({length:BK_IDS.jamLast-BK_IDS.jamFirst+1},(_,i)=>({id:BK_IDS.jamFirst+i,name:"Jam pack "+(i+1)})),{id:BK_IDS.session,name:"Jam session"}];
export const bkJamCapsMsg = () => Array.from(bkCall(85, [1, 0]));
export function bkJamCaps(d) {
  try { const raw = bkCall(86, [], [d]), h = codecWords(raw.slice(0, 12)), b = raw.slice(12); return h[0] === 1 && h[1] === 0 && h[2] === 0 && b.length === 6 && b[0] === 1 && b[3] === 4 && b[4] === 26 && b[5] === 30 ? {version: 1, slots: 4, max: b[1] | b[2] << 8} : null; } catch (_) { return null; }
}
export const bkObjects = (jam) => jam ? [...BK_OBJECTS,...BK_JAM_OBJECTS] : BK_OBJECTS;

// the player's MIDI map (0.16.11, ui.c mm_*): the emulator's learned controls ({"c:ch:cc" | "n:ch:note": {t: "b<button>" | "k<key>" | "e<knob>" | "m", mode}})
// as the firmware keeps them: "MAP1", the count, 3 pad bytes, then 48 entries of 3 bytes (the channel | 0x10 a CC | how a knob counts << 5, the number,
// the control: a button 0-13, 32 + a key, 64 + a knob). MASTER is the hardware's alone: not sent. The .perfmap file is the learned list as JSON.
// A setting (0.16.18, learned on the editor page): {t: "p<parameter>", mode, name}, a CC only; its first byte has 0x80 set and the control is the parameter
// (0 .. 175). A map with any setting is "MAP2" (a firmware before 0.16.18 refuses it whole); one without stays "MAP1", as before.
export const MM_OBJ = BK_IDS.map, MM_MAX = 48, MM_LEN = 8 + 3 * MM_MAX, MM_NPAR = DEVICE_DATA.parameters.count;
const MM_MODE = { abs: 0, rel64: 1, rel2c: 2 };
export function mmEncode(maps) {                                     // the bytes, and how many controls went in (and how many were left out)
  const entries = [];
  let n = 0, left = 0, set = 0;
  for (const [id, m] of Object.entries(maps || {})) {
    const [ty, ch, num] = id.split(":"), t = String((m && m.t) || ""), par = t[0] === "p" ? +t.slice(1) : -1;
    const ctl = t[0] === "b" ? +t.slice(1) : t[0] === "k" ? 32 + +t.slice(1) : t[0] === "e" ? 64 + +t.slice(1) : par >= 0 && par < MM_NPAR && ty === "c" ? par : -1;
    if (!(ctl >= 0) || !(ty === "c" || ty === "n") || n >= MM_MAX) { left++; continue; }
    entries.push([(+ch & 15) | (ty === "c" ? 0x10 : 0) | ((MM_MODE[m.mode] || 0) << 5) | (par >= 0 ? 0x80 : 0), +num & 127, ctl]); n++;
    if (par >= 0) set++;
  }
  return { bytes: bkCall(55, [set ? 2 : 1], [Uint8Array.from(entries.flat())]), n, left };
}
export function mmDecode(b) {                                        // the learned list back, or null when it is not a map
  let raw, head;
  try { raw = bkCall(65, [], [b]); head = codecWords(raw.slice(0, 8)); } catch (_) { return null; }
  const maps = {}, modes = ["abs", "rel64", "rel2c"];
  for (let i = 0; i < head[1]; i++) {
    const [a, num, ctl] = raw.subarray(11 + 3 * i, 14 + 3 * i), cc = a & 0x10, set = a & 0x80;
    const t = set ? "p" + ctl : ctl < 32 ? "b" + ctl : ctl < 64 ? "k" + (ctl - 32) : "e" + (ctl - 64);
    maps[(cc ? "c" : "n") + ":" + (a & 15) + ":" + num] = cc && (set || ctl >= 64) ? { t, mode: modes[(a >> 5) & 3] || "abs" } : { t };
  }
  return maps;
}
export const bkName = (id) => ([...BK_OBJECTS,...BK_JAM_OBJECTS].find((o) => o.id === id) || { name: "object " + id }).name;

// the checksum: 128 minus the sum of the data bytes, modulo 128
export function bkSum(a) { return codecWords(bkCall(84, [], [a]))[0];
}

// a chunk for the FM-1 to write: raw bytes idx * BK_CHUNK .. of an object len bytes long (7D 50 42 03 obj idx(2) len(2) n(2) [2n nibbles] chk)
export function bkChunkMsg(obj, idx, len, bytes) { return Array.from(bkCall(31, [obj, idx, len], [bytes]));
}
export const bkReadMsg = (obj) => Array.from(bkCall(92, [0, obj]));
export const bkCommitMsg = (obj, len) => Array.from(bkCall(92, [1, obj, len]));
export const bkRestartMsg = () => Array.from(bkCall(92, [2]));

// a frame from the FM-1 (F0 7D 50 43 obj idx(2) len(2) n(2) [2n nibbles] chk F7): its object, place, the object's length and its bytes; null when it is damaged
export function bkParseFrame(d) {
  try { const raw = bkCall(30, [], [d]), v = codecWords(raw.slice(0, 12)); return {obj: v[0], idx: v[1], len: v[2], data: raw.slice(12)}; } catch (_) { return null; }
}

// an answer (F0 7D 50 44 obj idx status F7): the chunk's place, or 0x7F for the write; status 0 is well
export function bkParseAck(d) {
  try { const v = codecWords(bkCall(35, [], [d])); return v[0] === 0x44 ? {obj: v[1], idx: v[2], status: v[3]} : null; } catch (_) { return null; }
}

export function bkCrc32(a) { return codecWords(bkCall(1, [], [a]))[0];
}

// the file: "P1BK", version 1, the number of objects, a little JSON about it, then each object (id, length, bytes), and the CRC-32 of all that comes before
export function bkEncode(objs, info) {
  let at = 0; const config = objs.flatMap(o => { const v = [o.id, at, o.data.length]; at += o.data.length; return v; });
  return bkCall(45, [], [new TextEncoder().encode(JSON.stringify(info || {})), codecIntegers(config), codecConcat(objs.map(o => o.data))]);
}
export function bkDecode(buf) {
  let v;
  try { v = codecWords(bkCall(46, [], [buf])); } catch (e) { throw new Error(e.code === -2 || e.code === -1 && buf.length < 12 ? "not a compatible " + DEVICE_DATA.product.name + " backup" : e.code === -3 ? "a backup of a newer kind" : "the backup is damaged"); }
  let info = {}; try { info = JSON.parse(new TextDecoder().decode(buf.subarray(v[0], v[0] + v[1]))); } catch (_) {}
  const objs = Array.from({length: v[2]}, (_, i) => { const p = 5 + i * 3; return {id: v[p], data: buf.slice(v[p + 1], v[p + 1] + v[p + 2])}; });
  return {info, objs};
}
