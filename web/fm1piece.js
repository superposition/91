// SPDX-License-Identifier: GPL-3.0-only
// Sharing a piece (the pieces page, web/pieces): a saved piece out of a genre's bank into a .piece file, a code or a link, and back into a bank, with its
// own DX7 voice when it uses one. Nothing here touches the browser or the FM-1, so a node test (tests/web_piece_test.js) checks it, and checks a bank it
// builds by loading it in the simulator. The bank's layout is firmware/src/app/project.c's; the voices' object is ui.c's vox_t.

export const PC_HDR = 16, PC_REC = 376, PC_NPIECES = 10, PC_PRG = 6, PC_REC_PAR = 2 + 2 * 6 * 6, PC_RECIPE = 0xFF;   // IMG_HDR, IMG_REC, NPIECES, EN_PRG_LEN, REC_PAR, REC_RECIPE
export const PC_TAIL = PC_HDR + PC_NPIECES * PC_REC, PC_LEN = PC_TAIL + PC_NPIECES * PC_PRG;
export const PC_NUSER = 16, PC_VOX_LEN = 8 + PC_NUSER * 128;
import { getCodecs, codecWords, codecIntegers, codecConcat } from "./codec-runtime.mjs";
const pcCall = (op, args = [], blobs = []) => getCodecs().call(op, args, blobs);
const pcView = (bank, genre) => codecWords(pcCall(60, [genre], [bank]));
const crc32 = a => codecWords(pcCall(1, [], [a]))[0];
const pcJson = b => { try { return JSON.parse(new TextDecoder().decode(b)) || {}; } catch (_) { return {}; } };
const pcTake = (b, v, i) => b.slice(v[i], v[i] + v[i + 1]);
// ---- a genre's bank (the flash object): its header, and a piece out of it
// what is wrong with it for the pieces page (null: nothing), or "empty"
export function pcBankCheck(bank, genre) {
  if (!bank?.length) return "empty";
  try { const v = pcView(bank, genre); return v[0] && v[4] === 1 && v[5] === 1 ? null : "old"; } catch (_) { return "old"; }
}
export const pcSaved = bank => { try { const v = pcView(bank, 255); return v[0] && v[4] === 1 && v[5] === 1 ? v[3] : 0; } catch (_) { return 0; } };

// the piece in slot `slot` (0-9) of a genre's bank, as the file keeps it
export function pcFromBank(bank, genre, slot) {
  const bad = pcBankCheck(bank, genre);
  if (bad) throw new Error(bad === "empty" ? "nothing saved in this genre" : "this genre uses a different saved-piece layout: save a piece in NAIWAN 91 once, then try again");
  if (!(pcSaved(bank) >> slot & 1)) throw new Error("that piece is not saved");
  return pcDecode(pcCall(61, [genre, slot], [bank]));
}

// a setting of the piece (index p; lo: its lowest, params.json's min), or undefined when the piece predates it
export function pcGet(piece, p, lo) {
  const v = codecWords(pcCall(68, [p, lo, piece.npar], [piece.rec])); return v[0] ? v[1] | 0 : undefined;
}
export function pcSet(piece, p, lo, v) { piece.rec.set(pcCall(68, [p, lo, piece.npar, 1, v - lo], [piece.rec])); }

