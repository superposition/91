// SPDX-License-Identifier: GPL-3.0-only
// Both browser audio paths use the same processor, message contract and WASM.
export function createAudioSession({
  onFrame = () => {},
  AudioContext = globalThis.AudioContext,
  AudioWorkletNode = globalThis.AudioWorkletNode,
  fetch = globalThis.fetch,
  loadHttp = () => import("../worklet.js?v=" + Date.now()),
  wasmURL = new URL("../amb.wasm", import.meta.url),
  workletURL = new URL("../worklet.js?v=" + Date.now(), import.meta.url),
  startupTimeout = 30000,
} = {}) {
  let context = null, node = null, pending = null, disposed = false;
  let controller = null, cancelStartup = null, generation = 0, teardown = null;

  async function release() {
    generation++;
    controller?.abort(); controller = null;
    cancelStartup?.(); cancelStartup = null;
    const oldNode = node, oldContext = context;
    node = context = null;
    if (oldNode) {
      oldNode.port.onmessage = null;
      oldNode.onprocessorerror = null;
      try { oldNode.port.postMessage({ type: "dispose" }); } catch {}
      try { oldNode.disconnect(); } catch {}
      try { if (oldNode.dispose) oldNode.dispose(); else oldNode.port.close(); } catch {}
    }
    if (oldContext && oldContext.state !== "closed") {
      try { await oldContext.close(); } catch {}
    }
  }

  function start({ store, master, seed }) {
    if (disposed) return Promise.reject(new Error("The instrument has closed."));
    if (pending) return pending;
    if (node && context?.state === "running") return Promise.resolve(node);
    pending = (async () => {
      let timer;
      try {
        const token = ++generation;
        const interrupted = new Promise((resolve, reject) => {
          cancelStartup = () => reject(new Error("The instrument has closed."));
          timer = setTimeout(() => reject(new Error("The instrument took too long to start. Try again.")), startupTimeout);
        });
        // Also covers synchronous startup failures before the first race.
        interrupted.catch(() => {});
        // Start/resume in the user's click stack, before fetch or module awaits.
        context = new AudioContext({ sampleRate: 44100, latencyHint: "interactive" });
        const resume = context.resume();
        controller = new AbortController();
        const wasmReady = fetch(wasmURL, { cache: "no-store", signal: controller.signal }).then((r) => {
          if (!r.ok) throw new Error(`amb.wasm: ${r.status}`);
          return r.arrayBuffer();
        });
        const nodeReady = (async () => {
          const current = context;
          if (current.audioWorklet && typeof AudioWorkletNode === "function") {
            await current.audioWorklet.addModule(workletURL);
            if (disposed || token !== generation) throw new Error("The instrument has closed.");
            node = new AudioWorkletNode(current, "amb", { numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2] });
            return node;
          }
          const { createHttpAudioNode } = await loadHttp();
          if (disposed || token !== generation) throw new Error("The instrument has closed.");
          node = createHttpAudioNode(current);
          return node;
        })();
        // Attach rejection handlers to all startup work immediately.
        const [wasm, audioNode] = await Promise.race([Promise.all([wasmReady, nodeReady, resume]), interrupted]);
        node = audioNode;
        if (disposed) throw new Error("The instrument has closed.");
        const ready = new Promise((resolve, reject) => {
          const finish = (error) => error ? reject(error) : resolve();
          node.onprocessorerror = () => finish(new Error("The instrument could not start audio. Try again."));
          node.port.onmessage = ({ data }) => {
            if (disposed) return;
            if (data.type === "ready") finish();
            else if (data.type === "error") finish(new Error(data.message));
            else if (data.type === "frame") onFrame(data);
          };
        });
        node.connect(context.destination);
        node.port.postMessage({ type: "load", wasm, store, master, seed }, [wasm]);
        await Promise.race([ready, interrupted]);
        return node;
      } catch (error) {
        await release();
        throw error;
      } finally { clearTimeout(timer); cancelStartup = null; pending = null; }
    })();
    return pending;
  }

  return {
    start,
    get node() { return node; },
    get active() { return !!node && !disposed; },
    send(message) { if (!disposed && node) node.port.postMessage(message); },
    dispose() {
      if (teardown) return teardown;
      disposed = true;
      teardown = (async () => {
        await release();
        // Cancellation releases start even if a browser module promise stalls.
        try { await pending; } catch {}
      })();
      return teardown;
    },
  };
}
