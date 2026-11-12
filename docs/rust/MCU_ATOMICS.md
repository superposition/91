# PI32v2 atomic ABI: qualification in progress

The PI32v2 target advertises no Rust atomics. The `fm1-rt/pi32v2` feature
selects a four-byte, four-byte-aligned `UnsafeCell<u32>` wrapper whose accesses
all pass through `fm1_mcu_atomic`. Default hosted/WASM builds retain core
atomics. `fm1-jam-bridge/pi32v2` forwards the feature; it does not enable a
firmware image or alter the default hardware build.

The experimental HAL uses the AC79 SDK's `testset b[...]` hardware primitive,
an aligned byte lock, `csync` barriers, and exact ICFG save/restore with bit8
masked during the operation. Every supported operation is sequentially
consistent, including Relaxed requests. Payload storage remains in place;
there is one extra 32-byte-aligned lock. Rust never exposes a reference to its
cell, and the opaque ABI call retains no pointer.

Eight runtime tests pass through a hosted mutex implementation of this ABI,
including the existing 500,000-word concurrent queue, snapshot and staging
tests and direct wrapping/CAS/bit-operation checks. Rust 1.94.1 type/borrow
checking passes for the MCU feature on the 32-bit WASM target. The vendor
compiler accepts the actual HAL instructions and main translation unit.
All five current types/formats/engine/runtime/bridge crates also compile
through pinned mrustc into vendor PI32v2 objects with this feature.

The first physical probe passed on `FM-1_7000303` on 12 November 2026. The
original instrument plus the Rust atomic probe installed with the unchanged
recovery stub: image 512,488 B, ordinary RAM 97,396 B, pool 339,420 B. All operations
(load/store, wrapping add/subtract, max, CAS, exchange and bit operations)
passed. 64 actual CPU1 jobs and CPU0 concurrently performed 32,768 increments;
every 512-increment batch and the final total were exact. Both cores preserved
their ICFG values, all 64 jobs joined, and the lock was clear at the end.
The entire test took 6,490 µs; the slowest foreground 256-increment batch was
103 µs. These are contention measurements for this small probe.

All 30 stored objects again matched the original backup. A follow-up 30-second
USB-audio capture had zero XRUN/late/crash deltas and zero shedding; maximum
render 11,159 µs against 11,609 µs, with CPU 79%. Main/IRQ stack high-water remained
6,176 / 2,184 B. [Exact measurements and hashes](mcu-atomic-physical-proof.json)
and [the optional boot-time probe](../../rust/toolchain/atomic_probe.rs) retain
the evidence. The probe can be checked with
`cargo test -p fm1-probe --features atomic-probe`; its target runtime selects
the `pi32v2` backend when emitted through mrustc.

Required before production enablement: safe handling of a worker
fault or stop while it owns the byte lock; and full-engine deadline/stack/
memory qualification. The revised acquisition bounds retries at 65,536:
CPU0 physically holds CPU1, marks its fault and persistent retirement guard,
then clears the abandoned byte lock. It leaves the outstanding request intact
so the later join reports lost output. CPU1 cannot recover a stuck CPU0 owner;
it parks for CPU0's bounded join or the watchdog. This revised failure path
still needs the physical stop/heartbeat proof and integrated worker tests.
The completing FX port batches its earlier per-sample counter updates. Its
whole-image hardware cost still needs measurement against the render deadline.

The physical bridge now routes panic, abort and mrustc's explicit
`panic_impl` / `_Unwind_Resume` entries through one HAL panic boundary. CPU1
parks for CPU0's bounded join and confirmed hold. CPU0 stops audio DMA, holds
CPU1 and marks the current boot failed before waiting for the watchdog. The
standard C abort/resume entries use void signatures with bodies that never
return, matching their C declarations despite mrustc's char representation of
Rust `!`. Rust 1.94.1 release Clippy checks this path on a 32-bit non-WASM
target; mrustc plus the vendor compiler accepts all five crates with the new
entries. Actual panic/silence and stopped-worker tests remain physical gates.

The integrated link also requires `memmove` for overlapping pad-key state
shifts. Its implementation is a small Rust runtime primitive using
`MaybeUninit<u8>` so padding can move without becoming an initialized typed
value. Volatile byte operations prevent recursive lowering to the runtime
symbol being implemented. A C adapter preserves the standard void-pointer
signature and immediately forwards to that Rust primitive. All overlapping
ranges in a32-byte buffer match standard Rust byte movement, including
zero-length/null input and a partially uninitialized object representation.

## CPU0 command publication

The queue has two CPU0 producer contexts: application input and render/drain
code. In particular, `naiwan_bing` calls `en_note_v`, which posts queued notes
from the render consumer. The earlier foreground-only source audit missed
that path. Per-word atomics do not prevent a render interrupt from preempting
a foreground push between loading and publishing its write index.

The MCU `post` adapter now masks CPU0 IRQs around the complete Rust publication
and diagnostic updates, restoring the exact previous ICFG word afterward.
This makes those contexts one serialized logical producer without changing
the 4,104-byte queue, capacity, order or overflow behavior. CPU1 is rejected
before queue access; its worker retains the separate deferred MIDI list.
The mask uses ICFG bit8 rather than the older flash-only cli/sti helper, which
does not preserve an already-disabled interrupt state.

`sh tests/rust_command_irq.sh` links the actual Rust queue through its opaque
MCU ABI and extracts the real C post/guard functions. A hosted register model
requests a render interruption at the first index read, defers it until the
whole foreground publication returns, and verifies both commands in order.
It also covers an already-masked caller, exact flag restoration and CPU1
refusal. The model does not execute PI32 instructions or establish latency;
real IRQ-preemption, timing and device qualification remain open.
