// SPDX-License-Identifier: GPL-3.0-only
// API contract and resource-lifecycle coverage, without launching a browser.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { setTimeout as delay } from 'node:timers/promises';
import { createAudioSession } from '../../web/emu/src/audio.mjs';
import { createPanel } from '../../web/emu/src/panel.mjs';
import { createControls } from '../../web/emu/src/controls.mjs';
import { createMidi } from '../../web/emu/src/midi.mjs';
import { createStreamView } from '../../web/emu/src/stream.mjs';
import { createStorage, STORE_KEY, MAP_KEY } from '../../web/emu/src/storage.mjs';
import { mountEmulator } from '../../web/emu/src/app.mjs';
import { browser, Target } from './browser_doubles.mjs';

const worklet = await import('data:text/javascript;base64,' + fs.readFileSync('web/emu/worklet.js').toString('base64'));
const wasm = fs.readFileSync(process.argv[2] || 'build/emu/amb.wasm');
const flush = () => delay(5);
const deferred = () => { let resolve; const promise = new Promise((done) => { resolve = done; }); return { promise, resolve }; };
const config = { store: {}, seed: 91, master: 2800 };

function audioEnvironment({ secure = false, fetch, module = Promise.resolve(), resume = Promise.resolve(), startupTimeout = 2000 } = {}) {
  const contexts = [], nodes = [], frames = [];
  function makeNode(context) {
    const channel = new MessageChannel();
    const processor = new worklet.Amb({ port: channel.port1, audioTime: () => context.currentTime });
    const node = { context, port: channel.port2, connects: new Set(), disconnects: 0,
      connect(destination) { this.connects.add(destination); },
      disconnect(destination) { this.disconnects++; if (destination) this.connects.delete(destination); else this.connects.clear(); },
      render(n = 128) { const channels = [new Float32Array(n), new Float32Array(n)]; processor.process([], [channels]); context.currentTime += n / 44100; return channels; },
      processor };
    if (!secure) node.dispose = () => { processor.dispose(); node.disconnect(); channel.port1.close(); channel.port2.close(); };
    nodes.push(node); return node;
  }
  class Context {
    constructor() { this.destination = {}; this.currentTime = 0; this.sampleRate = 44100; this.state = 'suspended'; this.closes = 0; contexts.push(this); if (secure) this.audioWorklet = { addModule: () => module }; }
    async resume() { await resume; if (this.state !== 'closed') this.state = 'running'; }
    async close() { this.closes++; this.state = 'closed'; }
  }
  const session = createAudioSession({ AudioContext: Context, AudioWorkletNode: function(context) { return makeNode(context); },
    fetch: fetch || (async () => ({ ok: true, arrayBuffer: async () => wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength) })),
    loadHttp: async () => { await module; return { createHttpAudioNode: makeNode }; },
    onFrame: (frame) => frames.push(frame), startupTimeout });
  return { session, contexts, nodes, frames };
}

// Boot both selections through the real processor, then close all owned resources.
let securePCM;
for (const secure of [true, false]) {
  const env = audioEnvironment({ secure });
  const first = env.session.start(config);
  assert.equal(env.session.start(config), first, 'coalesce double-click startup');
  const node = await first;
  assert.equal(Object.keys(node.processor.ex).length, 24, 'preserved WASM exports');
  let peak = 0;
  const samples = [];
  for (let block = 0; block < 4096; block++) {
    for (const channel of node.render()) { samples.push(...channel); for (const value of channel) peak = Math.max(peak, Math.abs(value)); }
  }
  await flush();
  assert(peak > 0.001);
  assert(env.frames.some((frame) => frame.fb?.length === 240 * 240));
  assert(env.frames.some((frame) => frame.rust?.plans > 0));
  if (secure) securePCM = samples; else assert.deepEqual(samples, securePCM);
  await env.session.dispose(); await env.session.dispose(); await flush();
  assert.equal(env.contexts[0].closes, 1);
  assert.equal(node.port.onmessage, null);
  assert.equal(node.processor.ex, null);
  assert.equal(node.processor.process([], [[new Float32Array(128)]]), false);
  assert.equal(node.connects.size, 0);
  await assert.rejects(env.session.start(config), /closed/);
}

