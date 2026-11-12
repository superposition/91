# Saved IRQ state for platform capture and flash windows

This is the first isolated part of [R17 / issue #18](https://github.com/superposition/91/issues/18).
`firmware/hal/fm1_irq_state.h` provides two always-inline PI32v2 bindings:

```c
uint32_t flags = fm1_irq_state_save();
/* bounded copy or existing RAM-run flash operation */
fm1_irq_state_restore(flags);
```

Save reads the complete incoming ICFG word, clears only the SDK global IRQ gate
(bit 8), and synchronizes. Restore synchronizes preceding memory accesses,
restores every original ICFG bit, and synchronizes again. It never executes an
unconditional `sti`. An already masked or nested caller therefore stays masked.
The bindings inline into their callers so a flash-off RAM routine introduces no
call into XIP. They do not exclude CPU1, own a DMA span, or provide a cross-core
publication handshake.

`EN_RUST_HAL` selects these bindings for the existing `irq_save`/`irq_restore`
flash windows and the plain-window cache invalidation. The original C0 paths
remain selectable. New physical UI observation capture should use the named
bindings through its platform adapter, copy only bounded CPU0-published data,
and restore IRQ state before Rust UI/FFT/render work. CPU1-private buffers must
remain inaccessible until the existing whole-job completion/retirement protocol
has reclaimed them. IRQ masking is not a replacement for that protocol.

The host test executes the actual header with explicit mock ISA operations:

```sh
cc -O2 -Ifirmware/hal tests/host/hal_irq_state_test.c -o build/hal-irq-test
build/hal-irq-test
```

It checks 65,536 incoming 32-bit words with nested save/restore, including both
global-gate states and preservation of every other bit. It verifies two reads,
four writes and two preceding restore fences per nested pair. This is a host
instruction-boundary test, not a new device/latency/RAM-link qualification.
The root platform work owns vendor disassembly, physical IRQ-state probes and
release enablement. This commit does not enable HAL, alter the pinned recovery
stub, access hardware, or complete the remaining startup/DMA/driver acceptance.
