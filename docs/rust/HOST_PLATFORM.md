# Shared native and browser platform model

`fm1-app::host_platform`, behind the `host-platform` feature, contains the
bounded control state, host MIDI queue, store metadata, software framebuffer
operations and virtual render/UI schedule. Native simulator and browser builds
select this feature; physical MCU builds do not select the host clock/model.

The native simulator now uses this implementation directly. Its existing boxed
framebuffer and saved-object byte arrays remain the payload allocations; the
shared model contains their metadata. No second framebuffer, object bank or
MIDI queue is introduced. Native clock diagnostics retain their previous
counter separately from the shared schedule used to decide deadlines.

The model preserves four initially missing objects and empty remaining objects,
31 slots of 3,840 bytes, restore locking, wrapping write counts, the host's
256-event overwrite queue, one-shot dim-key masks, byte-swapped fills and raw
RGB565 blits. Encoder burst addition rejects signed overflow before mutation.
That guard is for the foreign browser input seam; the physical acceleration
policy remains in its separate reviewed controller module.

Native store reads copy directly into a foreign raw destination. A C caller may
supply uninitialized output, so the ABI forms no Rust slice/reference to those
bytes. Missing/invalid objects return before touching output; a zero-length
load requires no destination pointer. The source remains initialized and
disjoint, and no callback occurs during the copy.

Validation at this checkpoint:

- Native five-way compatibility: all 97 scenarios, 538 artifacts and three
  fresh-process repeatability sweeps match, including all 19 inherited failure
  fingerprints and 129 damaged-store trials.
- Shared model/simulator unit tests, formatting and strict Clippy pass.
- Strict-provenance Miri passes the actual native ABI load into a
  `MaybeUninit<[u8; 4]>`, plus missing/invalid/empty-object cases.

The browser adapter uses the same model through a six-function callback table:
boot, service, render, frame, input and elapsed-time observation. Its Rust
schedule advances in one-millisecond steps, renders 256-frame blocks and
requests a UI frame every 16 milliseconds. Each state borrow ends before a
callback can reenter the platform. The C layer retains application ABI thunks
and the `clock_gettime` observation; it has no platform state or clock loop.

Framebuffer, saved-object bytes and the render block have separate private
`UnsafeCell` roots. Persistent pointers come directly from these roots. Store
loads copy into raw foreign output, including uninitialized destinations,
without forming a Rust reference to the destination. Strict-provenance Miri
covers callback reentry and retained frame/store addresses. The existing 24
public browser exports and fixed 32 MiB WASM memory remain unchanged.

The inherited clock uses wrapping `u32` milliseconds and an increasing `f64`
audio deadline. Its approximately 49.7-day epoch limit is preserved; this
checkpoint does not qualify operation across that wrap. Browser frame/store
messages and structured-clone copies remain JavaScript binding work.