// Disposal must not wait for module loading, WASM fetch, or AudioContext resume.
for (const secure of [true, false]) {
  const module = deferred(), env = audioEnvironment({ secure, module: module.promise });
  const starting = env.session.start(config); const rejection = assert.rejects(starting, /closed/);
  await env.session.dispose(); await rejection;
  assert.equal(env.contexts[0].closes, 1); assert.equal(env.nodes.length, 0);
  module.resolve(); await flush(); assert.equal(env.nodes.length, 0, 'late module cannot create an orphan node');
}
for (const mode of ['fetch', 'resume']) {
  const stalled = deferred();
  const env = audioEnvironment(mode === 'fetch' ? { fetch: () => stalled.promise } : { resume: stalled.promise });
  const starting = env.session.start(config); const rejection = assert.rejects(starting, /closed/);
  await flush(); await env.session.dispose(); await rejection;
  assert.equal(env.contexts[0].closes, 1);
  for (const node of env.nodes) assert.equal(node.processor.ex, null);
  stalled.resolve({ ok: true, arrayBuffer: async () => new ArrayBuffer(0) });
}
{
  let requests = 0;
  const env = audioEnvironment({ fetch: async () => {
    if (!requests++) throw new Error('network unavailable');
    return { ok: true, arrayBuffer: async () => wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength) };
  } });
  await assert.rejects(env.session.start(config), /network unavailable/);
  assert.equal(env.contexts[0].closes, 1);
  await env.session.start(config);
  await env.session.dispose();
  assert(env.contexts.every((context) => context.closes === 1));
}
{
  let requests = 0;
  const env = audioEnvironment({ fetch: async () => ({ ok: true, arrayBuffer: async () => requests++ ? wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength) : new ArrayBuffer(0) }) });
  await assert.rejects(env.session.start(config), /WebAssembly|buffer|empty/i);
  await env.session.start(config); await env.session.dispose();
  assert(env.contexts.every((context) => context.closes === 1));
}
{
  const env = audioEnvironment({ module: new Promise(() => {}), startupTimeout: 20 });
  await assert.rejects(env.session.start(config), /too long/);
  assert.equal(env.contexts[0].closes, 1); await env.session.dispose();
}
{
  const processor = new worklet.Amb({ port: { postMessage() {}, close() {} }, audioTime: () => 0 });
  const loading = processor.onMessage({ type: 'load', wasm, ...config });
  processor.dispose(); await loading; assert.equal(processor.ex, null);
}

const backing = new Map(), store = createStorage(() => ({ getItem: (key) => backing.get(key), setItem: (key, value) => backing.set(key, value), removeItem: (key) => backing.delete(key) }));
store.saveStore({ 2: Uint8Array.from([0, 127, 128, 255]) });
assert.equal(backing.get(STORE_KEY), '{"2":"AH+A/w=="}');
assert.deepEqual(store.loadStore(), { 2: Uint8Array.from([0, 127, 128, 255]) });
store.saveMaps({ 'c:0:21': { t: 'e2', mode: 'rel64' } });
assert.deepEqual(store.loadMaps(), { 'c:0:21': { t: 'e2', mode: 'rel64' } });
assert(backing.has(MAP_KEY));
const unavailable = createStorage(() => { throw new Error('blocked storage'); });
assert.deepEqual(unavailable.loadStore(), {}); unavailable.saveStore({}); unavailable.forgetStore();

// Exercise real panel/input logic with API doubles, including release on teardown.
const env = browser(), messages = [];
const panel = createPanel(env.document);
const controls = createControls(panel, { window: env.window, send: (message) => messages.push(message), isActive: () => true });
env.window.emit('keydown', { code: 'KeyQ' }); assert.deepEqual(messages.at(-1), { type: 'keys', mask: 1 });
env.window.emit('keyup', { code: 'KeyQ' }); assert.equal(messages.at(-1).mask, 0);
panel.buttons.PLAY.hit.emit('pointerdown', { pointerId: 1 }); assert.equal(messages.at(-1).mask, 1 << 10);
panel.device.emit('pointercancel', { pointerId: 1 }); assert.equal(messages.at(-1).mask, 0);
panel.keys[3].hit.emit('pointerdown', { pointerId: 2 });
panel.keys[5].hit.emit('pointerdown', { pointerId: 3 }); assert.equal(messages.at(-1).mask, (1 << 3) | (1 << 5));
panel.device.emit('lostpointercapture', { pointerId: 2 }); assert.equal(messages.at(-1).mask, 1 << 5);
panel.device.emit('pointerup', { pointerId: 3 });
panel.knobs.KNOB1.hit.emit('wheel', { deltaY: -60, deltaMode: 0 }); assert.deepEqual(messages.at(-1), { type: 'enc', role: 3, n: 1 });
panel.knobs.KNOB3.hit.emit('pointerdown', { pointerId: 8, clientY: 50 });
panel.knobs.KNOB3.hit.emit('pointermove', { pointerId: 8, clientY: 29 }); assert.deepEqual(messages.at(-1), { type: 'enc', role: 5, n: 3 });
panel.knobs.KNOB3.hit.emit('pointerup');
panel.buttons.REC.hit.emit('pointerdown', { pointerId: 9, shiftKey: true }); assert.equal(messages.at(-1).mask, 1 << 11);
panel.buttons.REC.hit.emit('pointerdown', { pointerId: 9 }); assert.equal(messages.at(-1).mask, 0);
env.window.emit('keydown', { code: 'Space' }); env.window.emit('blur'); assert.equal(messages.at(-1).mask, 0);
const fb = new Uint16Array(240 * 240).fill(0x00F8); panel.drawScreen(fb);
assert.deepEqual([...panel.screen.context.image.data.slice(0, 4)], [255, 0, 0, 255]);

