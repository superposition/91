// SPDX-License-Identifier: GPL-3.0-only
// A mounted browser feature owns every listener and callback it creates.
export function createLifetime() {
  let disposed = false;
  const cleanups = new Set();
  const add = (cleanup) => {
    if (disposed) cleanup(); else cleanups.add(cleanup);
    return () => { if (cleanups.delete(cleanup)) cleanup(); };
  };
  return {
    get disposed() { return disposed; },
    add,
    on(target, name, handler, options) {
      if (disposed) return () => {};
      target.addEventListener(name, handler, options);
      return add(() => target.removeEventListener(name, handler, options));
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      for (const cleanup of [...cleanups].reverse()) {
        cleanups.delete(cleanup);
        try { cleanup(); } catch { /* finish releasing the other resources */ }
      }
    },
  };
}
