// SPDX-License-Identifier: GPL-3.0-only
// Small browser API doubles for lifecycle/contract tests; not a browser renderer.
export class Target {
  listeners = new Map();
  addEventListener(type, listener) {
    if (!this.listeners.has(type)) this.listeners.set(type, new Set());
    this.listeners.get(type).add(listener);
  }
  removeEventListener(type, listener) { this.listeners.get(type)?.delete(listener); }
  emit(type, values = {}) {
    const event = { type, target: this, preventDefault() {}, stopPropagation() {}, ...values };
    for (const listener of this.listeners.get(type) || []) listener(event);
    return event;
  }
  get listenerCount() { return [...this.listeners.values()].reduce((n, listeners) => n + listeners.size, 0); }
}

class Classes {
  values = new Set();
  add(...names) { names.forEach((name) => this.values.add(name)); }
  remove(...names) { names.forEach((name) => this.values.delete(name)); }
  contains(name) { return this.values.has(name); }
  toggle(name, force = !this.contains(name)) { force ? this.add(name) : this.remove(name); return force; }
}

export class Element extends Target {
  constructor(tag = 'div') {
    super(); this.tagName = tag; this.children = []; this.attributes = new Map();
    this.style = { setProperty(name, value) { this[name] = value; } };
    this.dataset = {}; this.classList = new Classes(); this.parentNode = null;
    this.context = {
      createImageData: (w, h) => ({ data: new Uint8ClampedArray(w * h * 4) }),
      putImageData(image) { this.image = image; },
      drawImage() {}, clearRect() {}, fillRect() {}, beginPath() {}, moveTo() {}, lineTo() {}, stroke() {},
    };
  }
  set className(value) { this.classList = new Classes(); this.classList.add(...value.split(/\s+/)); }
  set textContent(value) { this.text = value; for (const child of this.children) child.parentNode = null; this.children = []; }
  get textContent() { return this.text || ''; }
  set innerHTML(value) {
    this.replaceChildren(); const stack = [this];
    for (const token of value.matchAll(/<\/?([\w-]+)([^>]*)>/g)) {
      if (token[0].startsWith('</')) { stack.pop(); continue; }
      const child = new Element(token[1]);
      for (const attr of token[2].matchAll(/([\w-]+)="([^"]*)"/g)) child.setAttribute(attr[1], attr[2]);
      stack.at(-1).appendChild(child);
      if (!['input', 'br', 'img'].includes(token[1])) stack.push(child);
    }
  }
  setAttribute(name, value) {
    this.attributes.set(name, String(value));
    if (name === 'class') this.className = value;
    if (name.startsWith('data-')) this.dataset[name.slice(5)] = value;
  }
  getAttribute(name) { return this.attributes.get(name); }
  hasAttribute(name) { return this.attributes.has(name); }
  removeAttribute(name) { this.attributes.delete(name); }
  appendChild(child) { child.remove(); child.parentNode = this; this.children.push(child); return child; }
  append(child) { this.appendChild(child); }
  insertBefore(child, next) {
    if (!next) return this.appendChild(child);
    if (child === next) return child;
    child.remove(); const index = this.children.indexOf(next);
    if (index < 0) throw new Error('Missing insertion anchor');
    child.parentNode = this; this.children.splice(index, 0, child); return child;
  }
  replaceChildren() { for (const child of [...this.children]) child.remove(); }
  remove() {
    if (this.parentNode) this.parentNode.children.splice(this.parentNode.children.indexOf(this), 1);
    this.parentNode = null;
  }
  get nextSibling() { return this.parentNode?.children[this.parentNode.children.indexOf(this) + 1] || null; }
  get lastElementChild() { return this.children.at(-1); }
  get firstElementChild() { return this.children[0]; }
  getBBox() { return { x: 0, y: 0, width: 50, height: 50 }; }
  getContext() { return this.context; }
  setPointerCapture() {}
  click() { this.emit('click'); }
  add(option) { this.appendChild(new Element('option')); }
  matches(selector) {
    if (selector.startsWith('.')) return this.classList.contains(selector.slice(1));
    if (selector.startsWith('[')) return this.getAttribute('data-a') === selector.match(/"([^"]+)"/)[1];
    return this.tagName === selector;
  }
  querySelector(selector) {
    const [first, ...rest] = selector.split(' ');
    for (const child of this.children) {
      if (child.matches(first)) { if (!rest.length) return child; const nested = child.querySelector(rest.join(' ')); if (nested) return nested; }
      const nested = child.querySelector(selector); if (nested) return nested;
    }
    return null;
  }
  closest(selector) { return this.matches(selector) ? this : this.parentNode?.closest(selector); }
}

export function browser() {
  const elements = new Map(), all = [];
  const make = (name) => { const element = new Element(name); all.push(element); return element; };
  const body = make('body'), home = make('div'), device = make('div'), panel = make('svg'), screen = make('canvas'), start = make('div');
  body.append(home); home.append(device); device.append(panel); device.append(screen); device.append(start);
  const ids = ['device', 'panel', 'screen', 'start', 'go', 'reset', 'forget', 'status', 'midiOn', 'midiStatus', 'learn', 'unlearn', 'maps', 'mapsave', 'mapload', 'mapsend'];
  for (const id of ids) elements.set(id, ({ device, panel, screen, start })[id] || make('div'));
  const bar = make('div'); bar.className = 'bar'; body.append(bar);
  const document = { body, documentElement: make('html'), getElementById: (id) => elements.get(id),
    createElement: make, createElementNS: (ns, tag) => make(tag), querySelector: (s) => body.querySelector(s) };
  const window = new Target(); let next = 0;
  Object.assign(window, {
    Option: class extends Element {},
    innerWidth: 1920, innerHeight: 1080, raf: new Map(),
    requestAnimationFrame(callback) { this.raf.set(++next, callback); return next; },
    cancelAnimationFrame(id) { this.raf.delete(id); },
    getComputedStyle: () => ({ getPropertyValue: () => '#1D6FB8' }), matchMedia: () => ({ matches: false }),
    location: { href: 'http://example.test/emu/', search: '' }, history: { replaceState() {} },
  });
  return { document, window, elements, all };
}
