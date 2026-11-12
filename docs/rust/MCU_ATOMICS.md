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
recovery stub: image512,488B, ordinary RAM97,396B, pool339,420B. All operations
(load/store, wrapping add/subtract, max, CAS, exchange and bit operations)
passed.64 actual CPU1 jobs and CPU0 concurrently performed32,768 increments;
every512-increment batch and the final total were exact. Both cores preserved
their ICFG values, all64 jobs joined, and the lock was clear at the end.
The entire test took6,490µs; the slowest foreground256-increment batch was
103µs. These are contention measurements for this small probe.

All30 stored objects again matched the original backup. A follow-up30-second
USB-audio capture had zero XRUN/late/crash deltas and zero shedding; maximum
render11,159µs against11,609µs, with CPU79%. Main/IRQ stack high-water remained
6,176/2,184B. [Exact measurements and hashes](mcu-atomic-physical-proof.json)
and [the optional boot-time probe](../../rust/toolchain/atomic_probe.rs) retain
the evidence. The probe can be checked with
`cargo test -p fm1-probe --features atomic-probe`; its target runtime selects
the `pi32v2` backend when emitted through mrustc.

Required before production enablement: safe handling of a worker
fault or stop while it owns the byte lock; and full-engine deadline/stack/
memory qualification. The current spin acquisition can wait indefinitely if
a stopped core leaves the lock held. That failure path must be resolved for
the production worker contract. Per-sample diagnostic counters also need a
measured hardware cost; they must not consume the render deadline unnoticed.
