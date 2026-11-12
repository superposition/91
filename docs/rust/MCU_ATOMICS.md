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

These results do not qualify the target backend. Required before enabling it
in a device image: physical two-core exclusion and IRQ-state tests; measured
latency/contention; generated-object barriers; safe handling of a worker
fault or stop while it owns the byte lock; and full-engine deadline/stack/
memory qualification. The current spin acquisition can wait indefinitely if
a stopped core leaves the lock held. That failure path must be resolved for
the production worker contract. Per-sample diagnostic counters also need a
measured hardware cost; they must not consume the render deadline unnoticed.