// ---- the Progression page's chord types (0.16.6, project.c knd_t "KND1": magic, then 3 bytes for each genre's ten pieces; OBJ_KINDS, its own object as
// the banks are full). A piece's three bytes (eight slots, three bits each; all 0: every slot the piece's own type) travel in the note's JSON as "k", so
// a page from before them reads the piece as it always did (without the types) and nothing about the file's layout changes.
export const PC_KND_LEN = 3, PC_KND_MAGIC = 0x31444E4B;          // EN_PRK_LEN, "KND1"
// 0.16.28: the object is the pieces' extras ("PCX1", project.c): magic, 16, the genres, 2 spare; then 16 bytes a piece: its 3 types, flags (bit 0:
// its FILTERS and SCREAM saved with it), those 7 settings (the FILTERS page's types: pad, bass, acid; SCREAM's drive, amount, wow, target), spare
export const PC_PX_MAGIC = 0x31584350, PC_PX_HDR = 8, PC_PX_LEN = 16, PC_PX_NFX = 7;
// 0.16.29: the pieces' bass steps, their own object ("BSQ1", project.c OBJ_BSTEP): magic, 16, the genres, 2 spare; then 16 bytes a piece (amb.h EN_BS_*)
export const PC_BS_MAGIC = 0x31515342, PC_BS_HDR = 8, PC_BS_LEN = 16;
// a piece's bass steps (null: none set)
export function pcBassOf(bytes, genre, slot) {
  const value = pcCall(63, [genre, slot, 2], [bytes]); return value.length ? value : null;
}
// the object with a piece's steps put in (null clears them), for a firmware with `ngenres` genres
export function pcBassWith(bs, genre, slot, steps, ngenres) {
  return pcCall(64, [genre, slot, ngenres, 2], [bs, null, null, steps]);
}
// a piece's types out of the object, either form (null: the piece's own in every slot, or no object)
export function pcKindsOf(bytes, genre, slot) {
  const value = pcCall(63, [genre, slot, 0], [bytes]); return value.length ? value : null;
}
// its FILTERS and SCREAM, when they were saved with it (null: the defaults)
export function pcFxOf(bytes, genre, slot) {
  const value = pcCall(63, [genre, slot, 1], [bytes]); return value.length ? value : null;
}
// the extras object (a firmware of 0.16.28 on: the about answer's version 2) with a piece's types and settings put in (null: none), for a firmware
// with `ngenres` genres; what else it held stays (an older KND1 object's types carried over)
export function pcExtrasWith(px, genre, slot, kinds, fx, ngenres) {
  return pcCall(64, [genre, slot, ngenres, 1], [px, kinds, fx]);
}
// the object with a piece's types put in (null clears them), for a firmware with `ngenres` genres; what else it held stays
export function pcKindsWith(knd, genre, slot, kinds, ngenres) {
  return pcCall(64, [genre, slot, ngenres, 0], [knd, kinds]);
}
const bytesOk = (a, n) => Array.isArray(a) && a.length === n && a.every((v) => Number.isInteger(v) && v >= 0 && v < 256);
const kindsOk = (k) => bytesOk(k, PC_KND_LEN);
const kindsIn = (meta, kinds, fx, bass) => {
  const m = { ...meta };
  if (kinds && kinds.some((v) => v)) m.k = Array.from(kinds);
  if (fx) m.x = Array.from(fx);                                   // (0.16.28) its FILTERS and SCREAM
  if (bass && bass.some((v) => v)) m.s = Array.from(bass);        // (0.16.29) its bass steps
  return m;
};
function kindsOut(meta) {                                         // the note without "k", "x" and "s", and what they carried (or null)
  const { k, x, s, ...rest } = meta || {};
  return { meta: rest, kinds: kindsOk(k) && k.some((v) => v) ? Uint8Array.from(k) : null, fx: bytesOk(x, PC_PX_NFX) ? Uint8Array.from(x) : null,
           bass: bytesOk(s, PC_BS_LEN) && s.some((v) => v) ? Uint8Array.from(s) : null };
}

// ---- the file: "P1PC", 1, genre, slot, flags (1: a voice), the settings a record holds (2), the record's length (2), the note's length (2), then the record,
// the progression, the voice (128 bytes, when flagged), a little JSON (title, by, note, names; "k" the chord types) and the CRC-32 of all before it
export function pcEncode(piece) {
  const meta = new TextEncoder().encode(JSON.stringify(kindsIn(piece.meta || {}, piece.kinds, piece.fx, piece.bass)));
  return pcCall(41, [piece.genre, piece.slot, piece.npar, piece.voice ? 1 : 0], [piece.rec, piece.prg, piece.voice, meta]);
}
export function pcDecode(buf) {
  let v;
  try { v = codecWords(pcCall(42, [], [buf])); }
  catch (e) { throw new Error(e.code === -2 || e.code === -1 && buf.length < 18 ? "not a compatible NAIWAN 91 piece" : e.code === -3 ? "this piece format is not supported: open the current NAIWAN 91 pieces page" : "the piece is damaged"); }
  const km = kindsOut(pcJson(pcTake(buf, v, 10)));
  return { genre: v[0], slot: v[1], npar: v[2], rec: pcTake(buf, v, 4), prg: pcTake(buf, v, 6), voice: v[9] ? pcTake(buf, v, 8) : null, ...km };
}

// ---- the code (and a link's #p=): the file, deflated, in base64url, after "P1."
const b64u = (u8) => { let s = ""; for (const c of u8) s += String.fromCharCode(c); return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, ""); };
const unb64u = (s) => Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/") + "===".slice((s.length + 3) % 4)), (c) => c.charCodeAt(0));
async function pipe(u8, stream) {
  const r = new Blob([u8]).stream().pipeThrough(stream);
  return new Uint8Array(await new Response(r).arrayBuffer());
}
export async function pcToCode(x) { return "P1." + b64u(await pipe(x.kind === "set" ? pcSetEncode(x) : pcEncode(x), new CompressionStream("deflate-raw"))); }
// a piece, or a set (kind "set") when the code holds one
export async function pcFromCode(text) {
  const m = /P1\.([A-Za-z0-9_-]+)/.exec(String(text || ""));
  if (!m) throw new Error("no piece code there");
  let raw;
  try { raw = await pipe(unb64u(m[1]), new DecompressionStream("deflate-raw")); } catch (_) { throw new Error("the code is damaged (cut short?)"); }
  return pcDecodeAny(raw);
}
export const pcDecodeAny = (buf) => (buf.length > 4 && buf[3] === 0x54 ? pcSetDecode(buf) : pcDecode(buf));   // "P1ST" or "P1PC"
export const pcHash = (rec) => crc32(rec);