// MIDI keeps existing remote/map wire numbers while releasing ports and holds.
const input = Object.assign(new Target(), { id: 'fm1', name: 'FM-1', manufacturer: 'M-VAVE', state: 'connected', connection: 'closed', closes: 0, close() { this.closes++; return Promise.resolve(); } });
const output = { name: 'FM-1', state: 'connected', connection: 'closed', closes: 0, sent: [], send(message) { this.sent.push(message); }, close() { this.closes++; return Promise.resolve(); } };
const access = Object.assign(new Target(), { inputs: new Map([['in', input]]), outputs: new Map([['out', output]]) });
const midi = createMidi(panel, controls, { storage: store, ...env, navigator: { requestMIDIAccess: async () => access } });
await midi.connect(); assert.equal(input.listenerCount, 1);
input.emit('midimessage', { data: [0x90, 60, 100] }); assert.equal(messages.at(-1).mask, 1 << 7);
input.emit('midimessage', { data: [0x80, 60, 0] }); assert.equal(messages.at(-1).mask, 0);
input.emit('midimessage', { data: [0xB0, 21, 65] }); assert.deepEqual(messages.at(-1), { type: 'enc', role: 2, n: 1 });
input.emit('midimessage', { data: [0xBF, 116, 127] }); assert.deepEqual(output.sent[0], [0xBF, 117, 127]);
midi.lights(1 << 10, 1); assert(output.sent.some((m) => m.join() === '191,112,127'));
input.emit('midimessage', { data: [0x9F, 48, 100] }); assert.equal(messages.at(-1).mask, 1);

// Stream toggles own a single analyser and animation, then restore the device.
const stream = createStreamView(panel, controls, env);
const audioNode = { connects: new Set(), context: { sampleRate: 44100, createAnalyser: () => ({ frequencyBinCount: 1024, disconnect() {}, getFloatFrequencyData(array) { array.fill(-100); } }) }, connect(dest) { this.connects.add(dest); }, disconnect(dest) { this.connects.delete(dest); } };
stream.setAudioNode(audioNode); assert.equal(audioNode.connects.size, 0);
stream.set(true); assert.equal(audioNode.connects.size, 1); assert.equal(env.window.raf.size, 1);
stream.set(true); assert.equal(env.window.raf.size, 1);
const [frameId, frame] = [...env.window.raf][0]; env.window.raf.delete(frameId); frame(100);
assert.equal(env.window.raf.size, 1, 'stream schedules one next frame');
controls.turn(panel.knobs.KNOB2, 1);
stream.set(false); assert.equal(audioNode.connects.size, 0); assert.equal(env.window.raf.size, 0);
stream.set(true); stream.dispose(); stream.dispose();
assert.equal(audioNode.connects.size, 0); assert.equal(env.window.raf.size, 0); assert(!env.document.body.classList.contains('stream'));
midi.dispose(); midi.dispose();
assert.equal(input.listenerCount, 0); assert.equal(access.listenerCount, 0); assert.equal(input.closes, 1);
assert.equal(output.closes, 1);
assert.deepEqual(output.sent.at(-1), [0xBF, 117, 0]);
controls.dispose(); panel.dispose();
assert.equal(env.window.listenerCount, 0);
assert(env.all.every((element) => element.listenerCount === 0));
assert.equal(panel.svg.children.length, 0);

// A permission request resolving after unmount must not attach MIDI handlers.
{
  const late = deferred(), ui = browser(), panel = createPanel(ui.document);
  const controls = createControls(panel, { window: ui.window, send() {}, isActive: () => false });
  const midi = createMidi(panel, controls, { ...ui, storage: store, navigator: { requestMIDIAccess: () => late.promise } });
  const connecting = midi.connect(); midi.dispose(); late.resolve(access); await connecting;
  assert.equal(input.listenerCount, 0); assert.equal(access.listenerCount, 0);
  controls.dispose(); panel.dispose();
}

// Mount/unmount the entry point repeatedly without retaining generated targets.
{
  const ui = browser(); ui.window.navigator = {}; ui.window.localStorage = { getItem() { return null; } };
  for (let cycle = 0; cycle < 3; cycle++) {
    const mounted = mountEmulator(ui.document, ui.window);
    assert.equal(ui.elements.get('panel').children.length > 0, true);
    await mounted.dispose();
    assert.equal(ui.window.listenerCount, 0);
    assert(ui.all.every((element) => element.listenerCount === 0));
    assert.equal(ui.elements.get('panel').children.length, 0);
    assert.equal(ui.window.raf.size, 0);
  }
}
console.log(JSON.stringify({ browser_api_contract: 'pass', secure_http_boot_and_pcm: true, startup_cancel_timeout_retry: true, input_midi_stream_storage_lifecycle: true, interactive_browser_qa: 'not run' }));
