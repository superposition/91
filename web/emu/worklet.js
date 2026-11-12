// SPDX-License-Identifier: GPL-3.0-only
// NAIWAN 91 in an AudioWorklet: amb.wasm is the whole FM-1 app (web/emu/amb_web.c). Each render
// quantum asks it for 128 frames, which runs the device's clock forward; between quanta the page's
// input goes in and, about 30 times a second, the screen, the lights and any saved objects go out.

const clock = globalThis.performance ? () => globalThis.performance.now() : () => Date.now();

const ProcessorBase = globalThis.AudioWorkletProcessor || class {
  constructor(options) { this.port = options.port; }
};

export class Amb extends ProcessorBase {
  constructor(options) {
    super(options);
    this.audioTime = options?.audioTime || (() => currentTime);
    this.ex = null;
    this.input = null;
    this.leftViews = this.rightViews = this.dataView = null;
    this.sentBlits = -1;
    this.sentWrites = -1;
    this.lastFrame = 0;
    this.busy = 0;
    this.frames = 0;
    this.disposed = false;
    this.port.onmessage = (e) => {
      this.onMessage(e.data).catch((error) => {
        if (!this.disposed) this.port.postMessage({ type: "error", message: error.message || String(error) });
      });
    };
  }

  async onMessage(m) {
    if (m.type === "dispose") { this.dispose(); return; }
    if (this.disposed) return;
    if (m.type === "load") {
      const wasi = {
        clock_time_get: (id, prec, out) => {
          // Split the same rounded Number directly into WASI's little-endian
          // u64. BigInt construction would allocate on every render block.
          const ns = Math.round(this.audioTime() * 1e9);
          this.dataView.setUint32(out, ns % 4294967296, true);
          this.dataView.setUint32(out + 4, Math.floor(ns / 4294967296), true);
          return 0;
        },
        fd_write: (fd, iov, n, out) => { this.dataView.setUint32(out, 0, true); return 0; },
        proc_exit: () => {},
      };
      const stub = new Proxy(wasi, { get: (t, k) => (k in t ? t[k] : () => 0) });
      const env = new Proxy({}, { get: () => () => 0 });
      const { instance } = await WebAssembly.instantiate(m.wasm, { wasi_snapshot_preview1: stub, env });
      if (this.disposed) return;
      this.ex = instance.exports;
      this.mem = this.ex.memory;
      this.dataView = new DataView(this.mem.buffer);
      if (this.ex._initialize) this.ex._initialize();
      // Fixed WASM memory and stable output addresses: create every bounded
      // audio view at startup, including unusual device render quantum sizes.
      const left = new Float32Array(this.mem.buffer, this.ex.web_out_l(), 1024);
      const right = new Float32Array(this.mem.buffer, this.ex.web_out_r(), 1024);
      this.leftViews = new Array(1025); this.rightViews = new Array(1025);
      for (let n = 0; n <= 1024; n++) {
        this.leftViews[n] = left.subarray(0, n);
        this.rightViews[n] = right.subarray(0, n);
      }
      const max = this.ex.web_store_max();
      for (const [obj, bytes] of Object.entries(m.store || {})) {
        const b = new Uint8Array(bytes).subarray(0, max);
        new Uint8Array(this.mem.buffer, this.ex.web_store(+obj), b.length).set(b);
        this.ex.web_store_set_len(+obj, b.length);
      }
      this.sentWrites = this.ex.web_store_writes();
      this.ex.web_master(m.master ?? 2800);
      if (this.ex.web_seed) this.ex.web_seed(m.seed >>> 0);   // a different start each time
      this.ex.web_boot();
      this.port.postMessage({ type: "ready" });
    } else if (!this.ex) {
      return;
    } else if (m.type === "buttons") {
      this.ex.web_buttons(m.mask >>> 0);
    } else if (m.type === "keys") {
      this.ex.web_keys(m.mask >>> 0);
    } else if (m.type === "enc") {
      this.ex.web_enc(m.role, m.n | 0);
    } else if (m.type === "master") {
      this.ex.web_master(m.value | 0);
    } else if (m.type === "poll") {
      // Frame/store serialization runs in a MessagePort task, outside process.
      // A delayed page coalesces observations without delaying PCM production.
      const now = this.audioTime();
      if (now - this.lastFrame >= 1 / 30) {
        this.lastFrame = now;
        this.publish();
      }
    }
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    this.ex = this.mem = this.input = null;
    this.leftViews = this.rightViews = this.dataView = null;
    this.port.onmessage = null;
    this.port.close?.();
  }

