// SPDX-License-Identifier: GPL-3.0-only
// Synchronous, bounded binary services. The instrument module remains separate.
const CODEC_BASE64 = "/*CODECS_BASE64*/";
let codecBytes;
// NODE_ONLY_BEGIN
if (CODEC_BASE64.startsWith("/*") && typeof process !== "undefined" && process.versions?.node) {
  codecBytes = new Uint8Array(await (await import("node:fs/promises")).readFile(new URL("../build/codecs/fm1-codecs.wasm", import.meta.url)));
}
// NODE_ONLY_END
let codecClient;
export class CodecError extends Error {
  constructor(code) { super("Invalid or unsupported file data (" + code + ")"); this.name = "CodecError"; this.code = code; }
}
export function codecWords(bytes) {
  if (bytes.length % 4) throw new CodecError(-1);
  const v = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return Array.from({length: bytes.length / 4}, (_, i) => v.getUint32(i * 4, true));
}
export function codecIntegers(values, width = 4) {
  const bytes = new Uint8Array(values.length * width), view = new DataView(bytes.buffer);
  values.forEach((value, i) => width === 2 ? view.setUint16(i * width, value, true) : view.setUint32(i * width, value, true));
  return bytes;
}
export function codecConcat(chunks) {
  const out = new Uint8Array(chunks.reduce((n, b) => n + b.length, 0));
  let at = 0; for (const b of chunks) { out.set(b, at); at += b.length; } return out;
}
export function getCodecs() {
  if (codecClient) return codecClient;
  const bytes = codecBytes || Uint8Array.from(atob(CODEC_BASE64), c => c.charCodeAt(0));
  const ex = new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports;
  if (ex.codec_abi() !== 1) throw new Error("Unsupported file codec");
  const capacity = ex.codec_capacity(), input = ex.codec_input(), output = ex.codec_output();
  codecClient = Object.freeze({
    call(op, args = [], blobs = []) {
      if (args.length > 32 || blobs.length > 16) throw new CodecError(-1);
      const data = blobs.map(b => b ? Uint8Array.from(b) : new Uint8Array());
      const size = 256 + data.reduce((n, b) => n + b.length, 0);
      if (size > capacity) throw new CodecError(-8);
      const window = new Uint8Array(ex.memory.buffer, input, size); window.fill(0);
      const header = new DataView(ex.memory.buffer, input, 256);
      args.forEach((v, i) => header.setUint32(i * 4, v, true));
      let at = 256;
      data.forEach((b, i) => { header.setUint32(128 + i * 8, at, true); header.setUint32(132 + i * 8, b.length, true); window.set(b, at); at += b.length; });
      const n = ex.codec_call(op, size);
      if (n < 0) throw new CodecError(n);
      return new Uint8Array(ex.memory.buffer, output, n).slice();
    },
    counter: index => ex.codec_counter(index),
    guards: () => ex.codec_guards() === 1,
    memoryBytes: () => ex.memory.buffer.byteLength,
  });
  return codecClient;
}