// ---- a set: the set list (sixteen places, each a genre's piece or empty), whether Play goes through it, the SET page's Mix, the pieces in it that are
// stored (a factory piece needs nothing: it is the same everywhere), and their DX7 voices once each. "P1ST", 1, then 16 places (genre * 10 + piece + 1, 0
// empty), Play, Mix, the pieces, the voices, the note's length (2); each piece: genre, place, its voice (0xFF none), settings (2), record, progression; each
// voice 128 bytes; the note (JSON; "k": each piece's chord types by its place in the pieces, {"0": [a, b, c]}); the CRC-32
export function pcSetEncode(set) {
  const k = {}, x = {}, sb = {};
  set.pieces.forEach((pc, i) => {
    if (pc.kinds && pc.kinds.some((v) => v)) k[i] = Array.from(pc.kinds);
    if (pc.fx) x[i] = Array.from(pc.fx);
    if (pc.bass && pc.bass.some((v) => v)) sb[i] = Array.from(pc.bass);
  });
  const { k: _old, x: _oldx, s: _olds, ...m0 } = set.meta || {};
  const m1 = { ...m0 };
  if (Object.keys(k).length) m1.k = k;
  if (Object.keys(x).length) m1.x = x;
  if (Object.keys(sb).length) m1.s = sb;
  const meta = new TextEncoder().encode(JSON.stringify(m1));
  const ps = set.pieces;
  return pcCall(43, [set.on ? 1 : 0, set.mix || 0], [Uint8Array.from({length: 16}, (_, i) => set.list[i] || 0),
    codecIntegers(ps.flatMap(pc => [pc.genre, pc.slot, pc.npar, pc.vidx === undefined || pc.vidx < 0 ? 255 : pc.vidx])),
    codecConcat(ps.map(pc => pc.rec)), codecConcat(ps.map(pc => pc.prg)), codecConcat(set.voices), meta]);
}
export function pcSetDecode(buf) {
  let v;
  try { v = codecWords(pcCall(44, [], [buf])); }
  catch (e) { throw new Error(e.code === -2 || e.code === -1 && buf.length < 30 ? "not a compatible NAIWAN 91 set" : e.code === -3 ? "this set format is not supported: open the current NAIWAN 91 pieces page" : "the set is damaged"); }
  const list = Array.from(buf.slice(v[0], v[0] + 16)), on = !!v[1], mix = v[2], np = v[3], nv = v[4];
  const pieces = [], voices = [];
  for (let i = 0; i < np; i++) { const at = 7 + i * 6; pieces.push({genre: v[at], slot: v[at + 1], vidx: v[at + 2] === 255 ? -1 : v[at + 2], npar: v[at + 3], rec: buf.slice(v[at + 4], v[at + 4] + PC_REC), prg: buf.slice(v[at + 5], v[at + 5] + PC_PRG), voice: null, meta: {}}); }
  for (let i = 0; i < nv; i++) { const at = v[7 + np * 6 + i]; voices.push(buf.slice(at, at + 128)); }
  const { k, x, s, ...rest } = pcJson(pcTake(buf, v, 5));
  pieces.forEach((pc, i) => {
    if (pc.vidx >= 0 && pc.vidx < voices.length) pc.voice = voices[pc.vidx];
    pc.kinds = k && kindsOk(k[i]) && k[i].some(v => v) ? Uint8Array.from(k[i]) : null;
    pc.fx = x && bytesOk(x[i], PC_PX_NFX) ? Uint8Array.from(x[i]) : null;
    pc.bass = s && bytesOk(s[i], PC_BS_LEN) && s[i].some(v => v) ? Uint8Array.from(s[i]) : null;
  });
  return {kind: "set", list, on, mix, pieces, voices, meta: rest};
}
// the set list's places as genre and piece (null: empty)
export const pcSetPlaces = (list) => list.map((x) => (x ? { genre: Math.floor((x - 1) / 10), slot: (x - 1) % 10 } : null));