  publish() {
    const ex = this.ex;
    const blits = ex.web_blits();
    const msg = { type: "frame", buttons: ex.web_lit_buttons() >>> 0, keys: ex.web_lit_keys() >>> 0 };
    if (ex.web_rust_stat) msg.rust = {
      validations: ex.web_rust_stat(0), plans: ex.web_rust_stat(1),
      rejections: ex.web_rust_stat(2), fallbacks: ex.web_rust_stat(3),
      bassEvents: ex.web_rust_stat(4),
      sessions: ex.web_rust_stat(5),
      commands: { accepted: ex.web_rust_stat(6), consumed: ex.web_rust_stat(7),
        dropped: ex.web_rust_stat(8), highWater: ex.web_rust_stat(9) },
      transport: { pulses: ex.web_rust_stat(10), chunks: ex.web_rust_stat(11),
        controls: ex.web_rust_stat(12) },
      handoff: { published: ex.web_rust_stat(13), copied: ex.web_rust_stat(14),
        claims: ex.web_rust_stat(15), activations: ex.web_rust_stat(16),
        rejected: ex.web_rust_stat(17) },
      sequencer: { lanes: ex.web_rust_stat(18), mutations: ex.web_rust_stat(19),
        waits: ex.web_rust_stat(20), boundaries: ex.web_rust_stat(21) },
      melodic: { acidSteps: ex.web_rust_stat(22), acidMutations: ex.web_rust_stat(23),
        pluckSteps: ex.web_rust_stat(24), bassSteps: ex.web_rust_stat(25),
        barDecisions: ex.web_rust_stat(26), midiTransport: ex.web_rust_stat(27) },
      effects: { masterSamples: ex.web_rust_stat(28), echoSamples: ex.web_rust_stat(29),
        tankSamples: ex.web_rust_stat(30), colourSampleCalls: ex.web_rust_stat(31) },
      factory: { accesses: ex.web_rust_stat(32), codecs: ex.web_rust_stat(33),
        rejections: ex.web_rust_stat(34), copiedBytes: ex.web_rust_stat(35) },
    };
    if (ex.web_bg) msg.bg = Array.from(new Float32Array(this.mem.buffer, ex.web_bg(), 12));   // for the stream view's background
    if (this.frames >= 44100) {                       // the share of real time spent running the device
      msg.load = Math.round(100 * this.busy / (this.frames / 44.1));
      this.busy = this.frames = 0;
    }
    const transfer = [];
    if (blits !== this.sentBlits) {
      this.sentBlits = blits;
      msg.fb = new Uint16Array(this.mem.buffer, ex.web_fb(), 240 * 240).slice();
      transfer.push(msg.fb.buffer);
    }
    const writes = ex.web_store_writes();
    if (writes !== this.sentWrites) {
      this.sentWrites = writes;
      msg.store = {};
      for (let o = 0; o < ex.web_nobj(); o++) {
        const n = ex.web_store_len(o);
        if (n >= 0) msg.store[o] = new Uint8Array(this.mem.buffer, ex.web_store(o), n).slice();
      }
    }
    this.port.postMessage(msg, transfer);
  }

  process(inputs, outputs) {
    if (this.disposed) return false;
    if (!this.ex) return true;
    const out = outputs[0];
    const n = out[0].length;
    const t0 = clock();
    for (let offset = 0; offset < n;) {
      const chunk = Math.min(1024, n - offset);
      this.ex.web_render(chunk);
      out[0].set(this.leftViews[chunk], offset);
      if (out[1]) out[1].set(this.rightViews[chunk], offset);
      offset += chunk;
    }
    this.busy += clock() - t0;
    this.frames += n;
    return true;
  }
}

if (typeof registerProcessor === "function") registerProcessor("amb", Amb);

// AudioWorklet is restricted to secure origins. A local-network HTTP preview
// can run the same processor through the older main-thread Web Audio callback.
export function createHttpAudioNode(context) {
  const channel = new MessageChannel();
  const processor = new Amb({ port: channel.port1, audioTime: () => context.currentTime });
  const node = context.createScriptProcessor(1024, 0, 2);
  node.port = channel.port2;
  const inputs = [], channels = [null, null], outputs = [channels];
  node.onaudioprocess = (event) => {
    channels[0] = event.outputBuffer.getChannelData(0);
    channels[1] = event.outputBuffer.getChannelData(1);
    return processor.process(inputs, outputs);
  };
  let disposed = false;
  node.dispose = () => {
    if (disposed) return;
    disposed = true;
    node.onaudioprocess = null;
    processor.dispose();
    node.disconnect(); channel.port1.close(); channel.port2.close();
  };
  return node;
}
