# Shared native and browser platform model

`fm1-app::host_platform`, behind the `host-platform` feature, contains the
bounded control state, host MIDI queue, store metadata, software framebuffer
operations and virtual render/UI schedule. The feature is selected by the
native simulator; physical MCU builds do not select the host clock/model.

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

The browser still uses the previous C platform/clock until its adapter change
lands. This checkpoint establishes the shared implementation and its native
consumer; it does not claim completion of browser binding work or MCU timing.
