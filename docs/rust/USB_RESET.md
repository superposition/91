# SysEx publication across USB reset

This is the explicit #19 correction after the ordinary TX parity checkpoint
`f34ad6b`. App SysEx publication now belongs to one existing USB reset epoch.
A reset followed by immediate reconfiguration cannot resume an old message
partway through its bytes. C0 and the separate polled loader policy remain.

## Transaction and state

The app captures `usb.resets` only while the exact saved-IRQ guard also observes
up/configuration. An inactive bus returns the reserved MAX token. Each event
rechecks the epoch inside the same short guard as slot write, publication and
write-index advancement. The source's at-most-three bytes are prepared before
masking. Every wait, clock read and idle callback runs outside the new masked
span; restoring IRQs preserves every incoming ICFG bit.

| Phase | Guarded work |
| --- | --- |
|Begin|Capture active reset generation; check capacity; mark the existing producer busy|
|Packet|Recheck generation; write one bounded event; publish its index|
|Backpressure|Restore IRQs before timeout/idle; retry with a fresh guarded generation check|
|Finish/abort|Clear the existing busy flag and restore the exact incoming state|

Reset+reconfigure back to configuration 1 still changes the epoch. Once reset
interrupts a partially published message, the queue contains no old tail and
publication stops. A reset after all bytes and the final successful commit
may clear the queue before the function returns 0: enqueue success has never
promised a host ACK. That operation linearizes immediately before reset; no
post-reset tail is allowed in either case.

An incoming disabled IRQ gate cannot wait for the app's timer consumer. Such
a call is admitted only if its whole bounded message fits existing free slots;
otherwise it refuses before publication. No code unconditionally enables IRQs.
The loader's explicit role bit selects the unchanged legacy polled sender:
its IRQ-off idle callback can poll USB itself, and no app IRQ/epoch callback is
required or called there.

The existing reset counter saturates at MAX instead of reissuing a generation.
Exhaustion latches the existing up/detached flags, denies retry and endpoint
work, and guards host mode/start admission (including rechecking after device
polling is disabled). All buffers stay alive; logical inactive flags are not
a physical DMA-stop or memory-reuse proof. A new boot is the next lifetime.
No payload, allocation or persistent state is added. The scoped TX descriptor
grows 52?64 bytes on PI32v2 (104?128 native 64) for the epoch and exact IRQ
callbacks. C/vendor and Rust assert the updated layout; target stack/cycle
cost remains unqualified. The unsafe boundary still creates no endpoint,
message, index or scalar Rust references.

## Evidence and intentional differences

Pinned Rust 1.94.1:

```sh
sh tests/rust_usb_reset.sh ../baseline-2026-11-11/source
sh tests/rust_usb_tx.sh ../baseline-2026-11-11/source
```

The reset fixture verifies all 440 frozen source hashes and runs actual
`usb_poll`, bus-reset clearing, EP0 SET_CONFIGURATION and the producer. Across
21,680 cases per implementation it injects reset/reset+reconfigure at every
packet ordinal and guard/publication/idle boundary, including full/wrapped
queues, near-MAX generations and exact incoming ICFG states. Retained endpoint
roots remain unchanged until simulated completion. A compact event digest
bounds test-harness memory; full SysEx queue bytes and semantic state are also
serialized. All checks remain active with `-DNDEBUG`; no side-effectful call is
hidden inside an optional C assertion.

Unmodified frozen C and fresh C0 are byte-exact and witness 3,090 stale-tail
cases. Corrected Rust checks every partial-reset queue is empty and has no
further tail, verifies saturation/retry denial and never idles with incoming
IRQs off. The old impossible IRQ-off/dead-consumer wait is not fabricated as a
legacy clock experiment; only the corrected path is exercised for that refusal.
Both implementations match their own native/WASM trace: 7,718,084 bytes each,
legacy SHA-256
`73e600da0ee018fee5243d23f5fb38abc3466c5464d06f5912602fc0004afca7`,
corrected SHA-256
`cabfb851bad5240e30521e80921dbb9e5bc539d14d84e9abffd36ad70475fcfd`.
These traces intentionally differ; no parity tolerance is introduced.

Healthy active-app traffic and every legacy loader case still match frozen C,
fresh C0 and both WASM consumers: 277,484 snapshots / 39,903,824 bytes across
four OTA/app/loader profiles. App unconfigured sends are now rejected and are
checked in correction tests rather than mislabeled ordinary parity; loader
unconfigured/IRQ-off behavior remains in its original full corpus.

All 169 Linux workspace tests pass. Formatting, Clippy with warnings denied and
no_std/WASM checks pass. Strict-provenance Miri passes retained roots,
reentrant consumer and incoming-IRQ-off no-wait refusal (2.49s). Vendor PI32v2
C main/engine compile for USB1/HAL0 and USB1/HAL1 with the 64-byte descriptor and
real named IRQ provider. There is no new Rust MCU emission/link, image/stack /
deadline or physical claim. Source and artifact hashes are in
[usb-reset-proof.json](usb-reset-proof.json).

Unknown SIE read/write completion and uncertain endpoint arm/kick still require
the separately planned observed-status and retained-lease correction. Host
retirement/role changes, CDC, UAC and the remaining device lifecycle policy
also remain open. This reset fix does not turn a returned zero, a timeout,
logical reset or mode flag into DMA completion. No device, image, live server
or remote issue changed.
