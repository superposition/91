// SPDX-License-Identifier: GPL-3.0-only
import { createLifetime } from "./lifecycle.mjs";
import { createPanel } from "./panel.mjs";
import { createControls } from "./controls.mjs";
import { createAudioSession } from "./audio.mjs";
import { createStorage } from "./storage.mjs";
import { createMidi } from "./midi.mjs";
import { createStreamView } from "./stream.mjs";

export function mountEmulator(document = globalThis.document, window = globalThis.window) {
  const scope = createLifetime();
  const storage = createStorage(() => window.localStorage);
  const panel = createPanel(document);
  const audio = createAudioSession({ onFrame(frame) {
    if (frame.fb) panel.drawScreen(frame.fb);
    panel.drawLights(frame.buttons, frame.keys);
    midi.lights(frame.buttons, frame.keys);
    window.ambLast = frame;
    if (frame.store) storage.saveStore(frame.store);
  } });
  const controls = createControls(panel, { send: (message) => audio.send(message), isActive: () => audio.active, window });
  const midi = createMidi(panel, controls, { storage, document, window, navigator: window.navigator });
  const stream = createStreamView(panel, controls, { document, window });
  const go = document.getElementById("go"), start = document.getElementById("start"), status = document.getElementById("status");
  let starting = false, teardown = null;

  scope.on(go, "click", async () => {
    if (starting) return;
    starting = true; go.disabled = true; go.textContent = "Starting...";
    try {
      const node = await audio.start({ store: storage.loadStore(), master: controls.master, seed: (Math.random() * 4294967295) >>> 0 || 1 });
      if (scope.disposed) return;
      stream.setAudioNode(node);
      start.hidden = true; status.textContent = "On.";
      controls.resync();
    } catch (error) {
      if (!scope.disposed) {
        go.disabled = false; go.textContent = "Switch on";
        status.textContent = "Could not start: " + (error?.message || error) + ". Chrome, Edge, Firefox and Safari should all work.";
      }
    } finally { starting = false; }
  });
  scope.on(document.getElementById("reset"), "click", () => { void dispose().then(() => window.location.reload()); });
  scope.on(document.getElementById("forget"), "click", () => {
    const button = document.getElementById("forget");
    if (button.dataset.armed) {
      storage.forgetStore(); void dispose().then(() => window.location.reload());
    } else {
      button.dataset.armed = "1"; button.textContent = "Click again: forget all stored pieces";
    }
  });

  function dispose() {
    if (teardown) return teardown;
    scope.dispose(); stream.dispose(); midi.dispose(); controls.dispose(); panel.dispose();
    teardown = audio.dispose();
    start.hidden = false; go.disabled = false; go.textContent = "Switch on";
    status.textContent = "Off. Sound starts when you switch it on.";
    return teardown;
  }
  return { dispose };
}

// Close audio when navigating away; a restored back/forward-cache page mounts
// fresh listeners and controls rather than retaining a closed AudioContext.
if (typeof document !== "undefined") {
  let mounted = mountEmulator();
  window.addEventListener("pagehide", () => { void mounted.dispose(); });
  window.addEventListener("pageshow", (event) => { if (event.persisted) mounted = mountEmulator(); });
}