// the player's record (OBJ_GLOB, project.c glob_t: magic, tune, midi, theme, rsv, sl[16], sl_on, sl_rsv[3] = zoom, mix, -) with a set's list, Play and Mix;
// an empty one (or the old 12 bytes) takes the defaults given (tune, midi, theme)
export const PC_GLOB_LEN = 32;
export function pcGlobWith(glob, set, defs) {
  let values = [defs.tune, defs.midi, defs.theme, 0], tail = new Uint8Array(20);
  try { const raw = pcCall(69, [0], [glob]), v = codecWords(raw.slice(0, 16)); values = v; if (raw.length > 16) tail.set(raw.slice(16)); } catch (_) {}
  const list = tail.slice(0, 16); list.set(set.list.slice(0, 16));
  return pcCall(50, [set.on ? 1 : 0, tail[17], set.mix || 0, tail[19]], [codecIntegers(values, 2), list]);
}
export const pcGlobList = glob => { try { const b = pcCall(69, [0], [glob]); if (b.length !== 36) throw new Error(); return {list: Array.from(b.slice(16, 32)), on: !!b[32], mix: b[34]}; } catch (_) { return {list: new Array(16).fill(0), on: false, mix: 0}; } };

// a set's voices into the voices' object: each to a voice already loaded, else the next empty slot; the answer maps the set's voice i to a slot (-1: no room)
export function pcSetVoices(vox, voices) {
  let cur = vox && vox.length ? vox : new Uint8Array(0);
  const map = [];
  let changed = false;
  for (const v of voices) {
    const plan = pcVoicePlan(cur, v, PC_NUSER);
    if (plan.kind === "same") { map.push(plan.slot); continue; }
    if (plan.slot >= PC_NUSER) { map.push(-1); continue; }
    cur = pcVoicesWith(cur, v, plan.slot);
    map.push(plan.slot);
    changed = true;
  }
  return { vox: changed ? cur : null, map };
}

// ---- into a bank: the piece in slot `slot` of genre's bank (bank: what the FM-1 has, maybe empty), for a firmware with `npar` settings a piece
// (the about answer). Settings the piece predates are written as "the recipe's" (0xFF); so are those of the bank's other pieces when the bank was saved by a
// firmware with fewer. Settings the firmware does not know are dropped. The answer: the new bank, and what was lost or filled.
export function pcIntoBank(bank, genre, slot, piece, npar) {
  if (pcBankCheck(bank, genre) === "old") throw new Error("this genre on the FM-1 uses a different saved-piece layout: save a piece in NAIWAN 91 once, then try again");
  const raw = pcCall(62, [genre, slot, npar, piece.npar], [bank, piece.rec, piece.prg]), v = codecWords(raw.slice(0, 12));
  return { bank: raw.slice(12), dropped: v[0], filled: v[1], replaced: !!v[2] };
}

// ---- the DX7 voice: the pluck's sound (P_CM_SND) past the plain ones (nplain) and the built-in DX voices (nfmfact) is one of the player's 16 slots
export const pcVoiceName = v => String.fromCharCode(...Array.from(pcCall(69, [2], [v])).map(c => c < 32 || c > 126 ? 32 : c)).trimEnd() || "(no name)";
export function pcUserSlot(piece, iSnd, nplain, nfmfact) {
  const v = pcGet(piece, iSnd, 0);
  return v === undefined ? -1 : v - nplain - nfmfact;
}
// the voices' object (OBJ_VOICES; maybe empty): how many are loaded, and each
export function pcVoices(vox) {
  try { const bytes = pcCall(69, [1], [vox]); return Array.from({length: bytes.length / 128}, (_, i) => bytes.slice(i * 128, (i + 1) * 128)); } catch (_) { return []; }
}
const same = (a, b) => a.length === b.length && a.every((x, i) => x === b[i]);
// where the piece's voice goes: "same" (already loaded there: nothing to write), "free" (the next empty slot), or "conflict" (its own slot holds another
// voice: overwrite it, or take `free` when there is room, -1 when all 16 are in use)
export function pcVoicePlan(vox, voice, want) {
  const have = pcVoices(vox);
  const k = have.findIndex((v) => same(v, voice));
  if (k >= 0) return { kind: "same", slot: k };
  if (want < have.length) return { kind: "conflict", slot: want, other: pcVoiceName(have[want]), free: have.length < PC_NUSER ? have.length : -1 };
  return { kind: "free", slot: have.length };
}
// the voices' object with `voice` in `slot` (at most one past the loaded ones: slots fill in order)
export function pcVoicesWith(vox, voice, slot) {
  const have = pcVoices(vox);
  if (slot < 0 || slot > have.length || slot >= PC_NUSER) throw new Error("no room for the voice");
  have[slot] = voice; return pcCall(52, [], [codecConcat(have)]);
}

// the answer to "about" (F0 7D 50 45 ver npar(2) genres voices F7, firmware/src/backup_proto.h), or null
export function pcParseAbout(d) {
  try { const v = codecWords(pcCall(35, [], [d])); return v[0] === 0x45 ? {ver: v[1], npar: v[2], genres: v[3], voices: v[4]} : null; } catch (_) { return null; }
}
export const pcAboutMsg = () => Array.from(pcCall(92, [3]));
