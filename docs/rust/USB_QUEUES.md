# CPU0 MIDI queues and IRQ restoration

This #19 checkpoint moves the actual 64-event MIDI output admission and input
pop into safe no_std `fm1-hal::usb_queues`. `midi_out_event` and physical
`plat_midi_in` select these kernels under `EN_RUST_USB=1`; C0 is retained.
The producer checks configuration/host output, preserves full-queue drops and
wrapping indexes, writes the slot before publication and then advances its
write index. The consumer copies/delivers a complete word before publishing
its read index. No allocator, new state, payload or C state packet is added.

The raw bridge keeps only four scoped pointer fields, independently bounds its
slot indexes and rejects invalid/alignment/null-callback inputs. It never
forms a Rust reference into slots, indexes or delivery memory. C may pass an
uninitialized output word: the adapter initializes it by a raw write only on
successful delivery. Empty or rejected calls leave output untouched.

## IRQ and role contract

The existing physical `plat_midi_out` already guards the entire publication.
Its legacy HAL0 helper, however, restores IRQs with unconditional `sti`, even
when called with IRQs disabled. USB1 now directly uses the named
`fm1_irq_state_save/restore` binding for this wrapper, independently of HAL
selection. It preserves every incoming ICFG bit, including an initially clear
IRQ gate. C0 behavior, shared flash helpers and other HAL bindings are unchanged.
This is an explicit software correction, not a claim about a previous crash.

One CPU0 producer transaction owns the output queue while masked. Its Rust
call and callback have returned before IRQ restoration permits a deferred
producer. Input's sole CPU0 consumer can be interrupted by the serialized
receive/TRS producer, which writes only free/unpublished slots with its own
scoped raw wrapper. No Rust foreign-buffer borrow crosses that interruption.
The original publication binding is retained; volatile accesses do not make
this a portable threaded or multicore atomic queue. Role serialization, live
fixed allocations and controller/IRQ provider facts remain required.

## Evidence

With Rust 1.94.1:

```sh
sh tests/rust_usb_queues.sh ../baseline-2026-11-11/source
cargo test --locked -p fm1-hal --test usb_queues
```

The native/WASM driver fixture extracts the actual producer, consumer and
physical wrapper; it does not clone their algorithm. All 440 baseline hashes
are verified. Frozen C0, fresh C0, Rust USB1/HAL0 and USB1/HAL1 have the same
queue/packet trace across 41,600 cases: 16,883,840 bytes, SHA-256
`2d08e7e418a7833683c7d9acfcc96d07ebcd73b2647b5beef4ac2e0244b36ef8`.
Fresh C0 WASM and both Rust HAL variants match the same frozen native trace.

Cases combine all 0..64 queue fills, four index/wrap bases, four arbitrary ICFG
patterns, initially enabled/disabled gates, configuration/host-output gating
and interrupts before masking, after masking, at publication, before restore
and immediately after restore. The provider model defers masked IRQs and
executes them as soon as the caller permits. It asserts exact saved ICFG for
USB1, while independently witnessing the intentional C0/HAL0 unconditional
re-enable behavior. This flag difference is asserted separately from the
identical packet trace. Actual input pops also initialize the C output and
verify complete FIFO contents/index advancement.

The safe-core property test adds 3,640 schedules. A peer producer or consumer
interrupts after every abstract scalar, slot and publication operation,
including full/empty and index wrap. Independent expected FIFO sequences
verify no unpublished slot, loss, duplication, overwrite or capacity growth;
full-queue conservative refusal remains permitted. These are single-CPU role
models, not an OS-thread or hardware stress claim.

All 167 Linux workspace tests pass. Clippy with warnings denied, formatting and
no_std/WASM checks pass. Strict-provenance Miri passes retained raw queue roots,
full/wrapped queues, invalid-input refusal and uninitialized output delivery
(0.76s). Vendor PI32v2 main/engine C objects compile in USB1/HAL0 and USB1/HAL1,
proving the independent named IRQ binding is available. Rust MCU emission,
whole-image link, stack/cycle/deadline and physical combined-traffic evidence
remain unqualified for this checkpoint. Retained sources, traces and logs are
in [usb-queues-proof.json](usb-queues-proof.json).

Remaining active C includes device output packet traversal/SysEx priority,
SysEx transmit batching/wait policy, CDC rings, SIE and device lifecycle, host
control/enumeration/hub/attach/loss/restart, UAC producer/resampling/stream and
packet ownership, and foreground message application. Input/parameter queue
admission and MIDI output push/input pop are ported; this does not claim those
remaining consumer or transport paths. No device, image, live server or remote
issue changed.
